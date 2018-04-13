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

use actix_web::middleware::{csrf, DefaultHeaders, Logger};

pub fn build(db: Addr<Syn, DbExecutor>, acl: Arc<RwLock<Acl>>) -> App<State> {
    App::with_state(State::new(db, acl))
        .middleware(Logger::default())
        .middleware(DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
        .middleware(csrf::CsrfFilter::new().allow_xhr())
        .prefix("/api")
        .default_resource(|r| r.method(Method::GET).h(NormalizePath::default()))
}