use std::path::Path;

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, Verifier};
use serde::{Deserialize, Serialize};

use crate::canonical::canonicalize_json;
use crate::container::PackageContainer;
use crate::crypto::{digest_strings_equal, digests_equal, format_digest, parse_digest};
use crate::error::{OsdfError, Result};
use crate::revision::parse_revision;
use crate::signature::verifying_key_from_urn;
use crate::types::{SignedTreeHead, TransparencyProof};

pub const DOMAIN_LOG_LEAF: &str = "OSDF-LOG-LEAF-v1";
pub const DOMAIN_LOG_NODE: &str = "OSDF-LOG-NODE-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LedgerPolicy {
    #[default]
    Disabled,
    Optional,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrustedLog {
    pub log_id: String,
    pub log_public_key_urn: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LatestRevisionPolicy {
    #[default]
    Disabled,
    Optional,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLatestRevision {
    pub document_id: String,
    pub revision: u32,
    pub revision_event_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaf_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LedgerConfig {
    #[serde(default)]
    pub policy: LedgerPolicy,
    #[serde(default)]
    pub trusted_logs: Vec<TrustedLog>,
    #[serde(default)]
    pub latest_revision_policy: LatestRevisionPolicy,
    #[serde(default)]
    pub latest_revisions: Vec<DocumentLatestRevision>,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            policy: LedgerPolicy::Disabled,
            trusted_logs: Vec::new(),
            latest_revision_policy: LatestRevisionPolicy::Disabled,
            latest_revisions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LedgerStore {
    pub log_id: String,
    pub log_public_key_urn: String,
    pub leaves: Vec<String>,
    #[serde(default)]
    pub latest_revisions: Vec<DocumentLatestRevision>,
}

impl LedgerStore {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    pub fn append_only_log(&self) -> AppendOnlyLog {
        let leaves = self
            .leaves
            .iter()
            .filter_map(|value| parse_digest(value).ok())
            .collect();
        AppendOnlyLog { leaves }
    }
}

pub fn transparency_proof_path(revision: u32) -> String {
    format!("transparency/rev-{revision:06}.proof.json")
}

pub fn log_leaf_hash(revision_event_hash: &[u8; 32]) -> [u8; 32] {
    crate::crypto::digest_prefixed(DOMAIN_LOG_LEAF, &[revision_event_hash])
}

pub fn log_node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    crate::crypto::digest_prefixed(DOMAIN_LOG_NODE, &[left, right])
}

#[derive(Debug, Clone, Default)]
pub struct AppendOnlyLog {
    leaves: Vec<[u8; 32]>,
}

impl AppendOnlyLog {
    pub fn from_event_hashes(leaves: Vec<[u8; 32]>) -> Self {
        Self { leaves }
    }

    pub fn tree_size(&self) -> u64 {
        self.leaves.len() as u64
    }

    pub fn append(&mut self, revision_event_hash: [u8; 32]) -> u64 {
        self.leaves.push(revision_event_hash);
        (self.leaves.len() - 1) as u64
    }

    pub fn root(&self) -> [u8; 32] {
        if self.leaves.is_empty() {
            return log_node_hash(&[0u8; 32], &[0u8; 32]);
        }
        let mut level: Vec<[u8; 32]> = self.leaves.iter().map(log_leaf_hash).collect();
        while level.len() > 1 {
            level = collapse_level(&level);
        }
        level[0]
    }

    pub fn inclusion_proof(&self, leaf_index: u64) -> Result<Vec<[u8; 32]>> {
        let index = leaf_index as usize;
        if index >= self.leaves.len() {
            return Err(OsdfError::Integrity(format!(
                "leaf index {leaf_index} out of range for tree size {}",
                self.leaves.len()
            )));
        }

        let mut proof = Vec::new();
        let mut level: Vec<[u8; 32]> = self.leaves.iter().map(log_leaf_hash).collect();
        let mut index = index;
        let mut size = level.len();

        while size > 1 {
            let sibling = if index.is_multiple_of(2) {
                if index + 1 < size {
                    index + 1
                } else {
                    proof.push(level[index]);
                    index /= 2;
                    size = collapse_level_len(size);
                    level = collapse_level(&level);
                    continue;
                }
            } else {
                index - 1
            };
            proof.push(level[sibling]);
            index /= 2;
            size = collapse_level_len(size);
            level = collapse_level(&level);
        }

        Ok(proof)
    }
}

fn collapse_level_len(size: usize) -> usize {
    size.div_ceil(2)
}

fn collapse_level(level: &[[u8; 32]]) -> Vec<[u8; 32]> {
    let mut next = Vec::new();
    let mut index = 0;
    while index < level.len() {
        if index + 1 < level.len() {
            next.push(log_node_hash(&level[index], &level[index + 1]));
            index += 2;
        } else {
            next.push(level[index]);
            index += 1;
        }
    }
    next
}

pub fn verify_inclusion_proof(
    revision_event_hash: &[u8; 32],
    leaf_index: u64,
    tree_size: u64,
    proof: &[[u8; 32]],
) -> Result<[u8; 32]> {
    if tree_size == 0 {
        return Err(OsdfError::Integrity(
            "cannot verify inclusion proof against empty log".to_string(),
        ));
    }
    if leaf_index >= tree_size {
        return Err(OsdfError::Integrity(format!(
            "leaf index {leaf_index} out of range for tree size {tree_size}"
        )));
    }

    let mut hash = log_leaf_hash(revision_event_hash);
    let mut index = leaf_index as usize;
    let mut size = tree_size as usize;
    let mut proof_iter = proof.iter();

    while size > 1 {
        if index.is_multiple_of(2) {
            if index + 1 < size {
                let sibling = proof_iter
                    .next()
                    .ok_or_else(|| OsdfError::Integrity("inclusion proof too short".to_string()))?;
                hash = log_node_hash(&hash, sibling);
            }
        } else {
            let sibling = proof_iter
                .next()
                .ok_or_else(|| OsdfError::Integrity("inclusion proof too short".to_string()))?;
            hash = log_node_hash(sibling, &hash);
        }
        index /= 2;
        size = collapse_level_len(size);
    }

    if proof_iter.next().is_some() {
        return Err(OsdfError::Integrity("inclusion proof too long".to_string()));
    }

    Ok(hash)
}

pub fn sign_tree_head(
    signing_key: &SigningKey,
    log_id: &str,
    tree_size: u64,
    root_hash: &[u8; 32],
    timestamp: &str,
) -> Result<SignedTreeHead> {
    let log_key_reference = crate::signature::verifying_key_to_urn(&signing_key.verifying_key());
    let mut head = SignedTreeHead {
        log_id: log_id.to_string(),
        tree_size,
        root_hash: format_digest(root_hash),
        timestamp: timestamp.to_string(),
        log_key_reference,
        algorithm: "Ed25519".to_string(),
        signature: String::new(),
    };
    head.signature = sign_tree_head_payload(signing_key, &head)?;
    Ok(head)
}

pub fn sign_tree_head_payload(signing_key: &SigningKey, head: &SignedTreeHead) -> Result<String> {
    let mut unsigned = head.clone();
    unsigned.signature = String::new();
    let payload = canonicalize_json(&serde_json::to_value(&unsigned)?)?;
    let signature = signing_key.sign(&payload);
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes()))
}

pub fn verify_signed_tree_head(head: &SignedTreeHead) -> Result<()> {
    if head.algorithm != "Ed25519" {
        return Err(OsdfError::Signature(format!(
            "unsupported log signature algorithm: {}",
            head.algorithm
        )));
    }

    let verifying_key = verifying_key_from_urn(&head.log_key_reference)?;
    let mut unsigned = head.clone();
    unsigned.signature = String::new();
    let payload = canonicalize_json(&serde_json::to_value(&unsigned)?)?;
    let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&head.signature)
        .map_err(|err| OsdfError::Signature(format!("invalid log signature encoding: {err}")))?;
    let signature = ed25519_dalek::Signature::from_slice(&signature_bytes).map_err(|err| {
        OsdfError::Signature(format!("invalid ed25519 log signature bytes: {err}"))
    })?;
    verifying_key
        .verify(&payload, &signature)
        .map_err(|err| OsdfError::Signature(format!("log signed tree head invalid: {err}")))?;
    Ok(())
}

