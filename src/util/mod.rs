pub mod config;
pub mod llama;
pub mod reqwest;

use std::path::PathBuf;

use snafu::ResultExt;

use tracing::{info, warn};
use upload_backend::Backend;

use crate::store;

pub async fn convert_storage(
    storage: Vec<config::Storage>,
) -> Result<Vec<Box<dyn Backend + Send + Sync>>, Error> {
    let db = store::Db::get_onedrive().context(DbSnafu)?;

    let mut backends: Vec<Box<dyn Backend + Send + Sync>> = Vec::new();
    for s in storage {
        match s {
            config::Storage::Local { root } => {
                info! {"Loading Local: {:?}", root};
                tokio::fs::create_dir_all(&root).await.context(IoSnafu)?;
                backends.push(Box::new(upload_backend::backend::Local::new(root)));
            }
            config::Storage::Onedrive {
                name,
                client_id,
                client_secret,
                root,
                api_type,
            } => {
                info! {"Loading Onedrive: {}", name};
                let refresh_token = db.get_refresh_token(name.clone()).context(DbSnafu)?;
                let onedrive = login_onedrive(
                    &client_id,
                    &client_secret,
                    &api_type,
                    &root,
                    refresh_token.as_deref(),
                )
                .await;

                if let Err(e) = &onedrive {
                    warn!("Error loading {} Onedrive: {}", name, e);
                    continue;
                }
                let onedrive = onedrive.unwrap();

                db.insert_refresh_token(onedrive.refresh_token(), name.clone())
                    .context(DbSnafu)?;

                backends.push(Box::new(onedrive));
            }
        }
    }

    Ok(backends)
}

async fn login_onedrive(
    client_id: &str,
    client_secret: &str,
    api_type: &upload_backend::backend::OnedriveApiType,
    root: &PathBuf,
    refresh_token: Option<&str>,
) -> Result<upload_backend::backend::Onedrive, Error> {
    match refresh_token {
        Some(token) => {
            let ret = upload_backend::backend::Onedrive::new_with_refresh_token(
                client_id,
                client_secret,
                token,
                api_type.clone(),
                root,
            )
            .await;

            match ret {
                Ok(onedrive) => Ok(onedrive),
                Err(e) => {
                    warn!("Error loading Onedrive: {}", e);
                    let onedrive = upload_backend::backend::Onedrive::new_with_code(
                        client_id,
                        client_secret,
                        "http://localhost:18080".to_string(),
                        api_type.clone(),
                        root,
                    )
                    .await
                    .map_err(|e| Error::Onedrive {
                        error: e.to_string(),
                    })?;

                    Ok(onedrive)
                }
            }
        }
        None => upload_backend::backend::Onedrive::new_with_code(
            client_id,
            client_secret,
            "http://localhost:18080".to_string(),
            api_type.clone(),
            root,
        )
        .await
        .map_err(|e| Error::Onedrive {
            error: e.to_string(),
        }),
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Error loading DB: {}", source))]
    Db { source: redb::Error },

    #[snafu(display("Error loading Onedrive: {}", error))]
    Onedrive { error: String },

    #[snafu(display("Error IO: {}", source))]
    Io { source: std::io::Error },
}
