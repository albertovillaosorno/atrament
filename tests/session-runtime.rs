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
#[path = "../src/backend/session-runtime/adapter-inbound/lib.rs"]
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

const INDEX_HTML: &[u8] =
    include_bytes!("../src/browser/workspace/adapter-inbound/index.html");
const WORKSPACE_CSS: &[u8] =
    include_bytes!("../src/browser/workspace/adapter-inbound/workspace.css");
const MAIN_JAVASCRIPT: &[u8] = include_bytes!(
    "../src/browser/workspace/adapter-inbound/generated/main.js"
);
const SESSION_FRAGMENT_JAVASCRIPT: &[u8] = include_bytes!(
    "../src/browser/workspace/adapter-inbound/generated/session-fragment.js"
);

fn response_parts(response: &[u8]) -> (&str, &[u8]) {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("response has header terminator");
    let body_start = split + 4;
    let head = std::str::from_utf8(&response[..body_start])
        .expect("response head is UTF-8");
    (head, &response[body_start..])
}

fn request(target: &str, host: &str) -> Vec<u8> {
    let request = format!("GET {target} HTTP/1.1\r\nHost: {host}\r\n\r\n");
    runtime::route_request(request.as_bytes(), host)
}

#[test]
fn serves_embedded_frontend_resources_without_caching() {
    let host = "127.0.0.1:43123";
    let cases = [
        ("/", "text/html; charset=utf-8", INDEX_HTML),
        ("/index.html", "text/html; charset=utf-8", INDEX_HTML),
        ("/workspace.css", "text/css; charset=utf-8", WORKSPACE_CSS),
        (
            "/generated/main.js",
            "text/javascript; charset=utf-8",
            MAIN_JAVASCRIPT,
        ),
        (
            "/generated/session-fragment.js",
            "text/javascript; charset=utf-8",
            SESSION_FRAGMENT_JAVASCRIPT,
        ),
    ];

    for (target, content_type, expected_body) in cases {
        let response = request(target, host);
        let (head, body) = response_parts(&response);
        assert!(head.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(head.contains(&format!("Content-Type: {content_type}\r\n")));
        assert!(head.contains("Cache-Control: no-store\r\n"));
        assert!(head.contains("X-Content-Type-Options: nosniff\r\n"));
        assert_eq!(body, expected_body);
    }
}

#[test]
fn compiled_frontend_module_is_referenced_by_the_served_document() {
    let response = request("/", "127.0.0.1:43123");
    let (_, body) = response_parts(&response);
    let html = std::str::from_utf8(body).expect("HTML is UTF-8");
    assert!(html.contains("src=\"./generated/main.js\""));
    assert!(html.contains("href=\"./workspace.css\""));
}

#[test]
fn session_credential_requires_one_exact_bearer_value() {
    let secret = "a".repeat(64);
    let accepted = format!(
        "POST /api HTTP/1.1\r\nHost: 127.0.0.1:43123\r\n\
         Authorization: Bearer {secret}\r\n\r\n",
    );
    assert!(runtime::request_has_session_credential(
        accepted.as_bytes(),
        &secret,
    ));

    let rejected = [
        "POST /api HTTP/1.1\r\nHost: 127.0.0.1:43123\r\n\r\n".to_owned(),
        format!("POST /api HTTP/1.1\r\nAuthorization: {secret}\r\n\r\n",),
        format!(
            "POST /api HTTP/1.1\r\nAuthorization: Bearer {}\r\n\r\n",
            "b".repeat(64),
        ),
        format!(
            "POST /api HTTP/1.1\r\nAuthorization: Bearer {secret}\r\n\
             Authorization: Bearer {secret}\r\n\r\n",
        ),
    ];
    for request in rejected {
        assert!(!runtime::request_has_session_credential(
            request.as_bytes(),
            &secret,
        ));
    }
}

#[test]
fn browser_origin_requires_one_exact_canonical_value() {
    let origin = "http://127.0.0.1:43123";
    let accepted = format!("POST /api HTTP/1.1\r\nOrigin: {origin}\r\n\r\n",);
    assert!(runtime::request_has_exact_origin(
        accepted.as_bytes(),
        origin,
    ));

    let rejected = [
        "POST /api HTTP/1.1\r\n\r\n".to_owned(),
        "POST /api HTTP/1.1\r\nOrigin: http://localhost:43123\r\n\r\n"
            .to_owned(),
        format!(
            "POST /api HTTP/1.1\r\nOrigin: {origin}\r\n\
             Origin: {origin}\r\n\r\n",
        ),
    ];
    for request in rejected {
        assert!(!runtime::request_has_exact_origin(
            request.as_bytes(),
            origin,
        ));
    }
}
