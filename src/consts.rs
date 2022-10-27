pub static CHAIN_PREFIX: &str = "secret";
pub static CHAIN_ID: &str = "secretdev-1";
pub static SCRT_DERIVATION_PATH: &str = "m/44'/529'/0'/0/0";
pub static DEFAULT_RPC_HOST: &str = "localhost";
pub static DOCKER_IMAGE: &str = "ghcr.io/scrtlabs/localsecret";
pub static COIN_DENOM: &str = "uscrt";

pub const DEFAULT_RPC_PORT: u16 = 26657;
pub const FAUCET_PORT: u16 = 5000;
pub const UPLOAD_GAS: u64 = 1_000_000;
pub const UPLOAD_AMOUNT: u64 = 250_000;
pub const INIT_GAS: u64 = 500_000;
pub const INIT_AMOUNT: u64 = 125_000;
pub const EXEC_GAS: u64 = 200_000;
pub const EXEC_AMOUNT: u64 = 50_000;