pub fn parse_transparency_proof(
    container: &PackageContainer,
    revision: u32,
) -> Result<TransparencyProof> {
    let path = transparency_proof_path(revision);
    let bytes = container
        .get(&path)
        .ok_or_else(|| OsdfError::Integrity(format!("missing `{path}`")))?;
    Ok(serde_json::from_slice(bytes)?)
}

pub fn build_transparency_proof(
    log_id: &str,
    leaf_index: u64,
    revision_event_hash: [u8; 32],
    proof: &[[u8; 32]],
    signed_tree_head: SignedTreeHead,
) -> TransparencyProof {
    TransparencyProof {
        proof_version: "1".to_string(),
        log_id: log_id.to_string(),
        log_entry_id: format!("{log_id}#{leaf_index}"),
        leaf_index,
        tree_size: signed_tree_head.tree_size,
        revision_event_hash: format_digest(&revision_event_hash),
        inclusion_proof: proof.iter().map(format_digest).collect(),
        signed_tree_head,
    }
}

pub fn verify_transparency_proof(
    proof: &TransparencyProof,
    expected_revision_event_hash: &str,
    config: &LedgerConfig,
) -> Result<()> {
    if config.policy == LedgerPolicy::Disabled {
        return Ok(());
    }

    if !digest_strings_equal(&proof.revision_event_hash, expected_revision_event_hash) {
        return Err(OsdfError::Integrity(
            "transparency proof revisionEventHash mismatch".to_string(),
        ));
    }

    let leaf = parse_digest(&proof.revision_event_hash)?;
    let path: Vec<[u8; 32]> = proof
        .inclusion_proof
        .iter()
        .map(|node| parse_digest(node))
        .collect::<Result<_>>()?;
    let computed_root = verify_inclusion_proof(&leaf, proof.leaf_index, proof.tree_size, &path)?;

    let declared_root = parse_digest(&proof.signed_tree_head.root_hash)?;
    if !digests_equal(&computed_root, &declared_root) {
        return Err(OsdfError::Integrity(
            "transparency inclusion proof does not match signed tree head".to_string(),
        ));
    }

    if proof.signed_tree_head.tree_size != proof.tree_size {
        return Err(OsdfError::Integrity(
            "transparency proof treeSize mismatch".to_string(),
        ));
    }

    verify_signed_tree_head(&proof.signed_tree_head)?;

    if config.policy == LedgerPolicy::Required || config.policy == LedgerPolicy::Optional {
        let trusted = config
            .trusted_logs
            .iter()
            .any(|entry| entry.log_id == proof.log_id);
        if !trusted {
            return Err(OsdfError::Signature(format!(
                "log id `{}` is not in trusted log registry",
                proof.log_id
            )));
        }

        let key_trusted = config.trusted_logs.iter().any(|entry| {
            entry.log_id == proof.log_id
                && entry.log_public_key_urn == proof.signed_tree_head.log_key_reference
        });
        if !key_trusted {
            return Err(OsdfError::Signature(format!(
                "log key `{}` is not trusted for log `{}`",
                proof.signed_tree_head.log_key_reference, proof.log_id
            )));
        }
    }

    Ok(())
}

