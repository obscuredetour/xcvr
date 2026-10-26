//! The client against a mock Transmission: the session-id handshake, auth, and
//! error handling, which a real server only shows in the right circumstances.

use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;
use transmission::{AddOutcome, AddTorrent, Client, Config, Error, Ids, RpcCode, TorrentField, TorrentSource};
use wiremock::matchers::{body_partial_json, header, header_exists, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const RPC: &str = "/transmission/rpc";
const SESSION: &str = "X-Transmission-Session-Id";

fn client(server: &MockServer) -> Client {
    let addr = server.address();
    Client::new(Config::new(addr.ip().to_string(), addr.port())).unwrap()
}

/// The 409 Transmission sends when the session id is missing or stale.
fn conflict(session: &str) -> ResponseTemplate {
    ResponseTemplate::new(409).insert_header(SESSION, session).insert_header("X-Transmission-Rpc-Version", "6.0.1")
}

/// A JSON-RPC answer echoing the request's id.
fn answer(result: serde_json::Value) -> impl Fn(&Request) -> ResponseTemplate {
    move |req: &Request| {
        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        ResponseTemplate::new(200).set_body_json(json!({ "jsonrpc": "2.0", "result": result, "id": body["id"] }))
    }
}

/// Requests with the right session id get `respond`; anything else gets a 409 handing it out.
async fn with_session(
    server: &MockServer,
    session: &str,
    respond: impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static,
) {
    Mock::given(method("POST"))
        .and(path(RPC))
        .and(header(SESSION, session))
        .respond_with(respond)
        .with_priority(1)
        .mount(server)
        .await;
    Mock::given(method("POST")).and(path(RPC)).respond_with(conflict(session)).with_priority(10).mount(server).await;
}

#[tokio::test]
async fn handshake_then_retry() {
    let server = MockServer::start().await;
    with_session(&server, "abc", answer(json!({ "version": "4.1.3 (deadbeef)", "rpc_version_semver": "6.0.1" }))).await;

    let c = client(&server);
    let s = c.session_get(&[]).await.unwrap();
    assert_eq!(s.version.as_deref(), Some("4.1.3 (deadbeef)"));
    assert_eq!(c.rpc_version().as_deref(), Some("6.0.1"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2, "one 409, then the retry");
    assert!(requests[0].headers.get(SESSION).is_none());
    assert_eq!(requests[1].headers.get(SESSION).unwrap(), "abc");

    // The id is kept: the next call goes straight through.
    c.session_stats().await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn requests_are_json_rpc() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header(SESSION, "s"))
        .and(header("content-type", "application/json"))
        .and(body_partial_json(json!({
            "jsonrpc": "2.0",
            "method": "torrent_get",
            "params": { "ids": ["cafe"], "fields": ["hash_string", "name"] }
        })))
        .respond_with(answer(json!({ "torrents": [{ "hash_string": "cafe", "name": "Debian" }] })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST")).respond_with(conflict("s")).with_priority(10).mount(&server).await;

    let got = client(&server)
        .torrent_get(&Ids::hashes(["cafe"]), &[TorrentField::HashString, TorrentField::Name])
        .await
        .unwrap();
    assert_eq!(got.torrents[0].name.as_deref(), Some("Debian"));
    assert!(got.removed.is_empty());
}

/// A server whose session id changes after it has answered twice, as when
/// Transmission restarts or expires the id.
struct RotatingSession {
    answered: AtomicUsize,
}

impl Respond for RotatingSession {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        let current = if self.answered.load(Ordering::SeqCst) < 2 { "first" } else { "second" };
        if req.headers.get(SESSION).and_then(|v| v.to_str().ok()) != Some(current) {
            return conflict(current);
        }
        self.answered.fetch_add(1, Ordering::SeqCst);
        answer(json!({}))(req)
    }
}

#[tokio::test]
async fn expired_session_is_renewed() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(RotatingSession { answered: AtomicUsize::new(0) }).mount(&server).await;

    let c = client(&server);
    c.torrent_start(&Ids::All).await.unwrap(); // 409 "first", answered
    c.torrent_stop(&Ids::All).await.unwrap(); // answered; the id now rotates
    c.torrent_start(&Ids::All).await.unwrap(); // 409 "second", answered

    let sent: Vec<_> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| r.headers.get(SESSION).map(|v| v.to_str().unwrap().to_owned()))
        .collect();
    assert_eq!(sent, [None, Some("first"), Some("first"), Some("first"), Some("second")].map(|s| s.map(str::to_owned)));
}

