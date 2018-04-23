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

//! Torrent model

use super::schema::*;
use super::*;
use super::{category::Category, user::User};

#[derive(Debug, Queryable, Insertable, AsChangeset, Identifiable, Associations, Serialize)]
#[table_name = "torrents"]
#[primary_key(id)]
#[belongs_to(Category)]
#[belongs_to(User)]
pub struct Torrent {
    pub id: Uuid,
    pub name: String,
    pub info_hash: Bytes,
    pub category_id: Uuid,
    pub user_id: Option<Uuid>,
    pub description: String,
    pub size: i64,
    pub visible: bool,
    pub completed: i32,
    pub last_action: Option<Timestamp>,
    pub last_seeder: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Default for Torrent {
    fn default() -> Self {
        Torrent {
            id: Default::default(),
            name: Default::default(),
            info_hash: Default::default(),
            category_id: Default::default(),
            user_id: Default::default(),
            description: Default::default(),
            size: Default::default(),
            visible: Default::default(),
            completed: Default::default(),
            last_action: Default::default(),
            last_seeder: Default::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Torrent {
    pub fn create(
        name: &str,
        description: &str,
        info_hash: &[u8],
        category_id: &Uuid,
        user_id: &Uuid,
        size: i64,
        db: &PgConnection,
    ) -> Result<Self> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let torrent = NewTorrent {
            id: &id,
            name,
            description,
            info_hash,
            category_id,
            user_id,
            size,
            visible: false,
            completed: 0,
            last_action: None,
            last_seeder: None,
            created_at: &now,
            updated_at: &now,
        };

        torrent.insert(db)
    }

    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        use schema::torrents::dsl;
        dsl::torrents.find(id).first::<Self>(db).ok()
    }

    pub fn find_by_info_hash(info_hash: &[u8], db: &PgConnection) -> Option<Self> {
        use schema::torrents::dsl;
        dsl::torrents
            .filter(dsl::info_hash.eq(info_hash))
            .first::<Self>(db)
            .ok()
    }

    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        use schema::torrents::dsl;
        let query = diesel::update(torrents::table)
            .set(self)
            .filter(dsl::id.eq(&self.id));
        trace!(
            "query: {}",
            diesel::debug_query::<diesel::pg::Pg, _>(&query)
        );
        query.execute(db).chain_err(|| "torrent update failed")
    }
}

impl MaybeHasUser for Torrent {
    fn user_id(&self) -> &Option<Uuid> {
        &self.user_id
    }
}

#[derive(Debug, Insertable)]
#[table_name = "torrents"]
pub struct NewTorrent<'a> {
    pub id: &'a Uuid,
    pub name: &'a str,
    pub info_hash: &'a [u8],
    pub category_id: &'a Uuid,
    pub user_id: &'a Uuid,
    pub description: &'a str,
    pub size: i64,
    pub visible: bool,
    pub completed: i32,
    pub last_action: Option<&'a Timestamp>,
    pub last_seeder: Option<&'a Timestamp>,
    pub created_at: &'a Timestamp,
    pub updated_at: &'a Timestamp,
}

impl<'a> NewTorrent<'a> {
    /// Insert the torrent into the database
    fn insert(&self, db: &PgConnection) -> Result<Torrent> {
        let res = self.insert_into(torrents::table).get_result(db);

        match res {
            Ok(torrent) => Ok(torrent),
            Err(e) => bail!("failed to create torrent: {}", e),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset, Identifiable)]
#[table_name = "torrent_meta_files"]
pub struct TorrentMetaFile {
    pub id: Uuid,
    pub data: Bytes,
}

impl TorrentMetaFile {
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        use schema::torrent_meta_files::dsl;
        dsl::torrent_meta_files.find(id).first::<Self>(db).ok()
    }
}

#[derive(Debug, Insertable)]
#[table_name = "torrent_meta_files"]
pub struct NewTorrentMetaFile<'a> {
    pub id: &'a Uuid,
    pub data: &'a [u8],
}

impl<'a> NewTorrentMetaFile<'a> {
    pub fn create(&self, db: &PgConnection) -> Result<usize> {
        self.insert_into(torrent_meta_files::table)
            .execute(db)
            .chain_err(|| "failed to insert meta file")
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset, Identifiable, Associations, Serialize)]
#[table_name = "torrent_files"]
#[belongs_to(Torrent)]
pub struct TorrentFile {
    pub id: Uuid,
    pub torrent_id: Uuid,
    pub file_name: String,
    pub size: i64,
}

impl TorrentFile {
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        use schema::torrent_files::dsl;
        dsl::torrent_files.find(id).first::<Self>(db).ok()
    }

    pub fn find_for_torrent(torrent_id: &Uuid, db: &PgConnection) -> Vec<Self> {
        use schema::torrent_files::dsl;
        dsl::torrent_files
            .filter(dsl::torrent_id.eq(torrent_id))
            .order(dsl::file_name.asc())
            .load::<Self>(db)
            .unwrap_or_else(|_| vec![])
    }
}

#[derive(Debug, Insertable)]
#[table_name = "torrent_files"]
pub struct NewTorrentFile<'a> {
    pub id: &'a Uuid,
    pub torrent_id: &'a Uuid,
    pub file_name: &'a str,
    pub size: i64,
}

