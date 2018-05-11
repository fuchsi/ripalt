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

use std::collections::HashMap;
use std::fmt::Write;

use markdown;
use tera::{from_value, Result, Value};

use tera::GlobalFn;
use util;

pub fn data_size(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Number(number) => {
            let bytes = number.as_f64().unwrap_or_else(|| 0f64);
            Ok(Value::String(util::data_size(bytes)))
        }
        Value::Null => Ok(Value::String(util::data_size(0))),
        _ => Err("not a number".into()),
    }
}

pub fn format_date(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    static FORMAT_STRING: &'static str = "%d.%m.%Y %H:%M:%S";
    static FORMAT_STRING_UTC: &'static str = "%d.%m.%Y %H:%M:%S UTC";
    let date = match value {
        Value::String(s) => match DateTime::parse_from_rfc3339(&s[..]).map_err(|e| format!("not a date string: {}", e))
        {
            Ok(date) => date.with_timezone(&Utc),
            Err(_) => return Ok(Value::String(s)),
        },
        Value::Null => {
            return Ok(Value::String(String::from("---")));
        }
        _ => return Err("not a date time string".into()),
    };

    if let Some(Value::Number(timezone)) = args.get("timezone") {
        if let Some(timezone) = timezone.as_i64() {
            let local = date.with_timezone(&FixedOffset::east(timezone as i32));
            return Ok(Value::String(local.format(FORMAT_STRING).to_string()));
        }
    }

    Ok(Value::String(date.format(FORMAT_STRING_UTC).to_string()))
}

pub fn duration(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Number(number) => {
            let mut seconds = number.as_f64().unwrap_or_else(|| 0f64);
            let duration = {
                let mut dur_str = String::new();
                let days = seconds / 86400f64;

                if days >= 1f64 {
                    write!(&mut dur_str, "{:.0}d ", days).map_err(|e| format!("{}", e))?;
                    seconds %= 86400f64;
                }
                let hours = seconds / 3600f64;
                if hours >= 1f64 {
                    write!(&mut dur_str, "{:02.0}:", hours).map_err(|e| format!("{}", e))?;
                    seconds %= 3600f64;
                }
                let minutes = seconds / 60f64;
                if minutes >= 1f64 || hours >= 1f64 {
                    write!(&mut dur_str, "{:02.0}:", minutes).map_err(|e| format!("{}", e))?;
                }
                seconds %= 60f64;
                if seconds >= 1f64 || hours >= 1f64 || minutes >= 1f64 {
                    write!(&mut dur_str, "{:02.0}", seconds).map_err(|e| format!("{}", e))?;
                }

                dur_str.trim_right().trim_right_matches(':').to_owned()
            };
            Ok(Value::String(duration.to_string()))
        }
        Value::Null => Ok(Value::String("0s".to_string())),
        _ => Ok(Value::String("0s".to_string())),
    }
}

pub fn markdown(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::String(s) => Ok(Value::String(markdown::to_html(&s))),
        _ => bail!("markdown: not a string"),
    }
}

pub fn quote(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::String(s) => Ok(Value::String(
            s.lines()
                .map(|line| format!("> {}", line))
                .fold(String::new(), |acc, x| format!("{}\n{}", acc, x)),
        )),
        _ => bail!("quote: not a string"),
    }
}

pub fn is_allowed(acl: AclContainer) -> GlobalFn {
    Box::new(move |args| -> Result<Value> {
        let (user_id, group_id) = match args.get("user") {
            Some(val) => match val {
                Value::Object(user) => {
                    let user_id = match user.get("id") {
                        Some(val) => match from_value::<Uuid>(val.clone()) {
                            Ok(id) => id,
                            Err(e) => bail!("invalid user id: {}", e),
                        },
                        None => bail!("no user id"),
                    };
                    let group_id = match user.get("group_id") {
                        Some(val) => match from_value::<Uuid>(val.clone()) {
                            Ok(id) => id,
                            Err(e) => bail!("invalid group id: {}", e),
                        },
                        None => bail!("no group id"),
                    };
                    (user_id, group_id)
                }
                _ => bail!("invalid user object"),
            },
            None => {
                let user_id = match args.get("user_id") {
                    Some(val) => match from_value::<Uuid>(val.clone()) {
                        Ok(id) => id,
                        Err(e) => bail!("invalid user id: {}", e),
                    },
                    None => bail!("no user id"),
                };
                let group_id = match args.get("group_id") {
                    Some(val) => match from_value::<Uuid>(val.clone()) {
                        Ok(id) => id,
                        Err(e) => bail!("invalid group id: {}", e),
                    },
                    None => bail!("no group id"),
                };
                (user_id, group_id)
            }
        };

        let ns = match args.get("ns") {
            Some(val) => match from_value::<String>(val.clone()) {
                Ok(v) => v,
                Err(e) => bail!("invalid namespace: {}", e),
            },
            None => bail!("no namespace"),
        };
        let perm = match args.get("perm") {
            Some(val) => match from_value::<String>(val.clone()) {
                Ok(v) => v,
                Err(e) => bail!("invalid permission: {}", e),
            },
            None => "read".to_string(),
        };
        let perm = Permission::from(perm);

        Ok(Value::Bool(acl.is_allowed(&user_id, &group_id, &ns, &perm)))
    })
}