pub fn revision_event_hash_for(container: &PackageContainer, revision: u32) -> Result<String> {
    let record = parse_revision(container, revision)?;
    Ok(record.revision_event_hash)
}

#[cfg(feature = "native-create")]
pub fn revision_event_hash_bytes(container: &PackageContainer, revision: u32) -> Result<[u8; 32]> {
    let digest = revision_event_hash_for(container, revision)?;
    parse_digest(&digest)
}

#[cfg(feature = "native-create")]
pub fn find_leaf_index(store: &LedgerStore, revision_event_hash: &[u8; 32]) -> Option<u64> {
    let digest = format_digest(revision_event_hash);
    store
        .leaves
        .iter()
        .position(|leaf| leaf == &digest)
        .map(|index| index as u64)
}

#[cfg(feature = "native-create")]
pub fn create_ledger_store(log_id: Option<String>, signing_key: &SigningKey) -> LedgerStore {
    let log_id = log_id.unwrap_or_else(|| format!("urn:osdf:log:{}", uuid::Uuid::new_v4()));
    LedgerStore {
        log_id,
        log_public_key_urn: crate::signature::verifying_key_to_urn(&signing_key.verifying_key()),
        leaves: Vec::new(),
        latest_revisions: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerSigningKeyFile {
    pub log_id: String,
    pub seed_b64: String,
}

#[cfg(feature = "native-create")]
pub fn save_signing_key(path: &Path, log_id: &str, signing_key: &SigningKey) -> Result<()> {
    let file = LedgerSigningKeyFile {
        log_id: log_id.to_string(),
        seed_b64: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signing_key.to_bytes()),
    };
    std::fs::write(path, serde_json::to_vec_pretty(&file)?)?;
    Ok(())
}

#[cfg(feature = "native-create")]
pub fn load_signing_key(path: &Path) -> Result<(String, SigningKey)> {
    let bytes = std::fs::read(path)?;
    let file: LedgerSigningKeyFile = serde_json::from_slice(&bytes)?;
    let seed = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&file.seed_b64)
        .map_err(|err| OsdfError::Signature(format!("invalid ledger key seed encoding: {err}")))?;
    let seed: [u8; 32] = seed
        .try_into()
        .map_err(|_| OsdfError::Signature("ledger key seed must be 32 bytes".to_string()))?;
    Ok((file.log_id, SigningKey::from_bytes(&seed)))
}

