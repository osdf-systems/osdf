pub const MAGIC: &[u8; 6] = b"OSDF\0\x01";
pub const FORMAT_VERSION: &str = "1.0-draft";
pub const PROFILE_CORE: &str = "OSDF-Core";

pub const DOMAIN_OBJECT: &str = "OSDF-OBJECT-v1";
pub const DOMAIN_LEAF: &str = "OSDF-LEAF-v1";
pub const DOMAIN_NODE: &str = "OSDF-NODE-v1";
pub const DOMAIN_REV_EVENT: &str = "OSDF-REV-EVENT-v1";
pub const DOMAIN_REVISION: &str = "OSDF-REVISION-v1";
pub const DOMAIN_LOG_LEAF: &str = "OSDF-LOG-LEAF-v1";
pub const DOMAIN_LOG_NODE: &str = "OSDF-LOG-NODE-v1";

pub const HEADER_VERSION: u8 = 1;
pub const HEADER_SIZE: usize = 15; // magic(6) + version(1) + package_bytes(8)
pub const HEADER_PATH: &str = "osdf-header.bin";
pub const ENVELOPE_PATH: &str = "public-envelope.json";
pub const MANIFEST_PATH: &str = "manifests/package-manifest.json";

pub const MAX_ENTRIES: usize = 10_000;
pub const MAX_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_COMPRESSION_RATIO: u64 = 200;
