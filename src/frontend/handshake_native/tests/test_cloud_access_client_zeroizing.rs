use std::sync::Arc;

use handshake_native::backend_client::CloudAccessClient;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeroize::Zeroizing;

const CANARY: &str = "sk-mt017-native-zeroizing-canary-NEVER-RETURN";

fn header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .expect("request carries Content-Length")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_access_client_consumes_zeroizing_key_on_success_and_transport_failure() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind native-client capture server");
    let addr = listener.local_addr().expect("capture server address");
    let (request_tx, request_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener
            .accept()
            .await
            .expect("accept native-client request");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 2048];
        loop {
            let read = stream
                .read(&mut chunk)
                .await
                .expect("read native-client request");
            assert!(read > 0, "client closed before completing request body");
            request.extend_from_slice(&chunk[..read]);
            if let Some(end) = header_end(&request) {
                let headers = std::str::from_utf8(&request[..end]).expect("request headers utf8");
                if request.len() >= end + content_length(headers) {
                    break;
                }
            }
        }
        request_tx.send(request).expect("deliver captured request");
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}",
            )
            .await
            .expect("write native-client success response");
    });

    let client = CloudAccessClient::new(format!("http://{addr}"));
    let success = client
        .store_key("openai", Zeroizing::new(CANARY.to_owned()))
        .await;
    assert!(
        success.is_ok(),
        "zeroizing success request failed: {success:?}"
    );
    assert!(
        !format!("{success:?}").contains(CANARY),
        "successful return surface must not echo key material"
    );
    let request = request_rx
        .await
        .expect("receive captured native-client request");
    server.await.expect("capture server completes");
    let end = header_end(&request).expect("captured request header terminator");
    let body: serde_json::Value =
        serde_json::from_slice(&request[end..]).expect("captured request body is JSON");
    assert_eq!(
        body["api_key"], CANARY,
        "the emitted request body receives the borrowed zeroizing secret exactly once"
    );

    let unavailable = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("reserve unavailable endpoint");
    let unavailable_addr = unavailable.local_addr().expect("unavailable address");
    drop(unavailable);
    let failure = CloudAccessClient::new(format!("http://{unavailable_addr}"))
        .store_key("openai", Zeroizing::new(CANARY.to_owned()))
        .await
        .expect_err("closed endpoint must produce a transport failure");
    assert!(
        !failure.to_string().contains(CANARY),
        "transport error surface must not echo key material: {failure}"
    );
}
