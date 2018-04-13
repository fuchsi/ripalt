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

use std::sync::{Arc, RwLock, RwLockReadGuard};
use actix::prelude::*;

use db::{DbConn, DbExecutor, Pool};
use models::acl::Acl;
use template::TemplateContainer;
use template::TemplateSystem;

/// State represents the shared state for the application
pub struct State {
    db: Addr<Syn, DbExecutor>,
    acl: Arc<RwLock<Acl>>,
    template: Option<TemplateContainer>,
}

impl State {
    /// Create a new State object
    pub fn new(db: Addr<Syn, DbExecutor>, acl: Arc<RwLock<Acl>>) -> Self {
        State {
            db,
            acl,
            template: None,
        }
    }

    /// Set the template object
    pub fn set_template(&mut self, template: TemplateContainer) {
        self.template = Some(template);
    }

    /// Get the database object
    pub fn db(&self) -> &Addr<Syn, DbExecutor> {
        &self.db
    }

    /// Get the ACL object
    pub fn acl(&self) -> RwLockReadGuard<Acl> {
        self.acl.read().unwrap()
    }

    /// Get the Template object
    pub fn template(&self) -> RwLockReadGuard<TemplateSystem> {
        match &self.template {
            Some(template) => template.read().unwrap(),
            None => panic!("template system not initialized"),
        }
    }

    /// Get a cloned arc container with the template object
    pub fn template_arc(&self) -> TemplateContainer {
        match &self.template {
            Some(template) => template.clone(),
            None => panic!("template system not initialized"),
        }
    }
}

pub fn init_acl(pool: &Pool) -> Arc<RwLock<Acl>> {
    let mut acl = Acl::new();
    acl.load(&DbConn(pool.get().unwrap()));

    Arc::new(RwLock::new(acl))
}