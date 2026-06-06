use osdf_core::{
    verify_package_bytes, verify_package_bytes_with_config, verify_package_bytes_with_ledger,
    LedgerConfig, VerificationStatus, VerifierConfig,
};

#[test]
fn ledger_fixture_passes_with_trust_config() {
    let bytes = include_bytes!("../../../fixtures/valid/valid-rev2-with-ledger-proof.osdf");
    let trust_json = include_str!("../../../fixtures/valid/ledger-trust.json");
    let trust: LedgerConfig = serde_json::from_str(trust_json).expect("trust config");
    let report = verify_package_bytes_with_ledger(bytes, &trust);
    assert_eq!(report.overall, VerificationStatus::Pass);
    assert!(report.checks.iter().any(|check| {
        check.code == "OSDF_LEDGER_INCLUSION_PROOF_VALID"
            && check.status == VerificationStatus::Pass
    }));
    assert!(report.checks.iter().any(|check| {
        check.code == "OSDF_LATEST_REVISION_CONFIRMED" && check.status == VerificationStatus::Pass
    }));
}

#[test]
fn ledger_fixture_marks_outdated_revision() {
    let bytes = include_bytes!("../../../fixtures/valid/valid-with-ledger-proof.osdf");
    let trust_json = include_str!("../../../fixtures/valid/ledger-trust.json");
    let trust: LedgerConfig = serde_json::from_str(trust_json).expect("trust config");
    let report = verify_package_bytes_with_ledger(bytes, &trust);
    assert_eq!(report.overall, VerificationStatus::Warning);
    assert!(report.checks.iter().any(|check| {
        check.code == "OSDF_LATEST_REVISION_OUTDATED" && check.status == VerificationStatus::Warning
    }));
}

#[test]
fn identity_fixture_passes_with_trust_config() {
    let bytes = include_bytes!("../../../fixtures/valid/valid-with-identity.osdf");
    let trust_json = include_str!("../../../fixtures/valid/identity-trust.json");
    let config: VerifierConfig =
        serde_json::from_str(&format!("{{\"identity\":{trust_json}}}")).expect("verifier config");
    let report = verify_package_bytes_with_config(bytes, &config);
    assert_eq!(report.overall, VerificationStatus::Pass);
    assert!(report.checks.iter().any(|check| {
        check.code == "OSDF_IDENTITY_RESOLVED" && check.status == VerificationStatus::Pass
    }));
}

#[test]
fn committed_fixture_matches_native_expectations() {
    let bytes = include_bytes!("../../../fixtures/valid/valid-committed.osdf");

    let report = verify_package_bytes(bytes);

    assert_eq!(report.overall, VerificationStatus::Pass);
    assert_eq!(report.revision, Some(1));
    assert!(report
        .checks
        .iter()
        .any(|check| check.code == "OSDF_SIGNATURE_CRYPTO"));
}

#[test]
fn draft_fixture_passes() {
    let bytes = include_bytes!("../../../fixtures/valid/valid-draft.osdf");
    let report = verify_package_bytes(bytes);
    assert_eq!(report.overall, VerificationStatus::Pass);
    assert_eq!(report.revision, Some(0));
}

#[test]
fn invalid_fixture_is_rejected() {
    let bytes = include_bytes!("../../../fixtures/invalid/undeclared-object.osdf");
    let report = verify_package_bytes(bytes);
    assert_eq!(report.overall, VerificationStatus::Fail);
}

#[test]
fn missing_magic_fixture_is_rejected() {
    let bytes = include_bytes!("../../../fixtures/invalid/missing-magic.osdf");
    let report = verify_package_bytes(bytes);
    assert_eq!(report.overall, VerificationStatus::Fail);
}

#[test]
fn duplicate_path_fixture_is_rejected() {
    let bytes = include_bytes!("../../../fixtures/invalid/duplicate-path.osdf");
    let report = verify_package_bytes(bytes);
    assert_eq!(report.overall, VerificationStatus::Fail);
}

#[test]
fn report_serializes_to_json() {
    let bytes = include_bytes!("../../../fixtures/valid/valid-committed.osdf");
    let report = verify_package_bytes(bytes);
    let json = serde_json::to_value(&report).expect("report must serialize");
    assert_eq!(json["overall"], "PASS");
    assert!(json["checks"].is_array());
}
