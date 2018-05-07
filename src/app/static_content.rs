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
use actix_web::Form;
use handlers::UserSubjectMsg;
use handlers::static_content::{LoadStaticContentMsg, UpdateStaticContentMsg};
use models::static_content::Content;
use models::acl::Subject;

#[derive(Deserialize)]
pub struct ContentForm {
    id: String,
    title: String,
    content: String,
    content_type: String,
}

impl Into<UpdateStaticContentMsg> for ContentForm {
    fn into(self) -> UpdateStaticContentMsg {
        let ContentForm{id, title, content, content_type} = self;
        UpdateStaticContentMsg::new(id, title, content, content_type)
    }
}

#[derive(Serialize)]
struct StaticContentContext {
    id: String,
    title: String,
    content: String,
    updated_at: DateTime<Utc>,
    may_edit: bool,
}

impl From<Content> for StaticContentContext {
    fn from(c: Content) -> Self {
        let content = c.render();
        let Content{id, title, content: _, content_type: _, created_at: _, updated_at} = c;
        Self{
            id,
            title,
            content,
            updated_at,
            may_edit: false,
        }
    }
}

fn view(mut req: HttpRequest<State>, id: String) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };

    let msg = LoadStaticContentMsg::new(id);
    req.clone().state().db().send(msg)
        .from_err()
        .and_then(move |result| {
            match result {
                Ok(c) => {
                    let subj = UserSubject::new(&user_id, &group_id, req.state().acl());
                    let may_edit = subj.may_write(&c);
                    let mut ctx = StaticContentContext::from(c);
                    ctx.may_edit = may_edit;
                    Template::render(&req.state().template(), "static_content/view.html", &ctx)
                },
                Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
            }
        })
        .responder()
}

pub fn faq(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    view(req, "faq".to_string())
}

pub fn rules(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    view(req, "rules".to_string())
}

pub fn edit(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };

    let id = {
        let id = req.match_info().get("id");
        if id.is_none() {
            return Box::new(FutErr(ErrorNotFound("no content id")));
        }
        id.unwrap().to_string()
    };

    let msg = LoadStaticContentMsg::new(id);
    req.clone().state().db().send(msg)
        .from_err()
        .and_then(move |result| {
            match result {
                Ok(c) => {
                    let subj = UserSubject::new(&user_id, &group_id, req.state().acl());
                    let may_edit = subj.may_write(&c);
                    let mut ctx = Context::new();
                    ctx.insert("content", &c);
                    if may_edit {
                        Template::render(&req.state().template(), "static_content/edit.html", &ctx)
                    } else {
                        sync_redirect("/")
                    }
                },
                Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
            }
        })
        .responder()
}

pub fn update(mut req: HttpRequest<State>, data: Form<ContentForm>) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };


    let id = {
        let id = req.match_info().get("id");
        if id.is_none() {
            return Box::new(FutErr(ErrorNotFound("no content id")));
        }
        id.unwrap().to_string()
    };

    let data = data.into_inner();
    let mut msg: UpdateStaticContentMsg = data.into();
    let subj = UserSubjectMsg::new(user_id, group_id, req.state().acl().clone());
    msg.set_acl(subj);
    req.clone().state().db().send(msg)
        .from_err()
        .and_then(move |result| {
            match result {
                Ok(c) => {
                    let may_edit = true;
                    let mut ctx = StaticContentContext::from(c);
                    ctx.may_edit = may_edit;
                    Template::render(&req.state().template(), "static_content/view.html", &ctx)
                },
                Err(e) => {
                    let mut ctx = Context::new();
                    ctx.insert("error", &e.to_string());
                    ctx.insert("back_link", &format!("/content/edit/{}", id));
                    ctx.insert("title", "Edit failed");

                    Template::render(&req.state().template(), "static_content/edit_failed.html", &ctx)
                },
            }
        })
        .responder()
}
