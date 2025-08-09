//! Utility functions

use crate::Result;

/// Generate a random string of specified length
pub fn generate_random_string(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    let mut rng = rand::thread_rng();

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Validate email format
pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

/// Hash a password
pub fn hash_password(password: &str) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}
