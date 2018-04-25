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

//! Peer Model

use super::schema::*;
use super::*;
use super::{torrent::Torrent, user::User};
use ipnetwork::IpNetwork;

#[derive(Debug, Queryable, Insertable, AsChangeset, Identifiable, Associations)]
#[table_name = "peers"]
#[primary_key(id)]
#[belongs_to(Torrent)]
#[belongs_to(User)]
pub struct Peer {
    pub id: Uuid,
    pub torrent_id: Uuid,
    pub user_id: Uuid,
    pub ip_address: IpNetwork,
    pub port: i32,
    pub bytes_uploaded: i64,
    pub bytes_downloaded: i64,
    pub bytes_left: i64,
    pub seeder: bool,
    pub peer_id: Bytes,
    pub user_agent: String,
    pub crypto_enabled: bool,
    pub crypto_port: Option<i32>,
    pub offset_uploaded: i64,
    pub offset_downloaded: i64,
    pub created_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub updated_at: Timestamp,
}

impl Peer {
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        use schema::peers::dsl;
        dsl::peers.find(id).first::<Self>(db).ok()
    }

    /// Returns all peers for a torrent
    ///
    /// Return value is a Vector of a Tuple (Peer, UserName)
    pub fn find_for_torrent(torrent_id: &Uuid, db: &PgConnection) -> Vec<(Self, String)> {
        use schema::peers::dsl;
        dsl::peers
            .filter(dsl::torrent_id.eq(torrent_id))
            .order(dsl::updated_at.desc())
            .load::<Self>(db)
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .map(|peer: Peer| {
                let user_name = peer.user_name(db);
                (peer, user_name)
            })
            .collect()
    }

    pub fn seeder_for_torrent(torrent_id: &Uuid, limit: i64, db: &PgConnection) -> Vec<Self> {
        Self::peers_for_torrent(torrent_id, true, limit, db)
    }

    pub fn leecher_for_torrent(torrent_id: &Uuid, limit: i64, db: &PgConnection) -> Vec<Self> {
        Self::peers_for_torrent(torrent_id, false, limit, db)
    }

    pub fn peers_for_torrent(torrent_id: &Uuid, seeder: bool, limit: i64, db: &PgConnection) -> Vec<Self> {
        use schema::peers::dsl;
        dsl::peers
            .filter(dsl::torrent_id.eq(torrent_id))
            .filter(dsl::seeder.eq(seeder))
            .order(dsl::updated_at.desc())
            .limit(limit)
            .load::<Self>(db)
            .unwrap_or_else(|_| Vec::new())
    }

    pub fn find_for_announce(torrent_id: &Uuid, user_id: &Uuid, peer_id: &[u8], db: &PgConnection) -> Option<Self> {
        use schema::peers::dsl;
        dsl::peers
            .filter(dsl::torrent_id.eq(torrent_id))
            .filter(dsl::user_id.eq(user_id))
            .filter(dsl::peer_id.eq(peer_id))
            .first::<Peer>(db)
            .ok()
    }

    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        let query = diesel::insert_into(peers::table)
            .values(self)
            .on_conflict(on_constraint("peers_tup_key"))
            .do_update()
            .set(self);
        trace!("query: {}", diesel::debug_query::<diesel::pg::Pg, _>(&query));
        query.execute(db)
            .chain_err(|| "peer update failed")
    }

    pub fn delete(&self, db: &PgConnection) -> Result<usize> {
        diesel::delete(self)
            .execute(db)
            .chain_err(|| "peer delete failed")
    }
}

impl HasUser for Peer {
    fn user_id(&self) -> &Uuid {
        &self.user_id
    }
}


