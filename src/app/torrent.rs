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

use bytes::Bytes;
use codepage_437::{BorrowFromCp437, CP437_CONTROL};
use futures::Future;
use multipart::server::{save::SavedData, Entries, Multipart, SaveResult};
use std::io::{Cursor, Read};

use handlers::torrent::*;
use models::{torrent::TorrentFile, Category, Torrent, TorrentMsg, TorrentList};

#[derive(Debug, Serialize)]
struct ListContext {
    categories: Vec<Category>,
    list: Vec<TorrentList>,
    total_count: i64,
    count: i64,
    page: i64,
    pages: i64,
    per_page: i64,
    category: Option<Uuid>,
    name: Option<String>,
    visible: Option<String>,
    timezone: i32,
}

#[derive(Debug, Deserialize)]
pub struct ListForm {
    category: String,
    visible: String,
    name: String,
    page: i64,
}

impl ListForm {
    fn visible(&self) -> Visible {
        match &self.visible[..] {
            x if x == "all" => Visible::All,
            x if x == "visible" => Visible::Visible,
            x if x == "dead" => Visible::Invisible,
            _ => Visible::Visible,
        }
    }
}

pub fn list(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let user_id = match session_creds(&mut req) {
        Some((u, _)) => u,
        None => return async_redirect("/login"),
    };

    // page size is overwritten in the handler with the user defined value
    let page_size = SETTINGS.read().unwrap().user.default_torrents_per_page;
    let page = 1i64;
    let mut torrent_list = LoadTorrentList::new(&user_id);
    torrent_list.page(page as i64, page_size as i64);

    let fut_form;
    {
        let req = req.clone();
        fut_form = req.urlencoded::<ListForm>()
            .then(move |result| match result {
                Ok(form) => {
                    let visible = form.visible();
                    let ListForm { name, mut category, page, .. } = form;
                    torrent_list.name(name);
                    if let Ok(category) = Uuid::parse_str(&category[..]) {
                        torrent_list.category(category);
                    }
                    torrent_list.visible(visible);
                    torrent_list.page(page, page_size);

                    Ok(torrent_list)
                }
                // return the "default" torrent_list, if the form could not be parsed (ie is not present/not a post request)
                Err(_) => Ok(torrent_list),
            });
    }

    let fut_db;
    {
        let req = req.clone();
        fut_db = fut_form.and_then(move |torrent_list| req.state().db().send(torrent_list))
    }

    let fut_response = fut_db.from_err().and_then(move |result: Result<TorrentListMsg>| {
        let msg = result.unwrap_or_else(|_| TorrentListMsg::default());
        let categories = categories(req.state());
        let total_count = msg.count;
        let count = msg.torrents.len() as i64;
        let pages = total_count / msg.request.per_page;

        let ctx = ListContext {
            categories,
            list: msg.torrents,
            total_count,
            count,
            page: msg.request.page,
            pages,
            per_page: msg.request.per_page,
            name: msg.request.name,
            visible: Some(msg.request.visible.to_string()),
            category: msg.request.category,
            timezone: msg.timezone,
        };
        let reg = &req.state().template();
        Template::render(&reg, "torrent/list.html", &ctx).map(|t| t.into())
    });

    fut_response.responder()
}

pub fn new(req: HttpRequest<State>) -> SyncResponse<Template> {
    let mut ctx = HashMap::new();
    ctx.insert("categories", categories(req.state()));

    Template::render(&req.state().template(), "torrent/new.html", &ctx)
}

#[derive(Serialize)]
struct UploadContext {
    categories: Vec<Category>,
    error: String,
}

