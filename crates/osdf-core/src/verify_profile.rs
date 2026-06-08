//! Verification profiles and compact fast-verify results.
//!
//! Each profile has distinct guarantees. Benchmark and document them separately;
//! do not claim parsed-container throughput as full portable ingest throughput.

/// Which verification path was executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationProfile {
    /// ZIP-backed OSDF-Core JSON package with full structured forensic report.
    CoreJsonPortableFull,
    /// Same cryptographic and policy checks as [`Self::CoreJsonPortableFull`], compact result only.
    CoreJsonPortableFast,
    /// Cryptographic checks on a container that was already parsed from ZIP (no archive re-read).
    CoreJsonParsedFast,
    /// Phase II: canonical binary manifest encoding (not implemented in public alpha).
    CoreBinaryPortable,
    /// Phase II/III: compact authorization capsule for high-QPS PDP/PEP (not implemented).
    ZtTokenV1,
}

impl VerificationProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::CoreJsonPortableFull => "OSDF-Core-JSON portable (full report)",
            Self::CoreJsonPortableFast => "OSDF-Core-JSON portable (fast verify)",
            Self::CoreJsonParsedFast => "OSDF-Core-JSON parsed-container (fast revalidation)",
            Self::CoreBinaryPortable => "OSDF-Core-Binary portable (planned)",
            Self::ZtTokenV1 => "OSDF-ZT-Token v1 (planned)",
        }
    }

    pub fn parses_zip(self) -> bool {
        matches!(
            self,
            Self::CoreJsonPortableFull | Self::CoreJsonPortableFast
        )
    }
}

/// Compact pass/fail for gateway allow-or-deny decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastVerifyResult {
    Pass,
    Fail(FastFailCode),
}

impl FastVerifyResult {
    pub fn is_pass(self) -> bool {
        matches!(self, Self::Pass)
    }
}

/// Stable failure codes for fast verify (no string allocation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum FastFailCode {
    InvalidMagic = 1,
    UnsafeZipPath = 2,
    DuplicatePath = 3,
    TrailingBytes = 4,
    ManifestParseFailure = 5,
    MissingDeclaredObject = 6,
    ObjectSizeMismatch = 7,
    ObjectHashMismatch = 8,
    UndeclaredObject = 9,
    RevisionChainInvalid = 10,
    SignatureInvalid = 11,
    LedgerProofInvalid = 12,
    TrustPolicyRejected = 13,
    UnsupportedProfile = 14,
    EncryptedPayload = 15,
    ContainerError = 16,
}

impl FastFailCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidMagic => "INVALID_MAGIC",
            Self::UnsafeZipPath => "UNSAFE_ZIP_PATH",
            Self::DuplicatePath => "DUPLICATE_PATH",
            Self::TrailingBytes => "TRAILING_BYTES",
            Self::ManifestParseFailure => "MANIFEST_PARSE_FAILURE",
            Self::MissingDeclaredObject => "MISSING_DECLARED_OBJECT",
            Self::ObjectSizeMismatch => "OBJECT_SIZE_MISMATCH",
            Self::ObjectHashMismatch => "OBJECT_HASH_MISMATCH",
            Self::UndeclaredObject => "UNDECLARED_OBJECT",
            Self::RevisionChainInvalid => "REVISION_CHAIN_INVALID",
            Self::SignatureInvalid => "SIGNATURE_INVALID",
            Self::LedgerProofInvalid => "LEDGER_PROOF_INVALID",
            Self::TrustPolicyRejected => "TRUST_POLICY_REJECTED",
            Self::UnsupportedProfile => "UNSUPPORTED_PROFILE",
            Self::EncryptedPayload => "ENCRYPTED_PAYLOAD",
            Self::ContainerError => "CONTAINER_ERROR",
        }
    }
}

/// Immutable package parsed once from wire bytes.
///
/// Use [`crate::verify_fast::verify_parsed_package_fast`] for hot-path revalidation
/// (policy re-checks, cached gateway inspection). Each newly ingested file must still
/// be parsed at least once via [`crate::verify_fast::parse_package`].
#[derive(Debug, Clone)]
pub struct ParsedPackage {
    pub container: crate::container::PackageContainer,
    pub archive_bytes: u64,
}

impl ParsedPackage {
    pub fn archive_bytes(&self) -> u64 {
        self.archive_bytes
    }

    pub fn container(&self) -> &crate::container::PackageContainer {
        &self.container
    }
}
