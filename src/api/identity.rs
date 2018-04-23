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

use actix_web::middleware::identity::{Identity, IdentityPolicy};
use actix_web::middleware::Response;
use jwt::{decode, Validation};
use std::rc::Rc;

pub struct ApiIdentityPolicy(Rc<ApiIdentityInner>);

impl ApiIdentityPolicy {
    pub fn new(key: &[u8]) -> ApiIdentityPolicy {
        ApiIdentityPolicy(Rc::new(ApiIdentityInner::new(key)))
    }
}

impl<S> IdentityPolicy<S> for ApiIdentityPolicy {
    type Identity = ApiIdentity;
    type Future = FutureResult<ApiIdentity, actix_web::Error>;

    fn from_request(&self, request: &mut HttpRequest<S>) -> Self::Future {
        let identity = self.0.load(request);
        if identity.is_some() {
            FutOk(ApiIdentity { identity })
        } else {
            FutErr(actix_web::error::ErrorUnauthorized("unauthorized"))
        }
    }
}

pub struct ApiIdentity {
    identity: Option<String>,
}

impl Identity for ApiIdentity {
    fn identity(&self) -> Option<&str> {
        self.identity.as_ref().map(|s| s.as_ref())
    }

    fn remember(&mut self, key: String) {
        warn!("Identity::remember is not available");
        self.identity = Some(key);
    }

    fn forget(&mut self) {
        warn!("Identity::forget is not available");
        self.identity = None;
    }

    fn write(&mut self, resp: HttpResponse) -> actix_web::error::Result<Response> {
        warn!("Identity::write is not available");

        Ok(Response::Done(resp))
    }
}

struct ApiIdentityInner {
    key: Vec<u8>,
}

impl ApiIdentityInner {
    fn new(key: &[u8]) -> ApiIdentityInner {
        ApiIdentityInner { key: key.to_vec() }
    }

    fn load<S>(&self, req: &mut HttpRequest<S>) -> Option<String> {
        if let Ok(user_id) = req.session().get::<String>("user_id") {
            return user_id;
        }
        if let Some(header) = req.headers().get("authorization") {
            if let Ok(header) = header.to_str() {
                if header.to_lowercase().starts_with("bearer") {
                    let validation = Validation::default();
                    let token_data = match decode::<Claims>(&header[7..], &self.key, &validation) {
                        Ok(c) => c,
                        Err(_) => return None,
                    };
                    let user_id = token_data.claims.user_id.clone();
                    return Some(user_id);
                }
            }
        }
        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    iat: i64,
    user_id: String,
}
