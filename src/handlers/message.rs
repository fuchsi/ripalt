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

use models::message::{MessageFolder, NewMessage};
use std::collections::HashMap;

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub sender_id: Option<Uuid>,
    pub receiver_id: Uuid,
    pub subject: String,
    pub body: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
    pub sender_name: String,
    pub receiver_name: String,
}

impl MessageResponse {
    pub fn set_user_names(&mut self, db: &PgConnection) {
        self.receiver_name = models::username(&self.receiver_id, db).unwrap();
        self.sender_name = match self.sender_id {
            Some(ref sender_id) => models::username(sender_id, db).unwrap_or_else(|| "System".to_string()),
            None => "System".to_string(),
        };
    }
}

impl From<models::Message> for MessageResponse {
    fn from(msg: models::Message) -> Self {
        let models::Message {
            id,
            folder_id,
            sender_id,
            receiver_id,
            subject,
            body,
            is_read,
            created_at,
        } = msg;
        MessageResponse {
            id,
            folder_id,
            sender_id,
            receiver_id,
            subject,
            body,
            is_read,
            created_at,
            sender_name: Default::default(),
            receiver_name: Default::default(),
        }
    }
}

pub struct LoadMessagesMsg {
    folder: String,
    only_unread: bool,
    user_id: Uuid,
}

impl LoadMessagesMsg {
    pub fn new(folder: String, only_unread: bool, user_id: Uuid) -> Self {
        LoadMessagesMsg {
            folder,
            only_unread,
            user_id,
        }
    }
}

impl Message for LoadMessagesMsg {
    type Result = Result<Vec<MessageResponse>>;
}

impl Handler<LoadMessagesMsg> for DbExecutor {
    type Result = Result<Vec<MessageResponse>>;

    fn handle(&mut self, msg: LoadMessagesMsg, _ctx: &mut Self::Context) -> <Self as Handler<LoadMessagesMsg>>::Result {
        let conn = self.conn();

        let folder = MessageFolder::find_by_name(&msg.folder, &msg.user_id, &conn);
        match folder {
            Some(folder) => {
                let messages = if msg.only_unread {
                    models::Message::fetch_unread(&folder.id, &conn)
                } else {
                    models::Message::fetch_by_folder(&folder.id, &conn)
                };

                let messages = messages
                    .into_iter()
                    .map(|m| {
                        let mut mr = MessageResponse::from(m);
                        mr.set_user_names(&conn);
                        mr
                    })
                    .collect();

                Ok(messages)
            }
            None => bail!("message folder not found"),
        }
    }
}

pub struct LoadMessageMsg {
    id: Uuid,
    user_id: Uuid,
}

impl LoadMessageMsg {
    pub fn new(id: Uuid, user_id: Uuid) -> Self {
        LoadMessageMsg { id, user_id }
    }
}

impl Message for LoadMessageMsg {
    type Result = Result<MessageResponse>;
}

impl Handler<LoadMessageMsg> for DbExecutor {
    type Result = Result<MessageResponse>;

    fn handle(&mut self, msg: LoadMessageMsg, _: &mut Self::Context) -> <Self as Handler<LoadMessageMsg>>::Result {
        let conn = self.conn();
        match models::Message::find(&msg.id, &conn) {
            Some(message) => match MessageFolder::find(&message.folder_id, &conn) {
                Some(folder) => {
                    if folder.user_id == msg.user_id {
                        let mut mr = MessageResponse::from(message);
                        mr.set_user_names(&conn);
                        Ok(mr)
                    } else {
                        bail!("not allowed")
                    }
                }
                None => bail!("message folder not found"),
            },
            None => bail!("message not found"),
        }
    }
}

pub struct NewMessageMsg {
    receiver: Uuid,
    subject: String,
    body: String,
    reply_to: Option<Uuid>,
    user_id: Uuid,
}

impl NewMessageMsg {
    pub fn new(msg: api::message::NewMessage, user_id: Uuid) -> Self {
        NewMessageMsg {
            user_id,
            ..NewMessageMsg::from(msg)
        }
    }
}

impl From<api::message::NewMessage> for NewMessageMsg {
    fn from(msg: api::message::NewMessage) -> Self {
        let api::message::NewMessage {
            receiver,
            subject,
            body,
            reply_to,
        } = msg;
        NewMessageMsg {
            receiver,
            subject,
            body,
            reply_to,
            user_id: Uuid::default(),
        }
    }
}

impl Message for NewMessageMsg {
    type Result = Result<MessageResponse>;
}

impl Handler<NewMessageMsg> for DbExecutor {
    type Result = Result<MessageResponse>;

