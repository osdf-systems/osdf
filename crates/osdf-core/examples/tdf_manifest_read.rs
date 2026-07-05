//! Offline **structural** read of an OpenTDF golden `.tdf` (ZIP) fixture.
//!
//! This is the fair, offline analogue to OSDF's *structural* verify: it opens
//! the TDF ZIP container, reads the `0.manifest.json` entry, and JSON-parses it.
//! It performs **no** KAS round-trip, key unwrap, or payload decryption — those
//! require a running OpenTDF platform and `otdfctl`. The benchmark harness labels
//! this bar "OpenTDF structural manifest read (offline)".
//!
//! ```text
//! cargo run --release -p osdf-core --example tdf_manifest_read -- \
//!   fixtures/benchmarks/opentdf/small-java-4.3.0-e0f8caf.tdf
//! ```
//!
//! Depends only on crates already used by osdf-core (`zip`, `serde_json`), so it
//! does not assume `unzip`/`jq`/`python` are present on the host (notably Windows).

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

const MANIFEST_ENTRY: &str = "0.manifest.json";

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let path: PathBuf = match args.next() {
        Some(value) => PathBuf::from(value),
        None => {
            eprintln!(
                "usage: tdf_manifest_read <path-to.tdf>\n\
                 reads `{MANIFEST_ENTRY}` from the TDF ZIP and parses it as JSON"
            );
            return ExitCode::from(2);
        }
    };

    match read_manifest(&path) {
        Ok(object_count) => {
            // Keep stdout quiet enough for hyperfine but prove real work happened.
            println!(
                "ok: {} parsed {MANIFEST_ENTRY} ({} top-level manifest keys)",
                path.display(),
                object_count
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn read_manifest(path: &std::path::Path) -> Result<usize, String> {
    let data =
        std::fs::read(path).map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|err| format!("`{}` is not a valid ZIP/TDF: {err}", path.display()))?;

    let mut manifest_bytes = Vec::new();
    {
        let mut entry = archive
            .by_name(MANIFEST_ENTRY)
            .map_err(|err| format!("`{MANIFEST_ENTRY}` not found in TDF: {err}"))?;
        entry
            .read_to_end(&mut manifest_bytes)
            .map_err(|err| format!("failed to read `{MANIFEST_ENTRY}`: {err}"))?;
    }

    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| format!("`{MANIFEST_ENTRY}` is not valid JSON: {err}"))?;

    let key_count = manifest.as_object().map(|map| map.len()).unwrap_or(0);
    Ok(key_count)
}
