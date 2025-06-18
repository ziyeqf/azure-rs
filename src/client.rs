use std::sync::Arc;

use azure_core::{
    credentials::TokenCredential,
    http::{
        policies::{BearerTokenCredentialPolicy, Policy},
        ClientMethodOptions, ClientOptions, Context, Method, Pipeline, Request, Url,
    },
    Result,
};
use bytes::Bytes;

use crate::poller::{Poller, Response};

#[derive(Debug)]
pub struct Client {
    endpoint: Url,
    pipeline: Pipeline,
}

impl Client {
    pub fn new(
        endpoint: &str,
        credential: Arc<dyn TokenCredential>,
        options: Option<ClientOptions>,
    ) -> Result<Self> {
        let options = options.unwrap_or_default();
        let endpoint = Url::parse(endpoint)?;
        let auth_policy: Arc<dyn Policy> = Arc::new(BearerTokenCredentialPolicy::new(
            credential,
            vec!["https://management.azure.com/.default"],
        ));
        let pipeline = Pipeline::new(
            option_env!("CARGO_PKG_NAME"),
            option_env!("CARGO_PKG_VERSION"),
            options,
            vec![auth_policy],
            vec![],
        );
        Ok(Self { endpoint, pipeline })
    }

    pub async fn run(
        &self,
        method: Method,
        api_path: &str,
        api_version: &str,
        body: Option<Bytes>,
        options: Option<ClientMethodOptions<'_>>,
    ) -> Result<Response> {
        let options = options.unwrap_or_default();
        let mut url = self.endpoint.clone();
        url = url.join(api_path)?;
        url.query_pairs_mut()
            .append_pair("api-version", api_version);
        let mut request = Request::new(url, method);
        request.insert_header("accept", "application/json");
        if let Some(body) = body {
            request.insert_header("content-type", "application/json");
            request.set_body(body);
        }

        let ctx = Context::with_context(&options.context);
        let raw_resp = self.pipeline.send(&ctx, &mut request).await?;
        let resp = Response::from_raw_response(raw_resp).await?;

        // For PUT, POST, PATCH, DELETE operations that can be a LRO, try to
        if [Method::Put, Method::Post, Method::Delete, Method::Patch]
            .iter()
            .any(|m| *m == method)
        {
            if let Ok(mut poller) = Poller::new(self.pipeline.clone(), &request, &resp, None).await
            {
                return poller.poll_until_done(&ctx, None).await;
            }
        }

        Ok(resp)
    }
}