pub fn lookup_latest_revision<'a>(
    config: &'a LedgerConfig,
    document_id: &str,
) -> Option<&'a DocumentLatestRevision> {
    config
        .latest_revisions
        .iter()
        .find(|entry| entry.document_id == document_id)
}

pub fn register_latest_revision(
    store: &mut LedgerStore,
    document_id: &str,
    revision: u32,
    revision_event_hash: &[u8; 32],
    leaf_index: u64,
) {
    let entry = DocumentLatestRevision {
        document_id: document_id.to_string(),
        revision,
        revision_event_hash: format_digest(revision_event_hash),
        leaf_index: Some(leaf_index),
        updated_at: Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
    };

    if let Some(existing) = store
        .latest_revisions
        .iter_mut()
        .find(|item| item.document_id == document_id)
    {
        *existing = entry;
    } else {
        store.latest_revisions.push(entry);
    }
}

#[cfg(feature = "native-create")]
pub fn append_revision_to_store(
    store: &mut LedgerStore,
    document_id: &str,
    revision: u32,
    revision_event_hash: [u8; 32],
) -> u64 {
    let leaf_index = store.leaves.len() as u64;
    store.leaves.push(format_digest(&revision_event_hash));
    register_latest_revision(
        store,
        document_id,
        revision,
        &revision_event_hash,
        leaf_index,
    );
    leaf_index
}

#[cfg(feature = "native-create")]
pub fn build_proof_for_store(
    store: &LedgerStore,
    leaf_index: u64,
    revision_event_hash: [u8; 32],
    log_signing_key: &SigningKey,
) -> Result<TransparencyProof> {
    let log = store.append_only_log();
    if leaf_index >= log.tree_size() {
        return Err(OsdfError::Integrity(format!(
            "leaf index {leaf_index} out of range for ledger tree size {}",
            log.tree_size()
        )));
    }
    let proof_nodes = log.inclusion_proof(leaf_index)?;
    let root = log.root();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let head = sign_tree_head(
        log_signing_key,
        &store.log_id,
        log.tree_size(),
        &root,
        &timestamp,
    )?;
    Ok(build_transparency_proof(
        &store.log_id,
        leaf_index,
        revision_event_hash,
        &proof_nodes,
        head,
    ))
}

#[cfg(feature = "native-create")]
pub fn trust_config_for_store(store: &LedgerStore, policy: LedgerPolicy) -> LedgerConfig {
    LedgerConfig {
        policy,
        trusted_logs: vec![TrustedLog {
            log_id: store.log_id.clone(),
            log_public_key_urn: store.log_public_key_urn.clone(),
        }],
        latest_revision_policy: if store.latest_revisions.is_empty() {
            LatestRevisionPolicy::Disabled
        } else {
            LatestRevisionPolicy::Optional
        },
        latest_revisions: store.latest_revisions.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::generate_signing_key;

    #[test]
    fn inclusion_proof_roundtrip() {
        let mut log = AppendOnlyLog::default();
        let a = [1u8; 32];
        let b = [2u8; 32];
        log.append(a);
        log.append(b);

        let proof = log.inclusion_proof(1).unwrap();
        let root = log.root();
        let computed = verify_inclusion_proof(&b, 1, 2, &proof).unwrap();
        assert_eq!(computed, root);
    }

    #[test]
    fn signed_tree_head_roundtrip() {
        let key = generate_signing_key();
        let root = [9u8; 32];
        let head =
            sign_tree_head(&key, "urn:osdf:log:test", 1, &root, "2026-06-04T12:00:00Z").unwrap();
        verify_signed_tree_head(&head).unwrap();
    }
}
