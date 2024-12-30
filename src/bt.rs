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

        let option = SessionOptions {
            persistence: None,
            enable_upnp_port_forwarding: download.upnp,
            listen_port_range: Some(Range {
                start: download.download_port,
                end: download.download_port + 1,
            }),
            ..Default::default()
        };

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

    pub async fn delete_torrent_by_hash(&self, info_hash: Id20) -> Result<(), Error> {
        let session = self.0.clone();
        session
            .delete(librqbit::api::TorrentIdOrHash::Hash(info_hash), true)
            .await
            .map_err(|error| Error::Delete {
                error: error.to_string(),
            })?;

        Ok(())
    }

    pub async fn delete_torrent_by_id(&self, id: usize) -> Result<(), Error> {
        let session = self.0.clone();
        session
            .delete(librqbit::api::TorrentIdOrHash::Id(id), true)
            .await
            .map_err(|error| Error::Delete {
                error: error.to_string(),
            })?;

        Ok(())
    }

    pub async fn pause_torrent_by_handle(&self, handle: &Arc<ManagedTorrent>) -> Result<(), Error> {
        let session = self.0.clone();
        session
            .pause(handle)
            .await
            .map_err(|error| Error::Delete {
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

#[cfg(test)]
mod test {
    use crate::{bt::SessionGuard, util};

    // test delete_torrent_by_hash
    #[tokio::test]
    async fn test_delete_torrent_by_hash() {
        let settings = util::config::Settings::load_from_file("settings.json").unwrap();
        let session = SessionGuard::get(settings.download).await.unwrap();
        let info_hash = session
            .add_torrent("magnet:?xt=urn:btih:5d9140ed25be2cff3b981566792b668ab6976f58&tr=http%3a%2f%2ft.nyaatracker.com%2fannounce&tr=http%3a%2f%2ftracker.kamigami.org%3a2710%2fannounce&tr=http%3a%2f%2fshare.camoe.cn%3a8080%2fannounce&tr=http%3a%2f%2fopentracker.acgnx.se%2fannounce&tr=http%3a%2f%2fanidex.moe%3a6969%2fannounce&tr=http%3a%2f%2ft.acg.rip%3a6699%2fannounce&tr=https%3a%2f%2ftr.bangumi.moe%3a9696%2fannounce&tr=udp%3a%2f%2ftr.bangumi.moe%3a6969%2fannounce&tr=http%3a%2f%2fopen.acgtracker.com%3a1096%2fannounce&tr=udp%3a%2f%2ftracker.opentrackr.org%3a1337%2fannounce")
            .await
            .unwrap()
            .1
            .info_hash().as_string();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let result = session
            .delete_torrent_by_hash(info_hash.parse().unwrap())
            .await;
        assert!(result.is_ok());
    }
}
