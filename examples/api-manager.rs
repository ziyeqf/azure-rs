use std::env;
use std::{path::PathBuf, str::FromStr};

use anyhow::Result;
use azure::api_mgr::ApiManager;
use azure::arg::CliInput;
use azure::client::Client;
use azure_identity::DefaultAzureCredential;

#[tokio::main]
async fn main() -> Result<()> {
    let api_manager = ApiManager::new(PathBuf::from_str("./metadata")?);
    let args: Vec<_> = env::args().skip(1).collect();
    let input = CliInput::new(args);
    //println!("{:#?}", input);
    let api = api_manager.build_api(&input)?;
    if input.is_help() {
        let res = api.help();
        println!("{res}");
    } else {
        let credential = DefaultAzureCredential::new()?;
        let client = Client::new(
            "https://management.azure.com",
            vec!["https://management.azure.com/.default"],
            credential,
            None,
        )?;
        let res = api.execute(&client).await?;
        println!("{res}");
    }
    Ok(())
}
