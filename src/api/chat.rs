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
use handlers::chat::{LoadChatMessagesMsg, PublishChatMessagesMsg};
use handlers::UserSubjectMsg;
use actix_web::Json;
use models::chat::ChatRoom;

pub fn messages(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    if req.credentials().is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let msg = match LoadChatMessagesMsg::try_from(&req) {
        Ok(msg) => msg,
        Err(e) => return Box::new(FutErr(ErrorBadRequest(e.to_string()))),
    };

    req.state().db().send(msg)
        .from_err()
        .and_then(|result: Result<Vec<models::chat::ChatMessageWithUser>>| {
            match result {
                Ok(messages) => Ok(HttpResponse::Ok().json(messages)),
                Err(e) => Err(ErrorForbidden(e.to_string())),
            }
        })
        .responder()
}

#[derive(Debug, Deserialize)]
pub struct PublishMessage {
    chat: i16,
    message: String,
}

pub fn publish(req: HttpRequest<State>, data: Json<PublishMessage>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, group_id) = credentials.take().unwrap();

    trace!("data: {:#?}", data);
    let PublishMessage { chat, message } = data.into_inner();
    let user = UserSubjectMsg::new(*user_id, *group_id, req.state().acl_arc());
    let chat = match ChatRoom::try_from(chat) {
        Ok(chat) => chat,
        Err(e) => return Box::new(FutErr(ErrorBadRequest(e.to_string()))),
    };
    let msg = PublishChatMessagesMsg::new(chat, message, user);

    req.state().db().send(msg)
        .from_err()
        .and_then(|result: Result<models::chat::ChatMessage>| {
            match result {
                Ok(message) => Ok(HttpResponse::Ok().json(message)),
                Err(e) => Err(ErrorForbidden(e.to_string())),
            }
        })
        .responder()
}