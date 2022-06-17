use cosmwasm_std::{Api, Extern, HumanAddr, Querier, QueryResult, StdError, StdResult, Storage};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, schemars::JsonSchema)]
pub enum QueryMsg {
    Greet { user: HumanAddr },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct QueryAnswer {
    pub greet: String,
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::Greet { user } => {
            try_greet_user(deps, user).and_then(|ans| cosmwasm_std::to_binary(&ans))
        }
    }
}

pub fn greet_user(greeting: &str, user: &HumanAddr) -> String {
    format!("{greeting} {user}, we have been waiting for you.")
}

fn try_greet_user<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    user: HumanAddr,
) -> StdResult<QueryAnswer> {
    let greeting = deps
        .storage
        .get(b"greeting")
        .map(String::from_utf8)
        .transpose()
        .map_err(|err| StdError::generic_err(format!("Invalid UTF-8 greeting: {err}")))?
        .ok_or(StdError::not_found("Greeting not found"))?;

    Ok(QueryAnswer {
        greet: greet_user(&greeting, &user),
    })
}
