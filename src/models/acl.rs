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

//! ACL (Access Control List) Models and functions

use super::schema::{acl_group_rules, acl_user_rules};
use super::*;
use std::collections::HashMap;

/// ACL permissions
#[derive(DbEnum, Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum Permission {
    None,
    Read,
    Write,
    Create,
    Delete,
}

trait Rule {
    /// Check if the rule permits the action `perm`
    fn is_allowed(&self, perm: &Permission) -> bool {
        perm <= self.permission()
    }

    /// Permit the role the given `perm` right in the namespace
    fn permit(&mut self, perm: Permission);
    /// Get the permission
    fn permission(&self) -> &Permission;
    /// Get the namespace
    fn namespace(&self) -> &str;
    /// Get the role
    fn role(&self) -> &Uuid;
}

/// The ACL Group Rules
#[derive(Queryable, Debug, Associations, Identifiable, Insertable, PartialEq)]
#[table_name = "acl_group_rules"]
#[belongs_to(Group)]
struct GroupRule {
    id: Uuid,
    namespace: String,
    group_id: Uuid,
    permission: Permission,
}

impl GroupRule {
    /// Load the corresponding group rule from the database and check if permits the action `perm`
    ///
    /// `group` - the group to test
    ///
    /// `ns` - acl namespace
    ///
    /// `perm` - permission to test
    ///
    /// `db` - a database connection
    pub fn allowed(group: &Uuid, ns: &str, perm: &Permission, db: &PgConnection) -> bool {
        use schema::acl_group_rules::dsl::*;

        let rule = acl_group_rules
            .filter(namespace.eq(ns))
            .filter(group_id.eq(group))
            .first::<GroupRule>(db);

        match rule {
            Ok(rule) => rule.is_allowed(perm),
            Err(_) => false,
        }
    }
}

impl Rule for GroupRule {
    fn permit(&mut self, perm: Permission) {
        self.permission = perm;
    }

    fn permission(&self) -> &Permission {
        &self.permission
    }

    fn namespace(&self) -> &str {
        &self.namespace[..]
    }

    fn role(&self) -> &Uuid {
        &self.group_id
    }
}

/// The ACL User Rules
#[derive(Queryable, Debug, Associations, Identifiable, Insertable, PartialEq)]
#[table_name = "acl_user_rules"]
#[belongs_to(User)]
struct UserRule {
    id: Uuid,
    namespace: String,
    user_id: Uuid,
    permission: Permission,
}

impl UserRule {
    /// Load the corresponding user rule from the database and check if permits the action `perm`
    ///
    /// `user` - the user to test
    ///
    /// `ns` - acl namespace
    ///
    /// `perm` - permission to test
    ///
    /// `db` - a database connection
    pub fn allowed(user: &Uuid, ns: &str, perm: &Permission, db: &PgConnection) -> bool {
        use schema::acl_user_rules::dsl::*;

        let rule = acl_user_rules
            .filter(namespace.eq(ns))
            .filter(user_id.eq(user))
            .first::<UserRule>(db);

        match rule {
            Ok(rule) => rule.is_allowed(perm),
            Err(_) => false,
        }
    }
}

impl Rule for UserRule {
    fn permit(&mut self, perm: Permission) {
        self.permission = perm;
    }

    fn permission(&self) -> &Permission {
        &self.permission
    }

    fn namespace(&self) -> &str {
        &self.namespace[..]
    }

    fn role(&self) -> &Uuid {
        &self.user_id
    }
}

type RuleHashMap<T> = HashMap<String, HashMap<Uuid, T>>;

/// The ACL Structure
///
/// contains all user (`AclUserRule`) and group rules (`AclGroupRule`)
#[derive(Debug, Default)]
pub struct Acl {
    group_rules: RuleHashMap<GroupRule>,
    user_rules: RuleHashMap<UserRule>,
    groups: HashMap<Uuid, Uuid>,
}

impl Acl {
    /// Create a new empty `Acl` object
    pub fn new() -> Self {
        Self {
            group_rules: RuleHashMap::new(),
            user_rules: RuleHashMap::new(),
            groups: HashMap::new(),
        }
    }

    /// Load the ACL structure from the database
    ///
    /// # Panics
    ///
    /// This function panics if the underlying database shits the bed.
    pub fn load(&mut self, db: &PgConnection) {
        self.load_group_acls(db);
        self.load_user_acls(db);
        self.load_groups(db);
    }

    fn load_groups(&mut self, db: &PgConnection) {
        use schema::groups::dsl::*;
        let group_list = groups
            .select((id, parent_id))
            .filter(parent_id.is_not_null())
            .load::<(Uuid, Option<Uuid>)>(db)
            .unwrap();

        for (gid, pid) in group_list {
            if let Some(pid) = pid {
                self.groups.insert(gid, pid);
            }
        }
    }

