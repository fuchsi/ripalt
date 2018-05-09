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

use actix_web::AsyncResponder;
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_bencode;

use std::convert::TryFrom;
use std::net::{IpAddr, SocketAddr};

use self::announce::{AnnounceRequest, AnnounceResponse};
use self::scrape::{ScrapeFile, ScrapeRequest, ScrapeResponse};
use models;

mod announce;
mod scrape;

pub fn build(db: Addr<Syn, DbExecutor>) -> App<State> {
    let acl = Arc::new(RwLock::new(Acl::new()));
    let state = State::new(db, acl);

    App::with_state(state)
        .middleware(Logger::default())
        .middleware(DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
        .prefix("/tracker")
        .resource("/announce/{passcode}", |r| {
            r.method(Method::GET).f(tracker::announce)
        })
        .resource("/scrape", |r| r.method(Method::GET).f(tracker::scrape))
}

#[derive(Debug, Clone)]
pub struct Peer {
    peer_id: Option<Vec<u8>>,
    sock_addr: SocketAddr,
    crypto: bool,
    crypto_port: Option<u16>,
}

impl Serialize for Peer {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = s.serialize_map(Some(3))?;
        if let Some(ref peer_id) = self.peer_id {
            map.serialize_entry("peer id", &ByteString(peer_id))?;
        }
        map.serialize_entry("ip", &self.sock_addr.ip())?;
        let port = if self.crypto {
            if let Some(port) = self.crypto_port {
                port
            } else {
                self.sock_addr.port()
            }
        } else {
            self.sock_addr.port()
        };
        map.serialize_entry("port", &port)?;

        map.end()
    }
}

impl Peer {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::with_capacity(6);

        if let IpAddr::V4(v4addr) = self.sock_addr.ip() {
            let octets = v4addr.octets();
            bytes.push(octets[0]);
            bytes.push(octets[1]);
            bytes.push(octets[2]);
            bytes.push(octets[3]);
            let port = if self.crypto {
                if let Some(port) = self.crypto_port {
                    port
                } else {
                    self.sock_addr.port()
                }
            } else {
                self.sock_addr.port()
            };
            let (b0, b1) = ((port >> 8) as u8, port as u8);
            bytes.push(b0);
            bytes.push(b1)
        }

        bytes
    }
}

struct ByteString<'a>(&'a [u8]);

impl<'a> Serialize for ByteString<'a> {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_bytes(self.0)
    }
}

struct CompactPeerList<'a>(Vec<&'a Peer>);

impl<'a> CompactPeerList<'a> {
    pub fn new() -> Self {
        CompactPeerList(Vec::new())
    }

    pub fn push(&mut self, p: &'a Peer) {
        self.0.push(p);
    }
}

impl<'a> Serialize for CompactPeerList<'a> {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut bytes = Vec::with_capacity(self.0.len() * 6);

        for peer in &self.0 {
            let mut pbytes = peer.as_bytes();
            bytes.append(&mut pbytes);
        }

        s.serialize_bytes(&bytes)
    }
}

impl From<models::Peer> for Peer {
    fn from(p: models::Peer) -> Self {
        Self {
            peer_id: Some(p.peer_id),
            crypto: p.crypto_enabled,
            crypto_port: p.crypto_port.map(|port| port as u16),
            sock_addr: SocketAddr::new(p.ip_address.ip(), p.port as u16),
        }
    }
}

pub trait ResponseData {
    fn set_failure_reason(&mut self, reason: String);
}

#[derive(Debug, Default, Clone)]
pub struct AnnounceData {
    failure_reason: Option<String>,
    warning_message: Option<String>,
    interval: u16,
    min_interval: Option<u16>,
    tracker_id: Option<Vec<u8>>,
    complete: u32,
    incomplete: u32,
    peers: Vec<Peer>,

    // internal flags
    compact: bool,
    crypto_flags: bool,
}

impl ResponseData for AnnounceData {
    fn set_failure_reason(&mut self, reason: String) {
        self.failure_reason = Some(reason);
    }
}

