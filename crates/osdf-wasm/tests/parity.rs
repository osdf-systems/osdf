use osdf_core::{verify_package_bytes, VerificationStatus};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn committed_fixture_matches_native_expectations() {
    let bytes = include_bytes!("../../../fixtures/valid/valid-committed.osdf");

    let report = verify_package_bytes(bytes);

    assert_eq!(report.overall, VerificationStatus::Pass);
    assert_eq!(report.revision, Some(1));
    assert!(report
        .checks
        .iter()
        .any(|check| check.code == "OSDF_SIGNATURE_VALID"));
}

#[wasm_bindgen_test]
fn invalid_fixture_is_rejected() {
    let bytes = include_bytes!("../../../fixtures/invalid/undeclared-object.osdf");

    let report = verify_package_bytes(bytes);

    assert_eq!(report.overall, VerificationStatus::Fail);
}

#[wasm_bindgen_test]
fn exported_api_serializes_report() {
    let bytes = include_bytes!("../../../fixtures/valid/valid-committed.osdf");

    let js_value = osdf_wasm::verify_osdf(bytes).expect("WASM API should serialize a valid report");

    assert!(!js_value.is_null());
    assert!(!js_value.is_undefined());
}
