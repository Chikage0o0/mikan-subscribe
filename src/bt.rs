use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
};
use snafu::Snafu;
use std::fmt::Debug;
use std::sync::{Arc, OnceLock};

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

        let option = SessionOptions::default();

        let session = Session::new_with_opts(DOWNLOADING_PATH.into(), option)
            .await
            .map_err(|error| Error::CreateSession {
                error: error.to_string(),
            })?;
        let session = SessionGuard(session);
        SESSION.set(session.clone()).unwrap();

        Ok(session)
    }

    pub async fn add_torrent(&self, magnet: &str) -> Result<(usize, Arc<ManagedTorrent>), Error> {
        let session = self.0.clone();

        let response = session
            .add_torrent(
                AddTorrent::from_url(magnet),
                Some(AddTorrentOptions {
                    overwrite: true,
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

        Ok((id, handle))
    }

    pub fn delete_torrent(&self, id: usize) -> Result<(), Error> {
        let session = self.0.clone();
        session
            .delete(id, true)
            .map_err(|error| Error::FinishDownload {
                error: error.to_string(),
            })?;

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
