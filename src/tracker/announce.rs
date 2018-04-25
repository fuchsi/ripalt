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

use super::*;

use actix_web::HttpMessage;

use chrono::prelude::*;
use ipnetwork::IpNetwork;
use url::percent_encoding::percent_decode;
use uuid::Uuid;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::net::{IpAddr};
use std::str::FromStr;

use models::{self, Torrent, TorrentList, User, torrent::Transfer};

#[derive(Debug, Copy, Clone)]
pub enum Event {
    None,
    Started,
    Stopped,
    Completed,
}

impl FromStr for Event {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "started" => Ok(Event::Started),
            "stopped" => Ok(Event::Stopped),
            "completed" => Ok(Event::Completed),
            _ => Err("invalid event".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnnounceRequest {
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    compact: bool,
    no_peer_id: bool,
    event: Event,
    numwant: u16,
    key: Option<Vec<u8>>,
    tracker_id: Option<Vec<u8>>,
    ip_address: IpAddr,
    passcode: Vec<u8>,
    user_agent: String,

    // Message Stream Encryption Extension
    support_crypto: bool,
    require_crypto: bool,
    crypto_port: Option<u16>,
}

impl<S> TryFrom<HttpRequest<S>> for AnnounceRequest {
    type Error = Error;

    fn try_from(req: HttpRequest<S>) -> Result<Self> {
        trace!("Request: {:#?}", req);
        trace!("uri: {:#?}", req.uri());
        let query_str = req.uri().query().ok_or_else(|| "no query")?;
        trace!("query: {:#?}", query_str);
        let iter = query_str.split('&');
        let mut query_parts: HashMap<&str, &str> = HashMap::new();
        for part in iter {
            if let Some(pos) = part.find('=') {
                let key = &part[0..pos];
                let value = &part[pos + 1..];
                query_parts.insert(key, value);
            }
        }
        trace!("query_parts: {:#?}", query_parts);

        let default_numwant = SETTINGS
            .read()
            .map_err(|e| format!("{}", e))?
            .tracker
            .default_numwant;

        let q = req.query();
        trace!("info hash from query: {:#?}", q.get("info_hash"));
        let info_hash = query_parts
            .get("info_hash")
            .ok_or_else(|| "info_hash not in query")?
            .as_bytes();
        trace!("info_hash len={}: {:?}", info_hash.len(), info_hash);
        let info_hash = percent_decode(info_hash)
            .if_any()
            .ok_or_else(|| "malformed info hash")?;
        trace!("info_hash len={}: {:?}", info_hash.len(), info_hash);

        let peer_id = query_parts
            .get("peer_id")
            .ok_or_else(|| "peer_id not in query")?
            .as_bytes();
        let peer_id = percent_decode(peer_id).if_any().unwrap_or_else(|| peer_id.to_vec());

        let port = q.get("port")
            .ok_or_else(|| "port not in query")?
            .parse::<u16>()?;
        let uploaded = q.get("uploaded")
            .ok_or_else(|| "uploaded not in query")?
            .parse::<u64>()?;
        let downloaded = q.get("downloaded")
            .ok_or_else(|| "downloaded not in query")?
            .parse::<u64>()?;
        let left = q.get("left")
            .ok_or_else(|| "left not in query")?
            .parse::<u64>()?;
        let compact = q.get("compact").map(|v| v == "1").unwrap_or_else(|| false);
        let no_peer_id = q.get("no_peer_id")
            .map(|v| v == "1")
            .unwrap_or_else(|| false);
        let event = q.get("event")
            .map(|v| v.parse::<Event>())
            .unwrap_or_else(|| Ok(Event::None))?;
        let numwant = q.get("numwant")
            .map(|v| v.parse::<u16>())
            .unwrap_or_else(|| Ok(default_numwant))?;
        let key = query_parts.get("peer_id").map(|key| {
            percent_decode(key.as_bytes())
                .if_any()
                .unwrap_or_else(|| key.as_bytes().to_vec())
        });
        let tracker_id = query_parts.get("trackerid").map(|tracker_id| {
            percent_decode(tracker_id.as_bytes())
                .if_any()
                .unwrap_or_else(|| tracker_id.as_bytes().to_vec())
        });
        let ip_address = req.peer_addr().ok_or_else(|| "could not get ip addr")?.ip();
        let passcode = req.match_info()
            .get("passcode")
            .ok_or_else(|| "passcode not in query")?;
        let passcode = util::from_hex(passcode)?;
        let user_agent = req.headers()
            .get("user-agent")
            .ok_or_else(|| "user agent header not found")?
            .to_str()?
            .to_owned();

        let support_crypto = q.get("supportcrypto")
            .map(|v| v == "1")
            .unwrap_or_else(|| false);
        let require_crypto = q.get("requirecrypto")
            .map(|v| v == "1")
            .unwrap_or_else(|| false);
        let crypto_port = q.get("cryptoport").map(|p| p.parse::<u16>().unwrap_or(0));

        Ok(AnnounceRequest {
            info_hash,
            peer_id,
            port,
            uploaded,
            downloaded,
            left,
            compact,
            no_peer_id,
            event,
            numwant,
            key,
            tracker_id,
            ip_address,
            passcode,
            user_agent,
            support_crypto,
            require_crypto,
            crypto_port,
        })
    }
}

impl Message for AnnounceRequest {
    type Result = Result<AnnounceResponse>;
}

impl Handler<AnnounceRequest> for DbExecutor {
    type Result = Result<AnnounceResponse>;

    fn handle(
        &mut self,
        msg: AnnounceRequest,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<AnnounceRequest>>::Result {
        let add_download: i64;
        let add_upload: i64;
        let mut add_time_seeded: i32 = 0;
        let mut new_peer = false;

        let conn = self.conn();
        let mut user =
            User::find_by_passcode(&msg.passcode, &conn).ok_or_else(|| "invalid passcode")?;
        let mut torrent =
            Torrent::find_by_info_hash(&msg.info_hash, &conn).ok_or_else(|| "invalid info hash")?;
        let peer = match models::Peer::find_for_announce(&torrent.id, &user.id, &msg.peer_id, &conn)
            {
                Some(mut peer) => {
                    add_download = msg.downloaded as i64 - peer.bytes_downloaded;
                    add_upload = msg.uploaded as i64 - peer.bytes_uploaded;
                    if peer.seeder {
                        let duration = Utc::now().signed_duration_since(peer.updated_at);
                        add_time_seeded = duration.num_seconds() as i32;
                    }

                    peer.bytes_downloaded = msg.downloaded as i64;
                    peer.bytes_uploaded = msg.uploaded as i64;
                    peer.bytes_left = msg.left as i64;
                    if let Event::Completed = msg.event {
                        peer.seeder = true;
                    }
                    peer.crypto_enabled = msg.support_crypto || msg.require_crypto;
                    peer.updated_at = Utc::now();

                    peer
                }
                None => {
                    add_download = 0;
                    add_upload = 0;
                    new_peer = true;
                    trace!("NEW PEER!!!");

                    models::Peer {
                        id: Uuid::new_v4(),
                        torrent_id: torrent.id,
                        user_id: user.id,
                        ip_address: IpNetwork::from(msg.ip_address),
                        port: i32::from(msg.port),
                        bytes_uploaded: 0,
                        bytes_downloaded: 0,
                        bytes_left: msg.left as i64,
                        seeder: msg.left == 0,
                        peer_id: msg.peer_id.to_vec(),
                        user_agent: msg.user_agent.to_owned(),
                        crypto_enabled: msg.support_crypto || msg.require_crypto,
                        crypto_port: msg.crypto_port.map(i32::from),
                        offset_downloaded: 0,
                        offset_uploaded: 0,
                        created_at: Utc::now(),
                        finished_at: if msg.left == 0 {
                            Some(Utc::now())
                        } else {
                            None
                        },
                        updated_at: Utc::now(),
                    }
                }
            };
        let mut transfer = match Transfer::find_for_announce(&torrent.id, &user.id, &conn) {
            Some(mut transfer) => {
                transfer.bytes_uploaded += add_upload;
                transfer.bytes_downloaded += add_download;
                transfer.time_seeded += add_time_seeded;
                transfer.updated_at = Utc::now();
                transfer
            },
            None => Transfer::from(&peer),
        };

        trace!("add download: {}", add_download);
        trace!("add upload: {}", add_upload);
        trace!("add seed time: {}", add_time_seeded);

        user.downloaded += add_download;
        user.uploaded += add_upload;
        torrent.last_action = Some(Utc::now());
        torrent.visible = true;

        match msg.event {
            Event::Completed => {
                torrent.completed += 1;
                torrent.last_seeder = Some(Utc::now());
                transfer.completed_at = Some(Utc::now());
                peer.save(&conn)?;
            }
            Event::Stopped => {
                if !new_peer {
                    peer.delete(&conn)?;
                }
                if peer.seeder {
                    torrent.last_seeder = Some(Utc::now());
                    torrent.save(&conn)?;
                }
            }
            _ => {
                if peer.seeder {
                    torrent.last_seeder = Some(Utc::now());
                }
                peer.save(&conn)?;
            }
        }

        torrent.save(&conn)?;
        user.save(&conn)?;
        transfer.save(&conn)?;

        let want = !peer.seeder;
        let mut peers = models::Peer::peers_for_torrent(&torrent.id, want, i64::from(msg.numwant), &conn);
        let rest = msg.numwant - peers.len() as u16;
        if rest > 0 {
            let mut peers2 = models::Peer::peers_for_torrent(&torrent.id, !want, i64::from(rest), &conn);
            peers.append(&mut peers2);
        }
        // if the client does not support crypto, set the crypto flag for all peers to false,
        // to avoid returning the crypto_port in the serialize step.
        if !(msg.support_crypto || msg.require_crypto) {
            peers = peers.into_iter().map(|mut p| {p.crypto_enabled = false; p}).collect();
        }

        let (complete, incomplete) = TorrentList::peer_count(&torrent.id, &conn);

        Ok(AnnounceResponse {
            peers: Some(peers),
            complete: complete as u32,
            incomplete: incomplete as u32,
            crypto_flags: msg.support_crypto || msg.require_crypto,
            compact: msg.compact,
            no_peer_id: msg.no_peer_id,
            tracker_id: msg.tracker_id.map(|v| v.to_vec()),
        })
    }
}

#[derive(Debug)]
pub struct AnnounceResponse {
    peers: Option<Vec<models::Peer>>,
    tracker_id: Option<Vec<u8>>,
    complete: u32,
    incomplete: u32,

    // internal flags
    compact: bool,
    no_peer_id: bool,
    crypto_flags: bool,
}

impl AnnounceResponse {
    pub fn peers(&mut self) -> Vec<models::Peer> {
        self.peers.take().unwrap_or_else(Vec::new)
    }

    pub fn tracker_id(&mut self) -> Option<Vec<u8>> {
        self.tracker_id.take()
    }

    pub fn complete(&self) -> u32 {
        self.complete
    }

    pub fn incomplete(&self) -> u32 {
        self.incomplete
    }

    pub fn compact(&self) -> bool {
        self.compact
    }

    pub fn no_peer_id(&self) -> bool {
        self.no_peer_id
    }

    pub fn crypto_flags(&self) -> bool {
        self.crypto_flags
    }
}