use azure_core::{Error, Result};

use azure_core::http::{Method, Pipeline, Request, StatusCode, Url};

use crate::client::poller::utils::{self, get_provisioning_state};

use super::utils::result_helper;
use super::PollingHandler;
use super::{utils::LROStatus, Response};

pub struct Poller {
    pl: Pipeline,

    // The response of the last call (either sync or poll call).
    resp: Response,

    // The URL for polling.
    poll_url: Url,

    // The LRO's current state.
    cur_state: LROStatus,
}

impl Poller {
    pub fn new(pl: Pipeline, req: &Request, resp: Response) -> Result<Self> {
        let provision_state = get_provisioning_state(&resp)?;
        let cur_state = match resp.status_code {
            StatusCode::Created => {
                if let Some(state) = provision_state {
                    state
                } else {
                    LROStatus::InProgress
                }
            }
            StatusCode::Ok => {
                if let Some(state) = provision_state {
                    state
                } else {
                    LROStatus::Succeeded
                }
            }
            StatusCode::NoContent => LROStatus::Succeeded,
            _ => LROStatus::InProgress,
        };

        Ok(Self {
            pl,
            resp: resp.clone(),
            poll_url: req.url().clone(),
            cur_state,
        })
    }
}

impl PollingHandler for Poller {
    // applicable returns true if the LRO is using no headers, just provisioning state.
    // This is only applicable to PATCH and PUT methods and assumes no polling headers.
    fn applicable(req: &Request, _: &Response) -> bool {
        // we can't check for absense of headers due to some misbehaving services
        // like redis that return a Location header but don't actually use that protocol
        *req.method() == Method::Put || *req.method() == Method::Patch
    }

    async fn poll(&mut self, ctx: &azure_core::http::Context<'_>) -> Result<Response> {
        if self.done() {
            return Ok(self.resp.clone());
        }
        let mut req = Request::new(self.poll_url.clone(), Method::Get);
        let resp = self.pl.send(ctx, &mut req).await?;
        let resp = Response::from_raw_response(resp).await?;
        if !utils::is_valid_status_code(resp.status_code) {
            self.resp = resp.clone();
            return Err(Error::message(resp.into(), "invalid response status code"));
        }
        if resp.status_code == StatusCode::NoContent {
            self.resp = resp.clone();
            self.cur_state = LROStatus::Succeeded;
            return Ok(resp);
        }

        if resp.body.is_empty() {
            // a missing response body in non-204 case is an error
            return Err(Error::message(
                resp.into(),
                "non-204 response has no response body",
            ));
        }
        let state = match get_provisioning_state(&resp)? {
            Some(state) => state,
            // a response body without provisioning state is considered terminal success
            None => LROStatus::Succeeded,
        };
        self.cur_state = state;
        self.resp = resp.clone();
        Ok(resp)
    }

    fn done(&self) -> bool {
        self.cur_state.is_terminal()
    }

    async fn result(&self, _: &azure_core::http::Context<'_>) -> Result<Response> {
        result_helper(&self.resp, self.cur_state.is_failed(), None)
    }
}
