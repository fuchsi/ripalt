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

use super::schema::*;
use super::*;
use ipnetwork::IpNetwork;
use models::message::NewMessageFolder;
use ring::digest;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use util::{self, password, rand};

/// New users
#[allow(dead_code)]
pub const STATUS_NEW: i16 = 0;
/// Inactive/parked users
#[allow(dead_code)]
pub const STATUS_INACTIVE: i16 = 1;
/// Active/normal users
pub const STATUS_ACTIVE: i16 = 2;
/// Locked users
#[allow(dead_code)]
pub const STATUS_LOCKED: i16 = 3;
/// Banned (permanent or temporary) users
#[allow(dead_code)]
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
    pub last_active: Option<Timestamp>,
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
            last_active: None,
        }
    }
}

impl User {
    /// Find an `User` by its ID
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<User> {
        users::dsl::users.find(id).first::<User>(db).ok()
    }

    /// Find an `User` by the username
    pub fn find_by_name(name: &str, db: &PgConnection) -> Option<User> {
        users::dsl::users
            .filter(users::dsl::name.eq(name))
            .first::<User>(db)
            .ok()
    }

    /// Find an `User` by the email address
    pub fn find_by_email(email: &str, db: &PgConnection) -> Option<User> {
        users::dsl::users
            .filter(users::dsl::email.eq(email))
            .first::<User>(db)
            .ok()
    }

    /// Find an `User` by the passcode
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

    /// Create a new `User`
    pub fn create(db: &PgConnection, name: String, email: String, password: &str, group: &Group) -> Result<User> {
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

    /// Insert the `User` into the database
    fn insert(&self, db: &PgConnection) -> Result<User> {
        let res = self.insert_into(users::table).get_result(db);

        match res {
            Ok(user) => Ok(user),
            Err(e) => bail!("failed to create user: {}", e),
        }
    }

    /// Create a new confirm id
    ///
    /// # Returns
    ///
    /// the confirm id
    pub fn create_confirm_id(&self, db: &PgConnection) -> Bytes {
        let bytes = rand::gen_random_bytes(16);
        let digest = digest::digest(&digest::SHA256, &bytes).as_ref().to_vec();
        let property = Property::new(String::from("confirm_id"), util::to_hex(&digest), &self.id);
        property.save(db).unwrap();

        digest
    }

    /// Save the `User` into the database
    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        use schema::users::dsl;
        let query = diesel::update(users::table).set(self).filter(dsl::id.eq(&self.id));
        trace!("query: {}", diesel::debug_query::<diesel::pg::Pg, _>(&query));
        query.execute(db).chain_err(|| "user update failed")
    }

    /// Update the `last_active` timestamp to now.
    pub fn update_last_active(&mut self, db: &PgConnection) -> Result<usize> {
        use schema::users::dsl;
        self.last_active = Some(Utc::now());
        diesel::update(users::table)
            .set(dsl::last_active.eq(&self.last_active))
            .filter(dsl::id.eq(&self.id))
            .execute(db)
            .chain_err(|| "user update failed")
    }

    /// Create the [**Message Folders**](../message/struct.MessageFolder.html)
    pub fn create_message_folders(&self, db: &PgConnection) -> Result<()> {
        let folders = vec!["inbox", "sent", "system"];

        for name in folders {
            NewMessageFolder::new(&self.id, name, 0).save(db)?;
        }

        Ok(())
    }

    pub fn create_profile(&self, db: &PgConnection) -> Result<usize> {
        let profile = UserProfile {
            id: self.id,
            ..Default::default()
        };
        profile.save(db)
    }

    pub fn profile(&self, db: &PgConnection) -> UserProfile {
        UserProfile::belonging_to(self)
            .first::<UserProfile>(db)
            .unwrap_or_else(|_| UserProfile {
                id: self.id,
                ..Default::default()
            })
    }
}

impl Serialize for User {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut root = serializer.serialize_struct("user", 11)?;
        root.serialize_field("id", &self.id)?;
        root.serialize_field("name", &self.name)?;
        root.serialize_field("email", &self.email)?;
        root.serialize_field("status", &self.status)?;
        root.serialize_field("created_at", &self.created_at)?;
        root.serialize_field("updated_at", &self.updated_at)?;
        root.serialize_field("last_active", &self.last_active)?;
        root.serialize_field("passcode", &util::to_hex(&self.passcode))?;
        root.serialize_field("uploaded", &self.uploaded)?;
        root.serialize_field("downloaded", &self.downloaded)?;
        root.serialize_field("group_id", &self.group_id)?;
        root.serialize_field("ip_address", &self.ip_address.map(|ip| ip.to_string()))?;
        root.end()
    }
}

pub trait HasUser {
    fn user_name(&self, db: &PgConnection) -> String {
        username(self.user_id(), db).unwrap_or_default()
    }

    fn user_id(&self) -> &Uuid;
}

