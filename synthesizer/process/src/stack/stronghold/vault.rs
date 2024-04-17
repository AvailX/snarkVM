use std::path::PathBuf;

use console::program::{anyhow, Request, Result,ProgramID,Identifier,Value,ValueType,Field};
use console::prelude::Network;

use iota_stronghold::procedures::Curve;
use tauri_plugin_aleo_stronghold::{
    execute_procedure, remove_secret, save_secret, BytesDto, LocationDto, ProcedureDto,
    Slip10DeriveInputDto, StrongholdCollection,
};
use utilities::FromBytes;

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

    let result =match execute_procedure(hold, path, self.client, procedure).await {
        Ok(x) => Ok(x),
        Err(e) => Err(anyhow!("Failed to sign request: {}", e.to_string())),
    }?;

    Ok(Request::from_bytes_le(&result).map_err(|e| anyhow!("Failed to parse request: {}", e.to_string()))?)

   }
}
