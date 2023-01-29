use std::{
    error::Error,
    io::Write,
    process::{Command, Stdio},
};

use serde_json::json;
use tempdir::TempDir;
use warp::{http::StatusCode, Rejection, Reply};

use crate::{
    parameters::{VerifyParams},
};

pub async fn index() -> Result<impl Reply, Rejection> {
    let routes = json!({
        "routes": {
            "GET /": "Welcome to Otter API",
            "GET /health": "Check if the API is up",
        }
    });
    Ok(warp::reply::json(&routes))
}

pub async fn health() -> Result<impl Reply, Rejection> {
    Ok(warp::reply::with_status("Ok", StatusCode::OK))
}

pub async fn verify(params: VerifyParams) -> Result<impl Reply, Rejection> {
    verify_inner(params).await.map_err(|e| {
        println!("got: {:?}", e);
        warp::reject::reject()
    })
}

async fn verify_inner(params: VerifyParams) -> Result<impl Reply, Box<dyn Error>> {
    let tmp_dir = TempDir::new("verify-repo")?;

    let tmp_path = tmp_dir.path().to_str().unwrap();
    
    let output = Command::new("git")
        .arg("clone")
        .arg("--")
        .arg(&params.repo)
        .arg(tmp_path)
        .output()?;

    let output = Command::new("git")
        .arg("checkout")
        .arg(&params.commit)
        .current_dir(tmp_path)
        .output()?;
    println!("cloned: {:?}", output);

    let toml_path = tmp_dir.path().join(params.output_path.clone()).join("Cargo.toml");
    
    let mut child = Command::new("docker")
        .arg("build")
        .arg("-q")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let child_stdin = child.stdin.as_mut().unwrap();
        child_stdin.write_all(include_bytes!("../docker/Dockerfile"))?;
    }

    let image_hash = String::from_utf8(child.wait_with_output()?.stdout)?;
    let image_hash = image_hash.trim_end();
    println!("{:?}", image_hash);

    let result = Command::new("docker")
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
        .output()?;

    let data = std::fs::read(tmp_dir.path().join(params.output_path))?;
    let hash = sha256::digest(&data[..]);
    
    let tmp_dir = TempDir::new("solana-program")?;
    let tmp_path = tmp_dir.path().to_str().unwrap();
    let program_path = format!("{}/program.so", tmp_path);
    let output = Command::new("solana")
        .arg("program")
        .arg("dump")
        .arg(&params.program_id)
        .arg(program_path.clone())
        .output()?;
    
    let program_data = std::fs::read(&program_path)?;
    let program_hash = sha256::digest(&program_data[..]);

    Ok(warp::reply::json(&json!({
        "hash": hash,
        "program_hash": program_hash
    })))
}