pub trait MaybeHasUser {
    fn user_name(&self, db: &PgConnection) -> Option<String> {
        match self.user_id() {
            Some(user_id) => username(user_id, db),
            None => None,
        }
    }

    fn user_id(&self) -> &Option<Uuid>;
}

/// A User Property / Setting.
#[derive(Queryable, Debug, Associations, Identifiable, Insertable, AsChangeset, Serialize, Clone)]
#[table_name = "user_properties"]
#[belongs_to(User)]
pub struct Property {
    id: Uuid,
    user_id: Uuid,
    name: String,
    value: Value,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl Property {
    /// Construct a new `Property` instance.
    pub fn new<T>(name: String, value: T, user_id: &Uuid) -> Self
    where
        T: Serialize,
    {
        let value = serde_json::to_value(value).unwrap();
        Property {
            id: Uuid::new_v4(),
            name,
            value,
            user_id: *user_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Save the `Property` into the database.
    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        self.insert_into(user_properties::table)
            .on_conflict(on_constraint("user_properties_user_id_name_key"))
            .do_update()
            .set(UpdateProperty::from(self))
            .execute(db)
            .map_err(|e| format!("failed to save property: {}", e).into())
    }

    /// Find a `Property` by its name for a user.
    pub fn find(user_id: &Uuid, name: &str, db: &PgConnection) -> Option<Property> {
        use schema::user_properties::dsl;

        dsl::user_properties
            .filter(dsl::user_id.eq(&user_id))
            .filter(dsl::name.eq(name))
            .first::<Property>(db)
            .ok()
    }

    /// Find a `Property` by its name and value.
    pub fn find_by_name_value<T>(name: &str, value: T, db: &PgConnection) -> Option<Property>
    where
        T: Serialize,
    {
        use schema::user_properties::dsl;
        let json = match serde_json::to_value(value) {
            Ok(value) => value,
            Err(_) => return None,
        };

        dsl::user_properties
            .filter(dsl::name.eq(name))
            .filter(dsl::value.eq(&json))
            .first::<Property>(db)
            .ok()
    }

    /// Delete the `Property` from the database.
    pub fn delete(&self, db: &PgConnection) -> Result<usize> {
        use schema::user_properties::dsl;
        ::diesel::delete(user_properties::table)
            .filter(dsl::id.eq(self.id))
            .execute(db)
            .chain_err(|| "delete property failed")
    }

    /// Fetch all properties for an user
    pub fn fetch_for_user(id: &Uuid, db: &PgConnection) -> Vec<Property> {
        use schema::user_properties::dsl as p;
        user_properties::table
            .filter(p::user_id.eq(id))
            .order_by(p::name.asc())
            .load::<Property>(db)
            .unwrap()
    }

    pub fn user_id(&self) -> &Uuid {
        &self.user_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn set_value<T: Serialize>(&mut self, value: T) {
        if let Ok(value) = serde_json::to_value(value) {
            self.value = value;
        }
    }
}

#[derive(AsChangeset)]
#[table_name = "user_properties"]
struct UpdateProperty<'prop> {
    user_id: &'prop Uuid,
    name: &'prop str,
    value: &'prop Value,
}

impl<'prop> From<&'prop Property> for UpdateProperty<'prop> {
    fn from(p: &'prop Property) -> Self {
        Self {
            user_id: &p.user_id,
            name: &p.name[..],
            value: &p.value,
        }
    }
}

/// The User stats
#[derive(Debug, Default, Serialize)]
pub struct UserStatsMsg {
    pub id: Uuid,
    pub name: String,
    pub uploaded: i64,
    pub downloaded: i64,
    pub ratio: f64,
    pub uploads: i64,
    pub downloads: i64,
}

#[derive(Default)]
pub struct UserProfileMsg {
    pub user: User,
    pub active_uploads: Vec<UserTransfer>,
    pub active_downloads: Vec<UserTransfer>,
    pub uploads: Vec<UserUpload>,
    pub completed: Vec<CompletedTorrent>,
    pub connections: Vec<UserConnection>,
    pub timezone: i32,
    pub may_view_passcode: bool,
    pub profile: UserProfile,
}

#[derive(Debug, Serialize, Queryable, Identifiable)]
#[table_name = "user_transfer"]
pub struct UserTransfer {
    id: Uuid,
    torrent_id: Uuid,
    user_id: Uuid,
    name: String,
    is_seeder: bool,
    size: i64,
    seeder: i64,
    leecher: i64,
    bytes_uploaded: i64,
    bytes_downloaded: i64,
    total_uploaded: i64,
    total_downloaded: i64,
}

impl UserTransfer {
    pub fn fetch_for_user(user_id: &Uuid, db: &PgConnection) -> Vec<UserTransfer> {
        user_transfer::table
            .filter(user_transfer::dsl::user_id.eq(user_id))
            .order_by(user_transfer::dsl::name.asc())
            .load::<UserTransfer>(db)
            .unwrap()
    }

