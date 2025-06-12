use std::sync::Arc;

use azure_core::{
    credentials::TokenCredential,
    http::{
        policies::{BearerTokenCredentialPolicy, Policy},
        ClientMethodOptions, ClientOptions, Context, Method, Pipeline, RawResponse, Request, Url,
    },
    Result,
};

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
        options: Option<ClientMethodOptions<'_>>,
    ) -> Result<RawResponse> {
        let options = options.unwrap_or_default();
        let mut url = self.endpoint.clone();
        url = url.join(api_path)?;
        url.query_pairs_mut()
            .append_pair("api-version", api_version);
        let mut request = Request::new(url, method);
        request.insert_header("accept", "application/json");

        let ctx = Context::with_context(&options.context);
        self.pipeline.send(&ctx, &mut request).await
    }
}
