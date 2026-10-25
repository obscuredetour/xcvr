use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result, RpcCode};

pub const DEFAULT_PORT: u16 = 9091;
pub const DEFAULT_PATH: &str = "/transmission/rpc";

const SESSION_HEADER: &str = "X-Transmission-Session-Id";
/// Sent with every 409 from RPC 6.0.0 on, so a client can tell JSON-RPC servers from older ones.
const VERSION_HEADER: &str = "X-Transmission-Rpc-Version";
/// A 409 hands out a new session id; more than this many in a row means something's wrong.
const MAX_SESSION_RETRIES: usize = 3;

/// Where Transmission is and how to sign in.
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    /// The RPC path, `/transmission/rpc` unless the host changed it.
    pub path: String,
    pub username: Option<String>,
    pub password: Option<String>,
    /// How long a whole request may take. Adding a torrent from a URL makes
    /// Transmission fetch it before answering, so this isn't short.
    pub timeout: Duration,
    pub connect_timeout: Duration,
}

impl Config {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            path: DEFAULT_PATH.to_owned(),
            username: None,
            password: None,
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(5),
        }
    }

    pub fn with_credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    pub fn url(&self) -> String {
        // An IPv6 literal goes in brackets.
        let host = if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        let path = if self.path.starts_with('/') { self.path.clone() } else { format!("/{}", self.path) };
        format!("http://{host}:{}{path}", self.port)
    }
}

/// A connection to one Transmission. Cheap to share (`Arc`): requests can run
/// at once, and the session id they need is kept here.
pub struct Client {
    http: reqwest::Client,
    url: String,
    username: Option<String>,
    password: Option<String>,
    session_id: Mutex<Option<String>>,
    rpc_version: Mutex<Option<String>>,
    next_id: AtomicU64,
}

#[derive(Serialize)]
struct Request<'a, P> {
    jsonrpc: &'static str,
    method: &'a str,
    params: &'a P,
    id: u64,
}

#[derive(Deserialize)]
struct Response<R> {
    result: Option<R>,
    error: Option<ErrorObject>,
}

#[derive(Deserialize)]
struct ErrorObject {
    code: i64,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Option<ErrorData>,
}

#[derive(Deserialize)]
struct ErrorData {
    #[serde(default)]
    error_string: Option<String>,
}

/// Params for methods that take none.
#[derive(Serialize)]
pub(crate) struct NoParams {}

impl Client {
    pub fn new(config: Config) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .build()
            .map_err(Error::Request)?;
        Ok(Self {
            http,
            url: config.url(),
            username: config.username.filter(|u| !u.is_empty()),
            password: config.password,
            session_id: Mutex::new(None),
            rpc_version: Mutex::new(None),
            next_id: AtomicU64::new(1),
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// The RPC version the server reported in its last 409 (`6.0.1` on 4.1.1), if it has sent one.
    pub fn rpc_version(&self) -> Option<String> {
        self.rpc_version.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Call `method` with `params`, handling the session-id handshake, and
    /// decode its `result` as `R`.
    pub async fn call<P: Serialize, R: DeserializeOwned>(&self, method: &str, params: &P) -> Result<R> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = serde_json::to_vec(&Request { jsonrpc: "2.0", method, params, id })?;

        for _ in 0..=MAX_SESSION_RETRIES {
            let mut request =
                self.http.post(&self.url).header(reqwest::header::CONTENT_TYPE, "application/json").body(body.clone());
            if let Some(session) = self.session_id() {
                request = request.header(SESSION_HEADER, session);
            }
            if let Some(user) = &self.username {
                request = request.basic_auth(user, self.password.as_deref());
            }

            let response = request.send().await.map_err(|e| self.transport_error(e))?;
            match response.status() {
                StatusCode::CONFLICT => {
                    self.take_handshake(&response)?;
                    continue;
                }
                StatusCode::UNAUTHORIZED => return Err(Error::Unauthorized),
                StatusCode::FORBIDDEN => {
                    let text = response.text().await.unwrap_or_default();
                    return Err(Error::Forbidden(summarize(&text)));
                }
                status if !status.is_success() => {
                    let text = response.text().await.unwrap_or_default();
                    return Err(Error::Http { status: status.as_u16(), body: summarize(&text) });
                }
                _ => {}
            }

            let bytes = response.bytes().await.map_err(|e| self.transport_error(e))?;
            let parsed: Response<R> = serde_json::from_slice(&bytes)?;
            if let Some(err) = parsed.error {
                return Err(Error::Rpc {
                    code: RpcCode::from(err.code),
                    message: err.message,
                    detail: err.data.and_then(|d| d.error_string).filter(|s| !s.is_empty()),
                });
            }
            return parsed.result.ok_or_else(|| Error::Protocol("no result or error in the response".into()));
        }
        Err(Error::Protocol("Transmission kept asking for a new session id".into()))
    }

    fn session_id(&self) -> Option<String> {
        self.session_id.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// A 409: keep the session id it hands out, and check it speaks JSON-RPC.
    fn take_handshake(&self, response: &reqwest::Response) -> Result<()> {
        let headers = response.headers();
        let version = headers.get(VERSION_HEADER).and_then(|v| v.to_str().ok()).map(str::to_owned);
        match &version {
            Some(v) if major(v).is_some_and(|m| m >= crate::RPC_MAJOR) => {}
            // No header at all: Transmission 4.0 or earlier.
            other => {
                return Err(Error::UnsupportedVersion {
                    found: other.clone().unwrap_or_else(|| "5.x or older".into()),
                });
            }
        }
        *self.rpc_version.lock().unwrap_or_else(|e| e.into_inner()) = version;

        let session = headers
            .get(SESSION_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::Protocol("409 without a session id".into()))?;
        *self.session_id.lock().unwrap_or_else(|e| e.into_inner()) = Some(session.to_owned());
        Ok(())
    }

    fn transport_error(&self, e: reqwest::Error) -> Error {
        if e.is_timeout() {
            Error::Timeout
        } else if e.is_connect() {
            Error::Unreachable { url: self.url.clone(), source: e }
        } else {
            Error::Request(e)
        }
    }
}

/// `6.0.1` → 6.
fn major(version: &str) -> Option<u64> {
    version.trim().split('.').next()?.parse().ok()
}

/// An error page as one short line: tags stripped, whitespace collapsed.
fn summarize(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let line = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() > 200 { format!("{}…", line.chars().take(200).collect::<String>()) } else { line }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_forms() {
        assert_eq!(Config::new("mac-mini.local", 9091).url(), "http://mac-mini.local:9091/transmission/rpc");
        assert_eq!(Config::new("fe80::1", 9091).url(), "http://[fe80::1]:9091/transmission/rpc");
        let mut c = Config::new("10.0.0.2", 8080);
        c.path = "rpc".into();
        assert_eq!(c.url(), "http://10.0.0.2:8080/rpc");
    }

    #[test]
    fn version_major() {
        assert_eq!(major("6.0.1"), Some(6));
        assert_eq!(major("5"), Some(5));
        assert_eq!(major("x"), None);
    }

    #[test]
    fn error_pages_become_one_line() {
        let page = "<html><head><title>403: Forbidden</title></head><body><h1>403: Forbidden</h1>\
                    <p>Unauthorized IP Address.</p><p>Either disable the IP address whitelist</p></body></html>";
        assert_eq!(
            summarize(page),
            "403: Forbidden 403: Forbidden Unauthorized IP Address. Either disable the IP address whitelist"
        );
    }
}
