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

//! Chat API
//!
//! [**ChatRoom**](../../models/chat/enum.ChatRoom.html) is used to identify the chatroom.
//!
//! [**ChatMessageWithUser**](../../models/chat/struct.ChatMessageWithUser.html) is used whenever a message should be returned.

use super::*;

use actix_web::AsyncResponder;
use actix_web::Json;
use api::identity::RequestIdentity;
use handlers::chat::{LoadChatMessagesMsg, PublishChatMessagesMsg};
use handlers::UserSubjectMsg;
use models::chat::{ChatMessageWithUser, ChatRoom};
use std::convert::TryFrom;

/// Fetch chat messages
///
/// `GET /api/v1/chat/messages`
///
/// # Parameters
///
/// | Parameter | Type  | Description |
/// |-----------|-------|-------------|
/// | `chat`    | `i16` | Which chatroom to use. 1: [**Public**](../../models/chat/enum.ChatRoom.html#variant.Public), 2: [**Team**](../../models/chat/enum.ChatRoom.html#variant.Team) |
/// | `since`   | `i64` | Unix Timestamp. Fetch only messages newer than this Timestamp. |
/// | `limit`   | `i64` | Limit response to the latest `limit` messages. |
///
/// # Returns
///
/// If successful, `messages` returns a list of [**Messages**](../../models/chat/struct.ChatMessageWithUser.html).
///
/// Each message consists of:
///
/// | Field        | Type            | Description |
/// |--------------|-----------------|-------------|
/// | `id`         | `Uuid`          | Unique Message ID |
/// | `user_id`    | `Uuid`          | User ID |
/// | `chat`       | `i16`           | Chatroom |
/// | `message`    | `String`        | The actual message content |
/// | `created_at` | `Datetime<Utc>` | Timestamp when the message was created |
/// | `user_name`  | `String`        | Name of the User |
/// | `user_group` | `Uuid`          | User Group ID |
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest` if the request parameters are invalid.
/// - `ErrorForbidden` if the client is not allowed to read the given chatroom.
pub fn messages(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    if req.credentials().is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let msg = match LoadChatMessagesMsg::try_from(&req) {
        Ok(msg) => msg,
        Err(e) => return Box::new(FutErr(ErrorBadRequest(e.to_string()))),
    };

    req.state()
        .db()
        .send(msg)
        .from_err()
        .and_then(|result: Result<Vec<ChatMessageWithUser>>| match result {
            Ok(messages) => Ok(HttpResponse::Ok().json(messages)),
            Err(e) => Err(ErrorForbidden(e.to_string())),
        })
        .responder()
}

/// Publish Message Payload
#[derive(Deserialize)]
pub struct PublishMessage {
    /// In which chatroom should the message be posted
    pub chat: i16,
    /// The actual chat message
    pub message: String,
}

/// Publish a new chat message
///
/// `POST /api/v1/chat/publish`
///
/// # Payload
///
/// [**PublishMessage**](struct.PublishMessage.html) as JSON.
///
/// # Returns
///
/// If successful, publish returns the [posted message](../../models/chat/struct.ChatMessageWithUser.html).
///
/// | Field        | Type            | Description |
/// |--------------|-----------------|-------------|
/// | `id`         | `Uuid`          | Unique Message ID |
/// | `user_id`    | `Uuid`          | User ID |
/// | `chat`       | `i16`           | Chatroom |
/// | `message`    | `String`        | The actual message content |
/// | `created_at` | `Datetime<Utc>` | Timestamp when the message was created |
/// | `user_name`  | `String`        | Name of the User |
/// | `user_group` | `Uuid`          | User Group ID |
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest` if the request payload is invalid.
/// - `ErrorForbidden` if the client is not allowed to write to the given chatroom.
pub fn publish(req: HttpRequest<State>, data: Json<PublishMessage>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, group_id) = credentials.take().unwrap();

    let PublishMessage { chat, message } = data.into_inner();
    let user = UserSubjectMsg::new(*user_id, *group_id, req.state().acl_arc());
    let chat = match ChatRoom::try_from(chat) {
        Ok(chat) => chat,
        Err(e) => return Box::new(FutErr(ErrorBadRequest(e.to_string()))),
    };
    let msg = PublishChatMessagesMsg::new(chat, message, user);

    req.state()
        .db()
        .send(msg)
        .from_err()
        .and_then(move |result: Result<ChatMessageWithUser>| match result {
            Ok(message) => Ok(HttpResponse::Ok().json(message)),
            Err(e) => Err(ErrorForbidden(e.to_string())),
        })
        .responder()
}
