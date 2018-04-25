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

pub fn profile(mut req: HttpRequest<State>) -> Either<HttpResponse, FutureResponse<HttpResponse>> {
    let user_id = match req.session().get::<Uuid>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => return Either::A(redirect("/login")),
    };

    let cloned = req.clone();
    let fut = req.state().db().send(LoadUserProfileMsg(user_id, user_id, req.state().acl_arc()))
        .from_err()
        .and_then(move |result: Result<UserProfileMsg>| {
            match result {
                Ok(user) => Template::render(&cloned.state().template(), "user/profile.html", &user)
                    .map(|t| t.into()),
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
    let fut = req.state().db().send(LoadUserProfileMsg(user_id, cur_user_id, req.state().acl_arc()))
        .from_err()
        .and_then(move |result: Result<UserProfileMsg>| {
            match result {
                Ok(user) => {
                    Template::render(&cloned.state().template(), "user/profile.html", &user)
                        .map(|t| t.into())
                },
                Err(e) => {
                    info!("user '{}' not found: {}", user_id, e);
                    Err(ErrorNotFound(e.to_string()))
                }
            }
        });

    Either::B(fut.responder())
}