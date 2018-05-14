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

//! Comment API
//!
//! [**TorrentCommentResponse**](../../handlers/torrent/struct.TorrentCommentResponse.html) is used whenever a comment should be returned

use super::*;
use actix_web::AsyncResponder;
use actix_web::FromRequest;
use actix_web::Json;
use handlers::torrent::{LoadCommentMsg, LoadCommentsMsg, EditCommentMsg, DeleteCommentMsg, NewCommentMsg};
use handlers::UserSubjectMsg;
use std::convert::TryFrom;

/// New comment payload
#[derive(Deserialize)]
pub struct NewComment {
    torrent_id: Uuid,
    content: String,
}

/// Edit comment payload
#[derive(Deserialize)]
pub struct EditComment {
    id: Uuid,
    content: String,
}

/// Delete comment payload
#[derive(Deserialize)]
pub struct DeleteComment {
    id: Uuid,
}

/// Fetch comments
///
/// `GET /api/v1/comment/torrent`
///
/// # Parameters
///
/// | Parameter    | Type   | Description |
/// |--------------|--------|-------------|
/// | `torrent_id` | `Uuid` | Torrent ID |
///
/// # Returns
///
/// If successful, `torrent` returns a list of [**Comments**](../../handlers/torrent/struct.TorrentCommentResponse.html)
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest`
///     - if the request parameters are invalid.
///     - if the torrent does not exist.
pub fn torrent(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let mut query = match Query::<HashMap<String, String>>::extract(&req) {
        Ok(q) => q,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
    };

    let torrent_id = match query.remove("torrent_id") {
        Some(mut torrent_id) => match Uuid::parse_str(&torrent_id) {
            Ok(torrent_id) => torrent_id,
            Err(e) => return Box::new(FutOk(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() }))),
        },
        None => return Box::new(FutOk(HttpResponse::BadRequest().json(JsonErr { error: "no torrent id".to_string() }))),
    };

    let subj = match UserSubjectMsg::try_from(&req) {
        Ok(subj) => subj,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e.to_string())))
    };
    let msg = LoadCommentsMsg::new(torrent_id, subj);

    req.state().db().send(msg)
        .from_err()
        .and_then(|result| {
            match result {
                Ok(comments) => Ok(HttpResponse::Ok().json(comments)),
                Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
            }
        })
        .responder()
}

/// Fetch a single comment
///
/// `GET /api/v1/comment/get`
///
/// # Parameters
///
/// | Parameter | Type   | Description |
/// |-----------|--------|-------------|
/// | `id`      | `Uuid` | Comment ID  |
///
/// # Returns
///
/// If successful, `torrent` returns a [**Comment**](../../handlers/torrent/struct.TorrentCommentResponse.html)
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest` if the request parameters are invalid.
pub fn comment(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let mut query = match Query::<HashMap<String, String>>::extract(&req) {
        Ok(q) => q,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
    };

    let id = match query.remove("id") {
        Some(mut id) => match Uuid::parse_str(&id) {
            Ok(id) => id,
            Err(e) => return Box::new(FutOk(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() }))),
        },
        None => return Box::new(FutOk(HttpResponse::BadRequest().json(JsonErr { error: "no id".to_string() }))),
    };

    let subj = match UserSubjectMsg::try_from(&req) {
        Ok(subj) => subj,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e.to_string())))
    };
    let msg = LoadCommentMsg::new(id, subj);

    req.state().db().send(msg)
        .from_err()
        .and_then(|result| {
            match result {
                Ok(comment) => Ok(HttpResponse::Ok().json(comment)),
                Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
            }
        })
        .responder()
}

/// Publish a new comment
///
/// `POST /api/v1/comment/new`
///
/// # Payload
///
/// [**NewComment**](struct.NewComment.html) as JSON.
///
/// # Returns
///
/// If successful, `new` returns the published [**Comment**](../../handlers/torrent/struct.TorrentCommentResponse.html).
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest`
///     - if the request parameters are invalid.
///     - if the torrent does not exist.
///     - if any error occurs when storing the comment.
pub fn new(req: HttpRequest<State>, data: Json<NewComment>) -> FutureResponse<HttpResponse> {
    let subj = match UserSubjectMsg::try_from(&req) {
        Ok(subj) => subj,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e.to_string())))
    };
    let NewComment { torrent_id, content } = data.into_inner();
    let msg = NewCommentMsg::new(torrent_id, content, subj);

    req.state().db().send(msg)
        .from_err()
        .and_then(|result| {
            match result {
                Ok(comment) => Ok(HttpResponse::Ok().json(comment)),
                Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
            }
        })
        .responder()
}

/// Edit a comment
///
/// `POST /api/v1/comment/edit`
///
/// # Payload
///
/// [**EditComment**](struct.EditComment.html) as JSON.
///
/// # Returns
///
/// If successful, `edit` returns the edited [**Comment**](../../handlers/torrent/struct.TorrentCommentResponse.html).
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest`
///     - if the request parameters are invalid.
///     - if the torrent does not exist.
///     - if any error occurs when storing the comment.
pub fn edit(req: HttpRequest<State>, data: Json<EditComment>) -> FutureResponse<HttpResponse> {
    let subj = match UserSubjectMsg::try_from(&req) {
        Ok(subj) => subj,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e.to_string())))
    };
    let EditComment { id, content } = data.into_inner();
    let msg = EditCommentMsg::new(id, content, subj);

    req.state().db().send(msg)
        .from_err()
        .and_then(|result| {
            match result {
                Ok(comment) => Ok(HttpResponse::Ok().json(comment)),
                Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
            }
        })
        .responder()
}

/// Delete a comment
///
/// `POST /api/v1/comment/delete`
///
/// # Payload
///
/// [**DeleteComment**](struct.DeleteComment.html) as JSON.
///
/// # Returns
///
/// If successful, `new` returns the id of the deleted Comment
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest`
///     - if the request parameters are invalid.
///     - if the torrent does not exist.
///     - if any error occurs when deleting the comment.
pub fn delete(req: HttpRequest<State>, data: Json<DeleteComment>) -> FutureResponse<HttpResponse> {
    let subj = match UserSubjectMsg::try_from(&req) {
        Ok(subj) => subj,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e.to_string())))
    };
    let id = data.id;
    let msg = DeleteCommentMsg::new(id, subj);

    req.state().db().send(msg)
        .from_err()
        .and_then(move |result| {
            match result {
                Ok(deleted) => {
                    let mut list = Vec::new();
                    if deleted > 0 {
                        list.push(id);
                    }

                    Ok(HttpResponse::Ok().json(list))
                },
                Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
            }
        })
        .responder()
}