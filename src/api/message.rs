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

//! Message API
//!
//! [**MessageResponse**](../../handlers/message/struct.MessageResponse.html) is used whenever a message should be returned

use super::*;

use actix_web::AsyncResponder;
use actix_web::FromRequest;
use actix_web::Json;
use identity::RequestIdentity;
use handlers::message::{DeleteMessagesMsg, LoadMessageMsg, LoadMessagesMsg, MarkMessagesMsg, NewMessageMsg};

/// New message payload
#[derive(Deserialize)]
pub struct NewMessage {
    /// Receivers user name
    pub receiver: String,
    pub subject: String,
    pub body: String,
    /// Some Message ID if it's a reply
    pub reply_to: Option<Uuid>,
}

/// Message list payload
#[derive(Deserialize)]
pub struct MessageListMsg {
    /// A list of Message IDs
    pub messages: Vec<Uuid>,
}

#[derive(Serialize)]
struct JsonErr {
    pub error: String,
}

/// Fetch messages
///
/// `GET /api/v1/message/messages`
///
/// # Parameters
///
/// | Parameter | Type     | Description |
/// |-----------|----------|-------------|
/// | `folder`  | `String` | Message folder. `inbox`, `sent` or `system` |
/// | `unread`  | `String` | Fetch only unread messages. `0` or `1` |
///
/// # Returns
///
/// If successful, `messages` returns a list of [**Messages**](../../handlers/message/struct.MessageResponse.html)
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest` if the request parameters are invalid.
///     Or if the message folder does not exist.
pub fn messages(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let mut query = match Query::<HashMap<String, String>>::extract(&req) {
        Ok(q) => q,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
    };

    let folder = query.remove("folder").unwrap_or_else(|| "inbox".to_string() );
    let unread = query.remove("unread").unwrap_or_else(|| "0".to_string()) == "1";
    let loadmsg = LoadMessagesMsg::new(folder, unread, *user_id);

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

/// Fetch only unread messages.
///
/// Basically the same as [**messages**](fn.messages.html) with `unread` set to `1`.
///
/// `GET /api/v1/message/unread`
///
/// # Parameters
///
/// | Parameter | Type     | Description |
/// |-----------|----------|-------------|
/// | `folder`  | `String` | Message folder. `inbox`, `sent` or `system` |
///
/// # Returns
///
/// If successful, `messages` returns a list of [**Messages**](../../handlers/message/struct.MessageResponse.html)
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest` if the request parameters are invalid.
///     Or if the message folder does not exist.
pub fn unread(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let mut query = match Query::<HashMap<String, String>>::extract(&req) {
        Ok(q) => q,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
    };

    let folder = query.remove("folder").unwrap_or_else(|| "inbox".to_string());
    let loadmsg = LoadMessagesMsg::new(folder, true, *user_id);

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

/// Fetch a single message
///
/// `GET /api/v1/message/message`
///
/// # Parameters
///
/// | Parameter | Type     | Description |
/// |-----------|----------|-------------|
/// | `id`      | `Uuid`   | Message ID  |
///
/// # Returns
///
/// If successful, `message` returns the [**Message**](../../handlers/message/struct.MessageResponse.html).
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest`
///     - if the request parameters are invalid.
///     - if the message does not exist.
///     - if the message folder does not exist.
///     - if the user is not allowed to read the message.
pub fn message(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let mut credentials = req.credentials();
    if credentials.is_none() {
        return Box::new(FutErr(ErrorUnauthorized("unauthorized")));
    }
    let (user_id, _) = credentials.take().unwrap();

    let query = match Query::<HashMap<String, String>>::extract(&req) {
        Ok(q) => q,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(e))),
    };

    let mut id = query.get("id");
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

/// Send a new message to a user
///
/// `POST /api/v1/message/send`
///
/// # Payload
///
/// [**NewMessage**](struct.NewMessage.html) as JSON.
///
/// # Returns
///
/// If successful, `send` returns the sent [**Message**](../../handlers/message/struct.MessageResponse.html).
///
/// The returned message is either an copy of the message in the `sent` folder or the original sent message,
/// depending on the current users settings.
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest`
///     - if the request parameters are invalid.
///     - if the sender does not exist.
///     - if the receiver does not exist.
///     - if the receiver has no inbox folder. *(should never happen)*
///     - if any error occurs when storing the message.
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

/// Delete one or more messages
///
/// `POST /api/v1/message/delete`
///
/// # Payload
///
/// [**MessageListMsg**](struct.MessageListMsg.html) as JSON.
///
/// # Returns
///
/// A list of Message IDs, which were deleted.
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest`
///     - if the request parameters are invalid.
///     - if any error occurs when storing the message.
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

/// Mark one or more messages as read
///
/// `POST /api/v1/message/mark_read`
///
/// # Payload
///
/// [**MessageListMsg**](struct.MessageListMsg.html) as JSON.
///
/// # Returns
///
/// A list of Message IDs, which were marked as read.
///
/// # Errors
///
/// - `ErrorUnauthorized` if the client is not authorized.
/// - `ErrorBadRequest`
///     - if the request parameters are invalid.
///     - if any error occurs when storing the message.
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
