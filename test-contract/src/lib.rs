pub mod handle;
pub mod init;
pub mod query;

pub use handle::{HandleAnswer, HandleMsg};
pub use init::InitMsg;
pub use query::{QueryAnswer, QueryMsg};

pub mod contract {
    use cosmwasm_std::{Api, Env, Extern, HandleResult, InitResult, Querier, QueryResult, Storage};

    use crate::handle;
    use crate::init;
    use crate::query;

    pub fn init<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        env: Env,
        msg: init::InitMsg,
    ) -> InitResult {
        init::init(deps, env, msg)
    }

    pub fn handle<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        env: Env,
        msg: handle::HandleMsg,
    ) -> HandleResult {
        handle::handle(deps, env, msg)
    }

    pub fn query<S: Storage, A: Api, Q: Querier>(
        deps: &Extern<S, A, Q>,
        msg: query::QueryMsg,
    ) -> QueryResult {
        query::query(deps, msg)
    }
}

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);
