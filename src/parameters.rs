use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct VerifyParams {
    pub repo: String,
    pub path: String,
    pub commit: String,
    pub output_path: String,
    pub program_id: String
}
