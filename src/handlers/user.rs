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

use chrono::Duration;
use diesel::QueryDsl;
use fast_chemail;
use image::DynamicImage;
use image::GenericImage;
use models::{
    user::{CompletedTorrent, Property, UserConnection, UserProfileMsg, UserSettingsMsg, UserTransfer, UserUpload},
    Category, Group, User,
};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::net::IpAddr;
use std::path::Path;
use tempfile::TempPath;

/// Load User Message
///
/// RequireUserMsg(user_id, update_last_active)
pub struct RequireUserMsg(pub Uuid, pub bool);

impl Message for RequireUserMsg {
    type Result = Result<models::User>;
}

impl Handler<RequireUserMsg> for DbExecutor {
    type Result = Result<models::User>;

    fn handle(&mut self, msg: RequireUserMsg, _ctx: &mut Self::Context) -> <Self as Handler<RequireUserMsg>>::Result {
        match models::User::find(&msg.0, &self.conn()) {
            Some(mut user) => {
                if msg.1 {
                    user.update_last_active(&self.conn())?;
                }
                Ok(user)
            }
            None => bail!("user not found"),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

impl Message for LoginForm {
    type Result = Result<User>;
}

impl Handler<LoginForm> for DbExecutor {
    type Result = Result<User>;

    fn handle(&mut self, msg: LoginForm, _: &mut Self::Context) -> <Self as Handler<LoginForm>>::Result {
        let conn = self.conn();

        let user = match User::find_by_name(&msg.username, &conn) {
            Some(user) => user,
            None => bail!("User not found"),
        };

        if user.status != models::user::STATUS_ACTIVE {
            bail!("User not active");
        }

        if user.verify_password(&msg.password) {
            Ok(user)
        } else {
            bail!("Wrong password")
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SignupForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    pub terms: String,
}

impl SignupForm {
    fn passwords_match(&self) -> bool {
        self.password == self.password_confirmation
    }

    fn username_unique(&self, conn: &DbConn) -> bool {
        match User::find_by_name(&self.username[..], conn) {
            Some(_) => false,
            None => true,
        }
    }

    fn email_unique(&self, conn: &DbConn) -> bool {
        match User::find_by_email(&self.email[..], conn) {
            Some(_) => false,
            None => true,
        }
    }

    fn username_valid(&self) -> bool {
        if self.username.len() < 4 {
            return false;
        }
        let re = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_\-]+$").unwrap();
        re.is_match(&self.username)
    }

    fn email_valid(&self) -> bool {
        fast_chemail::is_valid_email(&self.email)
    }

    fn password_valid(&self) -> bool {
        self.password.len() >= 8
    }

    pub fn is_valid(&self, conn: &DbConn) -> Result<bool> {
        if !self.username_valid() {
            bail!("username is invalid");
        }
        if !self.email_valid() {
            bail!("email address is invalid")
        }
        if !self.passwords_match() {
            bail!("passwords do not match");
        }
        if !self.password_valid() {
            bail!("password is invalid");
        }
        if !self.username_unique(conn) {
            bail!("username is already taken");
        }
        if !self.email_unique(conn) {
            bail!("email address is already taken");
        }

        Ok(true)
    }
}

impl Message for SignupForm {
    type Result = Result<String>;
}

impl Handler<SignupForm> for DbExecutor {
    type Result = Result<String>;

    fn handle(&mut self, msg: SignupForm, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();

        if msg.is_valid(&conn)? {
            let settings = match SETTINGS.read() {
                Ok(s) => s,
                Err(e) => bail!("failed to read settings: {}", e),
            };
            let gid = &settings.user.default_group;

            if let Some(group) = Group::find(gid, &conn) {
                let user = User::create(
                    &conn,
                    msg.username.clone(),
                    msg.email.clone(),
                    &msg.password[..],
                    &group,
                )?;
                user.create_message_folders(&conn)?;

                let confirm_id = user.create_confirm_id(&conn);

                Ok(util::to_hex(&confirm_id))
            } else {
                bail!("default group not found")
            }
        } else {
            bail!("invalid data")
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConfirmMsg {
    pub id: String,
    pub ip_address: IpAddr,
}

impl Message for ConfirmMsg {
    type Result = Result<User>;
}

impl Handler<ConfirmMsg> for DbExecutor {
    type Result = Result<User>;

    fn handle(&mut self, msg: ConfirmMsg, _: &mut Self::Context) -> <Self as Handler<ConfirmMsg>>::Result {
        let conn = self.conn();

        if let Some(property) = models::Property::find_by_name_value("confirm_id", msg.id, &conn) {
            if let Some(mut user) = models::User::find(property.user_id(), &conn) {
                user.status = models::user::STATUS_ACTIVE;
                user.ip_address = Some(msg.ip_address.into());

                match user.save(&conn) {
                    Ok(_) => match property.delete(&conn) {
                        Ok(_) => Ok(user),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                }
            } else {
                bail!("User not found")
            }
        } else {
            bail!("Confirm Id not found")
        }
    }
}

pub struct LoadUserStatsMsg(pub Uuid);

impl Message for LoadUserStatsMsg {
    type Result = Result<models::user::UserStatsMsg>;
}

impl Handler<LoadUserStatsMsg> for DbExecutor {
    type Result = Result<models::user::UserStatsMsg>;

    fn handle(
        &mut self,
        msg: LoadUserStatsMsg,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<LoadUserStatsMsg>>::Result {
        use schema::peers::dsl;
        let db: &PgConnection = &self.conn();
        match models::User::find(&msg.0, db) {
            Some(user) => {
                let ratio = if user.downloaded > 0 {
                    user.uploaded as f64 / user.downloaded as f64
                } else {
                    0f64
                };
                let uploads = schema::peers::table
                    .count()
                    .filter(dsl::user_id.eq(&user.id))
                    .filter(dsl::seeder.eq(true))
                    .first(db)
                    .unwrap();
                let downloads = schema::peers::table
                    .count()
                    .filter(dsl::user_id.eq(&user.id))
                    .filter(dsl::seeder.eq(false))
                    .first(db)
                    .unwrap();

                Ok(models::user::UserStatsMsg {
                    id: user.id,
                    name: user.name,
                    uploaded: user.uploaded,
                    downloaded: user.downloaded,
                    ratio,
                    uploads,
                    downloads,
                })
            }
            None => bail!("user not found"),
        }
    }
}

pub struct LoadUserProfileMsg(pub Uuid, pub Uuid, pub AclContainer);

impl Message for LoadUserProfileMsg {
    type Result = Result<models::user::UserProfileMsg>;
}

impl Handler<LoadUserProfileMsg> for DbExecutor {
    type Result = Result<models::user::UserProfileMsg>;

    fn handle(
        &mut self,
        msg: LoadUserProfileMsg,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<LoadUserProfileMsg>>::Result {
        let db: &PgConnection = &self.conn();
        match models::User::find(&msg.0, db) {
            Some(user) => {
                let acl = msg.2;

                let mut transfers = UserTransfer::fetch_for_user(&user.id, &db);
                let mut active_uploads: Vec<UserTransfer> = Vec::new();
                let mut active_downloads: Vec<UserTransfer> = Vec::new();
                for transfer in transfers {
                    if transfer.is_seeder() {
                        active_uploads.push(transfer);
                    } else {
                        active_downloads.push(transfer);
                    }
                }

                let completed = CompletedTorrent::fetch_for_user(&user.id, &db);
                let connections: Vec<UserConnection>;
                let may_view_passcode: bool;

                {
                    // get the current user
                    let _current_user;
                    let current_user = if msg.0 != msg.1 {
                        _current_user = models::User::find(&msg.1, db).unwrap();
                        &_current_user
                    } else {
                        &user
                    };

                    if user.id == msg.1
                        || acl.is_allowed(
                            &current_user.id,
                            &current_user.group_id,
                            "user#connections",
                            &Permission::Read,
                        ) {
                        connections = UserConnection::fetch_for_user(&user.id, &db);
                    } else {
                        connections = Vec::new();
                    }

                    may_view_passcode = user.id == msg.1
                        || acl.is_allowed(
                            &current_user.id,
                            &current_user.group_id,
                            "user#passcode",
                            &Permission::Read,
                        );
                }
                let uploads = UserUpload::fetch_for_user(&user.id, &db);
                let timezone = util::user::user_timezone(&msg.1, &db);

                Ok(UserProfileMsg {
                    user,
                    active_uploads,
                    active_downloads,
                    completed,
                    connections,
                    uploads,
                    timezone,
                    may_view_passcode,
                })
            }
            None => bail!("user not found"),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ActiveUsers {
    pub group_order: Vec<Uuid>,
    pub groups: HashMap<Uuid, Group>,
    pub user_list: HashMap<Uuid, Vec<(Uuid, String)>>,
}

pub struct ActiveUsersMsg(pub Duration);

impl Message for ActiveUsersMsg {
    type Result = Result<ActiveUsers>;
}

impl Handler<ActiveUsersMsg> for DbExecutor {
    type Result = Result<ActiveUsers>;

    fn handle(&mut self, msg: ActiveUsersMsg, _ctx: &mut Self::Context) -> <Self as Handler<ActiveUsersMsg>>::Result {
        use schema::groups::dsl as g;
        use schema::users::dsl as u;
        let db: &PgConnection = &self.conn();
        let date: DateTime<Utc> = Utc::now().checked_sub_signed(msg.0).unwrap();

        let res = schema::users::table
            .select((u::id, u::name, u::group_id))
            .filter(u::last_active.ge(&date))
            .order_by(u::group_id.desc())
            .then_order_by(u::name.asc())
            .load::<(Uuid, String, Uuid)>(db);

        let res_groups = schema::groups::table.order_by(g::parent_id.desc()).load::<Group>(db);

        let groups: Vec<Group> = match res_groups {
            Ok(groups) => groups,
            Err(e) => bail!("query failed: {}", e),
        };
        let users: Vec<(Uuid, String, Uuid)> = match res {
            Ok(users) => users,
            Err(e) => bail!("query failed: {}", e),
        };

        let mut group_order = Vec::with_capacity(groups.len());
        let mut set_groups = HashMap::with_capacity(groups.len());
        for group in groups {
            if group.parent_id.is_none() {
                group_order.push(group.id);
            } else if let Some(pid) = group.parent_id {
                match group_order.binary_search(&pid) {
                    Ok(index) => {
                        let index = index + 1;
                        group_order.insert(index, group.id);
                    }
                    Err(_) => {
                        group_order.push(group.id);
                    }
                }
            }
            set_groups.insert(group.id, group);
        }

        let mut active_users: HashMap<Uuid, Vec<(Uuid, String)>> = HashMap::new();
        for (uid, uname, gid) in users {
            if let Some(user_list) = active_users.get_mut(&gid) {
                user_list.push((uid, uname));
                continue;
            }

            active_users.insert(gid, vec![(uid, uname)]);
        }

        Ok(ActiveUsers {
            group_order,
            groups: set_groups,
            user_list: active_users,
        })
    }
}

pub struct LoadSettingsMsg(pub Uuid);

impl Message for LoadSettingsMsg {
    type Result = Result<UserSettingsMsg>;
}

impl Handler<LoadSettingsMsg> for DbExecutor {
    type Result = Result<UserSettingsMsg>;

    fn handle(&mut self, msg: LoadSettingsMsg, _: &mut Self::Context) -> <Self as Handler<LoadSettingsMsg>>::Result {
        let conn = self.conn();
        let user = User::find(&msg.0, &conn).ok_or_else(|| "user not found")?;
        let profile = user.profile(&conn);
        let properties = Property::fetch_for_user(&msg.0, &conn);
        let categories = Category::all(&conn);
        Ok(UserSettingsMsg::new(user, profile, properties, categories))
    }
}

#[derive(Default)]
pub struct UpdateProfileMsg {
    id: Uuid,
    avatar: Option<(String, TempPath)>,
    flair: Option<String>,
    about: Option<String>,
}

impl UpdateProfileMsg {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    /// Set the new avatar
    ///
    /// # Returns
    /// the previous avatar
    pub fn set_avatar(&mut self, avatar: Option<(String, TempPath)>) -> Option<(String, TempPath)> {
        let old = self.avatar.take();
        self.avatar = avatar;
        old
    }

    /// Set the new avatar
    ///
    /// # Returns
    /// the previous avatar
    pub fn set_avatar2(&mut self, name: String, file: TempPath) -> Option<(String, TempPath)> {
        let old = self.avatar.take();
        self.avatar = Some((name, file));
        old
    }

    /// Set the new flair
    ///
    /// # Returns
    /// the previous flair
    pub fn set_flair(&mut self, flair: Option<String>) -> Option<String> {
        let old = self.flair.take();
        self.flair = flair;
        old
    }

    /// Set the new about text
    ///
    /// # Returns
    /// the previous about text
    pub fn set_about(&mut self, about: Option<String>) -> Option<String> {
        let old = self.about.take();
        self.about = about;
        old
    }
}

#[derive(Default)]
pub struct UpdateUserSettingsMsg {
    id: Uuid,
    properties: Vec<Property>,
    profile: UpdateProfileMsg,
}

impl UpdateUserSettingsMsg {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            profile: UpdateProfileMsg::new(id),
            ..Default::default()
        }
    }

    pub fn profile(&self) -> &UpdateProfileMsg {
        &self.profile
    }

    pub fn profile_mut(&mut self) -> &mut UpdateProfileMsg {
        &mut self.profile
    }

    pub fn properties(&self) -> &Vec<Property> {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut Vec<Property> {
        &mut self.properties
    }

    pub fn push_property(&mut self, property: Property) {
        self.properties.push(property)
    }

    pub fn push_create_property<T: Serialize>(&mut self, name: String, value: T) {
        let property = Property::new(name, value, &self.id);
        self.push_property(property)
    }
}

impl Message for UpdateUserSettingsMsg {
    type Result = Result<UserSettingsMsg>;
}

impl Handler<UpdateUserSettingsMsg> for DbExecutor {
    type Result = Result<UserSettingsMsg>;

    fn handle(
        &mut self,
        mut msg: UpdateUserSettingsMsg,
        _: &mut Self::Context,
    ) -> <Self as Handler<UpdateUserSettingsMsg>>::Result {
        let conn = self.conn();
        let user = User::find(&msg.id, &conn).ok_or_else(|| "user not found")?;
        let mut properties = Property::fetch_for_user(&msg.id, &conn);
        let mut profile = user.profile(&conn);

        for new_property in msg.properties_mut().drain(..) {
            for old_property in &mut properties {
                if new_property.name() == old_property.name() {
                    old_property.set_value(new_property.value());
                    old_property.save(&conn)?;
                    break;
                }
            }
            new_property.save(&conn)?;
            properties.push(new_property);
        }

        profile.about = msg.profile_mut().about.take();
        profile.flair = msg.profile_mut().flair.take();
        if let Some((name, path)) = msg.profile_mut().avatar.take() {
            {
                let avatar = Avatar::new(&msg.profile().id, &name, &path);
                avatar.store()?;
            }
            profile.avatar = Some(name);
        }
        profile.save(&conn)?;

        let categories = Category::all(&conn);
        Ok(UserSettingsMsg::new(user, profile, properties, categories))
    }
}

struct Avatar<'a> {
    user_id: &'a Uuid,
    file_name: &'a str,
    path: &'a TempPath,
}

impl<'a> Avatar<'a> {
    pub fn new(user_id: &'a Uuid, file_name: &'a str, path: &'a TempPath) -> Self {
        Avatar {
            user_id,
            file_name,
            path,
        }
    }

    pub fn store(&self) -> Result<()> {
        self.store_image()?;
        self.store_thumbnail()
    }

    pub fn store_image(&self) -> Result<()> {
        let path = format!("webroot/aimg/{}", self.user_id);
        if fs::metadata(&path).is_err() {
            fs::create_dir(&path)?;
        }
        let path = format!("{}/{}", path, self.file_name);
        fs::copy(self.path, path)?;

        Ok(())
    }

    pub fn store_thumbnail(&self) -> Result<()> {
        let thumbnail = self.generate_thumbnail()?;
        let path = format!("webroot/aimg/{}/t{}", self.user_id, self.file_name);
        thumbnail.save(path)?;

        Ok(())
    }

    fn generate_thumbnail(&self) -> Result<DynamicImage> {
        let img = self.open_image()?;
        let width = SETTINGS.read().unwrap().user.avatar_thumbnail_width;
        //let height = self.calc_height(width, img.width(), img.height());

        let scaled = img.thumbnail(width, img.height());

        Ok(scaled)
    }

    fn open_image(&self) -> Result<DynamicImage> {
        let fin = File::open(self.path)?;
        let fin = BufReader::new(fin);

        let path = Path::new(self.file_name);
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .map_or("".to_string(), |s| s.to_ascii_lowercase());

        let format = match &ext[..] {
            "jpg" | "jpeg" => image::ImageFormat::JPEG,
            "png" => image::ImageFormat::PNG,
            "gif" => image::ImageFormat::GIF,
            "webp" => image::ImageFormat::WEBP,
            "tif" | "tiff" => image::ImageFormat::TIFF,
            "tga" => image::ImageFormat::TGA,
            "bmp" => image::ImageFormat::BMP,
            "ico" => image::ImageFormat::ICO,
            "hdr" => image::ImageFormat::HDR,
            "pbm" | "pam" | "ppm" | "pgm" => image::ImageFormat::PNM,
            format => bail!("Image format image/{:?} is not supported.", format),
        };

        image::load(fin, format).map_err(|e| format!("Image error: {}", e).into())
    }
}
