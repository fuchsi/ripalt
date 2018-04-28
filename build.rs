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
extern crate sass_rs;
extern crate walkdir;

use walkdir::{DirEntry, WalkDir};
use std::env;
use std::io::prelude::*;
use std::fs::File;

fn main() {
    println!("cargo:rerun-if-changed=assets");
    compile_scss();
}

fn compile_scss() {
    let mut options = sass_rs::Options::default();
    options.output_style = sass_rs::OutputStyle::Compact;
    if env::var("PROFILE").unwrap() == "release" {
        options.output_style = sass_rs::OutputStyle::Compressed;
    }

    let out_path = "webroot/static/css";

    let walker = WalkDir::new("assets/scss").into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_str().unwrap();
        if file_name.ends_with(".scss") {
            let options = options.clone();
            match sass_rs::compile_file(entry.path(), options) {
                Ok(css) => {
                    let file_name = &file_name[0..file_name.len() - 5];
                    let mut file = File::create(format!("{}/{}.css", out_path, file_name));
                    let buf = css.as_bytes();
                    println!("Compiling assets/scss/{}.scss", file_name);
                    match file {
                        Ok(mut file) => match file.write(buf) {
                            Ok(n) => if n < buf.len() {
                                eprintln!("compile_scss:{}: incomplete write", file_name);
                            },
                            Err(e) => eprintln!("compile_scss:{}: io error: {}", file_name, e),
                        },
                        Err(e) => eprintln!("compile_scss:{}: could not create target file: {}", file_name, e),
                    }
                },
                Err(e) => eprintln!("compile_scss:{}: sass error: {}", file_name, e),
            }
        }
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}