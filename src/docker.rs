use crate::{client::Client, consts, Result};

pub fn docker_run<F>(f: F) -> Result<()>
where
    F: FnOnce(&Client) -> Result<()> + std::panic::UnwindSafe,
{
    let docker_args = [
        "-d",
        "-p",
        &format!("{}:{}", consts::DEFAULT_RPC_PORT, consts::DEFAULT_RPC_PORT),
        "-p",
        &format!("{}:{}", consts::FAUCET_PORT, consts::FAUCET_PORT),
        consts::DOCKER_IMAGE,
    ];

    cosmrs::dev::docker_run(docker_args, || {
        let client = Client::init(consts::DEFAULT_RPC_HOST, consts::DEFAULT_RPC_PORT)?;
        client.wait_for_first_block()?;
        f(&client)
    })
}
