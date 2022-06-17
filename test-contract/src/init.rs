use cosmwasm_std::{Api, Env, Extern, InitResponse, InitResult, Querier, Storage};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, schemars::JsonSchema)]
pub struct InitMsg {
    pub greeting: String,
}

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> InitResult {
    deps.storage.set(b"greeting", msg.greeting.as_bytes());
    Ok(InitResponse::default())
}
