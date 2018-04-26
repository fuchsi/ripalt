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

use schema::chat_messages;
use schema::chat_messages::dsl as cm;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter, Write};

#[derive(PartialEq)]
pub enum ChatRoom {
    Public,
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
            &ChatRoom::Public => "public".to_string(),
            &ChatRoom::Team => "team".to_string(),
        }
    }
}

#[derive(Clone, Debug, Queryable, Identifiable, Insertable, Associations, Serialize)]
#[table_name = "chat_messages"]
#[belongs_to(User)]
pub struct ChatMessage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub chat: i16,
    pub message: String,
    pub created_at: Timestamp,
}

#[derive(Clone, Debug, Queryable, Identifiable, Associations, Serialize)]
#[table_name = "chat_messages"]
#[belongs_to(User)]
pub struct ChatMessageWithUser {
    pub id: Uuid,
    pub user_id: Uuid,
    pub chat: i16,
    pub message: String,
    pub created_at: Timestamp,
    pub user_name: String,
}

impl ChatMessageWithUser {
    pub fn messages_for_chat(chat: i16, since: Option<Timestamp>, limit: i64, db: &PgConnection) -> Vec<Self> {
        use schema::users;
        use schema::users::dsl as u;

        let mut query = chat_messages::table.into_boxed()
            .inner_join(users::table)
            .select((cm::id, cm::user_id, cm::chat, cm::message, cm::created_at, u::name))
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

#[derive(Insertable)]
#[table_name = "chat_messages"]
pub struct NewChatMessage<'a> {
    id: Uuid,
    user_id: &'a Uuid,
    chat: &'a i16,
    message: &'a str,
}

impl<'a> NewChatMessage<'a> {
    pub fn new(user_id: &'a Uuid, chat: &'a i16, message: &'a str) -> NewChatMessage<'a> {
        let id = Uuid::new_v4();
        NewChatMessage {
            id,
            user_id,
            chat,
            message,
        }
    }

    pub fn save(&self, db: &PgConnection) -> Result<ChatMessage> {
        self.insert_into(chat_messages::table)
            .get_result::<ChatMessage>(db)
            .map_err(|e| format!("failed to create chat message: {}", e).into())
    }
}
