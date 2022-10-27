use cosmrs::{
    rpc::endpoint::broadcast::tx_commit::Response as BroadcastResponse,
    tx::{Body, Fee, Msg, MsgProto, SignDoc, SignerInfo},
};
use prost::Message;

use crate::{account::Account, client::types::Event, crypto::Decrypter, Error, Result, TxResponse};

pub mod builder {
    use std::{
        marker::PhantomData,
        path::{Path, PathBuf},
    };

    use cosmrs::{tx::Fee, Coin};

    use crate::{
        client::types::ContractInit, Account, CodeId, Contract, Error, Result, TxResponse,
    };

    pub type InitTx<'a> = Tx<'a, Unspecified, Unspecified>;

    pub trait Broadcast {
        type Response;

        fn broadcast(self) -> Result<TxResponse<Self::Response>>;
    }

    pub struct Unspecified;

    pub struct Upload {
        path: PathBuf,
    }

    pub struct Initialize<M> {
        msg: M,
        code_id: CodeId,
        label: Option<String>,
    }

    impl<M> Initialize<M> {
        fn label(&self) -> String {
            static UNNAMED_COUNT: std::sync::atomic::AtomicUsize =
                std::sync::atomic::AtomicUsize::new(0);

            self.label.clone().unwrap_or_else(|| {
                format!(
                    "unnamed_{}",
                    UNNAMED_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                )
            })
        }
    }

    pub struct Execute<M, R> {
        msg: M,
        contract: Contract,
        sent_funds: Vec<Coin>,
        _response: PhantomData<R>,
    }

    pub struct Tx<'a, Kind, From> {
        client: &'a crate::Client,
        kind: Kind,
        from: From,
        fee: Option<Fee>,
    }

    impl<'a, Kind, From> Tx<'a, Kind, From> {
        pub fn gas_fee(mut self, amount: u64, gas: u64) -> Self {
            let fee = super::gas::fee(amount, gas);
            self.fee = Some(fee);
            self
        }

        pub fn broadcast(self) -> Result<TxResponse<<Self as Broadcast>::Response>>
        where
            Self: Broadcast,
        {
            <Self as Broadcast>::broadcast(self)
        }
    }

    impl<'a, Kind> Tx<'a, Kind, Unspecified> {
        pub fn from(self, from: &Account) -> Tx<'a, Kind, Account> {
            Tx {
                client: self.client,
                kind: self.kind,
                from: from.clone(),
                fee: self.fee,
            }
        }
    }

    impl<'a, From> Tx<'a, Unspecified, From> {
        pub fn upload<P: AsRef<Path>>(self, path: P) -> Tx<'a, Upload, From> {
            Tx {
                client: self.client,
                kind: Upload {
                    path: path.as_ref().to_path_buf(),
                },
                from: self.from,
                fee: self.fee,
            }
        }

        pub fn init<M: serde::Serialize>(
            self,
            msg: M,
            code_id: CodeId,
        ) -> Tx<'a, Initialize<M>, From> {
            Tx {
                client: self.client,
                kind: Initialize {
                    msg,
                    code_id,
                    label: None,
                },
                from: self.from,
                fee: self.fee,
            }
        }

        pub fn execute<M: serde::Serialize, R: serde::de::DeserializeOwned>(
            self,
            msg: M,
            contract: &Contract,
        ) -> Tx<'a, Execute<M, R>, From> {
            Tx {
                client: self.client,
                kind: Execute {
                    msg,
                    contract: contract.clone(),
                    sent_funds: vec![],
                    _response: PhantomData,
                },
                from: self.from,
                fee: self.fee,
            }
        }
    }

    impl<'a, M, From> Tx<'a, Initialize<M>, From> {
        pub fn label(mut self, label: impl Into<String>) -> Self {
            self.kind.label = Some(label.into());
            self
        }
    }

    impl<'a, M, R, From> Tx<'a, Execute<M, R>, From> {
        pub fn send_uscrt(mut self, amount: u64) -> Self {
            if let Some(coin) = self.kind.sent_funds.first_mut() {
                coin.amount += amount.into();
                return self;
            }

            let coin = Coin {
                denom: "uscrt".parse().expect("safe: correct denom"),
                amount: amount.into(),
            };

            self.kind.sent_funds.push(coin);
            self
        }
    }

    impl<'a> Broadcast for Tx<'a, Upload, Account> {
        type Response = CodeId;

        fn broadcast(self) -> Result<TxResponse<Self::Response>> {
            let Tx {
                client,
                from,
                fee,
                kind,
            } = self;

            use cosmrs::secret_cosmwasm::MsgStoreCode;

            let wasm_byte_code = std::fs::read(&kind.path)
                .map_err(|err| Error::ContractFile(format!("{}", kind.path.display()), err))?;

            let msg = MsgStoreCode {
                sender: from.id(),
                wasm_byte_code,
                source: None,
                builder: None,
            };

            let gas = fee.unwrap_or_else(|| super::gas::upload());

            client.broadcast_msg(msg, &from, gas)
        }
    }

    impl<'a, M: serde::Serialize> Broadcast for Tx<'a, Initialize<M>, Account> {
        type Response = Contract;

        fn broadcast(self) -> Result<TxResponse<Self::Response>> {
            let Tx {
                client,
                kind,
                from,
                fee,
            } = self;

            use cosmrs::secret_cosmwasm::MsgInstantiateContract;

            let label = kind.label();

            if client.query_contract_label_exists(&label)? {
                return Err(Error::ContractLabelExists(label));
            }

            let code_hash = client.query_code_hash_by_code_id(kind.code_id)?;

            let (_, encrypted_msg) = client.encrypt_msg(&kind.msg, &code_hash, &from)?;

            let msg = MsgInstantiateContract {
                sender: from.id(),
                code_id: kind.code_id.into(),
                label,
                init_msg: encrypted_msg,
            };

            let gas = fee.unwrap_or_else(|| super::gas::init());

            client
                .broadcast_msg(msg, &from, gas)
                .map(|tx: TxResponse<ContractInit>| tx.map(|c| c.into_contract(code_hash)))
        }
    }

    impl<'a, M: serde::Serialize, R: serde::de::DeserializeOwned> Broadcast
        for Tx<'a, Execute<M, R>, Account>
    {
        type Response = R;

        fn broadcast(self) -> Result<TxResponse<Self::Response>> {
            let Tx {
                client,
                kind,
                from,
                fee,
            } = self;

            let (nonce, encrypted_msg) =
                client.encrypt_msg(&kind.msg, kind.contract.code_hash(), &from)?;

            use cosmrs::secret_cosmwasm::MsgExecuteContract;
            let msg = MsgExecuteContract {
                sender: from.id(),
                contract: kind.contract.id(),
                msg: encrypted_msg,
                sent_funds: kind.sent_funds,
            };

            let decrypter = client.decrypter(&nonce, &from)?;

            let gas = fee.unwrap_or_else(|| super::gas::exec());

            client
                .broadcast_msg_raw(msg, &from, gas)
                .map(|btr| btr.with_error_decrypt(decrypter))
                .and_then(Result::from)
                .and_then(|tx| tx.try_map(|cit| decrypter.decrypt(&cit)))
                .and_then(|tx| tx.try_map(|plt| String::from_utf8(plt)))
                .and_then(|tx| tx.try_map(|b64| base64::decode(b64)))
                .and_then(|tx| tx.try_map(|buf| serde_json::from_slice(&buf)))
        }
    }

    pub(crate) fn new(client: &crate::Client) -> Tx<'_, Unspecified, Unspecified> {
        Tx {
            client,
            kind: Unspecified,
            from: Unspecified,
            fee: None,
        }
    }
}

