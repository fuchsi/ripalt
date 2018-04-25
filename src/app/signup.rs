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

pub fn signup(req: HttpRequest<State>) -> SyncResponse<Template> {
    let mut ctx = Context::new();
    ctx.insert("username", "");
    ctx.insert("email", "");
    ctx.insert("error", "");
    ctx.insert("confirm_id", "");
    Template::render(&req.state().template(), "signup/signup.html", &ctx)
}

pub fn take_signup(
    req: HttpRequest<State>,
) -> Box<Future<Item = HttpResponse, Error = actix_web::Error>> {
    // Clone the request before creating the form out of it.
    // Bcz cloned requests don't contain the body smh. ¯\_(ツ)_/¯
    let req2 = req.clone();
    let form = match req.urlencoded::<SignupForm>().wait() {
        Ok(form) => form,
        Err(e) => return Box::new(future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))))
    };
    let req = req2;
    let hbs_reg = req.state().template_arc();

    req.state()
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
                        Err(e) => return future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
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

            let reg = match hbs_reg.read() {
                Ok(s) => s,
                Err(e) => return future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
            };
            future::result(Template::render(&reg, tpl, ctx).map(|t| t.into() ))
        })
        .responder()
}

pub fn confirm(mut req: HttpRequest<State>) -> Box<Future<Item = HttpResponse, Error = actix_web::Error>> {
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

    let hbs_reg = req.state().template_arc();

    req.state()
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
                        Err(e) => return future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
                    };
                    match req.session().set("group_id", user.group_id) {
                        Ok(_) => {},
                        Err(e) => return future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
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

            let reg = match hbs_reg.read() {
                Ok(s) => s,
                Err(e) => return future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
            };
            match Template::render(&reg, tpl, ctx) {
                Ok(resp) => future::ok(resp.into()),
                Err(e) => future::err(e),
            }
        })
        .responder()
}
