mod asyncop;
mod body;
mod loc;
mod noop;
mod op;
mod utils;

use azure_core::error::ErrorKind;
use azure_core::http::{Method, Request};
use std::time::Duration;
use typespec_client_core::sleep::sleep;
use typespec_client_core::time;
use utils::FinalStateVia;

use azure_core::Error;
use azure_core::{
    http::{Context, Pipeline, StatusCode},
    Result,
};

use super::response::Response;

trait PollingHandler {
    fn applicable(req: &Request, resp: &Response) -> bool;

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
    Op(op::Poller),
    Body(body::Poller),
    Noop(noop::Poller),
}

#[derive(Debug, Default, Clone)]
pub struct NewPollerOptions {
    // final_state contains the final-state-via value for the LRO.
    // NOTE: used only for Azure-AsyncOperation and Operation-Location LROs.
    final_state: Option<FinalStateVia>,

    // operation_location_result_path contains the JSON path to the result's
    // payload when it's included with the terminal success response.
    // NOTE: only used for Operation-Location LROs.
    operation_location_result_path: Option<String>,
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
    ) -> Result<Self> {
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
        let handler = if asyncop::Poller::applicable(req, resp) {
            // async poller must be checked first as it can also have a location header
            Handler::AsyncOp(asyncop::Poller::new(
                pl,
                req,
                resp.clone(),
                opts.final_state,
            )?)
        } else if op::Poller::applicable(req, resp) {
            // op poller must be checked before loc as it can also have a location header
            Handler::Op(op::Poller::new(
                pl,
                req,
                resp.clone(),
                opts.final_state,
                opts.operation_location_result_path,
            )?)
        } else if loc::Poller::applicable(req, resp) {
            Handler::Loc(loc::Poller::new(pl, resp.clone())?)
        } else if body::Poller::applicable(req, resp) {
            // must test body poller last as it's a subset of the other pollers.
            // TODO: this is ambiguous for PATCH/PUT if it returns a 200 with no polling headers (sync completion)
            Handler::Body(body::Poller::new(pl, req, resp.clone())?)
        } else if resp.status_code == StatusCode::Accepted
            && [Method::Delete, Method::Post]
                .iter()
                .any(|v| v == req.method())
        {
            // if we get here it means we have a 202 with no polling headers.
            // for DELETE and POST this is a hard error per ARM RPC spec.
            return Err(Error::message(
                resp.clone().into(),
                "response is missing polling URL",
            ));
        } else {
            Handler::Noop(noop::Poller::new(resp))
        };

        Ok(Self {
            handler,
            resp: resp.clone(),
        })
    }

    pub async fn poll(&mut self, ctx: &Context<'_>) -> Result<Response> {
        if self.done() {
            return Ok(self.resp.clone());
        }

        let resp = match &mut self.handler {
            Handler::AsyncOp(poller) => poller.poll(ctx).await?,
            Handler::Loc(poller) => poller.poll(ctx).await?,
            Handler::Op(poller) => poller.poll(ctx).await?,
            Handler::Body(poller) => poller.poll(ctx).await?,
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

            let duration = time::Duration::try_from(
                utils::retry_after(&resp)
                    .unwrap_or(opts.frequency.unwrap_or(Duration::from_secs(30))),
            )
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
            sleep(duration).await;
        }
    }

    pub fn done(&self) -> bool {
        match &self.handler {
            Handler::AsyncOp(poller) => poller.done(),
            Handler::Loc(poller) => poller.done(),
            Handler::Op(poller) => poller.done(),
            Handler::Body(poller) => poller.done(),
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
            Handler::Op(poller) => poller.result(ctx).await,
            Handler::Body(poller) => poller.result(ctx).await,
            Handler::Noop(poller) => poller.result(ctx).await,
        }
    }
}
