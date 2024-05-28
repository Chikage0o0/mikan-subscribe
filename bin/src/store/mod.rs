use redb::Error;
use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

const DB_PATH: &str = "config/ms.db";

mod anime;
mod download;
mod onedrive;
mod subscribe;

static DB: OnceLock<Arc<Db>> = OnceLock::new();
static SUBSCRIBE: OnceLock<Arc<subscribe::Subscribe>> = OnceLock::new();
static DOWNLOAD: OnceLock<Arc<download::Tasks>> = OnceLock::new();
static ONEDRIVE: OnceLock<Arc<onedrive::Onedrive>> = OnceLock::new();
static ANIME: OnceLock<Arc<anime::Anime>> = OnceLock::new();

#[derive(Debug)]
pub struct Db(redb::Database);

impl Db {
    pub fn db() -> Result<Arc<Self>, Error> {
        if let Some(db) = DB.get() {
            return Ok(db.clone());
        }
        let db = redb::Database::create(DB_PATH)?;
        let db = Arc::new(Self(db));
        DB.set(db.clone()).unwrap();
        Ok(db)
    }

    pub fn get_subscribe() -> Result<Arc<subscribe::Subscribe>, Error> {
        if let Some(subscribe) = SUBSCRIBE.get() {
            Ok(subscribe.clone())
        } else {
            let db = Self::db()?;
            let subscribe = Arc::new(subscribe::Subscribe(db));
            subscribe.init()?;
            SUBSCRIBE.set(subscribe.clone()).unwrap();
            Ok(subscribe)
        }
    }

    pub fn get_download() -> Result<Arc<download::Tasks>, Error> {
        if let Some(download) = DOWNLOAD.get() {
            Ok(download.clone())
        } else {
            let db = Self::db()?;
            let download = Arc::new(download::Tasks(db));
            download.init()?;
            DOWNLOAD.set(download.clone()).unwrap();
            Ok(download)
        }
    }

    pub fn get_onedrive() -> Result<Arc<onedrive::Onedrive>, Error> {
        if let Some(onedrive) = ONEDRIVE.get() {
            Ok(onedrive.clone())
        } else {
            let db = Self::db()?;
            let onedrive = Arc::new(onedrive::Onedrive(db));
            onedrive.init()?;
            ONEDRIVE.set(onedrive.clone()).unwrap();
            Ok(onedrive)
        }
    }

    pub fn get_anime() -> Result<Arc<anime::Anime>, Error> {
        if let Some(anime) = ANIME.get() {
            Ok(anime.clone())
        } else {
            let db = Self::db()?;
            let anime = Arc::new(anime::Anime(db));
            anime.init()?;
            ANIME.set(anime.clone()).unwrap();
            Ok(anime)
        }
    }
}

impl Deref for Db {
    type Target = redb::Database;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
