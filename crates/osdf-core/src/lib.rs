pub mod canonical;
pub mod constants;
pub mod container;
pub mod crypto;
pub mod error;
pub mod identity;
pub mod ledger;
pub mod manifest;
pub mod merkle;
pub mod report;
pub mod revision;
pub mod signature;
pub mod types;
pub mod verify;
pub mod verify_audit;
pub mod verify_fast;
pub mod verify_profile;

#[cfg(feature = "native-create")]
pub mod build;

pub use container::PackageContainer;
pub use error::{OsdfError, Result};
#[cfg(feature = "native-create")]
pub use identity::{build_delegation_credential, sign_delegation_credential};
pub use identity::{
    IdentityConfig, IdentityPolicy, ResolvedSignerIdentity, TrustedOrganization, VerifierConfig,
};
#[cfg(feature = "native-create")]
pub use ledger::{
    append_revision_to_store, build_proof_for_store, create_ledger_store, find_leaf_index,
    load_signing_key, revision_event_hash_bytes, save_signing_key, trust_config_for_store,
};
pub use ledger::{
    build_transparency_proof, lookup_latest_revision, parse_transparency_proof,
    register_latest_revision, sign_tree_head, transparency_proof_path, verify_inclusion_proof,
    verify_signed_tree_head, verify_transparency_proof, AppendOnlyLog, DocumentLatestRevision,
    LatestRevisionPolicy, LedgerConfig, LedgerPolicy, LedgerSigningKeyFile, LedgerStore,
    TrustedLog,
};
pub use report::{
    Severity, VerificationCheck, VerificationFinding, VerificationMessage, VerificationMode,
    VerificationReport, VerificationSection, VerificationSectionReport, VerificationStatus,
    CORE_VERSION,
};
pub use verify::{
    inspect_container, verify_container, verify_container_with_config,
    verify_container_with_ledger, verify_package_bytes, verify_package_bytes_with_config,
    verify_package_bytes_with_ledger, InspectReport,
};
pub use verify_fast::{
    fast_fail_from_code, fast_fail_from_error, parse_package, verify_container_fast,
    verify_package_bytes_fast, verify_parsed_package_fast,
};
pub use verify_profile::{FastFailCode, FastVerifyResult, ParsedPackage, VerificationProfile};
#[cfg(feature = "native-create")]
pub use verify::{verify_package_path_with_config, verify_package_path_with_ledger};

#[cfg(feature = "native-create")]
pub use build::{
    attach_transparency_proof, commit_revision, create_package, create_package_with_document,
    write_package, CommitOptions, CreateOptions,
};

#[cfg(feature = "native-create")]
pub use signature::{generate_signing_key, verifying_key_to_urn};

#[cfg(feature = "native-create")]
pub use verify::verify_package_path;
