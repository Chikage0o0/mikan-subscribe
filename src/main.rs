mod bt;
mod store;
mod subscribe;
mod util;
mod worker;

use subscribe::get_feed;
use tokio::signal;
use tracing::{debug, error};
use tracing::{info, Level};
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use util::llama;
use util::reqwest::init_client;
use worker::DownloadHandle;

#[tokio::main]
async fn main() {
    let filtered_layer = fmt::layer()
        .with_filter(FilterFn::new(|metadata| metadata.level() <= &Level::DEBUG))
        .with_filter(FilterFn::new(|metadata| {
            !metadata.target().starts_with("librqbit")
                && !metadata.target().starts_with("llama")
                && !metadata.target().starts_with("h2")
                && !metadata.target().starts_with("hyper")
        }));
    tracing_subscriber::registry().with(filtered_layer).init();

    let settings = util::config::Settings::load_from_file("settings.json").unwrap();
    let _ = init_client(settings.proxy).unwrap();
    if let Some(llama) = settings.llama {
        llama::Llama::init(&llama.model, &llama.url, &llama.token).unwrap();
    }

    let _upload_worker = worker::upload_video(settings.storage).await;
    let download_worker = DownloadHandle::init(settings.download).await.unwrap();

    info!("Service started");
    let download_worker_cloned = download_worker.clone();
    tokio::spawn(async move {
        let subscribe = settings.subscribe;
        let db = store::Db::get_subscribe().unwrap();

        loop {
            info!("Checking feed");
            let feed = get_feed(&subscribe).await;

            if let Err(e) = feed {
                tracing::error!("Error getting feed: {}", e);
                continue;
            }
            let feed = feed.unwrap();

            for (name, item) in feed {
                match db.get(name.clone()) {
                    Ok(Some(_)) => {
                        debug!("Already in processed {}", name);
                    }
                    Ok(None) => {
                        debug!("Processing {}", name);
                        let ret = download_worker_cloned
                            .add(name.to_owned(), item.clone())
                            .await;
                        if let Err(e) = ret {
                            error!("Error adding download task: {}", e);
                        }

                        // Insert into database to avoid duplicate processing
                        db.insert(name.clone()).unwrap_or_else(|e| {
                            error!("Error inserting into database: {}", e);
                        });
                    }
                    Err(e) => {
                        error!("Error when accessing database {}", e);
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(600)).await;
        }
    });

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("Service stopped");
}
