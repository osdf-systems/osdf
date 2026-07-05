//! Compact fail-closed verification without forensic report construction.
//!
//! Semantics match [`crate::verify::verify_package_bytes_with_config`] for decisive
//! pass/fail outcomes. Warnings (e.g. outdated revision) do not fail the fast path;
//! rerun the full verifier for forensic detail.

use std::collections::HashSet;

use crate::constants::{HEADER_PATH, MAGIC, PROFILE_CORE};
use crate::container::{parse_header_bytes, PackageContainer};
use crate::error::OsdfError;
use crate::identity::{resolve_signer_identity, IdentityPolicy, VerifierConfig};
use crate::ledger::{
    lookup_latest_revision, parse_transparency_proof, revision_event_hash_for,
    verify_transparency_proof, LatestRevisionPolicy, LedgerPolicy,
};
use crate::manifest::{parse_envelope, parse_manifest, verify_manifest_objects};
use crate::report::message_from_error;
use crate::revision::{parse_revision, verify_revision_chain};
use crate::signature::verify_signatures;
use crate::types::PayloadMode;
use crate::verify_audit::trailing_bytes_after_zip;
use crate::verify_profile::{FastFailCode, FastVerifyResult, ParsedPackage};

pub fn parse_package(bytes: &[u8]) -> Result<ParsedPackage, FastFailCode> {
    let container = read_container_strict(bytes)?;
    Ok(ParsedPackage {
        archive_bytes: bytes.len() as u64,
        container,
    })
}

pub fn verify_package_bytes_fast(bytes: &[u8], config: &VerifierConfig) -> FastVerifyResult {
    let parsed = match parse_package(bytes) {
        Ok(parsed) => parsed,
        Err(code) => return FastVerifyResult::Fail(code),
    };
    verify_parsed_package_fast(&parsed, config)
}

pub fn verify_parsed_package_fast(
    parsed: &ParsedPackage,
    config: &VerifierConfig,
) -> FastVerifyResult {
    verify_container_fast(&parsed.container, config)
}

pub fn verify_container_fast(
    container: &PackageContainer,
    config: &VerifierConfig,
) -> FastVerifyResult {
    let envelope = match parse_envelope(container) {
        Ok(envelope) => envelope,
        Err(_) => return FastVerifyResult::Fail(FastFailCode::ManifestParseFailure),
    };

    if envelope.profile != PROFILE_CORE || envelope.payload_mode != PayloadMode::Inline {
        return FastVerifyResult::Fail(FastFailCode::UnsupportedProfile);
    }

    if envelope.payload_mode == PayloadMode::Encrypted {
        return FastVerifyResult::Fail(FastFailCode::EncryptedPayload);
    }

    let manifest = match parse_manifest(container) {
        Ok(manifest) => manifest,
        Err(_) => return FastVerifyResult::Fail(FastFailCode::ManifestParseFailure),
    };

    if let Err(err) = verify_manifest_objects(container, &manifest) {
        return FastVerifyResult::Fail(fast_fail_from_error(&err));
    }

    if let Err(err) = verify_revision_chain(container) {
        return FastVerifyResult::Fail(fast_fail_from_error(&err));
    }

    if manifest.revision > 0 {
        match verify_signatures(container) {
            Ok(signatures) if signatures.is_empty() => {
                return FastVerifyResult::Fail(FastFailCode::SignatureInvalid);
            }
            Ok(signatures) => {
                if let Err(code) = verify_identity_fast(container, &manifest, config, &signatures) {
                    return FastVerifyResult::Fail(code);
                }
            }
            Err(err) => return FastVerifyResult::Fail(fast_fail_from_error(&err)),
        }
    }

    if let Err(code) = verify_ledger_fast(container, &manifest, &config.ledger) {
        return FastVerifyResult::Fail(code);
    }

    if let Err(code) = verify_latest_revision_fast(container, &manifest, &config.ledger) {
        return FastVerifyResult::Fail(code);
    }

    FastVerifyResult::Pass
}

fn read_container_strict(bytes: &[u8]) -> Result<PackageContainer, FastFailCode> {
    if trailing_bytes_after_zip(bytes) > 0 {
        return Err(FastFailCode::TrailingBytes);
    }

    let container =
        PackageContainer::read_from_bytes(bytes).map_err(|err| fast_fail_from_error(&err))?;

    let mut seen_folded = HashSet::new();
    for path in container.paths() {
        if !seen_folded.insert(path.to_ascii_lowercase()) {
            return Err(FastFailCode::DuplicatePath);
        }
    }

    let header_bytes = container
        .get(HEADER_PATH)
        .ok_or(FastFailCode::InvalidMagic)?;
    if header_bytes.len() < MAGIC.len() || &header_bytes[..MAGIC.len()] != MAGIC.as_slice() {
        return Err(FastFailCode::InvalidMagic);
    }

    let header = parse_header_bytes(header_bytes).map_err(|err| fast_fail_from_error(&err))?;
    if header.package_bytes != container.archive_bytes {
        return Err(FastFailCode::ContainerError);
    }

    Ok(container)
}

fn verify_identity_fast(
    container: &PackageContainer,
    manifest: &crate::types::PackageManifest,
    config: &VerifierConfig,
    signatures: &[crate::types::SignatureEnvelope],
) -> Result<(), FastFailCode> {
    if config.identity.policy == IdentityPolicy::Disabled {
        return Ok(());
    }

    let signing_timestamp = parse_revision(container, manifest.revision)
        .ok()
        .map(|record| record.committed_timestamp)
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    let signature = signatures
        .iter()
        .find(|entry| entry.revision == manifest.revision)
        .ok_or(FastFailCode::SignatureInvalid)?;

    match resolve_signer_identity(&signature.signer_key, &signing_timestamp, &config.identity) {
        Ok(Some(_)) => Ok(()),
        Ok(None) if config.identity.policy == IdentityPolicy::Required => {
            Err(FastFailCode::TrustPolicyRejected)
        }
        Ok(None) => Ok(()),
        Err(_) => Err(FastFailCode::TrustPolicyRejected),
    }
}

