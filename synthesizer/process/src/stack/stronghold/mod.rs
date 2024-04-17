pub mod client;
pub mod store;
pub mod vault;

use std::path::PathBuf;
use console::program::{anyhow,Result};
use tauri_plugin_aleo_stronghold::{destroy, save, StrongholdCollection};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Stronghold {
    pub(crate) path: String,
}

impl Stronghold {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    pub async fn save(&self, collection: &StrongholdCollection) -> Result<()> {
        let path = PathBuf::from(self.path.clone());
        match save(collection, path).await {
            Ok(x) => Ok(x),
            Err(e) => Err(anyhow!("Failed to save stronghold: {}", e.to_string())),
        }
    }

    pub async fn destroy(&self, collection: &StrongholdCollection) -> Result<()> {
        let path = PathBuf::from(self.path.clone());
        match destroy(collection, path).await {
            Ok(x) => Ok(x),
            Err(e) => Err(anyhow!("Failed to destroy stronghold: {}", e.to_string())),
        }
    }
}
