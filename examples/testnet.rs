use localsecret::Contract;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    TokenInfo {},
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    TokenInfo {
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: Option<cosmwasm_std::Uint128>,
    },
}

fn main() {
    localsecret::env()
        .external()
        .external_rpc_host("https://secret-4.api.trivium.network")
        .enclave_key_hex("083b1a03661211d5a4cc8d39a77795795862f7730645573b2bcc2c1920c53c04")
        .run(|client| {
            let contract = Contract::try_from_address_with_code_hash(
                "secret1k0jntykt7e4g3y88ltc60czgjuqdy4c9e8fzek",
                "AF74387E276BE8874F07BEC3A87023EE49B0E7EBE08178C49D0A49C3C98ED60E",
            )?;

            let ans: QueryAnswer =
                client.query_contract(&QueryMsg::TokenInfo {}, &contract, &localsecret::a())?;

            println!("{ans:#?}");

            Ok(())
        })
        .unwrap();
}
