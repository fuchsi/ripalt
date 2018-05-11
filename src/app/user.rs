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
use handlers::user::{LoadSettingsMsg, LoadUserProfileMsg, UpdateProfileMsg, UpdateUserSettingsMsg};
use models::user::{UserProfileMsg, UserSettingsMsg};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

pub fn profile(req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let user_id = match req.session().get::<Uuid>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => return Either::A(redirect("/login")),
    };

    let cloned = req.clone();
    let fut = req.state()
        .db()
        .send(LoadUserProfileMsg(user_id, user_id, req.state().acl().clone()))
        .from_err()
        .and_then(move |result: Result<UserProfileMsg>| match result {
            Ok(user) => {
                let mut ctx = Context::new();
                ctx.insert("user", &user.user);
                ctx.insert("active_uploads", &user.active_uploads);
                ctx.insert("active_downloads", &user.active_downloads);
                ctx.insert("uploads", &user.uploads);
                ctx.insert("completed", &user.completed);
                ctx.insert("connections", &user.connections);
                ctx.insert("timezone", &user.timezone);
                ctx.insert("may_view_passcode", &user.may_view_passcode);
                Template::render_with_user(&cloned, "user/profile.html", &mut ctx)
            }
            Err(e) => {
                info!("user '{}' not found: {}", user_id, e);
                Err(ErrorNotFound(e.to_string()))
            }
        });

    Either::B(fut.responder())
}

pub fn view(req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let user_id = match req.match_info().get("id") {
        Some(user_id) => match Uuid::parse_str(user_id) {
            Ok(user_id) => user_id,
            Err(_) => return Either::A(HttpResponse::NotFound().finish()),
        },
        None => return Either::A(HttpResponse::NotFound().finish()),
    };
    let cur_user_id = match req.session().get::<Uuid>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => return Either::A(redirect("/login")),
    };

    let cloned = req.clone();
    let fut = req.state()
        .db()
        .send(LoadUserProfileMsg(user_id, cur_user_id, req.state().acl().clone()))
        .from_err()
        .and_then(move |result: Result<UserProfileMsg>| match result {
            Ok(user) => {
                let mut ctx = Context::new();
                ctx.insert("user", &user.user);
                ctx.insert("active_uploads", &user.active_uploads);
                ctx.insert("active_downloads", &user.active_downloads);
                ctx.insert("uploads", &user.uploads);
                ctx.insert("completed", &user.completed);
                ctx.insert("connections", &user.connections);
                ctx.insert("timezone", &user.timezone);
                ctx.insert("may_view_passcode", &user.may_view_passcode);
                Template::render_with_user(&cloned, "user/profile.html", &mut ctx)
            }
            Err(e) => {
                info!("user '{}' not found: {}", user_id, e);
                Err(ErrorNotFound(e.to_string()))
            }
        });

    Either::B(fut.responder())
}

pub fn settings(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let user_id = match req.user_id() {
        Some(user_id) => *user_id,
        None => return async_redirect("/login"),
    };

    let cloned = req.clone();
    let fut = req.state().db().send(LoadSettingsMsg(user_id)).from_err().and_then(
        move |result: Result<UserSettingsMsg>| match result {
            Ok(user) => {
                let mut ctx = Context::new();
                let timezones = timezones();
                let defaults = default_settings();
                ctx.insert("user", &user.user);
                ctx.insert("profile", &user.profile);
                ctx.insert("properties", &user.properties);
                ctx.insert("timezones", &timezones);
                ctx.insert("defaults", &defaults);
                ctx.insert("categories", &user.categories);
                Template::render_with_user(&cloned, "user/settings.html", &mut ctx)
            }
            Err(e) => {
                info!("user '{}' not found: {}", user_id, e);
                Err(ErrorNotFound(e.to_string()))
            }
        },
    );

    fut.responder()
}

pub fn update_settings(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let user_id = match req.user_id() {
        Some(user_id) => *user_id,
        None => return async_redirect("/login"),
    };

    let cloned = req.clone();
    let fut_prepare = req.clone()
        .body()
        .limit(8_388_608) // 8MB Limit
        .from_err()
        .and_then(move |body| -> Result<UpdateUserSettingsMsg> {
            let content_type = cloned.headers()[header::CONTENT_TYPE].to_str().unwrap();
            let mpr = MultipartRequest::new(content_type, body);
            let mut multipart = Multipart::from_request(mpr).unwrap();
            // Fetching all data and processing it.
            // save().temp() reads the request fully, parsing all fields and saving all files
            // in a new temporary directory under the OS temporary directory.
            match multipart.save().temp() {
                SaveResult::Full(entries) => {
                    let user = UpdateUserSettingsMsg::new(user_id);
                    process_update_settings(&entries, user)
                },
                SaveResult::Partial(_, reason) => {
                    Err(format!("partial read: {:?}", reason).into())
                },
                SaveResult::Error(error) => Err(format!("io error: {}", error).into()),
            }
        });

    let cloned = req.clone();
    let fut_process = fut_prepare
        .map_err(|e| {
            error!("{}", e);
            ErrorInternalServerError(e.to_string())
        })
        .and_then(move |user| {
            cloned.state().db().send(user).map_err(|error| {
                error!("{}", error);
                ErrorInternalServerError(error)
            })
        });

    fut_process
        .from_err()
        .and_then(move |result| match result {
            Ok(user) => {
                let mut ctx = Context::new();
                let timezones = timezones();
                let defaults = default_settings();
                ctx.insert("user", &user.user);
                ctx.insert("profile", &user.profile);
                ctx.insert("properties", &user.properties);
                ctx.insert("timezones", &timezones);
                ctx.insert("defaults", &defaults);
                ctx.insert("categories", &user.categories);
                Template::render_with_user(&req, "user/settings.html", &mut ctx)
            }
            Err(e) => {
                let mut ctx = Context::new();
                ctx.insert("error", &e.to_string());
                Template::render_with_user(&req, "user/settings_failed.html", &mut ctx)
            }
        })
        .responder()
}

