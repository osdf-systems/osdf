use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use base64::Engine;
use ed25519_dalek::Verifier;
#[cfg(feature = "native-create")]
use ed25519_dalek::{Signer, SigningKey};

use crate::canonical::canonicalize_json;
use crate::error::{OsdfError, Result};
use crate::signature::verifying_key_from_urn;
#[cfg(feature = "native-create")]
use crate::signature::verifying_key_to_urn;
use crate::types::SigningDelegationCredential;

pub const CREDENTIAL_TYPE_DELEGATION: &str = "OSDF_ORGANIZATION_SIGNING_DELEGATION";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum IdentityPolicy {
    #[default]
    Disabled,
    Optional,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IdentityConfig {
    #[serde(default)]
    pub policy: IdentityPolicy,
    #[serde(default)]
    pub organizations: Vec<TrustedOrganization>,
    #[serde(default)]
    pub delegations: Vec<SigningDelegationCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrustedOrganization {
    pub organization_id: String,
    pub display_name: String,
    pub root_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSignerIdentity {
    pub signer_key: String,
    pub organization_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    pub resolution_path: String,
    pub signing_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerifierConfig {
    #[serde(default)]
    pub ledger: crate::ledger::LedgerConfig,
    #[serde(default)]
    pub identity: IdentityConfig,
}

impl IdentityConfig {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

pub fn resolve_signer_identity(
    signer_key: &str,
    signing_timestamp: &str,
    config: &IdentityConfig,
) -> Result<Option<ResolvedSignerIdentity>> {
    if config.policy == IdentityPolicy::Disabled {
        return Ok(None);
    }

    let signing_at = parse_timestamp(signing_timestamp)?;

    for organization in &config.organizations {
        if organization.root_keys.iter().any(|key| key == signer_key) {
            return Ok(Some(ResolvedSignerIdentity {
                signer_key: signer_key.to_string(),
                organization_id: organization.organization_id.clone(),
                display_name: organization.display_name.clone(),
                department: None,
                resolution_path: "organization-root-key".to_string(),
                signing_timestamp: signing_timestamp.to_string(),
            }));
        }
    }

    for delegation in &config.delegations {
        if delegation.subject_key != signer_key {
            continue;
        }

        verify_delegation_credential(delegation, config)?;
        let organization = config
            .organizations
            .iter()
            .find(|org| org.organization_id == delegation.organization_id)
            .ok_or_else(|| {
                OsdfError::Signature(format!(
                    "delegation references unknown organization `{}`",
                    delegation.organization_id
                ))
            })?;

        let valid_from = parse_timestamp(&delegation.valid_from)?;
        let valid_until = parse_timestamp(&delegation.valid_until)?;
        if signing_at < valid_from || signing_at > valid_until {
            return Err(OsdfError::Signature(format!(
                "signing timestamp {signing_timestamp} outside delegation validity window"
            )));
        }

        return Ok(Some(ResolvedSignerIdentity {
            signer_key: signer_key.to_string(),
            organization_id: organization.organization_id.clone(),
            display_name: organization.display_name.clone(),
            department: delegation.department.clone(),
            resolution_path: "organization-signing-delegation".to_string(),
            signing_timestamp: signing_timestamp.to_string(),
        }));
    }

    Ok(None)
}

pub fn verify_delegation_credential(
    credential: &SigningDelegationCredential,
    config: &IdentityConfig,
) -> Result<()> {
    if credential.credential_type != CREDENTIAL_TYPE_DELEGATION {
        return Err(OsdfError::Signature(format!(
            "unsupported credential type: {}",
            credential.credential_type
        )));
    }

    let organization = config
        .organizations
        .iter()
        .find(|org| org.organization_id == credential.organization_id)
        .ok_or_else(|| {
            OsdfError::Signature(format!(
                "delegation organization `{}` is not in trust registry",
                credential.organization_id
            ))
        })?;

    if !organization
        .root_keys
        .iter()
        .any(|key| key == &credential.issuer_key)
    {
        return Err(OsdfError::Signature(format!(
            "delegation issuer key `{}` is not a trusted root key for `{}`",
            credential.issuer_key, organization.organization_id
        )));
    }

    let verifying_key = verifying_key_from_urn(&credential.issuer_key)?;
    let payload = delegation_signing_payload(credential)?;
    let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&credential.signature)
        .map_err(|err| {
            OsdfError::Signature(format!("invalid delegation signature encoding: {err}"))
        })?;
    let signature = ed25519_dalek::Signature::from_slice(&signature_bytes).map_err(|err| {
        OsdfError::Signature(format!("invalid ed25519 delegation signature bytes: {err}"))
    })?;
    verifying_key
        .verify(&payload, &signature)
        .map_err(|err| OsdfError::Signature(format!("delegation credential invalid: {err}")))?;

    Ok(())
}

pub fn delegation_signing_payload(credential: &SigningDelegationCredential) -> Result<Vec<u8>> {
    let mut unsigned = credential.clone();
    unsigned.signature = String::new();
    canonicalize_json(&serde_json::to_value(&unsigned)?)
}

#[cfg(feature = "native-create")]
pub fn sign_delegation_credential(
    issuer_key: &SigningKey,
    credential: &mut SigningDelegationCredential,
) -> Result<()> {
    credential.issuer_key = verifying_key_to_urn(&issuer_key.verifying_key());
    credential.credential_type = CREDENTIAL_TYPE_DELEGATION.to_string();
    let payload = delegation_signing_payload(credential)?;
    let signature = issuer_key.sign(&payload);
    credential.signature =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes());
    Ok(())
}

#[cfg(feature = "native-create")]
pub fn build_delegation_credential(
    organization_id: &str,
    department: Option<&str>,
    subject_key: &str,
    valid_from: &str,
    valid_until: &str,
) -> SigningDelegationCredential {
    SigningDelegationCredential {
        credential_type: CREDENTIAL_TYPE_DELEGATION.to_string(),
        organization_id: organization_id.to_string(),
        department: department.map(str::to_string),
        subject_key: subject_key.to_string(),
        valid_from: valid_from.to_string(),
        valid_until: valid_until.to_string(),
        issuer_key: String::new(),
        signature: String::new(),
    }
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .or_else(|_| {
            value
                .parse::<DateTime<Utc>>()
                .map_err(|err| OsdfError::Signature(format!("invalid timestamp `{value}`: {err}")))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::generate_signing_key;

    #[test]
    fn resolves_delegated_signer() {
        let root_key = generate_signing_key();
        let delegate_key = generate_signing_key();
        let root_urn = verifying_key_to_urn(&root_key.verifying_key());
        let delegate_urn = verifying_key_to_urn(&delegate_key.verifying_key());

        let mut delegation = build_delegation_credential(
            "urn:osdf:org:demo",
            Some("Department of Revenue"),
            &delegate_urn,
            "2020-01-01T00:00:00Z",
            "2030-01-01T00:00:00Z",
        );
        sign_delegation_credential(&root_key, &mut delegation).unwrap();

        let config = IdentityConfig {
            policy: IdentityPolicy::Required,
            organizations: vec![TrustedOrganization {
                organization_id: "urn:osdf:org:demo".to_string(),
                display_name: "State of Colorado".to_string(),
                root_keys: vec![root_urn],
            }],
            delegations: vec![delegation],
        };

        let resolved = resolve_signer_identity(&delegate_urn, "2026-06-04T18:00:00Z", &config)
            .unwrap()
            .expect("identity should resolve");

        assert_eq!(resolved.display_name, "State of Colorado");
        assert_eq!(
            resolved.department.as_deref(),
            Some("Department of Revenue")
        );
        assert_eq!(resolved.resolution_path, "organization-signing-delegation");
    }
}
