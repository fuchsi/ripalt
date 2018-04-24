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

//! Data models

use super::*;

use diesel;
use diesel::pg::upsert::on_constraint;
use serde_json;

use SETTINGS;

use schema;
use error::*;

/// Convenient wrapper around [DateTime](/chrono/struct.DateTime.html)<[Utc](/chrono/struct.Utc.html)>
pub type Timestamp = DateTime<Utc>;
/// Convenient wrapper around `Vec<u8>`
pub type Bytes = Vec<u8>;

pub use self::group::Group;
pub use self::user::{User, Property};
pub use self::torrent::{Torrent, TorrentMsg, TorrentMetaFile, TorrentList};
pub use self::category::Category;
pub use self::peer::Peer;
pub use self::user::{HasUser, MaybeHasUser};

pub mod user;
pub mod group;
pub mod torrent;
pub mod peer;
pub mod category;
pub mod acl;
