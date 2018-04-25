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
    let bencode = serde_bencode::from_bytes(data)?;
    if let Value::Dict(root) = bencode {
        if let Some(info) = root.get(&b"info".to_vec()) {
            let info = serde_bencode::to_bytes(info)?;
            let digest = digest::digest(&digest::SHA1, &info);
            Ok(digest.as_ref().to_vec())
        } else {
            bail!("info dict not found");
        }
    } else {
        bail!("meta file is no dict");
    }
}

/// Get all files for a torrent meta file
pub fn files(data: &[u8]) -> Result<Vec<(String, i64)>> {
    use ::std::fmt::Write;
    let bencode = serde_bencode::from_bytes(data)?;
    let root = if let Value::Dict(root) = bencode {
        root
    } else {
        bail!("meta file is no dict");
    };
    let info = if let Some(Value::Dict(info)) = root.get(&b"info".to_vec()) {
        info
    } else {
        bail!("info dict not found");
    };

    match info.get(&b"files".to_vec()) {
        // multiple file mode
        Some(Value::List(f)) => {
            let mut files: Vec<(String, i64)> = Vec::new();
            let mut iter =  f.into_iter();

            for entry in iter {
                let file = match entry {
                    Value::Dict(entry) => entry,
                    _ => bail!("file entry is not a dict"),
                };
                let size = match file.get(&b"length".to_vec()).ok_or("length not found in file dict")? {
                    Value::Int(s) => s,
                    _ => bail!("length is not an int"),
                };
                let name = match file.get(&b"path".to_vec()).ok_or("path not found in file dict")? {
                    Value::List(path) => {
                        let mut name = String::new();
                        for part in path {
                            if let Value::Bytes(p) = part {
                                write!(&mut name, "/{}", String::from_utf8(p.to_vec())?)?;
                            }
                        }
                        name
                    }
                    _ => bail!("path is not a list"),
                };

                files.push(((&name[1..]).to_owned(), *size));
            }
            Ok(files)
        }
        // single file mode
        None => {
            let size = match info.get(&b"length".to_vec()).ok_or("length not found in file dict")? {
                Value::Int(s) => s,
                _ => bail!("length is not an int"),
            };
            let name = match info.get(&b"name".to_vec()).ok_or("name not found in file dict")? {
                Value::Bytes(n) => String::from_utf8(n.to_vec())?,
                _ => bail!("name is not a str"),
            };

            Ok(vec![(name, *size)])
        }
        _ => bail!("files is not a list")
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