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
use handlers::user::LoginForm;
use actix_web::AsyncResponder;
use actix_web::HttpMessage;

pub fn login(req: HttpRequest<State>) -> SyncResponse<HttpResponse> {
    let mut ctx = Context::new();
    ctx.insert("username", "");
    ctx.insert("error", "");
    Template::render(&req.state().template(), "login/login.html", &ctx)
}

pub fn take_login(mut req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let cloned = req.clone();
    let form = match cloned.urlencoded::<LoginForm>().wait() {
        Ok(form) => form,
        Err(e) => return Box::new(FutErr(ErrorInternalServerError(format!("{}", e))))
    };

    let cloned = req.clone();
    cloned.state()
        .db()
        .send(form.clone())
        .from_err()
        .and_then(move |r: Result<User>| {
            let mut ctx = Context::new();
            let mut fail = true;

            match r {
                Ok(user) => {
                    match req.session().set("user_id", user.id) {
                        Ok(_) => {},
                        Err(e) => return Err(ErrorInternalServerError(format!("{}", e))),
                    };
                    match req.session().set("group_id", user.group_id) {
                        Ok(_) => {},
                        Err(e) => return Err(ErrorInternalServerError(format!("{}", e))),
                    };
                    fail = false;
                },
                Err(e) => {
                    ctx.insert("error", &format!("{}", e));
                    ctx.insert("username", &form.username);
                }
            }

            if fail {
                let tpl = req.state().template();
                Template::render(&tpl, "login/login.html", &ctx)
            } else {
                Ok(redirect("/"))
            }
        })
        .responder()
}

pub fn logout(mut req: HttpRequest<State>) -> SyncResponse<HttpResponse> {
    req.session().clear();
    let t: Vec<&str> = vec![];
    let url = req.url_for("index", &t).unwrap();
    sync_redirect(&url.to_string())
}