impl super::Client {
    pub fn tx(&self) -> builder::InitTx<'_> {
        builder::new(self)
    }

    fn broadcast_msg_raw<M>(
        &self,
        msg: M,
        account: &Account,
        gas: Fee,
    ) -> Result<BroadcastTxResponse>
    where
        M: Msg,
    {
        const HEIGHT_TIMEOUT_INTERVAL: u32 = 10;

        let last_block_height = self.last_block_height()?;
        let account_info = self.query_account_info(account)?;

        let body = Body::new(
            vec![msg.to_any()?],
            String::new(),
            last_block_height + HEIGHT_TIMEOUT_INTERVAL,
        );

        let auth_info = SignerInfo::single_direct(
            Some(account.signing_key().public_key()),
            account_info.sequence_number,
        )
        .auth_info(gas);

        let sign_doc = SignDoc::new(&body, &auth_info, &chain_id(), account_info.account_number)?;

        let tx_raw = sign_doc.sign(&account.signing_key())?;

        let res = self.block_on(tx_raw.broadcast_commit(&self.rpc))?;

        Ok(broadcast_tx_response(M::Proto::TYPE_URL, res))
    }

    fn broadcast_msg<T, M>(&self, msg: M, account: &Account, gas: Fee) -> Result<TxResponse<T>>
    where
        T: TryFrom<Vec<u8>>,
        crate::Error: From<T::Error>,
        M: Msg,
    {
        self.broadcast_msg_raw(msg, account, gas)
            .and_then(Result::from)
            .and_then(|tx| tx.try_map(T::try_from))
    }
}

