pub mod account;
pub mod client;
pub(crate) mod consts;
pub(crate) mod crypto;

mod docker;

pub use account::{a, b, c, d, Account};
pub use client::{
    tx::builder::*,
    types::{CodeHash, CodeId, Contract, TxResponse},
    Client,
};
pub use error::Error;

pub type Result<T> = std::result::Result<T, error::Error>;

pub struct LocalSecret;

pub fn env() -> LocalSecret {
    LocalSecret
}

impl LocalSecret {
    pub fn run<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&Client) -> Result<()> + std::panic::UnwindSafe,
    {
        docker::docker_run(f)
    }
}

pub mod error {
    pub use crate::client::types::ParseError;
    pub use crate::crypto::cert::MalformedError;
    pub use crate::crypto::CryptoError;

    // TODO: Too many top level errors, kill dem
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("Failed to initialise tokio runtime: {0}")]
        Runtime(std::io::Error),
        #[error("RPC failure: {0}")]
        Rpc(#[from] cosmrs::rpc::Error),
        #[error("Failed to read contract file: {0} - {1}")]
        ContractFile(String, std::io::Error),
        #[error("Contract with label {0} already deployed")]
        ContractLabelExists(String),
        #[error("Contract Info not found for code id: {0}")]
        ContractInfoNotFound(crate::CodeId),
        #[error("Timed out waiting for first block after {0} seconds")]
        FirstBlockTimeout(u128),
        #[error("ABCI Query failed: {0}")]
        AbciQuery(String),
        #[error("Decoding protobuf response failed: {0}")]
        ProtobufDecode(#[from] prost::DecodeError),
        #[error("CosmWasm Error: {0}")]
        CosmwWasm(#[from] cosmwasm_std::StdError),
        #[error("Account {0} not found")]
        AccountNotFound(cosmwasm_std::HumanAddr),
        #[error("CosmRs error: {0}")]
        CosmRs(#[from] cosmrs::ErrorReport),
        #[error("Broadcast error - check tx failed: {0}")]
        BroadcastTxCheck(String),
        #[error("Broadcast error - deliver tx failed: {0}")]
        BroadcastTxDeliver(String),
        #[error("Failed to parse message response: {0}")]
        ParseMsgResponse(#[from] ParseError),
        #[error("Parsing TEE cert failed: {0}")]
        ParseTEECert(#[from] MalformedError),
        #[error("Cryptographic error: {0}")]
        Crypto(#[from] CryptoError),
        #[error("Failed to deserialise JSON response: {0}")]
        Json(#[from] serde_json::Error),
        #[error("Failed to decode Base64 response: {0}")]
        Base64(#[from] base64::DecodeError),
        #[error(transparent)]
        Utf8(#[from] std::string::FromUtf8Error),
    }
}
