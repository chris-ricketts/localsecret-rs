use std::collections::HashMap;

pub(crate) trait FromMsgData: Sized {
    type Error: Into<ParseMsgResponseError>;

    fn try_from_msg_data(msg_data: &[u8]) -> Result<Self, Self::Error>;
}

pub(crate) trait MsgDataExt {
    fn parse<T>(&self) -> Result<T, ParseMsgResponseError>
    where
        T: FromMsgData,
        T::Error: Into<ParseMsgResponseError>;
}

impl MsgDataExt for cosmrs::proto::cosmos::base::abci::v1beta1::MsgData {
    fn parse<T>(&self) -> Result<T, ParseMsgResponseError>
    where
        T: FromMsgData,
        T::Error: Into<ParseMsgResponseError>,
    {
        T::try_from_msg_data(&self.data).map_err(T::Error::into)
    }
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

impl FromMsgData for CodeId {
    type Error = ParseCodeIdError;

    fn try_from_msg_data(msg_data: &[u8]) -> Result<Self, Self::Error> {
        let s = std::str::from_utf8(msg_data)?;
        let code_id = s.parse()?;
        Ok(CodeId(code_id))
    }
}

#[derive(Debug, Clone)]
pub struct ContractInit;

// TODO: Stil working out how to parse from instantiate response
impl FromMsgData for ContractInit {
    type Error = ParseMsgResponseError;

    fn try_from_msg_data(msg_data: &[u8]) -> Result<Self, Self::Error> {
        println!("{:?}", msg_data);
        Ok(ContractInit)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseMsgResponseError {
    #[error("Failed to parse code id: {0}")]
    CodeId(#[from] ParseCodeIdError),
    #[error("Failed to parse contract intitialisation")]
    ContractInit,
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
