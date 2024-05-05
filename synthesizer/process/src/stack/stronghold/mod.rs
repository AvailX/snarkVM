// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

pub async fn init_stronghold(
    password: &str,
) -> AvailResult<(StrongholdCollection, Stronghold, Client)> {
    let path = app_root(
        AppDataType::UserData,
        &AppInfo {
            name: "avail_wallet",
            author: "Avail",
        },
    )?;

    let pbkdf = PasswordHashFunction(Box::new(|password| {
        // Hash the password here with e.g. argon2, blake2b or any other secure algorithm
        // Here is an example implementation using the `rust-argon2` crate for hashing the password

        use argon2::{hash_raw, Config, Variant, Version};

        let config = Config {
            lanes: 4,
            mem_cost: 10_000,
            time_cost: 10,
            variant: Variant::Argon2id,
            version: Version::Version13,
            ..Default::default()
        };

        let salt = "your-salt".as_bytes();

        let key = hash_raw(password.as_ref(), salt, &config).expect("failed to hash password");

        key.to_vec()
    }));

    let vault_path = format! {"{}/vault.hold",path.to_str().unwrap()};
    let vault_path_buf = PathBuf::from(vault_path.clone());
    let hold = StrongholdCollection::default();

    let stronghold =
        match initialize(&hold, pbkdf, vault_path_buf.clone(), password.to_string()).await {
            Ok(_) => Ok(Stronghold::new(&vault_path)),
            Err(e) => Err(AvailError::new(
                AvailErrorType::Internal,
                e.to_string(),
                "Failed to initiliaze key stronghold".to_string(),
            )),
        }?;

    let client_name = BytesDto::Text("com.avail.stronghold".to_string());

    let client = match load_client(&hold, vault_path_buf.clone(), client_name).await {
        Ok(x) => {
            println!("client loaded");
            Ok(Client::new(
                &vault_path,
                BytesDto::Text("com.avail.stronghold".to_string()),
            ))
        }
        Err(_) => match create_client(
            &hold,
            vault_path_buf,
            BytesDto::Text("com.avail.stronghold".to_string()),
        )
        .await
        {
            Ok(_) => {
                println!("client created");
                Ok(Client::new(
                    &vault_path,
                    BytesDto::Text("com.avail.stronghold".to_string()),
                ))
            }
            Err(e) => Err(AvailError::new(
                AvailErrorType::Internal,
                e.to_string(),
                "Failed to create client".to_string(),
            )),
        },
    }?;

    Ok((hold, stronghold, client))
}