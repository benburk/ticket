/// Generate a short ticket ID like "tk-a1b2"
/// Uses prefix + random hex chars
pub fn generate(existing: &[String]) -> String {
    let prefix = "tk";

    for hex_len in 4..=8 {
        for _ in 0..100 {
            let mut bytes = [0u8; 8];
            getrandom::getrandom(&mut bytes).expect("failed to get random bytes");
            let hex = hex::encode(bytes);
            let id = format!("{}-{}", prefix, &hex[..hex_len]);

            if !existing.contains(&id) {
                return id;
            }
        }
    }

    // Fallback with longer hex
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).expect("failed to get random bytes");
    format!("{}-{}", prefix, hex::encode(bytes))
}
