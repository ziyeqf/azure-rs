use core::unreachable;
use std::collections::HashMap;

use crate::client::Client;

use super::metadata::{Command, Operation, Schema};
use anyhow::{bail, Result};
use clap::ArgMatches;

pub struct CommandInvocation {
    command: Command,
    matches: ArgMatches,
}

impl CommandInvocation {
    pub fn new(command: &Command, matches: &ArgMatches) -> Self {
        Self {
            command: command.clone(),
            matches: matches.clone(),
        }
    }

    pub async fn invoke(&self, client: &Client) -> Result<serde_json::Value> {
        if self.command.operations.is_empty() {
            bail!("No operation found for command {}", self.command.name);
        }
        let operation = self.command.operations.first().unwrap();
        let operation_ionvocation = OperationInvocation::new(operation, &self.matches);
        operation_ionvocation.invoke(client).await
    }
}

struct OperationInvocation {
    operation: Operation,
    matches: ArgMatches,
}

impl OperationInvocation {
    pub fn new(operation: &Operation, matches: &ArgMatches) -> Self {
        Self {
            operation: operation.clone(),
            matches: matches.clone(),
        }
    }

    pub async fn invoke(
        &self,
        client: &crate::client::Client,
    ) -> anyhow::Result<serde_json::Value> {
        if self.operation.http.is_none() {
            anyhow::bail!(
                r#"HTTP information not found for operation "{}""#,
                self.operation
                    .operation_id
                    .clone()
                    .unwrap_or("".to_string()),
            );
        }

        let http = self.operation.http.as_ref().unwrap();
        let mut path = http.path.clone();
        for param in &http.request.path.params {
            if let Some(value) = self.matches.get_one::<String>(&param.arg) {
                path = path.replace(&format!("{{{}}}", param.name), value);
            } else if let Some(true) = param.required {
                anyhow::bail!("missing required path parameter: {}", param.name);
            } else {
                unreachable!(
                    r#"optional path parameter "{}" not supported yet!"#,
                    param.name
                )
            }
        }
        let mut query_pairs = HashMap::new();
        // TODO: handle query parameters (query.params)
        for param in &http.request.query.consts {
            query_pairs.insert(param.name.clone(), param.default.value.clone());
        }
        let body: Option<bytes::Bytes> = if let Some(body_meta) = &http.request.body {
            if let Some(schema) = &body_meta.json.schema {
                self.build_value(schema.clone())?
                    .map(|v| bytes::Bytes::from(v.to_string()))
            } else {
                None
            }
        } else {
            None
        };
        let response = client
            .run(
                http.request.method.into(),
                path.as_str(),
                &query_pairs["api-version"],
                body,
                None,
            )
            .await?;
        for response_meta in &http.responses {
            if let Some(status_codes) = &response_meta.status_code {
                if status_codes.contains(&(u16::from(response.status_code) as i64)) {
                    return Ok(serde_json::from_slice(&response.body)?);
                }
            }
        }
        anyhow::bail!(
            "error response: {}\n\n{}",
            response.status_code,
            String::from_utf8_lossy(&response.body)
        );
    }

    fn build_value(&self, schema: Schema) -> Result<Option<serde_json::Value>> {
        match schema.type_.as_str() {
            "object" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<String>(arg) {
                        Ok(Some(serde_json::from_str(value)?))
                    } else if let Some(true) = schema.required {
                        anyhow::bail!("missing required object property: {}", arg);
                    } else {
                        let mut map = serde_json::Map::new();
                        if let Some(props) = &schema.props {
                            for prop in props {
                                if let Some(prop_name) = &prop.name {
                                    let value = self.build_value(prop.clone())?;
                                    if let Some(value) = value {
                                        map.insert(prop_name.clone(), value);
                                    }
                                } else {
                                    anyhow::bail!("Property without a name in object schema");
                                }
                            }
                        }
                        Ok(Some(serde_json::Value::Object(map)))
                    }
                } else {
                    let mut map = serde_json::Map::new();
                    if let Some(props) = &schema.props {
                        for prop in props {
                            if let Some(prop_name) = &prop.name {
                                let value = self.build_value(prop.clone())?;
                                if let Some(value) = value {
                                    map.insert(prop_name.clone(), value);
                                }
                            } else {
                                anyhow::bail!("Property without a name in object schema");
                            }
                        }
                    }
                    Ok(Some(serde_json::Value::Object(map)))
                }
            }
            s if s.starts_with("array") => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<String>(arg) {
                        Ok(Some(serde_json::from_str(value)?))
                    } else if let Some(true) = schema.required {
                        anyhow::bail!("Missing required array property: {}", arg);
                    } else {
                        Ok(None)
                    }
                } else {
                    anyhow::bail!("Array schema is not supported without a name");
                }
            }
            "string" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<String>(arg) {
                        Ok(Some(serde_json::Value::String(value.clone())))
                    } else if let Some(true) = schema.required {
                        anyhow::bail!("Missing required string property: {}", arg);
                    } else {
                        Ok(None)
                    }
                } else {
                    anyhow::bail!("Array schema is not supported without a name");
                }
            }
            s if s.starts_with("integer") => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<i32>(arg) {
                        Ok(Some((*value).into()))
                    } else if let Some(true) = schema.required {
                        anyhow::bail!("Missing required integer property: {}", arg);
                    } else {
                        Ok(None)
                    }
                } else {
                    anyhow::bail!("Array schema is not supported without a name");
                }
            }
            "boolean" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<bool>(arg) {
                        Ok(Some((*value).into()))
                    } else if let Some(true) = schema.required {
                        anyhow::bail!("Missing required boolean property: {}", arg);
                    } else {
                        Ok(None)
                    }
                } else {
                    anyhow::bail!("Array schema is not supported without a name");
                }
            }
            _ => {
                // We suppose any other type as a json value first, if failed, try to parse it as a string
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<String>(arg) {
                        match serde_json::from_str(value) {
                            Ok(v) => Ok(Some(v)),
                            Err(_) => Ok(Some(serde_json::Value::String(value.clone()))),
                        }
                    } else if let Some(true) = schema.required {
                        anyhow::bail!("Missing required property: {}", arg);
                    } else {
                        Ok(None)
                    }
                } else {
                    anyhow::bail!("Schema is not supported without a name");
                }
            }
        }
    }
}
