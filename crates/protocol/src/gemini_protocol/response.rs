use crate::gemtext::gemtext_body::{GemTextBody, MimeType};
use std::fmt::{Debug, Display, Formatter};

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

        macro_rules! display_option {
            ($variant:ident, $msg:expr, $text:expr) => {
                match $msg {
                    Some(m) => f.write_fmt(format_args!(concat!($text, " ({})"), m)),
                    None => f.write_str($text),
                }
            };
        }

        match self {
            MustPromptForInput(p) => f.write_fmt(format_args!("Input Expected ({})", p)),
            MustPromptSensitiveInput(_) => f.write_str("Input Expected (Sensitive)"),
            Success(b) => f.write_fmt(format_args!("Success ({})", b.mime)),
            TemporaryRedirect(path) => f.write_fmt(format_args!("Temporary Redirect to {}", path)),
            PermanentRedirect(path) => f.write_fmt(format_args!("Permanent Redirect to {}", path)),
            UnexpectedErrorTryAgain(msg) => {
                display_option!(UnexpectedErrorTryAgain, msg, "Unexpected Error, Try Again")
            }
            ServerUnavailable(msg) => display_option!(ServerUnavailable, msg, "Server Unavailable"),
            CGIError(msg) => display_option!(CGIError, msg, "CGI Error"),
            ProxyError(msg) => display_option!(ProxyError, msg, "Proxy Error"),
            SlowDown(msg) => display_option!(SlowDown, msg, "Slow Down"),
            PermanentFailure(msg) => display_option!(PermanentFailure, msg, "Permanent Failure"),
            ResourceNotFound(msg) => display_option!(ResourceNotFound, msg, "Resource Not Found"),
            ResourceGone(msg) => display_option!(ResourceGone, msg, "Resource Gone"),
            ProxyRequestRefused(msg) => {
                display_option!(ProxyRequestRefused, msg, "Proxy Request Refused")
            }
            BadRequest(msg) => display_option!(BadRequest, msg, "Bad Request"),
            CertificateRequired(msg) => {
                display_option!(CertificateRequired, msg, "Client Certificate Required")
            }
            CertificateNotAuthorized(msg) => {
                display_option!(CertificateNotAuthorized, msg, "Certificate Not Authorized")
            }
            CertificateNotValid(msg) => {
                display_option!(CertificateNotValid, msg, "Certificate Not Valid")
            }
        }
    }
}
