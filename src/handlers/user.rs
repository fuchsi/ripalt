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

use std::net::{IpAddr};
use models::{Group, User, user::{UserProfileMsg, UserTransfer, CompletedTorrent, UserConnection, UserUpload}};
use diesel::QueryDsl;
use regex::Regex;
use fast_chemail;

pub struct RequireUser(pub Uuid);

impl RequireUser {
    pub fn new(id: Uuid) -> Self {
        RequireUser(id)
    }
}

impl Message for RequireUser {
    type Result = Result<models::User>;
}

impl Handler<RequireUser> for DbExecutor {
    type Result = Result<models::User>;

    fn handle(&mut self, msg: RequireUser, _ctx: &mut Self::Context) -> <Self as Handler<RequireUser>>::Result {
        match models::User::find(&msg.0, &self.conn()) {
            Some(user) => Ok(user),
            None => bail!("user not found"),
        }
    }
}

#[derive(Debug, Deserialize)]
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

    pub fn is_valid(&self, conn: &DbConn) -> Result<()> {
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

        Ok(())
    }
}

impl Message for SignupForm {
    type Result = Result<String>;
}

impl Handler<SignupForm> for DbExecutor {
    type Result = Result<String>;

    fn handle(&mut self, msg: SignupForm, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();

        match msg.is_valid(&conn) {
            Ok(_) => {
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

                    let confirm_id = user.create_confirm_id(&conn);

                    Ok(util::to_hex(&confirm_id))
                } else {
                    bail!("default group not found")
                }
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Confirm {
    pub id: String,
    pub ip_address: IpAddr,
}

impl Message for Confirm {
    type Result = Result<User>;
}

impl Handler<Confirm> for DbExecutor {
    type Result = Result<User>;

    fn handle(&mut self, msg: Confirm,  _: &mut Self::Context) -> <Self as Handler<Confirm>>::Result {
        let conn = self.conn();

        if let Some(property) = models::Property::get_from_name_value("confirm_id", msg.id, &conn) {
            if let Some(mut user) = models::User::find(&property.user_id, &conn) {
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

pub struct UserStats(pub Uuid);

impl Message for UserStats {
    type Result = Result<models::user::UserStatsMsg>;
}

impl Handler<UserStats> for DbExecutor {
    type Result = Result<models::user::UserStatsMsg>;

    fn handle(&mut self, msg: UserStats, _ctx: &mut Self::Context) -> <Self as Handler<UserStats>>::Result {
        use schema::peers::dsl;
        let db: &PgConnection = &self.conn();
        match models::User::find(&msg.0, db) {
            Some(user) => {
                let ratio = user.uploaded as f64 / user.downloaded as f64;
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

                Ok(models::user::UserStatsMsg{
                    id: user.id,
                    name: user.name,
                    uploaded: user.uploaded,
                    downloaded: user.downloaded,
                    ratio,
                    uploads,
                    downloads,
                })
            },
            None => bail!("user not found"),
        }
    }
}

pub struct UserProfile(pub Uuid, pub Uuid, pub AclContainer);

impl Message for UserProfile {
    type Result = Result<models::user::UserProfileMsg>;
}

impl Handler<UserProfile> for DbExecutor {
    type Result = Result<models::user::UserProfileMsg>;

    fn handle(&mut self, msg: UserProfile, _ctx: &mut Self::Context) -> <Self as Handler<UserProfile>>::Result {
        let db: &PgConnection = &self.conn();
        match models::User::find(&msg.0, db) {
            Some(user) => {
                let acl = msg.2.read().unwrap();

                let mut transfers = UserTransfer::find_for_user(&user.id, &db);
                let mut active_uploads: Vec<UserTransfer> = Vec::new();
                let mut active_downloads: Vec<UserTransfer> = Vec::new();
                for transfer in transfers {
                    if transfer.is_seeder {
                        active_uploads.push(transfer);
                    } else {
                        active_downloads.push(transfer);
                    }
                }
                let completed = CompletedTorrent::find_for_user(&user.id, &db);
                let connections =
                {
                    let _current_user;
                    let current_user = if msg.0 != msg.1 {
                        _current_user = models::User::find(&msg.1, db).unwrap();
                        &_current_user
                    } else {
                        &user
                    };
                    if user.id == msg.1 || acl.is_allowed(current_user, "user#connections", &AclPermission::Read, &db) {
                        UserConnection::find_for_user(&user.id, &db)
                    } else {
                        Vec::new()
                    }
                };
                let uploads = UserUpload::find_for_user(&user.id, &db);

                Ok(UserProfileMsg{
                    user,
                    active_uploads,
                    active_downloads,
                    completed,
                    connections,
                    uploads,
                })
            },
            None => bail!("user not found"),
        }
    }
}