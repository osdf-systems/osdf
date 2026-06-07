mod demo;

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use osdf_core::{
    append_revision_to_store, attach_transparency_proof, build_proof_for_store, commit_revision,
    create_ledger_store, create_package, find_leaf_index, generate_signing_key, inspect_container,
    load_signing_key, revision_event_hash_bytes, save_signing_key, trust_config_for_store,
    verify_package_path_with_config, write_package, CommitOptions, CreateOptions, IdentityPolicy,
    LatestRevisionPolicy, LedgerPolicy, LedgerStore, PackageContainer, VerificationStatus,
    VerifierConfig, CORE_VERSION,
};

#[derive(Parser)]
#[command(name = "osdf", version, about = "Open Secure Document Format CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate an OSDF package
    Verify {
        #[arg(value_parser = existing_file_path)]
        path: PathBuf,
        /// Output format (`human` or `json`)
        #[arg(long, value_name = "FORMAT", default_value = "human")]
        format: OutputFormatArg,
        /// Emit JSON verification report (alias for `--format json`)
        #[arg(long, default_value_t = false)]
        json: bool,
        /// JSON file with ledger trust configuration (`LedgerConfig`)
        #[arg(long, value_parser = existing_file_path)]
        ledger_config: Option<PathBuf>,
        /// JSON file with organizational identity trust configuration (`IdentityConfig`)
        #[arg(long, value_parser = existing_file_path)]
        identity_config: Option<PathBuf>,
        /// Require transparency ledger proof (overrides config policy)
        #[arg(long, value_enum, conflicts_with = "ledger_config")]
        ledger_policy: Option<LedgerPolicyArg>,
        /// Require signer organizational identity resolution (overrides config policy)
        #[arg(long, value_enum, conflicts_with = "identity_config")]
        identity_policy: Option<IdentityPolicyArg>,
        /// Compare local revision against configured latest-revision registry
        #[arg(long, value_enum)]
        latest_revision_policy: Option<LatestRevisionPolicyArg>,
    },
    /// Show package metadata and declared objects
    Inspect {
        path: PathBuf,
        /// Output format (`human` or `json`)
        #[arg(long, value_name = "FORMAT", default_value = "human")]
        format: OutputFormatArg,
        /// Emit JSON (alias for `--format json`)
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Print tool and library versions
    Version,
    /// Create a new OSDF package
    Create {
        output: PathBuf,
        #[arg(long, default_value = "Untitled Document")]
        title: String,
        #[arg(long)]
        document_id: Option<String>,
        #[arg(long, default_value_t = false)]
        commit: bool,
    },
    /// Commit a new signed revision to an existing package
    CommitRevision {
        path: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Manage a file-backed transparency ledger
    Ledger {
        #[command(subcommand)]
        command: LedgerCommands,
    },
    /// Narrated verification demo for README / stakeholder walkthrough
    Demo {
        #[command(subcommand)]
        command: DemoCommands,
    },
}

#[derive(Subcommand)]
enum DemoCommands {
    /// Verify PASS, tamper 1 byte, verify FAIL — with timings
    Safety {
        /// Signed OSDF package to verify (defaults to committed fixture)
        #[arg(value_parser = existing_file_path)]
        path: Option<PathBuf>,
        /// Write SVG assets for README (`demo-verify-pass.svg`, `demo-verify-fail.svg`)
        #[arg(long, value_name = "DIR")]
        write_readme_assets: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum LedgerCommands {
    /// Initialize a new append-only ledger store and signing key
    Init {
        /// Ledger store JSON output path
        #[arg(long)]
        store: PathBuf,
        /// Log operator signing key output path
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        log_id: Option<String>,
    },
    /// Append a package revision event hash to the ledger
    Append {
        #[arg(long)]
        store: PathBuf,
        #[arg(long, value_parser = existing_file_path)]
        package: PathBuf,
    },
    /// Attach a transparency proof from the ledger to a package
    AttachProof {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        key: PathBuf,
        #[arg(long, value_parser = existing_file_path)]
        package: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        /// Write a matching trust config JSON for `osdf verify --ledger-config`
        #[arg(long)]
        trust_config: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LedgerPolicyArg {
    Optional,
    Required,
}

impl From<LedgerPolicyArg> for LedgerPolicy {
    fn from(value: LedgerPolicyArg) -> Self {
        match value {
            LedgerPolicyArg::Optional => LedgerPolicy::Optional,
            LedgerPolicyArg::Required => LedgerPolicy::Required,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum IdentityPolicyArg {
    Optional,
    Required,
}

impl From<IdentityPolicyArg> for IdentityPolicy {
    fn from(value: IdentityPolicyArg) -> Self {
        match value {
            IdentityPolicyArg::Optional => IdentityPolicy::Optional,
            IdentityPolicyArg::Required => IdentityPolicy::Required,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormatArg {
    Human,
    Json,
}

impl OutputFormatArg {
    fn json_output(self, legacy_json_flag: bool) -> bool {
        legacy_json_flag || matches!(self, Self::Json)
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LatestRevisionPolicyArg {
    Optional,
    Required,
}

impl From<LatestRevisionPolicyArg> for LatestRevisionPolicy {
    fn from(value: LatestRevisionPolicyArg) -> Self {
        match value {
            LatestRevisionPolicyArg::Optional => LatestRevisionPolicy::Optional,
            LatestRevisionPolicyArg::Required => LatestRevisionPolicy::Required,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Verify {
            path,
            format,
            json,
            ledger_config,
            identity_config,
            ledger_policy,
            identity_policy,
            latest_revision_policy,
        } => {
            let config = load_verifier_config(
                ledger_config.as_deref(),
                ledger_policy,
                identity_config.as_deref(),
                identity_policy,
                latest_revision_policy,
            )?;
            let report = verify_package_path_with_config(&path, &config);
            if format.json_output(json) {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_verify_report(&report);
            }
            if report.overall == VerificationStatus::Fail {
                std::process::exit(1);
            }
        }
        Commands::Inspect { path, format, json } => {
            let container = PackageContainer::read_from_path(&path)?;
            let report = inspect_container(&container)?;
            if format.json_output(json) {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_inspect_report(&report);
            }
        }
        Commands::Version => {
            println!("osdf {}", env!("CARGO_PKG_VERSION"));
            println!("osdf-core {CORE_VERSION}");
        }
        Commands::Create {
            output,
            title,
            document_id,
            commit,
        } => {
            let signing_key = if commit {
                Some(generate_signing_key())
            } else {
                None
            };
            let container = create_package(CreateOptions {
                title,
                document_id,
                signing_key,
                commit,
            })?;
            write_package(&container, &output)?;
            eprintln!("created {}", output.display());
        }
        Commands::CommitRevision { path, output } => {
            let output = output.unwrap_or(path.clone());
            let mut container = PackageContainer::read_from_path(&path)?;
            let revision = commit_revision(
                &mut container,
                CommitOptions {
                    signing_key: generate_signing_key(),
                    signer_key_reference: None,
                },
            )?;
            write_package(&container, &output)?;
            eprintln!("committed revision {revision} to {}", output.display());
        }
        Commands::Ledger { command } => match command {
            LedgerCommands::Init { store, key, log_id } => {
                let signing_key = generate_signing_key();
                let ledger_store = create_ledger_store(log_id, &signing_key);
                save_signing_key(&key, &ledger_store.log_id, &signing_key)?;
                ledger_store.save(&store)?;
                eprintln!("ledger store: {}", store.display());
                eprintln!("log signing key: {}", key.display());
                eprintln!("log id: {}", ledger_store.log_id);
            }
            LedgerCommands::Append { store, package } => {
                let mut ledger_store = LedgerStore::load(&store)?;
                let container = PackageContainer::read_from_path(&package)?;
                let manifest = osdf_core::manifest::parse_manifest(&container)?;
                if manifest.revision == 0 {
                    anyhow::bail!("ledger append requires a committed revision (revision > 0)");
                }
                let event_hash = revision_event_hash_bytes(&container, manifest.revision)?;
                if find_leaf_index(&ledger_store, &event_hash).is_some() {
                    anyhow::bail!("revision event hash already present in ledger");
                }
                let index = append_revision_to_store(
                    &mut ledger_store,
                    &manifest.document_id,
                    manifest.revision,
                    event_hash,
                );
                ledger_store.save(&store)?;
                eprintln!(
                    "appended revision {} at leaf index {index} to {}",
                    manifest.revision,
                    store.display()
                );
            }
            LedgerCommands::AttachProof {
                store,
                key,
                package,
                output,
                trust_config,
            } => {
                let ledger_store = LedgerStore::load(&store)?;
                let (key_log_id, log_signing_key) = load_signing_key(&key)?;
                if key_log_id != ledger_store.log_id {
                    anyhow::bail!("log signing key log id does not match ledger store");
                }
                let output = output.unwrap_or(package.clone());
                let mut container = PackageContainer::read_from_path(&package)?;
                let manifest = osdf_core::manifest::parse_manifest(&container)?;
                if manifest.revision == 0 {
                    anyhow::bail!("attach-proof requires a committed revision (revision > 0)");
                }
                let event_hash = revision_event_hash_bytes(&container, manifest.revision)?;
                let leaf_index = find_leaf_index(&ledger_store, &event_hash).ok_or_else(|| {
                    anyhow::anyhow!(
                        "revision event hash not found in ledger; run `osdf ledger append` first"
                    )
                })?;
                let proof =
                    build_proof_for_store(&ledger_store, leaf_index, event_hash, &log_signing_key)?;
                attach_transparency_proof(&mut container, proof)?;
                write_package(&container, &output)?;
                eprintln!("attached transparency proof to {}", output.display());
                if let Some(trust_path) = trust_config {
                    let config = trust_config_for_store(&ledger_store, LedgerPolicy::Required);
                    std::fs::write(&trust_path, serde_json::to_vec_pretty(&config)?)?;
                    eprintln!("trust config: {}", trust_path.display());
                }
            }
        },
        Commands::Demo { command } => match command {
            DemoCommands::Safety {
                path,
                write_readme_assets,
            } => {
                demo::run_safety_demo(path.as_deref(), write_readme_assets.as_deref())?;
            }
        },
    }

    Ok(())
}

fn load_verifier_config(
    ledger_path: Option<&std::path::Path>,
    ledger_policy: Option<LedgerPolicyArg>,
    identity_path: Option<&std::path::Path>,
    identity_policy: Option<IdentityPolicyArg>,
    latest_revision_policy: Option<LatestRevisionPolicyArg>,
) -> anyhow::Result<VerifierConfig> {
    let mut config = VerifierConfig::default();

    if let Some(path) = ledger_path {
        let bytes = std::fs::read(path)
            .with_context(|| format!("failed to read ledger config file `{}`", path.display()))?;
        config.ledger = serde_json::from_slice(&bytes).with_context(|| {
            format!("failed to parse ledger config JSON in `{}`", path.display())
        })?;
    }
    if let Some(policy) = ledger_policy {
        config.ledger.policy = policy.into();
    }
    if let Some(policy) = latest_revision_policy {
        config.ledger.latest_revision_policy = policy.into();
    }

    if let Some(path) = identity_path {
        let bytes = std::fs::read(path)
            .with_context(|| format!("failed to read identity config file `{}`", path.display()))?;
        config.identity = serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "failed to parse identity config JSON in `{}`",
                path.display()
            )
        })?;
    }
    if let Some(policy) = identity_policy {
        config.identity.policy = policy.into();
    }

    Ok(config)
}

fn existing_file_path(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("file not found: {value}"))
    }
}

fn print_verify_report(report: &osdf_core::VerificationReport) {
    println!("Document verification\n");
    println!("Overall: {}\n", report.overall.as_str());

    for section in &report.sections {
        println!("{}", section.title);
        for check in &section.checks {
            println!("  [{}] {}", check.status.as_str(), check.label);
            if let Some(details) = &check.details {
                println!("    {details}");
            }
        }
        println!();
    }

    if let Some(document_id) = &report.document_id {
        println!("Document ID: {document_id}");
    }
    if let Some(revision) = report.revision {
        println!("Revision: {revision}");
    }
    if let Some(profile) = &report.profile {
        println!("Profile: {profile}");
    }
    println!("Signatures: {}", report.signature_count);

    for finding in &report.findings {
        if finding.severity == osdf_core::Severity::Info {
            continue;
        }
        println!(
            "\n[{}] {}",
            match finding.severity {
                osdf_core::Severity::Fail => "FAIL",
                osdf_core::Severity::Warning => "WARN",
                osdf_core::Severity::Info => "INFO",
            },
            finding.summary
        );
        println!("  {}", finding.impact);
        println!("  ({})", finding.code);
    }
}

fn print_inspect_report(report: &osdf_core::InspectReport) {
    println!("OSDF inspect\n");
    println!("Document ID: {}", report.document_id);
    println!("Revision: {}", report.revision);
    println!("Profile: {}", report.profile);
    println!("Package bytes: {}", report.package_bytes);
    println!("Content bytes: {}", report.content_bytes);
    println!("Objects: {}", report.object_count);
    println!("\nPaths:");
    for path in &report.paths {
        println!("  - {path}");
    }
    if !report.signatures.is_empty() {
        println!("\nSignatures:");
        for signature in &report.signatures {
            println!(
                "  - revision {} by {}",
                signature.revision, signature.signer_key
            );
        }
    }
}
