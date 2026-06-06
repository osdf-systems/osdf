use crate::container::PackageContainer;
use crate::error::Result;
use crate::identity::VerifierConfig;
use crate::ledger::LedgerConfig;
use crate::manifest::parse_envelope;
use crate::report::{message_from_error, record_error, ReportBuilder, VerificationReport};
use crate::signature::verify_signatures;
use crate::types::SignatureEnvelope;
use crate::verify_audit::{
    audit_and_read_container, audit_latest_revision, audit_manifest, audit_revision_chain,
    audit_signatures, audit_transparency, audit_verification_context,
};

pub use crate::report::{
    Severity, VerificationCheck, VerificationFinding, VerificationMessage, VerificationMode,
    VerificationSection, VerificationSectionReport, VerificationStatus as ReportStatus,
    CORE_VERSION,
};

pub fn verify_package_bytes(data: &[u8]) -> VerificationReport {
    verify_package_bytes_with_config(data, &VerifierConfig::default())
}

pub fn verify_package_bytes_with_ledger(data: &[u8], ledger: &LedgerConfig) -> VerificationReport {
    verify_package_bytes_with_config(
        data,
        &VerifierConfig {
            ledger: ledger.clone(),
            ..VerifierConfig::default()
        },
    )
}

pub fn verify_package_bytes_with_config(
    data: &[u8],
    config: &VerifierConfig,
) -> VerificationReport {
    let ledger = &config.ledger;
    let mut builder = ReportBuilder::new();
    let Some(container) = audit_and_read_container(data, &mut builder) else {
        audit_transparency(&mut builder, None, None, ledger);
        audit_verification_context(&mut builder, config);
        return builder.finish();
    };

    let envelope = match parse_envelope(&container) {
        Ok(envelope) => {
            builder.profile(envelope.profile.clone());
            let safe = envelope.profile == crate::constants::PROFILE_CORE
                && envelope.payload_mode == crate::types::PayloadMode::Inline;
            if !safe {
                record_error(
                    &mut builder,
                    "OSDF_PROFILE_UNSUPPORTED",
                    "package profile or payload mode is not supported in this verifier",
                );
            }
            envelope
        }
        Err(err) => {
            let (code, technical) = message_from_error(&err);
            record_error(&mut builder, &code, technical);
            audit_transparency(&mut builder, None, None, ledger);
            audit_verification_context(&mut builder, config);
            return builder.finish();
        }
    };

    if envelope.payload_mode == crate::types::PayloadMode::Encrypted {
        record_error(
            &mut builder,
            "OSDF_PAYLOAD_ENCRYPTED",
            "encrypted packages are not supported in this verifier",
        );
        audit_transparency(&mut builder, None, None, ledger);
        audit_verification_context(&mut builder, config);
        return builder.finish();
    }

    let Some(manifest) = audit_manifest(&container, &mut builder) else {
        audit_transparency(&mut builder, Some(&container), None, ledger);
        audit_verification_context(&mut builder, config);
        return builder.finish();
    };

    audit_revision_chain(&container, &manifest, &mut builder);
    audit_signatures(&container, &manifest, &mut builder, &config.identity);
    audit_transparency(&mut builder, Some(&container), Some(&manifest), ledger);
    audit_latest_revision(&mut builder, &container, &manifest, ledger);
    audit_verification_context(&mut builder, config);
    builder.finish()
}

#[cfg(feature = "native-create")]
pub fn verify_package_path(path: &std::path::Path) -> VerificationReport {
    verify_package_path_with_config(path, &VerifierConfig::default())
}

#[cfg(feature = "native-create")]
pub fn verify_package_path_with_ledger(
    path: &std::path::Path,
    ledger: &LedgerConfig,
) -> VerificationReport {
    verify_package_path_with_config(
        path,
        &VerifierConfig {
            ledger: ledger.clone(),
            ..VerifierConfig::default()
        },
    )
}

#[cfg(feature = "native-create")]
pub fn verify_package_path_with_config(
    path: &std::path::Path,
    config: &VerifierConfig,
) -> VerificationReport {
    match std::fs::read(path) {
        Ok(data) => verify_package_bytes_with_config(&data, config),
        Err(err) => {
            let mut builder = ReportBuilder::new();
            record_error(
                &mut builder,
                "OSDF_IO_ERROR",
                format!("failed to read file: {err}"),
            );
            audit_transparency(&mut builder, None, None, &config.ledger);
            audit_verification_context(&mut builder, config);
            builder.finish()
        }
    }
}

pub fn verify_container(container: &PackageContainer) -> VerificationReport {
    verify_container_with_config(container, &VerifierConfig::default())
}

pub fn verify_container_with_ledger(
    container: &PackageContainer,
    ledger: &LedgerConfig,
) -> VerificationReport {
    verify_container_with_config(
        container,
        &VerifierConfig {
            ledger: ledger.clone(),
            ..VerifierConfig::default()
        },
    )
}

pub fn verify_container_with_config(
    container: &PackageContainer,
    config: &VerifierConfig,
) -> VerificationReport {
    verify_package_bytes_with_config(
        &container.to_bytes().expect("container must serialize"),
        config,
    )
}

pub fn inspect_container(container: &PackageContainer) -> Result<InspectReport> {
    use crate::manifest::parse_manifest;

    let envelope = parse_envelope(container)?;
    let manifest = parse_manifest(container)?;
    let signatures = verify_signatures(container).unwrap_or_default();

    Ok(InspectReport {
        document_id: manifest.document_id,
        revision: manifest.revision,
        profile: envelope.profile,
        package_bytes: container.archive_bytes,
        content_bytes: manifest.content_bytes,
        object_count: manifest.objects.len(),
        paths: manifest
            .objects
            .into_iter()
            .map(|object| object.path)
            .collect(),
        signatures,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectReport {
    pub document_id: String,
    pub revision: u32,
    pub profile: String,
    pub package_bytes: u64,
    pub content_bytes: u64,
    pub object_count: usize,
    pub paths: Vec<String>,
    pub signatures: Vec<SignatureEnvelope>,
}
