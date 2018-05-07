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

use handlers::user::LoadUserProfileMsg;
use models::user::UserProfileMsg;
use actix_web::AsyncResponder;

pub fn profile(mut req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let user_id = match req.session().get::<Uuid>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => return Either::A(redirect("/login")),
    };

    let cloned = req.clone();
    let fut = req.state().db().send(LoadUserProfileMsg(user_id, user_id, req.state().acl().clone()))
        .from_err()
        .and_then(move |result: Result<UserProfileMsg>| {
            match result {
                Ok(user) => {
                    let mut ctx = Context::new();
                    ctx.insert("user", &user.user);
                    ctx.insert("active_uploads", &user.active_uploads);
                    ctx.insert("active_downloads", &user.active_downloads);
                    ctx.insert("uploads", &user.uploads);
                    ctx.insert("completed", &user.completed);
                    ctx.insert("connections", &user.connections);
                    ctx.insert("timezone", &user.timezone);
                    ctx.insert("may_view_passcode", &user.may_view_passcode);
                    Template::render_with_user(&cloned, "user/profile.html", &mut ctx)
                },
                Err(e) => {
                    info!("user '{}' not found: {}", user_id, e);
                    Err(ErrorNotFound(e.to_string()))
                }
            }
        });

    Either::B(fut.responder())
}

pub fn view(mut req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let user_id = match req.match_info().get("id"){
        Some(user_id) => match Uuid::parse_str(user_id) {
            Ok(user_id) => user_id,
            Err(_) => return Either::A(HttpResponse::NotFound().finish()),
        },
        None => return Either::A(HttpResponse::NotFound().finish()),
    };
    let cur_user_id = match req.session().get::<Uuid>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => return Either::A(redirect("/login")),
    };

    let cloned = req.clone();
    let fut = req.state().db().send(LoadUserProfileMsg(user_id, cur_user_id, req.state().acl().clone()))
        .from_err()
        .and_then(move |result: Result<UserProfileMsg>| {
            match result {
                Ok(user) => {
                    let mut ctx = Context::new();
                    ctx.insert("user", &user.user);
                    ctx.insert("active_uploads", &user.active_uploads);
                    ctx.insert("active_downloads", &user.active_downloads);
                    ctx.insert("uploads", &user.uploads);
                    ctx.insert("completed", &user.completed);
                    ctx.insert("connections", &user.connections);
                    ctx.insert("timezone", &user.timezone);
                    ctx.insert("may_view_passcode", &user.may_view_passcode);
                    Template::render_with_user(&cloned, "user/profile.html", &mut ctx)
                },
                Err(e) => {
                    info!("user '{}' not found: {}", user_id, e);
                    Err(ErrorNotFound(e.to_string()))
                }
            }
        });

    Either::B(fut.responder())
}