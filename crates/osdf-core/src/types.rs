use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicEnvelope {
    pub format: String,
    pub version: String,
    pub public_document_id: String,
    pub profile: String,
    pub payload_mode: PayloadMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PayloadMode {
    Inline,
    Encrypted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageManifest {
    pub format: String,
    pub format_version: String,
    pub document_id: String,
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_revision_commitment: Option<String>,
    pub content_bytes: u64,
    pub revision_root_hash: String,
    pub public_commitment: String,
    pub manifest_digest: String,
    pub objects: Vec<ManifestObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ManifestObject {
    pub path: String,
    pub object_type: String,
    pub bytes: u64,
    #[serde(default = "default_digest_algorithm")]
    pub digest_algorithm: String,
    pub digest: String,
}

fn default_digest_algorithm() -> String {
    "SHA-256".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RevisionRecord {
    pub document_id: String,
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_revision_hash: Option<String>,
    pub revision_root_hash: String,
    pub public_commitment: String,
    pub revision_salt: String,
    pub committed_timestamp: String,
    pub signer_key_reference: String,
    pub revision_event_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SignatureScope {
    pub mode: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mutable_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SignatureEnvelope {
    pub signature_version: String,
    pub document_id: String,
    pub revision: u32,
    pub revision_commitment: String,
    pub scope: SignatureScope,
    pub signer_key: String,
    pub algorithm: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SignedTreeHead {
    pub log_id: String,
    pub tree_size: u64,
    pub root_hash: String,
    pub timestamp: String,
    pub log_key_reference: String,
    pub algorithm: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransparencyProof {
    pub proof_version: String,
    pub log_id: String,
    pub log_entry_id: String,
    pub leaf_index: u64,
    pub tree_size: u64,
    pub revision_event_hash: String,
    pub inclusion_proof: Vec<String>,
    pub signed_tree_head: SignedTreeHead,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SigningDelegationCredential {
    pub credential_type: String,
    pub organization_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    pub subject_key: String,
    pub valid_from: String,
    pub valid_until: String,
    pub issuer_key: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentNode {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<DocumentNode>,
}

impl DocumentNode {
    pub fn minimal(title: &str) -> Self {
        Self {
            node_type: "document".to_string(),
            language: Some("en-US".to_string()),
            level: None,
            value: None,
            children: vec![DocumentNode {
                node_type: "heading".to_string(),
                language: None,
                level: Some(1),
                value: None,
                children: vec![DocumentNode {
                    node_type: "text".to_string(),
                    language: None,
                    level: None,
                    value: Some(title.to_string()),
                    children: vec![],
                }],
            }],
        }
    }
}
