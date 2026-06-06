use std::collections::BTreeMap;
use std::path::Path;

use chrono::Utc;
use ed25519_dalek::SigningKey;
use rand::RngCore;
use uuid::Uuid;

use crate::constants::{ENVELOPE_PATH, FORMAT_VERSION, HEADER_PATH, MANIFEST_PATH, PROFILE_CORE};
use crate::container::{make_header_bytes, PackageContainer};
use crate::crypto::{format_digest, public_revision_commitment, revision_event_hash, sha256_bytes};
use crate::error::{OsdfError, Result};
use crate::manifest::{compute_manifest_digest, compute_object_entry, parse_manifest};
use crate::merkle::merkle_root;
use crate::revision::{parse_revision, revision_path};
use crate::signature::{
    generate_signing_key, sign_revision_payload, signature_path, verifying_key_to_urn,
};
use crate::types::{
    DocumentNode, PackageManifest, PayloadMode, PublicEnvelope, RevisionRecord, SignatureEnvelope,
    SignatureScope,
};

#[derive(Debug, Clone)]
pub struct CreateOptions {
    pub title: String,
    pub document_id: Option<String>,
    pub signing_key: Option<SigningKey>,
    pub commit: bool,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            title: "Untitled Document".to_string(),
            document_id: None,
            signing_key: None,
            commit: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitOptions {
    pub signing_key: SigningKey,
    pub signer_key_reference: Option<String>,
}

pub fn create_package(options: CreateOptions) -> Result<PackageContainer> {
    let document_id = options
        .document_id
        .unwrap_or_else(|| format!("urn:osdf:doc:{}", Uuid::new_v4()));

    let document = DocumentNode::minimal(&options.title);
    let document_json = serde_json::to_vec_pretty(&document)?;

    let mut container = PackageContainer {
        entries: BTreeMap::new(),
        archive_bytes: 0,
    };

    container.insert(HEADER_PATH, make_header_bytes(0))?;
    container.insert("content/document.json", document_json)?;

    if options.commit {
        let signing_key = options.signing_key.unwrap_or_else(generate_signing_key);
        commit_revision_on_new(
            &mut container,
            &document_id,
            CommitOptions {
                signing_key,
                signer_key_reference: None,
            },
        )?;
    } else {
        finalize_container(&mut container, &document_id, 0, None, None)?;
    }

    Ok(container)
}

pub fn create_package_with_document(
    document_id: &str,
    document_bytes: &[u8],
    signing_key: &SigningKey,
) -> Result<PackageContainer> {
    let mut container = PackageContainer {
        entries: BTreeMap::new(),
        archive_bytes: 0,
    };
    container.insert("content/document.json", document_bytes.to_vec())?;
    commit_revision_on_new(
        &mut container,
        document_id,
        CommitOptions {
            signing_key: signing_key.clone(),
            signer_key_reference: None,
        },
    )?;
    Ok(container)
}

pub fn commit_revision(container: &mut PackageContainer, options: CommitOptions) -> Result<u32> {
    let manifest = parse_manifest(container)?;
    let next_revision = manifest.revision + 1;

    let (parent_event_hash, parent_revision_commitment) = if manifest.revision == 0 {
        (None, None)
    } else {
        let prior = parse_revision(container, manifest.revision)?;
        (
            Some(prior.revision_event_hash.clone()),
            Some(prior.public_commitment.clone()),
        )
    };

    let parent_revision_hash = parent_event_hash
        .as_deref()
        .map(crate::crypto::parse_digest)
        .transpose()?;

    let mut salt = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut salt);

    let signer_key_reference = options
        .signer_key_reference
        .clone()
        .unwrap_or_else(|| verifying_key_to_urn(&options.signing_key.verifying_key()));

    finalize_container(
        container,
        &manifest.document_id,
        next_revision,
        parent_revision_commitment.as_deref(),
        Some(&salt),
    )?;

    let updated = parse_manifest(container)?;
    let root = crate::crypto::parse_digest(&updated.revision_root_hash)?;
    let commitment = crate::crypto::parse_digest(&updated.public_commitment)?;
    let timestamp = Utc::now().to_rfc3339();

    let event_hash = revision_event_hash(
        &updated.document_id,
        next_revision,
        parent_revision_hash.as_ref(),
        &root,
        &commitment,
        &timestamp,
        &signer_key_reference,
    );

    let revision = RevisionRecord {
        document_id: updated.document_id.clone(),
        revision: next_revision,
        parent_revision_hash: parent_event_hash,
        revision_root_hash: updated.revision_root_hash.clone(),
        public_commitment: updated.public_commitment.clone(),
        revision_salt: format_digest(&salt),
        committed_timestamp: timestamp,
        signer_key_reference: signer_key_reference.clone(),
        revision_event_hash: format_digest(&event_hash),
    };

    container.insert(
        revision_path(next_revision),
        serde_json::to_vec_pretty(&revision)?,
    )?;

