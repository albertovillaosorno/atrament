// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
//
// Boundary-Contract:
// - Owns:
//   - Regression evidence for loopback runtime admission.
// - Must-Not:
//   - Provide production transport or session authority.
// - Allows:
//   - Inputs: Deterministic HTTP request bytes and loopback socket probes.
//   - Outputs: Assertions over public runtime behavior.
//   - Side effects: Ephemeral loopback listener binding during tests.
// - Split-When:
//   - Runtime transport tests require separate process-level fixtures.
// - Merge-When:
//   - The session runtime no longer has an independent transport boundary.
// - Summary:
//   - Verifies the initial localhost runtime admission contract.
// - Description:
//   - Covers port allocation, canonical Host admission, and public routing.
// - Usage:
//   - Compile this root test harness against the session runtime adapter.
// - Defaults:
//   - Uses ephemeral loopback ports and deterministic request fixtures.
//
use std::net::{Ipv4Addr, SocketAddr};

#[allow(dead_code)]
#[path = "../src/backend/session-runtime/adapter-inbound/main.rs"]
mod runtime;

fn status_line(response: &[u8]) -> &str {
    std::str::from_utf8(response)
        .expect("response is UTF-8")
        .split("\r\n")
        .next()
        .expect("response has status line")
}

#[test]
fn binds_ipv4_loopback_on_an_os_assigned_port() {
    let runtime = runtime::Runtime::bind().expect("runtime binds");
    let address = runtime.local_addr().expect("listener address");
    assert_eq!(address.ip(), Ipv4Addr::LOCALHOST);
    assert_ne!(address.port(), 0);
    assert_eq!(
        runtime.expected_host(),
        format!("127.0.0.1:{}", address.port()),
    );
    assert_eq!(
        runtime.origin(),
        format!("http://{}", runtime.expected_host()),
    );
}

#[test]
fn health_requires_the_exact_canonical_host() {
    let host = "127.0.0.1:43123";
    let accepted = runtime::route_request(
        b"GET /health HTTP/1.1\r\nHost: 127.0.0.1:43123\r\n\r\n",
        host,
    );
    assert_eq!(status_line(&accepted), "HTTP/1.1 200 OK");

    let rejected_hosts = [
        "localhost:43123",
        "0.0.0.0:43123",
        "example.test:43123",
        "127.0.0.1",
    ];
    for rejected_host in rejected_hosts {
        let request =
            format!("GET /health HTTP/1.1\r\nHost: {rejected_host}\r\n\r\n",);
        assert_eq!(
            status_line(&runtime::route_request(request.as_bytes(), host)),
            "HTTP/1.1 421 Misdirected Request",
        );
    }
}

#[test]
fn malformed_or_missing_host_is_rejected_before_routing() {
    let host = "127.0.0.1:43123";
    let rejected = [
        "GET /health HTTP/1.1\r\n\r\n",
        "POST /health HTTP/1.1\r\nHost: 127.0.0.1:43123\r\n\r\n",
        "GET /health HTTP/1.0\r\nHost: 127.0.0.1:43123\r\n\r\n",
        concat!(
            "GET /health HTTP/1.1\r\n",
            "Host: 127.0.0.1:43123\r\n",
            "Host: 127.0.0.1:43123\r\n\r\n",
        ),
    ];
    for request in rejected {
        assert_eq!(
            status_line(&runtime::route_request(request.as_bytes(), host)),
            "HTTP/1.1 400 Bad Request",
        );
    }
}

#[test]
fn unrelated_paths_do_not_expose_runtime_state() {
    let response = runtime::route_request(
        b"GET /session HTTP/1.1\r\nHost: 127.0.0.1:43123\r\n\r\n",
        "127.0.0.1:43123",
    );
    assert_eq!(status_line(&response), "HTTP/1.1 404 Not Found");
    assert!(
        !std::str::from_utf8(&response)
            .expect("response is UTF-8")
            .contains("secret"),
    );
}

#[test]
fn listener_address_type_is_ipv4_loopback() {
    let runtime = runtime::Runtime::bind().expect("runtime binds");
    assert!(matches!(
        runtime.local_addr().expect("listener address"),
        SocketAddr::V4(address) if *address.ip() == Ipv4Addr::LOCALHOST
    ));
}
