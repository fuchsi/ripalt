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

pub fn login(req: HttpRequest<State>) -> SyncResponse<Template> {
    let mut ctx = Context::new();
    ctx.insert("username", "");
    ctx.insert("error", "");
    Template::render(&req.state().template(), "login/login.html", &ctx)
}

pub fn take_login(
    req: HttpRequest<State>,
) -> Box<Future<Item = HttpResponse, Error = actix_web::Error>> {
    let req2 = req.clone();
    let form = match req.urlencoded::<LoginForm>().wait() {
        Ok(form) => form,
        Err(e) => return Box::new(future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))))
    };
    let mut req = req2;
    let hbs_reg = req.state().template_arc();

    req.state()
        .db()
        .send(form)
        .from_err()
        .and_then(move |r: Result<User>| {
            let mut ctx = HashMap::new();
            let mut fail = true;

            match r {
                Ok(user) => {
                    debug!("set session / user_id to: {:?}", user.id);
                    match req.session().set("user_id", user.id) {
                        Ok(_) => {},
                        Err(e) => return future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
                    };
                    fail = false;
                },
                Err(e) => {
                    ctx.insert("error", format!("{}", e));
                }
            }

            if fail {
                let reg = match hbs_reg.read() {
                    Ok(s) => s,
                    Err(e) => return future::err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
                };
                match Template::render(&reg, "login/login.html", ctx) {
                    Ok(resp) => future::ok(resp.into()),
                    Err(e) => future::err(e),
                }
            } else {
                future::ok(HttpResponse::TemporaryRedirect().header(header::LOCATION, "/").finish())
            }
        })
        .responder()
}

pub fn logout(mut req: HttpRequest<State>) -> SyncResponse<HttpResponse> {
    req.session().clear();
    let t: Vec<&str> = vec![];
    let url = req.url_for("index", &t).unwrap();
    Ok(HttpResponse::TemporaryRedirect().header(header::LOCATION, url.to_string()).finish())
}
