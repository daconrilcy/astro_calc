use astral_llm_infra::CalculatorClient;
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
};

#[tokio::test]
async fn calculator_client_uses_internal_calculation_paths() {
    let (base_url, mut requests) = spawn_calculator_stub(4).await;
    let client = CalculatorClient::new(base_url, Some("test-key".to_string()), 5_000)
        .expect("calculator client");

    client
        .calculate_simplified_natal(&json!({ "request": "simplified" }))
        .await
        .expect("simplified request");
    client
        .calculate_natal(&json!({ "request": "natal" }))
        .await
        .expect("natal request");
    client
        .calculate_horoscope_daily_natal(&json!({ "request": "daily" }))
        .await
        .expect("daily horoscope request");
    client
        .calculate_horoscope_period_natal(&json!({ "request": "period" }))
        .await
        .expect("period horoscope request");

    let observed = collect_requests(&mut requests, 4).await;
    assert_eq!(
        observed,
        vec![
            "POST /v1/internal/calculations/natal/simplified HTTP/1.1",
            "POST /v1/internal/calculations/natal HTTP/1.1",
            "POST /v1/internal/calculations/horoscope/daily-natal HTTP/1.1",
            "POST /v1/internal/calculations/horoscope/period/natal HTTP/1.1",
        ]
    );
}

async fn spawn_calculator_stub(expected_requests: usize) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind calculator stub");
    let address = listener.local_addr().expect("calculator stub addr");
    let (sender, receiver) = mpsc::channel(expected_requests);

    tokio::spawn(async move {
        for _ in 0..expected_requests {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let request_line = read_request_line(&mut stream).await;
            sender.send(request_line).await.expect("record request");
            let body = r#"{"ok":true}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        }
    });

    (format!("http://{address}"), receiver)
}

async fn read_request_line(stream: &mut tokio::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    loop {
        let mut chunk = [0_u8; 512];
        let read = stream.read(&mut chunk).await.expect("read request");
        assert!(read > 0, "request closed before headers");
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(2).any(|window| window == b"\r\n") {
            break;
        }
    }
    let headers = String::from_utf8_lossy(&buffer);
    headers.lines().next().expect("request line").to_string()
}

async fn collect_requests(receiver: &mut mpsc::Receiver<String>, expected: usize) -> Vec<String> {
    let mut observed = Vec::with_capacity(expected);
    for _ in 0..expected {
        observed.push(receiver.recv().await.expect("observed request"));
    }
    observed
}
