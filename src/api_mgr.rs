use crate::api::Api;
use crate::arg::CliInput;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ApiManager {
    // This is not needed when embed-api or for wasm32.
    #[allow(dead_code)]
    path: PathBuf,
}

impl ApiManager {
    pub fn build_api(&self, cli_input: &CliInput) -> Result<Api> {
        let pos_args = cli_input.pos_args();
        pos_args
            .first()
            .ok_or(anyhow::anyhow!(
                "no positional argument specified from the CLI input"
            ))
            .and_then(|group| {
                let metadata = self.read_group_metadata(group)?;
                Ok(Api::new(metadata, cli_input.clone()))
            })?
    }
}

#[cfg(any(feature = "embed-api", target_arch = "wasm32"))]
mod embedded {
    use crate::api::metadata::Metadata;
    use anyhow::{anyhow, Result};
    use std::path::PathBuf;

    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "metadata/"]
    struct Asset;

    impl super::ApiManager {
        pub fn new(_: PathBuf) -> Self {
            Self {
                path: PathBuf::new(),
            }
        }

        pub fn read_group_metadata(&self, group: &str) -> Result<Metadata> {
            let bytes: Vec<u8> = Asset::get(format!("{group}.json").as_str())
                .map(|d| d.data.to_vec())
                .ok_or(anyhow!("{group}.json doesn't exist"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}

#[cfg(not(any(feature = "embed-api", target_arch = "wasm32")))]
mod fs {
    use crate::api::metadata::Metadata;
    use anyhow::{Context, Result};
    use std::path::PathBuf;

    use std::fs::read;

    impl super::ApiManager {
        pub fn new(path: PathBuf) -> Self {
            Self { path }
        }
        pub fn read_group_metadata(&self, group: &str) -> Result<Metadata> {
            let bytes = read(self.path.join(format!("{group}.json")))
                .context(format!("reading {group}.json"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}
