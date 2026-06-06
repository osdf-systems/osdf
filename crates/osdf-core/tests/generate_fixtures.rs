//! Generate committed fixture files under `fixtures/`.
//!
//! Run with: `cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored`

use std::io::Read;

use osdf_core::manifest::parse_manifest;
use osdf_core::types::DocumentNode;
use osdf_core::{
    append_revision_to_store, attach_transparency_proof, build_delegation_credential,
    build_proof_for_store, commit_revision, create_ledger_store, create_package,
    create_package_with_document, generate_signing_key, sign_delegation_credential,
    trust_config_for_store, verify_package_bytes, verify_package_bytes_with_config,
    verifying_key_to_urn, CommitOptions, CreateOptions, IdentityConfig, IdentityPolicy,
    LedgerPolicy, PackageContainer, TrustedOrganization, VerificationStatus, VerifierConfig,
};

#[test]
#[ignore]
fn write_fixtures() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    std::fs::create_dir_all(root.join("valid")).unwrap();
    std::fs::create_dir_all(root.join("invalid")).unwrap();

    let draft = create_package(CreateOptions {
        title: "Valid Draft".to_string(),
        ..Default::default()
    })
    .unwrap();
    draft
        .write_to_path(&root.join("valid/valid-draft.osdf"))
        .unwrap();

    let committed = create_package(CreateOptions {
        title: "Valid Committed".to_string(),
        commit: true,
        ..Default::default()
    })
    .unwrap();
    committed
        .write_to_path(&root.join("valid/valid-committed.osdf"))
        .unwrap();

    let org_root_key = generate_signing_key();
    let delegate_key = generate_signing_key();
    let identity_committed = create_package(CreateOptions {
        title: "Valid With Identity".to_string(),
        commit: true,
        signing_key: Some(delegate_key.clone()),
        ..Default::default()
    })
    .unwrap();
    identity_committed
        .write_to_path(&root.join("valid/valid-with-identity.osdf"))
        .unwrap();
    let delegate_urn = verifying_key_to_urn(&delegate_key.verifying_key());
    let mut delegation = build_delegation_credential(
        "urn:osdf:org:colorado-state-demo",
        Some("Department of Revenue"),
        &delegate_urn,
        "2020-01-01T00:00:00Z",
        "2030-01-01T00:00:00Z",
    );
    sign_delegation_credential(&org_root_key, &mut delegation).unwrap();
    let identity_config = IdentityConfig {
        policy: IdentityPolicy::Required,
        organizations: vec![TrustedOrganization {
            organization_id: "urn:osdf:org:colorado-state-demo".to_string(),
            display_name: "State of Colorado".to_string(),
            root_keys: vec![verifying_key_to_urn(&org_root_key.verifying_key())],
        }],
        delegations: vec![delegation],
    };
    std::fs::write(
        root.join("valid/identity-trust.json"),
        serde_json::to_vec_pretty(&identity_config).unwrap(),
    )
    .unwrap();

    let log_key = generate_signing_key();
    let mut ledger_store = create_ledger_store(Some("urn:osdf:log:fixture".to_string()), &log_key);
    let committed_manifest = parse_manifest(&committed).unwrap();
    let event_hash = osdf_core::revision_event_hash_bytes(&committed, 1).unwrap();
    let leaf_index = append_revision_to_store(
        &mut ledger_store,
        &committed_manifest.document_id,
        committed_manifest.revision,
        event_hash,
    );
    let proof = build_proof_for_store(&ledger_store, leaf_index, event_hash, &log_key).unwrap();
    let mut with_ledger = committed.clone();
    attach_transparency_proof(&mut with_ledger, proof).unwrap();
    with_ledger
        .write_to_path(&root.join("valid/valid-with-ledger-proof.osdf"))
        .unwrap();

    let mut rev2 = committed.clone();
    commit_revision(
        &mut rev2,
        CommitOptions {
            signing_key: generate_signing_key(),
            signer_key_reference: None,
        },
    )
    .unwrap();
    let rev2_manifest = parse_manifest(&rev2).unwrap();
    let event_hash2 = osdf_core::revision_event_hash_bytes(&rev2, 2).unwrap();
    let leaf_index2 = append_revision_to_store(
        &mut ledger_store,
        &rev2_manifest.document_id,
        rev2_manifest.revision,
        event_hash2,
    );
    let proof2 = build_proof_for_store(&ledger_store, leaf_index2, event_hash2, &log_key).unwrap();
    attach_transparency_proof(&mut rev2, proof2).unwrap();
    rev2.write_to_path(&root.join("valid/valid-rev2-with-ledger-proof.osdf"))
        .unwrap();

    let trust = trust_config_for_store(&ledger_store, LedgerPolicy::Required);
    std::fs::write(
        root.join("valid/ledger-trust.json"),
        serde_json::to_vec_pretty(&trust).unwrap(),
    )
    .unwrap();

    write_ledger_invalid(
        &root,
        &ledger_store,
        &log_key,
        "ledger-leaf-mismatch.osdf",
        |proof| {
            proof.revision_event_hash = format!("sha256:{}", "aa".repeat(32));
        },
    );

    write_ledger_invalid(
        &root,
        &ledger_store,
        &log_key,
        "ledger-bad-inclusion.osdf",
        |proof| {
            if let Some(node) = proof.inclusion_proof.first_mut() {
                *node = format!("sha256:{}", "bb".repeat(32));
            }
        },
    );

    let tax_key = generate_signing_key();
    let tax_doc_id = "urn:osdf:doc:taxes-demo-2025";
    let template_bytes = std::fs::read(root.join("content/taxes-template.json")).unwrap();
    let taxes_template =
        create_package_with_document(tax_doc_id, &template_bytes, &tax_key).unwrap();
    taxes_template
        .write_to_path(&root.join("valid/taxes-template.osdf"))
        .unwrap();

    let filled_bytes = std::fs::read(root.join("content/taxes-filled.json")).unwrap();
    let mut taxes_filled = taxes_template.clone();
    taxes_filled
        .insert("content/document.json", filled_bytes)
        .unwrap();
    commit_revision(
        &mut taxes_filled,
        CommitOptions {
            signing_key: tax_key,
            signer_key_reference: None,
        },
    )
    .unwrap();
    taxes_filled
        .write_to_path(&root.join("valid/Taxes.osdf"))
        .unwrap();
    taxes_filled
        .write_to_path(&root.join("valid/taxes-filled.osdf"))
        .unwrap();

    write_invalid(
        &root,
        "missing-magic.osdf",
        |container| {
            let mut header = osdf_core::container::make_header_bytes(container.archive_bytes);
            header[..6].copy_from_slice(b"BADMAG");
            container.insert("osdf-header.bin", header).unwrap();
        },
        false,
    );

    write_invalid(
        &root,
        "undeclared-object.osdf",
        |container| {
            container
                .insert("secret/extra.bin", b"surprise".to_vec())
                .unwrap();
        },
        true,
    );

    write_invalid(
        &root,
        "missing-declared-object.osdf",
        |container| {
            container.remove("content/document.json");
        },
        true,
    );

    write_invalid(
        &root,
        "tampered-content-hash.osdf",
        |container| {
            if let Some(entry) = container.entries.get_mut("content/document.json") {
                let mut doc: DocumentNode = serde_json::from_slice(&entry.bytes).unwrap();
                if let Some(text) = doc
                    .children
                    .first_mut()
                    .and_then(|heading| heading.children.first_mut())
                {
                    text.value = Some("Tampered".to_string());
                }
                entry.bytes = serde_json::to_vec_pretty(&doc).unwrap();
            }
        },
        true,
    );

    write_invalid(
        &root,
        "tampered-signature.osdf",
        |container| {
            for path in container.paths().cloned().collect::<Vec<_>>() {
                if path.starts_with("signatures/") {
                    if let Some(entry) = container.entries.get_mut(&path) {
                        if !entry.bytes.is_empty() {
                            let last = entry.bytes.len() - 1;
                            entry.bytes[last] ^= 0xFF;
                        }
                    }
                }
            }
        },
        true,
    );

    write_invalid(
        &root,
        "missing-signature.osdf",
        |container| {
            container
                .entries
                .retain(|path, _| !path.starts_with("signatures/"));
        },
        true,
    );

    write_invalid(
        &root,
        "fake-parent-commitment.osdf",
        |container| {
            tamper_manifest_field(container, |manifest| {
                manifest.parent_revision_commitment = Some(format!("sha256:{}", "cc".repeat(32)));
            });
        },
        true,
    );

    write_invalid(
        &root,
        "deleted-parent-revision.osdf",
        |container| {
            container.remove("revisions/rev-000001.json");
        },
        true,
    );

    std::fs::write(
        root.join("invalid/duplicate-path.osdf"),
        build_duplicate_path_zip_python(&root.join("valid/valid-draft.osdf")),
    )
    .unwrap();

    std::fs::write(
        root.join("invalid/trailing-bytes.osdf"),
        append_bytes(&draft.to_bytes().unwrap(), b"TRAILER"),
    )
    .unwrap();

    std::fs::write(
        root.join("invalid/path-traversal.osdf"),
        build_zip_with_raw_path(&draft.to_bytes().unwrap(), "../escape.bin", b"bad"),
    )
    .unwrap();

    for name in [
        "valid/valid-draft.osdf",
        "valid/valid-committed.osdf",
        "valid/valid-with-identity.osdf",
        "valid/valid-with-ledger-proof.osdf",
        "valid/taxes-template.osdf",
        "valid/Taxes.osdf",
    ] {
        let bytes = std::fs::read(root.join(name)).unwrap();
        let report = verify_package_bytes(&bytes);
        assert_eq!(
            report.overall,
            VerificationStatus::Pass,
            "{name}: {:?}",
            report.findings
        );
    }

    let trust_bytes = std::fs::read(root.join("valid/ledger-trust.json")).unwrap();
    let trust: osdf_core::LedgerConfig = serde_json::from_slice(&trust_bytes).unwrap();
    let rev2_bytes = std::fs::read(root.join("valid/valid-rev2-with-ledger-proof.osdf")).unwrap();
    let rev2_report = verify_package_bytes_with_config(
        &rev2_bytes,
        &VerifierConfig {
            ledger: trust.clone(),
            ..VerifierConfig::default()
        },
    );
    assert_eq!(
        rev2_report.overall,
        VerificationStatus::Pass,
        "rev2 ledger fixture: {:?}",
        rev2_report.findings
    );
    assert!(rev2_report.checks.iter().any(|check| {
        check.code == "OSDF_LATEST_REVISION_CONFIRMED" && check.status == VerificationStatus::Pass
    }));

    let rev1_bytes = std::fs::read(root.join("valid/valid-with-ledger-proof.osdf")).unwrap();
    let rev1_report = verify_package_bytes_with_config(
        &rev1_bytes,
        &VerifierConfig {
            ledger: trust.clone(),
            ..VerifierConfig::default()
        },
    );
    assert_eq!(
        rev1_report.overall,
        VerificationStatus::Warning,
        "outdated rev1 ledger fixture: {:?}",
        rev1_report.findings
    );
    assert!(rev1_report
        .checks
        .iter()
        .any(|check| { check.code == "OSDF_LATEST_REVISION_OUTDATED" }));

    let identity_bytes = std::fs::read(root.join("valid/identity-trust.json")).unwrap();
    let identity: IdentityConfig = serde_json::from_slice(&identity_bytes).unwrap();
    let identity_package = std::fs::read(root.join("valid/valid-with-identity.osdf")).unwrap();
    let identity_report = verify_package_bytes_with_config(
        &identity_package,
        &VerifierConfig {
            identity,
            ..VerifierConfig::default()
        },
    );
    assert_eq!(
        identity_report.overall,
        VerificationStatus::Pass,
        "identity fixture: {:?}",
        identity_report.findings
    );
    assert!(identity_report
        .checks
        .iter()
        .any(|check| check.code == "OSDF_IDENTITY_RESOLVED"));

    let invalid_expectations: Vec<(&str, &[&str])> = vec![
        ("missing-magic.osdf", &["OSDF_CONTAINER_INVALID_MAGIC"]),
        (
            "undeclared-object.osdf",
            &["OSDF_MANIFEST_UNDECLARED_OBJECT"],
        ),
        (
            "duplicate-path.osdf",
            &["OSDF_CONTAINER_DUPLICATE_PATH", "OSDF_HEADER_PACKAGE_BYTES"],
        ),
        (
            "trailing-bytes.osdf",
            &["OSDF_CONTAINER_TRAILING_BYTES", "OSDF_HEADER_PACKAGE_BYTES"],
        ),
        (
            "tampered-content-hash.osdf",
            &["OSDF_MANIFEST_HASH_MISMATCH", "OSDF_MANIFEST_SIZE_MISMATCH"],
        ),
        (
            "tampered-signature.osdf",
            &[
                "OSDF_SIGNATURE_INVALID",
                "OSDF_MANIFEST_HASH_MISMATCH",
                "OSDF_JSON_ERROR",
            ],
        ),
        (
            "missing-signature.osdf",
            &[
                "OSDF_SIGNATURE_INVALID",
                "OSDF_SIGNATURE_MISSING",
                "OSDF_MANIFEST_MISSING_OBJECT",
            ],
        ),
        (
            "missing-declared-object.osdf",
            &["OSDF_MANIFEST_MISSING_OBJECT"],
        ),
        (
            "fake-parent-commitment.osdf",
            &["OSDF_REVISION_PARENT_COMMITMENT"],
        ),
        (
            "deleted-parent-revision.osdf",
            &["OSDF_REVISION_CHAIN_BROKEN", "OSDF_MANIFEST_MISSING_OBJECT"],
        ),
    ];

    for (name, expected_codes) in invalid_expectations {
        let bytes = std::fs::read(root.join(format!("invalid/{name}"))).unwrap();
        let report = verify_package_bytes(&bytes);
        assert_eq!(report.overall, VerificationStatus::Fail, "{name}");
        assert!(
            expected_codes.iter().any(|code| {
                report.findings.iter().any(|finding| finding.code == *code)
                    || report.checks.iter().any(|check| check.code == *code)
            }),
            "{name} expected one of {expected_codes:?}, got findings {:?} checks {:?}",
            report.findings,
            report.checks
        );
    }

    let ledger_invalid_expectations: Vec<(&str, &[&str])> = vec![
        ("ledger-leaf-mismatch.osdf", &["OSDF_LEDGER_LEAF_MISMATCH"]),
        (
            "ledger-bad-inclusion.osdf",
            &["OSDF_LEDGER_INCLUSION_INVALID"],
        ),
    ];

    for (name, expected_codes) in ledger_invalid_expectations {
        let bytes = std::fs::read(root.join(format!("invalid/{name}"))).unwrap();
        let report = verify_package_bytes_with_config(
            &bytes,
            &VerifierConfig {
                ledger: trust.clone(),
                ..VerifierConfig::default()
            },
        );
        assert_eq!(report.overall, VerificationStatus::Fail, "{name}");
        assert!(
            expected_codes.iter().any(|code| {
                report.findings.iter().any(|finding| finding.code == *code)
                    || report.checks.iter().any(|check| check.code == *code)
            }),
            "{name} expected one of {expected_codes:?}, got findings {:?} checks {:?}",
            report.findings,
            report.checks
        );
    }
}

