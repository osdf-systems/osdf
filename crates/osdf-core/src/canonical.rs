use crate::error::{OsdfError, Result};

pub fn canonicalize_json(value: &serde_json::Value) -> Result<Vec<u8>> {
    serde_json_canonicalizer::to_vec(value)
        .map_err(|err| OsdfError::Manifest(format!("JCS canonicalization failed: {err}")))
}

pub fn canonicalize_json_str(json: &str) -> Result<Vec<u8>> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    canonicalize_json(&value)
}
