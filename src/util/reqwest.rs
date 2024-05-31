use std::sync::{Arc, OnceLock};

use reqwest::Error;

use tracing::warn;

static CLIENT: OnceLock<Arc<reqwest::Client>> = OnceLock::new();

pub fn client() -> Arc<reqwest::Client> {
    CLIENT.get().unwrap().clone()
}

pub fn init_client(proxy: Option<String>) -> Result<Arc<reqwest::Client>, Error> {
    if let Some(client) = CLIENT.get() {
        return Ok(client.clone());
    }

    let mut client = reqwest::ClientBuilder::new();

    if let Some(proxy) = proxy {
        if !proxy.is_empty() {
            let proxy = reqwest::Proxy::all(proxy);
            match proxy {
                Ok(proxy) => {
                    client = client.proxy(proxy);
                }
                Err(e) => {
                    warn!("Error setting proxy: {}", e);
                }
            }
        }
    }
    let client = client.build()?;
    let client = Arc::new(client);
    CLIENT.set(client.clone()).unwrap();
    Ok(client)
}
