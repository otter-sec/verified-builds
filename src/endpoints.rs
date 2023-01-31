use std::{
    collections::HashMap,
    error::Error,
    io::Write,
    process::{Command, Stdio},
    str::from_utf8,
    sync::{Arc, RwLock},
};

use serde::Deserialize;
use serde_json::json;
use tempdir::TempDir;
use warp::{http::StatusCode, Rejection, Reply};

use crate::parameters::{BufferHashParams, DockerfileParams, ProgramHashParams, VerifyParams};

pub const DOCKERFILE: &[u8] = include_bytes!("../docker/Dockerfile");

pub async fn index() -> Result<impl Reply, Rejection> {
    let routes = json!({
        "routes": {
            "GET /": "Welcome to Otter API",
        }
    });
    Ok(warp::reply::json(&routes))
}

pub async fn dockerfile(_: DockerfileParams) -> Result<impl Reply, Rejection> {
    Ok(from_utf8(DOCKERFILE).unwrap())
}

pub async fn health() -> Result<impl Reply, Rejection> {
    Ok(warp::reply::with_status("Ok", StatusCode::OK))
}

pub async fn program_hash(params: ProgramHashParams) -> Result<impl Reply, Rejection> {
    program_hash_inner(params).map_err(|e| {
        println!("got: {:?}", e);
        warp::reject::reject()
    })
}

pub fn program_hash_inner(params: ProgramHashParams) -> Result<impl Reply, Box<dyn Error>> {
    Ok(warp::reply::json(&json!({
        "hash": dump_and_get_hash(&params.program_id, DumpType::Program)?
    })))
}

pub async fn buffer_hash(params: BufferHashParams) -> Result<impl Reply, Rejection> {
    buffer_hash_inner(params).map_err(|e| {
        println!("got: {:?}", e);
        warp::reject::reject()
    })
}

pub fn buffer_hash_inner(params: BufferHashParams) -> Result<impl Reply, Box<dyn Error>> {
    Ok(warp::reply::json(&json!({
        "hash": dump_and_get_hash(&params.program_id, DumpType::Buffer)?
    })))
}

enum DumpType {
    Buffer,
    Program,
}

fn get_binary_hash(program_data: Vec<u8>) -> String {
    let buffer = program_data
        .into_iter()
        .rev()
        .skip_while(|&x| x == 0)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();

    sha256::digest(&buffer[..])
}

fn dump_and_get_hash(addr: &String, typ: DumpType) -> Result<String, Box<dyn Error>> {
    let tmp_dir = TempDir::new("solana-program")?;
    let tmp_path = tmp_dir.path().to_str().unwrap();
    let program_path = format!("{}/program.so", tmp_path);
            
    
    let mut cmd = Command::new("solana");

    let cmd = match typ {
        DumpType::Buffer => {
            cmd.arg("account")
                .arg(addr)
                .arg("--output-file")
                .arg(program_path.clone())
        },
        DumpType::Program => {
            cmd
                .arg("program")
                .arg("dump")
                .arg(addr)
                .arg(program_path.clone())
        }
    };

    cmd.output()?
        .status
        .exit_ok()?;

    Ok(get_binary_hash(std::fs::read(&program_path)?))
}

pub async fn verify(
    params: VerifyParams,
    cache: Arc<RwLock<HashMap<(String, String, String), String>>>,
) -> Result<impl Reply, Rejection> {
    verify_inner(params, cache).await.map_err(|e| {
        println!("got: {:?}", e);
        warp::reject::reject()
    })
}

#[derive(Deserialize, Debug)]
struct Config {
    package: Package,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
}

async fn verify_inner(
    params: VerifyParams,
    cache: Arc<RwLock<HashMap<(String, String, String), String>>>,
) -> Result<impl Reply, Box<dyn Error>> {
    let key = (
        params.repo.clone(),
        params.path.clone(),
        params.commit.clone(),
    );

    let cached_val;
    {
        let map = cache.read().unwrap();
        cached_val = map.get(&key).map(|x| (*x).clone());
    }

    let hash = if let Some(val) = cached_val {
        val
    } else {
        if params.path.contains("..") {
            Err("bad request")?;
        }

        let tmp_dir = TempDir::new("verify-repo")?;

        let tmp_path = tmp_dir.path().to_str().unwrap();

        Command::new("git")
            .arg("clone")
            .arg("--")
            .arg(&params.repo)
            .arg(tmp_path)
            .output()?
            .status
            .exit_ok()?;

        Command::new("git")
            .arg("checkout")
            .arg(&params.commit)
            .current_dir(tmp_path)
            .output()?
            .status
            .exit_ok()?;

        let toml_path = tmp_dir.path().join(params.path.clone()).join("Cargo.toml");
        let toml: Config = toml::from_str(&std::fs::read_to_string(&toml_path)?)?;

        let package_name = toml.package.name;

        let mut child = Command::new("docker")
            .arg("build")
            .arg("-q")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        {
            let child_stdin = child.stdin.as_mut().unwrap();
            child_stdin.write_all(DOCKERFILE)?;
        }

        let result = child.wait_with_output()?;
        result.status.exit_ok()?;

        let image_hash = String::from_utf8(result.stdout)?;
        let image_hash = image_hash.trim_end();

        Command::new("docker")
            .arg("run")
            .arg("--volume")
            .arg(format!("{}:/build", tmp_dir.path().to_str().unwrap()))
            .arg("--workdir")
            .arg(format!("/build/{}", params.path))
            .arg("--rm")
            .arg("--")
            .arg(image_hash)
            .arg("sh")
            .arg("-c")
            .arg(format!("cargo build-bpf -- --locked --frozen"))
            .output()?
            .status
            .exit_ok()?;

        let hash = get_binary_hash(std::fs::read(
            tmp_dir
                .path()
                .join("target/deploy")
                .join(format!("{}.so", package_name.replace("-", "_"))),
        )?);

        cache.write().unwrap().insert(key, hash.clone());

        hash
    };

    Ok(warp::reply::json(&json!({
        "hash": hash,
    })))
}
