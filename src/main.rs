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

//! Ripalt is a Anti-Leech-Torrent Tracker CMS based on [actix-web](https://github.com/actix/actix-web)
//!

#![recursion_limit = "1024"]
#![feature(decl_macro, use_extern_macros, custom_derive, try_from)]
// allow pass by value, since most request handlers don't consume HttpRequest
// allow unused import, because of false positives when importing traits
#![cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value, unused_imports))]

extern crate actix;
extern crate actix_redis;
extern crate actix_web;
extern crate futures;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derive_enum;
extern crate bytes;
extern crate chrono;
extern crate dotenv;
extern crate ipnetwork;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate uuid;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate argon2rs;
extern crate codepage_437;
extern crate config;
extern crate data_encoding;
extern crate fast_chemail;
extern crate image;
extern crate jsonwebtoken as jwt;
extern crate markdown;
extern crate multipart;
extern crate notify;
extern crate num_cpus;
extern crate number_prefix;
extern crate rand;
extern crate regex;
extern crate ring;
extern crate serde_bencode;
extern crate serde_json;
extern crate tempfile;
extern crate tera;
extern crate url;
extern crate walkdir;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

pub mod api;
pub mod app;
mod cleanup;
pub mod db;
mod error;
pub mod handlers;
pub mod identity;
pub mod models;
mod schema;
pub mod settings;
pub mod state;
pub mod template;
pub mod tracker;
pub mod util;

use std::sync::{mpsc, Arc, RwLock};
use std::thread;

use actix::prelude::*;
use actix_web::error::{ErrorBadRequest, ErrorForbidden, ErrorInternalServerError, ErrorNotFound, ErrorUnauthorized};
use actix_web::middleware::{csrf, CookieSessionBackend, DefaultHeaders, ErrorHandlers, Logger, RequestSession,
                            SessionStorage};
use actix_web::{fs::StaticFiles,
                http::{header, Method, NormalizePath, StatusCode},
                server::HttpServer,
                App,
                Either,
                FutureResponse,
                HttpRequest,
                HttpResponse,
                Responder};
//use actix_redis::RedisSessionBackend;
use chrono::prelude::*;
use diesel::prelude::*;
use dotenv::dotenv;
use futures::future;
use futures::future::{err as FutErr, ok as FutOk, FutureResult};
use futures::prelude::*;
use uuid::Uuid;

use db::{DbConn, DbExecutor};
use error::*;
use handlers::user::RequireUserMsg;
use models::acl::{Acl, Permission, UserSubject};
use settings::Settings;
use state::{AclContainer, State};
use template::Template;
use identity::RequestIdentity;

lazy_static! {
    pub(crate) static ref SETTINGS: RwLock<Settings> = RwLock::new(Settings::new().unwrap());
}

fn main() {
    dotenv().ok();
    env_logger::init();

    let sys = actix::System::new("ripalt");
    let pool = db::init_pool();
    let acl = state::init_acl(&pool);

    // Start n parallel db executors
    let cloned_pool = pool.clone();
    let addr = SyncArbiter::start(num_cpus::get(), move || DbExecutor::new(pool.clone()));

    // Create a new Tera object and wrap it in some thread safe boxes
    // RwLock is needed for the file watcher below, to reload templates when they are changed.
    let tpl = template::init_tera(Arc::clone(&acl));

    // If debug mode is enabled, start a file watcher in order to reload the templates
    let mut tpl_handle = None;
    let mut tpl_tx = None;
    if SETTINGS.read().unwrap().debug {
        let (tx, rx) = mpsc::channel();
        tpl_tx = Some(tx);
        let tpl = tpl.clone();
        tpl_handle = Some(
            thread::Builder::new()
                .name("template watcher".to_string())
                .spawn(move || {
                    template::template_file_watcher(tpl, &rx);
                })
                .unwrap(),
        );
    }

    let (cleanup_tx, rx) = mpsc::channel();
    let cleanup_handle = thread::Builder::new()
        .name("cleanup".to_string())
        .spawn(move || cleanup::cleanup(DbExecutor::new(cloned_pool), &rx))
        .unwrap();

    let http_bind = &SETTINGS.read().unwrap().bind[..];

    // start the main http server
    HttpServer::new(move || {
        vec![
            tracker::build(addr.clone()),
            api::build(addr.clone(), acl.clone()),
            app::build(addr.clone(), tpl.clone(), acl.clone()),
        ]
    }).shutdown_timeout(2)
        .bind(http_bind)
        .unwrap()
        .start();

    info!("listening on {}", http_bind);

    // run all the stuff
    let _ = sys.run();

    // stop and join the template file watcher
    if let Some(thread) = tpl_handle {
        if let Some(tx) = tpl_tx {
            info!("sending shutdown to template watcher");
            tx.send(true).unwrap();
            thread.join().unwrap();
        }
    }

    cleanup_tx.send(true).unwrap();
    cleanup_handle.join().unwrap();
}

fn require_user() -> RequireUserMsg {
    RequireUserMsg(uuid::Uuid::default(), true)
}

impl actix_web::pred::Predicate<State> for RequireUserMsg {
    fn check(&self, req: &mut actix_web::HttpRequest<State>) -> bool {
        match req.session().get::<uuid::Uuid>("user_id") {
            Ok(user_id) => match user_id {
                Some(user_id) => {
                    let require_user = RequireUserMsg(user_id, true);
                    let user = req.state().db().send(require_user).wait().unwrap();
                    match user {
                        Ok(_) => true,
                        Err(_) => false,
                    }
                }
                None => false,
            },
            Err(_) => false,
        }
    }
}

fn session_creds<S>(req: &mut actix_web::HttpRequest<S>) -> Option<(Uuid, Uuid)> {
    let user_id = match req.session().get::<Uuid>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => return None,
    };
    let group_id = match req.session().get::<Uuid>("group_id").unwrap_or(None) {
        Some(group_id) => group_id,
        None => return None,
    };

    Some((user_id, group_id))
}

trait RequestUser {
    fn current_user(&self) -> Option<models::User>;
}

impl RequestUser for HttpRequest<State> {
    fn current_user(&self) -> Option<models::User> {
        if let Some(user_id) = self.user_id() {
            let req_user = RequireUserMsg(*user_id, true);
            match self.state().db().send(req_user).wait() {
                Ok(user) => user.ok(),
                Err(e) => {
                    warn!("failed to load user: {}", e);
                    None
                },
            }
        } else {
            None
        }
    }
}