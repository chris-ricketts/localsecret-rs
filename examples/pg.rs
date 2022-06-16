fn main() {
    if let Err(err) = localsecret::env().run(session) {
        println!("localsecret session failed: {err}");
    }
}

fn session(client: &localsecret::Client) -> localsecret::Result<()> {
    let a = localsecret::a();

    let code_id = client
        .upload_contract("examples/storage.wasm.gz", &a)?
        .into_inner();

    let contract = client
        .init_contract(&InitMsg {}, "foo", code_id, &a)?
        .into_inner();

    let tx_res = execute_storage_contract(
        client,
        &contract,
        &a,
        HandleMsg::ItemWrite {
            data: random_config(),
        },
    )?;

    println!("Msg execution used {} gas", tx_res.gas_used);

    Ok(())
}

fn execute_storage_contract(
    client: &localsecret::Client,
    contract: &localsecret::Contract,
    account: &localsecret::Account,
    msg: HandleMsg,
) -> localsecret::Result<localsecret::TxResponse<HandleAnswer>> {
    client.exec_contract(&msg, contract, account)
}

// Testing with gas study storage contract for now

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InitMsg {}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub address: cosmwasm_std::HumanAddr,
    pub number: cosmwasm_std::Uint128,
    pub other_data: String,
    pub array: Vec<u64>,
}

fn random_config() -> Config {
    use localsecret::account::Account;
    use nanorand::rand::Rng;

    let mut seed = [0; 64];
    let mut rng = nanorand::rand::ChaCha8::new();
    rng.fill_bytes(&mut seed);

    Config {
        address: Account::from_seed(seed).human_address(),
        number: cosmwasm_std::Uint128(1_000_000),
        other_data: "Yo".to_string(),
        array: vec![1, 2, 4, 5],
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ItemWrite { data: Config },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    Answer {},
}
