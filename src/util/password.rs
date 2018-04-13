//! Password handling functions
//!
//! # Example
//!
//! ```
//! use util::{password, rand};
//!
//! fn main() {
//!     let password = "correct horse battery staple";
//!     let salt = rand::gen_random_bytes(32);
//!     let hash = password::generate_passhash(password.as_bytes(), &salt);
//!
//!     // verify password
//!     assert!(password::verify(password.as_bytes(), &hash, &salt));
//! }
//! ```

use argon2rs::{defaults, verifier, Argon2, Variant};

/// Generate the argon2i hash for the given password and salt
pub fn generate_passhash(password: &[u8], salt: &[u8]) -> Vec<u8> {
    let mut passhash = [0u8; defaults::LENGTH];
    let a2 = Argon2::default(Variant::Argon2i);
    a2.hash(&mut passhash, password, &salt, &[], &[]);

    passhash.to_vec()
}

/// Verify that he given password is identical to the stored one
pub fn verify(stored: &[u8], supplied: &[u8], salt: &[u8]) -> bool {
    let hash = generate_passhash(supplied, salt);

    verifier::constant_eq(&stored, &hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use util::rand;

    #[test]
    fn test_passhash() {
        let password = "correct horse battery staple";
        let salt = rand::gen_random_bytes(32);
        let hash = generate_passhash(password.as_bytes(), &salt);

        assert_eq!(hash.len(), defaults::LENGTH);
        assert!(verify(&hash, password.as_bytes(), &salt));
    }
}
