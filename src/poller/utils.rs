use chrono::{DateTime, Utc};
use std::{
    fmt::{write, Display},
    time::Duration,
};

use super::Response;

use azure_core::{
    http::{headers::HeaderName, StatusCode},
    Result,
};
use serde_json::{from_slice, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LROStatus {
    Unknown,
    Succeeded,
    Canceled,
    Failed,
    InProgress,

    // Followings are non-conformant states that been seen in the wild
    Cancelled,
    Completed,
}

impl Display for LROStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LROStatus::Succeeded => "Succeeded",
            LROStatus::Canceled => "Canceled",
            LROStatus::Failed => "Failed",
            LROStatus::InProgress => "InProgress",
            LROStatus::Cancelled => "Cancelled",
            LROStatus::Completed => "Completed",
            LROStatus::Unknown => "<unknown>",
        };
        f.write_str(s)
    }
}

impl LROStatus {
    pub fn from_str(v: &str) -> Self {
        match v {
            "Succeeded" => LROStatus::Succeeded,
            "Canceled" => LROStatus::Canceled,
            "Failed" => LROStatus::Failed,
            "InProgress" => LROStatus::InProgress,
            "Cancelled" => LROStatus::Cancelled,
            "Completed" => LROStatus::Completed,
            _ => LROStatus::Unknown,
        }
    }

    pub fn is_failed(&self) -> bool {
        [LROStatus::Failed, LROStatus::Canceled, LROStatus::Cancelled]
            .iter()
            .find(|v| **v == *self)
            .is_some()
    }

    pub fn is_succeeded(&self) -> bool {
        [LROStatus::Succeeded, LROStatus::Completed]
            .iter()
            .find(|v| **v == *self)
            .is_some()
    }

    pub fn is_terminal(&self) -> bool {
        self.is_failed() || self.is_succeeded()
    }
}

// get_provisioning_state returns the LRO's state from the response body.
// If there is no state in the response body the None is returned.
pub fn get_provisioning_state(resp: &Response) -> Result<Option<LROStatus>> {
    let m: HashMap<String, Value> = from_slice(&resp.body)?;
    let props = m.get("properties").and_then(|v| v.as_object());
    let state = props
        .and_then(|p| p.get("provisioningState"))
        .and_then(|s| s.as_str());
    Ok(state.map(LROStatus::from_str))
}

// get_lro_status returns the LRO's status from the response body.
// Typically used for Azure-AsyncOperation flows.
// If there is no status in the response body the None is returned.
pub fn get_lro_status(resp: &Response) -> Result<Option<LROStatus>> {
    let m: HashMap<String, Value> = from_slice(&resp.body)?;
    let status = m.get("status").and_then(|v| v.as_str());
    Ok(status.map(LROStatus::from_str))
}

pub fn retry_after(resp: &Response) -> Option<Duration> {
    struct Candidate {
        header: &'static str,
        to_duration: fn(u64) -> Duration,
        // custom is used when the regular algorithm failed and is optional.
        // the returned duration is used verbatim (units is not applied).
        custom: Option<fn(&str) -> Option<Duration>>,
    }

    let candidates = vec![
        Candidate {
            header: "Retry-After-Ms",
            to_duration: Duration::from_millis,
            custom: None,
        },
        Candidate {
            header: "x-ms-retry-after-ms",
            to_duration: Duration::from_millis,
            custom: None,
        },
        Candidate {
            header: "Retry-After",
            to_duration: Duration::from_secs,
            custom: Some(|s| {
                if let Ok(t) = DateTime::parse_from_rfc2822(s) {
                    let now = Utc::now();
                    let d = now.signed_duration_since(t.with_timezone(&Utc));
                    Some(Duration::from_millis(d.num_milliseconds() as u64))
                } else {
                    None
                }
            }),
        },
    ];

    for c in &candidates {
        if let Some(v) = resp
            .headers
            .get_optional_str(&HeaderName::from_static(c.header))
        {
            if let Ok(v) = v.parse::<u64>() {
                return Some((c.to_duration)(v));
            } else if let Some(custom) = c.custom {
                return custom(v);
            }
        }
    }

    None
}

pub fn is_valid_status_code(status_code: StatusCode) -> bool {
    [
        StatusCode::Ok,
        StatusCode::Accepted,
        StatusCode::Created,
        StatusCode::NoContent,
    ]
    .iter()
    .find(|&&code| code == status_code)
    .is_some()
}

// IsNonTerminalHTTPStatusCode returns true if the HTTP status code should be
// considered non-terminal thus eligible for retry.
pub fn is_non_terminal_http_status_code(status_code: StatusCode) -> bool {
    [
        StatusCode::RequestTimeout,
        StatusCode::TooManyRequests,
        StatusCode::InternalServerError,
        StatusCode::BadGateway,
        StatusCode::ServiceUnavailable,
        StatusCode::GatewayTimeout,
    ]
    .iter()
    .find(|v| **v == status_code)
    .is_some()
}