pub fn create(mut req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let user_id = match req.session().get::<Uuid>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => return Either::A(redirect("/login")),
    };

    let cloned = req.clone();
    let fut_prepare = req.clone()
        .body()
        .from_err()
        .and_then(move |body| -> Result<NewTorrent> {
            let content_type = cloned.headers()[header::CONTENT_TYPE].to_str().unwrap();
            let mpr = MultipartRequest::new(content_type, body);
            let mut multipart = Multipart::from_request(mpr).unwrap();
            // Fetching all data and processing it.
            // save().temp() reads the request fully, parsing all fields and saving all files
            // in a new temporary directory under the OS temporary directory.
            match multipart.save().temp() {
                SaveResult::Full(entries) => process_upload(&entries),
                SaveResult::Partial(_, reason) => Err(format!("partial read: {:?}", reason).into()),
                SaveResult::Error(error) => Err(format!("io error: {}", error).into()),
            }
        });

    let fut_process = fut_prepare
        .map_err(|error| actix_web::error::ErrorInternalServerError(format!("{}", error)))
        .and_then(move |mut torrent| {
            torrent.user = user_id;
            Ok(torrent)
        });

    let cloned = req.clone();
    let fut_result = fut_process.and_then(move |torrent| {
        cloned
            .state()
            .db()
            .send(torrent)
            .map_err(|error| actix_web::error::ErrorInternalServerError(format!("{}", error)))
    });

    let cloned = req.clone();
    let fut_response = fut_result.and_then(move |result: Result<models::Torrent>| match result {
        Ok(torrent) => sync_redirect(&format!("/torrent/{}", torrent.id)[..]),
        Err(e) => {
            let ctx = UploadContext {
                categories: categories(cloned.state()),
                error: e.to_string(),
            };

            Template::render(&cloned.state().template(), "torrent/new.html", &ctx).map(|t| t.into())
        }
    });

    let cloned = req.clone();
    let fut_response = fut_response.or_else(move |e| {
        let ctx = UploadContext {
            categories: categories(cloned.state()),
            error: e.to_string(),
        };

        let hbs = &cloned.state().template();
        Template::render(hbs, "torrent/new.html", &ctx).map(|t| t.into())
    });

    Either::B(fut_response.responder())
}

fn process_upload(entries: &Entries) -> Result<NewTorrent> {
    let mut upload_builder = NewTorrentBuilder::new();
    let mut key = String::from("torrent_name");
    let mut has_name = false;

    let name = &entries.fields[&key][0];
    if let SavedData::Text(ref name) = name.data {
        trace!("name: {:?}", name);
        if !name.is_empty() {
            has_name = true;
            upload_builder.name(name);
        }
    }
    key = String::from("description");
    let desc = &entries.fields[&key][0];
    if let SavedData::Text(ref desc) = desc.data {
        trace!("desc: {:?}", desc);
        upload_builder.description(desc);
    }
    key = String::from("category");
    let category = &entries.fields[&key][0];
    if let SavedData::Text(ref category) = category.data {
        trace!("category: {:?}", category);
        upload_builder.category(Uuid::parse_str(&category[..]).unwrap());
    }
    key = String::from("nfo_file");
    let nfo = &entries.fields[&key][0];
    match nfo.headers.content_type {
        Some(ref c) if c.type_() == "text" => match nfo.data {
            SavedData::Bytes(ref b) => {
                trace!("nfo from bytes: {:?}", b);
                upload_builder.nfo(b.clone());
            }
            SavedData::Text(ref s) => {
                trace!("nfo from str: {:?}", s);
                upload_builder.nfo(s.as_bytes().to_vec());
            }
            SavedData::File(ref path, size) => {
                let mut file = std::fs::File::open(path).unwrap();
                let mut buf: Vec<u8> = Vec::with_capacity(size as usize);
                let res = file.read_to_end(&mut buf);
                trace!("nfo from file({:?} {}B): {:?}", path, size, buf);
                trace!("read result: {:?}", res);
                upload_builder.nfo(buf);
            }
        },
        _ => bail!("no nfo"),
    }
    key = String::from("meta_file");
    let meta_file = &entries.fields[&key][0];
    match meta_file.headers.content_type {
        Some(ref c) if c.type_() == "application" && c.subtype() == "x-bittorrent" => {
            match meta_file.data {
                SavedData::Bytes(ref b) => {
                    trace!("meta_file from bytes: {:?}", b);
                    upload_builder.raw_meta(b.clone());
                }
                SavedData::Text(ref s) => {
                    trace!("meta_file from s: {:?}", s);
                    upload_builder.raw_meta(s.as_bytes().to_vec());
                }
                SavedData::File(ref path, size) => {
                    let mut file = std::fs::File::open(path).unwrap();
                    let mut buf: Vec<u8> = Vec::with_capacity(size as usize);
                    let res = file.read_to_end(&mut buf);
                    trace!("meta_file from file({:?} {}B): {:?}", path, size, buf);
                    trace!("read result: {:?}", res);
                    upload_builder.raw_meta(buf);
                }
            }

            if !has_name {
                if let Some(ref name) = meta_file.headers.filename {
                    let name = &name[0..(name.len()-8)];
                    upload_builder.name(&name);
                }
            }
        }
        _ => bail!("no meta file"),
    }

    upload_builder.finish()
}

#[derive(Debug, Clone)]
struct MultipartRequest {
    body: Bytes,
    boundary: String,
}

