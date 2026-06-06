//! Integration tests and malformed fixture checks.

use osdf_core::{
    append_revision_to_store, attach_transparency_proof, build_delegation_credential,
    build_proof_for_store, commit_revision, create_ledger_store, create_package,
    generate_signing_key, sign_delegation_credential, trust_config_for_store, verify_container,
    verify_container_with_config, verify_package_bytes, verifying_key_to_urn, CommitOptions,
    CreateOptions, IdentityConfig, IdentityPolicy, LedgerPolicy, PackageContainer,
    TrustedOrganization, VerificationStatus, VerifierConfig,
};

#[test]
fn valid_draft_roundtrip() {
    let container = create_package(CreateOptions {
        title: "Fixture Draft".to_string(),
        ..Default::default()
    })
    .unwrap();

    let bytes = container.to_bytes().unwrap();
    let reparsed = PackageContainer::read_from_bytes(&bytes).unwrap();
    let report = verify_container(&reparsed);
    assert_eq!(
        report.overall,
        VerificationStatus::Pass,
        "{:?}",
        report.errors
    );
}

#[test]
fn valid_committed_roundtrip() {
    let container = create_package(CreateOptions {
        title: "Fixture Committed".to_string(),
        commit: true,
        ..Default::default()
    })
    .unwrap();

    let bytes = container.to_bytes().unwrap();
    let report = verify_package_bytes(&bytes);
    assert_eq!(
        report.overall,
        VerificationStatus::Pass,
        "{:?}",
        report.errors
    );
    assert_eq!(report.revision, Some(1));
}

#[test]
fn rejects_missing_magic_header() {
    let container = create_package(CreateOptions::default()).unwrap();
    let mut broken = container.clone();
    broken
        .insert("osdf-header.bin", b"BADMAGIC".to_vec())
        .unwrap();
    let bytes = broken.to_bytes().unwrap();
    let report = verify_package_bytes(&bytes);
    assert_eq!(report.overall, VerificationStatus::Fail);
}

#[test]
fn rejects_undeclared_object() {
    let container = create_package(CreateOptions::default()).unwrap();
    let mut broken = container.clone();
    broken
        .insert("secret/extra.bin", b"surprise".to_vec())
        .unwrap();
    let bytes = broken.to_bytes().unwrap();
    let report = verify_package_bytes(&bytes);
    assert_eq!(report.overall, VerificationStatus::Fail);
}

#[test]
fn rejects_duplicate_paths_in_zip() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let mut buffer = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buffer);
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        writer.start_file("dup.txt", options).unwrap();
        writer.write_all(b"one").unwrap();
        let duplicate = writer.start_file("dup.txt", options);
        assert!(
            duplicate.is_err(),
            "zip writer should reject duplicate paths"
        );
    }
}

#[test]
fn load_static_invalid_fixtures() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/invalid");
    if !root.exists() {
        return;
    }

    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("osdf") {
            continue;
        }
        let bytes = std::fs::read(entry.path()).unwrap();
        let report = verify_package_bytes(&bytes);
        assert_eq!(
            report.overall,
            VerificationStatus::Fail,
            "expected invalid fixture {:?} to fail verification",
            entry.path()
        );
    }
}

#[test]
fn identity_resolution_roundtrip() {
    let org_root_key = generate_signing_key();
    let delegate_key = generate_signing_key();
    let container = create_package(CreateOptions {
        title: "Identity Roundtrip".to_string(),
        commit: true,
        signing_key: Some(delegate_key.clone()),
        ..Default::default()
    })
    .unwrap();

    let delegate_urn = verifying_key_to_urn(&delegate_key.verifying_key());
    let mut delegation = build_delegation_credential(
        "urn:osdf:org:demo",
        Some("Department of Revenue"),
        &delegate_urn,
        "2020-01-01T00:00:00Z",
        "2030-01-01T00:00:00Z",
    );
    sign_delegation_credential(&org_root_key, &mut delegation).unwrap();

    let config = VerifierConfig {
        identity: IdentityConfig {
            policy: IdentityPolicy::Required,
            organizations: vec![TrustedOrganization {
                organization_id: "urn:osdf:org:demo".to_string(),
                display_name: "State of Colorado".to_string(),
                root_keys: vec![verifying_key_to_urn(&org_root_key.verifying_key())],
            }],
            delegations: vec![delegation],
        },
        ..VerifierConfig::default()
    };

    let report = verify_container_with_config(&container, &config);
    assert_eq!(
        report.overall,
        VerificationStatus::Pass,
        "{:?}",
        report.findings
    );
    assert!(report.checks.iter().any(|check| {
        check.code == "OSDF_IDENTITY_RESOLVED" && check.status == VerificationStatus::Pass
    }));
    assert_eq!(report.signer_identities.len(), 1);
    assert_eq!(
        report.signer_identities[0].display_name,
        "State of Colorado"
    );
}

