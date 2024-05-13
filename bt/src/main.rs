use librqbit::Session;
use std::time::Duration;

use librqbit::{AddTorrent, AddTorrentOptions};
use tracing::info;
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Hello, world!");
    let session = Session::new("/home/chikage/Project/mikan-subscribe/tmp".into())
        .await
        .unwrap();
    let handle = session
    .add_torrent(
        AddTorrent::from_url(
            "magnet:?xt=urn:btih:69d27f972bf062bfcc9cacc6ec63a6f44c72fae9&tr=https%3A%2F%2Ftr.bangumi.moe%3A9696%2Fannounce&tr=http%3A%2F%2Ftr.bangumi.moe%3A6969%2Fannounce&tr=udp%3A%2F%2Ftr.bangumi.moe%3A6969%2Fannounce&tr=http%3A%2F%2Fopen.acgtracker.com%3A1096%2Fannounce&tr=http%3A%2F%2F208.67.16.113%3A8000%2Fannounce&tr=udp%3A%2F%2F208.67.16.113%3A8000%2Fannounce&tr=http%3A%2F%2Ftracker.ktxp.com%3A6868%2Fannounce&tr=http%3A%2F%2Ftracker.ktxp.com%3A7070%2Fannounce&tr=http%3A%2F%2Ft2.popgo.org%3A7456%2Fannonce&tr=http%3A%2F%2Fbt.sc-ol.com%3A2710%2Fannounce&tr=http%3A%2F%2Fshare.camoe.cn%3A8080%2Fannounce&tr=http%3A%2F%2F61.154.116.205%3A8000%2Fannounce&tr=http%3A%2F%2Fbt.rghost.net%3A80%2Fannounce&tr=http%3A%2F%2Ftracker.openbittorrent.com%3A80%2Fannounce&tr=http%3A%2F%2Ftracker.publicbt.com%3A80%2Fannounce&tr=http%3A%2F%2Ftracker.prq.to%2Fannounce&tr=http%3A%2F%2Fopen.nyaatorrents.info%3A6544%2Fannounce",
        ),
        Some(AddTorrentOptions {
            // Allow writing on top of existing files.
            overwrite: true,
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_handle()
    .unwrap();

    info!("Details: {:?}", &handle.info().info);

    // Print stats periodically.
    tokio::spawn({
        let handle = handle.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let stats = handle.stats();
                info!("{stats:}");
            }
        }
    });
    handle.wait_until_completed().await.unwrap();
}
