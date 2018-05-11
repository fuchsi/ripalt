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

use diesel::PgConnection;
use models::user::Property;
use uuid::Uuid;

use SETTINGS;

pub fn user_timezone(user_id: &Uuid, db: &PgConnection) -> i32 {
    if let Some(prop) = Property::find(user_id, "timezone", db) {
        if let Some(number) = prop.value().as_i64() {
            return number as i32;
        }
    }

    SETTINGS.read().unwrap().user.default_timezone
}