enum BroadcastTxResponse {
    TxCheckError(String),
    TxDeliverErrorPlain(String),
    TxDeliverErrorEncrypted(String, Vec<u8>),
    Delivered(TxResponse<Vec<u8>>),
}

impl BroadcastTxResponse {
    fn into_result_with_decrypt(
        self,
        with_decrypt: Option<Decrypter>,
    ) -> Result<TxResponse<Vec<u8>>> {
        match self {
            BroadcastTxResponse::TxCheckError(err) => Err(Error::BroadcastTxCheck(err)),
            BroadcastTxResponse::TxDeliverErrorPlain(err) => Err(Error::BroadcastTxDeliver(err)),
            BroadcastTxResponse::TxDeliverErrorEncrypted(log, ciphertext) => Err(with_decrypt
                .map(|decrypter| decrypter.decrypt(&ciphertext))
                .transpose()?
                .map(|plaintext| String::from_utf8(plaintext))
                .transpose()?
                .map_or_else(|| Error::BroadcastTxDeliver(log), Error::BroadcastTxDeliver)),
            BroadcastTxResponse::Delivered(tx_res) => Ok(tx_res),
        }
    }

    fn with_error_decrypt<'a>(self, decrypt: Decrypter) -> WithErrorDecryption {
        WithErrorDecryption { decrypt, btr: self }
    }
}

impl From<BroadcastTxResponse> for Result<TxResponse<Vec<u8>>> {
    fn from(btr: BroadcastTxResponse) -> Self {
        btr.into_result_with_decrypt(None)
    }
}

struct WithErrorDecryption {
    decrypt: Decrypter,
    btr: BroadcastTxResponse,
}

impl From<WithErrorDecryption> for Result<TxResponse<Vec<u8>>> {
    fn from(wed: WithErrorDecryption) -> Self {
        wed.btr.into_result_with_decrypt(Some(wed.decrypt))
    }
}

fn try_extract_encrypted_error(log: &str) -> Option<Vec<u8>> {
    log.split_once("encrypted:")
        .and_then(|(_, rest)| rest.split_once(":"))
        .and_then(|(b64, _)| base64::decode(b64.trim()).ok())
}

fn broadcast_tx_response(msg_type: &str, bcast_res: BroadcastResponse) -> BroadcastTxResponse {
    if bcast_res.check_tx.code.is_err() {
        return BroadcastTxResponse::TxCheckError(bcast_res.check_tx.log.to_string());
    }

    if bcast_res.deliver_tx.code.is_err() {
        let log = bcast_res.deliver_tx.log.to_string();
        return if let Some(ciphertext) = try_extract_encrypted_error(&log) {
            BroadcastTxResponse::TxDeliverErrorEncrypted(log, ciphertext)
        } else {
            BroadcastTxResponse::TxDeliverErrorPlain(log)
        };
    }

    let gas_used = bcast_res.deliver_tx.gas_used.into();
    let events = bcast_res
        .deliver_tx
        .events
        .into_iter()
        .map(|e| {
            let attrs = e
                .attributes
                .into_iter()
                .map(|a| (a.key.to_string(), a.value.to_string()))
                .collect();
            Event {
                _type: e.type_str,
                attrs,
            }
        })
        .collect();

    let response = bcast_res
        .deliver_tx
        .data
        .and_then(|data| {
            use cosmrs::proto::cosmos::base::abci::v1beta1::TxMsgData;
            TxMsgData::decode(data.as_bytes())
                .expect("unexpected data in response")
                .data
                .into_iter()
                .find(|msg| msg.msg_type == msg_type)
        })
        .map(|msg| msg.data);

    BroadcastTxResponse::Delivered(TxResponse {
        response,
        gas_used,
        events,
    })
}

fn chain_id() -> cosmrs::tendermint::chain::Id {
    crate::consts::CHAIN_ID.parse().expect("invalid chain id")
}

mod gas {
    use cosmrs::tx::Fee;

    use crate::consts;

    fn uscrt(amount: u64) -> cosmrs::Coin {
        let denom = consts::COIN_DENOM
            .parse()
            .expect("invalid coin denomination");

        cosmrs::Coin {
            denom,
            amount: amount.into(),
        }
    }

    pub fn fee(amount: u64, gas: u64) -> Fee {
        Fee::from_amount_and_gas(uscrt(amount), gas)
    }

    pub fn upload() -> Fee {
        fee(consts::UPLOAD_AMOUNT, consts::UPLOAD_GAS)
    }

    pub fn init() -> Fee {
        fee(consts::INIT_AMOUNT, consts::INIT_GAS)
    }

    pub fn exec() -> Fee {
        fee(consts::EXEC_AMOUNT, consts::EXEC_GAS)
    }
}
