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

//! Chat Handlers

use super::*;

use identity::RequestIdentity;
use models::chat::{ChatMessageWithUser, ChatRoom, NewChatMessage};
use models::acl::Subject;

/// Loads chat messages from the database backend
pub struct LoadChatMessagesMsg {
    chat: ChatRoom,
    since: Option<DateTime<Utc>>,
    limit: i64,
    user: UserSubjectMsg,
}

impl LoadChatMessagesMsg {
    /// Construct a new `LoadChatMessagesMsg` instance
    pub fn new(chat: ChatRoom, since: Option<DateTime<Utc>>, limit: i64, user: UserSubjectMsg) -> Self {
        LoadChatMessagesMsg{chat, since, limit, user}
    }
}

impl<'req> TryFrom<&'req HttpRequest<State>> for LoadChatMessagesMsg {
    type Error = Error;

    fn try_from(req: &HttpRequest<State>) -> Result<Self> {
        let chat: i16 = req.query().get("chat").unwrap_or_else(|| "1").parse()?;
        let chat = ChatRoom::try_from(chat)?;
        let since = match req.query().get("since") {
            Some(s) => {
                let ts: i64 = s.parse()?;
                Some(Utc.timestamp(ts, 0))
            },
            None => None,
        };

        let limit: i64 = req.query().get("limit").unwrap_or_else(|| "50").parse()?;
        let mut identity = req.credentials();
        let (user_id, group_id) = identity.take().unwrap();
        let user = UserSubjectMsg::new(*user_id, *group_id, req.state().acl_arc());

        Ok(LoadChatMessagesMsg::new(chat, since, limit, user))
    }
}

impl Message for LoadChatMessagesMsg {
    type Result = Result<Vec<ChatMessageWithUser>>;
}

impl Handler<LoadChatMessagesMsg> for DbExecutor {
    type Result = Result<Vec<ChatMessageWithUser>>;

    fn handle(&mut self, msg: LoadChatMessagesMsg, _: &mut Self::Context) -> <Self as Handler<LoadChatMessagesMsg>>::Result {
        let conn = self.conn();
        let subj = UserSubject::from(&msg.user);
        if !subj.may_read(&msg.chat) {
            bail!("not allowed");
        }

        Ok(ChatMessageWithUser::messages_for_chat(msg.chat.into(), msg.since, msg.limit, &conn))
    }
}

/// Publishes a new chat message.
pub struct PublishChatMessagesMsg {
    chat: ChatRoom,
    message: String,
    user: UserSubjectMsg,
}

impl PublishChatMessagesMsg {
    /// Construct a new `PublishChatMessagesMsg` instance
    pub fn new<T: Into<ChatRoom>>(chat: T, message: String, user: UserSubjectMsg) -> Self {
        Self{
            chat: chat.into(),
            message,
            user,
        }
    }
}

impl Message for PublishChatMessagesMsg {
    type Result = Result<ChatMessageWithUser>;
}

impl Handler<PublishChatMessagesMsg> for DbExecutor {
    type Result = Result<ChatMessageWithUser>;

    fn handle(&mut self, msg: PublishChatMessagesMsg, _: &mut Self::Context) -> <Self as Handler<PublishChatMessagesMsg>>::Result {
        let conn = self.conn();
        let subj = UserSubject::from(&msg.user);
        if !subj.may_write(&msg.chat) {
            bail!("not allowed");
        }

        let chat: i16 = msg.chat.into();
        let new_message = NewChatMessage::new(&msg.user.uid, &chat, &msg.message);
        let mut message = ChatMessageWithUser::from(new_message.save(&conn)?);
        message.user_name = models::username(&message.user_id, &conn).unwrap();
        message.user_group = *subj.group_id();
        Ok(message)
    }
}
