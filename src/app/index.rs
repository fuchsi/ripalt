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

use models::chat::ChatRoom;
use models::acl::Subject;
use handlers::user::ActiveUsersMsg;
use chrono::Duration;
use actix_web::AsyncResponder;

#[derive(Serialize)]
struct Chat {
    id: String,
    nid: i16,
    name: String,
    active: bool,
}

pub fn authenticated(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let (user_id, group_id) = match session_creds(&mut req) {
        Some((u, g)) => (u, g),
        None => return async_redirect("/login"),
    };
    let mut ctx = Context::new();

    // Chat
    let subj = UserSubject::new(&user_id, &group_id, req.state().acl_arc());
    let mut chatrooms = vec![Chat{id: ChatRoom::Public.to_string(), nid: ChatRoom::Public.into(), name: "Shoutbox".to_string(), active: true}];
    if subj.may_read(&ChatRoom::Team) {
        chatrooms.push(Chat{id: ChatRoom::Team.to_string(), nid: ChatRoom::Team.into(), name: "Teambox".to_string(), active: false});
    }
    ctx.insert("chatrooms", &chatrooms);

    // Active Users
    let active_users = ActiveUsersMsg(Duration::minutes(30));
    let cloned = req.clone();
    req.state().db().send(active_users)
        .from_err()
        .and_then(move |result| {
            match result {
                Ok(active_users) => {
                    ctx.insert("active_users", &active_users);
                },
                Err(e) => {
                    warn!("could not fetch active users: {}", e);
                },
            }
            let tpl = cloned.state().template();
            Template::render(&tpl, "index/authenticated.html", &ctx)
        })
        .responder()
}


pub fn index(req: HttpRequest<State>) -> SyncResponse<HttpResponse> {
    let ctx = Context::new();
    Template::render(&req.state().template(), "index/public.html", &ctx)
}
