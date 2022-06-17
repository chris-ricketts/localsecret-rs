use cosmwasm_std::{
    Api, Env, Extern, HandleResponse, HandleResult, Querier, StdError, StdResult, Storage,
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ModifyGreeting { greeting: String },
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Clone, Debug)]
pub struct HandleAnswer {
    pub old_greeting: String,
    pub new_greeting: String,
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::ModifyGreeting { greeting } => modify_greeting(deps, greeting),
    }
}

pub fn modify_greeting<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    new_greeting: String,
) -> StdResult<HandleResponse> {
    let old_greeting = deps
        .storage
        .get(b"greeting")
        .map(String::from_utf8)
        .transpose()
        .map_err(|err| StdError::generic_err(format!("Invalid UTF-8 greeting: {err}")))?
        .ok_or(StdError::not_found("Greeting not found"))?;

    deps.storage.set(b"greeting", new_greeting.as_bytes());

    Ok(HandleResponse {
        data: cosmwasm_std::to_binary(&HandleAnswer {
            old_greeting,
            new_greeting,
        })
        .map(Some)?,
        ..Default::default()
    })
}
