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

use std::path::PathBuf;

use snarkvm_console::program::{anyhow, Field, FromBytes, Identifier, ProgramID, Request, Result, Value, ValueType};
use snarkvm_console::network::Network;

use iota_stronghold::procedures::Curve;
use tauri_plugin_aleo_stronghold::{
    execute_procedure, remove_secret, save_secret, BytesDto, LocationDto, ProcedureDto,
    Slip10DeriveInputDto, StrongholdCollection,
};



/// A key-value storage that allows create, update and delete operations.
/// It does not allow reading the data, so one of the procedures must be used to manipulate
/// the stored data, allowing secure storage of secrets.
#[derive(Clone)]
pub struct Vault {
    path: String,
    client: BytesDto,
    name: BytesDto,
}

impl Vault {
    pub fn new(path: &str, client: BytesDto, name: BytesDto) -> Self {
        Self {
            path: path.to_string(),
            client,
            name,
        }
    }

    pub async fn insert(
        self,
        value: &[u8],
        hold: &StrongholdCollection,
        record_path: &str,
    ) -> Result<()> {
        let path = PathBuf::from(self.path);
        let record_path = BytesDto::Text(record_path.to_string());

        match save_secret(
            hold,
            path,
            self.client,
            self.name,
            record_path,
            value.to_vec(),
        )
        .await
        {
            Ok(x) => Ok(x),
            Err(e) => Err(anyhow!("Failed to save record: {}", e.to_string())),
        }
    }

    pub async fn remove_secret(
        self,
        hold: &StrongholdCollection,
        record_path: &str,
    ) -> Result<()> {
        let path = PathBuf::from(self.path);
        let record_path = BytesDto::Text(record_path.to_string());

        match remove_secret(hold, path, self.client, self.name, record_path).await {
            Ok(x) => Ok(x),
            Err(e) => Err(anyhow!("Failed to remove stronghold: {}", e.to_string())),
        }
    }

   pub async fn aleo_sign_request<N:Network>(
    self,
    hold: &StrongholdCollection,
    pk_path: &str,
    program_id: ProgramID<N>,
    function_name: Identifier<N>,
    inputs: Vec<Value<N>>,
    input_types: Vec<ValueType<N>>,
    root_tvk: Option<Field<N>>,
    is_root: bool,
   )-> Result<Request<N>>{
    let path = PathBuf::from(self.path);
    let record_path = BytesDto::Text(pk_path.to_string());
    let location = LocationDto::Generic {
        vault: self.name,
        record: record_path,
    };

    let procedure = ProcedureDto::<N>::AleoSignRequest { program_id, function_name, inputs, input_types, root_tvk, is_root, private_key: location };

    let result = match execute_procedure(hold, path, self.client, procedure).await {
        Ok(x) => Ok(x),
        Err(e) => Err(anyhow!("Failed to sign request: {}", e.to_string())),
    }?;


    Request::from_bytes_le(&result).map_err(|e| anyhow!("Failed to parse request: {}", e.to_string()))

   }
}
