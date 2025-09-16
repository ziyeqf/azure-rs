use std::{collections::HashMap};

// pub struct CommandParser {
//     // pub metadata: super::metadata::Command,
//     pub inner: clap::Command,
// }

// impl CommandParser {
//     pub fn parse<T, I>(self, args: I) -> clap::ArgMatches
//     where
//         I: IntoIterator<Item = T>,
//         T: Into<std::ffi::OsString> + Clone,
//     {
//         let matches = self.inner.get_matches_from(args);
//         return matches;
//     }
// }

// impl From<super::metadata::Command> for CommandParser {
//     fn from(metadata: super::metadata::Command) -> Self {
//         let mut cmd = clap::Command::new(&metadata.name);
//         for group in &metadata.arg_groups {
//             let name = &group.name;
//             for arg_meta in &group.args {
//                 let mut arg = clap::Arg::new(&arg_meta.var).aliases(&arg_meta.options);
//                 if let Some(help) = &arg_meta.help {
//                     arg = arg.help(&help.short);
//                 }
//                 if let Some(true) = arg_meta.required {
//                     arg = arg.required(true);
//                 }
//                 if let Some(group) = &arg_meta.group {
//                     arg = arg.group(group);
//                 }
//                 cmd = cmd.arg(arg);
//             }
//         }
//         Self { inner: cmd }
//     }
// }

// pub struct CommandArg(HashMap<String, serde_json::Value>);

// impl CommandArg {
//     pub fn new() -> Self {
//         Self(HashMap::new())
//     }
// }

pub struct CommandInvocation {
    pub metadata: super::metadata::Command,
    pub args: clap::ArgMatches,
}

impl CommandInvocation {
    pub async fn invoke(&self, client: &crate::client::Client) -> anyhow::Result<serde_json::Value> {
        if self.metadata.operations.is_empty() {
            anyhow::bail!("No operation found for command {}", self.metadata.name);
        }
        let operation = self.metadata.operations.first().unwrap();
        let operation_ionvocation = OperationInvocation {
            metadata: operation.clone(),
            args: self.args.clone(),
            ctx_args: HashMap::new(),
        };
        operation_ionvocation.invoke(client).await
    }
}

pub struct OperationInvocation {
    pub metadata: super::metadata::Operation,
    pub args: clap::ArgMatches,
    pub ctx_args: HashMap<String, serde_json::Value>,
}

impl OperationInvocation {
    pub async fn invoke(&self, client: &crate::client::Client) -> anyhow::Result<serde_json::Value> {
        if let Some(http) = &self.metadata.http {
            let mut path = http.path.clone();
            for param in &http.request.path.params {
                if let Some(value) = self.args.get_one::<String>(&param.arg) {
                    path = path.replace(&format!("{{{}}}", param.name), value);
                } else if let Some(value) = self.ctx_args.get(&param.arg) {
                    path = path.replace(&format!("{{{}}}", param.name), value.as_str().unwrap());
                } else if let Some(true) = param.required {
                    anyhow::bail!("Missing required path parameter: {}", param.name);
                } else {
                    panic!("Optional path parameter not supported yet!")
                }
            }
            let mut query_pairs = HashMap::new();
            // TODO: handle query parameters
            for param in &http.request.query.consts {
                query_pairs.insert(param.name.clone(), param.default.value.clone());
            }
            let body: Option<bytes::Bytes> = if let Some(body_meta) = &http.request.body {
                if let Some(schema) = &body_meta.json.schema {
                    self.build_value(schema)?.map(|v| bytes::Bytes::from(v.to_string()))
                } else {
                    None
                }
            } else {
                None
            };
            let response = client.run(http.request.method.into(), path.as_str(), &query_pairs["api-version"], body, None).await?;
            for response_meta in &http.responses {
                if let Some(status_codes) = &response_meta.status_code {
                    if status_codes.contains(&(u16::from(response.status_code) as i64)) {
                        // if let Some(json_meta) = response_meta.body.as_ref().map(|body| &body.json) {
                        //     if let Some(schema) = &json_meta.schema {
                        //         let v: serde_json::Value = serde_json::from_slice(&response.body)?;
                        //         println!("{}", serde_json::to_string_pretty(&v)?);
                        //         return Ok(());
                        //     }
                        // }
                        return Ok(serde_json::from_slice(&response.body)?);
                    }
                } else if let Some(true) = response_meta.is_error {
                    anyhow::bail!("Error response: {}", String::from_utf8_lossy(&response.body));
                }
            }
        }
        anyhow::bail!("No HTTP information found for this operation");
    }

    fn build_value(
        &self,
        schema: &super::metadata::Schema,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        match schema.type_.as_str() {
            "object" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.ctx_args.get(arg) {
                        Ok(Some(value.clone()))
                    } else if let Some(value) = self.args.get_one::<String>(arg) {
                        Ok(Some(serde_json::from_str(value)?))
                    } else if let Some(true) = schema.required {
                        anyhow::bail!("Missing required object property: {}", arg);
                    } else {
                        let mut map = serde_json::Map::new();
                        if let Some(props) = &schema.props {
                            for prop in props {
                                if let Some(prop_name) = &prop.name {
                                    let value = self.build_value(prop)?;
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
                    anyhow::bail!("Object schema is not supported without a name");
                }
            }
            s if s.starts_with("array") => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.ctx_args.get(arg) {
                        Ok(Some(value.clone()))
                    } else if let Some(value) = self.args.get_one::<String>(arg) {
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
                    if let Some(value) = self.ctx_args.get(arg) {
                        Ok(Some(value.clone()))
                    } else if let Some(value) = self.args.get_one::<String>(arg) {
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
                    if let Some(value) = self.ctx_args.get(arg) {
                        Ok(Some(value.clone()))
                    } else if let Some(value) = self.args.get_one::<i32>(arg) {
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
                    if let Some(value) = self.ctx_args.get(arg) {
                        Ok(Some(value.clone()))
                    } else if let Some(value) = self.args.get_one::<bool>(arg) {
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
                    if let Some(value) = self.ctx_args.get(arg) {
                        Ok(Some(value.clone()))
                    } else if let Some(value) = self.args.get_one::<String>(arg) {
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