fn write_ledger_invalid(
    root: &std::path::Path,
    ledger_store: &osdf_core::LedgerStore,
    log_key: &ed25519_dalek::SigningKey,
    name: &str,
    mutate: impl FnOnce(&mut osdf_core::types::TransparencyProof),
) {
    use osdf_core::find_leaf_index;

    let committed = create_package(CreateOptions {
        title: "Ledger Invalid Fixture".to_string(),
        commit: true,
        ..Default::default()
    })
    .unwrap();
    let event_hash = osdf_core::revision_event_hash_bytes(&committed, 1).unwrap();
    let mut store = ledger_store.clone();
    let manifest = parse_manifest(&committed).unwrap();
    let leaf_index = find_leaf_index(&store, &event_hash).unwrap_or_else(|| {
        append_revision_to_store(
            &mut store,
            &manifest.document_id,
            manifest.revision,
            event_hash,
        )
    });
    let proof = build_proof_for_store(&store, leaf_index, event_hash, log_key).unwrap();
    let mut container = committed;
    attach_transparency_proof(&mut container, proof).unwrap();
    let path = osdf_core::transparency_proof_path(1);
    let mut proof: osdf_core::types::TransparencyProof =
        serde_json::from_slice(container.get(&path).unwrap()).unwrap();
    mutate(&mut proof);
    container
        .insert(path, serde_json::to_vec_pretty(&proof).unwrap())
        .unwrap();
    refresh_header_bytes(&mut container);
    container
        .write_to_path(&root.join(format!("invalid/{name}")))
        .unwrap();
}

