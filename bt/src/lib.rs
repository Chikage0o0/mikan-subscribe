use chrono::Datelike as _;
use std::{
    fmt::Debug,
    sync::{Arc, OnceLock},
};

// use db::Db;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tracing::info;
use upload_backend::{backend::Onedrive, Backend};

// mod db;

const DOWNLOADING_PATH: &str = "tmp";

#[derive(Clone)]
pub struct SessionGuard(Arc<Session>);

impl Debug for SessionGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionGuard").finish()
    }
}
static SESSION: OnceLock<SessionGuard> = OnceLock::new();
impl SessionGuard {
    pub async fn get() -> Result<SessionGuard, Error> {
        if let Some(session) = SESSION.get() {
            return Ok(session.clone());
        }

        let mut option = SessionOptions::default();
        option.persistence = true;
        option.persistence_filename = Some("config/session.state".into());

        let session = Session::new_with_opts(DOWNLOADING_PATH.into(), option)
            .await
            .map_err(|error| Error::CreateSession {
                error: error.to_string(),
            })?;
        let session = SessionGuard(session);
        SESSION.set(session.clone()).unwrap();

        Ok(session)
    }

    pub async fn add_torrent(
        &self,
        magnet: &str,
        name: &str,
    ) -> Result<(usize, Arc<ManagedTorrent>), Error> {
        let session = self.0.clone();

        let response = session
            .add_torrent(
                AddTorrent::from_url(magnet),
                Some(AddTorrentOptions {
                    overwrite: true,
                    output_folder: Some(format!("{}/{}", DOWNLOADING_PATH, name)),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|error| Error::AddTorrent {
                error: error.to_string(),
            })?;

        let (id, handle) = match response {
            AddTorrentResponse::AlreadyManaged(id, handle) => (id, handle),
            AddTorrentResponse::ListOnly(_) => {
                unreachable!("ListOnly should not be returned by add_torrent")
            }
            AddTorrentResponse::Added(id, handle) => (id, handle),
        };

        // Db::db()
        //     .context(DatabaseSnafu)?
        //     .insert(
        //         id,
        //         Task {
        //             url: magnet.to_owned(),
        //             anime_title: name.to_owned(),
        //         },
        //     )
        //     .context(DatabaseSnafu)?;

        Ok((id, handle))
    }

    pub async fn after_add_torrent(
        &self,
        id: usize,
        folder_name: &str,
        handle: Arc<ManagedTorrent>,
        onedrive: &Onedrive,
    ) -> Result<(), Error> {
        let _ = onedrive;
        info!("Add torrent success: {:?}", handle.info().info);

        let tmp_file = match handle.info().info.name.as_ref() {
            Some(file_name) => format!("{}/{}/{}", DOWNLOADING_PATH, folder_name, file_name),
            None => format!("{}/{}", DOWNLOADING_PATH, folder_name),
        };
        let target_file = match handle.info().info.name.as_ref() {
            Some(file_name) => format!("{}/{}/{}", generate_folder_name(), folder_name, file_name),
            None => format!("{}/{}", generate_folder_name(), folder_name),
        };

        handle
            .wait_until_completed()
            .await
            .map_err(|error| Error::Downloading {
                error: error.to_string(),
            })?;

        // todo: upload to cloud
        let file = tokio::fs::File::open(&tmp_file).await.unwrap();
        let size = file.metadata().await.unwrap().len();
        let mut reader = tokio::io::BufReader::new(file);
        onedrive
            .upload(&mut reader, size, &target_file)
            .await
            .map_err(|error| Error::FinishDownload {
                error: error.to_string(),
            })?;

        let session = &self.0;
        session
            .delete(id, true)
            .map_err(|error| Error::FinishDownload {
                error: error.to_string(),
            })?;

        Ok(())
    }
}

fn generate_folder_name() -> String {
    let date = chrono::Local::now();
    let year = date.year();
    let month = date.month();

    match month {
        i if i < 4 => format!("{}年{}月", year, 1),
        i if i < 7 => format!("{}年{}月", year, 4),
        i if i < 10 => format!("{}年{}月", year, 7),
        _ => format!("{}年{}月", year, 10),
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    // #[snafu(display("Database error: {}", source))]
    // Database { source: redb::Error },
    #[snafu(display("Failed to create session: {}", error))]
    CreateSession { error: String },

    #[snafu(display("Failed to add torrent: {}", error))]
    AddTorrent { error: String },

    #[snafu(display("Failed to get session"))]
    GetTask { error: String },

    #[snafu(display("Downloading error: {}", error))]
    Downloading { error: String },

    #[snafu(display("Finish download error: {}", error))]
    FinishDownload { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub url: String,
    pub anime_title: String,
}