fn verify_ledger_fast(
    container: &PackageContainer,
    manifest: &crate::types::PackageManifest,
    ledger: &crate::ledger::LedgerConfig,
) -> Result<(), FastFailCode> {
    if ledger.policy == LedgerPolicy::Disabled || manifest.revision == 0 {
        return Ok(());
    }

    let proof = parse_transparency_proof(container, manifest.revision)
        .map_err(|_| FastFailCode::LedgerProofInvalid)?;

    let expected_hash = revision_event_hash_for(container, manifest.revision)
        .map_err(|_| FastFailCode::LedgerProofInvalid)?;

    if proof.revision_event_hash != expected_hash {
        return Err(FastFailCode::LedgerProofInvalid);
    }

    verify_transparency_proof(&proof, &expected_hash, ledger)
        .map_err(|_| FastFailCode::LedgerProofInvalid)?;

    Ok(())
}

fn verify_latest_revision_fast(
    container: &PackageContainer,
    manifest: &crate::types::PackageManifest,
    ledger: &crate::ledger::LedgerConfig,
) -> Result<(), FastFailCode> {
    if ledger.latest_revision_policy != LatestRevisionPolicy::Required || manifest.revision == 0 {
        return Ok(());
    }

    revision_event_hash_for(container, manifest.revision)
        .map_err(|_| FastFailCode::RevisionChainInvalid)?;

    if lookup_latest_revision(ledger, &manifest.document_id).is_none() {
        return Err(FastFailCode::LedgerProofInvalid);
    }

    // Full verifier reports WARNING for outdated; fast path treats only missing registry as fail.
    // Outdated remains Pass - quarantine via full rerun.
    Ok(())
}

pub fn fast_fail_from_error(err: &OsdfError) -> FastFailCode {
    let (code, _) = message_from_error(err);
    fast_fail_from_code(&code)
}

pub fn fast_fail_from_code(code: &str) -> FastFailCode {
    match code {
        "OSDF_CONTAINER_INVALID_MAGIC" => FastFailCode::InvalidMagic,
        "OSDF_CONTAINER_PATH_TRAVERSAL" => FastFailCode::UnsafeZipPath,
        "OSDF_CONTAINER_DUPLICATE_PATH" | "OSDF_CONTAINER_CASE_COLLISION" => {
            FastFailCode::DuplicatePath
        }
        "OSDF_CONTAINER_TRAILING_BYTES" => FastFailCode::TrailingBytes,
        "OSDF_MANIFEST_ERROR" | "OSDF_MANIFEST_PARSED" => FastFailCode::ManifestParseFailure,
        "OSDF_MANIFEST_MISSING_OBJECT" => FastFailCode::MissingDeclaredObject,
        "OSDF_MANIFEST_SIZE_MISMATCH" => FastFailCode::ObjectSizeMismatch,
        "OSDF_MANIFEST_HASH_MISMATCH" | "OSDF_INTEGRITY_ERROR" => FastFailCode::ObjectHashMismatch,
        "OSDF_MANIFEST_UNDECLARED_OBJECT" => FastFailCode::UndeclaredObject,
        "OSDF_REVISION_CHAIN_BROKEN" | "OSDF_REVISION_PARENT_COMMITMENT" => {
            FastFailCode::RevisionChainInvalid
        }
        "OSDF_SIGNATURE_INVALID" | "OSDF_SIGNATURE_MISSING" => FastFailCode::SignatureInvalid,
        "OSDF_LEDGER_PROOF_MISSING"
        | "OSDF_LEDGER_INCLUSION_INVALID"
        | "OSDF_LEDGER_LEAF_MISMATCH"
        | "OSDF_LEDGER_SIGNED_ROOT_INVALID"
        | "OSDF_LEDGER_LOG_KEY_UNTRUSTED" => FastFailCode::LedgerProofInvalid,
        "OSDF_IDENTITY_UNTRUSTED" => FastFailCode::TrustPolicyRejected,
        "OSDF_PROFILE_UNSUPPORTED" => FastFailCode::UnsupportedProfile,
        "OSDF_PAYLOAD_ENCRYPTED" => FastFailCode::EncryptedPayload,
        _ => FastFailCode::ContainerError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::verify_package_bytes_with_config;
    use crate::VerificationStatus;

    #[test]
    fn fast_matches_full_on_committed_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/valid/valid-committed.osdf");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        let config = VerifierConfig::default();
        let full = verify_package_bytes_with_config(&bytes, &config);
        let fast = verify_package_bytes_fast(&bytes, &config);
        if full.overall == VerificationStatus::Pass {
            assert_eq!(fast, FastVerifyResult::Pass);
        } else {
            assert!(matches!(fast, FastVerifyResult::Fail(_)));
        }
    }

    #[test]
    fn parsed_fast_matches_portable_fast() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/valid/valid-committed.osdf");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        let config = VerifierConfig::default();
        let parsed = parse_package(&bytes).unwrap();
        let portable = verify_package_bytes_fast(&bytes, &config);
        let revalidation = verify_parsed_package_fast(&parsed, &config);
        assert_eq!(portable, revalidation);
    }
}
