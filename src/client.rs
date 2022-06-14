use std::cell::RefCell;

use cosmrs::rpc::{self, Client as RpcClient};
use tokio::runtime::Runtime;

use crate::{account::Account, consts, crypto, Error, Result};

// the client query impl
mod query;
// the client tx impl
mod tx;
pub mod types;

pub struct Client {
    rt: Runtime,
    rpc: rpc::HttpClient,
    enclave_pubk: RefCell<Option<Vec<u8>>>,
}

impl Client {
    pub(crate) fn init() -> Result<Client> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(Error::Runtime)?;

        let rpc_url = format!("{}:{}", consts::RPC_URL, consts::RPC_PORT);
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

    fn enclave_public_key(&self) -> Result<Vec<u8>> {
        if let Some(pubk) = self.enclave_pubk.borrow().as_ref() {
            return Ok(pubk.clone());
        }

        let key = self.query_tx_key()?;

        let pubk = crypto::cert::consenus_io_pubk(&key)?;

        self.enclave_pubk.replace(Some(pubk.clone()));

        Ok(pubk)
    }

    fn encrypt_tx_msg<M: serde::Serialize>(
        &self,
        msg: &M,
        code_hash: &[u8],
        account: &Account,
    ) -> Result<Vec<u8>> {
        let msg = serde_json::to_vec(msg).expect("msg cannot be serialized as JSON");
        let plaintext = [code_hash, msg.as_slice()].concat();
        let (prvk, pubk) = account.prv_pub_bytes();
        println!("Generating nonce...");
        let nonce = crypto::generate_nonce();
        println!("Fetching enclave public key...");
        let consensus_io_key = self.enclave_public_key()?;
        println!("Generating tx encryption key...");
        let encryption_key = crypto::encryption_key(&prvk, &consensus_io_key, &nonce)?;
        println!("Encrypting plaintext...");
        let ciphertext = crypto::encrypt(&encryption_key, &pubk, &plaintext, &nonce)?;
        Ok(ciphertext)
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
