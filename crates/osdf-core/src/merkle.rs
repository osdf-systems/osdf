use crate::constants::HEADER_PATH;
use crate::crypto::{merkle_leaf, merkle_node};
use crate::types::ManifestObject;

pub fn merkle_scope_objects(objects: &[ManifestObject]) -> Vec<&ManifestObject> {
    objects
        .iter()
        .filter(|object| object.path.starts_with("content/") || object.path == HEADER_PATH)
        .collect()
}

pub fn merkle_root(objects: &[ManifestObject]) -> [u8; 32] {
    let scoped = merkle_scope_objects(objects);
    let mut leaves = Vec::with_capacity(scoped.len());

    for object in scoped {
        let digest = crate::crypto::parse_digest(&object.digest).expect("digest parsed in verify");
        leaves.push(merkle_leaf(
            &object.path,
            &object.object_type,
            object.bytes,
            &digest,
        ));
    }

    leaves.sort();

    if leaves.is_empty() {
        return [0u8; 32];
    }

    if leaves.len() == 1 {
        return leaves[0];
    }

    let mut level = leaves;
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut index = 0;
        while index < level.len() {
            let left = level[index];
            let right = if index + 1 < level.len() {
                level[index + 1]
            } else {
                level[index]
            };
            next.push(merkle_node(&left, &right));
            index += 2;
        }
        level = next;
    }

    level[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::format_digest;
    use crate::crypto::sha256_bytes;

    fn object(path: &str, object_type: &str, content: &[u8]) -> ManifestObject {
        ManifestObject {
            path: path.to_string(),
            object_type: object_type.to_string(),
            bytes: content.len() as u64,
            digest_algorithm: "SHA-256".to_string(),
            digest: format_digest(&sha256_bytes(content)),
        }
    }

    #[test]
    fn root_is_stable_for_sorted_inputs() {
        let a = object("a.txt", "binary", b"a");
        let b = object("b.txt", "binary", b"b");
        let root1 = merkle_root(&[a.clone(), b.clone()]);
        let root2 = merkle_root(&[b, a]);
        assert_eq!(root1, root2);
    }
}
