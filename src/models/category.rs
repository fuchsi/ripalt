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

//! Torrent category model

use super::*;
use super::schema::categories;

#[derive(Debug, Queryable, Insertable, AsChangeset, Identifiable, PartialEq, Serialize)]
#[table_name = "categories"]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Category {
    pub fn find(id: &Uuid, db: &PgConnection) -> Option<Self> {
        use self::categories::dsl;

        dsl::categories.find(id).first::<Category>(db).ok()
    }

    pub fn all(db: &PgConnection) -> Vec<Self> {
        use self::categories::dsl;

        dsl::categories
            .order(dsl::name.asc())
            .load::<Self>(db)
            .unwrap_or_default()
    }
}

impl Default for Category {
    fn default() -> Self {
        Category{
            id: Default::default(),
            name: Default::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}