#[test]
fn ledger_proof_roundtrip() {
    let container = create_package(CreateOptions {
        title: "Ledger Roundtrip".to_string(),
        commit: true,
        ..Default::default()
    })
    .unwrap();

    let log_key = generate_signing_key();
    let mut store = create_ledger_store(Some("urn:osdf:log:test".to_string()), &log_key);
    let manifest = osdf_core::manifest::parse_manifest(&container).unwrap();
    let event_hash =
        osdf_core::revision_event_hash_bytes(&container, 1).expect("revision event hash");
    let leaf_index = append_revision_to_store(
        &mut store,
        &manifest.document_id,
        manifest.revision,
        event_hash,
    );
    let proof =
        build_proof_for_store(&store, leaf_index, event_hash, &log_key).expect("build proof");

    let mut with_proof = container.clone();
    attach_transparency_proof(&mut with_proof, proof).expect("attach proof");

    let without_ledger = verify_container(&with_proof);
    assert_eq!(without_ledger.overall, VerificationStatus::Pass);

    let trust = trust_config_for_store(&store, LedgerPolicy::Required);
    let with_ledger = verify_container_with_config(
        &with_proof,
        &VerifierConfig {
            ledger: trust,
            ..VerifierConfig::default()
        },
    );
    assert_eq!(
        with_ledger.overall,
        VerificationStatus::Pass,
        "{:?}",
        with_ledger.findings
    );
    assert!(with_ledger.checks.iter().any(|check| {
        check.code == "OSDF_LEDGER_INCLUSION_PROOF_VALID"
            && check.status == VerificationStatus::Pass
    }));
    assert!(with_ledger.checks.iter().any(|check| {
        check.code == "OSDF_LATEST_REVISION_CONFIRMED" && check.status == VerificationStatus::Pass
    }));
}

#[test]
fn latest_revision_outdated_warning() {
    let mut rev1 = create_package(CreateOptions {
        title: "Latest Revision Fixture".to_string(),
        commit: true,
        ..Default::default()
    })
    .unwrap();
    let log_key = generate_signing_key();
    let mut store = create_ledger_store(Some("urn:osdf:log:latest-test".to_string()), &log_key);
    let rev1_manifest = osdf_core::manifest::parse_manifest(&rev1).unwrap();
    let event_hash1 =
        osdf_core::revision_event_hash_bytes(&rev1, 1).expect("revision 1 event hash");
    let leaf_index1 = append_revision_to_store(
        &mut store,
        &rev1_manifest.document_id,
        rev1_manifest.revision,
        event_hash1,
    );
    let proof1 = build_proof_for_store(&store, leaf_index1, event_hash1, &log_key)
        .expect("build rev1 proof");
    attach_transparency_proof(&mut rev1, proof1).expect("attach rev1 proof");

    let mut rev2 = rev1.clone();
    commit_revision(
        &mut rev2,
        CommitOptions {
            signing_key: generate_signing_key(),
            signer_key_reference: None,
        },
    )
    .expect("commit revision 2");
    let rev2_manifest = osdf_core::manifest::parse_manifest(&rev2).unwrap();
    let event_hash2 =
        osdf_core::revision_event_hash_bytes(&rev2, 2).expect("revision 2 event hash");
    let leaf_index2 = append_revision_to_store(
        &mut store,
        &rev2_manifest.document_id,
        rev2_manifest.revision,
        event_hash2,
    );
    let proof2 =
        build_proof_for_store(&store, leaf_index2, event_hash2, &log_key).expect("build proof");
    attach_transparency_proof(&mut rev2, proof2).expect("attach proof");

    let trust = trust_config_for_store(&store, LedgerPolicy::Required);
    let outdated = verify_container_with_config(
        &rev1,
        &VerifierConfig {
            ledger: trust.clone(),
            ..VerifierConfig::default()
        },
    );
    assert_eq!(outdated.overall, VerificationStatus::Warning);
    assert!(outdated.checks.iter().any(|check| {
        check.code == "OSDF_LATEST_REVISION_OUTDATED" && check.status == VerificationStatus::Warning
    }));

    let current = verify_container_with_config(
        &rev2,
        &VerifierConfig {
            ledger: trust,
            ..VerifierConfig::default()
        },
    );
    assert_eq!(current.overall, VerificationStatus::Pass);
    assert!(current.checks.iter().any(|check| {
        check.code == "OSDF_LATEST_REVISION_CONFIRMED" && check.status == VerificationStatus::Pass
    }));
}

#[test]
fn load_static_valid_fixtures() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/valid");
    if !root.exists() {
        return;
    }

    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("osdf") {
            continue;
        }
        let bytes = std::fs::read(entry.path()).unwrap();
        let report = verify_package_bytes(&bytes);
        assert_eq!(
            report.overall,
            VerificationStatus::Pass,
            "expected valid fixture {:?} to pass: {:?}",
            entry.path(),
            report.errors
        );
    }
}

#[test]
fn committed_fixture_has_expected_report() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/valid/valid-committed.osdf");
    if !path.exists() {
        return;
    }

    let bytes = std::fs::read(path).unwrap();
    let report = verify_package_bytes(&bytes);

    assert_eq!(report.overall, VerificationStatus::Pass);
    assert_eq!(report.revision, Some(1));
    assert!(report
        .checks
        .iter()
        .any(|check| check.code == "OSDF_SIGNATURE_CRYPTO"));
}
