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

#[derive(Debug)]
pub struct NewTorrent {
    pub name: String,
    pub description: String,
    pub meta_file: Vec<u8>,
    pub nfo_file: Vec<u8>,
    //    pub image_file: Option<Vec<u8>>,
    pub category: Uuid,
    pub user: Uuid,
    pub info_hash: Vec<u8>,
    pub size: i64,
    pub files: Vec<NewFile>,
}

impl NewTorrent {
    fn insert_meta(&self, id: &Uuid, conn: &DbConn) -> Result<usize> {
        let meta = models::torrent::NewTorrentMetaFile{
            id,
            data: &self.meta_file,
        };

        meta.create(&conn)
    }

    fn insert_files(&self, id: &Uuid, conn: &DbConn) -> Result<()> {
        for f in &self.files {
            let file_id = Uuid::new_v4();
            let file = models::torrent::NewTorrentFile{
                id: &file_id,
                torrent_id: id,
                file_name: &f.file_name[..],
                size: f.size
            };

            file.create(&conn)?;
        }

        Ok(())
    }

    fn insert_nfo(&self, id: &Uuid, conn: &DbConn) -> Result<usize> {
        let nfo_id = Uuid::new_v4();
        let nfo = models::torrent::NewTorrentNFO{
            id: &nfo_id,
            torrent_id: id,
            data: &self.nfo_file,
        };

        nfo.create(&conn)
    }
}

impl Message for NewTorrent {
    type Result = Result<models::Torrent>;
}

impl Handler<NewTorrent> for DbExecutor {
    type Result = Result<models::Torrent>;

    fn handle(
        &mut self,
        msg: NewTorrent,
        _: &mut Self::Context,
    ) -> <Self as Handler<NewTorrent>>::Result {
        let conn = self.conn();

        let _category = match models::category::Category::find(&msg.category, &conn) {
            Some(c) => c,
            None => bail!("category not found"),
        };

        let torrent = models::Torrent::create(
            &msg.name,
            &msg.description,
            &msg.info_hash,
            &msg.category,
            &msg.user,
            msg.size,
            &conn,
        )?;

        msg.insert_meta(&torrent.id, &conn)?;
        msg.insert_files(&torrent.id, &conn)?;
        msg.insert_nfo(&torrent.id, &conn)?;

        Ok(torrent)
    }
}



#[derive(Debug, Default)]
pub struct NewTorrentBuilder {
    name: String,
    description: String,
    meta_file: Vec<u8>,
    nfo_file: Vec<u8>,
    category: Uuid,
    user: Uuid,
}

impl NewTorrentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name<T>(&mut self, name: &T) -> &Self
    where
        T: ToString,
    {
        self.name = name.to_string();
        self
    }

    pub fn description<T>(&mut self, desc: &T) -> &Self
    where
        T: ToString,
    {
        self.description = desc.to_string();
        self
    }

    pub fn category(&mut self, category: Uuid) -> &Self {
        self.category = category;
        self
    }

    pub fn user(&mut self, user: Uuid) -> &Self {
        self.user = user;
        self
    }

    pub fn raw_meta(&mut self, meta_file: Vec<u8>) -> &Self {
        assert!(!meta_file.is_empty(), "meta file is empty");
        self.meta_file = meta_file;
        self
    }

    pub fn nfo(&mut self, nfo_file: Vec<u8>) -> &Self {
        assert!(!nfo_file.is_empty(), "nfo_file is empty");
        self.nfo_file = nfo_file;
        self
    }

    pub fn finish(self) -> Result<NewTorrent> {
        let info_hash = util::torrent::info_hash(&self.meta_file)?;
        let files: Vec<NewFile> = util::torrent::files(&self.meta_file)?
            .into_iter()
            .map(|(file_name, size)| NewFile { file_name, size })
            .collect();
        let size = files.iter().fold(0i64, |acc, ref x| acc + x.size);

        Ok(NewTorrent {
            name: self.name,
            description: self.description,
            category: self.category,
            user: self.user,
            meta_file: self.meta_file,
            nfo_file: self.nfo_file,
            size,
            info_hash,
            files,
        })
    }
}

pub struct Categories {}

impl Message for Categories {
    type Result = Result<Vec<models::Category>>;
}

impl Handler<Categories> for DbExecutor {
    type Result = Result<Vec<models::Category>>;

    fn handle(
        &mut self,
        _msg: Categories,
        _: &mut Self::Context,
    ) -> <Self as Handler<Categories>>::Result {
        use schema::categories::dsl;
        let conn = self.conn();
        let db: &PgConnection = &conn;

        dsl::categories
            .order(dsl::name.asc())
            .load::<models::Category>(db)
            .chain_err(|| "failed to load categories")
    }
}

#[derive(Debug)]
pub struct NewFile {
    file_name: String,
    size: i64,
}

pub struct LoadTorrent {
    id: Uuid,
    user_id: Uuid,
}

impl LoadTorrent {
    pub fn new(id: &Uuid, user_id: &Uuid) -> LoadTorrent {
        LoadTorrent{
            id: id.clone(),
            user_id: user_id.clone(),
        }
    }
}

impl Message for LoadTorrent {
    type Result = Result<models::torrent::TorrentMsg>;
}

