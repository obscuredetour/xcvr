use std::fmt;

/// Everything a call can fail with. The messages are written to be shown to
/// the user as they are.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Nothing answered at the address: the host is off, asleep, or not on this network.
    #[error("Couldn't reach Transmission at {url}.")]
    Unreachable {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Transmission didn't answer in time.")]
    Timeout,

    /// HTTP 401.
    #[error("Transmission rejected the username or password.")]
    Unauthorized,

    /// HTTP 403: usually the RPC allow list (`rpc_whitelist`) doesn't include this machine.
    #[error("Transmission refused the connection: {0}")]
    Forbidden(String),

    /// Any other HTTP status that isn't a JSON-RPC response.
    #[error("Transmission answered with HTTP {status}{}", if body.is_empty() { String::new() } else { format!(": {body}") })]
    Http { status: u16, body: String },

    /// The server speaks an older RPC protocol than JSON-RPC (Transmission before 4.1).
    #[error(
        "This Transmission is too old: it speaks RPC {found}, and Transceiver needs 6.0 or later (Transmission 4.1)."
    )]
    UnsupportedVersion { found: String },

    /// An error object from Transmission.
    #[error("{}", detail.as_deref().unwrap_or(message))]
    Rpc { code: RpcCode, message: String, detail: Option<String> },

    /// The response wasn't what the protocol says it should be.
    #[error("Unexpected response from Transmission: {0}")]
    Protocol(String),

    #[error("Unexpected response from Transmission: {0}")]
    Decode(#[from] serde_json::Error),

    /// Any other transport problem.
    #[error("Couldn't talk to Transmission: {0}")]
    Request(#[source] reqwest::Error),
}

impl Error {
    /// Problems with reaching the host, as opposed to a request it understood and refused.
    pub fn is_connection(&self) -> bool {
        matches!(self, Error::Unreachable { .. } | Error::Timeout | Error::Request(_))
    }

    /// The OS said there's no route to the host. On macOS that's usually Local Network
    /// privacy blocking the app, not the host being away (a host that's away times out).
    pub fn is_host_unreachable(&self) -> bool {
        let Error::Unreachable { source, .. } = self else { return false };
        let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(source);
        while let Some(e) = cause {
            if let Some(io) = e.downcast_ref::<std::io::Error>()
                && io.kind() == std::io::ErrorKind::HostUnreachable
            {
                return true;
            }
            cause = e.source();
        }
        false
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// JSON-RPC error codes, the standard ones and Transmission's own
/// (`JsonRpc::Error::Code` in libtransmission/rpcimpl.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    SetAnnounceList,
    InvalidTrackerList,
    PathNotAbsolute,
    UnrecognizedInfo,
    SystemError,
    FileIndexOutOfRange,
    PieceIndexOutOfRange,
    HttpError,
    CorruptTorrent,
    Other(i64),
}

impl From<i64> for RpcCode {
    fn from(code: i64) -> Self {
        match code {
            -32700 => Self::ParseError,
            -32600 => Self::InvalidRequest,
            -32601 => Self::MethodNotFound,
            -32602 => Self::InvalidParams,
            -32603 => Self::InternalError,
            1 => Self::SetAnnounceList,
            2 => Self::InvalidTrackerList,
            3 => Self::PathNotAbsolute,
            4 => Self::UnrecognizedInfo,
            5 => Self::SystemError,
            6 => Self::FileIndexOutOfRange,
            7 => Self::PieceIndexOutOfRange,
            8 => Self::HttpError,
            9 => Self::CorruptTorrent,
            other => Self::Other(other),
        }
    }
}

impl fmt::Display for RpcCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
