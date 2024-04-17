use std::path::PathBuf;

use console::program::{anyhow,Result};
use tauri_plugin_aleo_stronghold::{
    get_store_record, remove_store_record, save_store_record, BytesDto, StrongholdCollection,
};

pub struct Store {
    path: String,
    client: BytesDto,
}

impl Store {
    pub fn new(path: String, client: BytesDto) -> Self {
        Self { path, client }
    }

    pub async fn get(
        self,
        key: String,
        hold: &StrongholdCollection,
    ) -> Result<Option<Vec<u8>>> {
        let path = PathBuf::from(self.path);
        match get_store_record(hold, path, self.client, key).await {
            Ok(record) => Ok(record),
            Err(e) =>  Err(anyhow!("Failed to get record: {}", e.to_string())),
        }
    }

    pub async fn insert(
        self,
        key: String,
        value: Vec<u8>,
        hold: &StrongholdCollection,
    ) -> Result<Option<Vec<u8>>> {
        let path = PathBuf::from(self.path);

        match save_store_record(hold, path, self.client, key, value, None).await {
            Ok(record) => Ok(record),
            Err(e) =>  Err(anyhow!("Failed to save record: {}", e.to_string())),
        }
    }

    pub async fn remove(
        self,
        key: String,
        hold: &StrongholdCollection,
    ) -> Result<Option<Vec<u8>>> {
        let path = PathBuf::from(self.path);

        match remove_store_record(hold, path, self.client, key).await {
            Ok(record) => Ok(record),
            Err(e) =>  Err(anyhow!("Failed to remove record: {}", e.to_string())),
        }
    }
}
