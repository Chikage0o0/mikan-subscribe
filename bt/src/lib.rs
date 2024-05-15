use std::{
    fmt::Debug,
    fs::create_dir_all,
    path::Path,
    sync::{Arc, OnceLock},
};
use tracing::error;
// use db::Db;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tracing::info;

// mod db;

const DOWNLOADING_PATH: &str = "tmp";
const DOWNLOAD_PATH: &str = "downloads";

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
        option.enable_upnp_port_forwarding = true;
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
    ) -> Result<(), Error> {
        info!("Add torrent success: {:?}", handle.info().info);

        let tmp_file = match handle.info().info.name.as_ref() {
            Some(file_name) => format!("{}/{}/{}", DOWNLOADING_PATH, folder_name, file_name),
            None => format!("{}/{}", DOWNLOADING_PATH, folder_name),
        };
        let target_file = match handle.info().info.name.as_ref() {
            Some(file_name) => format!("{}/{}/{}", DOWNLOAD_PATH, folder_name, file_name),
            None => format!("{}/{}", DOWNLOAD_PATH, folder_name),
        };

        handle
            .wait_until_completed()
            .await
            .map_err(|error| Error::Downloading {
                error: error.to_string(),
            })?;

        // todo: upload to cloud
        let session = &self.0;
        session
            .delete(id, false)
            .map_err(|error| Error::FinishDownload {
                error: error.to_string(),
            })?;

        // move to download folder
        let parent = Path::new(&target_file).parent().unwrap();
        create_dir_all(parent).unwrap();

        std::fs::rename(&tmp_file, &target_file).unwrap_or_else(|e| {
            error!("Failed to move {} to {}: {:?}", tmp_file, target_file, e);
        });

        Ok(())
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

#[cfg(test)]
mod test {

    #[tokio::test]
    async fn test_session_guard() {
        let _session = super::SessionGuard::get().await.unwrap();
    }
}
