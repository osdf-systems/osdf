use serde::{Deserialize, Serialize};

use crate::constants::FORMAT_VERSION;

pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerificationReport {
    pub format: String,
    pub format_version: String,
    pub document_id: Option<String>,
    pub revision: Option<u64>,
    pub profile: Option<String>,
    pub signature_count: u64,
    pub overall: VerificationStatus,
    pub verification_mode: VerificationMode,
    pub sections: Vec<VerificationSectionReport>,
    pub findings: Vec<VerificationFinding>,
    /// Flattened checks retained for export compatibility and tests.
    pub checks: Vec<VerificationCheck>,
    pub warnings: Vec<VerificationMessage>,
    pub errors: Vec<VerificationMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signer_identities: Vec<ResolvedSignerIdentityReport>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Pass,
    Warning,
    Fail,
    Info,
}

impl VerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warning => "WARNING",
            Self::Fail => "FAIL",
            Self::Info => "INFO",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum VerificationMode {
    #[default]
    OfflineCryptographic,
    OnlineEnhanced,
}

impl VerificationMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::OfflineCryptographic => "Offline cryptographic verification",
            Self::OnlineEnhanced => "Online enhanced verification",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OfflineCryptographic => "offlineCryptographic",
            Self::OnlineEnhanced => "onlineEnhanced",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VerificationSection {
    VerificationContext,
    Container,
    Manifest,
    Revision,
    Signatures,
    Transparency,
}

