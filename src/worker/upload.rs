use chrono::Datelike;
use chrono::NaiveDate;
use std::path::{Path, PathBuf};
use tokio::io::AsyncSeekExt as _;
use tokio::task::JoinHandle;
use tracing::info;

use crate::store::Db;
use crate::util::config::Storage;
use crate::util::convert_storage;
use crate::util::llama;
use crate::util::reqwest::client;

pub async fn upload_video(storages: Vec<Storage>) -> JoinHandle<()> {
    let backend = convert_storage(storages).await.unwrap();
    let download_db = Db::get_download().unwrap();

    tokio::spawn(async move {
        // sleep 随机时间，避免同时清理
        tokio::time::sleep(tokio::time::Duration::from_secs(rand::random::<u64>() % 60)).await;
        loop {
            // 获取下载完成的任务，但是还没有上传的
            let ret = download_db.get_with_state(|state| match state {
                crate::store::DownloadTaskState::Downloaded { .. } => true,
                _ => false,
            });
            if let Err(e) = ret {
                tracing::error!("Error getting download tasks: {}", e);
                continue;
            }
            let ret = ret.unwrap();

            for (name, task) in ret {
                match task.state {
                    crate::store::DownloadTaskState::Downloaded {
                        file_path,
                        info_hash,
                    } => {
                        let video_path = find_video_in_path(&file_path).await;
                        if video_path.is_none() {
                            tracing::error!("No video found in {:?}", file_path);
                            // set state to Finished
                            download_db
                                .update_state(
                                    name.clone(),
                                    crate::store::DownloadTaskState::Finished {
                                        file_path: file_path.clone(),
                                        info_hash,
                                        finish_time: chrono::Utc::now().timestamp() as u64,
                                    },
                                )
                                .unwrap_or_else(|e| {
                                    tracing::error!("Error updating state: {}", e);
                                });
                            continue;
                        }
                        let video_path = video_path.unwrap();

                        let bangumi_id = task.bangumi_id;
                        let video_name = generate_file_name(&video_path, bangumi_id).await;
                        let upload_path = Path::new(generate_folder_name(task.air_date).as_str())
                            .join(task.weekday)
                            .join(task.anime_title)
                            .join(video_name);

                        let file = tokio::fs::File::open(&video_path).await;
                        if let Err(e) = file {
                            tracing::error!("Error opening file {}: {}", video_path.display(), e);
                            continue;
                        }
                        let file = file.unwrap();
                        let size = file.metadata().await.unwrap().len();
                        let mut reader = tokio::io::BufReader::new(file);
                        for backend in &backend {
                            reader.seek(std::io::SeekFrom::Start(0)).await.unwrap();
                            let ret = backend.upload(&mut reader, size, upload_path.clone()).await;
                            if let Err(e) = ret {
                                tracing::error!("Error uploading: {}", e);
                                continue;
                            }
                        }

                        // set state to Finished

                        download_db
                            .update_state(
                                name.clone(),
                                crate::store::DownloadTaskState::Finished {
                                    file_path: file_path.clone(),
                                    info_hash,
                                    finish_time: chrono::Utc::now().timestamp() as u64,
                                },
                            )
                            .unwrap_or_else(|e| {
                                tracing::error!("Error updating state: {}", e);
                            });

                        info!("Uploaded: {}", name);
                    }
                    _ => unreachable!(),
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    })
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

async fn generate_file_name(path: &Path, bangumi_id: u64) -> String {
    if let Some(l) = llama::Llama::get() {
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        let file_name_clone = file_name.clone();
        let ext = path.extension().unwrap().to_str().unwrap().to_string();
        let blocking_spawn = tokio::task::spawn_blocking(move || l.decode(&file_name_clone));
        let ret = blocking_spawn.await.unwrap();
        match ret {
            Ok(ret) => {
                let episode = ret.episode;

                let client = client()
                    .get(format!(
                        "https://api.bgm.tv/v0/episodes?subject_id={}&type=0&limit=1000",
                        bangumi_id
                    ))
                    .header("User-Agent", "Chikage0o0/mikan-subscriber")
                    .send()
                    .await;
                match client {
                    Ok(client) => {
                        let client = client.json::<BangumiEpisode>().await;
                        if let Ok(client) = client {
                            for i in client.data {
                                if i.ep == episode || i.sort == episode {
                                    if i.name_cn.is_empty() {
                                        return format!(
                                            "{:02} - {}.{}",
                                            episode,
                                            sanitize_filename(&i.name),
                                            ext
                                        );
                                    } else {
                                        return format!(
                                            "{:02} - {}.{}",
                                            episode,
                                            sanitize_filename(&i.name_cn),
                                            ext
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error when get_name_from_bangumi: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error decoding {}: {}", file_name, e);
            }
        }
    }

    path.file_name().unwrap().to_str().unwrap().to_string()
}

fn sanitize_filename(filename: &str) -> String {
    // Define a set of special characters to be replaced
    let special_chars = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

    // Replace each special character with an underscore
    filename
        .chars()
        .map(|c| if special_chars.contains(&c) { '_' } else { c })
        .collect()
}

#[derive(Debug, serde::Deserialize)]
struct BangumiEpisode {
    pub data: Vec<Episode>,
}
#[derive(Debug, serde::Deserialize)]
struct Episode {
    name: String,
    name_cn: String,
    sort: u32,
    ep: u32,
}

#[cfg(test)]
mod tests {
    use crate::util::{self, llama, reqwest::init_client};

    #[tokio::test]
    async fn test_generate_file_name() {
        let settings = util::config::Settings::load_from_file("settings.json").unwrap();
        let _ = init_client(settings.proxy).unwrap();
        if let Some(llama) = settings.llama_model {
            let blocking_spawn = tokio::task::spawn_blocking(move || llama::Llama::init(&llama));
            if let Err(e) = blocking_spawn.await.unwrap() {
                eprintln!("Error loading llama model: {}", e);
            }
        }
        let path = std::path::Path::new(
            "[Up to 21°C] Henjin no Salad Bowl - 09 (CR 1920x1080 AVC AAC MKV) [37D7B6CE].mkv",
        );
        let bangumi_id = 444403;
        let file_name = super::generate_file_name(path, bangumi_id).await;

        println!("{}", file_name);
    }
}
