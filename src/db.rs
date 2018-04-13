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

use std::ops::Deref;
use diesel::pg::PgConnection;
use r2d2;
use r2d2_diesel::ConnectionManager;
use actix::prelude::*;
use SETTINGS;

/// An alias to the type for a pool of Diesel Postgres connections.
pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

/// Connection request guard type: a wrapper around an r2d2 pooled connection.
pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<PgConnection>>);

// For the convenience of using an &DbConn as an &PgConnection.
impl Deref for DbConn {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Database Executor
pub struct DbExecutor(pub Pool);

impl DbExecutor {
    /// Create a new Database Executor
    ///
    /// # Panics
    ///
    /// The method panics if the connection to the database could not be established
    pub fn new(pool: Pool) -> Self {
        DbExecutor(pool)
    }

    pub fn conn(&self) -> DbConn {
        DbConn(self.0.get().unwrap())
    }

    pub fn pg(&self) -> r2d2::PooledConnection<ConnectionManager<PgConnection>> {
        self.0.get().unwrap()
    }
}

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

/// Initializes a database pool.
pub fn init_pool() -> Pool {
    let database_url = SETTINGS.read().unwrap().database.url.to_owned();
    info!("starting new database pool for {}", database_url);

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::new(manager).expect("db pool")
}