    fn handle(&mut self, msg: NewMessageMsg, _: &mut Self::Context) -> <Self as Handler<NewMessageMsg>>::Result {
        let conn = self.conn();
        let sender = match models::User::find(&msg.user_id, &conn) {
            Some(sender) => sender,
            None => bail!("sender not found"),
        };
        let receiver = match models::User::find(&msg.receiver, &conn) {
            Some(receiver) => receiver,
            None => bail!("receiver not found"),
        };
        let rec_inbox = match MessageFolder::find_by_name("inbox", &receiver.id, &conn) {
            Some(folder) => folder,
            None => bail!("inbox not found"),
        };
        let message = NewMessage::new(&rec_inbox.id, &sender.id, &receiver.id, &msg.subject, &msg.body);
        let message = message.save(&conn)?;
        let settings = SETTINGS.read().unwrap();
        if settings.user.default_save_message_in_sent {
            if let Some(snd_sent) = MessageFolder::find_by_name("sent", &sender.id, &conn) {
                message.copy_to_folder(&snd_sent.id, &conn)?;
            }
        }
        if let Some(ref reply_to) = msg.reply_to {
            if settings.user.default_delete_message_on_reply {
                if let Some(original_message) = models::Message::find(reply_to, &conn) {
                    original_message.delete(&conn)?;
                }
            }
        }

        let mut mr = MessageResponse::from(message);
        mr.sender_name = sender.name;
        mr.receiver_name = receiver.name;

        Ok(mr)
    }
}

pub struct DeleteMessagesMsg {
    messages: Vec<Uuid>,
    user_id: Uuid,
}

impl DeleteMessagesMsg {
    pub fn new(messages: Vec<Uuid>, user_id: Uuid) -> Self {
        DeleteMessagesMsg { messages, user_id }
    }
}

impl Message for DeleteMessagesMsg {
    type Result = Result<Vec<Uuid>>;
}

impl Handler<DeleteMessagesMsg> for DbExecutor {
    type Result = Result<Vec<Uuid>>;

    fn handle(
        &mut self,
        msg: DeleteMessagesMsg,
        _: &mut Self::Context,
    ) -> <Self as Handler<DeleteMessagesMsg>>::Result {
        let conn = self.conn();

        let mut deleted = Vec::with_capacity(msg.messages.len());
        let mut folders = HashMap::new();

        for ref mid in msg.messages {
            if let Some(message) = models::Message::find(mid, &conn) {
                if !folders.contains_key(&message.folder_id) {
                    let from_db = match MessageFolder::find(&message.folder_id, &conn) {
                        Some(f) => f,
                        None => continue,
                    };
                    folders.insert(message.folder_id, from_db);
                };
                let folder = folders.get(&message.folder_id).unwrap();

                if message.owner(folder) == msg.user_id {
                    message.delete(&conn)?;
                    deleted.push(message.id);
                }
            }
        }

        Ok(deleted)
    }
}

pub struct MarkMessagesMsg {
    messages: Vec<Uuid>,
    user_id: Uuid,
    mark_as_read: bool,
}

impl MarkMessagesMsg {
    pub fn new(messages: Vec<Uuid>, user_id: Uuid, mark_as_read: bool) -> Self {
        MarkMessagesMsg {
            messages,
            user_id,
            mark_as_read,
        }
    }
}

impl Message for MarkMessagesMsg {
    type Result = Result<Vec<Uuid>>;
}

impl Handler<MarkMessagesMsg> for DbExecutor {
    type Result = Result<Vec<Uuid>>;

    fn handle(&mut self, msg: MarkMessagesMsg, _: &mut Self::Context) -> <Self as Handler<MarkMessagesMsg>>::Result {
        let conn = self.conn();

        let mut marked = Vec::with_capacity(msg.messages.len());
        let mut folders = HashMap::new();

        for ref mid in msg.messages {
            if let Some(mut message) = models::Message::find(mid, &conn) {
                if !folders.contains_key(&message.folder_id) {
                    let from_db = match MessageFolder::find(&message.folder_id, &conn) {
                        Some(f) => f,
                        None => continue,
                    };
                    folders.insert(message.folder_id, from_db);
                };
                let folder = folders.get(&message.folder_id).unwrap();

                if message.owner(folder) == msg.user_id {
                    if msg.mark_as_read {
                        message.mark_as_read(&conn)?;
                    } else {
                        message.mark_as_unread(&conn)?;
                    }
                    marked.push(message.id);
                }
            }
        }

        Ok(marked)
    }
}
