use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{Shutdown, TcpListener, TcpStream};

use crate::live_socket::ArgusClient;

const WITNESS_METHOD: &str = "argus.divider_witness";
const REPLY: &[u8] = br#"{"jsonrpc":"2.0","id":1,"result":{"status":"ready"}}"#;

fn private_reply_socket() -> (TcpListener, BufReader<TcpStream>, ArgusClient) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = ArgusClient::new(
        listener.local_addr().unwrap().to_string(),
        "fixture-token".to_owned(),
        "divider-framing".to_owned(),
    )
    .unwrap();
    let (peer, _) = listener.accept().expect("connected private client");
    let timeout = handshake_native::mcp::ACTION_RECEIPT_TRANSPORT_TIMEOUT;
    peer.set_read_timeout(Some(timeout)).unwrap();
    peer.set_write_timeout(Some(timeout)).unwrap();
    (listener, BufReader::new(peer), client)
}

fn assert_request(peer: &mut impl BufRead, method: &str) {
    let mut line = String::new();
    assert!(peer.read_line(&mut line).unwrap() > 0);
    let request: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(request["id"], 1);
    assert_eq!(request["method"], method);
    assert_eq!(request["client_session_id"], "divider-framing");
}

#[test]
fn divider_call_accepts_lf_while_peer_stays_open() {
    let (_listener, mut peer, mut client) = private_reply_socket();
    peer.get_mut().write_all(REPLY).unwrap();
    peer.get_mut().write_all(b"\n").unwrap();

    let reply = client
        .try_call_delimited(WITNESS_METHOD, serde_json::json!({}))
        .unwrap();
    assert_eq!(reply["result"]["status"], "ready");
    assert_request(&mut peer, WITNESS_METHOD);
}

#[test]
fn divider_call_rejects_valid_eof_and_never_replays_or_reconnects() {
    let (listener, mut peer, mut client) = private_reply_socket();
    peer.get_mut().write_all(REPLY).unwrap();
    peer.get_ref().shutdown(Shutdown::Write).unwrap();

    let first = client
        .try_call_delimited(WITNESS_METHOD, serde_json::json!({}))
        .unwrap_err();
    assert_eq!(first.kind(), ErrorKind::UnexpectedEof);

    let second = client
        .try_call_delimited(WITNESS_METHOD, serde_json::json!({}))
        .unwrap_err();
    assert_eq!(second.kind(), ErrorKind::NotConnected);
    assert_request(&mut peer, WITNESS_METHOD);
    let mut extra = String::new();
    assert_eq!(peer.read_line(&mut extra).unwrap(), 0);
    listener.set_nonblocking(true).unwrap();
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
}

#[test]
fn ordinary_call_accepts_valid_json_at_eof() {
    let (_listener, mut peer, mut client) = private_reply_socket();
    peer.get_mut().write_all(REPLY).unwrap();
    peer.get_ref().shutdown(Shutdown::Write).unwrap();

    let reply = client
        .try_call("argus.inspect", serde_json::json!({}))
        .unwrap();
    assert_eq!(reply["result"]["status"], "ready");
    assert_request(&mut peer, "argus.inspect");
}
