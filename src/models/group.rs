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

//! Use group model

use super::*;
use super::schema::groups;

#[derive(Queryable, Debug, Identifiable, Associations, PartialEq, Insertable, AsChangeset, Serialize)]
#[table_name = "groups"]
#[belongs_to(Group, foreign_key = "parent_id")]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Default for Group {
    fn default() -> Self {
        Group {
            id: Default::default(),
            name: Default::default(),
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Group {
    /// Find n Group by its id
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Group> {
        groups::dsl::groups.find(id).first::<Group>(db).ok()
    }
}