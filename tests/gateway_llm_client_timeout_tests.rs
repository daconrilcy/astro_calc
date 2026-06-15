use astral_gateway::{clients::HttpLlmClient, ports::LlmPort};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::test]
async fn gateway_llm_client_does_not_retry_horoscope_period_on_http_408() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let base_url = spawn_stub(
        attempts.clone(),
        vec![StubResponse::new(408, "Request Timeout")],
    )
    .await;
    let client = HttpLlmClient::new(base_url, None, 5_000).expect("client");

    let error = client
        .render_horoscope_period(&json!({ "request": "period-basic" }))
        .await
        .expect_err("period timeout must not be retried");

    assert!(error.to_string().contains("timed out"));
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn gateway_llm_client_does_not_retry_horoscope_period_on_validation_422() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let base_url = spawn_stub(
        attempts.clone(),
        vec![StubResponse::new(
            422,
            r#"{"error":{"code":"POST_SAFETY_VALIDATION_FAILED"}}"#,
        )],
    )
    .await;
    let client = HttpLlmClient::new(base_url, None, 5_000).expect("client");

    let error = client
        .render_horoscope_period(&json!({ "request": "period-basic" }))
        .await
        .expect_err("validation errors must not be retried");

    assert!(error.to_string().contains("status=422"));
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn gateway_llm_client_retries_daily_on_http_408() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let base_url = spawn_stub(
        attempts.clone(),
        vec![
            StubResponse::new(408, "Request Timeout"),
            StubResponse::new(200, r#"{"ok":true}"#),
        ],
    )
    .await;
    let client = HttpLlmClient::new(base_url, None, 5_000).expect("client");

    let response = client
        .render_horoscope_daily(&json!({ "request": "daily-basic" }))
        .await
        .expect("daily timeout should retry once");

    assert_eq!(response, json!({ "ok": true }));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[derive(Clone)]
struct StubResponse {
    status: u16,
    body: &'static str,
}

impl StubResponse {
    fn new(status: u16, body: &'static str) -> Self {
        Self { status, body }
    }
}

async fn spawn_stub(attempts: Arc<AtomicUsize>, responses: Vec<StubResponse>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind stub");
    let address = listener.local_addr().expect("stub addr");
    tokio::spawn(async move {
        for response in responses {
            let (mut stream, _) = listener.accept().await.expect("accept");
            read_http_request(&mut stream).await;
            attempts.fetch_add(1, Ordering::SeqCst);
            let http_response = format!(
                "HTTP/1.1 {} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.status,
                response.body.len(),
                response.body
            );
            stream
                .write_all(http_response.as_bytes())
                .await
                .expect("write response");
        }
    });
    format!("http://{address}")
}

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> Value {
    let mut buffer = Vec::new();
    let header_end = loop {
        let mut chunk = [0_u8; 1024];
        let read = stream.read(&mut chunk).await.expect("read request");
        assert!(read > 0, "request closed before headers");
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(pos) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let headers = String::from_utf8_lossy(&buffer[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    while buffer.len() < header_end + content_length {
        let mut chunk = [0_u8; 1024];
        let read = stream.read(&mut chunk).await.expect("read body");
        assert!(read > 0, "request closed before body");
        buffer.extend_from_slice(&chunk[..read]);
    }
    serde_json::from_slice(&buffer[header_end..header_end + content_length]).unwrap_or(Value::Null)
}
