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
use actix_web::Json;
use api::identity::RequestIdentity;
use handlers::message::{DeleteMessagesMsg, LoadMessageMsg, LoadMessagesMsg, MarkMessagesMsg, NewMessageMsg};

#[derive(Deserialize)]
pub struct NewMessage {
    pub receiver: String,
    pub subject: String,
    pub body: String,
    pub reply_to: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct MessageListMsg {
    pub messages: Vec<Uuid>,
}

#[derive(Serialize)]
struct JsonErr {
    pub error: String,
}

pub fn messages(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let folder = req.query().get("folder").unwrap_or_else(|| "inbox");
    let unread = req.query().get("unread").unwrap_or_else(|| "0") == "1";
    let loadmsg = LoadMessagesMsg::new(folder.to_string(), unread, *user_id);

    req.state()
        .db()
        .send(loadmsg)
        .from_err()
        .and_then(|result| match result {
            Ok(messages) => Ok(HttpResponse::Ok().json(messages)),
            Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
        })
        .responder()
}

pub fn unread(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let folder = req.query().get("folder").unwrap_or_else(|| "inbox");
    let loadmsg = LoadMessagesMsg::new(folder.to_string(), true, *user_id);

    req.state()
        .db()
        .send(loadmsg)
        .from_err()
        .and_then(|result| match result {
            Ok(messages) => Ok(HttpResponse::Ok().json(messages)),
            Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
        })
        .responder()
}

pub fn message(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let mut id = req.query().get("id");
    if id.is_none() {
        return Box::new(FutErr(ErrorBadRequest("no message id")));
    }
    let id = Uuid::parse_str(id.take().unwrap());
    if id.is_err() {
        return Box::new(FutErr(ErrorBadRequest("invalid message id")));
    }
    let id = id.unwrap();
    let loadmsg = LoadMessageMsg::new(id, *user_id);

    req.state()
        .db()
        .send(loadmsg)
        .from_err()
        .and_then(|result| match result {
            Ok(message) => Ok(HttpResponse::Ok().json(message)),
            Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
        })
        .responder()
}

pub fn send(req: HttpRequest<State>, data: Json<NewMessage>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let data = data.into_inner();
    let sendmsg = NewMessageMsg::new(data, *user_id);

    req.state()
        .db()
        .send(sendmsg)
        .from_err()
        .and_then(|result| match result {
            Ok(message) => Ok(HttpResponse::Ok().json(message)),
            Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
        })
        .responder()
}

pub fn delete(req: HttpRequest<State>, data: Json<MessageListMsg>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let data = data.into_inner();
    let delmsg = DeleteMessagesMsg::new(data.messages, *user_id);

    req.state()
        .db()
        .send(delmsg)
        .from_err()
        .and_then(|result| match result {
            Ok(messages) => Ok(HttpResponse::Ok().json(messages)),
            Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
        })
        .responder()
}

pub fn mark_read(req: HttpRequest<State>, data: Json<MessageListMsg>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let data = data.into_inner();
    let markmsg = MarkMessagesMsg::new(data.messages, *user_id, true);

    req.state()
        .db()
        .send(markmsg)
        .from_err()
        .and_then(|result| match result {
            Ok(messages) => Ok(HttpResponse::Ok().json(messages)),
            Err(e) => Ok(HttpResponse::BadRequest().json(JsonErr { error: e.to_string() })),
        })
        .responder()
}
