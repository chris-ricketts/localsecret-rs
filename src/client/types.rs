use std::collections::HashMap;

use cosmrs::AccountId;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Failed to parse code id: {0}")]
    CodeId(#[from] ParseCodeIdError),
    #[error("Failed to parse contract intitialisation: {0}")]
    ContractInit(#[from] ParseContractInitError),
}

#[derive(Debug, Clone, Copy)]
pub struct CodeId(u64);

impl std::fmt::Display for CodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CodeId> for u64 {
    fn from(ci: CodeId) -> Self {
        ci.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseCodeIdError {
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}

impl TryFrom<Vec<u8>> for CodeId {
    type Error = ParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        fn parse(value: &[u8]) -> Result<CodeId, ParseCodeIdError> {
            let s = std::str::from_utf8(value)?;
            let code_id = s.parse()?;
            Ok(CodeId(code_id))
        }
        parse(&value).map_err(Self::Error::from)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ContractInit(AccountId);

impl ContractInit {
    pub fn into_contract(self, code_hash: CodeHash) -> Contract {
        Contract {
            id: self.0,
            code_hash,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseContractInitError {
    #[error("Failed to parse address bytes: {0}")]
    Address(#[from] cosmrs::ErrorReport),
}

impl TryFrom<Vec<u8>> for ContractInit {
    type Error = ParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        fn parse(value: &[u8]) -> Result<ContractInit, ParseContractInitError> {
            let id = AccountId::new(crate::consts::CHAIN_PREFIX, &value)?;
            Ok(ContractInit(id))
        }
        parse(&value).map_err(Self::Error::from)
    }
}

#[derive(Debug, Clone)]
pub struct Contract {
    id: AccountId,
    code_hash: CodeHash,
}

impl Contract {
    pub fn human_address(&self) -> cosmwasm_std::HumanAddr {
        self.id.to_string().into()
    }

    pub fn code_hash_string(&self) -> String {
        self.code_hash.to_hex_string()
    }

    pub(crate) fn id(&self) -> AccountId {
        self.id.clone()
    }

    pub(crate) fn code_hash(&self) -> &CodeHash {
        &self.code_hash
    }
}

#[derive(Debug, Clone)]
pub struct CodeHash(Vec<u8>);

impl From<Vec<u8>> for CodeHash {
    fn from(b: Vec<u8>) -> Self {
        CodeHash(b)
    }
}

impl CodeHash {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn to_hex_string(&self) -> String {
        use core::fmt::Write;
        let mut s = String::with_capacity(2 * self.0.len());
        for byte in self.as_bytes() {
            write!(s, "{:02X}", byte).expect("could not write byte as hex to string");
        }
        s
    }
}

impl std::fmt::Display for CodeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

#[derive(Debug)]
pub struct Event {
    pub(crate) _type: String,
    pub(crate) attrs: HashMap<String, String>,
}

#[derive(Debug)]
pub struct TxResponse<T> {
    pub response: Option<T>,
    pub gas_used: u64,
    pub(crate) events: Vec<Event>,
}

impl<T> TxResponse<T> {
    pub fn event_attr(&self, event_type: &str, attr: &str) -> Option<&str> {
        self.events
            .iter()
            .find(|e| e._type == event_type)
            .and_then(|e| e.attrs.get(attr))
            .map(String::as_str)
    }

    /// panics if the response is `None`
    pub fn into_inner(self) -> T {
        self.response.unwrap()
    }

    pub(crate) fn map<U, F: FnOnce(T) -> U>(self, f: F) -> TxResponse<U> {
        TxResponse {
            response: self.response.map(f),
            gas_used: self.gas_used,
            events: self.events,
        }
    }

    pub(crate) fn try_map<U, E, F>(self, f: F) -> crate::Result<TxResponse<U>>
    where
        crate::Error: From<E>,
        F: FnOnce(T) -> Result<U, E>,
    {
        let response = self.response.map(|t| f(t)).transpose()?;
        Ok(TxResponse {
            response,
            gas_used: self.gas_used,
            events: self.events,
        })
    }
}

#[derive(Debug)]
pub(crate) struct AccountInfo {
    pub account_number: u64,
    pub sequence_number: cosmrs::tx::SequenceNumber,
}

impl From<cosmrs::proto::cosmos::auth::v1beta1::BaseAccount> for AccountInfo {
    fn from(ba: cosmrs::proto::cosmos::auth::v1beta1::BaseAccount) -> Self {
        AccountInfo {
            account_number: ba.account_number,
            sequence_number: ba.sequence,
        }
    }
}