impl<'a> NewTorrentFile<'a> {
    pub fn create(&self, db: &PgConnection) -> Result<usize> {
        self.insert_into(torrent_files::table)
            .execute(db)
            .chain_err(|| "failed to insert file")
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset, Identifiable, Associations, Serialize)]
#[table_name = "torrent_nfos"]
#[belongs_to(Torrent)]
pub struct TorrentNFO {
    pub id: Uuid,
    pub torrent_id: Uuid,
    pub data: Bytes,
}

impl TorrentNFO {
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        use schema::torrent_nfos::dsl;
        dsl::torrent_nfos.find(id).first::<Self>(db).ok()
    }

    pub fn find_for_torrent(torrent_id: &Uuid, db: &PgConnection) -> Option<Self> {
        use schema::torrent_nfos::dsl;
        dsl::torrent_nfos
            .filter(dsl::torrent_id.eq(torrent_id))
            .first::<Self>(db)
            .ok()
    }
}

#[derive(Debug, Insertable)]
#[table_name = "torrent_nfos"]
pub struct NewTorrentNFO<'a> {
    pub id: &'a Uuid,
    pub torrent_id: &'a Uuid,
    pub data: &'a [u8],
}

impl<'a> NewTorrentNFO<'a> {
    pub fn create(&self, db: &PgConnection) -> Result<usize> {
        self.insert_into(torrent_nfos::table)
            .execute(db)
            .chain_err(|| "failed to insert nfo")
    }
}

#[derive(Debug, Default)]
pub struct TorrentContainer {
    pub torrent: Torrent,
    pub torrent_user_name: Option<String>,
    pub category: Category,
    pub nfo: Option<TorrentNFO>,
    pub files: Vec<TorrentFile>,
    pub peers: Vec<(Peer, String)>,
}

impl TorrentContainer {
    pub fn find(id: &Uuid, db: &PgConnection) -> Result<Self> {
        if let Some(torrent) = Torrent::find(id, db) {
            let nfo = TorrentNFO::find_for_torrent(id, db);
            let files = TorrentFile::find_for_torrent(id, db);
            let peers = Peer::find_for_torrent(id, db);
            let torrent_user_name = torrent.user_name(db);
            let category = Category::find(&torrent.category_id, db).ok_or("category not found")?;

            Ok(TorrentContainer {
                torrent,
                torrent_user_name,
                category,
                nfo,
                files,
                peers,
            })
        } else {
            bail!("torrent not found: {}", id)
        }
    }
}

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[table_name = "torrent_list"]
pub struct TorrentList {
    pub id: Uuid,
    pub info_hash: Bytes,
    pub name: String,
    pub category_id: Uuid,
    pub category_name: String,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub size: i64,
    pub files: i64,
    pub visible: bool,
    pub completed: i32,
    pub seeder: i64,
    pub leecher: i64,
    pub last_action: Option<Timestamp>,
    pub last_seeder: Option<Timestamp>,
    pub created_at: Timestamp,
}

impl TorrentList {
    pub fn peer_count(torrent_id: &Uuid, db: &PgConnection) -> (i64, i64) {
        use schema::torrent_list::dsl;

        schema::torrent_list::table
            .select((dsl::seeder, dsl::leecher))
            .filter(dsl::id.eq(torrent_id))
            .first::<(i64, i64)>(db)
            .unwrap_or_else(|_| (0, 0))
    }

    pub fn peer_count_scrape(info_hash: &[u8], db: &PgConnection) -> (i64, i64, i32) {
        use schema::torrent_list::dsl;

        schema::torrent_list::table
            .select((dsl::seeder, dsl::leecher, dsl::completed))
            .filter(dsl::info_hash.eq(info_hash))
            .first::<(i64, i64, i32)>(db)
            .unwrap_or_else(|_| (0, 0, 0))
    }
}

#[derive(Debug, Queryable, Identifiable, Serialize, Insertable, AsChangeset, Associations)]
#[table_name = "transfers"]
#[belongs_to(Torrent)]
#[belongs_to(User)]
pub struct Transfer {
    pub id: Uuid,
    pub user_id: Uuid,
    pub torrent_id: Uuid,
    pub bytes_uploaded: i64,
    pub bytes_downloaded: i64,
    pub time_seeded: i32,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub completed_at: Option<Timestamp>,
}

impl Transfer {
    pub fn find_for_announce(
        torrent_id: &Uuid,
        user_id: &Uuid,
        db: &PgConnection,
    ) -> Option<Transfer> {
        transfers::table
            .filter(transfers::dsl::torrent_id.eq(torrent_id))
            .filter(transfers::dsl::user_id.eq(user_id))
            .first::<Transfer>(db)
            .ok()
    }
    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        let query = diesel::insert_into(transfers::table)
            .values(self)
            .on_conflict(on_constraint("transfers_user_id_torrent_id_key"))
            .do_update()
            .set(self);
        trace!(
            "query: {}",
            diesel::debug_query::<diesel::pg::Pg, _>(&query)
        );
        query.execute(db).chain_err(|| "transfer update failed")
    }
}

impl<'a> From<&'a Peer> for Transfer {
    fn from(peer: &Peer) -> Self {
        let completed_at = match peer.seeder {
            true => Some(Utc::now()),
            false => None,
        };
        Transfer {
            id: Uuid::new_v4(),
            user_id: peer.user_id.clone(),
            torrent_id: peer.torrent_id.clone(),
            bytes_uploaded: peer.bytes_uploaded,
            bytes_downloaded: peer.bytes_downloaded,
            time_seeded: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at,
        }
    }
}