impl Serialize for AnnounceData {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref failure_reason) = self.failure_reason {
            let mut root = s.serialize_map(Some(1))?;
            root.serialize_entry("failure reason", failure_reason)?;
            return root.end();
        }

        let mut root = s.serialize_map(None)?;

        if let Some(ref warning_message) = self.warning_message {
            root.serialize_entry("warning message", warning_message)?;
        }
        root.serialize_entry("interval", &self.interval)?;
        if let Some(ref min_interval) = self.min_interval {
            root.serialize_entry("min interval", min_interval)?;
        }
        if let Some(ref tracker_id) = self.tracker_id {
            root.serialize_entry("tracker id", &ByteString(tracker_id))?;
        }
        root.serialize_entry("complete", &self.complete)?;
        root.serialize_entry("incomplete", &self.incomplete)?;

        let mut crypto_flags: Vec<u8> = Vec::new();
        if self.compact {
            let mut peers = CompactPeerList::new();
            for peer in &self.peers {
                if self.crypto_flags && peer.sock_addr.is_ipv4() {
                    if peer.crypto {
                        crypto_flags.push(b'1');
                    } else {
                        crypto_flags.push(b'0');
                    }
                }
                peers.push(peer);
            }
            root.serialize_entry("peers", &peers)?;
        } else {
            if self.crypto_flags {
                for peer in &self.peers {
                    if peer.crypto {
                        crypto_flags.push(b'1');
                    } else {
                        crypto_flags.push(b'0');
                    }
                }
            }
            root.serialize_entry("peers", &self.peers)?;
        }

        if self.crypto_flags {
            root.serialize_entry("crypto_flags", &String::from_utf8(crypto_flags).unwrap())?;
        }

        root.end()
    }
}

#[derive(Debug)]
struct ScrapeFlags {
    min_interval: u16,
}

impl Serialize for ScrapeFlags {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut root = s.serialize_map(Some(1))?;
        root.serialize_entry("min interval", &self.min_interval)?;
        root.end()
    }
}

#[derive(Debug, Default)]
struct ScrapeFiles(Vec<ScrapeFile>);

impl Serialize for ScrapeFiles {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut root = s.serialize_map(Some(self.0.len()))?;
        for file in &self.0 {
            root.serialize_entry(&ByteString(&file.info_hash), &file)?;
        }
        root.end()
    }
}

#[derive(Debug, Default)]
pub struct ScrapeData {
    failure_reason: Option<String>,
    files: ScrapeFiles,
    min_interval: Option<u16>,
}

impl ResponseData for ScrapeData {
    fn set_failure_reason(&mut self, reason: String) {
        self.failure_reason = Some(reason);
    }
}

impl Serialize for ScrapeData {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref failure_reason) = self.failure_reason {
            let mut root = s.serialize_map(Some(1))?;
            root.serialize_entry("failure reason", failure_reason)?;
            return root.end();
        }

        let mut root = s.serialize_map(None)?;

        if let Some(min_interval) = self.min_interval {
            root.serialize_entry("flags", &ScrapeFlags { min_interval })?;
        }


        root.serialize_entry("files", &self.files)?;

        root.end()
    }
}

pub struct BencodeResponse<T>
where
    T: ResponseData + Serialize,
{
    data: T,
}

impl<T: Default + Serialize + ResponseData> BencodeResponse<T> {
    pub fn failure(reason: &str) -> BencodeResponse<T> {
        let mut data = T::default();
        data.set_failure_reason(reason.to_owned());
        BencodeResponse { data }
    }
}

impl<T: ResponseData + Serialize> Responder for BencodeResponse<T> {
    type Item = HttpResponse;
    type Error = Error;

    fn respond_to<S: 'static>(
        self,
        _req: &HttpRequest<S>,
    ) -> std::result::Result<<Self as Responder>::Item, <Self as Responder>::Error> {
        let bencode = serde_bencode::to_bytes(&self.data)?;

        trace!("bencode response: {}", String::from_utf8_lossy(&bencode));
        Ok(HttpResponse::Ok().content_type("text/plain").body(bencode))
    }
}

fn announce(
    req: HttpRequest<State>,
) -> Either<BencodeResponse<AnnounceData>, FutureResponse<BencodeResponse<AnnounceData>>> {
    let announce = match AnnounceRequest::try_from(req.clone()) {
        Ok(announce) => announce,
        Err(e) => return Either::A(BencodeResponse::failure(&format!("{}", e))),
    };
    trace!("AnnounceRequest: {:#?}", announce);
    Either::B(
        req.state()
            .db()
            .send(announce)
            .from_err()
            .and_then(|result: Result<AnnounceResponse>| match result {
                Ok(mut resp) => {
                    trace!("response data: {:#?}", resp);
                    let mut data = AnnounceData::default();
                    data.interval = match SETTINGS.read() {
                        Ok(s) => s.tracker.interval,
                        Err(e) => {
                            return Err(actix_web::error::ErrorInternalServerError(format!(
                                "{}",
                                e
                            )));
                        }
                    };
                    data.crypto_flags = resp.crypto_flags();
                    data.compact = resp.compact();
                    data.tracker_id = resp.tracker_id();
                    data.complete = resp.complete();
                    data.incomplete = resp.incomplete();

                    for peer in resp.peers() {
                        if data.compact && peer.ip_address.is_ipv6() {
                            data.compact = false;
                        }
                        let mut p = Peer::from(peer);
                        if resp.no_peer_id() {
                            p.peer_id = None;
                        }
                        data.peers.push(p);
                    }

                    Ok(BencodeResponse { data })
                }
                Err(e) => {
                    warn!("announce error: {}", e);
                    trace!("announce error: {:#?}", e);
                    Ok(BencodeResponse::failure(&format!("{}", e)))
                }
            })
            .responder(),
    )
}

