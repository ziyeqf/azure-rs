use crate::api::ctx::Ctx;
use crate::arg::CliInput;
use anyhow::Result;
use std::path::PathBuf;

pub mod ctx;
pub mod metadata;

#[derive(Debug, Clone)]
pub struct ApiManager {
    #[allow(dead_code)]
    path: PathBuf,
    rps: Vec<String>,
}

impl ApiManager {
    pub fn build_ctx(&self, cli_input: &CliInput) -> Result<Ctx> {
        let pos_args = cli_input.pos_args();
        pos_args
            .first()
            .ok_or(anyhow::anyhow!("the rp is not specified"))
            .and_then(|rp| {
                let metadata = self.read_metadata(rp)?;
                Ok(Ctx::new(metadata, cli_input.clone()))
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
        pub fn new(_: PathBuf) -> Result<Self> {
            let rps: Vec<String> = Asset::names()
                .map(|name| name.trim_end_matches(".json").to_string())
                .collect();
            Ok(Self {
                path: PathBuf::new(),
                rps,
            })
        }

        pub fn list_rps(&self) -> &Vec<String> {
            &self.rps
        }

        pub fn read_metadata(&self, rp: &str) -> Result<Metadata> {
            let bytes: Vec<u8> = Asset::get(format!("{rp}.json").as_str())
                .map(|d| d.data.to_vec())
                .ok_or(anyhow!("{rp}.json doesn't exist"))?;
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
        pub fn new(path: PathBuf) -> Result<Self> {
            // TODO: Validate the path
            let mut rps = vec![];
            for entry in path
                .read_dir()
                .context(format!("reading dir {}", path.display()))?
            {
                let path = entry?.path();
                if let Some(ext) = path.extension() {
                    if ext == "json" {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            rps.push(stem.to_owned());
                        }
                    }
                }
            }
            Ok(Self { path, rps })
        }

        pub fn list_rps(&self) -> &Vec<String> {
            &self.rps
        }

        pub fn read_metadata(&self, rp: &str) -> Result<Metadata> {
            let bytes =
                read(self.path.join(format!("{rp}.json"))).context(format!("reading {rp}.json"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}
