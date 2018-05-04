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
use models::static_content::Content;
use models::acl::Subject;

pub struct LoadStaticContentMsg {
    id: String,
}

impl LoadStaticContentMsg {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

impl Message for LoadStaticContentMsg {
    type Result = Result<Content>;
}

impl Handler<LoadStaticContentMsg> for DbExecutor {
    type Result = Result<Content>;

    fn handle(
        &mut self,
        msg: LoadStaticContentMsg,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<LoadStaticContentMsg>>::Result {
        let conn = self.conn();

        Content::find(&msg.id, &conn).ok_or_else(|| "failed to load content".into())
    }
}

#[derive(Default)]
pub struct UpdateStaticContentMsg {
    id: String,
    title: String,
    content: String,
    content_type: String,
    subj: Option<UserSubjectMsg>,
}

impl UpdateStaticContentMsg {
    pub fn new(id: String, title: String, content: String, content_type: String) -> Self {
        Self {
            id,
            title,
            content,
            content_type,
            .. Default::default()
        }
    }

    pub fn set_acl(&mut self, subj: UserSubjectMsg) {
        self.subj = Some(subj);
    }
}

impl Message for UpdateStaticContentMsg {
    type Result = Result<Content>;
}

impl Handler<UpdateStaticContentMsg> for DbExecutor {
    type Result = Result<Content>;

    fn handle(
        &mut self,
        mut msg: UpdateStaticContentMsg,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<UpdateStaticContentMsg>>::Result {
        let conn = self.conn();

        let mut content = Content::find(&msg.id, &conn).ok_or_else(|| -> Error { "failed to load content".into() })?;
        let subj_msg = msg.subj.take().ok_or_else(|| -> Error { "no acl message".into() } )?;
        let subj = UserSubject::from(&subj_msg);
        if !subj.may_write(&content) {
            bail!("not allowed");
        }
        content.title = msg.title;
        content.content = msg.content;
        content.content_type = msg.content_type;
        content.updated_at = Utc::now();
        content.save(&conn)?;

        Ok(content)
    }
}
