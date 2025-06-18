use super::{PollingHandler, Response};
use azure_core::{
    http::{Context, RawResponse, Request},
    Result,
};

pub struct Poller {}

impl Poller {
    pub fn new() -> Self {
        Poller {}
    }
}

impl PollingHandler for Poller {
    fn applicable(req: &Request, resp: &Response) -> bool {
        todo!()
    }

    async fn result(&self, ctx: &Context<'_>) -> Result<Response> {
        todo!()
    }

    async fn poll(&mut self, ctx: &Context<'_>) -> Result<Response> {
        todo!()
    }

    fn done(&self) -> bool {
        todo!()
    }
}
