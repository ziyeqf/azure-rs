use azure_core::error::{http_response_from_body, ErrorKind};
use azure_core::http::{headers::Headers, RawResponse, StatusCode};
use azure_core::{Error, Result};
use bytes::Bytes;

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
