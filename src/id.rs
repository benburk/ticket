use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generate a short ticket ID like "tk-a1b2"
/// Uses 2-letter prefix + 4 hex chars from UUID hash
pub fn generate(existing: &[String]) -> String {
    let prefix = "tk";

    for hash_len in 4..=8 {
        for _ in 0..100 {
            let uuid = Uuid::new_v4();
            let mut hasher = Sha256::new();
            hasher.update(uuid.as_bytes());
            let hash = hasher.finalize();
            let hex = hex::encode(&hash[..]);
            let id = format!("{}-{}", prefix, &hex[..hash_len]);

            if !existing.contains(&id) {
                return id;
            }
        }
    }

    // Fallback with longer hash
    let uuid = Uuid::new_v4();
    let mut hasher = Sha256::new();
    hasher.update(uuid.as_bytes());
    let hash = hasher.finalize();
    format!("{}-{}", prefix, hex::encode(&hash[..8]))
}

/// Generate a child ID for hierarchical tickets
/// e.g., "tk-a1b2" -> "tk-a1b2.1"
pub fn generate_child(parent_id: &str, existing: &[String]) -> String {
    let prefix = format!("{}.", parent_id);

    let max_num = existing
        .iter()
        .filter(|id| id.starts_with(&prefix))
        .filter_map(|id| {
            let suffix = &id[prefix.len()..];
            if !suffix.contains('.') {
                suffix.parse::<u32>().ok()
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);

    format!("{}{}", prefix, max_num + 1)
}
