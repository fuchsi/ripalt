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

//! Torrent and Torrent file related functions

use super::super::error::*;
use bip_bencode::{BDecodeOpt, BRefAccess, BencodeRef};
use ring::digest;
use serde_bencode::{self, value::Value};

/// Calculate the info hash for a torrent meta file
///
/// # Example
///
/// ```
/// use util::torrent::*;
/// use util::to_hex;
///
/// fn main() {
///     let torrent = b"d4:infod6:lengthi283115520e4:name34:install-amd64-minimal-20170907.iso12:piece lengthi16777216e6:pieces0:ee";
///     let hash = "b81090f152528a339402d9f6a41eb0addd4f5ef0";
///
///     assert_eq!(info_hash, to_hex(info_hash(torrent).unwrap());
/// }
/// ```
pub fn info_hash(data: &[u8]) -> Result<Vec<u8>> {
    let bencode = BencodeRef::decode(data, BDecodeOpt::default())?;
    let info = bencode
        .dict()
        .ok_or("meta file is no dict")?
        .lookup(b"info")
        .ok_or("info not found")?
        .buffer();

    println!("{}", String::from_utf8_lossy(info));
    let digest = digest::digest(&digest::SHA1, info);
    Ok(digest.as_ref().to_vec())
}

/// Get all files for a torrent meta file
pub fn files(data: &[u8]) -> Result<Vec<(String, i64)>> {
    use ::std::fmt::Write;
    let bencode = BencodeRef::decode(data, BDecodeOpt::default())?;
    let info = bencode
        .dict()
        .ok_or("meta file is no dict")?
        .lookup(b"info")
        .ok_or("info not found")?
        .dict()
        .ok_or("info is no dict")?;

    match info.lookup(b"files") {
        // multiple file mode
        Some(f) => {
            let f = f.list().ok_or("files is not a list")?;
            let mut files: Vec<(String, i64)> = Vec::new();
            let mut iter =  f.into_iter();

            for entry in iter {
                let file = entry.dict().ok_or("file entry is not a dict")?;
                let size = file.lookup(b"length")
                    .ok_or("length not found in file dict")?
                    .int()
                    .ok_or("length is not an int")?;
                let name = file.lookup(b"path")
                    .ok_or("path not found in file dict")?
                    .list()
                    .ok_or("path is not a list")?
                    .into_iter()
                    .map(|e| e.str().unwrap_or("").to_owned())
                    .fold(String::new(), |mut acc, x| {write!(&mut acc, "/{}", x).unwrap(); acc});

                files.push(((&name[1..]).to_owned(), size));
            }
            Ok(files)
        }
        // single file mode
        None => {
            let size = info.lookup(b"length")
                .ok_or("length not found in info dict")?
                .int()
                .ok_or("length is not an int")?;
            let name = info.lookup(b"name")
                .ok_or("name not found in info dict")?
                .str()
                .ok_or("name is not a str")?;

            Ok(vec![(name.to_owned(), size)])
        }
    }
}

pub fn rewrite(data: &[u8], announce_url: &str, comment: &str) -> Result<Vec<u8>> {
    let value = serde_bencode::from_bytes(data)?;
    if let Value::Dict(mut root) = value {
        root.insert(b"announce".to_vec(), Value::Bytes(announce_url.as_bytes().to_vec()));
        root.insert(b"comment".to_vec(), Value::Bytes(comment.as_bytes().to_vec()));
        root.remove(b"announce-list".as_ref());

        serde_bencode::to_bytes(&Value::Dict(root))
            .map_err(|e| e.into())
    } else {
        bail!("meta file is no dict");
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use ::util;

    fn single_file_torrent() -> (String, Vec<u8>) {
        let info_hash = "b81090f152528a339402d9f6a41eb0addd4f5ef0".to_string();
        let torrent = b"d4:infod6:lengthi283115520e4:name34:install-amd64-minimal-20170907.iso12:piece lengthi16777216e6:pieces0:ee".to_vec();

        (info_hash, torrent)
    }

    fn multiple_file_torrent() -> (String, Vec<u8>) {
        let info_hash = "f0b50dd6e052c2d04ca45239634bf6333367ba7f".to_string();
        let torrent = b"d4:infod5:filesld6:lengthi283115520e4:pathl34:install-amd64-minimal-20170907.isoeed6:lengthi522190848e4:pathl29:archlinux-2013.02.01-dual.isoeee4:name4:test12:piece lengthi16777216e6:pieces0:ee".to_vec();

        (info_hash, torrent)
    }

    fn multiple_dir_torrent() -> (String, Vec<u8>) {
        let info_hash = "484634d4a484d0f1741a6f95f7f45f34e9166e44".to_string();
        let torrent = b"d4:infod5:filesld6:lengthi522190848e4:pathl4:arch29:archlinux-2013.02.01-dual.isoeed6:lengthi283115520e4:pathl6:gentoo34:install-amd64-minimal-20170907.isoeee4:name4:test12:piece lengthi16777216e6:pieces0:ee".to_vec();

        (info_hash, torrent)
    }

    #[test]
    fn test_info_hash() {
        let (ih, t) = single_file_torrent();
        let calculated = info_hash(&t).unwrap();
        assert_eq!(util::to_hex(&calculated), ih);

        let (ih, t) = multiple_file_torrent();
        let calculated = info_hash(&t).unwrap();
        assert_eq!(util::to_hex(&calculated), ih);

        let (ih, t) = multiple_dir_torrent();
        let calculated = info_hash(&t).unwrap();
        assert_eq!(util::to_hex(&calculated), ih);
    }

    #[test]
    fn test_files_single() {
        let (_, t) = single_file_torrent();
        let mut f = files(&t).unwrap();
        let (file, size) = f.pop().unwrap();
        assert_eq!(String::from("install-amd64-minimal-20170907.iso"), file);
        assert_eq!(283115520, size);
    }

    #[test]
    fn test_files_multiple() {
        let (_, t) = multiple_file_torrent();
        let mut f = files(&t).unwrap();

        let (file, size) = f.pop().unwrap();
        assert_eq!(String::from("archlinux-2013.02.01-dual.iso"), file);
        assert_eq!(522190848, size);

        let (file, size) = f.pop().unwrap();
        assert_eq!(String::from("install-amd64-minimal-20170907.iso"), file);
        assert_eq!(283115520, size);
    }

    #[test]
    fn test_files_multiple_dir() {
        let (_, t) = multiple_dir_torrent();
        let mut f = files(&t).unwrap();

        let (file, size) = f.pop().unwrap();
        assert_eq!(String::from("gentoo/install-amd64-minimal-20170907.iso"), file);
        assert_eq!(283115520, size);

        let (file, size) = f.pop().unwrap();
        assert_eq!(String::from("arch/archlinux-2013.02.01-dual.iso"), file);
        assert_eq!(522190848, size);
    }
}