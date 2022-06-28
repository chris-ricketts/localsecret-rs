use std::path::Path;

use cosmrs::{
    rpc::endpoint::broadcast::tx_commit::Response as BroadcastResponse,
    tx::{Body, Fee, Msg, MsgProto, SignDoc, SignerInfo},
};
use prost::Message;

use super::types::ContractInit;
use crate::{
    account::Account, client::types::Event, crypto::Decrypter, CodeId, Contract, Error, Result,
    TxResponse,
};

impl super::Client {
    pub fn upload_contract<P: AsRef<Path>>(
        &self,
        path: P,
        account: &Account,
    ) -> Result<TxResponse<CodeId>> {
        use cosmrs::secret_cosmwasm::MsgStoreCode;

        let wasm_byte_code = std::fs::read(&path)
            .map_err(|err| Error::ContractFile(format!("{}", path.as_ref().display()), err))?;

        let msg = MsgStoreCode {
            sender: account.id(),
            wasm_byte_code,
            source: None,
            builder: None,
        };

        self.broadcast_msg(msg, account, gas::upload())
    }

    pub fn init_contract<M>(
        &self,
        msg: &M,
        label: &str,
        code_id: CodeId,
        account: &Account,
    ) -> Result<TxResponse<Contract>>
    where
        M: serde::Serialize,
    {
        use cosmrs::secret_cosmwasm::MsgInstantiateContract;

        if self.query_contract_label_exists(label)? {
            return Err(Error::ContractLabelExists(label.to_owned()));
        }

        let code_hash = self.query_code_hash_by_code_id(code_id)?;

        let (_, encrypted_msg) = self.encrypt_msg(msg, &code_hash, account)?;

        let msg = MsgInstantiateContract {
            sender: account.id(),
            code_id: code_id.into(),
            label: label.to_string(),
            init_msg: encrypted_msg,
        };

        self.broadcast_msg(msg, account, gas::init())
            .map(|tx: TxResponse<ContractInit>| tx.map(|c| c.into_contract(code_hash)))
    }

    pub fn exec_contract<M, R>(
        &self,
        msg: &M,
        contract: &Contract,
        account: &Account,
    ) -> Result<TxResponse<R>>
    where
        M: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        use cosmrs::secret_cosmwasm::MsgExecuteContract;

        let (nonce, encrypted_msg) = self.encrypt_msg(msg, contract.code_hash(), account)?;

        let msg = MsgExecuteContract {
            sender: account.id(),
            contract: contract.id(),
            msg: encrypted_msg,
        };

        let decrypter = self.decrypter(&nonce, account)?;

        self.broadcast_msg_raw(msg, account, gas::exec())
            .map(|btr| btr.with_error_decrypt(decrypter))
            .and_then(Result::from)
            .and_then(|tx| tx.try_map(|cit| decrypter.decrypt(&cit)))
            .and_then(|tx| tx.try_map(|plt| String::from_utf8(plt)))
            .and_then(|tx| tx.try_map(|b64| base64::decode(b64)))
            .and_then(|tx| tx.try_map(|buf| serde_json::from_slice(&buf)))
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

    fn fee(amount: u64, gas: u64) -> Fee {
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
