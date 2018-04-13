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

//! User models

use super::*;
use super::schema::{user_properties, users};
use util::{self, password, rand};
use ring::digest;
use ipnetwork::IpNetwork;

/// New users
pub const STATUS_NEW: i16 = 0;
/// Inactive/parked users
pub const STATUS_INACTIVE: i16 = 1;
/// Active/normal users
pub const STATUS_ACTIVE: i16 = 2;
/// Locked users
pub const STATUS_LOCKED: i16 = 3;
/// Banned (permanent or temporary) users
pub const STATUS_BANNED: i16 = 4;

const SALTBYTES: usize = 32;

#[derive(Queryable, Debug, Associations, Identifiable, Insertable, AsChangeset, PartialEq)]
#[table_name = "users"]
#[primary_key(id)]
#[belongs_to(Group)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password: Bytes,
    pub salt: Bytes,
    pub status: i16,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub passcode: Bytes,
    pub uploaded: i64,
    pub downloaded: i64,
    pub group_id: Uuid,
    pub ip_address: Option<IpNetwork>,
}

impl Default for User {
    fn default() -> Self {
        User {
            id: Default::default(),
            name: Default::default(),
            email: Default::default(),
            password: Default::default(),
            salt: Default::default(),
            status: STATUS_NEW,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            passcode: vec![],
            uploaded: 0,
            downloaded: 0,
            group_id: Default::default(),
            ip_address: None,
        }
    }
}

impl User {
    /// Find an User by its id
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<User> {
        users::dsl::users.find(id).first::<User>(db).ok()
    }

    /// Find an User by the username
    pub fn find_by_name(name: &str, db: &PgConnection) -> Option<User> {
        users::dsl::users
            .filter(users::dsl::name.eq(name))
            .first::<User>(db)
            .ok()
    }

    /// Find an User by the email address
    pub fn find_by_email(email: &str, db: &PgConnection) -> Option<User> {
        users::dsl::users
            .filter(users::dsl::email.eq(email))
            .first::<User>(db)
            .ok()
    }

    /// Find an User by the passcode
    pub fn find_by_passcode(passcode: &[u8], db: &PgConnection) -> Option<User> {
        users::dsl::users
            .filter(users::dsl::passcode.eq(passcode))
            .first::<User>(db)
            .ok()
    }

    /// Set the password to the generated hash of the given password.
    ///
    /// Generates a new `SALTBYTES` byte salt and sets `self.salt` to this salt.
    ///
    /// See the `util::password` module for the hashing implementation.
    pub fn set_password(&mut self, plain_password: &str) {
        let salt = rand::gen_random_bytes(SALTBYTES);
        self.password = password::generate_passhash(plain_password.as_bytes(), &salt);
        self.salt = salt;
    }

    /// Verify that he given password is identical to the stored one
    ///
    /// See the `util::password` module for the verify implementation.
    pub fn verify_password(&self, password: &str) -> bool {
        // use password::verify for a constant time compare.
        password::verify(&self.password, password.as_bytes(), &self.salt)
    }

    /// Create a new user
    pub fn create(
        db: &PgConnection,
        name: String,
        email: String,
        password: &str,
        group: &Group,
    ) -> Result<User> {
        let mut user = User::default();
        let passcode_len = match SETTINGS.read() {
            Ok(s) => s.user.passcode_length,
            Err(e) => {
                warn!("failed to read settings: {}", e);
                16
            }
        };
        user.id = Uuid::new_v4();
        user.name = name;
        user.email = email;
        user.set_password(password);
        user.passcode = rand::gen_random_bytes(passcode_len);
        user.group_id = group.id;

        user.insert(db)
    }

    /// Insert the user into the database
    fn insert(&self, db: &PgConnection) -> Result<User> {
        let res = self.insert_into(users::table).get_result(db);

        match res {
            Ok(user) => Ok(user),
            Err(e) => bail!("failed to create user: {}", e),
        }
    }

    pub fn create_confirm_id(&self, db: &PgConnection) -> Bytes {
        let bytes = rand::gen_random_bytes(16);
        let digest = digest::digest(&digest::SHA256, &bytes).as_ref().to_vec();
        let property = Property::new(String::from("confirm_id"), util::to_hex(&digest), &self.id);
        property.save(db).unwrap();

        digest
    }

    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        use schema::users::dsl;
        let query = diesel::update(users::table).set(self).filter(dsl::id.eq(&self.id));
        trace!("query: {}", diesel::debug_query::<diesel::pg::Pg, _>(&query));
        query.execute(db).chain_err(|| "user update failed")
    }
}

pub trait HasUser {
    fn user_name(&self, db: &PgConnection) -> Option<String> {
        use schema::users::dsl;
        users::table.select(users::name).filter(dsl::id.eq(self.user_id())).first(db).ok()
    }

    fn user_id(&self) -> &Uuid;
}

pub trait MaybeHasUser {
    fn user_name(&self, db: &PgConnection) -> Option<String> {
        use schema::users::dsl;
        match self.user_id() {
            Some(uid) => users::table.select(users::name).filter(dsl::id.eq(uid)).first(db).ok(),
            None => None,
        }
    }

    fn user_id(&self) -> &Option<Uuid>;
}


#[derive(Queryable, Debug, Associations, Identifiable, Insertable, AsChangeset)]
#[table_name = "user_properties"]
#[belongs_to(User)]
pub struct Property {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub value: serde_json::Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Property {
    pub fn new<T>(name: String, value: T, user_id: &Uuid) -> Self
    where
        T: Into<serde_json::Value>,
    {
        let value: serde_json::Value = value.into();
        Property {
            id: Uuid::new_v4(),
            name,
            value,
            user_id: *user_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn save(&self, db: &PgConnection) -> QueryResult<usize> {
        self.insert_into(user_properties::table).execute(db)
    }

    pub fn get_from_name_value<T>(name: &str, value: T, db: &PgConnection) -> Option<Property>
    where
        T: Into<serde_json::Value>,
    {
        use schema::user_properties::dsl;
        let json: serde_json::Value = value.into();

        dsl::user_properties
            .filter(dsl::name.eq(name))
            .filter(dsl::value.eq(&json))
            .first::<Property>(db)
            .ok()
    }

    pub fn delete(&self, db: &PgConnection) -> Result<usize> {
        use schema::user_properties::dsl;
        ::diesel::delete(user_properties::table).filter(dsl::id.eq(self.id)).execute(db).chain_err(|| "delete property failed")
    }
}
