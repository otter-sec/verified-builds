use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct VerifyParams {
    pub repo: String,
    pub path: String,
    pub commit: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProgramHashParams {
    pub program_id: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DockerfileParams {
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct BufferHashParams {
    pub program_id: String,
}
