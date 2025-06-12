use azure::client::Client;
use azure_core::http::Method::Get;
use azure_identity::DefaultAzureCredential;
use clap::Parser;
use std::error::Error;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    id: String,

    #[arg(short, long)]
    api_version: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let credential = DefaultAzureCredential::new()?;
    let client = Client::new("https://management.azure.com", credential, None)?;
    let resp = client.run(Get, &args.id, &args.api_version, None).await?;
    let body = resp.into_body().collect_string().await?;
    print!("{}", body);
    Ok(())
}
