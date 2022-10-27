#[test]
fn test_contract() {
    localsecret::env().run(test_contract_session).unwrap();
}

fn test_contract_session(client: &localsecret::Client) -> localsecret::Result<()> {
    let a = localsecret::a();

    let code_id = client
        .tx()
        .upload("target/test_contract.wasm.gz")
        .from(&a)
        .broadcast()?
        .into_inner();

    let contract = client
        .tx()
        .init(
            &test_contract::InitMsg {
                greeting: "YO".to_string(),
            },
            code_id,
        )
        .from(&a)
        .broadcast()?
        .into_inner();

    let greeting: test_contract::QueryAnswer = client.query_contract(
        &test_contract::QueryMsg::Greet {
            user: a.human_address(),
        },
        &contract,
        &a,
    )?;

    assert_eq!(
        test_contract::query::greet_user("YO", &a.human_address()),
        greeting.greet
    );

    let answer: test_contract::HandleAnswer = client
        .tx()
        .execute(
            &test_contract::HandleMsg::ModifyGreeting {
                greeting: "Hola".to_string(),
            },
            &contract,
        )
        .from(&a)
        .broadcast()?
        .into_inner();

    assert_eq!(
        answer,
        test_contract::HandleAnswer {
            old_greeting: "YO".to_string(),
            new_greeting: "Hola".to_string()
        }
    );

    let greeting: test_contract::QueryAnswer = client.query_contract(
        &test_contract::QueryMsg::Greet {
            user: a.human_address(),
        },
        &contract,
        &a,
    )?;

    assert_eq!(
        test_contract::query::greet_user("Hola", &a.human_address()),
        greeting.greet
    );

    Ok(())
}
