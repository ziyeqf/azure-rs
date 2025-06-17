use azure_core::http::{headers::LOCATION, Pipeline, Request, Url};
use azure_core::http::{Method, StatusCode};

use azure_core::{Error, Result};

use crate::poller::utils::{get_provisioning_state, is_non_terminal_http_status_code};

use super::utils::result_helper;
use super::PollingHandler;
use super::{utils::LROStatus, Response};

pub struct Poller {
    pl: Pipeline,

    // The response of the last call (either sync or poll call).
    resp: Response,

    loc_url: Url,

    // The LRO's current state.
    cur_state: LROStatus,
}

impl Poller {
    pub fn new(pl: Pipeline, resp: Response) -> Result<Self> {
        let loc_url = resp.headers.get_as(&LOCATION).map_err(|err| {
            err.context(format!("parsing header `{}` as a URL", LOCATION.as_str()))
        })?;
        let cur_state = get_provisioning_state(&resp)?.unwrap_or(LROStatus::InProgress);
        Ok(Self {
            pl,
            resp,
            loc_url,
            cur_state,
        })
    }
}

impl PollingHandler for Poller {
    fn applicable(resp: &Response) -> bool {
        resp.headers.get_optional_str(&LOCATION).is_some()
    }

    async fn poll(&mut self, ctx: &azure_core::http::Context<'_>) -> Result<Response> {
        if self.done() {
            return Ok(self.resp.clone());
        }
        let mut req = Request::new(self.loc_url.clone(), Method::Get);
        let resp = self.pl.send(ctx, &mut req).await?;
        let resp = Response::from_raw_response(resp).await?;

        self.resp = resp.clone();

        // location polling can return an updated polling URL
        if let Some(loc_url) = resp.headers.get_optional_as(&LOCATION)? {
            self.loc_url = loc_url;
        }

        // if provisioning state is available, use that. this is only
        // for some ARM LRO scenarios (e.g. DELETE with a Location header)
        // so if it's missing then use HTTP status code.
        if let Some(provision_state) = get_provisioning_state(&resp)? {
            self.cur_state = provision_state;
        } else if resp.status_code == StatusCode::Accepted {
            self.cur_state = LROStatus::InProgress;
        } else if resp.status_code.is_success() {
            // any 2xx other than a 202 indicates success
            self.cur_state = LROStatus::Succeeded;
        } else if is_non_terminal_http_status_code(resp.status_code) {
            // the request timed out or is being throttled.
            // DO NOT include this as a terminal failure. preserve
            // the existing state and return the response.
        } else {
            self.cur_state = LROStatus::Failed;
        }
        Ok(self.resp.clone())
    }

    fn done(&self) -> bool {
        self.cur_state.is_terminal()
    }

    async fn result(&self, ctx: &azure_core::http::Context<'_>) -> Result<Response> {
        result_helper(&self.resp, self.cur_state.is_failed(), None)
    }
}
