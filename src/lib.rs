use anyhow::Result;
use api::ApiManager;
use arg::CliInput;
use clap::{ArgMatches, Command};
use client::Client;
use std::path::PathBuf;

pub mod api;
pub mod arg;
pub mod client;
pub mod cmd;

#[cfg(target_arch = "wasm32")]
pub mod wasm_exports;

pub async fn run(p: PathBuf, client: &Client, raw_input: Vec<String>) -> Result<String> {
    let api_manager = ApiManager::new(p)?;

    let matches = get_matches(cmd::cmd(), raw_input.clone())?;

    match matches.subcommand() {
        Some(("api", matches)) => {
            let args = if let Some(args) = matches.get_many::<String>("args") {
                args.cloned().collect()
            } else {
                vec![]
            };
            let input = CliInput::new(args)?;
            let _ = get_matches(cmd::cmd_api(&api_manager, &input), raw_input.clone())?;

            // Invoke the api call
            let ctx = api_manager.build_ctx(&input)?;
            let res = ctx.execute(&client).await?;
            return Ok(res);
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn get_matches(cmd: Command, input: Vec<String>) -> Result<ArgMatches> {
    use anyhow::anyhow;
    use clap::builder::Styles;
    let cmd = cmd.styles(Styles::plain());
    cmd.try_get_matches_from(input)
        .map_err(|e| anyhow!("{}", e.render().ansi().to_string().replace("\\n", "\n")))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_matches(cmd: Command, input: Vec<String>) -> Result<ArgMatches> {
    Ok(cmd.get_matches_from(input))
}
