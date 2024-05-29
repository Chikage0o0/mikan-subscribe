mod store;
mod subscribe;
mod util;

use std::{path::Path, sync::Arc};

use clap::Parser;
use store::Db;

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

    todo!("Subscribe to feed");
}