    let mut signature_envelope = SignatureEnvelope {
        signature_version: "1".to_string(),
        document_id: updated.document_id.clone(),
        revision: next_revision,
        revision_commitment: updated.public_commitment.clone(),
        scope: SignatureScope {
            mode: "document".to_string(),
            mutable_fields: vec![],
        },
        signer_key: signer_key_reference,
        algorithm: "Ed25519".to_string(),
        signature: String::new(),
    };

    signature_envelope.signature =
        sign_revision_payload(&options.signing_key, &signature_envelope)?;
    container.insert(
        signature_path(next_revision),
        serde_json::to_vec_pretty(&signature_envelope)?,
    )?;

    finalize_container(
        container,
        &updated.document_id,
        next_revision,
        parent_revision_commitment.as_deref(),
        Some(&salt),
    )?;

    Ok(next_revision)
}

fn commit_revision_on_new(
    container: &mut PackageContainer,
    document_id: &str,
    options: CommitOptions,
) -> Result<()> {
    let mut salt = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut salt);
    let signer_key_reference = options
        .signer_key_reference
        .clone()
        .unwrap_or_else(|| verifying_key_to_urn(&options.signing_key.verifying_key()));

    finalize_container(container, document_id, 1, None, Some(&salt))?;

    let manifest = parse_manifest(container)?;
    let root = crate::crypto::parse_digest(&manifest.revision_root_hash)?;
    let commitment = crate::crypto::parse_digest(&manifest.public_commitment)?;
    let timestamp = Utc::now().to_rfc3339();

    let event_hash = revision_event_hash(
        document_id,
        1,
        None,
        &root,
        &commitment,
        &timestamp,
        &signer_key_reference,
    );

    let revision = RevisionRecord {
        document_id: document_id.to_string(),
        revision: 1,
        parent_revision_hash: None,
        revision_root_hash: manifest.revision_root_hash.clone(),
        public_commitment: manifest.public_commitment.clone(),
        revision_salt: format_digest(&salt),
        committed_timestamp: timestamp,
        signer_key_reference: signer_key_reference.clone(),
        revision_event_hash: format_digest(&event_hash),
    };

    container.insert(
        "revisions/rev-000001.json",
        serde_json::to_vec_pretty(&revision)?,
    )?;

    let mut signature_envelope = SignatureEnvelope {
        signature_version: "1".to_string(),
        document_id: document_id.to_string(),
        revision: 1,
        revision_commitment: manifest.public_commitment.clone(),
        scope: SignatureScope {
            mode: "document".to_string(),
            mutable_fields: vec![],
        },
        signer_key: signer_key_reference,
        algorithm: "Ed25519".to_string(),
        signature: String::new(),
    };

    signature_envelope.signature =
        sign_revision_payload(&options.signing_key, &signature_envelope)?;
    container.insert(
        signature_path(1),
        serde_json::to_vec_pretty(&signature_envelope)?,
    )?;

    finalize_container(container, document_id, 1, None, Some(&salt))?;
    Ok(())
}

