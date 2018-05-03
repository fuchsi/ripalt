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

use self::identity::{ApiIdentityPolicy, IdentityService};

mod chat;
pub(crate) mod identity;
pub(crate) mod message;
mod user;

pub fn build(db: Addr<Syn, DbExecutor>, acl: Arc<RwLock<Acl>>) -> App<State> {
    let settings = SETTINGS.read().unwrap();
    let jwt_secret = util::from_hex(&settings.jwt_secret).unwrap();
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

    App::with_state(State::new(db, acl))
        .middleware(Logger::default())
        .middleware(DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
        .middleware(
            csrf::CsrfFilter::new()
                .allow_xhr()
                .allowed_origin(listen)
                .allowed_origin(domain),
        )
        .middleware(SessionStorage::new(
            CookieSessionBackend::signed(&session_secret)
                .name(session_name)
                .secure(*session_secure),
        ))
        .middleware(IdentityService::new(ApiIdentityPolicy::new(
            &jwt_secret,
        )))
        .prefix("/api/v1")
        .resource("/user/stats", |r| r.method(Method::GET).a(user::stats))
        .resource("/chat/messages", |r| r.method(Method::GET).a(chat::messages))
        .resource("/chat/publish", |r| r.method(Method::POST).with2(chat::publish))
        .resource("/message/messages", |r| r.method(Method::GET).a(message::messages))
        .resource("/message/unread", |r| r.method(Method::GET).a(message::unread))
        .resource("/message/read", |r| r.method(Method::GET).a(message::message))
        .resource("/message/send", |r| r.method(Method::POST).with2(message::send))
        .resource("/message/delete", |r| r.method(Method::POST).with2(message::delete))
        .resource("/message/mark_read", |r| r.method(Method::POST).with2(message::mark_read))
        .default_resource(|r| r.method(Method::GET).h(NormalizePath::default()))
}