impl MultipartRequest {
    pub fn new(content_type: &str, body: Bytes) -> Self {
        debug!("content-type: {}", content_type);
        let boundary: String = match content_type.rfind("boundary=") {
            // todo: check for trailing stuff
            Some(index) => {
                let index = index + 9; // add length of 'boundary='
                String::from(&content_type[index..])
            }
            None => String::from("--"),
        };
        debug!("boundary: {}", boundary);
        MultipartRequest { body, boundary }
    }
}

impl multipart::server::HttpRequest for MultipartRequest {
    type Body = Cursor<Bytes>;

    fn multipart_boundary(&self) -> Option<&str> {
        Some(&self.boundary[..])
    }

    fn body(self) -> <Self as multipart::server::HttpRequest>::Body {
        Cursor::new(self.body)
    }
}

#[derive(Debug, Serialize)]
struct ShowContext<'a> {
    torrent: ShowTorrent<'a>,
    torrent_user_name: &'a Option<String>,
    nfo: String,
    files: &'a Vec<TorrentFile>,
    seeder: Vec<ShowPeer<'a>>,
    leecher: Vec<ShowPeer<'a>>,
    category: &'a models::Category,
    num_seeder: usize,
    num_leecher: usize,
    num_files: usize,
    may_edit: bool,
    may_delete: bool,
    timezone: i32,
}

impl<'a> From<&'a TorrentMsg> for ShowContext<'a> {
    fn from(tc: &'a TorrentMsg) -> Self {
        let mut seeder = Vec::new();
        let mut leecher = Vec::new();
        for (peer, user) in &tc.peers {
            let mut p = ShowPeer::from(peer);
            p.user_name = &user[..];
            if peer.seeder {
                seeder.push(p);
            } else {
                if peer.bytes_left != tc.torrent.size {
                    let downloaded = tc.torrent.size - peer.bytes_left;
                    p.complete_ratio = format!(
                        "{:.2}%",
                        (downloaded as f64 / tc.torrent.size as f64) * 100.0
                    )
                }
                leecher.push(p);
            }
        }
        let num_seeder = seeder.len();
        let num_leecher = leecher.len();
        let category = &tc.category;
        let nfo = match tc.nfo {
            Some(ref nfo) => String::borrow_from_cp437(&nfo.data, &CP437_CONTROL),
            None => String::from("no nfo"),
        };

        ShowContext {
            torrent: ShowTorrent::from(&tc.torrent),
            torrent_user_name: &tc.torrent_user_name,
            nfo,
            files: &tc.files,
            seeder,
            leecher,
            category,
            num_seeder,
            num_leecher,
            num_files: tc.files.len(),
            may_edit: false,
            may_delete: false,
            timezone: tc.timezone,
        }
    }
}

#[derive(Debug, Serialize)]
struct ShowPeer<'a> {
    id: &'a Uuid,
    torrent_id: &'a Uuid,
    user_id: &'a Uuid,
    user_name: &'a str,
    ip_address: String,
    bytes_uploaded: &'a i64,
    bytes_downloaded: &'a i64,
    bytes_left: &'a i64,
    ratio: String,
    complete_ratio: String,
    seeder: &'a bool,
    user_agent: &'a str,
    crypto_enabled: &'a bool,
    offset_uploaded: &'a i64,
    offset_downloaded: &'a i64,
    created_at: String,
    finished_at: String,
    updated_at: String,
}

