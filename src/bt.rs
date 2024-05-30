use librqbit::dht::Id20;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
};
use snafu::Snafu;
use std::fmt::Debug;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use crate::util::config::Download;

#[derive(Clone)]
pub struct SessionGuard(Arc<Session>);

impl Debug for SessionGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionGuard").finish()
    }
}

static SESSION: OnceLock<SessionGuard> = OnceLock::new();

impl SessionGuard {
    pub async fn get(download: Download) -> Result<SessionGuard, Error> {
        if let Some(session) = SESSION.get() {
            return Ok(session.clone());
        }

        let mut option = SessionOptions::default();
        option.persistence = false;
        option.enable_upnp_port_forwarding = download.upnp;
        option.listen_port_range = Some(Range {
            start: download.download_port,
            end: download.download_port + 1,
        });

        let session = Session::new_with_opts(download.tmp_dir, option)
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

    pub fn delete_torrent_by_hash(&self, info_hash: Id20) -> Result<(), Error> {
        let session = self.0.clone();

        let id = session.with_torrents(|torrents| {
            for (id, torrent) in torrents {
                if torrent.info_hash() == info_hash {
                    return Ok(id);
                }
            }
            Err(Error::Delete {
                error: format!("Torrent not found: {:?}", info_hash),
            })
        })?;
        session.delete(id, true).map_err(|error| Error::Delete {
            error: error.to_string(),
        })?;

        Ok(())
    }

    pub fn delete_torrent_by_id(&self, id: usize) -> Result<(), Error> {
        let session = self.0.clone();
        session.delete(id, true).map_err(|error| Error::Delete {
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

    #[snafu(display("Failed to delete torrent: {}", error))]
    Delete { error: String },
}
