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

//! Template system
//!
//! This template system uses [Tera](https://tera.netlify.com/)

use super::*;

use std::sync::{mpsc, Arc, RwLock};
use std::time::Duration;

use tera::Tera;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

pub type TemplateContainer = Arc<RwLock<Tera>>;
pub type TemplateSystem = Tera;

mod helper;

/// Initialize the Tera template system
pub fn init_tera() -> TemplateContainer {
    let mut tera = match Tera::new("templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            error!("{}", e);
            error!("{:#?}", e);
            panic!("failed to load templates {}", e);
        }
    };

    tera.register_filter("data_size", helper::data_size);
    tera.register_filter("format_date", helper::format_date);
    tera.register_filter("duration", helper::duration);

    Arc::new(RwLock::new(tera))
}

/// Watcher function to detect changed templates
///
/// If any file in the template fonder is changed, the handlebars registry is flushed and all
/// templates are re-registered again.
///
/// `main_rx` is the communication channel with the calling function, if it receives any value
/// the watcher function will terminate.
///
/// # Panics
/// This function panics if the `Watcher` can not be created, can not watch the template directory
/// or if it fails to acquire a Write-Lock on the `tera` object.
pub fn template_file_watcher(tpl: TemplateContainer, main_rx: &mpsc::Receiver<bool>) {
    info!("started template watcher");
    // Create a channel to receive the events.
    let (tx, rx) = mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2)).unwrap();

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/");
    debug!("watching {} for changes", path);
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::Recursive).unwrap();

    loop {
        // try to receive from the main_rx in order to terminate
        if main_rx.try_recv().is_ok() {
            info!("shutting down template watcher");
            return;
        }

        // receive with a timeout from the watcher channel.
        // the timeout is necessary to read from the main->watcher channel in order
        // to shut down.
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(event) => {
                info!("reloading templates: {:?}", event);
                let mut tera = tpl.write().unwrap();
                match tera.full_reload() {
                    Ok(_) => info!("templates reloaded"),
                    Err(e) => error!("failed to reload templates: {}", e),
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(e) => warn!("watch error: {:?}", e),
        }
    }
}

/// Container for the rendered template data
///
/// Template can be used as a `Responder` or `HttpResponse` for handler functions.
pub struct Template {
    pub body: String,
    pub content_type: String,
}

impl Template {
    /// Create a new Template container
    pub fn new(body: String, content_type: String) -> Self {
        Template { body, content_type }
    }

    /// Render a registered template
    ///
    /// render returns a `Result`, which is suitable to be returned by a handler function.
    /// If the template fails to render, `Error` is set to `ErrorInternalServerError`
    pub fn render<T>(
        tpl: &TemplateSystem,
        name: &str,
        ctx: T,
    ) -> ::std::result::Result<Self, actix_web::Error>
    where
        T: Serialize,
    {
        let s = tpl.render(name, &ctx).map_err(|e| {
            error!("{:#?}", e);
            ErrorInternalServerError(format!("{}", e))
        })?;

        let mut tpl = Template::default();
        tpl.body = s;

        Ok(tpl)
    }
}

impl Default for Template {
    fn default() -> Self {
        Template {
            body: Default::default(),
            content_type: String::from("text/html;  charset=utf-8"),
        }
    }
}

impl Into<HttpResponse> for Template {
    fn into(self) -> HttpResponse {
        HttpResponse::build(StatusCode::OK)
            .content_type(&self.content_type[..])
            .body(self.body)
    }
}

impl Responder for Template {
    type Item = HttpResponse;
    type Error = actix_web::Error;

    fn respond_to(
        self,
        _req: HttpRequest<()>,
    ) -> actix_web::Result<HttpResponse, actix_web::Error> {
        Ok(self.into())
    }
}