fn process_update_settings(entries: &Entries, mut user: UpdateUserSettingsMsg) -> Result<UpdateUserSettingsMsg> {
    if let Some(fields) = entries.fields.get(&"timezone".to_string()) {
        if let Some(field) = fields.get(0) {
            if let SavedData::Text(ref data) = field.data {
                let timezone = data.parse::<i32>()?;
                user.push_create_property("timezone".to_string(), timezone);
            }
        }
    }

    if let Some(fields) = entries.fields.get(&"torrents_per_page".to_string()) {
        if let Some(field) = fields.get(0) {
            if let SavedData::Text(ref data) = field.data {
                let torrents_per_page = data.parse::<i64>()?;
                user.push_create_property("torrents_per_page".to_string(), torrents_per_page);
            }
        }
    }

    if let Some(fields) = entries.fields.get(&"accept_messages".to_string()) {
        if let Some(field) = fields.get(0) {
            if let SavedData::Text(ref data) = field.data {
                user.push_create_property("accept_messages".to_string(), data.clone());
            }
        }
    }

    if let Some(fields) = entries.fields.get(&"delete_message_on_reply".to_string()) {
        if let Some(field) = fields.get(0) {
            if let SavedData::Text(ref data) = field.data {
                let delete_message_on_reply = data == "true";
                user.push_create_property("delete_message_on_reply".to_string(), delete_message_on_reply);
            }
        }
    }

    if let Some(fields) = entries.fields.get(&"save_message_in_sent".to_string()) {
        if let Some(field) = fields.get(0) {
            if let SavedData::Text(ref data) = field.data {
                let save_message_in_sent = data == "true";
                user.push_create_property("save_message_in_sent".to_string(), save_message_in_sent);
            }
        }
    }

    if let Some(fields) = entries.fields.get(&"default_categories".to_string()) {
        let default_categories: Vec<Uuid> = fields
            .into_iter()
            .filter_map(|field| {
                if let SavedData::Text(ref data) = field.data {
                    Uuid::parse_str(data).ok()
                } else {
                    None
                }
            })
            .collect();
        user.push_create_property("default_categories".to_string(), default_categories);
    }

    process_update_profile(entries, user.profile_mut(), "profile_")?;

    Ok(user)
}

fn process_update_profile(entries: &Entries, user: &mut UpdateProfileMsg, prefix: &str) -> Result<()> {
    if let Some(fields) = entries.fields.get(&format!("{}about", prefix)) {
        if let Some(field) = fields.get(0) {
            if let SavedData::Text(ref data) = field.data {
                user.set_about(Some(data.clone()));
            }
        }
    }

    if let Some(fields) = entries.fields.get(&format!("{}flair", prefix)) {
        if let Some(field) = fields.get(0) {
            if let SavedData::Text(ref data) = field.data {
                user.set_flair(Some(data.clone()));
            }
        }
    }

    if let Some(fields) = entries.fields.get(&format!("{}avatar", prefix)) {
        if let Some(field) = fields.get(0) {
            if let Some(ref name) = field.headers.filename {
                if !name.is_empty() {
                    match field.headers.content_type {
                        Some(ref c) if c.type_() == "image" => {}
                        _ => bail!("invalid content type"),
                    }

                    let ts = Utc::now().timestamp();
                    // we only need the filename for the extension.
                    // the file will be stored as: timestamp.ext
                    let p = Path::new(name);
                    let ext = p.extension()
                        .and_then(|s| s.to_str())
                        .map_or("".to_string(), |s| s.to_ascii_lowercase());
                    let name = format!("{}.{}", ts, ext);
                    match field.data {
                        SavedData::Bytes(ref b) => {
                            let mut file = NamedTempFile::new()?;
                            file.write_all(b)?;
                            user.set_avatar2(name, file.into_temp_path());
                        }
                        SavedData::Text(ref s) => {
                            let b = s.as_bytes();
                            let mut file = NamedTempFile::new()?;
                            file.write_all(b)?;
                            user.set_avatar2(name, file.into_temp_path());
                        }
                        SavedData::File(ref path, _size) => {
                            let mut file = NamedTempFile::new()?;
                            // copy the whole uploaded file into a new tempfile. it's stupid, but works...
                            // todo: fix this nonsense
                            let mut source = fs::File::open(path)?;
                            io::copy(&mut source, &mut file)?;
                            user.set_avatar2(name, file.into_temp_path());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
