use std::sync::Arc;

use librqbit::dht::Id20;
use snafu::{ResultExt, Snafu};
use tokio::select; // Add this import

use crate::{
    bt,
    store::{self, DownloadTask},
    subscribe::Subscription,
    util::config::Download,
};

async fn download_handle(setting: Download) -> Result<DownloadHandle, Error> {
    let seed_seconds = (setting.seed_hours * 3600.0) as u64;
    let max_download_seconds = (setting.max_download_hours * 3600.0) as u64;

    let thread_num = setting.threads;

    let mut threads = Vec::with_capacity(thread_num as usize);
    let session = bt::SessionGuard::get(setting).await.context(SessionSnafu)?;
    let db = store::Db::get_download().context(DbSnafu)?;

    let (tx, rx) = flume::unbounded();

    // Start download threads
    for _ in 0..thread_num {
        let rx_clone = rx.clone();
        let session_clone = session.clone();
        let db_clone = db.clone();

        let handle = tokio::spawn(async move {
            loop {
                // receive subscription
                let (name, sub): (String, Subscription) = rx_clone.recv_async().await.unwrap();
                let magnet = sub.magnet;

                tracing::info!("Downloading: {}", name);
                // Add torrent
                let ret = session_clone.add_torrent(&magnet).await;
                if let Err(e) = &ret {
                    tracing::error!("Error adding torrent: {}", e);
                    continue;
                }
                let (id, handle) = ret.unwrap();

                // Update state to downloading
                let ret =
                    db_clone.update_state(name.clone(), store::DownloadTaskState::Downloading);
                if let Err(e) = &ret {
                    tracing::error!("Error updating state: {}", e);
                    continue;
                }

                // Wait for download to complete
                // If download takes too long, delete the torrent and download record
                select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(max_download_seconds)) => {
                        tracing::error!("Download timeout: {}", name);
                        session_clone.delete_torrent_by_id(id).unwrap_or_else(|e| {
                            tracing::error!("Error deleting torrent: {}", e);
                        });
                        db_clone.delete(&name).unwrap_or_else(|e| {
                            tracing::error!("Error deleting download: {}", e);
                        });

                        continue;
                    }
                    ret = handle.wait_until_completed() => {
                        if let Err(e) = &ret {
                            tracing::error!("Error downloading: {}", e);
                            continue;
                        }

                    }
                };

                // download file or folder
                let file_name = handle.info().info.name.to_owned().unwrap().to_string();
                let file_path = handle.info().out_dir.join(&file_name);
                tracing::info!("Finished downloading: {}", name);

                let info_hash = handle.info().info_hash.to_owned().as_string();

                let ret = db_clone.update_state(
                    name.clone(),
                    store::DownloadTaskState::Downloaded {
                        file_path,
                        info_hash,
                    },
                );

                if let Err(e) = &ret {
                    tracing::error!("Error updating state: {}", e);
                    continue;
                }
            }
        });
        threads.push(handle);
    }

    Ok(DownloadHandle {
        _threads: threads,
        tx,
        seed_seconds,
        session,
    })
}

pub struct DownloadHandle {
    _threads: Vec<tokio::task::JoinHandle<()>>,
    tx: flume::Sender<(String, Subscription)>,
    seed_seconds: u64,
    session: bt::SessionGuard,
}

impl DownloadHandle {
    pub async fn add(&self, name: String, sub: Subscription) -> Result<(), Error> {
        let db = store::Db::get_download().context(DbSnafu)?;
        db.insert(
            name.clone(),
            DownloadTask {
                url: sub.magnet.clone(),
                anime_title: sub.anime.name.clone(),
                air_date: sub.anime.air_date.clone(),
                weekday: sub.anime.weekday.clone(),
                state: store::DownloadTaskState::Pending,
                added_at: chrono::Utc::now().timestamp() as u64,
            },
        )
        .context(DbSnafu)?;
        self.tx.send_async((name, sub)).await.context(SendSnafu)?;

        Ok(())
    }

    async fn add_from_task(&self, name: String, task: DownloadTask) -> Result<(), Error> {
        let sub = Subscription {
            magnet: task.url,
            anime: crate::subscribe::Anime {
                name: task.anime_title,
                air_date: task.air_date,
                weekday: task.weekday,
                rss: "".to_owned(),
                bangumi_link: "".to_owned(),
            },
        };
        self.add(name, sub).await
    }

    // Initialize download worker
    pub async fn init(setting: Download) -> Result<Arc<Self>, Error> {
        let handle = Arc::new(download_handle(setting).await?);
        let db = store::Db::get_download().context(DbSnafu)?;

        let ret = db
            .get_with_state(|state| match state {
                store::DownloadTaskState::Pending | store::DownloadTaskState::Downloading => true,
                _ => false,
            })
            .context(DbSnafu)?;

        for (name, task) in ret {
            handle.add_from_task(name, task).await?;
        }

        // 每隔一分钟检查一次是否有下载并上传完成的任务，并删除
        let handle_cloned = handle.clone();
        tokio::spawn(async move {
            // sleep 随机时间，避免同时清理
            tokio::time::sleep(tokio::time::Duration::from_secs(rand::random::<u64>() % 60)).await;
            loop {
                let handle_cloned = handle_cloned.clone();
                let blocking_handle = tokio::task::spawn_blocking(move || {
                    handle_cloned.delete_finished().unwrap_or_else(|e| {
                        tracing::error!("Error deleting finished: {}", e);
                    });
                });

                blocking_handle.await.unwrap_or_else(|e| {
                    tracing::error!("Error deleting finished: {}", e);
                });

                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });

        Ok(handle)
    }

    // Delete download record
    fn delete_download(&self, info_hash: Id20) -> Result<(), Error> {
        self.session
            .delete_torrent_by_hash(info_hash)
            .context(SessionSnafu)?;
        Ok(())
    }

    // Delete download records that have been finished for a certain amount of time
    fn delete_finished(&self) -> Result<(), Error> {
        let db = store::Db::get_download().context(DbSnafu)?;
        let ret = db
            .get_with_state(|state| match state {
                store::DownloadTaskState::Finished { .. } => true,
                _ => false,
            })
            .context(DbSnafu)?;

        for (name, task) in ret {
            match task.state {
                store::DownloadTaskState::Finished {
                    finish_time,
                    info_hash,
                    file_path,
                } => {
                    if finish_time + self.seed_seconds < chrono::Utc::now().timestamp() as u64 {
                        let ret = self.delete_download(info_hash.parse().unwrap());
                        if let Err(e) = ret {
                            tracing::warn!(
                                "Error deleting download: {},try to directly rm file",
                                e
                            );

                            if file_path.exists() {
                                if file_path.is_file() {
                                    if std::fs::remove_file(&file_path).is_err() {
                                        tracing::error!(
                                            "Error deleting file {}: {}",
                                            file_path.display(),
                                            e
                                        );
                                    }
                                } else {
                                    std::fs::remove_dir_all(&file_path).unwrap_or_else(|e| {
                                        tracing::error!(
                                            "Error deleting folder {}: {}",
                                            file_path.display(),
                                            e
                                        );
                                    });
                                }
                            }
                        }

                        db.delete(&name).unwrap_or_else(|e| {
                            tracing::error!("Error deleting download in db: {}: {}", name, e);
                        });
                    }
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Error executing download task: {}", source))]
    SessionError { source: bt::Error },

    #[snafu(display("Error connecting to database: {}", source))]
    DbError { source: redb::Error },

    #[snafu(display("Error sending subscription: {}", source))]
    Send {
        source: flume::SendError<(String, Subscription)>,
    },
}