pub fn write_package(container: &PackageContainer, path: &Path) -> Result<()> {
    let bytes = container.to_bytes()?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn finalize_container(
    container: &mut PackageContainer,
    document_id: &str,
    revision: u32,
    parent_revision_commitment: Option<&str>,
    revision_salt: Option<&[u8; 32]>,
) -> Result<()> {
    let envelope = PublicEnvelope {
        format: "OSDF".to_string(),
        version: FORMAT_VERSION.to_string(),
        public_document_id: document_id.to_string(),
        profile: PROFILE_CORE.to_string(),
        payload_mode: PayloadMode::Inline,
    };
    container.insert(ENVELOPE_PATH, serde_json::to_vec(&envelope)?)?;
    container.insert(HEADER_PATH, make_header_bytes(0))?;

    let (_manifest, manifest_bytes) = build_stable_manifest(
        container,
        document_id,
        revision,
        parent_revision_commitment,
        revision_salt,
    )?;
    container.insert(MANIFEST_PATH, manifest_bytes)?;

    let actual_bytes = container.to_bytes()?.len() as u64;
    container.insert(HEADER_PATH, make_header_bytes(actual_bytes))?;
    container.archive_bytes = actual_bytes;
    Ok(())
}

fn build_stable_manifest(
    container: &PackageContainer,
    document_id: &str,
    revision: u32,
    parent_revision_commitment: Option<&str>,
    revision_salt: Option<&[u8; 32]>,
) -> Result<(PackageManifest, Vec<u8>)> {
    let mut objects = container
        .entries
        .iter()
        .filter(|(path, _)| *path != MANIFEST_PATH && *path != ENVELOPE_PATH)
        .map(|(path, entry)| compute_object_entry(path, infer_object_type(path), &entry.bytes))
        .collect::<Vec<_>>();
    objects.sort_by(|left, right| left.path.cmp(&right.path));

    let revision_root_hash = format_digest(&merkle_root(&objects));
    let public_commitment = if revision == 0 {
        format_digest(&sha256_bytes(document_id.as_bytes()))
    } else {
        let salt = revision_salt.ok_or_else(|| {
            OsdfError::Manifest("committed revision requires revision salt".to_string())
        })?;
        format_digest(&public_revision_commitment(
            document_id,
            revision,
            salt,
            &crate::crypto::parse_digest(&revision_root_hash)?,
        ))
    };
    let content_bytes = objects
        .iter()
        .filter(|object| object.path.starts_with("content/"))
        .map(|object| object.bytes)
        .sum();

    let mut manifest = PackageManifest {
        format: "OSDF".to_string(),
        format_version: FORMAT_VERSION.to_string(),
        document_id: document_id.to_string(),
        revision,
        parent_revision_commitment: parent_revision_commitment.map(str::to_string),
        content_bytes,
        revision_root_hash,
        public_commitment,
        manifest_digest: String::new(),
        objects,
    };
    manifest.manifest_digest = compute_manifest_digest(&manifest);
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    Ok((manifest, manifest_bytes))
}

pub fn attach_transparency_proof(
    container: &mut PackageContainer,
    proof: crate::types::TransparencyProof,
) -> Result<()> {
    let manifest = parse_manifest(container)?;
    if manifest.revision == 0 {
        return Err(OsdfError::Manifest(
            "transparency proofs attach only to committed revisions".to_string(),
        ));
    }
    let path = crate::ledger::transparency_proof_path(manifest.revision);
    container.insert(path, serde_json::to_vec_pretty(&proof)?)?;
    refresh_manifest_and_header(container)
}

pub fn refresh_manifest_and_header(container: &mut PackageContainer) -> Result<()> {
    let manifest = parse_manifest(container)?;
    let (_manifest, manifest_bytes) =
        build_stable_manifest_preserve_commitments(container, &manifest)?;
    container.insert(MANIFEST_PATH, manifest_bytes)?;
    let actual_bytes = container.to_bytes()?.len() as u64;
    container.insert(HEADER_PATH, make_header_bytes(actual_bytes))?;
    container.archive_bytes = actual_bytes;
    Ok(())
}

fn build_stable_manifest_preserve_commitments(
    container: &PackageContainer,
    existing: &PackageManifest,
) -> Result<(PackageManifest, Vec<u8>)> {
    let mut objects = container
        .entries
        .iter()
        .filter(|(path, _)| *path != MANIFEST_PATH && *path != ENVELOPE_PATH)
        .map(|(path, entry)| compute_object_entry(path, infer_object_type(path), &entry.bytes))
        .collect::<Vec<_>>();
    objects.sort_by(|left, right| left.path.cmp(&right.path));

    let content_bytes = objects
        .iter()
        .filter(|object| object.path.starts_with("content/"))
        .map(|object| object.bytes)
        .sum();

    let mut manifest = PackageManifest {
        format: "OSDF".to_string(),
        format_version: FORMAT_VERSION.to_string(),
        document_id: existing.document_id.clone(),
        revision: existing.revision,
        parent_revision_commitment: existing.parent_revision_commitment.clone(),
        content_bytes,
        revision_root_hash: existing.revision_root_hash.clone(),
        public_commitment: existing.public_commitment.clone(),
        manifest_digest: String::new(),
        objects,
    };
    manifest.manifest_digest = compute_manifest_digest(&manifest);
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    Ok((manifest, manifest_bytes))
}

fn infer_object_type(path: &str) -> &'static str {
    match path {
        crate::constants::HEADER_PATH => "osdf-header",
        ENVELOPE_PATH => "public-envelope",
        path if path.starts_with("revisions/") => "revision-record",
        path if path.starts_with("signatures/") => "signature-envelope",
        path if path.starts_with("transparency/") => "transparency-proof",
        "content/document.json" => "document-tree",
        path if path.ends_with(".json") => "json-object",
        _ => "binary-asset",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::verify_container;

    #[test]
    fn create_draft_package_verifies() {
        let container = create_package(CreateOptions {
            title: "Test".to_string(),
            ..Default::default()
        })
        .unwrap();
        let report = verify_container(&container);
        assert_eq!(
            report.overall,
            crate::report::VerificationStatus::Pass,
            "{:?}",
            report.findings
        );
        assert_eq!(report.revision, Some(0));
    }

    #[test]
    fn create_committed_package_verifies() {
        let container = create_package(CreateOptions {
            title: "Committed".to_string(),
            commit: true,
            ..Default::default()
        })
        .unwrap();
        let report = verify_container(&container);
        assert_eq!(
            report.overall,
            crate::report::VerificationStatus::Pass,
            "{:?}",
            report.findings
        );
        assert_eq!(report.revision, Some(1));
        assert_eq!(report.signature_count, 1);
        assert!(report
            .checks
            .iter()
            .any(|check| check.code == "OSDF_SIGNATURE_CRYPTO"
                && check.status == crate::report::VerificationStatus::Pass));
    }
}
