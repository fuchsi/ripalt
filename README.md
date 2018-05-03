ripalt: An Anti-Leech Torrent Tracker
=====================================

## Overview

ripalt is a private **Bittorrent Tracker CMS** based on [actix-web](https://github.com/actix/actix-web)

### Features
Nothing special yet. Plain old Torrent tracking.

## Requirements

- Rust nightly (2018-04-29)
- PostgreSQL

**Additional Build Requirements**

- [libsass](https://github.com/sass/libsass)

### Browser Requirements

- Any Browser which supports ES6.
    - That means no Internet Explorer.

## Installation

### Get the source and compile it

Clone the repositry
```bash
git clone https://github.com/fuchsi/ripalt.git
cd ripalt
git submodule update --init
```
Build ripalt
```bash
cargo build
# or for release builds (might take a while)
cargo build --release
```

### Setup

Create a `.env` file in the ripalt directory.
```
RUST_LOG="actix=warn,actix_web=info,ripalt=info"
DATABASE_URL="postgres://user:password@localhost/database"
```
The file should at least contain the `DATABASE_URL`.

Copy the `config/ripalt.toml.example` to `config/ripalt.toml` and change it to your needs and settings.

Install [diesel_cli](https://github.com/diesel-rs/diesel/tree/master/diesel_cli) and apply the migrations
```bash
cargo install diesel_cli --no-default-features --features "postgres"
diesel migration run
```

Initialize the categories and groups:
```bash
psql user database -f doc/sql/categories.sql
psql user database -f doc/sql/groups.sql
```
Substitute `user` and `database` for your chosen settings.

**Note:** At the current state there is no initialization for additional tracker data, such as
- ACL
- Default User(s)

You'll have to create them on your own **and** update the `ripalt.toml` file for the created data.

## Usage

Run ripalt
```bash
cargo run ripalt
# or
target/debug/ripalt
# or for release builds
target/release/ripalt
```
and navigate your Browser to http://localhost:8081, or whatever you set in the config.

## Documentation
coming soon™

## SemVer
This project follows SemVer only for the public API, public API here meaning the API endpoints appearing the the docs.

