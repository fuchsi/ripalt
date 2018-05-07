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

use std::sync::{Arc, RwLock, RwLockReadGuard};

use db::{DbConn, DbExecutor, Pool};
use models::acl::Acl;
use template::TemplateContainer;
use template::TemplateSystem;

//pub type AclContainer = Arc<RwLock<Acl>>;

#[derive(Clone)]
pub struct AclContainer {
    inner: Arc<RwLock<Acl>>
}

impl AclContainer {
    pub fn new(acl: Arc<RwLock<Acl>>) -> Self {
        Self{inner: acl}
    }

    pub fn new_empty() -> Self {
        Self{inner: Arc::new(RwLock::new(Acl::new()))}
    }

    pub fn set_acl(&mut self, acl: Arc<RwLock<Acl>>) {
        self.inner = acl;
    }

    /// Reload the ACL structure from the database
    ///
    /// # Panics
    ///
    /// This function panics if the underlying database shits the bed.
    #[allow(dead_code)]
    pub fn reload(&mut self, db: &PgConnection) {
        self.inner.write().unwrap().reload(db)
    }

    /// Check if the `user` is allowed to do `perm` in namespace `ns`
    ///
    /// `user` - the user to test
    ///
    /// `ns` - acl namespace
    ///
    /// `perm` - permission to test
    pub fn is_allowed(&self, uid: &Uuid, gid: &Uuid, ns: &str, perm: &Permission) -> bool {
        self.inner.read().unwrap().is_allowed(uid, gid, ns, perm)
    }

    /// Check if the `group` is allowed to do `perm` in namespace `ns`
    ///
    /// if no explicit rule for the group is found check the parent group(s) recursively until an
    /// explicit rule is found.
    /// If no rule is found the function returns `false`
    pub fn is_group_allowed(&self, gid: &Uuid, ns: &str, perm: &Permission) -> bool {
        self.inner.read().unwrap().is_group_allowed(gid, ns, perm)
    }
}

/// State represents the shared state for the application
pub struct State {
    db: Addr<Syn, DbExecutor>,
    acl: AclContainer,
    template: Option<TemplateContainer>,
}

impl State {
    /// Create a new State object
    pub fn new(db: Addr<Syn, DbExecutor>, acl: Arc<RwLock<Acl>>) -> Self {
        State {
            db,
            acl: AclContainer::new(acl),
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
    pub fn acl(&self) -> &AclContainer {
        &self.acl
    }

    /// Get the Template object
    pub fn template(&self) -> RwLockReadGuard<TemplateSystem> {
        match &self.template {
            Some(template) => template.read().unwrap(),
            None => panic!("template system not initialized"),
        }
    }
}

pub fn init_acl(pool: &Pool) -> Arc<RwLock<Acl>> {
    let mut acl = Acl::new();
    acl.load(&DbConn(pool.get().unwrap()));

    Arc::new(RwLock::new(acl))
}