use std::fmt::{Debug, Display, Formatter};
use crate::gemtext::gemtext_body::{GemTextBody, MimeType};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OkResponse {
    pub mime: MimeType,
    pub body: GemTextBody,
}

// FIXME: Cow

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Response {
    /// Input Expected
    /// 10
    MustPromptForInput(String),
    /// 11
    MustPromptSensitiveInput(String),

    /// Success
    /// 20
    Success(OkResponse),

    /// Redirection
    /// 30
    TemporaryRedirect(String),
    /// 31
    PermanentRedirect(String),

    /// Temporary Failure
    /// 40
    UnexpectedErrorTryAgain(Option<String>),
    /// 41
    ServerUnavailable(Option<String>),
    /// 42
    CGIError(Option<String>),
    /// 43
    ProxyError(Option<String>),
    /// 44
    SlowDown(Option<String>),

    /// Permament Failure
    /// 50
    PermanentFailure(Option<String>),
    /// 51
    ResourceNotFound(Option<String>),
    /// 52
    ResourceGone(Option<String>),
    /// 53
    ProxyRequestRefused(Option<String>),
    /// 59
    BadRequest(Option<String>),

    /// Client Certificates
    /// 60
    CertificateRequired(Option<String>),
    /// 61
    CertificateNotAuthorized(Option<String>),
    /// 62
    CertificateNotValid(Option<String>),
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Response::*;

        match self {
            MustPromptForInput(p) => f.write_fmt(format_args!("Input Expected ({})", p)),
            MustPromptSensitiveInput(_) => f.write_str("Input Expected (Sensitive)"),
            Success(b) => f.write_fmt(format_args!("Success ({})", b.mime)),
            TemporaryRedirect(path) => f.write_fmt(format_args!("Temporary Redirect to {}", path)),
            PermanentRedirect(path) => f.write_fmt(format_args!("Permanent Redirect to {}", path)),
            UnexpectedErrorTryAgain(msg) => f.write_fmt(format_args!("Unexpected Error, Try Again ({:?})", msg)),
            ServerUnavailable(msg) => f.write_fmt(format_args!("Server Unavailable ({:?})", msg)),
            CGIError(msg) => f.write_fmt(format_args!("CGI Error ({:?})", msg)),
            ProxyError(msg) => f.write_fmt(format_args!("Proxy Error ({:?})", msg)),
            SlowDown(msg) => f.write_fmt(format_args!("Slow Down ({:?})", msg)),
            PermanentFailure(msg) => f.write_fmt(format_args!("Permanent Failure ({:?})", msg)),
            ResourceNotFound(msg) => f.write_fmt(format_args!("Resource Not Found ({:?})", msg)),
            ResourceGone(msg) => f.write_fmt(format_args!("Resource Gone ({:?})", msg)),
            ProxyRequestRefused(msg) => f.write_fmt(format_args!("Proxy Request Refused ({:?})", msg)),
            BadRequest(msg) => f.write_fmt(format_args!("Bad Request ({:?})", msg)),
            CertificateRequired(msg) => f.write_fmt(format_args!("Client Certificate Required ({:?})", msg)),
            CertificateNotAuthorized(msg) => f.write_fmt(format_args!("Certificate Not Authorized ({:?})", msg)),
            CertificateNotValid(msg) => f.write_fmt(format_args!("Certificate Not Valid ({:?})", msg)),
        }
    }
}