impl VerificationSection {
    pub fn title(self) -> &'static str {
        match self {
            Self::VerificationContext => "Verification mode",
            Self::Container => "Container safety",
            Self::Manifest => "Manifest integrity",
            Self::Revision => "Revision integrity",
            Self::Signatures => "Signatures",
            Self::Transparency => "Transparency",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerificationSectionReport {
    pub section: VerificationSection,
    pub title: String,
    pub checks: Vec<VerificationCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCheck {
    pub section: VerificationSection,
    pub code: String,
    pub label: String,
    pub status: VerificationStatus,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Info,
    Warning,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerificationFinding {
    pub code: String,
    pub severity: Severity,
    pub summary: String,
    pub impact: String,
    pub technical: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerificationMessage {
    pub code: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSignerIdentityReport {
    pub signer_key: String,
    pub organization_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    pub resolution_path: String,
    pub signing_timestamp: String,
}

pub struct ReportBuilder {
    report: VerificationReport,
    current_section: Option<VerificationSection>,
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportBuilder {
    pub fn new() -> Self {
        Self {
            report: VerificationReport::new(),
            current_section: None,
        }
    }

    pub fn document_id(&mut self, value: impl Into<String>) {
        self.report.document_id = Some(value.into());
    }

    pub fn revision(&mut self, value: u64) {
        self.report.revision = Some(value);
    }

    pub fn profile(&mut self, value: impl Into<String>) {
        self.report.profile = Some(value.into());
    }

    pub fn signature_count(&mut self, value: u64) {
        self.report.signature_count = value;
    }

    pub fn verification_mode(&mut self, mode: VerificationMode) {
        self.report.verification_mode = mode;
    }

    pub fn has_passing_check(&self, code: &str) -> bool {
        self.report
            .checks
            .iter()
            .any(|check| check.code == code && check.status == VerificationStatus::Pass)
    }

    pub fn has_check(&self, code: &str) -> bool {
        self.report.checks.iter().any(|check| check.code == code)
    }

    pub fn signer_identity(&mut self, identity: ResolvedSignerIdentityReport) {
        self.report.signer_identities.push(identity);
    }

    pub fn begin_section(&mut self, section: VerificationSection) {
        self.current_section = Some(section);
        self.report.sections.push(VerificationSectionReport {
            section,
            title: section.title().to_string(),
            checks: Vec::new(),
        });
    }

    pub fn check(
        &mut self,
        code: impl Into<String>,
        label: impl Into<String>,
        status: VerificationStatus,
        details: Option<String>,
    ) {
        let section = self
            .current_section
            .unwrap_or(VerificationSection::Container);
        let check = VerificationCheck {
            section,
            code: code.into(),
            label: label.into(),
            status,
            details,
        };
        self.report.checks.push(check.clone());
        if let Some(active) = self.report.sections.last_mut() {
            active.checks.push(check);
        }
    }

    pub fn finding(
        &mut self,
        code: impl Into<String>,
        severity: Severity,
        summary: impl Into<String>,
        impact: impl Into<String>,
        technical: impl Into<String>,
    ) {
        let code = code.into();
        let summary = summary.into();
        let impact = impact.into();
        let technical = technical.into();
        self.report.findings.push(VerificationFinding {
            code: code.clone(),
            severity,
            summary: summary.clone(),
            impact,
            technical: technical.clone(),
        });
        let message = VerificationMessage {
            code,
            message: technical,
            severity,
        };
        match severity {
            Severity::Fail => self.report.errors.push(message),
            Severity::Warning => self.report.warnings.push(message),
            Severity::Info => {}
        }
    }

    pub fn finish(mut self) -> VerificationReport {
        self.report.finalize();
        self.report
    }
}

impl VerificationReport {
    pub fn new() -> Self {
        Self {
            format: "OSDF".to_string(),
            format_version: FORMAT_VERSION.to_string(),
            document_id: None,
            revision: None,
            profile: None,
            signature_count: 0,
            overall: VerificationStatus::Fail,
            verification_mode: VerificationMode::OfflineCryptographic,
            sections: Vec::new(),
            findings: Vec::new(),
            checks: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            signer_identities: Vec::new(),
        }
    }

    pub fn finalize(&mut self) {
        if self
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Fail)
            || self
                .checks
                .iter()
                .any(|check| check.status == VerificationStatus::Fail)
            || !self.errors.is_empty()
        {
            self.overall = VerificationStatus::Fail;
            return;
        }

        if self
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Warning)
            || self
                .checks
                .iter()
                .any(|check| check.status == VerificationStatus::Warning)
            || !self.warnings.is_empty()
        {
            self.overall = VerificationStatus::Warning;
            return;
        }

        self.overall = VerificationStatus::Pass;
    }
}

impl Default for VerificationReport {
    fn default() -> Self {
        Self::new()
    }
}

pub fn finding_for_code(code: &str, _technical: &str) -> (String, String, Severity) {
    match code {
        "OSDF_CONTAINER_INVALID_MAGIC" | "OSDF_CONTAINER_MISSING_HEADER" => (
            "Invalid OSDF header".to_string(),
            "This file does not begin with a recognized OSDF package header. It may not be an OSDF document.".to_string(),
            Severity::Fail,
        ),
        "OSDF_CONTAINER_INVALID_HEADER" => (
            "Invalid OSDF header".to_string(),
            "The package header is malformed or incomplete.".to_string(),
            Severity::Fail,
        ),
        "OSDF_HEADER_PACKAGE_BYTES" => (
            "Package size mismatch".to_string(),
            "The signed header reports a different package size than the file you selected. The archive may have been truncated or appended.".to_string(),
            Severity::Fail,
        ),
        "OSDF_CONTAINER_TRAILING_BYTES" => (
            "Unexpected trailing bytes".to_string(),
            "Extra bytes were found after the end of the ZIP archive. The package may have been modified after creation.".to_string(),
            Severity::Fail,
        ),
        "OSDF_CONTAINER_DUPLICATE_PATH" => (
            "Duplicate archive path".to_string(),
            "The ZIP archive contains more than one entry with the same path. Do not trust this document.".to_string(),
            Severity::Fail,
        ),
        "OSDF_CONTAINER_PATH_TRAVERSAL" => (
            "Unsafe archive path".to_string(),
            "The package contains a path that could escape its intended directory structure. Do not open this file.".to_string(),
            Severity::Fail,
        ),
        "OSDF_CONTAINER_COMPRESSION_BOMB" => (
            "Suspicious compression ratio".to_string(),
            "An object in this package expands to an unusually large size. The file may be unsafe to process.".to_string(),
            Severity::Fail,
        ),
        "OSDF_CONTAINER_OVERSIZED_OBJECT" => (
            "Oversized archive object".to_string(),
            "An object exceeds the maximum allowed uncompressed size for this verifier.".to_string(),
            Severity::Fail,
        ),
        "OSDF_MANIFEST_UNDECLARED_OBJECT" => (
            "Undeclared file detected".to_string(),
            "This document contains a file that is not listed in its signed manifest. The package may have been modified after creation. Do not trust this document.".to_string(),
            Severity::Fail,
        ),
        "OSDF_MANIFEST_MISSING_OBJECT" => (
            "Missing declared file".to_string(),
            "A file listed in the signed manifest is missing from the package.".to_string(),
            Severity::Fail,
        ),
        "OSDF_MANIFEST_HASH_MISMATCH" => (
            "Content hash mismatch".to_string(),
            "A file's contents do not match the hash recorded in the signed manifest. The document may have been tampered with.".to_string(),
            Severity::Fail,
        ),
        "OSDF_MANIFEST_SIZE_MISMATCH" => (
            "Object size mismatch".to_string(),
            "A file's size does not match the value recorded in the signed manifest.".to_string(),
            Severity::Fail,
        ),
        "OSDF_REVISION_CHAIN_BROKEN" => (
            "Broken revision chain".to_string(),
            "The revision history does not link correctly from parent to child. The document's edit history cannot be trusted.".to_string(),
            Severity::Fail,
        ),
        "OSDF_REVISION_PARENT_COMMITMENT" => (
            "Invalid parent revision commitment".to_string(),
            "The parent revision commitment in the manifest does not match the prior revision record.".to_string(),
            Severity::Fail,
        ),
        "OSDF_SIGNATURE_MISSING" => (
            "Missing signature".to_string(),
            "This committed revision has no valid signature. Do not trust this document.".to_string(),
            Severity::Fail,
        ),
        "OSDF_SIGNATURE_INVALID" => (
            "Invalid signature".to_string(),
            "A signature on this document failed cryptographic verification.".to_string(),
            Severity::Fail,
        ),
        "OSDF_IDENTITY_NOT_RESOLVED" => (
            "Signer identity not yet resolved".to_string(),
            "The signature is cryptographically valid, but this verifier has not yet connected the signing key to an organization.".to_string(),
            Severity::Info,
        ),
        "OSDF_IDENTITY_RESOLVED" => (
            "Signer identity resolved".to_string(),
            "The signing key is bound to a trusted organization in the configured identity registry.".to_string(),
            Severity::Info,
        ),
        "OSDF_IDENTITY_DELEGATION_VALID" => (
            "Signing delegation valid".to_string(),
            "The signer key was authorized by a signed organizational delegation that was valid at signing time.".to_string(),
            Severity::Info,
        ),
        "OSDF_IDENTITY_UNTRUSTED" => (
            "Signer identity not trusted".to_string(),
            "The signing key is not listed as an organization root key and has no valid delegation in the configured identity registry.".to_string(),
            Severity::Fail,
        ),
        "OSDF_LEDGER_NOT_CONFIGURED" => (
            "Ledger verification not configured".to_string(),
            "Transparency ledger verification is not enabled for this profile.".to_string(),
            Severity::Info,
        ),
        "OSDF_LEDGER_PROOF_MISSING" => (
            "Ledger proof missing".to_string(),
            "This committed revision does not include a transparency ledger inclusion proof.".to_string(),
            Severity::Fail,
        ),
        "OSDF_LEDGER_LEAF_MISMATCH" => (
            "Ledger leaf mismatch".to_string(),
            "The ledger proof does not match the revision event hash recorded in this package.".to_string(),
            Severity::Fail,
        ),
        "OSDF_LEDGER_INCLUSION_INVALID" => (
            "Invalid ledger inclusion proof".to_string(),
            "The transparency log inclusion proof could not be recomputed to the signed tree root.".to_string(),
            Severity::Fail,
        ),
        "OSDF_LEDGER_SIGNED_ROOT_INVALID" => (
            "Invalid signed tree head".to_string(),
            "The transparency log signed tree head failed cryptographic verification.".to_string(),
            Severity::Fail,
        ),
        "OSDF_LEDGER_LOG_KEY_UNTRUSTED" => (
            "Untrusted ledger key".to_string(),
            "The transparency log public key is not in the configured trust registry.".to_string(),
            Severity::Fail,
        ),
        "OSDF_REVOCATION_NOT_CONFIGURED" => (
            "Revocation checking not configured".to_string(),
            "Key revocation checking is not enabled for this profile.".to_string(),
            Severity::Info,
        ),
        "OSDF_LEDGER_LATEST_REVISION_NOT_CHECKED" => (
            "Latest-revision lookup not performed".to_string(),
            "This verifier validated an embedded ledger proof but did not query a live log to confirm that no newer revision exists.".to_string(),
            Severity::Info,
        ),
        "OSDF_LATEST_REVISION_CONFIRMED" => (
            "Latest revision confirmed".to_string(),
            "The revision in this package matches the newest revision registered for this document in the configured ledger registry.".to_string(),
            Severity::Info,
        ),
        "OSDF_LATEST_REVISION_OUTDATED" => (
            "Authentic but outdated revision".to_string(),
            "This package is structurally valid, but a newer revision exists in the configured ledger registry.".to_string(),
            Severity::Warning,
        ),
        "OSDF_LATEST_REVISION_REGISTRY_UNAVAILABLE" => (
            "Latest-revision registry unavailable".to_string(),
            "Latest-revision checking was requested, but no registry entry exists for this document.".to_string(),
            Severity::Warning,
        ),
        "OSDF_LIVE_LATEST_REVISION_NOT_CHECKED" => (
            "Live latest-revision check not performed".to_string(),
            "Offline verification did not compare the local revision against the newest revision registered for this document lineage.".to_string(),
            Severity::Info,
        ),
        "OSDF_LEDGER_TREE_HEAD_FRESHNESS_NOT_CHECKED" => (
            "Tree-head freshness not evaluated".to_string(),
            "The signed tree head was verified cryptographically, but its timestamp was not checked against a freshness policy.".to_string(),
            Severity::Info,
        ),
        "OSDF_LEDGER_CONSISTENCY_PROOF_NOT_CHECKED" => (
            "Consistency proof not evaluated".to_string(),
            "The verifier did not confirm that the signed tree head extends a previously observed append-only checkpoint.".to_string(),
            Severity::Info,
        ),
        "OSDF_VERIFICATION_MODE_OFFLINE" => (
            "Offline verification mode".to_string(),
            "Verification used only embedded package data and configured trust material. No live ledger, identity, or revocation services were queried.".to_string(),
            Severity::Info,
        ),
        "OSDF_LIVE_REVOCATION_NOT_CHECKED" => (
            "Revocation status not checked".to_string(),
            "This verifier did not consult a revocation log or registry for the signing keys used by this document.".to_string(),
            Severity::Info,
        ),
        _ => (
            "Verification issue".to_string(),
            "This document failed one or more verification checks.".to_string(),
            Severity::Fail,
        ),
    }
}

pub fn message_from_error(err: &crate::error::OsdfError) -> (String, String) {
    match err {
        crate::error::OsdfError::Container(text) => {
            let code = if text.contains("duplicate path") {
                "OSDF_CONTAINER_DUPLICATE_PATH"
            } else if text.contains("invalid magic") {
                "OSDF_CONTAINER_INVALID_MAGIC"
            } else if text.contains("missing required header") {
                "OSDF_CONTAINER_MISSING_HEADER"
            } else if text.contains("path traversal") || text.contains("absolute paths") {
                "OSDF_CONTAINER_PATH_TRAVERSAL"
            } else if text.contains("compression bomb") {
                "OSDF_CONTAINER_COMPRESSION_BOMB"
            } else if text.contains("uncompressed size exceeds") {
                "OSDF_CONTAINER_OVERSIZED_OBJECT"
            } else if text.contains("case-insensitive path collision") {
                "OSDF_CONTAINER_CASE_COLLISION"
            } else if text.contains("header must be at least") {
                "OSDF_CONTAINER_INVALID_HEADER"
            } else {
                "OSDF_CONTAINER_ERROR"
            };
            (code.to_string(), text.clone())
        }
        crate::error::OsdfError::Manifest(text) => {
            let code = if text.contains("undeclared") {
                "OSDF_MANIFEST_UNDECLARED_OBJECT"
            } else {
                "OSDF_MANIFEST_ERROR"
            };
            (code.to_string(), text.clone())
        }
        crate::error::OsdfError::Integrity(text) => {
            let code = if text.contains("undeclared package object") {
                "OSDF_MANIFEST_UNDECLARED_OBJECT"
            } else if text.contains("declared object missing") {
                "OSDF_MANIFEST_MISSING_OBJECT"
            } else if text.contains("digest mismatch") {
                "OSDF_MANIFEST_HASH_MISMATCH"
            } else if text.contains("byte length mismatch") {
                "OSDF_MANIFEST_SIZE_MISMATCH"
            } else if text.contains("inclusion proof")
                || text.contains("transparency proof treeSize")
            {
                "OSDF_LEDGER_INCLUSION_INVALID"
            } else if text.contains("revisionEventHash mismatch") {
                "OSDF_LEDGER_LEAF_MISMATCH"
            } else {
                "OSDF_INTEGRITY_ERROR"
            };
            (code.to_string(), text.clone())
        }
        crate::error::OsdfError::Revision(text) => {
            let code = if text.contains("parent revision commitment") {
                "OSDF_REVISION_PARENT_COMMITMENT"
            } else {
                "OSDF_REVISION_CHAIN_BROKEN"
            };
            (code.to_string(), text.clone())
        }
        crate::error::OsdfError::Signature(text) => {
            let code = if text.contains("log signed tree head") {
                "OSDF_LEDGER_SIGNED_ROOT_INVALID"
            } else if text.contains("not in trusted log registry")
                || text.contains("not trusted for log")
            {
                "OSDF_LEDGER_LOG_KEY_UNTRUSTED"
            } else if text.contains("delegation")
                || text.contains("identity registry")
                || text.contains("organization")
            {
                "OSDF_IDENTITY_UNTRUSTED"
            } else {
                "OSDF_SIGNATURE_INVALID"
            };
            (code.to_string(), text.clone())
        }
        crate::error::OsdfError::Io(err) => ("OSDF_IO_ERROR".to_string(), err.to_string()),
        crate::error::OsdfError::Json(err) => ("OSDF_JSON_ERROR".to_string(), err.to_string()),
        crate::error::OsdfError::Zip(err) => ("OSDF_ZIP_ERROR".to_string(), err.to_string()),
    }
}

pub fn record_error(builder: &mut ReportBuilder, code: &str, technical: impl Into<String>) {
    let technical = technical.into();
    let (summary, impact, severity) = finding_for_code(code, &technical);
    builder.finding(code, severity, summary, impact, technical);
}
