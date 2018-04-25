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

use tera;

// Create the Error, ErrorKind, ResultExt, and Result types
error_chain! {
    links {
        Tera(tera::Error, tera::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        Fmt(::std::fmt::Error);
        Utf8(::std::string::FromUtf8Error);
        SerdeBencode(::serde_bencode::Error);
        ParseInt(::std::num::ParseIntError);
        ParseBool(::std::str::ParseBoolError);
        ActixWebToStr(::actix_web::http::header::ToStrError);
        DataEncodingDecode(::data_encoding::DecodeError);
        ParseUuid(::uuid::ParseError);
    }

    errors {
        SettingsPoison(t: String) {
            description("settings are poisoned")
            display("settings are poisoned: {}", t)
        }
    }
}

impl Into<::actix_web::Error> for Error {
    fn into(self) -> ::actix_web::Error {
        use actix_web::error;

        error::ErrorInternalServerError(format!("{}", self))
    }
}

impl Into<Box<::futures::Future<Item=::actix_web::HttpResponse, Error=::actix_web::Error>>> for Error {
    fn into(self) -> Box<::futures::Future<Item=::actix_web::HttpResponse, Error=::actix_web::Error>> {
        use futures::future::err;
        use actix_web::error;

        Box::new(err(error::ErrorInternalServerError(format!("{}", self))))
    }
}

impl From<::actix_web::error::PayloadError> for Error {
    fn from(e: ::actix_web::error::PayloadError) -> Error {
        format!("Payload error: {}", e).into()
    }
}