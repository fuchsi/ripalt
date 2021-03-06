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
use models::acl::Subject;

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
        let torrent = NewTorrent::new(name, info_hash, category_id, user_id, description, size);

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
        diesel::update(torrents::table)
            .set(self)
            .filter(dsl::id.eq(&self.id))
            .execute(db)
            .chain_err(|| "torrent update failed")
    }

    pub fn delete(&self, db: &PgConnection) -> Result<usize> {
        use schema::torrents::dsl as t;
        diesel::delete(schema::torrents::table)
            .filter(t::id.eq(&self.id))
            .execute(db)
            .map_err(|e| format!("failed to delete torrent: {}", e).into())
    }

    pub fn comments(&self, db: &PgConnection) -> Vec<TorrentComment> {
        TorrentComment::find_for_torrent(&self.id, db)
    }
}

impl MaybeHasUser for Torrent {
    fn user_id(&self) -> &Option<Uuid> {
        &self.user_id
    }
}

#[derive(Debug, Insertable, Identifiable)]
#[table_name = "torrents"]
pub struct NewTorrent<'a> {
    id: Uuid,
    name: &'a str,
    info_hash: &'a [u8],
    category_id: &'a Uuid,
    user_id: &'a Uuid,
    description: &'a str,
    size: i64,
}

impl<'a> NewTorrent<'a> {
    pub fn new(
        name: &'a str,
        info_hash: &'a [u8],
        category_id: &'a Uuid,
        user_id: &'a Uuid,
        description: &'a str,
        size: i64,
    ) -> Self {
        let id = Uuid::new_v4();

        NewTorrent {
            id,
            name,
            info_hash,
            category_id,
            user_id,
            description,
            size,
        }
    }
    /// Insert the torrent into the database
    pub fn insert(&self, db: &PgConnection) -> Result<Torrent> {
        let res = self.insert_into(torrents::table).get_result(db);

        match res {
            Ok(torrent) => Ok(torrent),
            Err(e) => bail!("failed to create torrent: {}", e),
        }
    }
}

#[derive(AsChangeset)]
#[table_name = "torrents"]
pub struct UpdateTorrent<'a> {
    name: &'a str,
    category_id: &'a Uuid,
    description: &'a str,
}

impl<'a> UpdateTorrent<'a> {
    pub fn new(name: &'a str, category_id: &'a Uuid, description: &'a str) -> Self {
        UpdateTorrent {
            name,
            category_id,
            description,
        }
    }

    pub fn update(&self, id: &Uuid, db: &PgConnection) -> Result<usize> {
        use schema::torrents::dsl as t;
        diesel::update(schema::torrents::table)
            .set(self)
            .filter(t::id.eq(id))
            .execute(db)
            .map_err(|e| format!("failed to update torrent: {}", e).into())
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

#[derive(Debug, Insertable, Identifiable)]
#[table_name = "torrent_meta_files"]
pub struct NewTorrentMetaFile<'a> {
    id: &'a Uuid,
    data: &'a [u8],
}

impl<'a> NewTorrentMetaFile<'a> {
    pub fn new(id: &'a Uuid, data: &'a [u8]) -> Self {
        NewTorrentMetaFile { id, data }
    }

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
            .unwrap_or_default()
    }
}

#[derive(Debug, Insertable, Identifiable)]
#[table_name = "torrent_files"]
pub struct NewTorrentFile<'a> {
    id: Uuid,
    torrent_id: &'a Uuid,
    file_name: &'a str,
    size: &'a i64,
}

impl<'a> NewTorrentFile<'a> {
    pub fn new(torrent_id: &'a Uuid, file_name: &'a str, size: &'a i64) -> Self {
        let id = Uuid::new_v4();
        NewTorrentFile {
            id,
            torrent_id,
            file_name,
            size,
        }
    }

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

#[derive(Debug, Insertable, Identifiable)]
#[table_name = "torrent_nfos"]
pub struct NewTorrentNFO<'a> {
    id: Uuid,
    torrent_id: &'a Uuid,
    data: &'a [u8],
}

impl<'a> NewTorrentNFO<'a> {
    pub fn new(torrent_id: &'a Uuid, data: &'a [u8]) -> Self {
        let id = Uuid::new_v4();
        NewTorrentNFO { id, torrent_id, data }
    }

