/*
 * ripalt
 * Copyright (C) 2018 Daniel Müller
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Utility functions for ripalt

pub mod rand;
pub mod password;
pub mod torrent;

use data_encoding::HEXLOWER;
use number_prefix::{binary_prefix, Prefixed, Standalone, PrefixNames};
use error::*;

const CHARS: &[u8] = b"0123456789abcdef";

pub fn to_hex(bytes: &[u8]) -> String {
    let mut v = Vec::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        v.push(CHARS[(byte >> 4) as usize]);
        v.push(CHARS[(byte & 0xf) as usize]);
    }

    unsafe { String::from_utf8_unchecked(v) }
}

pub fn from_hex(str: &str) -> Result<Vec<u8>> {
    HEXLOWER.decode(str.as_bytes())
        .map_err(|e| format!("decode from hex failed: {}", e).into())
}

pub fn data_size<T: Into<f64>>(bytes: T) -> String {
    let bytes: f64 = bytes.into();
    match binary_prefix(bytes) {
        Standalone(bytes) => format!("{} B", bytes),
        Prefixed(prefix, n) => format!("{:.2} {}B", n, prefix.symbol()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_to_hex() {
        assert_eq!(to_hex("foobar".as_bytes()), "666f6f626172");
    }
}
