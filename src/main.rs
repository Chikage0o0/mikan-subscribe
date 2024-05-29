mod bt;
mod store;
mod subscribe;
mod util;

use chrono::Datelike;
use chrono::NaiveDate;
use std::path::PathBuf;
use std::{convert, path::Path, sync::Arc};
use subscribe::get_feed;
use tracing::Level;
use util::convert_storage;

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish(),
    )
    .unwrap_or_else(|e| {
        eprintln!("Could not set up global logger: {}", e);
    });

    let settings = util::Settings::load_from_file("settings.json").unwrap();

    let backend = Arc::new(convert_storage(settings.storage).await.unwrap());
    let download_session = bt::SessionGuard::get().await.unwrap();

    tokio::spawn(async move {
        let subscribe = settings.subscribe;
        let backend_cloned = backend.clone();

        loop {
            let feed = get_feed(&subscribe).await;
            if let Err(e) = feed {
                tracing::error!("Error getting feed: {}", e);
                continue;
            }
            let feed = feed.unwrap();

            for item in feed {
                let magnet = item.magnet;

                let anime_name = item.anime.name;
                let anime_airdate = item.anime.air_date;
                let week = item.anime.weekday;
                let download_session = download_session.clone();
                let ret = download_session.add_torrent(&magnet).await;
                if let Err(e) = &ret {
                    tracing::error!("Error adding torrent: {}", e);
                    continue;
                }
                let (id, handle) = ret.unwrap();
                let backend = backend_cloned.clone();
                tokio::spawn(async move {
                    let path = handle.info().out_dir.to_owned();

                    let ret = handle.wait_until_completed().await;

                    if let Err(e) = &ret {
                        tracing::error!("Error downloading torrent: {}", e);
                        return;
                    }

                    let name = handle.info().info.name.to_owned().unwrap().to_string();

                    let download_dir = Path::new(&path).join(&name);
                    let file_path = find_video_in_path(&download_dir).await;
                    if let Some(file_path) = file_path {
                        for backend in backend.iter() {
                            let file = tokio::fs::File::open(&file_path).await;
                            if let Err(e) = file {
                                tracing::error!("Error opening file: {}", e);
                                return;
                            }
                            let mut reader = tokio::io::BufReader::new(file.unwrap());

                            let size = tokio::fs::metadata(&file_path).await;
                            if let Err(e) = size {
                                tracing::error!("Error getting file size: {}", e);
                                return;
                            }
                            let size = size.unwrap().len();

                            let path = Path::new(generate_folder_name(anime_airdate).as_str())
                                .join(&week)
                                .join(&anime_name)
                                .join(file_path.file_name().unwrap());

                            let ret = backend.upload(&mut reader, size, path.clone()).await;

                            if let Err(e) = ret {
                                tracing::error!("Error uploading file: {}", e);
                                continue;
                            }

                            tracing::info!("Uploaded file: {:?}", &path);
                        }
                    }

                    let ret = download_session.delete_torrent(id);
                    if let Err(e) = ret {
                        tracing::error!("Error deleting torrent: {}", e);
                    }
                });
            }
        }
    });

    tokio::signal::ctrl_c().await.unwrap();
}

fn generate_folder_name(date: NaiveDate) -> String {
    let year = date.year();
    let quarters = [
        NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(year, 4, 1).unwrap(),
        NaiveDate::from_ymd_opt(year, 7, 1).unwrap(),
        NaiveDate::from_ymd_opt(year, 10, 1).unwrap(),
    ];

    // Find the nearest quarter start date
    let nearest_quarter = quarters
        .iter()
        .min_by_key(|&&q| (date - q).num_days().abs())
        .unwrap();

    // Get the month of the nearest quarter
    let month = nearest_quarter.month();

    format!("{}年{}月", year, month)
}

async fn find_video_in_path(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }

    if !path.is_dir() {
        if let Some(ext) = path.extension() {
            if ext == "mp4" || ext == "mkv" {
                return Some(path.to_owned());
            }
        }
        return None;
    }

    let mut entries = tokio::fs::read_dir(path).await.ok()?;
    while let Some(entry) = entries.next_entry().await.ok()? {
        let path = entry.path();
        if path.is_dir() {
            let ret = Box::pin(find_video_in_path(&path)).await;
            if ret.is_some() {
                return ret;
            }
        } else {
            if let Some(ext) = path.extension() {
                if ext == "mp4" || ext == "mkv" {
                    return Some(path);
                }
            }
        }
    }
    None
}
