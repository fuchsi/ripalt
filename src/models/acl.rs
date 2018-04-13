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

use super::*;
use super::schema::{acl_group_rules, acl_user_rules};
use std::collections::HashMap;

/// ACL permissions
#[derive(DbEnum, Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum AclPermission {
    None,
    Read,
    Write,
    Create,
    Delete,
}

trait AclRule {
    /// Check if the rule permits the action `perm`
    fn is_allowed(&self, perm: &AclPermission) -> bool {
        perm <= self.permission()
    }

    /// Permit the role the given `perm` right in the namespace
    fn permit(&mut self, perm: AclPermission);
    /// Get the permission
    fn permission(&self) -> &AclPermission;
    /// Get the namespace
    fn namespace(&self) -> &str;
    /// Get the role
    fn role(&self) -> &Uuid;
}

/// The ACL Group Rules
#[derive(Queryable, Debug, Associations, Identifiable, Insertable, PartialEq)]
#[table_name = "acl_group_rules"]
#[belongs_to(Group)]
pub struct AclGroupRule {
    id: Uuid,
    namespace: String,
    group_id: Uuid,
    permission: AclPermission,
}

impl AclGroupRule {
    /// Load the corresponding group rule from the database and check if permits the action `perm`
    ///
    /// `group` - the group to test
    ///
    /// `ns` - acl namespace
    ///
    /// `perm` - permission to test
    ///
    /// `db` - a database connection
    pub fn allowed(group: &Uuid, ns: &str, perm: &AclPermission, db: &PgConnection) -> bool {
        use schema::acl_group_rules::dsl::*;

        let rule = acl_group_rules
            .filter(namespace.eq(ns))
            .filter(group_id.eq(group))
            .first::<AclGroupRule>(db);

        match rule {
            Ok(rule) => rule.is_allowed(perm),
            Err(_) => false,
        }
    }
}

impl AclRule for AclGroupRule {
    fn permit(&mut self, perm: AclPermission) {
        self.permission = perm;
    }

    fn permission(&self) -> &AclPermission {
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
pub struct AclUserRule {
    id: Uuid,
    namespace: String,
    user_id: Uuid,
    permission: AclPermission,
}

impl AclUserRule {
    /// Load the corresponding user rule from the database and check if permits the action `perm`
    ///
    /// `user` - the user to test
    ///
    /// `ns` - acl namespace
    ///
    /// `perm` - permission to test
    ///
    /// `db` - a database connection
    pub fn allowed(user: &Uuid, ns: &str, perm: &AclPermission, db: &PgConnection) -> bool {
        use schema::acl_user_rules::dsl::*;

        let rule = acl_user_rules
            .filter(namespace.eq(ns))
            .filter(user_id.eq(user))
            .first::<AclUserRule>(db);

        match rule {
            Ok(rule) => rule.is_allowed(perm),
            Err(_) => false,
        }
    }
}

impl AclRule for AclUserRule {
    fn permit(&mut self, perm: AclPermission) {
        self.permission = perm;
    }

    fn permission(&self) -> &AclPermission {
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
    group_rules: RuleHashMap<AclGroupRule>,
    user_rules: RuleHashMap<AclUserRule>,
}

impl Acl {
    /// Create a new empty `Acl` object
    pub fn new() -> Self {
        Self {
            group_rules: RuleHashMap::new(),
            user_rules: RuleHashMap::new(),
        }
    }

    /// Load the ACL structure from the database
    ///
    /// # Panics
    ///
    /// This function panics if the underlying database shits the bed.
    pub fn load(&mut self, db: &DbConn) {
        self.load_groups(db);
        self.load_users(db);
    }

    fn load_groups(&mut self, db: &PgConnection) {
        use schema::acl_group_rules::dsl::*;
        let rules = acl_group_rules
            .order(namespace)
            .order(group_id)
            .load::<AclGroupRule>(db)
            .unwrap();

        Acl::load_list(rules, &mut self.group_rules);
    }

    fn load_users(&mut self, db: &PgConnection) {
        use schema::acl_user_rules::dsl::*;
        let rules = acl_user_rules
            .order(namespace)
            .order(user_id)
            .load::<AclUserRule>(db)
            .unwrap();

        Acl::load_list(rules, &mut self.user_rules);
    }

    fn load_list<T>(rules: Vec<T>, map: &mut RuleHashMap<T>)
    where
        T: AclRule,
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
    pub fn reload(&mut self, db: &DbConn) {
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
    ///
    /// `db` - a database connection
    pub fn is_allowed(&self, user: &User, ns: &str, perm: &AclPermission, db: &DbConn) -> bool {
        // check if the user has an explicit rule
        if let Some(inner) = self.user_rules.get(ns) {
            if let Some(rule) = inner.get(&user.id) {
                return rule.is_allowed(perm);
            }
        }

        if let Some(group) = Group::find(&user.group_id, db) {
            return self.is_group_allowed(&group, ns, perm, db);
        }

        false
    }

    /// Check if the `group` is allowed to do `perm` in namespace `ns`
    ///
    /// if no explicit rule for the group is found check the parent group(s) recursively until an
    /// explicit rule is found.
    /// If no rule is found the function returns `false`
    pub fn is_group_allowed(&self, group: &Group, ns: &str, perm: &AclPermission, db: &DbConn) -> bool {
        // check if the group has an explicit rule
        if let Some(inner) = self.group_rules.get(ns) {
            if let Some(rule) = inner.get(&group.id) {
                return rule.is_allowed(perm);
            }
        }

        // check the parent group rules
        match group.parent_id {
            Some(ref gid) => match Group::find(gid, db) {
                Some(group) => self.is_group_allowed(&group, ns, perm, db),
                None => false,
            },
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed() {
        let rule = AclGroupRule {
            id: Uuid::new_v4(),
            namespace: String::from("test"),
            group_id: Uuid::new_v4(),
            permission: AclPermission::Write,
        };

        assert!(rule.is_allowed(&AclPermission::None));
        assert!(rule.is_allowed(&AclPermission::Read));
        assert!(rule.is_allowed(&AclPermission::Write));
        assert_eq!(rule.is_allowed(&AclPermission::Create), false);
        assert_eq!(rule.is_allowed(&AclPermission::Delete), false);
    }
}