fn write_invalid(
    root: &std::path::Path,
    name: &str,
    mutate: impl FnOnce(&mut PackageContainer),
    fix_header: bool,
) {
    let committed = create_package(CreateOptions {
        title: "Invalid Fixture".to_string(),
        commit: true,
        ..Default::default()
    })
    .unwrap();
    let mut container = committed;
    mutate(&mut container);
    if fix_header {
        refresh_header_bytes(&mut container);
    }
    container
        .write_to_path(&root.join(format!("invalid/{name}")))
        .unwrap();
}

fn refresh_header_bytes(container: &mut PackageContainer) {
    use osdf_core::constants::HEADER_PATH;
    use osdf_core::container::make_header_bytes;

    let bytes = container.to_bytes().unwrap();
    let length = bytes.len() as u64;
    container
        .insert(HEADER_PATH, make_header_bytes(length))
        .unwrap();
    container.archive_bytes = container.to_bytes().unwrap().len() as u64;
}

fn tamper_manifest_field(
    container: &mut PackageContainer,
    mutate: impl FnOnce(&mut osdf_core::types::PackageManifest),
) {
    use osdf_core::constants::MANIFEST_PATH;
    let bytes = container.get(MANIFEST_PATH).unwrap().to_vec();
    let mut manifest: osdf_core::types::PackageManifest = serde_json::from_slice(&bytes).unwrap();
    mutate(&mut manifest);
    container
        .insert(MANIFEST_PATH, serde_json::to_vec_pretty(&manifest).unwrap())
        .unwrap();
}

