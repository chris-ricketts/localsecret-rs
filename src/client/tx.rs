use std::path::Path;

use cosmrs::{
    rpc::endpoint::broadcast::tx_commit::Response as BroadcastResponse,
    tx::{Body, Fee, Msg, MsgProto, SignDoc, SignerInfo},
};
use prost::Message;

use super::types::{FromMsgData, MsgDataExt};
use crate::{
    account::Account, client::types::Event, CodeId, ContractInit, Error, Result, TxResponse,
};

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

    #[allow(dead_code)] // still need to do contract exec
    pub fn exec() -> Fee {
        fee(consts::EXEC_AMOUNT, consts::EXEC_GAS)
    }
}

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
    ) -> Result<TxResponse<ContractInit>>
    where
        M: serde::Serialize,
        // R: serde::de::DeserializeOwned,
    {
        use cosmrs::secret_cosmwasm::MsgInstantiateContract;

        if self.query_contract_label_exists(label)? {
            return Err(Error::ContractLabelExists(label.to_owned()));
        }

        let code_hash = self.query_code_hash_by_code_id(code_id)?;

        println!("Encrypting init msg");
        let encrypted_msg = self.encrypt_tx_msg(msg, &code_hash, account)?;

        let msg = MsgInstantiateContract {
            sender: account.id(),
            code_id: code_id.into(),
            label: label.to_string(),
            init_msg: encrypted_msg,
        };

        println!("Broadcasting instantiate msg");
        self.broadcast_msg(msg, account, gas::init())
    }

    fn broadcast_msg<T, M>(&self, msg: M, account: &Account, gas: Fee) -> Result<TxResponse<T>>
    where
        T: FromMsgData,
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

        broadcast_tx_response(M::Proto::TYPE_URL, res)
    }
}

fn broadcast_tx_response<T>(msg_type: &str, bcast_res: BroadcastResponse) -> Result<TxResponse<T>>
where
    T: FromMsgData,
{
    if bcast_res.check_tx.code.is_err() {
        return Err(Error::BroadcastTxCheck(bcast_res.check_tx.log.to_string()));
    }

    if bcast_res.deliver_tx.code.is_err() {
        return Err(Error::BroadcastTxDeliver(
            bcast_res.deliver_tx.log.to_string(),
        ));
    }

    println!("{bcast_res:#?}");

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
            TxMsgData::decode(data.value().as_slice())
                .expect("unexpected data in response")
                .data
                .into_iter()
                .find(|msg| msg.msg_type == msg_type)
        })
        .map(|msg| msg.parse())
        .transpose()?;

    Ok(TxResponse {
        response,
        gas_used,
        events,
    })
}

fn chain_id() -> cosmrs::tendermint::chain::Id {
    crate::consts::CHAIN_ID.parse().expect("invalid chain id")
}