impl<'a> From<&'a models::Peer> for ShowPeer<'a> {
    fn from(peer: &'a models::Peer) -> Self {
        let ip_address = format!("{}", peer.ip_address);
        let format_string = "%d.%m.%Y %H:%M:%S %Z";

        let created_at = peer.created_at.format(format_string).to_string();
        let updated_at = peer.updated_at.format(format_string).to_string();
        let finished_at = match peer.finished_at {
            Some(dt) => dt.format(format_string).to_string(),
            None => String::from("--"),
        };
        let ratio = match peer.bytes_downloaded {
            0 => String::from("0.000"),
            _ => format!(
                "{:.3}",
                (peer.bytes_uploaded as f64 / peer.bytes_downloaded as f64)
            ),
        };
        let complete_ratio = String::new();

        ShowPeer {
            id: &peer.id,
            torrent_id: &peer.torrent_id,
            user_id: &peer.user_id,
            user_name: "",
            ip_address,
            bytes_uploaded: &peer.bytes_uploaded,
            bytes_downloaded: &peer.bytes_downloaded,
            bytes_left: &peer.bytes_left,
            ratio,
            complete_ratio,
            seeder: &peer.seeder,
            user_agent: &peer.user_agent,
            crypto_enabled: &peer.crypto_enabled,
            offset_uploaded: &peer.offset_uploaded,
            offset_downloaded: &peer.offset_downloaded,
            created_at,
            finished_at,
            updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct ShowTorrent<'a> {
    id: &'a Uuid,
    name: &'a str,
    user_id: &'a Option<Uuid>,
    info_hash: String,
    category_id: &'a Uuid,
    description: &'a str,
    size: &'a i64,
    completed: &'a i32,
    last_action: &'a Option<DateTime<Utc>>,
    last_seeder: &'a Option<DateTime<Utc>>,
    created_at: &'a DateTime<Utc>,
}

impl<'a> From<&'a Torrent> for ShowTorrent<'a> {
    fn from(torrent: &'a Torrent) -> Self {

        ShowTorrent {
            id: &torrent.id,
            name: &torrent.name[..],
            user_id: &torrent.user_id,
            info_hash: util::to_hex(&torrent.info_hash),
            category_id: &torrent.category_id,
            description: &torrent.description[..],
            size: &torrent.size,
            completed: &torrent.completed,
            last_action: &torrent.last_action,
            last_seeder: &torrent.last_seeder,
            created_at: &torrent.created_at,
        }
    }
}

pub fn torrent(mut req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return Either::A(redirect("/login")),
    };
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => {
                return Either::A(
                    actix_web::error::ErrorInternalServerError(format!("{}", e)).into(),
                )
            }
        },
        Err(_) => return Either::A(not_found(req).unwrap()),
    };

    let fut_response = req.clone()
        .state()
        .db()
        .send(LoadTorrent::new(&id, &user_id))
        .from_err()
        .and_then(move |result: Result<TorrentMsg>| match result {
            Ok(tc) => {
                let mut ctx = ShowContext::from(&tc);
                {
                    let acl = req.state().acl();
                    let subj = UserSubject::new(&user_id, &group_id, &acl);
                    ctx.may_edit = subj.may_write(&tc.torrent);
                    ctx.may_delete = subj.may_delete(&tc.torrent);
                }
                Template::render(&req.state().template(), "torrent/show.html", &ctx)
                    .map(|t| t.into())
            }
            Err(e) => {
                info!("torrent '{}' not found: {}", id, e);
                not_found(req)
            }
        });

    Either::B(fut_response.responder())
}

pub fn edit(req: HttpRequest<State>) -> SyncResponse<Template> {
    let ctx = Context::new();
    Template::render(&req.state().template(), "torrent/edit.html", &ctx)
}

pub fn update(req: HttpRequest<State>) -> SyncResponse<Template> {
    let ctx = Context::new();
    Template::render(&req.state().template(), "torrent/edit.html", &ctx)
}

pub fn delete(req: HttpRequest<State>) -> SyncResponse<Template> {
    let ctx = Context::new();
    Template::render(&req.state().template(), "torrent/delete.html", &ctx)
}

pub fn download(mut req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => {
                return Either::A(
                    actix_web::error::ErrorInternalServerError(format!("{}", e)).into(),
                )
            }
        },
        Err(_) => return Either::A(not_found(req).unwrap()),
    };
    let uid = req.session().get::<uuid::Uuid>("user_id").unwrap().unwrap();
    let fut_response = req.clone()
        .state()
        .db()
        .send(LoadTorrentMeta {
            id: id.clone(),
            uid,
        })
        .from_err()
        .and_then(
            move |result: Result<(String, Vec<u8>, Vec<u8>)>| match result {
                Ok((name, meta_file, passcode)) => {
                    let announce_url = &SETTINGS.read().unwrap().tracker.announce_url[..];
                    let comment = &SETTINGS.read().unwrap().tracker.comment[..];
                    let announce_url = format!("{}/{}", announce_url, util::to_hex(&passcode));
                    let meta_file = util::torrent::rewrite(&meta_file, &announce_url[..], comment)
                        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{}", e)))?;

                    Ok(HttpResponse::build(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/x-bittorent")
                        .header(
                            header::CONTENT_DISPOSITION,
                            format!("attachment; filename=\"{}\"", name),
                        )
                        .body(meta_file))
                }
                Err(e) => {
                    info!("torrent '{}' not found: {}", id, e);
                    not_found(req)
                }
            },
        );

    Either::B(fut_response.responder())
}

fn categories(s: &State) -> Vec<models::Category> {
    if let Ok(categories) = s.db().send(Categories {}).wait() {
        categories.unwrap_or_else(|_| vec![])
    } else {
        vec![]
    }
}
