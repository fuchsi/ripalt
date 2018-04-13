//! RNG related functions

use rand::{OsRng, Rng};
use std::cell::RefCell;

/// Generate `len` random bytes
///
/// # Panics
///
/// The function panics if `len` is 0
///
/// # Example
///
/// ```
/// use util::rand;
///
/// fn main() {
///     let bytes = rand::gen_random_bytes(32);
///
///     assert!(bytes.len() == 32);
/// }
/// ```
pub fn gen_random_bytes(len: usize) -> Vec<u8> {
    assert!(len > 0);
    thread_local!(static RNG: RefCell<OsRng> = RefCell::new(OsRng::new().unwrap()));

    RNG.with(|rng| {
        let mut brng = rng.borrow_mut();

        let mut salt: Vec<u8> = Vec::with_capacity(len);
        unsafe {
            salt.set_len(len);
        }

        brng.fill_bytes(&mut salt);

        salt
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_bytes() {
        let bytes = gen_random_bytes(16);
        assert_eq!(bytes.len(), 16);
        assert_ne!(bytes, gen_random_bytes(16));
    }

    #[test]
    #[should_panic]
    fn test_random_bytes_panics() {
        gen_random_bytes(0);
    }
}