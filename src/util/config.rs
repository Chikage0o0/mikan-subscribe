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
pub struct Settings {
    pub storage: Vec<Storage>,
    pub subscribe: String,
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
    use super::*;

    const SETTINGS: &str = "settings.json";

    #[test]
    fn test_settings_file() {
        let settings = Settings {
            storage: vec![Storage::Local { root: "d".into() }],
            subscribe: "https://example.com".to_string(),
        };

        settings.save_to_file(SETTINGS).unwrap();
    }
}
