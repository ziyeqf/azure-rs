use azure_core::credentials::Secret;
use azure_identity::ClientSecretCredential;
use std::{path::PathBuf, result::Result};
use wasm_bindgen::prelude::*;

use crate::{api_mgr::ApiManager, arg::CliInput, client::Client};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub async fn run_cli(
    args: Vec<String>,
    tenant_id: &str,
    client_id: &str,
    secret: &str,
) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let api_manager = ApiManager::new(PathBuf::new());
    let args: Vec<_> = args.iter().skip(1).collect();
    let input = CliInput::new(args);
    //println!("{:#?}", input);
    let api = api_manager.build_api(&input).map_err(jsfy)?;
    if input.is_help() {
        let res = api.help();
        Ok(res)
    } else {
        let credential = ClientSecretCredential::new(
            tenant_id,
            client_id.to_string(),
            Secret::new(secret.to_string()),
            None,
        )
        .map_err(jsfy)?;

        let client = Client::new(
            "https://management.azure.com",
            vec!["https://management.azure.com/.default"],
            credential,
            None,
        )
        .map_err(jsfy)?;

        let res = api.execute(&client).await.map_err(jsfy)?;
        Ok(res)
    }
}

fn jsfy<E>(e: E) -> JsValue
where
    E: ToString,
{
    JsValue::from_str(e.to_string().as_str())
}
