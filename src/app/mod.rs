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

//! Application Handlers

use super::*;

use std::collections::HashMap;
use std::fmt::Write;

use models::User;

use actix_web::HttpMessage;
use template::TemplateContainer;
use tera::Context;
use bytes::Bytes;
use multipart::server::{save::SavedData, Entries, Multipart, SaveResult};
use std::io::Cursor;
use identity::{IdentityService, AppIdentityPolicy};

mod index;
mod login;
mod message;
mod signup;
mod static_content;
mod torrent;
mod user;

type SyncResponse<T> = actix_web::Result<T>;

fn sync_redirect(loc: &str) -> SyncResponse<HttpResponse> {
    Ok(redirect(loc))
}
fn async_redirect(loc: &str) -> FutureResponse<HttpResponse> {
    Box::new(future::ok(redirect(loc)))
}
fn redirect(loc: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .header(header::LOCATION, loc.to_owned())
        .finish()
}

pub fn build(
    db: Addr<Syn, DbExecutor>,
    tpl: TemplateContainer,
    acl: Arc<RwLock<Acl>>,
) -> App<State> {
    let settings = SETTINGS.read().unwrap();
    //    let redis = env::var("REDIS").unwrap_or(String::from("127.0.0.1::6379"));
    let session_secret = util::from_hex(&settings.session_secret).unwrap();
    let session_name = &settings.session_name[..];
    let session_secure = &settings.https;
    let listen = format!(
        "http{}://{}",
        if settings.https { "s" } else { "" },
        settings.bind
    );
    let domain = format!(
        "http{}://{}",
        if settings.https { "s" } else { "" },
        settings.domain
    );

    let mut state = State::new(db, acl);
    state.set_template(tpl);
    App::with_state(state)
        .middleware(Logger::default())
        .middleware(DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
        .middleware(
            csrf::CsrfFilter::new()
                .allow_xhr()
                .allowed_origin(listen)
                .allowed_origin(domain),
        )
//        .middleware(SessionStorage::new(RedisSessionBackend::new(
//            redis,
//            &session_secret,
//        ).cookie_name(session_name)))
        .middleware(SessionStorage::new(
            CookieSessionBackend::signed(&session_secret)
                .name(session_name)
                .secure(*session_secure),
        ))
        .middleware(
            ErrorHandlers::new().handler(StatusCode::INTERNAL_SERVER_ERROR, app::server_error)
                .handler(StatusCode::NOT_FOUND, app::server_error),
        )
        .middleware(IdentityService::new(AppIdentityPolicy::new()))
        .handler("/static", StaticFiles::new("webroot/static/"))
        .handler("/timg", StaticFiles::new("webroot/timg"))
        .resource("/", |r| {
            r.name("index");
            r.route().filter(require_user()).f(app::index::authenticated);

            r.f(app::index::index);
        })
        .resource("/signup", |r| {
            r.name("signup#signup");
            r.method(Method::GET).f(app::signup::signup);
            r.name("signup#take_signup");
            r.method(Method::POST).a(app::signup::take_signup);
        })
        .resource("/confirm/{id}", |r| {
            r.name("signup#confirm");
            r.method(Method::GET).a(app::signup::confirm)
        })
        .resource("/login", |r| {
            r.name("login#login");
            r.method(Method::GET).f(app::login::login);
            r.name("login#take_login");
            r.method(Method::POST).a(app::login::take_login);
        })
        .resource("/logout", |r| {
            r.name("login#logout");
            r.method(Method::GET).filter(require_user()).f(app::login::logout)
        })
        .resource("/torrents", |r| {
            r.name("torrent#list");
            r.route().filter(require_user()).a(app::torrent::list);
        })
        .resource("/torrent/upload", |r| {
            r.name("torrent#list");
            r.method(Method::GET).filter(require_user()).f(app::torrent::new);
            r.name("torrent#upload");
            r.method(Method::POST).filter(require_user()).f(app::torrent::create);
        })
        .resource("/torrent/edit/{id}", |r| {
            r.name("torrent#edit");
            r.method(Method::GET).filter(require_user()).a(app::torrent::edit);
            r.name("torrent#update");
            r.method(Method::POST).filter(require_user()).a(app::torrent::update);
        })
        .resource("/torrent/delete/{id}", |r| {
            r.name("torrent#delete");
            r.method(Method::GET).filter(require_user()).a(app::torrent::delete);
            r.name("torrent#delete");
            r.method(Method::POST).filter(require_user()).a(app::torrent::do_delete);
        })
        .resource("/torrent/download/{id}", |r| {
            r.name("torrent#download");
            r.method(Method::GET).filter(require_user()).f(app::torrent::download);
        })
        .resource("/torrent/nfo/{id}", |r| {
            r.name("torrent#nfo");
            r.method(Method::GET).filter(require_user()).f(app::torrent::nfo);
        })
        .resource("/torrent/{id}", |r| {
            r.name("torrent#read");
            r.method(Method::GET).filter(require_user()).f(app::torrent::torrent);
        })
        .resource("/user/profile", |r| {
            r.name("user#profile");
            r.method(Method::GET).filter(require_user()).f(app::user::profile);
        })
        .resource("/user/{id}", |r| {
            r.name("user#profile");
            r.method(Method::GET).filter(require_user()).f(app::user::view);
        })
        .resource("/messages/{folder:.*}", |r| {
            r.name("message#messages");
            r.method(Method::GET).filter(require_user()).f(app::message::messages);
        })
        .resource("/message/new", |r| {
            r.name("message#new");
            r.method(Method::GET).filter(require_user()).f(app::message::new);
        })
        .resource("/message/reply/{id}", |r| {
            r.name("message#reply");
            r.method(Method::GET).filter(require_user()).f(app::message::reply);
        })
        .resource("/message/{id}", |r| {
            r.name("message#message");
            r.method(Method::GET).filter(require_user()).f(app::message::message);
        })
        .resource("/faq", |r| {
            r.name("faq");
            r.method(Method::GET).filter(require_user()).a(app::static_content::faq)
        })
        .resource("/rules", |r| {
            r.name("rules");
            r.method(Method::GET).filter(require_user()).a(app::static_content::rules)
        })
        .resource("/content/edit/{id}", |r| {
            r.name("content#edit");
            r.method(Method::GET).filter(require_user()).a(app::static_content::edit);
            r.method(Method::POST).filter(require_user()).with2(app::static_content::update);
        })
        .default_resource(|r| r.f(app::not_found))
}

pub fn not_found(req: HttpRequest<State>) -> HttpResponse {
    use actix_web::dev::Handler;

    let mut h = NormalizePath::default();
    h.handle(req.clone())
}

pub fn server_error(
    req: &mut HttpRequest<State>,
    resp: HttpResponse,
) -> SyncResponse<actix_web::middleware::Response> {
    Ok(actix_web::middleware::Response::Done(render_error(
        req, resp,
    )))
}

fn render_error(req: &HttpRequest<State>, resp: HttpResponse) -> HttpResponse {
    let mut context = HashMap::new();
    context.insert("status", format!("{}", resp.status()));
    context.insert("uri", format!("{}", req.uri()));
    let mut headers = String::new();
    for (n, v) in req.headers() {
        writeln!(&mut headers, "{:?}: {:?}", n, v).unwrap();
    }
    context.insert("headers", headers);
    context.insert("error", "Internal Server Error".to_string());

    let tpl = if resp.status().is_server_error() {
        match resp.body() {
            actix_web::Body::Binary(b) => match b {
                actix_web::Binary::Bytes(bytes) => {
                    if let Ok(str) = String::from_utf8(bytes.to_vec()) {
                        context.insert("error", str);
                    }
                },
                actix_web::Binary::SharedString(str) => {
                    context.insert("error", str.to_string());
                },
                _ => {},
            },
            _ => {},
        }
        "error/5xx.html"
    } else {
        "error/4xx.html"
    };

    let mut new_resp: HttpResponse = match Template::render(&req.state().template(), tpl, &context)
    {
        Ok(r) => r.into(),
        Err(e) => {
            return resp.into_builder()
                .header(header::CONTENT_TYPE, "text/plain")
                .status(StatusCode::from_u16(500u16).unwrap())
                .body(format!("Internal Server Error\n{}", e))
        }
    };
    {
        let status = new_resp.status_mut();
        *status = resp.status();
    }

    new_resp
}

#[derive(Debug, Clone)]
struct MultipartRequest {
    body: Bytes,
    boundary: String,
}

impl MultipartRequest {
    pub fn new(content_type: &str, body: Bytes) -> Self {
        let boundary: String = match content_type.rfind("boundary=") {
            // todo: check for trailing stuff
            Some(index) => {
                let index = index + 9; // add length of 'boundary='
                String::from(&content_type[index..])
            }
            None => String::from("--"),
        };
        MultipartRequest { body, boundary }
    }
}

impl multipart::server::HttpRequest for MultipartRequest {
    type Body = Cursor<Bytes>;

    fn multipart_boundary(&self) -> Option<&str> {
        Some(&self.boundary[..])
    }

    fn body(self) -> <Self as multipart::server::HttpRequest>::Body {
        Cursor::new(self.body)
    }
}