use std::path::PathBuf;

use config::{Config, ConfigError, Environment, File};
use serde::{ser::SerializeMap as _, Deserialize, Serialize};
use upload_backend::backend;

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
    Webdav {
        name: String,
        url: String,
        auth: WebdavAuth,
    },
}

#[derive(Debug, Clone)]
pub struct WebdavAuth(pub backend::WebdavAuth);

impl Serialize for WebdavAuth {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match &self.0 {
            backend::WebdavAuth::Basic(username, password) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("type", "basic")?;
                map.serialize_entry("username", username)?;
                map.serialize_entry("password", password)?;
                map.end()
            }
            backend::WebdavAuth::Digest(username, password) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("type", "digest")?;
                map.serialize_entry("username", username)?;
                map.serialize_entry("password", password)?;
                map.end()
            }
            backend::WebdavAuth::Anonymous => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("type", "anonymous")?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for WebdavAuth {
    fn deserialize<D>(deserializer: D) -> Result<WebdavAuth, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let auth = match value.get("type").and_then(|v| v.as_str()) {
            Some("basic") => {
                let username = value.get("username").and_then(|v| v.as_str()).unwrap();
                let password = value.get("password").and_then(|v| v.as_str()).unwrap();
                backend::WebdavAuth::Basic(username.into(), password.into())
            }
            Some("digest") => {
                let username = value.get("username").and_then(|v| v.as_str()).unwrap();
                let password = value.get("password").and_then(|v| v.as_str()).unwrap();
                backend::WebdavAuth::Digest(username.into(), password.into())
            }
            Some("anonymous") => backend::WebdavAuth::Anonymous,
            _ => return Err(serde::de::Error::custom("invalid auth type")),
        };
        Ok(WebdavAuth(auth))
    }
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
                Storage::Webdav {
                    name: "name".into(),
                    url: "url".into(),
                    auth: WebdavAuth(backend::WebdavAuth::Basic("user".into(), "pass".into())),
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
