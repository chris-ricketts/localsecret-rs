#[derive(serde::Serialize, serde::Deserialize)]
pub struct InitMsg {}

fn session(client: &localsecret::Client) -> localsecret::Result<()> {
    let a = localsecret::account::a();
    let code_id = client
        .upload_contract("examples/storage.wasm.gz", &a)?
        .into_inner();

    let contract_init = client
        .init_contract(&InitMsg {}, "foo", code_id, &a)
        .unwrap();

    Ok(())
}

fn main() {
    if let Err(err) = localsecret::env().run(session) {
        println!("localsecret session failed: {err}");
    }
}