#[tokio::test]
async fn endless_409_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(conflict("never-accepted")).mount(&server).await;
    let err = client(&server).session_stats().await.unwrap_err();
    assert!(matches!(err, Error::Protocol(_)), "{err:?}");
}

#[tokio::test]
async fn old_transmission_is_refused() {
    let server = MockServer::start().await;
    // Transmission 4.0: a 409 without the RPC version header.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(409).insert_header(SESSION, "x"))
        .mount(&server)
        .await;
    let err = client(&server).session_stats().await.unwrap_err();
    assert!(matches!(err, Error::UnsupportedVersion { .. }), "{err:?}");
}

#[tokio::test]
async fn credentials_are_sent_and_rejections_reported() {
    let server = MockServer::start().await;
    // "alice:secret"
    Mock::given(header("authorization", "Basic YWxpY2U6c2VjcmV0"))
        .and(header_exists(SESSION))
        .respond_with(answer(json!({})))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(header("authorization", "Basic YWxpY2U6c2VjcmV0"))
        .respond_with(conflict("s"))
        .with_priority(5)
        .mount(&server)
        .await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(401)).with_priority(10).mount(&server).await;

    let addr = server.address();
    let good =
        Client::new(Config::new(addr.ip().to_string(), addr.port()).with_credentials("alice", "secret")).unwrap();
    good.torrent_verify(&Ids::one(1)).await.unwrap();

    let bad = Client::new(Config::new(addr.ip().to_string(), addr.port()).with_credentials("alice", "wrong")).unwrap();
    assert!(matches!(bad.torrent_verify(&Ids::one(1)).await.unwrap_err(), Error::Unauthorized));
}

#[tokio::test]
async fn forbidden_page_becomes_a_message() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(403).set_body_string("<h1>403: Forbidden</h1><p>Unauthorized IP Address.</p>"),
        )
        .mount(&server)
        .await;
    match client(&server).session_stats().await.unwrap_err() {
        Error::Forbidden(msg) => assert_eq!(msg, "403: Forbidden Unauthorized IP Address."),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn rpc_errors_carry_their_detail() {
    let server = MockServer::start().await;
    with_session(&server, "s", |req: &Request| {
        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "error": { "code": -32602, "message": "Invalid params", "data": { "error_string": "labels cannot contain comma (,) character" } },
            "id": body["id"]
        }))
    })
    .await;

    let set = transmission::TorrentSet { labels: Some(vec!["a,b".into()]), ..Default::default() };
    let err = client(&server).torrent_set(&Ids::one(1), &set).await.unwrap_err();
    match &err {
        Error::Rpc { code, message, detail } => {
            assert_eq!(*code, RpcCode::InvalidParams);
            assert_eq!(message, "Invalid params");
            assert_eq!(detail.as_deref(), Some("labels cannot contain comma (,) character"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(err.to_string(), "labels cannot contain comma (,) character");
}

#[tokio::test]
async fn duplicates_are_reported_as_such() {
    let server = MockServer::start().await;
    with_session(
        &server,
        "s",
        answer(json!({ "torrent_duplicate": { "id": 7, "name": "Debian", "hash_string": "cafe" } })),
    )
    .await;
    let out = client(&server)
        .torrent_add(&AddTorrent::new(TorrentSource::Link("magnet:?xt=urn:btih:cafe".into())))
        .await
        .unwrap();
    assert_eq!(
        out,
        AddOutcome::Duplicate(transmission::AddedTorrent { id: 7, name: "Debian".into(), hash_string: "cafe".into() })
    );
}

#[tokio::test]
async fn metainfo_is_sent_as_base64() {
    let server = MockServer::start().await;
    Mock::given(header(SESSION, "s"))
        .and(body_partial_json(json!({ "params": { "metainfo": "ZDQ6dGVzdGU=", "paused": true, "labels": ["iso"] } })))
        .respond_with(answer(json!({ "torrent_added": { "id": 1, "name": "test", "hash_string": "beef" } })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST")).respond_with(conflict("s")).with_priority(10).mount(&server).await;

    let mut add = AddTorrent::new(TorrentSource::Metainfo(b"d4:teste".to_vec()));
    add.paused = true;
    add.labels = vec!["iso".into()];
    let out = client(&server).torrent_add(&add).await.unwrap();
    assert!(matches!(out, AddOutcome::Added(t) if t.hash_string == "beef"));
}

#[tokio::test]
async fn nothing_listening_is_unreachable() {
    // Bind a port, then free it, so nothing answers there.
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let c = Client::new(Config::new("127.0.0.1", port)).unwrap();
    let err = c.session_stats().await.unwrap_err();
    assert!(matches!(err, Error::Unreachable { .. }), "{err:?}");
    assert!(err.is_connection());
}