fn append_bytes(source: &[u8], suffix: &[u8]) -> Vec<u8> {
    let mut out = source.to_vec();
    out.extend_from_slice(suffix);
    out
}

fn build_duplicate_path_zip_python(source: &std::path::Path) -> Vec<u8> {
    use std::process::Command;

    let script = r#"
import io
import sys
import zipfile

source = sys.argv[1]
with zipfile.ZipFile(source, "r") as archive:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as writer:
        for info in archive.infolist():
            writer.writestr(info, archive.read(info.filename))
        first = archive.infolist()[0]
        writer.writestr(first, archive.read(first.filename))
    sys.stdout.buffer.write(buffer.getvalue())
"#;

    let output = Command::new("python")
        .args(["-c", script, source.to_str().unwrap()])
        .output()
        .expect("python must be available to build duplicate-path fixture");

    assert!(
        output.status.success(),
        "duplicate-path fixture script failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn build_zip_with_raw_path(source: &[u8], raw_path: &str, payload: &[u8]) -> Vec<u8> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipArchive;

    let mut output = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut output);
        let mut writer = zip::ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();

        let input = std::io::Cursor::new(source);
        let mut archive = ZipArchive::new(input).unwrap();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).unwrap();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            writer.start_file(file.name(), options).unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.start_file(raw_path, options).unwrap();
        writer.write_all(payload).unwrap();
        writer.finish().unwrap();
    }
    output
}
