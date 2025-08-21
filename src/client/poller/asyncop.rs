use azure_core::error::ErrorKind;
use azure_core::http::headers::LOCATION;
use azure_core::http::{headers::AZURE_ASYNCOPERATION, Method, Pipeline};
use azure_core::http::{Context, Request, Url};
use azure_core::{Error, Result};

use crate::client::poller::utils::{self, get_lro_status, result_helper};

use super::utils::{get_provisioning_state, FinalStateVia, LROStatus};
use super::{PollingHandler, Response};

pub struct Poller {
    pl: Pipeline,

    // The response of the last call (either sync or poll call).
    resp: Response,

    // The URL from Azure-AsyncOperation header.
    async_url: Url,

    // The URL from Location header.
    loc_url: Option<Url>,

    // The URL from the initial LRO request.
    origin_url: Url,

    // The HTTP method from the initial LRO request.
    method: Method,

    // The value of final-state-via from swagger.
    final_state: Option<FinalStateVia>,

    // The LRO's current state.
    cur_state: LROStatus,
}

impl Poller {
    pub fn new(
        pl: Pipeline,
        req: &Request,
        resp: Response,
        final_state: Option<FinalStateVia>,
    ) -> Result<Self> {
        let async_url = resp.headers.get_as(&AZURE_ASYNCOPERATION).map_err(|err| {
            err.context(format!(
                "parsing header `{}` as a URL",
                AZURE_ASYNCOPERATION.as_str()
            ))
        })?;
        let loc_url = resp.headers.get_optional_as(&LOCATION)?;
        let cur_state = get_provisioning_state(&resp)?.unwrap_or(LROStatus::InProgress);
        Ok(Self {
            pl,
            resp,
            async_url,
            loc_url,
            origin_url: req.url().clone(),
            method: req.method(),
            final_state,
            cur_state,
        })
    }
}

impl PollingHandler for Poller {
    fn applicable(_: &Request, resp: &Response) -> bool {
        resp.headers
            .get_optional_str(&AZURE_ASYNCOPERATION)
            .is_some()
    }

    async fn poll(&mut self, ctx: &Context<'_>) -> Result<Response> {
        if self.done() {
            return Ok(self.resp.clone());
        }
        let mut req = Request::new(self.async_url.clone(), Method::Get);
        let resp = self.pl.send(ctx, &mut req).await?;
        let resp = Response::from_raw_response(resp).await?;
        if !utils::is_valid_status_code(resp.status_code) {
            self.resp = resp.clone();
            return Err(Error::message(resp.into(), "invalid response status code"));
        }
        let status = get_lro_status(&resp)?;
        if let Some(status) = status {
            self.cur_state = status;
            self.resp = resp.clone();
            Ok(resp)
        } else {
            Err(Error::message(
                ErrorKind::Other,
                "the response did not contain a status",
            ))
        }
    }

    fn done(&self) -> bool {
        self.cur_state.is_terminal()
    }

    async fn result(&self, ctx: &Context<'_>) -> Result<Response> {
        assert!(self.cur_state.is_terminal());

        if self.cur_state.is_failed() {
            return Err(self.resp.clone().into());
        }

        // if self.resp.status_code == StatusCode::NoContent {
        //     return Ok(None);
        // }

        let mut req: Option<Request> = None;
        match self.method {
            Method::Put | Method::Patch => {
                // for PATCH and PUT, the final GET is on the original resource URL
                req = Some(Request::new(self.origin_url.clone(), Method::Get));
            }
            Method::Post => {
                if let Some(final_state) = self.final_state {
                    match final_state {
                        FinalStateVia::AzureAsyncOp => { /* no final GET required */ }
                        FinalStateVia::Location => unreachable!("final-state-via location is not supposed to be handled by the async op poller"),
                        FinalStateVia::OriginalUri => {
                            req = Some(Request::new(self.origin_url.clone(), Method::Get));
                        }
                        FinalStateVia::OperationLocation => unreachable!("final-state-via operation-location is not supposed to be handled by the async op poller"),
                    }
                } else if let Some(ref loc_url) = self.loc_url {
                    req = Some(Request::new(loc_url.clone(), Method::Get));
                }
            }
            _ => {}
        }
        if req.is_none() {
            return Ok(self.resp.clone());
        }
        let mut req = req.unwrap();

        let raw_resp = self.pl.send(ctx, &mut req).await?;
        let resp = Response::from_raw_response(raw_resp).await?;

        result_helper(&resp, self.cur_state.is_failed(), None)
    }
}
