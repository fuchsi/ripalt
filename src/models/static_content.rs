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
use markdown;
use schema::static_content;

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Serialize)]
#[table_name = "static_content"]
pub struct Content {
    pub id: String,
    pub title: String,
    pub content: String,
    pub content_type: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Content {
    pub fn new(
        id: String,
        title: String,
        content: String,
        content_type: String,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Self {
        Content {
            id,
            title,
            content,
            content_type,
            created_at,
            updated_at,
        }
    }

    pub fn find(id: &str, db: &PgConnection) -> Option<Self> {
        static_content::table.find(id).first::<Self>(db).ok()
    }

    pub fn render(&self) -> String {
        if self.content_type == "text/markdown" {
            return markdown::to_html(&self.content);
        }

        self.content.clone()
    }

    pub fn save(&self, db: &PgConnection) -> Result<usize> {
        diesel::update(self)
            .set(self)
            .execute(db)
            .map_err(|e| format!("failed content: {}", e).into())
    }
}