    pub fn create(&self, db: &PgConnection) -> Result<usize> {
        self.insert_into(torrent_nfos::table)
            .execute(db)
            .chain_err(|| "failed to insert nfo")
    }
}

#[derive(Default)]
pub struct TorrentMsg {
    pub torrent: Torrent,
    pub torrent_user_name: Option<String>,
    pub category: Category,
    pub nfo: Option<TorrentNFO>,
    pub images: Vec<TorrentImage>,
    pub files: Vec<TorrentFile>,
    pub peers: Vec<(Peer, String)>,
    pub comments: Vec<TorrentCommentResponse>,
    pub timezone: i32,
}

impl TorrentMsg {
    pub fn find(id: &Uuid, db: &PgConnection, subj: &UserSubject) -> Result<Self> {
        if let Some(torrent) = Torrent::find(id, db) {
            let nfo = TorrentNFO::find_for_torrent(id, db);
            let images = TorrentImage::find_for_torrent(id, db);
            let files = TorrentFile::find_for_torrent(id, db);
            let peers = Peer::find_for_torrent(id, db);
            let torrent_user_name = torrent.user_name(db);
            let category = Category::find(&torrent.category_id, db).ok_or("category not found")?;
            let comments = torrent.comments(db).into_iter().map(|c| TorrentCommentResponse::new(c, &db, &subj)).collect();
            let timezone = SETTINGS.read().unwrap().user.default_timezone;

            Ok(TorrentMsg {
                torrent,
                torrent_user_name,
                category,
                nfo,
                images,
                files,
                peers,
                comments,
                timezone,
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
    pub comments: i64,
}

impl TorrentList {
    pub fn peer_count(torrent_id: &Uuid, db: &PgConnection) -> (i64, i64) {
        use schema::torrent_list::dsl;

        schema::torrent_list::table
            .select((dsl::seeder, dsl::leecher))
            .filter(dsl::id.eq(torrent_id))
            .first::<(i64, i64)>(db)
            .unwrap_or_default()
    }

    pub fn peer_count_scrape(info_hash: &[u8], db: &PgConnection) -> (i64, i64, i32) {
        use schema::torrent_list::dsl;

        schema::torrent_list::table
            .select((dsl::seeder, dsl::leecher, dsl::completed))
            .filter(dsl::info_hash.eq(info_hash))
            .first::<(i64, i64, i32)>(db)
            .unwrap_or_default()
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
    pub fn find_for_announce(torrent_id: &Uuid, user_id: &Uuid, db: &PgConnection) -> Option<Transfer> {
        transfers::table
            .filter(transfers::dsl::torrent_id.eq(torrent_id))
            .filter(transfers::dsl::user_id.eq(user_id))
            .first::<Transfer>(db)
            .ok()
    }
    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        diesel::insert_into(transfers::table)
            .values(self)
            .on_conflict(on_constraint("transfers_user_id_torrent_id_key"))
            .do_update()
            .set(self)
            .execute(db)
            .chain_err(|| "transfer update failed")
    }
}

impl<'a> From<&'a Peer> for Transfer {
    fn from(peer: &Peer) -> Self {
        let completed_at = if peer.seeder { Some(Utc::now()) } else { None };
        Transfer {
            id: Uuid::new_v4(),
            user_id: peer.user_id,
            torrent_id: peer.torrent_id,
            bytes_uploaded: peer.bytes_uploaded,
            bytes_downloaded: peer.bytes_downloaded,
            time_seeded: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at,
        }
    }
}

#[derive(Debug, Queryable, Associations, Insertable, Identifiable, Serialize)]
#[belongs_to(Torrent)]
pub struct TorrentImage {
    pub id: Uuid,
    pub torrent_id: Uuid,
    pub file_name: String,
    pub index: i16,
    pub created_at: Timestamp,
}

impl TorrentImage {
    pub fn find_for_torrent(torrent_id: &Uuid, db: &PgConnection) -> Vec<Self> {
        use schema::torrent_images::dsl;
        dsl::torrent_images
            .filter(dsl::torrent_id.eq(torrent_id))
            .order_by(dsl::index.asc())
            .load::<Self>(db)
            .unwrap_or_default()
    }
}

#[derive(Insertable, Identifiable)]
#[table_name = "torrent_images"]
pub struct NewTorrentImage<'a> {
    id: Uuid,
    torrent_id: &'a Uuid,
    file_name: &'a str,
    index: &'a i16,
}

impl<'a> NewTorrentImage<'a> {
    pub fn new(torrent_id: &'a Uuid, file_name: &'a str, index: &'a i16) -> Self {
        let id = Uuid::new_v4();
        NewTorrentImage {
            id,
            torrent_id,
            file_name,
            index,
        }
    }

    pub fn create(&self, db: &PgConnection) -> Result<usize> {
        diesel::insert_into(schema::torrent_images::table)
            .values(self)
            .execute(db)
            .map_err(|e| format!("torrent images insert failed: {}", e).into())
    }
}

#[derive(Queryable, Identifiable, Associations, Insertable, Serialize)]
#[belongs_to(User)]
#[belongs_to(Torrent)]
pub struct TorrentComment {
    id: Uuid,
    user_id: Uuid,
    torrent_id: Uuid,
    content: String,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl TorrentComment {
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        use schema::torrent_comments::dsl;
        dsl::torrent_comments.find(id).first::<Self>(db).ok()
    }

    pub fn find_for_torrent(torrent_id: &Uuid, db: &PgConnection) -> Vec<Self> {
        use schema::torrent_comments::dsl;
        dsl::torrent_comments
            .filter(dsl::torrent_id.eq(torrent_id))
            .order_by(dsl::created_at.asc())
            .load::<Self>(db)
            .unwrap_or_default()
    }

    pub fn set_content(&mut self, content: String) -> String {
        std::mem::replace(&mut self.content, content)
    }

    pub fn save(&mut self, db: &PgConnection) -> Result<usize> {
        use schema::torrent_comments::dsl as t;
        self.updated_at = Utc::now();
        diesel::update(schema::torrent_comments::table)
            .set(self.update_set())
            .filter(t::id.eq(&self.id))
            .execute(db)
            .map_err(|e| format!("failed to update torrent comment: {}", e).into())
    }

    fn update_set<'a>(&'a self) -> UpdateTorrentComment<'a> {
        UpdateTorrentComment {
            content: &self.content[..],
            updated_at: &self.updated_at,
        }
    }

    pub fn delete(&self, db: &PgConnection) -> Result<usize> {
        use schema::torrent_comments::dsl as t;
        diesel::delete(schema::torrent_comments::table)
            .filter(t::id.eq(&self.id))
            .execute(db)
            .map_err(|e| format!("failed to delete comment: {}", e).into())
    }
}

impl HasUser for TorrentComment {
    fn user_id(&self) -> &Uuid {
        &self.user_id
    }
}

#[derive(Serialize)]
pub struct TorrentCommentResponse {
    #[serde(flatten)]
    comment: TorrentComment,
    user_name: String,
    may_edit: bool,
    may_delete: bool,
}

impl TorrentCommentResponse {
    pub fn new(comment: TorrentComment, db: &PgConnection, subj: &UserSubject) -> Self {
        let user_name = comment.user_name(db);
        let may_edit = subj.may_write(&comment);
        let may_delete = subj.may_delete(&comment);
        Self { comment, user_name, may_edit, may_delete }
    }

    pub fn new2(comment: TorrentComment, db: &PgConnection) -> Self {
        let user_name = comment.user_name(db);
        let may_edit = false;
        let may_delete = false;
        Self { comment, user_name, may_edit, may_delete }
    }

    pub fn comment(&self) -> &TorrentComment {
        &self.comment
    }
}

#[derive(Insertable, Identifiable)]
#[table_name = "torrent_comments"]
pub struct NewTorrentComment<'a> {
    id: Uuid,
    user_id: &'a Uuid,
    torrent_id: &'a Uuid,
    content: &'a str,
}

impl<'a> NewTorrentComment<'a> {
    pub fn new(user_id: &'a Uuid, torrent_id: &'a Uuid, content: &'a str) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            user_id,
            torrent_id,
            content,
        }
    }

    pub fn create(&self, db: &PgConnection) -> Result<TorrentComment> {
        diesel::insert_into(schema::torrent_comments::table)
            .values(self)
            .get_result::<TorrentComment>(db)
            .map_err(|e| format!("torrent comment insert failed: {}", e).into())
    }
}

#[derive(AsChangeset)]
#[table_name = "torrent_comments"]
pub struct UpdateTorrentComment<'a> {
    content: &'a str,
    updated_at: &'a Timestamp,
}
