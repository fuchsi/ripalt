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
use handlers::message::{LoadMessageMsg, LoadMessagesMsg};

#[derive(Deserialize)]
pub struct NewMessage {
    pub receiver: Uuid,
    pub subject: String,
    pub body: String,
    pub reply_to: Option<Uuid>,
}

pub fn messages(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let user_id = match session_creds(&mut req) {
        Some((u, _)) => u,
        None => return async_redirect("/login"),
    };

    let folder = req.match_info().get("folder").unwrap_or_else(|| "inbox").to_string();
    let loadmsg = LoadMessagesMsg::new(folder.clone(), false, user_id);

    req.clone().state()
        .db()
        .send(loadmsg)
        .from_err()
        .and_then(move |result| match result {
            Ok(messages) => {
                let mut ctx = Context::new();
                ctx.insert("messages", &messages);
                ctx.insert("folder", &folder);
                Template::render(&req.state().template(), "message/list.html", &ctx)
            },
            Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
        })
        .responder()
}

pub fn message(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let user_id = match session_creds(&mut req) {
        Some((u, _)) => u,
        None => return async_redirect("/login"),
    };

    let id = {
        let mut id = req.match_info().get("id");
        if id.is_none() {
            return Box::new(FutErr(ErrorNotFound("no message id")));
        }
        let id = Uuid::parse_str(id.take().unwrap());
        if id.is_err() {
            return Box::new(FutErr(ErrorNotFound("invalid message id")));
        }
        id.unwrap()
    };
    let mut loadmsg = LoadMessageMsg::new(id, user_id);
    loadmsg.mark_as_read(true);

    req.clone().state()
        .db()
        .send(loadmsg)
        .from_err()
        .and_then(move |result| match result {
            Ok(message) => {
                let mut ctx = Context::new();
                ctx.insert("message", &message);
                Template::render(&req.state().template(), "message/show.html", &ctx)
            },
            Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
        })
        .responder()
}

pub fn new(req: HttpRequest<State>) -> SyncResponse<HttpResponse> {
    let mut ctx = Context::new();
    if let Some(receiver) = req.query().get("receiver") {
        ctx.insert("receiver", &receiver);
    }
    Template::render(&req.state().template(), "message/new.html", &ctx)
}

pub fn reply(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let user_id = match session_creds(&mut req) {
        Some((u, _)) => u,
        None => return async_redirect("/login"),
    };

    let id = {
        let mut id = req.match_info().get("id");
        if id.is_none() {
            return Box::new(FutErr(ErrorNotFound("no message id")));
        }
        let id = Uuid::parse_str(id.take().unwrap());
        if id.is_err() {
            return Box::new(FutErr(ErrorNotFound("invalid message id")));
        }
        id.unwrap()
    };
    let mut loadmsg = LoadMessageMsg::new(id, user_id);
    loadmsg.mark_as_read(true);

    req.clone().state()
        .db()
        .send(loadmsg)
        .from_err()
        .and_then(move |result| match result {
            Ok(message) => {
                let mut ctx = Context::new();
                ctx.insert("message", &message);
                Template::render(&req.state().template(), "message/new.html", &ctx)
            },
            Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
        })
        .responder()
}
