use crate::constants::MANIFEST_PATH;
use crate::container::PackageContainer;
use crate::crypto::{
    digest_strings_equal, digests_equal, parse_digest, public_revision_commitment,
    revision_event_hash,
};
use crate::error::{OsdfError, Result};
use crate::manifest::parse_manifest;
use crate::types::RevisionRecord;

pub fn revision_path(revision: u32) -> String {
    format!("revisions/rev-{revision:06}.json")
}

pub fn parse_revision(container: &PackageContainer, revision: u32) -> Result<RevisionRecord> {
    let path = revision_path(revision);
    let bytes = container
        .get(&path)
        .ok_or_else(|| OsdfError::Revision(format!("missing `{path}`")))?;
    Ok(serde_json::from_slice(bytes)?)
}

pub fn list_revisions(container: &PackageContainer) -> Result<Vec<u32>> {
    let mut revisions = Vec::new();
    for path in container.paths() {
        if let Some(number) = path
            .strip_prefix("revisions/rev-")
            .and_then(|suffix| suffix.strip_suffix(".json"))
        {
            let revision = number
                .parse::<u32>()
                .map_err(|_| OsdfError::Revision(format!("invalid revision filename: {path}")))?;
            revisions.push(revision);
        }
    }
    revisions.sort_unstable();
    Ok(revisions)
}

pub fn verify_revision_chain(container: &PackageContainer) -> Result<()> {
    let manifest = parse_manifest(container)?;
    let revisions = list_revisions(container)?;

    if revisions.is_empty() {
        if manifest.revision > 0 {
            return Err(OsdfError::Revision(
                "missing revision records for committed package".to_string(),
            ));
        }
        return Ok(());
    }

    if revisions.last().copied() != Some(manifest.revision) {
        return Err(OsdfError::Revision(
            "manifest revision does not match latest revision record".to_string(),
        ));
    }

    let mut previous_hash: Option<[u8; 32]> = None;

    for revision_number in revisions {
        let record = parse_revision(container, revision_number)?;
        if record.revision != revision_number {
            return Err(OsdfError::Revision(format!(
                "revision record number mismatch at {revision_number}"
            )));
        }

        if record.document_id != manifest.document_id {
            return Err(OsdfError::Revision(
                "revision document_id mismatch".to_string(),
            ));
        }

        let root = parse_digest(&record.revision_root_hash)?;
        let commitment = parse_digest(&record.public_commitment)?;
        let salt = parse_digest(&record.revision_salt)?;
        let event_hash = parse_digest(&record.revision_event_hash)?;

        let expected_commitment =
            public_revision_commitment(&record.document_id, record.revision, &salt, &root);
        if !digests_equal(&expected_commitment, &commitment) {
            return Err(OsdfError::Revision(
                "public commitment mismatch".to_string(),
            ));
        }

        if record.revision == manifest.revision {
            if !digest_strings_equal(&manifest.public_commitment, &record.public_commitment) {
                return Err(OsdfError::Revision(
                    "manifest public commitment mismatch".to_string(),
                ));
            }
            if !digest_strings_equal(&manifest.revision_root_hash, &record.revision_root_hash) {
                return Err(OsdfError::Revision(
                    "manifest revision root mismatch".to_string(),
                ));
            }
        }

        let parent = record
            .parent_revision_hash
            .as_deref()
            .map(parse_digest)
            .transpose()?;

        if revision_number == 1 {
            if parent.is_some() {
                return Err(OsdfError::Revision(
                    "initial revision must not have a parent hash".to_string(),
                ));
            }
        } else if parent != previous_hash {
            return Err(OsdfError::Revision(format!(
                "broken revision chain before revision {revision_number}"
            )));
        }

        let expected_event = revision_event_hash(
            &record.document_id,
            record.revision,
            parent.as_ref(),
            &root,
            &commitment,
            &record.committed_timestamp,
            &record.signer_key_reference,
        );

        if !digests_equal(&expected_event, &event_hash) {
            return Err(OsdfError::Revision(
                "revision event hash mismatch".to_string(),
            ));
        }

        previous_hash = Some(event_hash);
    }

    if let Some(parent_commitment) = &manifest.parent_revision_commitment {
        if manifest.revision <= 1 {
            return Err(OsdfError::Revision(
                "parent revision commitment set on initial revision".to_string(),
            ));
        }
        let prior = parse_revision(container, manifest.revision - 1)?;
        if !digest_strings_equal(&prior.public_commitment, parent_commitment) {
            return Err(OsdfError::Revision(
                "parent revision commitment mismatch".to_string(),
            ));
        }
    }

    let _ = MANIFEST_PATH;
    Ok(())
}
