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

pub fn authenticated(mut req: HttpRequest<State>) -> SyncResponse<Template> {
    debug!("AUTHENTICATED INDEX BITCHES!");
    match req.session().get::<Uuid>("user_id") {
        Ok(user_id) => match user_id {
            Some(user_id) => debug!("got a session / user_id = {:?}", user_id),
            None => debug!("got a session but no user_id"),
        },
        Err(e) => debug!("no session: {}", e),
    }
    let ctx = Context::new();
    Template::render(&req.state().template(), "index/authenticated.html", &ctx)
}


pub fn index(req: HttpRequest<State>) -> SyncResponse<Template> {
    let ctx = Context::new();
    Template::render(&req.state().template(), "index/public.html", &ctx)
}
