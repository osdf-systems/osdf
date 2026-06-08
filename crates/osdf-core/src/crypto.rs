use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::constants::{
    DOMAIN_LEAF, DOMAIN_NODE, DOMAIN_OBJECT, DOMAIN_REVISION, DOMAIN_REV_EVENT,
};
use crate::error::{OsdfError, Result};

pub fn sha256_bytes(data: impl AsRef<[u8]>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data.as_ref());
    hasher.finalize().into()
}

pub fn digest_prefixed(domain: &str, parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

pub fn object_digest(object_type: &str, content: &[u8]) -> [u8; 32] {
    digest_prefixed(DOMAIN_OBJECT, &[object_type.as_bytes(), content])
}

pub fn merkle_leaf(path: &str, object_type: &str, size: u64, digest: &[u8; 32]) -> [u8; 32] {
    digest_prefixed(
        DOMAIN_LEAF,
        &[
            path.as_bytes(),
            object_type.as_bytes(),
            &size.to_be_bytes(),
            digest,
        ],
    )
}

pub fn merkle_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    digest_prefixed(DOMAIN_NODE, &[left, right])
}

pub fn revision_event_hash(
    document_id: &str,
    revision_number: u32,
    parent_revision_hash: Option<&[u8; 32]>,
    revision_root_hash: &[u8; 32],
    public_commitment: &[u8; 32],
    committed_timestamp: &str,
    signer_key_reference: &str,
) -> [u8; 32] {
    let parent = parent_revision_hash.unwrap_or(&[0u8; 32]);
    digest_prefixed(
        DOMAIN_REV_EVENT,
        &[
            document_id.as_bytes(),
            &revision_number.to_be_bytes(),
            parent,
            revision_root_hash,
            public_commitment,
            committed_timestamp.as_bytes(),
            signer_key_reference.as_bytes(),
        ],
    )
}

pub fn public_revision_commitment(
    document_id: &str,
    revision_number: u32,
    revision_salt: &[u8; 32],
    revision_root_hash: &[u8; 32],
) -> [u8; 32] {
    digest_prefixed(
        DOMAIN_REVISION,
        &[
            document_id.as_bytes(),
            &revision_number.to_be_bytes(),
            revision_salt,
            revision_root_hash,
        ],
    )
}

pub fn format_digest(digest: &[u8; 32]) -> String {
    format!("sha256:{}", hex::encode(digest))
}

/// Constant-time equality for 32-byte digests (timing side-channel resistant).
pub fn digests_equal(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.ct_eq(right).into()
}

/// Compare two `sha256:…` digest strings using constant-time byte equality when both parse.
pub fn digest_strings_equal(left: &str, right: &str) -> bool {
    match (parse_digest(left), parse_digest(right)) {
        (Ok(left), Ok(right)) => digests_equal(&left, &right),
        _ => false,
    }
}

pub fn parse_digest(value: &str) -> Result<[u8; 32]> {
    let Some(hex_part) = value.strip_prefix("sha256:") else {
        return Err(OsdfError::Integrity(format!(
            "unsupported digest encoding: {value}"
        )));
    };
    let bytes = hex::decode(hex_part)
        .map_err(|err| OsdfError::Integrity(format!("invalid digest hex `{value}`: {err}")))?;
    bytes
        .try_into()
        .map_err(|_| OsdfError::Integrity(format!("digest must be 32 bytes: {value}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_roundtrip() {
        let digest = sha256_bytes(b"hello");
        let formatted = format_digest(&digest);
        assert_eq!(parse_digest(&formatted).unwrap(), digest);
    }

    #[test]
    fn digests_equal_matches_eq() {
        let a = sha256_bytes(b"a");
        let b = sha256_bytes(b"b");
        assert!(digests_equal(&a, &a));
        assert!(!digests_equal(&a, &b));
    }
}