impl Handler<LoadTorrent> for DbExecutor {
    type Result = Result<models::torrent::TorrentMsg>;

    fn handle(&mut self, msg: LoadTorrent, _: &mut Self::Context) -> <Self as Handler<LoadTorrent>>::Result {
        let conn = self.conn();
        let torrent = models::torrent::TorrentMsg::find(&msg.id, &conn);
        torrent.map(|mut t| {
            t.timezone = util::user::user_timezone(&msg.user_id, &conn);
            t
        })
    }
}


pub struct LoadTorrentMeta {
    pub id: Uuid,
    pub uid: Uuid,
}

impl Message for LoadTorrentMeta {
    type Result = Result<(String, Vec<u8>, Vec<u8>)>;
}

impl Handler<LoadTorrentMeta> for DbExecutor {
    type Result = Result<(String, Vec<u8>, Vec<u8>)>;

    fn handle(&mut self, msg: LoadTorrentMeta, _: &mut Self::Context) -> <Self as Handler<LoadTorrentMeta>>::Result {
        let conn = self.conn();
        let torrent = models::torrent::Torrent::find(&msg.id, &conn).ok_or("torrent not found")?;
        let meta_file = models::torrent::TorrentMetaFile::find(&msg.id, &conn).ok_or("meta file not found")?;
        let passcode = models::User::find(&msg.uid, &conn).ok_or("user not found")?.passcode;
        let name = format!("{}.torrent", torrent.name);

        Ok((name, meta_file.data, passcode))
    }
}


#[derive(Debug)]
pub enum Visible {
    Visible,
    Invisible,
    All,
}

impl Default for Visible {
    fn default() -> Self {
        Visible::Visible
    }
}

impl ToString for Visible {
    fn to_string(&self) -> String {
        match self {
            Visible::Visible => String::from("visible"),
            Visible::Invisible => String::from("dead"),
            Visible::All => String::from("all"),
        }
    }
}

#[derive(Default, Debug)]
pub struct LoadTorrentList {
    pub name: Option<String>,
    pub category: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub visible: Visible,
    pub page: i64,
    pub per_page: i64,
    pub current_user_id: Uuid,
}

impl LoadTorrentList {
    pub fn new(user_id: &Uuid) -> Self {
        LoadTorrentList {
            page: 1,
            per_page: 25,
            current_user_id: user_id.clone(),
            ..Default::default()
        }
    }

    pub fn name(&mut self, name: String) -> &Self {
        self.name = Some(name);
        self
    }

    pub fn category(&mut self, category_id: Uuid) -> &Self {
        self.category = Some(category_id);
        self
    }

    pub fn user(&mut self, user_id: Uuid) -> &Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn visible(&mut self, visible: Visible) -> &Self {
        self.visible = visible;
        self
    }

    pub fn page(&mut self, page: i64, per_page: i64) -> &Self {
        self.page = page;
        self.per_page = per_page;
        self
    }

    pub fn query(&self, db: &PgConnection) -> (Vec<models::TorrentList>, i64) {
        use schema::torrent_list::dsl;
        let mut query = dsl::torrent_list.into_boxed();
        let mut query2 = dsl::torrent_list.into_boxed();

        if let Some(name) = &self.name {
            query = query.filter(dsl::name.ilike(format!("%{}%", name)));
            query2 = query2.filter(dsl::name.ilike(format!("%{}%", name)));
        }
        if let Some(category) = &self.category {
            query = query.filter(dsl::category_id.eq(category));
            query2 = query2.filter(dsl::category_id.eq(category));
        }
        if let Some(user_id) = &self.user_id {
            query = query.filter(dsl::user_id.eq(user_id));
            query2 = query2.filter(dsl::user_id.eq(user_id));
        }
        match self.visible {
            Visible::Visible => {
                query = query.filter(dsl::visible.eq(true));
                query2 = query2.filter(dsl::visible.eq(true));
            }
            Visible::Invisible => {
                query = query.filter(dsl::visible.eq(false));
                query2 = query2.filter(dsl::visible.eq(false));
            }
            Visible::All => {}
        }

        debug!("query: {}", diesel::debug_query::<diesel::pg::Pg, _>(&query));

        let count = query2.count().get_result(db).unwrap();
        let list = query
            .limit(self.per_page)
            .offset((self.page - 1) * self.per_page)
            .load::<models::TorrentList>(db);

        (list.unwrap_or(Vec::new()), count)
    }
}

#[derive(Debug, Default)]
pub struct TorrentListMsg {
    pub torrents: Vec<models::TorrentList>,
    pub count: i64,
    pub request: LoadTorrentList,
    pub timezone: i32,
}

impl Message for LoadTorrentList {
    type Result = Result<TorrentListMsg>;
}

impl Handler<LoadTorrentList> for DbExecutor {
    type Result = Result<TorrentListMsg>;

    fn handle(&mut self, msg: LoadTorrentList, _: &mut Self::Context) -> <Self as Handler<LoadTorrentList>>::Result {
        let db = self.conn();
        let (list, count) = msg.query(&db);
        let timezone = util::user::user_timezone(&msg.current_user_id, &db);
        Ok(TorrentListMsg{
            torrents: list,
            count,
            request: msg,
            timezone,
        })
    }
}