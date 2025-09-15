use std::{path::PathBuf, str::FromStr};

use anyhow::Result;
use azure::api::ApiManager;
use azure::arg::CliInput;
use azure::client::Client;
use azure::cmd;
use azure_identity::DefaultAzureCredential;

#[tokio::main]
async fn main() -> Result<()> {
    let api_manager = ApiManager::new(PathBuf::from_str("./metadata")?)?;

    let matches = cmd::cmd().get_matches();
    match matches.subcommand() {
        Some(("api", matches)) => {
            let args = if let Some(args) = matches.get_many::<String>("args") {
                args.cloned().collect()
            } else {
                vec![]
            };
            let input = CliInput::new(args)?;
            cmd::cmd_api(&api_manager, &input).get_matches();

            // Invoke the api call
            let ctx = api_manager.build_ctx(&input)?;
            let credential = DefaultAzureCredential::new()?;
            let client = Client::new(
                "https://management.azure.com",
                vec!["https://management.azure.com/.default"],
                credential,
                None,
            )?;
            let res = ctx.execute(&client).await?;
            println!("{res}");
            return Ok(());
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }
}
