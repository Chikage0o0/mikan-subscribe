use std::path::PathBuf;

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Storage {
    Local {
        root: PathBuf,
    },
    Onedrive {
        name: String,
        client_id: String,
        client_secret: String,
        root: PathBuf,
        api_type: upload_backend::backend::OnedriveApiType,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub tmp_dir: PathBuf,
    pub upnp: bool,
    pub download_port: u16,
    pub threads: u16,
    pub seed_hours: f32,
    pub max_download_hours: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub storage: Vec<Storage>,
    pub subscribe: String,
    pub download: Download,
    pub proxy: Option<String>,
    pub llama: Option<Llama>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Llama {
    pub model: String,
    pub url: String,
    pub token: String,
}

impl Settings {
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name(path))
            .add_source(Environment::with_prefix("MK"))
            .build()?;
        settings.try_deserialize()
    }

    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use upload_backend::backend::OnedriveApiType;

    use super::*;

    const SETTINGS: &str = "settings.json.example";

    #[test]
    fn test_settings_file() {
        let settings = Settings {
            storage: vec![
                Storage::Local { root: "d".into() },
                Storage::Onedrive {
                    name: "name".into(),
                    client_id: "client_id".into(),
                    client_secret: "client".into(),
                    root: "root".into(),
                    api_type: OnedriveApiType::Organizations,
                },
            ],
            subscribe: "https://example.com".to_string(),
            download: Download {
                tmp_dir: "tmp".into(),
                upnp: false,
                download_port: 6881,
                threads: 5,
                seed_hours: 1.0,
                max_download_hours: 24.0,
            },
            proxy: Some("socks5://127.0.0.1:1080".to_string()),
            llama: Some(Llama {
                model: "model".into(),
                url: "url".into(),
                token: "token".into(),
            }),
        };

        settings.save_to_file(SETTINGS).unwrap();
    }
}
