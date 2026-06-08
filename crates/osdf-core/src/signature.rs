use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

#[cfg(feature = "native-create")]
use rand::rngs::OsRng;

use crate::canonical::canonicalize_json;
use crate::container::PackageContainer;
use crate::crypto::{digests_equal, parse_digest};
use crate::error::{OsdfError, Result};
use crate::manifest::parse_manifest;
use crate::types::SignatureEnvelope;

pub fn signature_path(revision: u32) -> String {
    format!("signatures/revision-{revision:06}.sig.json")
}

#[cfg(feature = "native-create")]
pub fn generate_signing_key() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

pub fn verifying_key_to_urn(key: &VerifyingKey) -> String {
    format!("urn:osdf:key:ed25519:{}", hex::encode(key.as_bytes()))
}

pub fn verifying_key_from_urn(urn: &str) -> Result<VerifyingKey> {
    let Some(hex_part) = urn.strip_prefix("urn:osdf:key:ed25519:") else {
        return Err(OsdfError::Signature(format!("unsupported key urn: {urn}")));
    };
    let bytes = hex::decode(hex_part)
        .map_err(|err| OsdfError::Signature(format!("invalid key hex in `{urn}`: {err}")))?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| OsdfError::Signature(format!("ed25519 key must be 32 bytes: {urn}")))?;
    VerifyingKey::from_bytes(&array)
        .map_err(|err| OsdfError::Signature(format!("invalid ed25519 key `{urn}`: {err}")))
}

pub fn sign_revision_payload(
    signing_key: &SigningKey,
    envelope: &SignatureEnvelope,
) -> Result<String> {
    let payload = canonicalize_json(&serde_json::to_value(envelope)?)?;
    let signature = signing_key.sign(&payload);
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes()))
}

pub fn parse_signature(container: &PackageContainer, revision: u32) -> Result<SignatureEnvelope> {
    let path = signature_path(revision);
    let bytes = container
        .get(&path)
        .ok_or_else(|| OsdfError::Signature(format!("missing `{path}`")))?;
    Ok(serde_json::from_slice(bytes)?)
}

fn verify_signature_envelope(
    envelope: &SignatureEnvelope,
    manifest: &crate::types::PackageManifest,
) -> Result<()> {
    if envelope.document_id != manifest.document_id {
        return Err(OsdfError::Signature(
            "signature document_id mismatch".to_string(),
        ));
    }

    if envelope.algorithm != "Ed25519" {
        return Err(OsdfError::Signature(format!(
            "unsupported signature algorithm: {}",
            envelope.algorithm
        )));
    }

    let commitment = parse_digest(&envelope.revision_commitment)?;
    let manifest_commitment = parse_digest(&manifest.public_commitment)?;
    if envelope.revision == manifest.revision && !digests_equal(&commitment, &manifest_commitment) {
        return Err(OsdfError::Signature(
            "signature revision commitment mismatch".to_string(),
        ));
    }

    let verifying_key = verifying_key_from_urn(&envelope.signer_key)?;
    let mut unsigned = envelope.clone();
    unsigned.signature = String::new();
    let payload = canonicalize_json(&serde_json::to_value(&unsigned)?)?;

    let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&envelope.signature)
        .map_err(|err| OsdfError::Signature(format!("invalid signature encoding: {err}")))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|err| OsdfError::Signature(format!("invalid ed25519 signature bytes: {err}")))?;

    verifying_key
        .verify(&payload, &signature)
        .map_err(|err| OsdfError::Signature(format!("signature verification failed: {err}")))?;

    if envelope.scope.mode != "document-except-fields" && !envelope.scope.mutable_fields.is_empty()
    {
        return Err(OsdfError::Signature(
            "Phase A supports only full-document or document-except-fields scopes".to_string(),
        ));
    }

    Ok(())
}

pub fn verify_signatures(container: &PackageContainer) -> Result<Vec<SignatureEnvelope>> {
    let manifest = parse_manifest(container)?;
    let mut verified = Vec::new();
    let mut failures = Vec::new();

    for path in container.paths() {
        if !path.starts_with("signatures/") || !path.ends_with(".sig.json") {
            continue;
        }

        let Some(bytes) = container.get(path) else {
            failures.push(OsdfError::Signature(format!("missing `{path}`")));
            continue;
        };

        let envelope: SignatureEnvelope = match serde_json::from_slice(bytes) {
            Ok(envelope) => envelope,
            Err(err) => {
                failures.push(OsdfError::Signature(format!(
                    "invalid signature JSON in `{path}`: {err}"
                )));
                continue;
            }
        };

        match verify_signature_envelope(&envelope, &manifest) {
            Ok(()) => verified.push(envelope),
            Err(err) => failures.push(err),
        }
    }

    if failures.is_empty() {
        Ok(verified)
    } else {
        Err(failures.into_iter().next().expect("failures non-empty"))
    }
}

#[cfg(all(test, feature = "native-create"))]
mod tests {
    use super::*;
    use crate::types::SignatureScope;

    #[test]
    fn sign_and_verify_payload() {
        let signing_key = generate_signing_key();
        let verifying_key = signing_key.verifying_key();
        let envelope = SignatureEnvelope {
            signature_version: "1".to_string(),
            document_id: "urn:osdf:doc:test".to_string(),
            revision: 1,
            revision_commitment: format!("sha256:{}", "ab".repeat(32)),
            scope: SignatureScope {
                mode: "document".to_string(),
                mutable_fields: vec![],
            },
            signer_key: verifying_key_to_urn(&verifying_key),
            algorithm: "Ed25519".to_string(),
            signature: String::new(),
        };

        let encoded = sign_revision_payload(&signing_key, &envelope).unwrap();
        let mut signed = envelope;
        signed.signature = encoded;

        let mut unsigned = signed.clone();
        unsigned.signature = String::new();
        let payload = canonicalize_json(&serde_json::to_value(&unsigned).unwrap()).unwrap();
        let signature = Signature::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(&signed.signature)
                .unwrap(),
        )
        .unwrap();
        verifying_key.verify(&payload, &signature).unwrap();
    }
}
