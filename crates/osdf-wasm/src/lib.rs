//! WebAssembly bindings for read-only OSDF verification.

use osdf_core::{
    inspect_container, verify_package_bytes_with_config, PackageContainer, VerifierConfig,
    CORE_VERSION,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[wasm_bindgen]
pub fn core_version() -> String {
    CORE_VERSION.to_owned()
}

#[wasm_bindgen]
pub fn build_commit() -> String {
    env!("GIT_COMMIT").to_owned()
}

#[wasm_bindgen]
pub fn verify_osdf(bytes: &[u8]) -> Result<JsValue, JsValue> {
    verify_with_config(bytes, None)
}

#[wasm_bindgen]
pub fn verify_osdf_with_ledger(bytes: &[u8], ledger_config_json: &str) -> Result<JsValue, JsValue> {
    let ledger: osdf_core::LedgerConfig = parse_json(ledger_config_json, "ledger config")?;
    verify_with_config(
        bytes,
        Some(VerifierConfig {
            ledger,
            ..VerifierConfig::default()
        }),
    )
}

#[wasm_bindgen]
pub fn verify_osdf_with_config(
    bytes: &[u8],
    verifier_config_json: &str,
) -> Result<JsValue, JsValue> {
    let config: VerifierConfig = parse_json(verifier_config_json, "verifier config")?;
    verify_with_config(bytes, Some(config))
}

#[wasm_bindgen]
pub fn inspect_package(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let container = PackageContainer::read_from_bytes(bytes)
        .map_err(|error| JsValue::from_str(&format!("Failed to read OSDF package: {error}")))?;
    let report = inspect_container(&container)
        .map_err(|error| JsValue::from_str(&format!("Failed to inspect package: {error}")))?;
    serde_wasm_bindgen::to_value(&report).map_err(|error| {
        JsValue::from_str(&format!("Failed to serialize inspect report: {error}"))
    })
}

#[wasm_bindgen]
pub fn read_package_entry_bytes(bytes: &[u8], path: &str) -> Result<Vec<u8>, JsValue> {
    let container = PackageContainer::read_from_bytes(bytes)
        .map_err(|error| JsValue::from_str(&format!("Failed to read OSDF package: {error}")))?;
    let Some(data) = container.get(path) else {
        return Err(JsValue::from_str(&format!(
            "Package entry not found: {path}"
        )));
    };
    Ok(data.to_vec())
}

#[wasm_bindgen]
pub fn read_package_entry(bytes: &[u8], path: &str) -> Result<JsValue, JsValue> {
    let container = PackageContainer::read_from_bytes(bytes)
        .map_err(|error| JsValue::from_str(&format!("Failed to read OSDF package: {error}")))?;
    let Some(data) = container.get(path) else {
        return Err(JsValue::from_str(&format!(
            "Package entry not found: {path}"
        )));
    };
    match std::str::from_utf8(data) {
        Ok(text) => Ok(JsValue::from_str(text)),
        Err(_) => Err(JsValue::from_str(&format!(
            "Package entry `{path}` is not valid UTF-8 text"
        ))),
    }
}

fn verify_with_config(bytes: &[u8], config: Option<VerifierConfig>) -> Result<JsValue, JsValue> {
    let report = match config {
        Some(config) => verify_package_bytes_with_config(bytes, &config),
        None => verify_package_bytes_with_config(bytes, &VerifierConfig::default()),
    };

    serde_wasm_bindgen::to_value(&report).map_err(|error| {
        JsValue::from_str(&format!("Failed to serialize verification report: {error}"))
    })
}

fn parse_json<T: for<'de> serde::de::Deserialize<'de>>(
    json: &str,
    label: &str,
) -> Result<T, JsValue> {
    serde_json::from_str(json)
        .map_err(|error| JsValue::from_str(&format!("Invalid {label} JSON: {error}")))
}
