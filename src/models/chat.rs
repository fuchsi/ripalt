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

//! Chat models
//!
//! [**ChatRoom**](enum.Chatroom.html) is used to identify the chatroom.
//!
//! [**ChatMessage**](struct.ChatMessage.html) is the data struct, which represents a row in the database.
//!
//! [**ChatMessageWithUser**](struct.ChatMessageWithUser.html) is the same as `ChatMessage`, but with the user name and user group added.
//!
//! [**NewChatMessage**](struct.NewChatMessage.html) can be used to create new messages.

use super::*;

use schema::chat_messages;
use schema::chat_messages::dsl as cm;
use std::convert::TryFrom;

/// The chatroom
///
/// Can be used with the [**UserSubject**](../../models/acl/struct.UserSubject.html) object to check the permissions of the user.
#[derive(PartialEq)]
pub enum ChatRoom {
    /// Public (all registered users) chatroom.
    /// When converted to `i16` it has the value `1`.
    Public,
    /// Team (moderator+) only chatroom.
    /// When converted to `i16` it has the value `2`.
    Team,
}

impl TryFrom<i16> for ChatRoom {
    type Error = Error;

    fn try_from(value: i16) -> Result<Self> {
        match value {
            1 => Ok(ChatRoom::Public),
            2 => Ok(ChatRoom::Team),
            _ => bail!("no chatroom with this id")
        }
    }
}

impl Into<i16> for ChatRoom {
    fn into(self) -> i16 {
        match self {
            ChatRoom::Public => 1,
            ChatRoom::Team => 2,
        }
    }
}

impl PartialEq<i16> for ChatRoom {
    fn eq(&self, other: &i16) -> bool {
        match ChatRoom::try_from(*other) {
            Ok(other) => other == *self,
            Err(_) => false,
        }
    }
}

impl ToString for ChatRoom {
    fn to_string(&self) -> String {
        match self {
            ChatRoom::Public => "public".to_string(),
            ChatRoom::Team => "team".to_string(),
        }
    }
}

/// Chat message data structure
///
/// Each instance represents a row in the database.
#[derive(Clone, Debug, Queryable, Identifiable, Insertable, Associations, Serialize)]
#[table_name = "chat_messages"]
#[belongs_to(User)]
pub struct ChatMessage {
    /// the unique message id
    pub id: Uuid,
    /// user id
    pub user_id: Uuid,
    /// chatroom. Can be converted to [**ChatRoom**](enum.Chatroom.html).
    pub chat: i16,
    /// the actual message
    pub message: String,
    /// timestamp when the message was created
    pub created_at: Timestamp,
}

/// Chat message data structure
///
/// is the same as `ChatMessage`, but with the user name and user group added.
///
/// Used whenever a chat message needs to be returned to the user.
#[derive(Clone, Debug, Queryable, Identifiable, Associations, Serialize)]
#[table_name = "chat_messages"]
#[belongs_to(User)]
pub struct ChatMessageWithUser {
    /// the unique message id
    pub id: Uuid,
    /// user id
    pub user_id: Uuid,
    /// chatroom. Can be converted to [**ChatRoom**](enum.Chatroom.html).
    pub chat: i16,
    /// the actual message
    pub message: String,
    /// timestamp when the message was created
    pub created_at: Timestamp,
    /// user name
    pub user_name: String,
    /// user group
    pub user_group: Uuid,
}

impl ChatMessageWithUser {
    /// Fetch all messages for a chatroom
    ///
    /// Filters by `chat`.
    ///
    /// If `since` is `Some` Timestamp, it only returns message newer than the timestamp.
    ///
    /// Returns at most `limit` messages
    pub fn messages_for_chat(chat: i16, since: Option<Timestamp>, limit: i64, db: &PgConnection) -> Vec<Self> {
        use schema::users;
        use schema::users::dsl as u;

        let mut query = chat_messages::table.into_boxed()
            .inner_join(users::table)
            .select((cm::id, cm::user_id, cm::chat, cm::message, cm::created_at, u::name, u::group_id))
            .filter(cm::chat.eq(chat));

        if let Some(since) = since {
            query = query.filter(cm::created_at.gt(since))
        }

        query.order_by(cm::created_at.desc())
            .limit(limit)
            .load::<Self>(db)
            .unwrap_or_default()
    }
}

impl From<ChatMessage> for ChatMessageWithUser {
    fn from(msg: ChatMessage) -> Self {
        ChatMessageWithUser{
            id: msg.id,
            user_id: msg.user_id,
            chat: msg.chat,
            message: msg.message,
            created_at: msg.created_at,
            user_name: String::default(),
            user_group: Uuid::default(),
        }
    }
}

/// A new chat message
#[derive(Insertable)]
#[table_name = "chat_messages"]
pub struct NewChatMessage<'a> {
    id: Uuid,
    user_id: &'a Uuid,
    chat: &'a i16,
    message: &'a str,
}

impl<'a> NewChatMessage<'a> {
    /// Constructs a new `NewChatMessage` instance.
    pub fn new(user_id: &'a Uuid, chat: &'a i16, message: &'a str) -> NewChatMessage<'a> {
        let id = Uuid::new_v4();
        NewChatMessage {
            id,
            user_id,
            chat,
            message,
        }
    }

    /// Save the message into the database.
    pub fn save(&self, db: &PgConnection) -> Result<ChatMessage> {
        self.insert_into(chat_messages::table)
            .get_result::<ChatMessage>(db)
            .map_err(|e| format!("failed to create chat message: {}", e).into())
    }
}
