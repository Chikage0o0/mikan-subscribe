use redb::Error;
use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

const DB_PATH: &str = "store.db";

mod anime;
mod download;
mod episode;
mod onedrive;
mod subscribe;

pub use download::Task as DownloadTask;
pub use download::TaskState as DownloadTaskState;

static DB: OnceLock<Arc<Db>> = OnceLock::new();
static SUBSCRIBE: OnceLock<Arc<subscribe::Subscribe>> = OnceLock::new();
static DOWNLOAD: OnceLock<Arc<download::Tasks>> = OnceLock::new();
static ONEDRIVE: OnceLock<Arc<onedrive::Onedrive>> = OnceLock::new();
static ANIME: OnceLock<Arc<anime::Anime>> = OnceLock::new();
static EPISODE: OnceLock<Arc<episode::Episode>> = OnceLock::new();

#[derive(Debug)]
pub struct Db(redb::Database);

impl Db {
    pub fn get_db() -> Result<Arc<Self>, Error> {
        if let Some(db) = DB.get() {
            return Ok(db.clone());
        }
        let db = Arc::new(Self(redb::Database::create(DB_PATH)?));

        DB.set(db.clone()).unwrap();
        Ok(db)
    }

    pub fn get_subscribe() -> Result<Arc<subscribe::Subscribe>, Error> {
        if let Some(subscribe) = SUBSCRIBE.get() {
            Ok(subscribe.clone())
        } else {
            let db = Self::get_db()?;
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
            let db = Self::get_db()?;
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
            let db = Self::get_db()?;
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
            let db = Self::get_db()?;
            let anime = Arc::new(anime::Anime(db));
            anime.init()?;
            ANIME.set(anime.clone()).unwrap();
            Ok(anime)
        }
    }

    pub fn get_episode() -> Result<Arc<episode::Episode>, Error> {
        if let Some(episode) = EPISODE.get() {
            Ok(episode.clone())
        } else {
            let db = Self::get_db()?;
            let episode = Arc::new(episode::Episode(db));
            episode.init()?;
            EPISODE.set(episode.clone()).unwrap();
            Ok(episode)
        }
    }
}

impl Deref for Db {
    type Target = redb::Database;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
