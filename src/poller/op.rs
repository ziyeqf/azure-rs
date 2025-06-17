use std::fmt::write;

use crate::poller::utils::{self, get_resource_location, result_helper};

use super::utils::{get_lro_status, get_provisioning_state};
use super::PollingHandler;
use super::{final_state::FinalStateVia, utils::LROStatus, Response};
use azure_core::error::ErrorKind;
use azure_core::http::headers::{HeaderName, AZURE_ASYNCOPERATION, LOCATION};
use azure_core::http::{Method, Pipeline, Request, Url};
use azure_core::{Error, Result};

// This is not defined in azure_core/src/http/headers.rs
pub const OPERATION_LOCATION: HeaderName = HeaderName::from_static("operation-location");

pub struct Poller {
    pl: Pipeline,

    // The response of the last call (either sync or poll call).
    resp: Response,

    // The URL from Operation-Location header.
    op_loc_url: Url,

    // The URL from Location header.
    loc_url: Option<Url>,

    // The URL from the initial LRO request.
    origin_url: Url,

    // The HTTP method from the initial LRO request.
    method: Method,

    // The value of final-state-via from swagger.
    final_state: Option<FinalStateVia>,

    // The JSON path to the result's payload when it's included with the terminal success response.
    result_path: Option<String>,

    // The LRO's current state.
    cur_state: LROStatus,
}

impl Poller {
    pub fn new(
        pl: Pipeline,
        req: &Request,
        resp: Response,
        final_state: Option<FinalStateVia>,
        result_path: Option<String>,
    ) -> Result<Self> {
        let op_loc_url = resp.headers.get_as(&OPERATION_LOCATION).map_err(|err| {
            err.context(format!(
                "parsing header `{}` as a URL",
                OPERATION_LOCATION.as_str()
            ))
        })?;
        let loc_url = resp.headers.get_optional_as(&LOCATION)?;
        let cur_state = get_provisioning_state(&resp)?.unwrap_or(LROStatus::InProgress);

        Ok(Self {
            pl,
            resp,
            op_loc_url,
            loc_url,
            origin_url: req.url().clone(),
            method: *req.method(),
            final_state,
            result_path,
            cur_state,
        })
    }
}

impl PollingHandler for Poller {
    fn applicable(resp: &Response) -> bool {
        resp.headers.get_optional_str(&OPERATION_LOCATION).is_some()
    }

    async fn poll(&mut self, ctx: &azure_core::http::Context<'_>) -> Result<Response> {
        if self.done() {
            return Ok(self.resp.clone());
        }
        let mut req = Request::new(self.op_loc_url.clone(), Method::Get);
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

    async fn result(&self, ctx: &azure_core::http::Context<'_>) -> Result<Response> {
        assert!(self.cur_state.is_terminal());

        if self.cur_state.is_failed() {
            return Err(self.resp.clone().into());
        }

        let mut req: Option<Request> = None;
        if let Some(FinalStateVia::Location) = self.final_state {
            if let Some(loc_url) = &self.loc_url {
                req = Some(Request::new(loc_url.clone(), Method::Get));
            }
        } else if let Some(rl) = get_resource_location(&self.resp)? {
            req = Some(Request::new(rl, Method::Get));
        } else {
            match self.method {
                Method::Patch | Method::Put => {
                    req = Some(Request::new(self.origin_url.clone(), Method::Get));
                }
                Method::Post => {
                    if let Some(loc_url) = &self.loc_url {
                        req = Some(Request::new(loc_url.clone(), Method::Get));
                    }
                }
                _ => {}
            }
        }

        if req.is_none() {
            return Ok(self.resp.clone());
        }
        let mut req = req.unwrap();

        let raw_resp = self.pl.send(ctx, &mut req).await?;
        let resp = Response::from_raw_response(raw_resp).await?;

        result_helper(
            &resp,
            self.cur_state.is_failed(),
            self.result_path.as_deref(),
        )
    }
}
