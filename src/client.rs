use std::cell::RefCell;

use cosmrs::rpc::{self, Client as RpcClient};
use tokio::runtime::Runtime;

use crate::{
    account::Account,
    crypto::{self, Decrypter, Nonce},
    CodeHash, Error, Result,
};

// the client query impl
mod query;
// the client tx impl
pub(crate) mod tx;
pub mod types;

pub struct Client {
    rt: Runtime,
    rpc: rpc::HttpClient,
    enclave_pubk: RefCell<Option<crypto::Key>>,
}

impl Client {
    pub(crate) fn init(rpc_host: &str, rpc_port: u16) -> Result<Client> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(Error::Runtime)?;

        let rpc_url = format!("http://{}:{}", rpc_host, rpc_port);
        let rpc = rpc::HttpClient::new(rpc_url.as_str())?;
        let enclave_pubk = RefCell::default();

        Ok(Client {
            rt,
            rpc,
            enclave_pubk,
        })
    }

    pub(crate) fn wait_for_first_block(&self) -> Result<()> {
        self.block_on(wait_for_first_block(&self.rpc))
    }

    pub fn last_block_height(&self) -> Result<u32> {
        let res = self.block_on(rpc::Client::latest_block(&self.rpc))?;
        Ok(res.block.header.height.value() as _)
    }

    fn enclave_public_key(&self) -> Result<crypto::Key> {
        if let Some(pubk) = self.enclave_pubk.borrow().as_ref() {
            return Ok(*pubk);
        }

        let key = self.query_tx_key()?;

        let pubk = crypto::cert::consenus_io_pubk(&key)?;

        self.enclave_pubk.replace(Some(pubk));

        Ok(pubk)
    }

    fn encrypt_msg<M: serde::Serialize>(
        &self,
        msg: &M,
        code_hash: &CodeHash,
        account: &Account,
    ) -> Result<(Nonce, Vec<u8>)> {
        let msg = serde_json::to_vec(msg).expect("msg cannot be serialized as JSON");
        let plaintext = [code_hash.to_hex_string().as_bytes(), msg.as_slice()].concat();
        self.encrypt_msg_raw(&plaintext, account)
    }

    fn encrypt_msg_raw(&self, msg: &[u8], account: &Account) -> Result<(Nonce, Vec<u8>)> {
        let (prvk, pubk) = account.prv_pub_bytes();
        let io_key = self.enclave_public_key()?;
        let nonce_ciphertext = crypto::encrypt(&prvk, &pubk, &io_key, msg)?;
        Ok(nonce_ciphertext)
    }

    fn decrypter(&self, nonce: &Nonce, account: &Account) -> Result<Decrypter> {
        let (secret, _) = account.prv_pub_bytes();
        let io_key = self.enclave_public_key()?;
        Ok(Decrypter::new(secret, io_key, *nonce))
    }

    fn block_on<R, F>(&self, fut: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        self.rt.block_on(fut)
    }
}

async fn wait_for_first_block(client: &rpc::HttpClient) -> Result<()> {
    const HEALTHY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
    const BLOCK_ATTEMPT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
    const BLOCK_ATTEMPTS: usize = 20;

    client
        .wait_until_healthy(HEALTHY_TIMEOUT)
        .await
        .map_err(|_| Error::FirstBlockTimeout(HEALTHY_TIMEOUT.as_secs() as _))?;

    for _ in 0..BLOCK_ATTEMPTS {
        if (client.latest_block().await).is_ok() {
            return Ok(());
        }
        tokio::time::sleep(BLOCK_ATTEMPT_INTERVAL).await;
    }

    Err(Error::FirstBlockTimeout(
        (HEALTHY_TIMEOUT.as_millis()
            + (BLOCK_ATTEMPTS as u128 * BLOCK_ATTEMPT_INTERVAL.as_millis()))
            / 1000,
    ))
}
