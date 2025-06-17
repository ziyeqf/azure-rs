mod asyncop;
mod final_state;
mod loc;
mod noop;
mod utils;

use azure_core::error::{http_response_from_body, ErrorKind};
use azure_core::http::Request;
use std::time::Duration;
use tokio::time::sleep;

use azure_core::Error;
use azure_core::{
    http::{headers::Headers, Context, Pipeline, RawResponse, StatusCode},
    Result,
};
use bytes::Bytes;
use final_state::FinalStateVia;

#[derive(Debug, Clone)]
pub struct Response {
    pub status_code: StatusCode,
    pub headers: Headers,
    pub body: Bytes,
}

impl Response {
    pub async fn from_raw_response(resp: RawResponse) -> Result<Self> {
        let (status_code, headers, body) = resp.deconstruct();
        let body = body.collect().await?;
        Ok(Self {
            status_code,
            headers,
            body,
        })
    }
}

impl From<Response> for ErrorKind {
    fn from(val: Response) -> Self {
        http_response_from_body(val.status_code, &val.body)
    }
}

impl From<Response> for Error {
    fn from(val: Response) -> Self {
        let error_kind: ErrorKind = val.into();
        error_kind.into_error()
    }
}

trait PollingHandler {
    fn applicable(resp: &Response) -> bool;

    // poll fetches the latest state of the LRO.
    async fn poll(&mut self, ctx: &Context<'_>) -> Result<Response>;

    // done returns true if the LRO has reached a terminal state.
    fn done(&self) -> bool;

    // result must be called once the LRO has reached a terminal state. It returns result of the operation.
    async fn result(&self, ctx: &Context<'_>) -> Result<Response>;
}

enum Handler {
    AsyncOp(asyncop::Poller),
    Loc(loc::Poller),
    Noop(noop::Poller),
}

#[derive(Debug, Default, Clone)]
pub struct NewPollerOptions {
    final_state: Option<FinalStateVia>,
}

#[derive(Debug, Clone, Default)]
pub struct PollUntilDoneOptions {
    // frequency is the time to wait between polling intervals in absence of a Retry-After header. Allowed minimum is one second.
    // Pass zero to accept the default value (30s).
    frequency: Option<Duration>,
}

pub struct Poller {
    handler: Handler,
    resp: Response,
}

impl Poller {
    pub async fn new(
        pl: Pipeline,
        req: &Request,
        resp: &Response,
        opts: Option<NewPollerOptions>,
    ) -> Result<Option<Self>> {
        let opts = opts.unwrap_or_default();

        // This is a back-stop in case the swagger is incorrect (i.e. missing one or more status codes for success).
        // ideally the codegen should return an error if the initial response failed and not even create a poller.
        if !utils::is_valid_status_code(resp.status_code) {
            return Err(Error::message(
                ErrorKind::Other,
                "the operation failed or was cancelled",
            ));
        }

        // Determine the polling method
        let handler = if asyncop::Poller::applicable(resp) {
            Handler::AsyncOp(asyncop::Poller::new(
                pl,
                req,
                resp.clone(),
                opts.final_state,
            )?)
        } else if loc::Poller::applicable(resp) {
            Handler::Loc(loc::Poller::new(pl, resp.clone())?)
        } else {
            return Ok(None);
        };

        Ok(Some(Self {
            handler,
            resp: resp.clone(),
        }))
    }

    pub async fn poll(&mut self, ctx: &Context<'_>) -> Result<Response> {
        if self.done() {
            return Ok(self.resp.clone());
        }

        let resp = match &mut self.handler {
            Handler::AsyncOp(poller) => poller.poll(ctx).await?,
            Handler::Loc(poller) => poller.poll(ctx).await?,
            Handler::Noop(poller) => poller.poll(ctx).await?,
        };

        self.resp = resp;
        Ok(self.resp.clone())
    }

    pub async fn poll_until_done(
        &mut self,
        ctx: &Context<'_>,
        opts: Option<PollUntilDoneOptions>,
    ) -> Result<Response> {
        let opts = opts.unwrap_or_default();

        loop {
            let resp = self.poll(ctx).await?;
            if self.done() {
                return self.result(ctx).await;
            }

            let duration = utils::retry_after(&resp)
                .unwrap_or(opts.frequency.unwrap_or(Duration::from_secs(30)));
            sleep(duration).await;
        }
    }

    pub fn done(&self) -> bool {
        match &self.handler {
            Handler::AsyncOp(poller) => poller.done(),
            Handler::Loc(poller) => poller.done(),
            Handler::Noop(poller) => poller.done(),
        }
    }

    // result returns the final response of the LRO operation when it reaches a terminal state.
    // If the LRO completed successfully, the Response is returned (which can be None).
    // If the LRO failed or was canceled, an Error of ErrorKind::HttpResponse is returned.
    async fn result(&self, ctx: &Context<'_>) -> Result<Response> {
        assert!(self.done());
        match &self.handler {
            Handler::AsyncOp(poller) => poller.result(ctx).await,
            Handler::Loc(poller) => poller.result(ctx).await,
            Handler::Noop(poller) => poller.result(ctx).await,
        }
    }
}
