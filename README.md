# Verified Builds API

To start the server
```bash
cargo r
```

The verify API is at `/verify` with parameters
```rust
pub struct VerifyParams {
    pub repo: String,
    pub path: String,
    pub commit: String,
    pub output_path: String,
    pub program_id: String
}
```