    fn load_group_acls(&mut self, db: &PgConnection) {
        use schema::acl_group_rules::dsl::*;
        let rules = acl_group_rules
            .order(namespace)
            .order(group_id)
            .load::<GroupRule>(db)
            .unwrap();

        Acl::load_list(rules, &mut self.group_rules);
    }

    fn load_user_acls(&mut self, db: &PgConnection) {
        use schema::acl_user_rules::dsl::*;
        let rules = acl_user_rules
            .order(namespace)
            .order(user_id)
            .load::<UserRule>(db)
            .unwrap();

        Acl::load_list(rules, &mut self.user_rules);
    }

    fn load_list<T>(rules: Vec<T>, map: &mut RuleHashMap<T>)
    where
        T: Rule,
    {
        for rule in rules {
            if let Some(inner) = map.get_mut(rule.namespace()) {
                inner.insert(rule.role().clone(), rule);
                continue;
            }

            let mut inner: HashMap<Uuid, T> = HashMap::new();
            let ns = rule.namespace().to_owned();
            inner.insert(rule.role().clone(), rule);
            map.insert(ns, inner);
        }
    }

    /// Reload the ACL structure from the database
    ///
    /// # Panics
    ///
    /// This function panics if the underlying database shits the bed.
    pub fn reload(&mut self, db: &PgConnection) {
        self.group_rules.clear();
        self.user_rules.clear();
        self.load(db);
    }

    /// Check if the `user` is allowed to do `perm` in namespace `ns`
    ///
    /// `user` - the user to test
    ///
    /// `ns` - acl namespace
    ///
    /// `perm` - permission to test
    pub fn is_allowed(&self, uid: &Uuid, gid: &Uuid, ns: &str, perm: &Permission) -> bool {
        // check if the user has an explicit rule
        if let Some(inner) = self.user_rules.get(ns) {
            if let Some(rule) = inner.get(uid) {
                return rule.is_allowed(perm);
            }
        }

        self.is_group_allowed(gid, ns, perm)
    }

    /// Check if the `group` is allowed to do `perm` in namespace `ns`
    ///
    /// if no explicit rule for the group is found check the parent group(s) recursively until an
    /// explicit rule is found.
    /// If no rule is found the function returns `false`
    pub fn is_group_allowed(&self, gid: &Uuid, ns: &str, perm: &Permission) -> bool {
        // check if the group has an explicit rule
        if let Some(inner) = self.group_rules.get(ns) {
            if let Some(rule) = inner.get(gid) {
                return rule.is_allowed(perm);
            }
        }

        // check the parent group rules
        match self.groups.get(gid) {
            Some(ref pid) => self.is_group_allowed(pid, ns, perm),
            None => false,
        }
    }
}

pub trait Subject<T> {
    fn may_read(&self, obj: &T) -> bool {
        self.may(obj, &Permission::Read)
    }
    fn may_write(&self, obj: &T) -> bool {
        self.may(obj, &Permission::Write)
    }
    fn may_create(&self, obj: &T) -> bool {
        self.may(obj, &Permission::Create)
    }
    fn may_delete(&self, obj: &T) -> bool {
        self.may(obj, &Permission::Delete)
    }
    fn may(&self, obj: &T, perm: &Permission) -> bool;
}

pub struct UserSubject<'a> {
    user_id: &'a Uuid,
    group_id: &'a Uuid,
    acl: AclContainer,
}

impl<'a> UserSubject<'a> {
    pub fn new(user_id: &'a Uuid, group_id: &'a Uuid, acl: AclContainer) -> UserSubject<'a> {
        UserSubject {
            user_id,
            group_id,
            acl,
        }
    }

    fn acl(&self) -> std::sync::RwLockReadGuard<Acl> {
        self.acl.read().unwrap()
    }
}

impl<'a> Subject<Torrent> for UserSubject<'a> {
    fn may(&self, obj: &Torrent, perm: &Permission) -> bool {
        if let Some(uid) = obj.user_id {
            if uid == *self.user_id {
                return true;
            }
        }

        self.acl().is_allowed(self.user_id, self.group_id, "torrent", perm)
    }
}

impl<'a> Subject<User> for UserSubject<'a> {
    fn may(&self, obj: &User, perm: &Permission) -> bool {
        if obj.id == *self.user_id {
            return true;
        }

        self.acl().is_allowed(self.user_id, self.group_id, "user", perm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed() {
        let rule = GroupRule {
            id: Uuid::new_v4(),
            namespace: String::from("test"),
            group_id: Uuid::new_v4(),
            permission: Permission::Write,
        };

        assert!(rule.is_allowed(&Permission::None));
        assert!(rule.is_allowed(&Permission::Read));
        assert!(rule.is_allowed(&Permission::Write));
        assert_eq!(rule.is_allowed(&Permission::Create), false);
        assert_eq!(rule.is_allowed(&Permission::Delete), false);
    }
}
