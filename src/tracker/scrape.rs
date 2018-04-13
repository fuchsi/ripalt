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

use std::convert::TryFrom;

use url::percent_encoding::percent_decode;

#[derive(Debug, Clone)]
pub struct ScrapeRequest {
    info_hashes: Vec<Vec<u8>>,
}

impl<S> TryFrom<HttpRequest<S>> for ScrapeRequest {
    type Error = Error;

    fn try_from(req: HttpRequest<S>) -> Result<Self> {
        trace!("Request: {:#?}", req);
        trace!("uri: {:#?}", req.uri());
        let query_str = req.uri().query().ok_or_else(|| "no query")?;
        trace!("query: {:#?}", query_str);
        let iter = query_str.split('&');
        let mut info_hashes_str: Vec<&str> = Vec::new();
        for part in iter {
            if let Some(pos) = part.find('=') {
                let key = &part[0..pos];
                if key == "info_hash" {
                    let value = &part[pos + 1..];
                    info_hashes_str.push(value);
                }
            }
        }
        trace!("info_hashes: {:#?}", info_hashes_str);
        let mut info_hashes: Vec<Vec<u8>> = Vec::new();
        for info_hash in info_hashes_str {
            let info_hash = percent_decode(info_hash.as_bytes()).if_any();
            if let Some(info_hash) = info_hash {
                info_hashes.push(info_hash);
            }
        }

        if info_hashes.is_empty() {
            bail!("no info hashes provided");
        }

        Ok(ScrapeRequest{info_hashes})
    }
}

impl Message for ScrapeRequest {
    type Result = Result<ScrapeResponse>;
}

impl Handler<ScrapeRequest> for DbExecutor {
    type Result = Result<ScrapeResponse>;

    fn handle(&mut self, msg: ScrapeRequest, _ctx: &mut Self::Context) -> <Self as Handler<ScrapeRequest>>::Result {
        let conn = self.conn();
        let mut files: Vec<ScrapeFile> = Vec::new();

        for info_hash in msg.info_hashes {
            let (complete, incomplete, downloaded) = models::TorrentList::peer_count_scrape(&info_hash, &conn);
            files.push(ScrapeFile{info_hash, complete, incomplete, downloaded});
        }

        Ok(ScrapeResponse{files})
    }
}

#[derive(Debug)]
pub struct ScrapeResponse {
    pub files: Vec<ScrapeFile>,
}

#[derive(Debug)]
pub struct ScrapeFile {
    pub info_hash: Vec<u8>,
    pub complete: i64,
    pub incomplete: i64,
    pub downloaded: i32,
}

impl Serialize for ScrapeFile {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut root = s.serialize_map(Some(3))?;
        root.serialize_entry("complete", &self.complete)?;
        root.serialize_entry("downloaded", &self.downloaded)?;
        root.serialize_entry("incomplete", &self.incomplete)?;
        root.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_scrape_file() {
        let file = ScrapeFile{
            info_hash: [0u8; 20].to_vec(),
            complete: 111,
            incomplete: 222,
            downloaded: 333,
        };

        let expected = "d8:completei111e10:downloadedi333e10:incompletei222ee";
        let actual = serde_bencode::to_string(&file).unwrap();
        assert_eq!(expected, actual);
    }
}