# localsecret-rs: Rust x LocalSecret docker

This crate provides a convenient way to test contracts in the LocalSecret docker container:

```rust
#[test]
fn test_contract() -> Result<()> {
    // Auto-spins up LocalSecret docker container and connects client over Tendermint RPC
    // Auto-tears down the contrainer when the 'session' is ended (even if it panics)
    localsecret::env().run(|client| {
        // access genesis accounts
        let a = localsecret::a();

        let code_id = client.upload_contract("target/your_contract.wasm.gz", &a)?.into_inner();

        let init_msg = your_contract::InitMsg { .. };

        let contract = client.init_contract(&init_msg, "your_contract", &code_id, &a)?.into_inner();

        let handle_msg = your_contract::HandleMsg::Foo { .. };

        let handle_ans = client.exec_contract(handle_msg, &contract, &a)?.into_inner();

        assert_eq!(handle_ans.foo, ...);

        let query_msg = your_contract::QueryMsg::Foo { .. };

        let query_ans = client.query_contract(query_msg, &contract, &a)?;

        assert_eq!(query_ans.foo, ...);

        Ok(())
    })
}
```

## Testing

You can see an example of a real contract being tested in: `tests/it.rs`

To run the tests:
```
❯ cargo install cargo-make // if you don't already have it
❯ cargo make test
```
