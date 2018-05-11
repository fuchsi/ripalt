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

//! Message data models

use super::*;
use schema::{message_folders, messages};
use diesel::pg::Pg;

/// A message folder
#[derive(Identifiable, Queryable, Insertable, Associations, Serialize)]
#[belongs_to(User)]
pub struct MessageFolder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub purge: i16,
}

impl MessageFolder {
    /// Find a `MessageFolder` by its ID.
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        message_folders::table.find(id).first::<Self>(db).ok()
    }

    /// Find a `MessageFolder` by its name for a user.
    pub fn find_by_name(name: &str, user_id: &Uuid, db: &PgConnection) -> Option<Self> {
        message_folders::table
            .filter(message_folders::dsl::name.eq(name))
            .filter(message_folders::dsl::user_id.eq(user_id))
            .first::<Self>(db)
            .ok()
    }
}

/// A new message folder
#[derive(Identifiable, Insertable)]
#[table_name = "message_folders"]
pub struct NewMessageFolder<'a> {
    id: Uuid,
    user_id: &'a Uuid,
    name: &'a str,
    purge: i16,
}

impl<'a> NewMessageFolder<'a> {
    /// Construct a new `NewMessageFolder` instance.
    pub fn new(user_id: &'a Uuid, name: &'a str, purge: i16) -> Self {
        NewMessageFolder {
            id: Uuid::new_v4(),
            user_id,
            name,
            purge,
        }
    }

    /// Save the message folder into the database.
    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        self.insert_into(message_folders::table)
            .execute(db)
            .map_err(|e| format!("failed to insert message folder: {}", e).into())
    }
}

/// A message
#[derive(Identifiable, Queryable, Insertable, Associations, Serialize, Clone)]
#[belongs_to(MessageFolder, foreign_key = "folder_id")]
#[belongs_to(User, foreign_key = "receiver_id")]
pub struct Message {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub sender_id: Option<Uuid>,
    pub receiver_id: Uuid,
    pub subject: String,
    pub body: String,
    pub is_read: bool,
    pub created_at: Timestamp,
}

impl Message {
    /// Find a `Message` by its ID.
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        messages::table.find(id).first::<Self>(db).ok()
    }

    /// Fetch all messages in a folder.
    pub fn fetch_by_folder(folder_id: &Uuid, db: &PgConnection) -> Vec<Self> {
        Self::by_folder(folder_id)
            .load::<Self>(db)
            .unwrap_or_default()
    }

    /// Fetch all **unread** messages in a folder.
    pub fn fetch_unread(folder_id: &Uuid, db: &PgConnection) -> Vec<Self> {
        Self::by_folder(folder_id)
            .filter(messages::is_read.eq(false))
            .load::<Self>(db)
            .unwrap_or_default()
    }

    fn by_folder(folder_id: &Uuid) -> messages::BoxedQuery<Pg> {
        messages::table.into_boxed()
            .filter(messages::folder_id.eq(folder_id))
            .order_by(messages::created_at.desc())
    }

    /// Copy the message to another folder.
    ///
    /// # Returns
    ///
    /// The copied Message
    pub fn copy_to_folder(&self, sent_folder_id: &Uuid, db: &PgConnection) -> Result<Message> {
        let mut new_msg = self.clone();
        new_msg.id = Uuid::new_v4();
        new_msg.folder_id = *sent_folder_id;

        new_msg.insert_into(messages::table)
            .get_result::<Message>(db)
            .map_err(|e| format!("failed to copy message: {}", e).into())
    }

    /// Delete this message
    pub fn delete(&self, db: &PgConnection) -> Result<usize> {
        diesel::delete(messages::table)
            .filter(messages::id.eq(&self.id))
            .execute(db)
            .map_err(|e| format!("failed to delete message: {}", e).into())
    }

    /// Get the owning user
    pub fn owner(&self, folder: &MessageFolder) -> Uuid {
        match &folder.name[..] {
            "inbox" => self.receiver_id,
            "sent" => self.sender_id.unwrap(),
            "system" => self.receiver_id,
            _ => unreachable!(),
        }
    }

    /// Mark this message as read
    pub fn mark_as_read(&mut self, db: &PgConnection) -> Result<usize> {
        if self.is_read {
            return Ok(0);
        }

        let res = self.switch_mark(db)?;
        self.is_read = !self.is_read;
        Ok(res)
    }

    /// Mark this message as unread
    pub fn mark_as_unread(&mut self, db: &PgConnection) -> Result<usize> {
        if !self.is_read {
            return Ok(0);
        }

        let res = self.switch_mark(db)?;
        self.is_read = !self.is_read;
        Ok(res)
    }

    fn switch_mark(&self, db: &PgConnection) -> Result<usize> {
        diesel::update(self)
            .set(messages::is_read.eq(!self.is_read))
            .execute(db)
            .map_err(|e| format!("failed to update message: {}", e).into())
    }
}

/// A new message
#[derive(Identifiable, Insertable, Clone)]
#[table_name = "messages"]
pub struct NewMessage<'a> {
    id: Uuid,
    folder_id: &'a Uuid,
    sender_id: Option<&'a Uuid>,
    receiver_id: &'a Uuid,
    subject: &'a str,
    body: &'a str,
}

impl<'a> NewMessage<'a> {
    /// Construct a new `NewMessage` instance.
    pub fn new(
        folder_id: &'a Uuid,
        sender_id: &'a Uuid,
        receiver_id: &'a Uuid,
        subject: &'a str,
        body: &'a str,
    ) -> Self {
        NewMessage {
            id: Uuid::new_v4(),
            folder_id,
            sender_id: Some(sender_id),
            receiver_id,
            subject,
            body,
        }
    }

    /// Save the message into the database.
    pub fn save(&self, db: &PgConnection) -> Result<Message> {
        self.insert_into(messages::table)
            .get_result::<Message>(db)
            .map_err(|e| format!("failed to insert message: {}", e).into())
    }
}
