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
use std::convert::TryFrom;

pub mod torrent;
pub mod user;

#[derive(Debug, Clone)]
pub struct UserSubjectMsg {
    uid: Uuid,
    gid: Uuid,
    acl: AclContainer,
}

impl UserSubjectMsg {
    pub fn new(uid: Uuid, gid: Uuid, acl: AclContainer) -> Self {
        Self { uid, gid, acl }
    }
}

impl<'req> TryFrom<&'req mut HttpRequest<State>> for UserSubjectMsg {
    type Error = Error;

    fn try_from(req: &mut HttpRequest<State>) -> Result<Self> {
        let (uid, gid) = match session_creds(req) {
            Some((u, g)) => (u, g),
            None => bail!("session credentials not available"),
        };
        let acl = req.state().acl_arc();
        Ok(Self::new(uid, gid, acl))
    }
}

impl<'a> From<&'a UserSubjectMsg> for UserSubject<'a> {
    fn from(msg: &UserSubjectMsg) -> UserSubject {
        UserSubject::new(&msg.uid, &msg.gid, Arc::clone(&msg.acl))
    }
}
