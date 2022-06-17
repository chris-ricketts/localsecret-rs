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

## TODOs to get to v0.1.0

- [ ] Rustdoc comments.
- [ ] Expose options to speed up the block time before starting the docker container.
- [ ] When launching a docker container, look for an unused port to bind to the container's RPC port (allows parallel testing).
- [ ] Deserialize decrypted `cosmwasm_std::StdError` json when a TX delivery fails or contract returns an error.
- [ ] Tidy up `TxResponse<ResponseMsg>` API so it's easier to access the response message (or `cosmwasm_std::StdError`).

## Testing

You can see an example of a real contract being tested in: `tests/it.rs`

To run the tests:
```
❯ cargo install cargo-make // if you don't already have it
❯ cargo make test
```

## Contributing 

Issues and PRs are very welcome.

Please try to follow the [Conventional Commit specification](https://www.conventionalcommits.org/en/v1.0.0/) for commit messages and PRs. 


