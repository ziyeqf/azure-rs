mod metadata;
use bytes::Bytes;

use crate::arg::{Arg, CliInput};
use crate::client::Client;
use anyhow::{anyhow, bail, Context, Result};
use azure_core::http::Method;
use metadata::{CommandHelper, CommandOrCommandGroup, Http, Metadata};
use serde_json::{Map, Value};
use std::fs::read;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Api {
    cli_input: CliInput,
    c_or_cg: CommandOrCommandGroup,
}

impl Api {
    pub fn new(metadata: Metadata, cli_input: CliInput) -> Result<Self> {
        let c_or_cg = metadata.resolve_command_or_command_group(&cli_input)?;
        Ok(Self { cli_input, c_or_cg })
    }

    pub fn help(&self) -> String {
        match &self.c_or_cg {
            CommandOrCommandGroup::Command(c) => c.help(),
            CommandOrCommandGroup::CommandGroup(cg) => cg.help(),
        }
    }

    pub async fn execute(&self, client: &Client) -> Result<String> {
        match &self.c_or_cg {
            CommandOrCommandGroup::Command(c) => {
                if c.operations.len() != 1 {
                    // TODO: support more than one operations
                    bail!("only support 1 operation now");
                }
                let op = &c.operations[0];
                if let Some(http) = &op.http {
                    let api_path = self.build_api_path(http)?;
                    let api_method = self.build_api_method(http)?;
                    let api_version = self.build_api_version(http)?;
                    let api_body = self.build_api_body(http)?;
                    let resp = client
                        .run(api_method, &api_path, &api_version, Some(api_body), None)
                        .await?;
                    Ok(String::from_utf8(resp.body.to_vec())?)
                } else {
                    // TODO: support non http operation
                    bail!("no http operation found for {}", c.name);
                }
            }
            CommandOrCommandGroup::CommandGroup(cg) => {
                bail!("command group \"{}\" is not executable", cg.name);
            }
        }
    }

    fn build_api_path(&self, http_desc: &Http) -> Result<String> {
        let (path, req_path) = (&http_desc.path, &http_desc.request.path);
        // Interpolate the path params into the API path.
        let c = self.c_or_cg.as_command();
        let default_arg_group = c
            .arg_groups
            .iter()
            .find(|ag| ag.name.as_str() == "")
            .ok_or(anyhow!("no arg group with name \"\""))?;
        let mut path = path.clone();
        for param in &req_path.params {
            let arg = default_arg_group
                .args
                .iter()
                .find(|a| a.var == param.arg)
                .ok_or(anyhow!(
                    "can't find argument with var \"{}\" from the default argument group",
                    param.arg
                ))?;
            let arg_value = self
                .cli_input
                .args
                .iter()
                .find_map(|ia| {
                    if let Arg::Optional(k, Some(v)) = ia {
                        if arg.options.contains(k) {
                            Some(v)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .ok_or(anyhow!("option \"{:#?}\" not specified", arg.options))?;
            path = path.replace(format!("{{{}}}", param.name).as_str(), arg_value);
        }
        Ok(path)
    }

    fn build_api_version(&self, http_desc: &Http) -> Result<String> {
        let api_version_q = http_desc
            .request
            .query
            .consts
            .iter()
            .find(|q| q.name == "api-version")
            .ok_or(anyhow!("no \"api-version\" found in request query"))?;
        Ok(api_version_q.default.value.clone())
    }

    fn build_api_method(&self, http_desc: &Http) -> Result<Method> {
        Ok(match http_desc.request.method {
            metadata::Method::Head => Method::Head,
            metadata::Method::Get => Method::Get,
            metadata::Method::Put => Method::Put,
            metadata::Method::Patch => Method::Patch,
            metadata::Method::Post => Method::Post,
            metadata::Method::Delete => Method::Delete,
        })
    }

    fn build_api_body(&self, http_desc: &Http) -> Result<Bytes> {
        let c = self.c_or_cg.as_command();
        let mut result = Value::Null;
        if let Some(body) = &http_desc.request.body {
            if let Some(schema) = &body.json.schema {
                if let Some(props) = &schema.props {
                    let mut obj = Map::new();

                    const ARG_GROUP_NAME_PARAM: &str = "Parameters";
                    let params_arg_group = c
                        .arg_groups
                        .iter()
                        .find(|ag| ag.name.as_str() == ARG_GROUP_NAME_PARAM)
                        .ok_or(anyhow!(
                            "no arg group with name \"{}\"",
                            ARG_GROUP_NAME_PARAM
                        ))?;

                    for prop in props {
                        if prop.arg.is_none() {
                            continue;
                        }
                        let prop_arg = prop.arg.as_ref().unwrap();
                        let arg = params_arg_group
                            .args
                            .iter()
                            .find(|a| a.var == *prop_arg)
                            .ok_or(anyhow!(
                                "can't find argument with var \"{}\" from the {} argument group",
                                prop_arg,
                                ARG_GROUP_NAME_PARAM,
                            ))?;
                        let arg_value = self.cli_input.args.iter().find_map(|ia| {
                            if let Arg::Optional(k, Some(v)) = ia {
                                if arg.options.contains(k) {
                                    Some(v)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        });
                        if arg_value.is_none() {
                            if !arg.required.unwrap_or(false) {
                                continue;
                            } else {
                                return Err(anyhow!("option \"{:#?}\" not specified", arg.options));
                            }
                        }
                        let arg_value = arg_value.unwrap();

                        if let Some(name) = &prop.name {
                            let v = match prop.type_.as_str() {
                                "ResourceLocation" | "string" => Value::String(arg_value.clone()),
                                _ => serde_json::from_str(arg_value)
                                    .context(format!("parsing json value {:#?}", arg_value))?,
                            };
                            obj.insert(name.clone(), v);
                        }
                    }
                    result = Value::Object(obj);
                }
            }
        }
        Ok(Bytes::from(serde_json::to_vec(&result)?))
    }
}

#[derive(Debug, Clone)]
pub struct ApiManager {
    path: PathBuf,
}

impl ApiManager {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

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

    fn read_group_metadata(&self, group: &str) -> Result<Metadata> {
        let bytes = read(self.path.join(format!("{}.json", group)))
            .context(format!("reading {}.json", group))?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}