    pub fn is_seeder(&self) -> bool {
        self.is_seeder
    }
}

#[derive(Debug, Serialize, Queryable, Identifiable)]
#[table_name = "completed_torrents"]
pub struct CompletedTorrent {
    id: Uuid,
    user_id: Uuid,
    torrent_id: Uuid,
    bytes_uploaded: i64,
    bytes_downloaded: i64,
    time_seeded: i32,
    completed_at: Timestamp,
    name: String,
    size: i64,
    is_seeder: bool,
    seeder: i64,
    leecher: i64,
}

impl CompletedTorrent {
    pub fn fetch_for_user(user_id: &Uuid, db: &PgConnection) -> Vec<CompletedTorrent> {
        completed_torrents::table
            .filter(completed_torrents::dsl::user_id.eq(user_id))
            .order_by(completed_torrents::dsl::name.asc())
            .load::<CompletedTorrent>(db)
            .unwrap()
    }
}

#[derive(Debug, Queryable)]
pub struct UserConnection {
    id: Uuid,
    user_agent: String,
    ip_address: IpNetwork,
    port: i32,
}

impl UserConnection {
    pub fn fetch_for_user(id: &Uuid, db: &PgConnection) -> Vec<UserConnection> {
        use schema::peers::dsl as p;
        peers::table
            .select((p::id, p::user_agent, p::ip_address, p::port))
            .filter(p::user_id.eq(id))
            .order_by(p::ip_address.asc())
            .load::<UserConnection>(db)
            .unwrap()
    }
}

impl Serialize for UserConnection {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut root = serializer.serialize_struct("user_connection", 4)?;
        root.serialize_field("id", &self.id)?;
        root.serialize_field("user_agent", &self.user_agent)?;
        root.serialize_field("ip_address", &self.ip_address.to_string())?;
        root.serialize_field("port", &self.port)?;
        root.end()
    }
}

#[derive(Debug, Serialize, Queryable)]
pub struct UserUpload {
    id: Uuid,
    user_id: Option<Uuid>,
    name: String,
    size: i64,
    seeder: i64,
    leecher: i64,
    created_at: Timestamp,
}

impl UserUpload {
    pub fn fetch_for_user(user_id: &Uuid, db: &PgConnection) -> Vec<UserUpload> {
        use schema::torrent_list::dsl as tl;
        torrent_list::table
            .select((
                tl::id,
                tl::user_id,
                tl::name,
                tl::size,
                tl::seeder,
                tl::leecher,
                tl::created_at,
            ))
            .distinct()
            .filter(tl::user_id.eq(user_id))
            .order_by(tl::created_at.desc())
            .load::<UserUpload>(db)
            .unwrap()
    }
}

/// Get the username for a user id
pub fn username(id: &Uuid, db: &PgConnection) -> Option<String> {
    lazy_static! {
        static ref USERS: Mutex<HashMap<Uuid, String>> = Mutex::new(HashMap::new());
    }

    let mut users = USERS.lock().unwrap();
    if let Some(name) = users.get(id) {
        return Some(name.to_owned());
    }

    let name: diesel::QueryResult<String> = users::table.select(users::name).filter(users::id.eq(id)).first(db);

    match name {
        Ok(name) => {
            users.insert(*id, name.clone());
            Some(name)
        }
        Err(_) => None,
    }
}

#[derive(Default)]
pub struct UserSettingsMsg {
    pub user: User,
    pub profile: UserProfile,
    pub properties: HashMap<String, Property>,
    pub categories: Vec<Category>,
}

impl UserSettingsMsg {
    pub fn new(user: User, profile: UserProfile, properties: Vec<Property>, categories: Vec<Category>) -> Self {
        let mut prop_map = HashMap::new();
        for prop in properties {
            prop_map.insert(prop.name.clone(), prop);
        }
        Self {
            user,
            profile,
            properties: prop_map,
            categories,
        }
    }
}

#[derive(Default, Serialize, Identifiable, Queryable, Insertable, Associations)]
#[belongs_to(User, foreign_key = "id")]
pub struct UserProfile {
    pub id: Uuid,
    pub avatar: Option<String>,
    pub flair: Option<String>,
    pub about: Option<String>,
}

impl UserProfile {
    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        self.insert_into(user_profiles::table)
            .on_conflict(on_constraint("user_profiles_pkey"))
            .do_update()
            .set(UpdateUserProfile::from(self))
            .execute(db)
            .map_err(|e| format!("failed to save profile: {}", e).into())
    }
}

#[derive(AsChangeset)]
#[table_name = "user_profiles"]
struct UpdateUserProfile<'profile> {
    avatar: Option<&'profile String>,
    flair: Option<&'profile String>,
    about: Option<&'profile String>,
}

impl<'profile> From<&'profile UserProfile> for UpdateUserProfile<'profile> {
    fn from(p: &'profile UserProfile) -> Self {
        Self {
            avatar: p.avatar.as_ref(),
            flair: p.flair.as_ref(),
            about: p.about.as_ref(),
        }
    }
}
