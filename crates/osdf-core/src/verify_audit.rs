use std::collections::{BTreeMap, HashSet};
use std::io::Read;

use zip::ZipArchive;

use crate::constants::{
    suspicious_compression_ratio, HEADER_PATH, HEADER_SIZE, MAGIC, MAX_ENTRIES,
    MAX_UNCOMPRESSED_BYTES,
};
use crate::container::{normalize_zip_path, parse_header_bytes, PackageContainer, PackageEntry};
use crate::report::{finding_for_code, ReportBuilder, VerificationStatus};

pub fn trailing_bytes_after_zip(data: &[u8]) -> usize {
    for index in (0..data.len().saturating_sub(22)).rev() {
        if !data[index..].starts_with(b"PK\x05\x06") {
            continue;
        }
        if index + 22 > data.len() {
            continue;
        }
        let comment_len = u16::from_le_bytes([data[index + 20], data[index + 21]]) as usize;
        let end = index + 22 + comment_len;
        return data.len().saturating_sub(end);
    }
    0
}

pub fn audit_and_read_container(
    data: &[u8],
    builder: &mut ReportBuilder,
) -> Option<PackageContainer> {
    builder.begin_section(crate::report::VerificationSection::Container);

    let archive_bytes = data.len() as u64;
    let cursor = std::io::Cursor::new(data);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(archive) => archive,
        Err(err) => {
            builder.check(
                "OSDF_CONTAINER_ZIP_VALID",
                "ZIP archive readable",
                VerificationStatus::Fail,
                Some(err.to_string()),
            );
            record_container_failure(builder, "OSDF_ZIP_ERROR", err.to_string());
            return None;
        }
    };

    builder.check(
        "OSDF_CONTAINER_ZIP_VALID",
        "ZIP archive readable",
        VerificationStatus::Pass,
        None,
    );

    if archive.len() > MAX_ENTRIES {
        record_container_failure(
            builder,
            "OSDF_CONTAINER_ERROR",
            format!("too many entries: {} (max {MAX_ENTRIES})", archive.len()),
        );
        return None;
    }

    let mut entries = BTreeMap::new();
    let mut seen_paths = HashSet::new();
    let mut seen_folded = HashSet::new();
    let paths_safe = true;
    let duplicate_free = true;
    let mut total_uncompressed = 0u64;

    for index in 0..archive.len() {
        let mut file = match archive.by_index(index) {
            Ok(file) => file,
            Err(err) => {
                record_container_failure(builder, "OSDF_ZIP_ERROR", err.to_string());
                return None;
            }
        };

        let raw_name = file.name().to_string();
        let normalized = match normalize_zip_path(&raw_name) {
            Ok(path) => path,
            Err(err) => {
                let (code, technical) = crate::report::message_from_error(&err);
                record_container_failure(builder, &code, technical);
                return None;
            }
        };

        if !seen_paths.insert(normalized.clone()) {
            record_container_failure(
                builder,
                "OSDF_CONTAINER_DUPLICATE_PATH",
                format!("duplicate path: {normalized}"),
            );
            return None;
        }

        let folded = normalized.to_ascii_lowercase();
        if !seen_folded.insert(folded) {
            record_container_failure(
                builder,
                "OSDF_CONTAINER_CASE_COLLISION",
                format!("case-insensitive path collision: {normalized}"),
            );
            return None;
        }

        if file.is_dir() {
            continue;
        }

        let compression_ratio = if file.compressed_size() == 0 {
            1
        } else {
            file.size().saturating_div(file.compressed_size())
        };
        if suspicious_compression_ratio(file.size(), file.compressed_size()) {
            record_container_failure(
                builder,
                "OSDF_CONTAINER_COMPRESSION_BOMB",
                format!("compression bomb suspected at `{normalized}` (ratio {compression_ratio})"),
            );
            return None;
        }

        total_uncompressed = total_uncompressed.saturating_add(file.size());
        if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
            record_container_failure(
                builder,
                "OSDF_CONTAINER_OVERSIZED_OBJECT",
                format!("uncompressed size exceeds limit ({MAX_UNCOMPRESSED_BYTES} bytes)"),
            );
            return None;
        }

        let mut bytes = Vec::with_capacity(file.size() as usize);
        if let Err(err) = file.read_to_end(&mut bytes) {
            record_container_failure(builder, "OSDF_ZIP_ERROR", err.to_string());
            return None;
        }

        if bytes.len() as u64 != file.size() {
            record_container_failure(
                builder,
                "OSDF_CONTAINER_ERROR",
                format!("size mismatch for `{normalized}`"),
            );
            return None;
        }

        entries.insert(
            normalized.clone(),
            PackageEntry {
                path: normalized,
                bytes,
            },
        );
    }

    builder.check(
        "OSDF_CONTAINER_PATHS_SAFE",
        "ZIP paths are safe",
        if paths_safe {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        None,
    );
    builder.check(
        "OSDF_CONTAINER_NO_DUPLICATE_PATHS",
        "No duplicate paths",
        if duplicate_free {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        None,
    );

    let trailing = trailing_bytes_after_zip(data);
    builder.check(
        "OSDF_CONTAINER_NO_TRAILING_BYTES",
        "No trailing bytes",
        if trailing == 0 {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        if trailing == 0 {
            None
        } else {
            Some(format!("{trailing} trailing byte(s) after ZIP end"))
        },
    );
    if trailing > 0 {
        record_container_failure(
            builder,
            "OSDF_CONTAINER_TRAILING_BYTES",
            format!("{trailing} trailing byte(s) after ZIP end"),
        );
    }

    let header_entry = entries.get(HEADER_PATH);
    let magic_ok = header_entry
        .map(|entry| {
            entry.bytes.len() >= MAGIC.len() && &entry.bytes[..MAGIC.len()] == MAGIC.as_slice()
        })
        .unwrap_or(false);
    builder.check(
        "OSDF_CONTAINER_MAGIC",
        "OSDF magic header recognized",
        if magic_ok {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        None,
    );

    let header = match header_entry.map(|entry| parse_header_bytes(&entry.bytes)) {
        Some(Ok(header)) => Some(header),
        Some(Err(err)) => {
            let (code, technical) = crate::report::message_from_error(&err);
            record_container_failure(builder, &code, technical);
            return None;
        }
        None => {
            record_container_failure(
                builder,
                "OSDF_CONTAINER_MISSING_HEADER",
                format!("missing required header `{HEADER_PATH}`"),
            );
            return None;
        }
    };

    let header = header?;
    let length_ok = header.package_bytes == archive_bytes;
    builder.check(
        "OSDF_HEADER_PACKAGE_BYTES",
        "Container byte length matches",
        if length_ok {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        if length_ok {
            None
        } else {
            Some(format!(
                "expected {}, got {}",
                header.package_bytes, archive_bytes
            ))
        },
    );
    if !length_ok {
        record_container_failure(
            builder,
            "OSDF_HEADER_PACKAGE_BYTES",
            format!(
                "header packageBytes mismatch: expected {}, got {}",
                header.package_bytes, archive_bytes
            ),
        );
    }

    let _ = HEADER_SIZE;
    Some(PackageContainer {
        entries,
        archive_bytes,
    })
}

fn record_container_failure(builder: &mut ReportBuilder, code: &str, technical: String) {
    let (summary, impact, severity) = finding_for_code(code, &technical);
    builder.finding(code, severity, summary, impact, technical);
}

pub fn audit_manifest(
    container: &PackageContainer,
    builder: &mut ReportBuilder,
) -> Option<crate::types::PackageManifest> {
    use crate::constants::{ENVELOPE_PATH, MANIFEST_PATH};
    use crate::crypto::{digest_strings_equal, digests_equal, parse_digest};
    use crate::manifest::{compute_manifest_digest, compute_object_entry, parse_manifest};
    use crate::merkle::merkle_root;

    builder.begin_section(crate::report::VerificationSection::Manifest);

    let manifest = match parse_manifest(container) {
        Ok(manifest) => manifest,
        Err(err) => {
            builder.check(
                "OSDF_MANIFEST_PARSED",
                "Manifest parsed",
                VerificationStatus::Fail,
                Some(err.to_string()),
            );
            let (code, technical) = crate::report::message_from_error(&err);
            crate::report::record_error(builder, &code, technical);
            return None;
        }
    };

    builder.check(
        "OSDF_MANIFEST_PARSED",
        "Manifest parsed",
        VerificationStatus::Pass,
        None,
    );
    builder.document_id(manifest.document_id.clone());
    builder.revision(manifest.revision as u64);

    let mut declared_present = true;
    let mut sizes_match = true;
    let mut hashes_match = true;

    for object in &manifest.objects {
        if object.path == MANIFEST_PATH {
            declared_present = false;
            crate::report::record_error(
                builder,
                "OSDF_MANIFEST_ERROR",
                "manifest must not include itself in objects[] (use manifestDigest)",
            );
            continue;
        }

        let Some(actual) = container.get(&object.path) else {
            declared_present = false;
            crate::report::record_error(
                builder,
                "OSDF_MANIFEST_MISSING_OBJECT",
                format!("declared object missing: {}", object.path),
            );
            continue;
        };

        if actual.len() as u64 != object.bytes {
            sizes_match = false;
            crate::report::record_error(
                builder,
                "OSDF_MANIFEST_SIZE_MISMATCH",
                format!(
                    "byte length mismatch for `{}`: expected {}, got {}",
                    object.path,
                    object.bytes,
                    actual.len()
                ),
            );
        }

        let expected = match parse_digest(&object.digest) {
            Ok(digest) => digest,
            Err(err) => {
                hashes_match = false;
                crate::report::record_error(
                    builder,
                    "OSDF_MANIFEST_HASH_MISMATCH",
                    err.to_string(),
                );
                continue;
            }
        };
        let computed_entry = compute_object_entry(&object.path, &object.object_type, actual);
        let computed = match parse_digest(&computed_entry.digest) {
            Ok(digest) => digest,
            Err(err) => {
                hashes_match = false;
                crate::report::record_error(
                    builder,
                    "OSDF_MANIFEST_HASH_MISMATCH",
                    err.to_string(),
                );
                continue;
            }
        };
        if !digests_equal(&expected, &computed) {
            hashes_match = false;
            crate::report::record_error(
                builder,
                "OSDF_MANIFEST_HASH_MISMATCH",
                format!("digest mismatch for `{}`", object.path),
            );
        }
    }

    builder.check(
        "OSDF_MANIFEST_DECLARED_OBJECTS",
        "Declared objects present",
        if declared_present {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        None,
    );
    builder.check(
        "OSDF_MANIFEST_SIZE_MATCH",
        "Object sizes match",
        if sizes_match {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        None,
    );
    builder.check(
        "OSDF_MANIFEST_HASH_MATCH",
        "Object hashes match",
        if hashes_match {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        None,
    );

    let mut undeclared_free = true;
    let mut allowed = manifest
        .objects
        .iter()
        .map(|object| object.path.as_str())
        .collect::<HashSet<_>>();
    allowed.insert(MANIFEST_PATH);
    allowed.insert(ENVELOPE_PATH);

    for path in container.paths() {
        if !allowed.contains(path.as_str()) {
            undeclared_free = false;
            crate::report::record_error(
                builder,
                "OSDF_MANIFEST_UNDECLARED_OBJECT",
                format!("undeclared package object: {path}"),
            );
        }
    }

    builder.check(
        "OSDF_MANIFEST_NO_UNDECLARED_OBJECTS",
        "No undeclared objects",
        if undeclared_free {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        None,
    );

    let manifest_bytes = container.get(MANIFEST_PATH).unwrap_or_default();
    if let Ok(parsed_manifest) =
        serde_json::from_slice::<crate::types::PackageManifest>(manifest_bytes)
    {
        if !digest_strings_equal(
            &compute_manifest_digest(&parsed_manifest),
            &manifest.manifest_digest,
        ) {
            crate::report::record_error(
                builder,
                "OSDF_MANIFEST_HASH_MISMATCH",
                "manifestDigest mismatch",
            );
        }
    }

    let computed_root = merkle_root(&manifest.objects);
    if let Ok(declared_root) = parse_digest(&manifest.revision_root_hash) {
        if !digests_equal(&computed_root, &declared_root) {
            crate::report::record_error(
                builder,
                "OSDF_MANIFEST_HASH_MISMATCH",
                "revision Merkle root mismatch",
            );
        }
    }

    Some(manifest)
}

pub fn audit_revision_chain(
    container: &PackageContainer,
    manifest: &crate::types::PackageManifest,
    builder: &mut ReportBuilder,
) {
    builder.begin_section(crate::report::VerificationSection::Revision);

    match crate::revision::verify_revision_chain(container) {
        Ok(()) => {
            builder.check(
                "OSDF_REVISION_CHAIN",
                "Revision chain valid",
                VerificationStatus::Pass,
                None,
            );
            builder.check(
                "OSDF_REVISION_PARENT_COMMITMENT",
                "Parent revision commitment valid",
                VerificationStatus::Pass,
                if manifest.parent_revision_commitment.is_some() {
                    None
                } else {
                    Some("not applicable for this revision".to_string())
                },
            );
            builder.check(
                "OSDF_REVISION_METADATA",
                "Current revision metadata valid",
                VerificationStatus::Pass,
                None,
            );
        }
        Err(err) => {
            let (code, technical) = crate::report::message_from_error(&err);
            let check_code = if code.contains("PARENT") {
                "OSDF_REVISION_PARENT_COMMITMENT"
            } else {
                "OSDF_REVISION_CHAIN"
            };
            builder.check(
                check_code,
                if check_code.contains("PARENT") {
                    "Parent revision commitment valid"
                } else {
                    "Revision chain valid"
                },
                VerificationStatus::Fail,
                Some(technical.clone()),
            );
            builder.check(
                "OSDF_REVISION_METADATA",
                "Current revision metadata valid",
                VerificationStatus::Fail,
                Some(technical.clone()),
            );
            crate::report::record_error(builder, &code, technical);
        }
    }
}

pub fn audit_signatures(
    container: &PackageContainer,
    manifest: &crate::types::PackageManifest,
    builder: &mut ReportBuilder,
    identity: &crate::identity::IdentityConfig,
) {
    use crate::signature::verify_signatures;

    builder.begin_section(crate::report::VerificationSection::Signatures);

    if manifest.revision == 0 {
        builder.check(
            "OSDF_SIGNATURE_STRUCTURE",
            "Signature structure valid",
            VerificationStatus::Info,
            Some("not required for revision 0".to_string()),
        );
        builder.check(
            "OSDF_SIGNATURE_CRYPTO",
            "Signature cryptographically valid",
            VerificationStatus::Info,
            Some("not required for revision 0".to_string()),
        );
        emit_identity_disabled_stub(builder, identity);
        builder.signature_count(0);
        return;
    }

    match verify_signatures(container) {
        Ok(signatures) if signatures.is_empty() && manifest.revision > 0 => {
            builder.signature_count(0);
            builder.check(
                "OSDF_SIGNATURE_STRUCTURE",
                "Signature structure valid",
                VerificationStatus::Fail,
                Some("committed revision has no signature file".to_string()),
            );
            builder.check(
                "OSDF_SIGNATURE_CRYPTO",
                "Signature cryptographically valid",
                VerificationStatus::Fail,
                Some("committed revision has no valid signature".to_string()),
            );
            crate::report::record_error(
                builder,
                "OSDF_SIGNATURE_MISSING",
                "committed revision has no valid signature",
            );
        }
        Ok(signatures) => {
            builder.signature_count(signatures.len() as u64);
            builder.check(
                "OSDF_SIGNATURE_STRUCTURE",
                "Signature structure valid",
                VerificationStatus::Pass,
                None,
            );
            builder.check(
                "OSDF_SIGNATURE_CRYPTO",
                "Signature cryptographically valid",
                VerificationStatus::Pass,
                Some(format!("{} signature(s) verified", signatures.len())),
            );
            audit_signer_identity(container, manifest, builder, identity, &signatures);
        }
        Err(err) => {
            builder.signature_count(0);
            let (code, technical) = crate::report::message_from_error(&err);
            builder.check(
                "OSDF_SIGNATURE_STRUCTURE",
                "Signature structure valid",
                VerificationStatus::Fail,
                Some(technical.clone()),
            );
            builder.check(
                "OSDF_SIGNATURE_CRYPTO",
                "Signature cryptographically valid",
                VerificationStatus::Fail,
                Some(technical.clone()),
            );
            crate::report::record_error(builder, &code, technical);
        }
    }
}

fn audit_signer_identity(
    container: &PackageContainer,
    manifest: &crate::types::PackageManifest,
    builder: &mut ReportBuilder,
    identity: &crate::identity::IdentityConfig,
    signatures: &[crate::types::SignatureEnvelope],
) {
    use crate::identity::{resolve_signer_identity, IdentityPolicy};
    use crate::report::{finding_for_code, ResolvedSignerIdentityReport, VerificationStatus};
    use crate::revision::parse_revision;

    if identity.policy == IdentityPolicy::Disabled {
        emit_identity_disabled_stub(builder, identity);
        return;
    }

    let signing_timestamp = parse_revision(container, manifest.revision)
        .ok()
        .map(|record| record.committed_timestamp)
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    let current_signature = signatures
        .iter()
        .find(|signature| signature.revision == manifest.revision)
        .or_else(|| signatures.last());

    let Some(signature) = current_signature else {
        emit_identity_disabled_stub(builder, identity);
        return;
    };

    match resolve_signer_identity(&signature.signer_key, &signing_timestamp, identity) {
        Ok(Some(resolved)) => {
            let detail = match resolved.department.as_deref() {
                Some(department) => format!("{} · {}", resolved.display_name, department),
                None => resolved.display_name.clone(),
            };
            builder.check(
                "OSDF_IDENTITY_RESOLVED",
                "Signer identity resolved",
                VerificationStatus::Pass,
                Some(detail),
            );
            if resolved.resolution_path == "organization-signing-delegation" {
                builder.check(
                    "OSDF_IDENTITY_DELEGATION_VALID",
                    "Signing delegation valid at signing time",
                    VerificationStatus::Pass,
                    resolved.department.clone(),
                );
            }
            builder.signer_identity(ResolvedSignerIdentityReport {
                signer_key: resolved.signer_key,
                organization_id: resolved.organization_id,
                display_name: resolved.display_name,
                department: resolved.department,
                resolution_path: resolved.resolution_path,
                signing_timestamp: resolved.signing_timestamp,
            });
        }
        Ok(None) => {
            if identity.policy == IdentityPolicy::Required {
                builder.check(
                    "OSDF_IDENTITY_UNTRUSTED",
                    "Signer identity not trusted",
                    VerificationStatus::Fail,
                    Some(signature.signer_key.clone()),
                );
                let (summary, impact, severity) =
                    finding_for_code("OSDF_IDENTITY_UNTRUSTED", &signature.signer_key);
                builder.finding(
                    "OSDF_IDENTITY_UNTRUSTED",
                    severity,
                    summary,
                    impact,
                    format!(
                        "signer key not in configured identity registry: {}",
                        signature.signer_key
                    ),
                );
            } else {
                builder.check(
                    "OSDF_IDENTITY_NOT_RESOLVED",
                    "Signer identity not yet resolved",
                    VerificationStatus::Info,
                    Some(signature.signer_key.clone()),
                );
                let (summary, impact, severity) =
                    finding_for_code("OSDF_IDENTITY_NOT_RESOLVED", &signature.signer_key);
                builder.finding(
                    "OSDF_IDENTITY_NOT_RESOLVED",
                    severity,
                    summary,
                    impact,
                    format!(
                        "signer key not in configured identity registry: {}",
                        signature.signer_key
                    ),
                );
            }
        }
        Err(err) => {
            let (code, technical) = crate::report::message_from_error(&err);
            builder.check(
                "OSDF_IDENTITY_UNTRUSTED",
                "Signer identity not trusted",
                VerificationStatus::Fail,
                Some(technical.clone()),
            );
            crate::report::record_error(builder, &code, technical);
        }
    }
}

fn emit_identity_disabled_stub(
    builder: &mut ReportBuilder,
    identity: &crate::identity::IdentityConfig,
) {
    use crate::identity::IdentityPolicy;
    use crate::report::{finding_for_code, VerificationStatus};

    if identity.policy != IdentityPolicy::Disabled {
        return;
    }

    builder.check(
        "OSDF_IDENTITY_NOT_RESOLVED",
        "Signer identity not yet resolved",
        VerificationStatus::Info,
        None,
    );
    let (summary, impact, severity) = finding_for_code(
        "OSDF_IDENTITY_NOT_RESOLVED",
        "signer identity lookup not configured",
    );
    builder.finding(
        "OSDF_IDENTITY_NOT_RESOLVED",
        severity,
        summary,
        impact,
        "signer identity lookup not configured".to_string(),
    );
}

pub fn audit_transparency(
    builder: &mut ReportBuilder,
    container: Option<&PackageContainer>,
    manifest: Option<&crate::types::PackageManifest>,
    ledger: &crate::ledger::LedgerConfig,
) {
    use crate::ledger::{
        parse_transparency_proof, revision_event_hash_for, verify_transparency_proof, LedgerPolicy,
    };
    use crate::report::{finding_for_code, VerificationStatus};

    builder.begin_section(crate::report::VerificationSection::Transparency);

    if ledger.policy == LedgerPolicy::Disabled {
        builder.check(
            "OSDF_LEDGER_NOT_CONFIGURED",
            "Ledger verification not configured",
            VerificationStatus::Info,
            None,
        );
        builder.check(
            "OSDF_REVOCATION_NOT_CONFIGURED",
            "Revocation checking not configured",
            VerificationStatus::Info,
            None,
        );
        for code in [
            "OSDF_LEDGER_NOT_CONFIGURED",
            "OSDF_REVOCATION_NOT_CONFIGURED",
        ] {
            let (summary, impact, severity) = finding_for_code(code, code);
            builder.finding(code, severity, summary, impact, code.to_string());
        }
        return;
    }

    let Some(container) = container else {
        builder.check(
            "OSDF_LEDGER_PROOF_PRESENT",
            "Ledger proof present",
            VerificationStatus::Fail,
            Some("package could not be read".to_string()),
        );
        crate::report::record_error(
            builder,
            "OSDF_LEDGER_PROOF_MISSING",
            "ledger verification requested but package is unavailable",
        );
        emit_revocation_info(builder);
        return;
    };

    let Some(manifest) = manifest else {
        builder.check(
            "OSDF_LEDGER_PROOF_PRESENT",
            "Ledger proof present",
            VerificationStatus::Fail,
            Some("manifest unavailable".to_string()),
        );
        crate::report::record_error(
            builder,
            "OSDF_LEDGER_PROOF_MISSING",
            "ledger verification requested but manifest is unavailable",
        );
        emit_revocation_info(builder);
        return;
    };

    if manifest.revision == 0 {
        builder.check(
            "OSDF_LEDGER_PROOF_PRESENT",
            "Ledger proof present",
            VerificationStatus::Info,
            Some("not required for revision 0".to_string()),
        );
        emit_revocation_info(builder);
        return;
    }

    match parse_transparency_proof(container, manifest.revision) {
        Ok(proof) => {
            builder.check(
                "OSDF_LEDGER_PROOF_PRESENT",
                "Ledger proof present",
                VerificationStatus::Pass,
                Some(proof.log_entry_id.clone()),
            );

            let expected_hash = match revision_event_hash_for(container, manifest.revision) {
                Ok(value) => value,
                Err(err) => {
                    let (code, technical) = crate::report::message_from_error(&err);
                    fail_ledger_checks(builder, &code, technical);
                    emit_revocation_info(builder);
                    return;
                }
            };

            let leaf_ok = proof.revision_event_hash == expected_hash;
            builder.check(
                "OSDF_LEDGER_LEAF_MATCHES_EVENT_HASH",
                "Ledger leaf matches revision event hash",
                if leaf_ok {
                    VerificationStatus::Pass
                } else {
                    VerificationStatus::Fail
                },
                if leaf_ok {
                    None
                } else {
                    Some(format!(
                        "expected {expected_hash}, got {}",
                        proof.revision_event_hash
                    ))
                },
            );
            if !leaf_ok {
                crate::report::record_error(
                    builder,
                    "OSDF_LEDGER_LEAF_MISMATCH",
                    format!(
                        "proof revisionEventHash does not match revision record: expected {expected_hash}"
                    ),
                );
            }

            match verify_transparency_proof(&proof, &expected_hash, ledger) {
                Ok(()) => {
                    builder.check(
                        "OSDF_LEDGER_INCLUSION_PROOF_VALID",
                        "Ledger inclusion proof valid",
                        VerificationStatus::Pass,
                        None,
                    );
                    builder.check(
                        "OSDF_LEDGER_SIGNED_ROOT_VALID",
                        "Ledger signed tree head valid",
                        VerificationStatus::Pass,
                        None,
                    );
                    builder.check(
                        "OSDF_LEDGER_LOG_KEY_TRUSTED",
                        "Ledger log key trusted",
                        VerificationStatus::Pass,
                        Some(proof.signed_tree_head.log_key_reference.clone()),
                    );
                    emit_ledger_freshness_stubs(builder, &proof, ledger);
                }
                Err(err) => {
                    let (code, technical) = crate::report::message_from_error(&err);
                    if code.contains("INCLUSION") || technical.contains("inclusion") {
                        builder.check(
                            "OSDF_LEDGER_INCLUSION_PROOF_VALID",
                            "Ledger inclusion proof valid",
                            VerificationStatus::Fail,
                            Some(technical.clone()),
                        );
                    } else if code.contains("SIGNATURE") || technical.contains("tree head") {
                        builder.check(
                            "OSDF_LEDGER_SIGNED_ROOT_VALID",
                            "Ledger signed tree head valid",
                            VerificationStatus::Fail,
                            Some(technical.clone()),
                        );
                    } else if code.contains("key") || technical.contains("trusted") {
                        builder.check(
                            "OSDF_LEDGER_LOG_KEY_TRUSTED",
                            "Ledger log key trusted",
                            VerificationStatus::Fail,
                            Some(technical.clone()),
                        );
                    } else {
                        builder.check(
                            "OSDF_LEDGER_INCLUSION_PROOF_VALID",
                            "Ledger inclusion proof valid",
                            VerificationStatus::Fail,
                            Some(technical.clone()),
                        );
                    }
                    crate::report::record_error(builder, &code, technical);
                }
            }
        }
        Err(err) => {
            builder.check(
                "OSDF_LEDGER_PROOF_PRESENT",
                "Ledger proof present",
                VerificationStatus::Fail,
                Some(err.to_string()),
            );
            if ledger.policy == LedgerPolicy::Required {
                crate::report::record_error(builder, "OSDF_LEDGER_PROOF_MISSING", err.to_string());
            }
        }
    }

    emit_revocation_info(builder);
}

pub fn audit_verification_context(
    builder: &mut ReportBuilder,
    config: &crate::identity::VerifierConfig,
) {
    use crate::ledger::LedgerPolicy;
    use crate::report::VerificationStatus;

    builder.begin_section(crate::report::VerificationSection::VerificationContext);
    builder.verification_mode(crate::report::VerificationMode::OfflineCryptographic);

    emit_info_stub(
        builder,
        "OSDF_VERIFICATION_MODE_OFFLINE",
        "Offline cryptographic verification",
        Some("embedded package data and configured trust material only".to_string()),
    );

    if config.ledger.policy != LedgerPolicy::Disabled
        && builder.has_passing_check("OSDF_LEDGER_INCLUSION_PROOF_VALID")
    {
        builder.check(
            "OSDF_LEDGER_EMBEDDED_PROOF_VALID",
            "Embedded ledger inclusion proof valid",
            VerificationStatus::Pass,
            None,
        );
    }

    let latest_revision_checked = builder.has_check("OSDF_LATEST_REVISION_CONFIRMED")
        || builder.has_check("OSDF_LATEST_REVISION_OUTDATED")
        || builder.has_check("OSDF_LATEST_REVISION_REGISTRY_UNAVAILABLE");

    if !latest_revision_checked {
        emit_info_stub(
            builder,
            "OSDF_LIVE_LATEST_REVISION_NOT_CHECKED",
            "Live latest-revision check not performed",
            Some("offline verification — local revision not compared to a live ledger".to_string()),
        );
    }

    emit_info_stub(
        builder,
        "OSDF_LIVE_REVOCATION_NOT_CHECKED",
        "Revocation status not checked",
        None,
    );

    if !builder.has_passing_check("OSDF_IDENTITY_RESOLVED") {
        builder.check(
            "OSDF_VERIFICATION_IDENTITY_UNRESOLVED",
            "Signer identity not yet resolved",
            VerificationStatus::Info,
            None,
        );
    }
}

fn emit_ledger_freshness_stubs(
    builder: &mut ReportBuilder,
    proof: &crate::types::TransparencyProof,
    ledger: &crate::ledger::LedgerConfig,
) {
    use crate::ledger::LatestRevisionPolicy;

    emit_info_stub(
        builder,
        "OSDF_LEDGER_TREE_HEAD_FRESHNESS_NOT_CHECKED",
        "Tree-head freshness not evaluated",
        Some(format!(
            "embedded checkpoint timestamp: {}",
            proof.signed_tree_head.timestamp
        )),
    );
    emit_info_stub(
        builder,
        "OSDF_LEDGER_CONSISTENCY_PROOF_NOT_CHECKED",
        "Consistency proof not evaluated",
        None,
    );
    if ledger.latest_revision_policy == LatestRevisionPolicy::Disabled {
        emit_info_stub(
            builder,
            "OSDF_LEDGER_LATEST_REVISION_NOT_CHECKED",
            "Latest-revision lookup not performed",
            Some("offline verification — inclusion proves logging, not currentness".to_string()),
        );
    }
}

pub fn audit_latest_revision(
    builder: &mut ReportBuilder,
    container: &PackageContainer,
    manifest: &crate::types::PackageManifest,
    ledger: &crate::ledger::LedgerConfig,
) {
    use crate::ledger::{lookup_latest_revision, revision_event_hash_for, LatestRevisionPolicy};
    use crate::report::{finding_for_code, VerificationStatus};

    if ledger.latest_revision_policy == LatestRevisionPolicy::Disabled {
        return;
    }

    if manifest.revision == 0 {
        return;
    }

    let local_hash = match revision_event_hash_for(container, manifest.revision) {
        Ok(value) => value,
        Err(err) => {
            let (code, technical) = crate::report::message_from_error(&err);
            builder.check(
                "OSDF_LATEST_REVISION_REGISTRY_UNAVAILABLE",
                "Latest-revision registry unavailable",
                VerificationStatus::Warning,
                Some(technical.clone()),
            );
            crate::report::record_error(builder, &code, technical);
            return;
        }
    };

    let Some(registry_entry) = lookup_latest_revision(ledger, &manifest.document_id) else {
        let status = match ledger.latest_revision_policy {
            LatestRevisionPolicy::Required => VerificationStatus::Warning,
            LatestRevisionPolicy::Optional | LatestRevisionPolicy::Disabled => {
                VerificationStatus::Info
            }
        };
        builder.check(
            "OSDF_LATEST_REVISION_REGISTRY_UNAVAILABLE",
            "Latest-revision registry unavailable",
            status,
            Some(format!(
                "no registry entry for document `{}`",
                manifest.document_id
            )),
        );
        let (summary, impact, severity) = finding_for_code(
            "OSDF_LATEST_REVISION_REGISTRY_UNAVAILABLE",
            "missing registry entry",
        );
        builder.finding(
            "OSDF_LATEST_REVISION_REGISTRY_UNAVAILABLE",
            severity,
            summary,
            impact,
            format!("no registry entry for document `{}`", manifest.document_id),
        );
        return;
    };

    let is_current = registry_entry.revision == manifest.revision
        && registry_entry.revision_event_hash == local_hash;

    if is_current {
        builder.check(
            "OSDF_LATEST_REVISION_CONFIRMED",
            "Latest revision confirmed",
            VerificationStatus::Pass,
            Some(format!(
                "revision {} matches ledger registry",
                manifest.revision
            )),
        );
        let (summary, impact, severity) = finding_for_code(
            "OSDF_LATEST_REVISION_CONFIRMED",
            "latest revision confirmed",
        );
        builder.finding(
            "OSDF_LATEST_REVISION_CONFIRMED",
            severity,
            summary,
            impact,
            format!("revision {} is current", manifest.revision),
        );
        return;
    }

    builder.check(
        "OSDF_LATEST_REVISION_OUTDATED",
        "Authentic but outdated revision",
        VerificationStatus::Warning,
        Some(format!(
            "package revision {} ({local_hash}) — registry latest revision {} ({})",
            manifest.revision, registry_entry.revision, registry_entry.revision_event_hash
        )),
    );
    let (summary, impact, severity) =
        finding_for_code("OSDF_LATEST_REVISION_OUTDATED", "outdated revision");
    builder.finding(
        "OSDF_LATEST_REVISION_OUTDATED",
        severity,
        summary,
        impact,
        format!(
            "registry latest revision {} supersedes package revision {}",
            registry_entry.revision, manifest.revision
        ),
    );
}

fn emit_info_stub(builder: &mut ReportBuilder, code: &str, label: &str, details: Option<String>) {
    use crate::report::{finding_for_code, VerificationStatus};

    builder.check(code, label, VerificationStatus::Info, details.clone());
    let (summary, impact, severity) = finding_for_code(code, code);
    builder.finding(code, severity, summary, impact, label.to_string());
}

fn fail_ledger_checks(builder: &mut ReportBuilder, code: &str, technical: String) {
    use crate::report::VerificationStatus;

    builder.check(
        "OSDF_LEDGER_LEAF_MATCHES_EVENT_HASH",
        "Ledger leaf matches revision event hash",
        VerificationStatus::Fail,
        Some(technical.clone()),
    );
    builder.check(
        "OSDF_LEDGER_INCLUSION_PROOF_VALID",
        "Ledger inclusion proof valid",
        VerificationStatus::Fail,
        Some(technical.clone()),
    );
    crate::report::record_error(builder, code, technical);
}

fn emit_revocation_info(builder: &mut ReportBuilder) {
    use crate::report::{finding_for_code, VerificationStatus};

    builder.check(
        "OSDF_REVOCATION_NOT_CONFIGURED",
        "Revocation checking not configured",
        VerificationStatus::Info,
        None,
    );
    let (summary, impact, severity) = finding_for_code(
        "OSDF_REVOCATION_NOT_CONFIGURED",
        "revocation checking not configured",
    );
    builder.finding(
        "OSDF_REVOCATION_NOT_CONFIGURED",
        severity,
        summary,
        impact,
        "revocation checking not configured".to_string(),
    );
}
