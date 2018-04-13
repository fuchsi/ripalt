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

use std::env;
use config::{ConfigError, Config, File, Environment};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub default_group: Uuid,
    pub passcode_length: usize,
}

#[derive(Debug, Deserialize)]
pub struct Email {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct Tracker {
    pub announce_url: String,
    pub comment: String,
    pub default_numwant: u16,
    pub interval: u16,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub debug: bool,
    pub session_name: String,
    pub session_secret: String,
    pub session_strict: bool,
    pub domain: String,
    pub https: bool,
    pub bind: String,
    pub database: Database,
    pub user: User,
    pub email: Email,
    pub tracker: Tracker,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        // Start off by merging in the "default" configuration file
        s.merge(File::with_name("config/ripalt"))?;

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional_
        let env = env::var("RUN_MODE").unwrap_or_else(|_|"development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        s.merge(File::with_name("config/local").required(false))?;

        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        s.merge(Environment::with_prefix("app"))?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into()
    }
}