use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use std::path::{Component, Path};

use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::constants::{
    suspicious_compression_ratio, HEADER_PATH, HEADER_SIZE, HEADER_VERSION, MAGIC, MAX_ENTRIES,
    MAX_UNCOMPRESSED_BYTES,
};
use crate::error::{OsdfError, Result};

#[derive(Debug, Clone)]
pub struct PackageEntry {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PackageContainer {
    pub entries: BTreeMap<String, PackageEntry>,
    pub archive_bytes: u64,
}

impl PackageContainer {
    pub fn read_from_path(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::read_from_bytes(&data)
    }

    pub fn read_from_bytes(data: &[u8]) -> Result<Self> {
        let archive_bytes = data.len() as u64;
        let cursor = std::io::Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)?;

        if archive.len() > MAX_ENTRIES {
            return Err(OsdfError::Container(format!(
                "too many entries: {} (max {MAX_ENTRIES})",
                archive.len()
            )));
        }

        let mut entries = BTreeMap::new();
        let mut seen_paths = HashSet::new();
        let mut total_uncompressed = 0u64;

        for index in 0..archive.len() {
            let mut file = archive.by_index(index)?;
            let raw_name = file.name().to_string();
            let normalized = normalize_zip_path(&raw_name)?;

            if !seen_paths.insert(normalized.clone()) {
                return Err(OsdfError::Container(format!(
                    "duplicate path: {normalized}"
                )));
            }

            if file.is_dir() {
                continue;
            }

            let compression_ratio = if file.compressed_size() == 0 {
                1
            } else {
                file.size().saturating_div(file.compressed_size())
            };
            if suspicious_compression_ratio(file.size(), file.compressed_size()) {
                return Err(OsdfError::Container(format!(
                    "compression bomb suspected at `{normalized}` (ratio {compression_ratio})"
                )));
            }

            total_uncompressed = total_uncompressed.saturating_add(file.size());
            if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
                return Err(OsdfError::Container(format!(
                    "uncompressed size exceeds limit ({MAX_UNCOMPRESSED_BYTES} bytes)"
                )));
            }

            let mut bytes = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut bytes)?;

            if bytes.len() as u64 != file.size() {
                return Err(OsdfError::Container(format!(
                    "size mismatch for `{normalized}`"
                )));
            }

            entries.insert(
                normalized.clone(),
                PackageEntry {
                    path: normalized,
                    bytes,
                },
            );
        }

        validate_header(entries.get(HEADER_PATH))?;

        Ok(Self {
            entries,
            archive_bytes,
        })
    }

    pub fn write_to_path(&self, path: &Path) -> Result<()> {
        let bytes = self.to_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buffer);
            let mut writer = ZipWriter::new(cursor);

            for (path, entry) in &self.entries {
                writer.start_file(path, zip_options_for(path))?;
                writer.write_all(&entry.bytes)?;
            }

            writer.finish()?;
        }
        Ok(buffer)
    }

    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.entries.get(path).map(|entry| entry.bytes.as_slice())
    }

    pub fn insert(&mut self, path: impl Into<String>, bytes: Vec<u8>) -> Result<()> {
        let path = path.into();
        normalize_zip_path(&path)?;
        self.entries
            .insert(path.clone(), PackageEntry { path, bytes });
        Ok(())
    }

    pub fn remove(&mut self, path: &str) -> Option<PackageEntry> {
        self.entries.remove(path)
    }

    pub fn paths(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    pub fn declared_paths(&self) -> BTreeMap<String, u64> {
        self.entries
            .iter()
            .map(|(path, entry)| (path.clone(), entry.bytes.len() as u64))
            .collect()
    }
}

pub fn normalize_zip_path(raw: &str) -> Result<String> {
    if raw.is_empty() {
        return Err(OsdfError::Container("empty path".to_string()));
    }

    if raw.contains('\\') {
        return Err(OsdfError::Container(format!(
            "backslashes are not allowed in paths: {raw}"
        )));
    }

    let path = Path::new(raw);
    let mut normalized = Vec::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_string_lossy();
                if part.contains("..") {
                    return Err(OsdfError::Container(format!(
                        "path traversal is not allowed: {raw}"
                    )));
                }
                normalized.push(part.to_string());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(OsdfError::Container(format!(
                    "absolute paths are not allowed: {raw}"
                )));
            }
            Component::ParentDir => {
                return Err(OsdfError::Container(format!(
                    "path traversal is not allowed: {raw}"
                )));
            }
            Component::CurDir => {}
        }
    }

    if normalized.is_empty() {
        return Err(OsdfError::Container(format!("invalid path: {raw}")));
    }

    Ok(normalized.join("/"))
}

fn zip_options_for(path: &str) -> SimpleFileOptions {
    if path == HEADER_PATH || path.ends_with(".json") {
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
    } else {
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
    }
}

fn validate_header(entry: Option<&PackageEntry>) -> Result<()> {
    let Some(entry) = entry else {
        return Err(OsdfError::Container(format!(
            "missing required header `{HEADER_PATH}`"
        )));
    };

    parse_header_bytes(&entry.bytes)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderInfo {
    pub version: u8,
    pub package_bytes: u64,
}

pub fn parse_header_bytes(bytes: &[u8]) -> Result<HeaderInfo> {
    if bytes.len() < HEADER_SIZE {
        return Err(OsdfError::Container(format!(
            "header must be at least {HEADER_SIZE} bytes"
        )));
    }

    if &bytes[..MAGIC.len()] != MAGIC.as_slice() {
        return Err(OsdfError::Container(
            "invalid magic bytes in osdf-header.bin".to_string(),
        ));
    }

    let version = bytes[MAGIC.len()];
    if version != HEADER_VERSION {
        return Err(OsdfError::Container(format!(
            "unsupported header version: {version}"
        )));
    }

    let package_bytes = u64::from_be_bytes(
        bytes[7..HEADER_SIZE]
            .try_into()
            .map_err(|_| OsdfError::Container("invalid header package_bytes field".to_string()))?,
    );

    Ok(HeaderInfo {
        version,
        package_bytes,
    })
}

pub fn make_header_bytes(package_bytes: u64) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    bytes.push(HEADER_VERSION);
    bytes.extend_from_slice(&package_bytes.to_be_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_paths() {
        assert!(normalize_zip_path("../etc/passwd").is_err());
        assert!(normalize_zip_path("content/../../secret").is_err());
        assert!(normalize_zip_path("/absolute/path").is_err());
    }

    #[test]
    fn accepts_normal_paths() {
        assert_eq!(
            normalize_zip_path("content/document.json").unwrap(),
            "content/document.json"
        );
    }
}
