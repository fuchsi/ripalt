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
use handlers::user::{ConfirmMsg, SignupForm};
use actix_web::AsyncResponder;
use actix_web::HttpMessage;

pub fn signup(req: HttpRequest<State>) -> SyncResponse<HttpResponse> {
    let mut ctx = Context::new();
    ctx.insert("username", "");
    ctx.insert("email", "");
    ctx.insert("error", "");
    ctx.insert("confirm_id", "");
    Template::render(&req.state().template(), "signup/signup.html", &ctx)
}

pub fn take_signup(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let cloned = req.clone();
    let form = match cloned.urlencoded::<SignupForm>().wait() {
        Ok(form) => form,
        Err(e) => return Box::new(future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))))
    };

    let cloned = req.clone();
    cloned.state()
        .db()
        .send(form.clone())
        .from_err()
        .and_then(move |r| {
            let mut ctx = HashMap::new();
            let mut fail = true;

            match r {
                Ok(confirm_id) => {
                    fail = false;
                    let settings = match SETTINGS.read() {
                        Ok(s) => s,
                        Err(e) => return Err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
                    };

                    if settings.email.enabled {
                        // send confirmation mail
                        ctx.insert("confirm_id", "".to_string());
                    } else {
                        ctx.insert("confirm_id", confirm_id);
                    }
                }
                Err(e) => {
                    ctx.insert("error", format!("{}", e));
                }
            }

            let tpl = if fail {
                ctx.insert("username", form.username);
                ctx.insert("email", form.email);
                "signup/signup.html"
            } else {
                "signup/signup_complete.html"
            };

            let template = req.state().template();
            Template::render(&template, tpl, ctx)
        })
        .responder()
}

pub fn confirm(req: HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let id = match req.match_info().query("id") {
        Ok(id) => id,
        Err(e) => return Box::new(future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))))
    };
    let ip_address = match req.peer_addr() {
        Some(sock_addr) => sock_addr.ip(),
        None => return Box::new(future::err(actix_web::error::ErrorInternalServerError("failed to get peer address")))
    };
    let confirm = ConfirmMsg {
        id,
        ip_address,
    };

    let cloned = req.clone();
    cloned.state()
        .db()
        .send(confirm)
        .from_err()
        .and_then(move |res| {
            let mut ctx = HashMap::new();
            let mut fail = true;

            match res {
                Ok(user) => {
                    match req.session().set("user_id", user.id) {
                        Ok(_) => {},
                        Err(e) => return Err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
                    };
                    match req.session().set("group_id", user.group_id) {
                        Ok(_) => {},
                        Err(e) => return Err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
                    };
                    fail = false;
                },
                Err(e) => {
                    ctx.insert("error", format!("{}", e));
                }
            }

            let tpl = if fail {
                "signup/confirm_fail.html"
            } else {
                "signup/confirm_complete.html"
            };

            let template = req.state().template();
            Template::render(&template, tpl, ctx)
        })
        .responder()
}
