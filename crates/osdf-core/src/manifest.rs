use crate::canonical::canonicalize_json;
use crate::constants::{ENVELOPE_PATH, HEADER_PATH, HEADER_SIZE, MANIFEST_PATH};
use crate::container::PackageContainer;
use crate::crypto::{digest_strings_equal, digests_equal, format_digest, object_digest, parse_digest};
use crate::error::{OsdfError, Result};
use crate::merkle::merkle_root;
use crate::types::{ManifestObject, PackageManifest, PublicEnvelope};

pub fn parse_envelope(container: &PackageContainer) -> Result<PublicEnvelope> {
    let bytes = container
        .get(ENVELOPE_PATH)
        .ok_or_else(|| OsdfError::Manifest(format!("missing `{ENVELOPE_PATH}`")))?;
    let envelope: PublicEnvelope = serde_json::from_slice(bytes)?;
    if envelope.format != "OSDF" {
        return Err(OsdfError::Manifest(
            "envelope format must be OSDF".to_string(),
        ));
    }
    Ok(envelope)
}

pub fn parse_manifest(container: &PackageContainer) -> Result<PackageManifest> {
    let bytes = container
        .get(MANIFEST_PATH)
        .ok_or_else(|| OsdfError::Manifest(format!("missing `{MANIFEST_PATH}`")))?;
    let manifest: PackageManifest = serde_json::from_slice(bytes)?;
    if manifest.format != "OSDF" {
        return Err(OsdfError::Manifest(
            "manifest format must be OSDF".to_string(),
        ));
    }
    Ok(manifest)
}

pub fn compute_manifest_digest(manifest: &PackageManifest) -> String {
    let mut value = serde_json::to_value(manifest).expect("manifest must serialize to json");
    if let Some(map) = value.as_object_mut() {
        map.remove("manifestDigest");
    }
    let canonical = canonicalize_json(&value).expect("manifest must canonicalize");
    format_digest(&object_digest("package-manifest", &canonical))
}

pub fn compute_object_entry(path: &str, object_type: &str, bytes: &[u8]) -> ManifestObject {
    let digest_bytes = if path == HEADER_PATH {
        header_object_digest(bytes)
    } else if looks_like_json_object_type(object_type, path) {
        match serde_json::from_slice::<serde_json::Value>(bytes) {
            Ok(value) => {
                let canonical = canonicalize_json(&value).expect("json object must canonicalize");
                object_digest(object_type, &canonical)
            }
            Err(_) => object_digest(object_type, bytes),
        }
    } else {
        object_digest(object_type, bytes)
    };

    ManifestObject {
        path: path.to_string(),
        object_type: object_type.to_string(),
        bytes: bytes.len() as u64,
        digest_algorithm: "SHA-256".to_string(),
        digest: format_digest(&digest_bytes),
    }
}

fn header_object_digest(bytes: &[u8]) -> [u8; 32] {
    let mut normalized = bytes.to_vec();
    if normalized.len() >= HEADER_SIZE {
        normalized[7..HEADER_SIZE].fill(0);
    }
    object_digest("osdf-header", &normalized)
}

pub fn verify_manifest_objects(
    container: &PackageContainer,
    manifest: &PackageManifest,
) -> Result<()> {
    let mut failures = Vec::new();

    for object in &manifest.objects {
        if object.path == MANIFEST_PATH {
            failures.push(OsdfError::Integrity(
                "manifest must not include itself in objects[] (use manifestDigest)".to_string(),
            ));
            continue;
        }

        let Some(actual) = container.get(&object.path) else {
            failures.push(OsdfError::Integrity(format!(
                "declared object missing: {}",
                object.path
            )));
            continue;
        };

        if actual.len() as u64 != object.bytes {
            failures.push(OsdfError::Integrity(format!(
                "byte length mismatch for `{}`: expected {}, got {}",
                object.path,
                object.bytes,
                actual.len()
            )));
        }

        let expected = match parse_digest(&object.digest) {
            Ok(digest) => digest,
            Err(err) => {
                failures.push(err);
                continue;
            }
        };
        let computed = if object.path == HEADER_PATH {
            header_object_digest(actual)
        } else {
            let computed_entry = compute_object_entry(&object.path, &object.object_type, actual);
            match parse_digest(&computed_entry.digest) {
                Ok(digest) => digest,
                Err(err) => {
                    failures.push(err);
                    continue;
                }
            }
        };

        if !digests_equal(&expected, &computed) {
            failures.push(OsdfError::Integrity(format!(
                "digest mismatch for `{}`",
                object.path
            )));
        }
    }

    if let Some(manifest_bytes) = container.get(MANIFEST_PATH) {
        match serde_json::from_slice::<PackageManifest>(manifest_bytes) {
            Ok(parsed_manifest) => {
                if !digest_strings_equal(
                    &compute_manifest_digest(&parsed_manifest),
                    &manifest.manifest_digest,
                ) {
                    failures.push(OsdfError::Integrity("manifestDigest mismatch".to_string()));
                }
            }
            Err(err) => failures.push(OsdfError::Manifest(format!(
                "manifest JSON invalid: {err}"
            ))),
        }
    } else {
        failures.push(OsdfError::Integrity(format!("missing `{MANIFEST_PATH}`")));
    }

    let mut allowed = manifest
        .objects
        .iter()
        .map(|object| object.path.as_str())
        .collect::<std::collections::HashSet<_>>();
    allowed.insert(MANIFEST_PATH);

    for path in [HEADER_PATH] {
        if !allowed.contains(path) {
            failures.push(OsdfError::Integrity(format!(
                "required object `{path}` must appear in signed manifest objects[]"
            )));
        }
    }

    if container.get(ENVELOPE_PATH).is_none() {
        failures.push(OsdfError::Integrity(format!(
            "missing required `{ENVELOPE_PATH}`"
        )));
    }
    allowed.insert(ENVELOPE_PATH);

    for path in container.paths() {
        if !allowed.contains(path.as_str()) {
            failures.push(OsdfError::Integrity(format!(
                "undeclared package object: {path}"
            )));
        }
    }

    let computed_root = merkle_root(&manifest.objects);
    match parse_digest(&manifest.revision_root_hash) {
        Ok(declared_root) => {
            if !digests_equal(&computed_root, &declared_root) {
                failures.push(OsdfError::Integrity(
                    "revision Merkle root mismatch".to_string(),
                ));
            }
        }
        Err(err) => failures.push(err),
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.into_iter().next().expect("failures non-empty"))
    }
}

fn looks_like_json_object_type(object_type: &str, path: &str) -> bool {
    object_type.contains("json")
        || object_type.contains("manifest")
        || object_type.contains("envelope")
        || object_type.contains("record")
        || object_type.contains("signature")
        || object_type.contains("tree")
        || path.ends_with(".json")
}

pub fn content_bytes(manifest: &PackageManifest) -> u64 {
    manifest
        .objects
        .iter()
        .filter(|object| object.path.starts_with("content/"))
        .map(|object| object.bytes)
        .sum()
}
