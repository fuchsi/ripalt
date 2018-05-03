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
use actix_web::HttpMessage;
use codepage_437::{BorrowFromCp437, CP437_CONTROL};
use futures::Future;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

use handlers::torrent::*;
use handlers::UserSubjectMsg;
use models::acl::Subject;
use models::{torrent::{TorrentFile, TorrentImage},
             Category,
             Torrent,
             TorrentList,
             TorrentMsg};

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
    let mut torrent_list = LoadTorrentListMsg::new(&user_id);
    torrent_list.page(page as i64, page_size as i64);

    let fut_form;
    {
        let req = req.clone();
        fut_form = req.urlencoded::<ListForm>()
            .then(move |result| match result {
                Ok(form) => {
                    let visible = form.visible();
                    let ListForm {
                        name,
                        mut category,
                        page,
                        ..
                    } = form;
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

    let fut_response = fut_db
        .from_err()
        .and_then(move |result: Result<TorrentListMsg>| {
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
            Template::render(&reg, "torrent/list.html", &ctx)
        });

    fut_response.responder()
}

pub fn new(req: HttpRequest<State>) -> SyncResponse<HttpResponse> {
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
        .limit(8_388_608) // 8MB Limit
        .from_err()
        .and_then(move |body| -> Result<NewTorrentMsg> {
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
        .map_err(|error| ErrorInternalServerError(error.to_string()))
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
            .map_err(|error| ErrorInternalServerError(error))
    });

    let cloned = req.clone();
    let fut_response = fut_result.and_then(move |result: Result<models::Torrent>| match result {
        Ok(torrent) => sync_redirect(&format!("/torrent/{}", torrent.id)[..]),
        Err(e) => {
            let ctx = UploadContext {
                categories: categories(cloned.state()),
                error: e.to_string(),
            };

            Template::render(&cloned.state().template(), "torrent/new.html", &ctx)
        }
    });

    let cloned = req.clone();
    let fut_response = fut_response.or_else(move |e| {
        let ctx = UploadContext {
            categories: categories(cloned.state()),
            error: e.to_string(),
        };

        let hbs = &cloned.state().template();
        Template::render(hbs, "torrent/new.html", &ctx)
    });

    Either::B(fut_response.responder())
}

fn process_upload(entries: &Entries) -> Result<NewTorrentMsg> {
    let mut upload_builder = NewTorrentBuilder::new();
    let mut key = String::from("torrent_name");
    let mut has_name = false;

    let name = &entries.fields[&key][0];
    if let SavedData::Text(ref name) = name.data {
        if !name.is_empty() {
            has_name = true;
            upload_builder.name(name);
        }
    }
    key = String::from("description");
    let desc = &entries.fields[&key][0];
    if let SavedData::Text(ref desc) = desc.data {
        upload_builder.description(desc);
    }
    key = String::from("category");
    let category = &entries.fields[&key][0];
    if let SavedData::Text(ref category) = category.data {
        upload_builder.category(Uuid::parse_str(&category[..]).unwrap());
    }
    key = String::from("nfo_file");
    let nfo = &entries.fields[&key][0];
    let use_nfo_as_description = &entries.fields.get(&"nfo_as_description".to_string())
        .map(|v| if let SavedData::Text(ref v) = v[0].data { v == "1"} else { false } )
        .unwrap_or_default();
    match nfo.headers.content_type {
        Some(ref c) if c.type_() == "text" => match nfo.data {
            SavedData::Bytes(ref b) => {
                upload_builder.nfo(b.clone());
            }
            SavedData::Text(ref s) => {
                upload_builder.nfo(s.as_bytes().to_vec());
            }
            SavedData::File(ref path, size) => {
                let mut file = fs::File::open(path)?;
                let mut buf: Vec<u8> = Vec::with_capacity(size as usize);
                file.read_to_end(&mut buf)?;
                upload_builder.nfo(buf);
            }
        },
        _ => bail!("no nfo"),
    }
    if *use_nfo_as_description {
        upload_builder.nfo_as_description();
    }

    key = String::from("meta_file");
    let meta_file = &entries.fields[&key][0];
    match meta_file.headers.content_type {
        Some(ref c) if c.type_() == "application" && c.subtype() == "x-bittorrent" => {
            match meta_file.data {
                SavedData::Bytes(ref b) => {
                    upload_builder.raw_meta(b.clone());
                }
                SavedData::Text(ref s) => {
                    upload_builder.raw_meta(s.as_bytes().to_vec());
                }
                SavedData::File(ref path, size) => {
                    let mut file = fs::File::open(path).unwrap();
                    let mut buf: Vec<u8> = Vec::with_capacity(size as usize);
                    file.read_to_end(&mut buf)?;
                    upload_builder.raw_meta(buf);
                }
            }

            if !has_name {
                if let Some(ref name) = meta_file.headers.filename {
                    let name = &name[0..(name.len() - 8)];
                    upload_builder.name(&name);
                }
            }
        }
        _ => bail!("no meta file"),
    }

    let ts = Utc::now().timestamp();
    key = String::from("images");
    if let Some(ref images) = &entries.fields.get(&key) {
        for (i, image) in images.iter().enumerate() {
            match image.headers.content_type {
                Some(ref c) if c.type_() == "image" => {
                    if let Some(ref name) = image.headers.filename {
                        // we only need the filename for the extension.
                        // the file will be stored as: index_timestamp.ext
                        let p = Path::new(name);
                        let ext = p.extension()
                            .and_then(|s| s.to_str())
                            .map_or("".to_string(), |s| s.to_ascii_lowercase());
                        let name = format!("{}_{}.{}", i, ts, ext);
                        match image.data {
                            SavedData::Bytes(ref b) => {
                                let mut file = NamedTempFile::new()?;
                                file.write_all(b)?;
                                upload_builder.add_image(&name, file.into_temp_path());
                            }
                            SavedData::Text(ref s) => {
                                let b = s.as_bytes();
                                let mut file = NamedTempFile::new()?;
                                file.write_all(b)?;
                                upload_builder.add_image(&name, file.into_temp_path());
                            }
                            SavedData::File(ref path, _size) => {
                                let mut file = NamedTempFile::new()?;
                                // copy the whole uploaded file into a new tempfile. it's stupid, but works...
                                // todo: fix this nonsense
                                let mut source = fs::File::open(path)?;
                                io::copy(&mut source, &mut file)?;
                                upload_builder.add_image(&name, file.into_temp_path());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    upload_builder.finish()
}

#[derive(Debug, Serialize)]
struct ShowContext<'a> {
    torrent: ShowTorrent<'a>,
    torrent_user_name: &'a Option<String>,
    nfo: String,
    files: &'a Vec<TorrentFile>,
    images: &'a Vec<TorrentImage>,
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
            images: &tc.images,
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
struct EditContext<'a> {
    torrent: ShowTorrent<'a>,
    may_delete: bool,
    categories: Vec<Category>,
}

impl<'a> From<&'a TorrentMsg> for EditContext<'a> {
    fn from(tc: &'a TorrentMsg) -> Self {
        EditContext {
            torrent: ShowTorrent::from(&tc.torrent),
            may_delete: false,
            categories: Vec::new(),
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

pub fn torrent(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
        },
        Err(e) => return Box::new(FutErr(ErrorNotFound(e))),
    };

    req.clone()
        .state()
        .db()
        .send(LoadTorrentMsg::new(&id, &user_id))
        .from_err()
        .and_then(move |result: Result<TorrentMsg>| match result {
            Ok(tc) => {
                let mut ctx = ShowContext::from(&tc);
                {
                    let subj = UserSubject::new(&user_id, &group_id, req.state().acl_arc());
                    ctx.may_edit = subj.may_write(&tc.torrent);
                    ctx.may_delete = subj.may_delete(&tc.torrent);
                }
                Template::render(&req.state().template(), "torrent/show.html", &ctx)
            }
            Err(e) => {
                info!("torrent '{}' not found: {}", id, e);
                Err(ErrorNotFound(e.to_string()))
            }
        })
        .responder()
}

pub fn edit(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
        },
        Err(e) => return Box::new(FutErr(ErrorNotFound(e))),
    };

    req.clone()
        .state()
        .db()
        .send(LoadTorrentMsg::new(&id, &user_id))
        .from_err()
        .and_then(move |result: Result<TorrentMsg>| match result {
            Ok(tc) => {
                let mut ctx = EditContext::from(&tc);
                ctx.categories = categories(&req.state());
                let may_edit = {
                    let subj = UserSubject::new(&user_id, &group_id, req.state().acl_arc());
                    ctx.may_delete = subj.may_delete(&tc.torrent);
                    subj.may_write(&tc.torrent)
                };

                if may_edit {
                    Template::render(&req.state().template(), "torrent/edit.html", &ctx)
                } else {
                    let mut ctx = Context::new();
                    ctx.insert("id", &id);
                    ctx.insert("title", &format!("Edit Torrent: {}", tc.torrent.name));
                    ctx.insert("message", "You are not allowed to edit this Torrent");
                    Template::render(&req.state().template(), "torrent/denied.html", &ctx)
                }
            }
            Err(e) => {
                info!("torrent '{}' not found: {}", id, e);
                Err(ErrorNotFound(e.to_string()))
            }
        })
        .responder()
}

pub fn update(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
        },
        Err(e) => return Box::new(FutErr(ErrorNotFound(e))),
    };

    let acl = req.state().acl_arc();
    let cloned = req.clone();
    let fut_prepare = req.clone()
        .body()
        .limit(8_388_608) // 8MB Limit
        .from_err()
        .and_then(move |body| -> Result<UpdateTorrentMsg> {
            let content_type = cloned.headers()[header::CONTENT_TYPE].to_str().unwrap();
            let mpr = MultipartRequest::new(content_type, body);
            let mut multipart = Multipart::from_request(mpr).unwrap();
            // Fetching all data and processing it.
            // save().temp() reads the request fully, parsing all fields and saving all files
            // in a new temporary directory under the OS temporary directory.
            match multipart.save().temp() {
                SaveResult::Full(entries) => {
                    let torrent = UpdateTorrentMsg::new(id, UserSubjectMsg::new(user_id, group_id, acl));
                    process_update(&entries, torrent)
                },
                SaveResult::Partial(_, reason) => {
                    debug!("{:#?}", reason);
                    Err(format!("partial read: {:?}", reason).into())
                },
                SaveResult::Error(error) => Err(format!("io error: {}", error).into()),
            }
        });

    let cloned = req.clone();
    let fut_result = fut_prepare
        .map_err(|e| {
            error!("{}", e);
            ErrorInternalServerError(e.to_string())
        })
        .and_then(move |torrent| {
            cloned.state().db().send(torrent).map_err(|error| {
                error!("{}", error);
                ErrorInternalServerError(error)
            })
        });

    let cloned = req.clone();
    fut_result
        .from_err()
        .and_then(move |result| {
            let mut ctx = Context::new();
            ctx.insert("id", &id);

            match result {
                Ok(_) => {
                    ctx.insert("message", "Torrent was edited.");
                    ctx.insert("title", "Edit Torrent");
                    ctx.insert("sub_title", "Edit Succeeded");
                    ctx.insert(
                        "continue_link",
                        &cloned
                            .url_for("torrent#read", &[id.to_string()])
                            .unwrap()
                            .to_string(),
                    );
                    Template::render(&cloned.state().template(), "torrent/success.html", &ctx)
                }
                Err(e) => {
                    ctx.insert("error", &e.to_string());
                    ctx.insert("title", "Edit Torrent");
                    ctx.insert("sub_title", "Edit Failed");
                    ctx.insert(
                        "back_link",
                        &cloned
                            .url_for("torrent#edit", &[id.to_string()])
                            .unwrap()
                            .to_string(),
                    );
                    Template::render(&cloned.state().template(), "torrent/failed.html", &ctx)
                }
            }
        })
        .responder()
}

fn process_update(entries: &Entries, mut msg: UpdateTorrentMsg) -> Result<UpdateTorrentMsg> {
    let mut key = "torrent_name".to_string();

    let name = &entries.fields[&key][0];
    if let SavedData::Text(ref name) = name.data {
        if !name.is_empty() {
            msg.name = name.clone();
        }
    }

    key = "description".to_string();
    let desc = &entries.fields[&key][0];
    if let SavedData::Text(ref desc) = desc.data {
        msg.description = desc.clone();
    }

    key = "category".to_string();
    let category = &entries.fields[&key][0];
    if let SavedData::Text(ref category) = category.data {
        msg.category = Uuid::parse_str(category)?;
    }
    key = "nfo_file".to_string();
    if let Some(ref nfo) = entries.fields.get(&key) {
        let nfo = &nfo[0];
        match nfo.headers.content_type {
            Some(ref c) if c.type_() == "text" => match nfo.data {
                SavedData::Bytes(ref b) => {
                    msg.nfo_file = Some(b.clone());
                }
                SavedData::Text(ref s) => {
                    msg.nfo_file = Some(s.as_bytes().to_vec());
                }
                SavedData::File(ref path, size) => {
                    let mut file = std::fs::File::open(path)?;
                    let mut buf: Vec<u8> = Vec::with_capacity(size as usize);
                    file.read_to_end(&mut buf)?;
                    msg.nfo_file = Some(buf);
                }
            },
            _ => {}
        }
    }

    let ts = Utc::now().timestamp();
    key = String::from("images");
    if let Some(ref images) = &entries.fields.get(&key) {
        let key = String::from("replace_images");
        if let Some(replace) = &entries.fields.get(&key) {
            if let SavedData::Text(ref replace) = replace[0].data {
                msg.replace_images = replace == "1";
            }
        }

        for (i, image) in images.iter().enumerate() {
            match image.headers.content_type {
                Some(ref c) if c.type_() == "image" => {
                    if let Some(ref name) = image.headers.filename {
                        // we only need the filename for the extension.
                        // the file will be stored as: index_timestamp.ext
                        let p = Path::new(name);
                        let ext = p.extension()
                            .and_then(|s| s.to_str())
                            .map_or("".to_string(), |s| s.to_ascii_lowercase());
                        let name = format!("{}_{}.{}", i, ts, ext);
                        match image.data {
                            SavedData::Bytes(ref b) => {
                                let mut file = NamedTempFile::new()?;
                                file.write_all(b)?;
                                msg.image_files.push((name, file.into_temp_path()));
                            }
                            SavedData::Text(ref s) => {
                                let b = s.as_bytes();
                                let mut file = NamedTempFile::new()?;
                                file.write_all(b)?;
                                msg.image_files.push((name, file.into_temp_path()));
                            }
                            SavedData::File(ref path, _size) => {
                                let mut file = NamedTempFile::new()?;
                                // copy the whole uploaded file into a new tempfile. it's stupid, but works...
                                // todo: fix this nonsense
                                let mut source = fs::File::open(path)?;
                                io::copy(&mut source, &mut file)?;
                                msg.image_files.push((name, file.into_temp_path()));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(msg)
}

pub fn delete(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
        },
        Err(e) => return Box::new(FutErr(ErrorNotFound(e))),
    };

    req.clone()
        .state()
        .db()
        .send(LoadTorrentMsg::new(&id, &user_id))
        .from_err()
        .and_then(move |result: Result<TorrentMsg>| match result {
            Ok(tc) => {
                let mut ctx = EditContext::from(&tc);
                let may_edit = {
                    let subj = UserSubject::new(&user_id, &group_id, req.state().acl_arc());
                    subj.may_delete(&tc.torrent)
                };

                if may_edit {
                    Template::render(&req.state().template(), "torrent/delete.html", &ctx)
                } else {
                    let mut ctx = Context::new();
                    ctx.insert("id", &id);
                    ctx.insert("title", &format!("Delete Torrent: {}", tc.torrent.name));
                    ctx.insert("message", "You are not allowed to delete this Torrent");
                    Template::render(&req.state().template(), "torrent/denied.html", &ctx)
                }
            }
            Err(e) => {
                info!("torrent '{}' not found: {}", id, e);
                Err(ErrorNotFound(e.to_string()))
            }
        })
        .responder()
}

#[derive(Deserialize)]
struct DeleteForm {
    id: Uuid,
    reason: String,
}

pub fn do_delete(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
        },
        Err(e) => return Box::new(FutErr(ErrorNotFound(e))),
    };

    let acl = req.state().acl_arc();
    let fut_prepare = req.clone().urlencoded::<DeleteForm>().from_err();

    let fut_process = fut_prepare.and_then(move |form| {
        let DeleteForm { id, reason } = form;
        let user = UserSubjectMsg::new(user_id, group_id, acl);
        let msg = DeleteTorrentMsg { id, reason, user };
        Ok(msg)
    });
    let cloned = req.clone();
    let fut_result = fut_process.and_then(move |torrent| {
        cloned
            .state()
            .db()
            .send(torrent)
            .map_err(|error| ErrorInternalServerError(error))
    });

    let cloned = req.clone();
    fut_result
        .from_err()
        .and_then(move |result| {
            let mut ctx = Context::new();
            ctx.insert("id", &id);

            match result {
                Ok(_) => {
                    ctx.insert("message", "Torrent was deleted");
                    ctx.insert("title", "Delete Torrent");
                    ctx.insert("sub_title", "Delete Succeeded");
                    ctx.insert("continue_link", "/torrents");
                    Template::render(&cloned.state().template(), "torrent/success.html", &ctx)
                }
                Err(e) => {
                    ctx.insert("error", &e.to_string());
                    ctx.insert("title", "Delete Torrent");
                    ctx.insert("sub_title", "Delete Failed");
                    ctx.insert(
                        "back_link",
                        &cloned
                            .url_for("torrent#read", &[id.to_string()])
                            .unwrap()
                            .to_string(),
                    );
                    Template::render(&cloned.state().template(), "torrent/failed.html", &ctx)
                }
            }
        })
        .responder()
}

pub fn download(mut req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => return Either::A(ErrorInternalServerError(format!("{}", e)).into()),
        },
        Err(e) => return Either::A(ErrorNotFound(e).into()),
    };
    let uid = req.session().get::<uuid::Uuid>("user_id").unwrap().unwrap();
    let fut_response = req.clone()
        .state()
        .db()
        .send(LoadTorrentMetaMsg { id, uid })
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
                    Err(ErrorNotFound(e.to_string()))
                }
            },
        );

    Either::B(fut_response.responder())
}

pub fn nfo(req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let id = match req.match_info().query::<String>("id") {
        Ok(id) => match Uuid::parse_str(&id[..]) {
            Ok(id) => id,
            Err(e) => return Either::A(ErrorInternalServerError(format!("{}", e)).into()),
        },
        Err(e) => return Either::A(ErrorNotFound(e).into()),
    };
    let fut_response = req.clone()
        .state()
        .db()
        .send(LoadTorrentNfoMsg { id })
        .from_err()
        .and_then(move |result: Result<(String, Vec<u8>)>| match result {
            Ok((name, nfo_file)) => Ok(HttpResponse::build(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/plain")
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", name),
                )
                .body(nfo_file)),
            Err(e) => {
                info!("nfo '{}' not found: {}", id, e);
                Err(ErrorNotFound(e.to_string()))
            }
        });

    Either::B(fut_response.responder())
}

fn categories(s: &State) -> Vec<models::Category> {
    if let Ok(categories) = s.db().send(LoadCategoriesMsg {}).wait() {
        categories.unwrap_or_else(|_| vec![])
    } else {
        vec![]
    }
}
