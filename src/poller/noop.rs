use super::{PollingHandler, Response};
use azure_core::{
    http::{headers::Headers, Context, Request, StatusCode},
    Result,
};

pub struct Poller {
    resp: Response,
}

impl Poller {
    pub fn new(resp: &Response) -> Self {
        Poller { resp: resp.clone() }
    }
}

impl PollingHandler for Poller {
    fn applicable(_: &Request, _: &Response) -> bool {
        true
    }

    async fn result(&self, _: &Context<'_>) -> Result<Response> {
        Ok(self.resp.clone())
    }

    async fn poll(&mut self, _: &Context<'_>) -> Result<Response> {
        Ok(self.resp.clone())
    }

    fn done(&self) -> bool {
        true
    }
}
