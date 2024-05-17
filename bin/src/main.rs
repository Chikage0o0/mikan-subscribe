mod db;

use std::{path::Path, sync::Arc};

use clap::Parser;
use db::Db;
use subscribe::get_feed;

use tracing::{error, info, Level};
use upload_backend::backend::Onedrive;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The URL of the feed  Mikanani.me
    #[arg(short, long)]
    subscribe: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish(),
    )
    .unwrap_or_else(|e| {
        eprintln!("Could not set up global logger: {}", e);
    });

    if Path::new("config").exists() == false {
        std::fs::create_dir("config").expect("Failed to create config dir");
    }

    let db = db::Db::db().expect("Failed to create db");

    let onedrive = match get_onedrive_config(db.clone()).await {
        Some(v) => v,
        None => {
            error!("Failed to get onedrive config");
            return;
        }
    };

    let session = bt::SessionGuard::get()
        .await
        .expect("Failed to create session");

    loop {
        let feed = match get_feed(args.subscribe.to_string()).await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to fetch feed: {}", e);
                std::thread::sleep(std::time::Duration::from_secs(60 * 5));
                continue;
            }
        };

        for f in feed {
            if db.get(f.url.clone()).unwrap_or(None).is_some() {
                info!("Already added: {}", f.name);
                continue;
            }

            info!("Adding torrent: {}", f.name);

            let (id, handle) = match session.add_torrent(&f.magnet, &f.anime_title).await {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to add torrent: {}", e);
                    continue;
                }
            };

            session
                .after_add_torrent(id, &f.anime_title, handle, &onedrive)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to after_add_torrent: {}", e);
                });

            db.insert(f.url.clone()).unwrap_or_else(|e| {
                error!("Failed to insert into db: {}", e);
            });
            info!("Finished adding torrent: {}", f.name);
        }
        db.clear_expire().unwrap_or_else(|e| {
            error!("Failed to clear expire: {}", e);
        });
        let refresh_token = onedrive.refresh_token();
        db.insert_refresh_token(refresh_token).unwrap_or_else(|e| {
            error!("Failed to insert refresh token: {}", e);
        });

        std::thread::sleep(std::time::Duration::from_secs(60 * 10));
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct OnedriveConfig {
    client_id: String,
    client_secret: String,
    folder_path: String,
}

async fn get_onedrive_config(db: Arc<Db>) -> Option<Onedrive> {
    let config = std::fs::read_to_string("config/onedrive.json").ok()?;
    let config: OnedriveConfig = serde_json::from_str(&config).ok()?;
    let refresh_token = db.get_refresh_token().ok().and_then(|v| v);

    if let Some(refresh_token) = refresh_token {
        if let Ok(v) = Onedrive::new_with_refresh_token(
            &config.client_id,
            &config.client_secret,
            refresh_token,
            upload_backend::backend::OnedriveApiType::Organizations,
            &config.folder_path,
        )
        .await
        {
            return Some(v);
        }
    }

    if let Ok(v) = Onedrive::new_with_code(
        &config.client_id,
        &config.client_secret,
        "http://localhost:20080/redirect",
        upload_backend::backend::OnedriveApiType::Organizations,
        &config.folder_path,
    )
    .await
    {
        return Some(v);
    }

    None
}

#[test]
fn test_get_onedrive_config() {
    let config = OnedriveConfig {
        client_id: "client_id".to_string(),
        client_secret: "client_secret".to_string(),
        folder_path: "folder_path".to_string(),
    };
    let config = serde_json::to_string(&config).unwrap();
    std::fs::write("onedrive.json", config).unwrap();
}
