mod db;

use std::path::Path;

use clap::Parser;
use subscribe::get_feed;
use tracing::{error, info, Level};

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
                .after_add_torrent(id, &f.anime_title, handle)
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

        std::thread::sleep(std::time::Duration::from_secs(60 * 10));
    }
}