fn scrape(
    req: HttpRequest<State>,
) -> Either<BencodeResponse<ScrapeData>, FutureResponse<BencodeResponse<ScrapeData>>> {
    let scrape = match ScrapeRequest::try_from(req.clone()) {
        Ok(scrape) => scrape,
        Err(e) => return Either::A(BencodeResponse::failure(&format!("{}", e))),
    };
    trace!("AnnounceRequest: {:#?}", scrape);

    Either::B(
        req.state()
            .db()
            .send(scrape)
            .from_err()
            .and_then(|result: Result<ScrapeResponse>| match result {
                Ok(resp) => {
                    trace!("response data: {:#?}", resp);
                    let mut data = ScrapeData::default();
                    data.min_interval = match SETTINGS.read() {
                        Ok(s) => Some(s.tracker.interval),
                        Err(_) => None,
                    };

                    data.files = ScrapeFiles(resp.files);

                    trace!("scrape data: {:#?}", data);

                    Ok(BencodeResponse { data })
                }
                Err(e) => {
                    warn!("scrape error: {}", e);
                    trace!("scrape error: {:#?}", e);
                    Ok(BencodeResponse::failure(&format!("{}", e)))
                }
            })
            .responder(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn v4_peer() -> Peer {
        Peer {
            peer_id: Some("v4_peer_padded_to_20".as_bytes().to_vec()),
            sock_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1337u16),
            crypto: false,
            crypto_port: None,
        }
    }

    fn v6_peer() -> Peer {
        Peer {
            peer_id: Some("v6_peer_padded_to_20".as_bytes().to_vec()),
            sock_addr: SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 1337u16),
            crypto: true,
            crypto_port: None,
        }
    }

    #[test]
    fn serialize_v4peer() {
        let mut peer = v4_peer();
        let encoded = serde_bencode::to_string(&peer).unwrap();
        let expected = "d2:ip9:127.0.0.17:peer id20:v4_peer_padded_to_204:porti1337ee";
        assert_eq!(expected, encoded);

        peer.crypto = true;
        peer.crypto_port = Some(31337);
        let encoded = serde_bencode::to_string(&peer).unwrap();
        let expected = "d2:ip9:127.0.0.17:peer id20:v4_peer_padded_to_204:porti31337ee";
        assert_eq!(expected, encoded);
    }

    #[test]
    fn serialize_v6peer() {
        let peer = v6_peer();
        let encoded = serde_bencode::to_string(&peer).unwrap();
        let expected = "d2:ip3:::17:peer id20:v6_peer_padded_to_204:porti1337ee";
        assert_eq!(expected, encoded);
    }

    #[test]
    fn peer_as_bytes() {
        let expected = vec![127u8, 0, 0, 1, 5, 57];
        assert_eq!(expected, v4_peer().as_bytes());
        let expected: Vec<u8> = Vec::new();
        assert_eq!(expected, v6_peer().as_bytes());
    }

    #[test]
    fn serialize_compact_peerlist() {
        let p1 = v4_peer();
        let mut p2 = v4_peer();
        p2.sock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 10, 0, 1)), 31337);
        let p3 = v6_peer();
        let mut list = CompactPeerList::new();
        list.push(&p1);
        list.push(&p2);
        list.push(&p3);

        let expected: Vec<u8> = vec![
            '1' as u8, '2' as u8, ':' as u8, 127, 0, 0, 1, 5, 57, 10, 10, 0, 1, 122, 105,
        ];
        assert_eq!(expected, serde_bencode::to_bytes(&list).unwrap());
    }

    #[test]
    fn serialize_announce_data() {
        let data = AnnounceData {
            failure_reason: Some("i am the reason".to_owned()),
            ..Default::default()
        };
        let actual = serde_bencode::to_string(&data).unwrap();
        assert_eq!("d14:failure reason15:i am the reasone", actual);

        let p1 = v4_peer();
        let mut p2 = v4_peer();
        p2.sock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 10, 0, 1)), 31337);
        p2.crypto = false;
        p2.crypto_port = Some(31338);
        let p3 = v6_peer();
        let peers = vec![p1, p2, p3];
        let mut crypto_peers = peers.clone();
        crypto_peers.get_mut(1).unwrap().crypto = true;

        let mut data = AnnounceData {
            failure_reason: None,
            warning_message: Some("sample warning".to_owned()),
            interval: 1337,
            min_interval: Some(666),
            tracker_id: Some("sample tracker id".as_bytes().to_owned()),
            complete: 23,
            incomplete: 42,
            peers,
            compact: false,
            crypto_flags: false,
        };

        let actual = serde_bencode::to_string(&data).unwrap();
        let expected = "d8:completei23e10:incompletei42e8:intervali1337e12:min intervali666e5:peersld2:ip9:127.0.0.17:peer id20:v4_peer_padded_to_204:porti1337eed2:ip9:10.10.0.17:peer id20:v4_peer_padded_to_204:porti31337eed2:ip3:::17:peer id20:v6_peer_padded_to_204:porti1337eee10:tracker id17:sample tracker id15:warning message14:sample warninge";
        assert_eq!(expected, actual);

        data.warning_message = None;
        data.min_interval = None;
        data.tracker_id = None;
        data.compact = true;

        let actual = serde_bencode::to_bytes(&data).unwrap();
        let mut expected = "d8:completei23e10:incompletei42e8:intervali1337e5:peers"
            .as_bytes()
            .to_vec();
        let mut peers: Vec<u8> = vec![
            '1' as u8, '2' as u8, ':' as u8, 127, 0, 0, 1, 5, 57, 10, 10, 0, 1, 122, 105,
        ];
        expected.append(&mut peers);
        expected.push('e' as u8);
        assert_eq!(expected, actual);

        data.compact = false;
        data.crypto_flags = true;
        data.peers = crypto_peers;

        let actual = serde_bencode::to_string(&data).unwrap();
        let expected = "d8:completei23e12:crypto_flags3:01110:incompletei42e8:intervali1337e5:peersld2:ip9:127.0.0.17:peer id20:v4_peer_padded_to_204:porti1337eed2:ip9:10.10.0.17:peer id20:v4_peer_padded_to_204:porti31338eed2:ip3:::17:peer id20:v6_peer_padded_to_204:porti1337eeee";
        assert_eq!(expected, actual);

        data.compact = true;

        let actual = serde_bencode::to_bytes(&data).unwrap();
        let mut expected =
            "d8:completei23e12:crypto_flags2:0110:incompletei42e8:intervali1337e5:peers"
                .as_bytes()
                .to_vec();
        let mut peers: Vec<u8> = vec![
            '1' as u8, '2' as u8, ':' as u8, 127, 0, 0, 1, 5, 57, 10, 10, 0, 1, 122, 106,
        ];
        expected.append(&mut peers);
        expected.push('e' as u8);
        assert_eq!(expected, actual);
    }

    #[test]
    fn serialize_scrape_data() {
        let data = ScrapeData {
            failure_reason: Some("i am the reason".to_owned()),
            ..Default::default()
        };
        let actual = serde_bencode::to_string(&data).unwrap();
        assert_eq!("d14:failure reason15:i am the reasone", actual);

        let files: Vec<ScrapeFile> = vec![
            ScrapeFile {
                info_hash: [65u8; 20].to_vec(),
                complete: 111,
                incomplete: 222,
                downloaded: 333,
            },
            ScrapeFile {
                info_hash: [78u8; 20].to_vec(),
                complete: 10,
                incomplete: 20,
                downloaded: 30,
            },
        ];
        let files = ScrapeFiles(files);

        let data = ScrapeData {
            failure_reason: None,
            min_interval: Some(900),
            files,
        };

        let actual = serde_bencode::to_string(&data).unwrap();
        let expected = "d5:filesd20:AAAAAAAAAAAAAAAAAAAAd8:completei111e10:downloadedi333e10:incompletei222ee20:NNNNNNNNNNNNNNNNNNNNd8:completei10e10:downloadedi30e10:incompletei20eee5:flagsd12:min intervali900eee";
        assert_eq!(expected, actual);
    }
}
