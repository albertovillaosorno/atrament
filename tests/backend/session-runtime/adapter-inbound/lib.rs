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

use atrament_diagnostic::{Completeness, DIAGNOSTIC_VERSION, DiagnosticSet};
use atrament_session_draft::{MAX_DRAFT_FIELD_BYTES, SessionDraftService};
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};
use atrament_session_handshake::{
    CAPABILITY_VERSION, HandshakeService, PRODUCT_VERSION, PROFILE_VERSION,
    PROMPT_VERSION, PROTOCOL_VERSION, RENDERER_VERSION,
};
use atrament_session_handshake_port::{
    HandshakeResult, SessionHandshake, VersionDimension, Versions,
};

const EXPECTED_HOST: &str = "127.0.0.1:43123";
const EXPECTED_ORIGIN: &str = "http://127.0.0.1:43123";
const EXPECTED_SECRET: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

static HANDSHAKE: HandshakeService = HandshakeService;

#[allow(dead_code)]
#[path = "../../../../src/backend/session-runtime/adapter-inbound/lib.rs"]
mod runtime;

fn route_with_draft(
    request: &[u8],
    host: &str,
    draft: &mut dyn SessionDraft,
) -> Vec<u8> {
    runtime::route_request(
        request,
        host,
        EXPECTED_ORIGIN,
        EXPECTED_SECRET,
        &HANDSHAKE,
        draft,
    )
}

fn route_runtime(request: &[u8], host: &str) -> Vec<u8> {
    let mut draft = SessionDraftService::default();
    route_with_draft(request, host, &mut draft)
}

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
    let accepted = route_runtime(
        b"GET /health HTTP/1.1\r\nHost: 127.0.0.1:43123\r\n\r\n",
        host,
    );
    assert_eq!(status_line(&accepted), "HTTP/1.1 200 OK");
    let (_, health_body) = response_parts(&accepted);
    assert_eq!(health_body, br#"{"product":"atrament","state":"ready"}"#,);

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
            status_line(&route_runtime(request.as_bytes(), host)),
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
            status_line(&route_runtime(request.as_bytes(), host)),
            "HTTP/1.1 400 Bad Request",
        );
    }
}

#[test]
fn unrelated_paths_do_not_expose_runtime_state() {
    let response = route_runtime(
        b"GET /session HTTP/1.1\r\nHost: 127.0.0.1:43123\r\n\r\n",
        EXPECTED_HOST,
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

const INDEX_HTML: &[u8] = include_bytes!(concat!(
    "../../../../src/browser/workspace/adapter-inbound/",
    "index.html"
));
const WORKSPACE_CSS: &[u8] = include_bytes!(concat!(
    "../../../../src/browser/workspace/adapter-inbound/",
    "workspace.css"
));
const MAIN_JAVASCRIPT: &[u8] = include_bytes!(concat!(
    "../../../../src/browser/workspace/adapter-inbound/",
    "generated/main.js"
));
const SESSION_DIAGNOSTIC_JAVASCRIPT: &[u8] = include_bytes!(concat!(
    "../../../../src/browser/workspace/adapter-inbound/",
    "generated/session-diagnostic.js"
));
const SESSION_DRAFT_JAVASCRIPT: &[u8] = include_bytes!(concat!(
    "../../../../src/browser/workspace/adapter-inbound/",
    "generated/session-draft.js"
));
const SESSION_FRAGMENT_JAVASCRIPT: &[u8] = include_bytes!(concat!(
    "../../../../src/browser/workspace/adapter-inbound/",
    "generated/session-fragment.js"
));
const SESSION_HANDSHAKE_JAVASCRIPT: &[u8] = include_bytes!(concat!(
    "../../../../src/browser/workspace/adapter-inbound/",
    "generated/session-handshake.js"
));

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
    route_runtime(request.as_bytes(), host)
}

fn handshake_request(
    authorization: Option<&str>,
    origin: Option<&str>,
    prompt_version: &str,
) -> Vec<u8> {
    let mut request = String::from("POST /api/handshake HTTP/1.1\r\n");
    request.push_str(&format!("Host: {}\r\n", EXPECTED_HOST));
    if let Some(value) = authorization {
        request.push_str(&format!("Authorization: {value}\r\n"));
    }
    if let Some(value) = origin {
        request.push_str(&format!("Origin: {value}\r\n"));
    }
    request.push_str(&format!(
        "X-Atrament-Capability-Version: {}\r\n",
        CAPABILITY_VERSION,
    ));
    request.push_str(&format!(
        "X-Atrament-Product-Version: {}\r\n",
        PRODUCT_VERSION,
    ));
    request.push_str(&format!(
        "X-Atrament-Profile-Version: {}\r\n",
        PROFILE_VERSION,
    ));
    request
        .push_str(&format!("X-Atrament-Prompt-Version: {prompt_version}\r\n",));
    request.push_str(&format!(
        "X-Atrament-Protocol-Version: {}\r\n",
        PROTOCOL_VERSION,
    ));
    request.push_str(&format!(
        "X-Atrament-Renderer-Version: {}\r\n\r\n",
        RENDERER_VERSION,
    ));
    route_runtime(request.as_bytes(), EXPECTED_HOST)
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
            "/generated/session-diagnostic.js",
            "text/javascript; charset=utf-8",
            SESSION_DIAGNOSTIC_JAVASCRIPT,
        ),
        (
            "/generated/session-draft.js",
            "text/javascript; charset=utf-8",
            SESSION_DRAFT_JAVASCRIPT,
        ),
        (
            "/generated/session-fragment.js",
            "text/javascript; charset=utf-8",
            SESSION_FRAGMENT_JAVASCRIPT,
        ),
        (
            "/generated/session-handshake.js",
            "text/javascript; charset=utf-8",
            SESSION_HANDSHAKE_JAVASCRIPT,
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

    for index in 0..secret.len() {
        let mut candidate = secret.clone().into_bytes();
        candidate[index] = b'b';
        let candidate = String::from_utf8(candidate)
            .expect("credential fixture remains ASCII");
        let request = format!(
            "POST /api HTTP/1.1\r\nAuthorization: Bearer {candidate}\r\n\r\n",
        );
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

#[test]
fn authenticated_handshake_returns_current_version_set() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let response = handshake_request(
        Some(&authorization),
        Some(EXPECTED_ORIGIN),
        PROMPT_VERSION,
    );
    let (head, body) = response_parts(&response);
    assert!(head.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(!head.contains("Access-Control-Allow-Origin"));
    let body = std::str::from_utf8(body).expect("handshake JSON is UTF-8");
    assert!(body.contains("\"result\":\"compatible\""));
    for version in [
        CAPABILITY_VERSION,
        PRODUCT_VERSION,
        PROFILE_VERSION,
        PROMPT_VERSION,
        PROTOCOL_VERSION,
        RENDERER_VERSION,
    ] {
        assert!(body.contains(version));
    }
}

#[test]
fn handshake_admission_failures_share_one_unauthenticated_response() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let wrong_authorization = format!("Bearer {}", "b".repeat(64));
    let responses = [
        handshake_request(None, Some(EXPECTED_ORIGIN), PROMPT_VERSION),
        handshake_request(
            Some(&wrong_authorization),
            Some(EXPECTED_ORIGIN),
            PROMPT_VERSION,
        ),
        handshake_request(Some(&authorization), None, PROMPT_VERSION),
        handshake_request(
            Some(&authorization),
            Some("http://localhost:43123"),
            PROMPT_VERSION,
        ),
    ];
    for response in &responses {
        assert_eq!(status_line(response), "HTTP/1.1 401 Unauthorized");
        assert_eq!(response, &responses[0]);
    }
}

#[test]
fn authenticated_version_mismatch_is_typed_and_blocking() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let response = handshake_request(
        Some(&authorization),
        Some(EXPECTED_ORIGIN),
        "atrament.prompt/0",
    );
    let (head, body) = response_parts(&response);
    assert!(head.starts_with("HTTP/1.1 409 Conflict\r\n"));
    let body = std::str::from_utf8(body).expect("handshake JSON is UTF-8");
    assert!(body.contains("atrament.handshake.version-mismatch"));
    assert!(body.contains(DIAGNOSTIC_VERSION));
    assert!(body.contains("\"dimension\":\"prompt\""));
    assert!(body.contains(PROMPT_VERSION));
    assert!(!body.contains("atrament.prompt/0"));
}

#[test]
fn handshake_requires_post_method() {
    let request = format!(
        "GET /api/handshake HTTP/1.1\r\nHost: {EXPECTED_HOST}\r\n\r\n",
    );
    let response = route_runtime(request.as_bytes(), EXPECTED_HOST);
    assert_eq!(status_line(&response), "HTTP/1.1 400 Bad Request");
}

#[test]
fn missing_required_handshake_version_blocks_compatibility() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let request = format!(
        concat!(
            "POST /api/handshake HTTP/1.1\r\n",
            "Host: {}\r\n",
            "Authorization: {}\r\n",
            "Origin: {}\r\n",
            "X-Atrament-Capability-Version: {}\r\n",
            "X-Atrament-Product-Version: {}\r\n",
            "X-Atrament-Profile-Version: {}\r\n",
            "X-Atrament-Protocol-Version: {}\r\n",
            "X-Atrament-Renderer-Version: {}\r\n\r\n",
        ),
        EXPECTED_HOST,
        authorization,
        EXPECTED_ORIGIN,
        CAPABILITY_VERSION,
        PRODUCT_VERSION,
        PROFILE_VERSION,
        PROTOCOL_VERSION,
        RENDERER_VERSION,
    );
    let response = route_runtime(request.as_bytes(), EXPECTED_HOST);
    let (head, body) = response_parts(&response);
    assert!(head.starts_with("HTTP/1.1 409 Conflict\r\n"));
    let body = std::str::from_utf8(body).expect("handshake JSON is UTF-8");
    assert!(body.contains("\"dimension\":\"prompt\""));
}

#[test]
fn duplicate_required_handshake_version_blocks_compatibility() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let request = format!(
        concat!(
            "POST /api/handshake HTTP/1.1\r\n",
            "Host: {}\r\n",
            "Authorization: {}\r\n",
            "Origin: {}\r\n",
            "X-Atrament-Capability-Version: {}\r\n",
            "X-Atrament-Product-Version: {}\r\n",
            "X-Atrament-Profile-Version: {}\r\n",
            "X-Atrament-Prompt-Version: {}\r\n",
            "X-Atrament-Prompt-Version: atrament.prompt/0\r\n",
            "X-Atrament-Protocol-Version: {}\r\n",
            "X-Atrament-Renderer-Version: {}\r\n\r\n",
        ),
        EXPECTED_HOST,
        authorization,
        EXPECTED_ORIGIN,
        CAPABILITY_VERSION,
        PRODUCT_VERSION,
        PROFILE_VERSION,
        PROMPT_VERSION,
        PROTOCOL_VERSION,
        RENDERER_VERSION,
    );
    let response = route_runtime(request.as_bytes(), EXPECTED_HOST);
    let (head, body) = response_parts(&response);
    assert!(head.starts_with("HTTP/1.1 409 Conflict\r\n"));
    let body = std::str::from_utf8(body).expect("handshake JSON is UTF-8");
    assert!(body.contains("\"dimension\":\"prompt\""));
    assert!(!body.contains("atrament.prompt/0"));
}

fn draft_replace_request(
    target: &str,
    authorization: Option<&str>,
    origin: Option<&str>,
    body: &[u8],
) -> Vec<u8> {
    let mut request =
        format!("POST {target} HTTP/1.1\r\nHost: {EXPECTED_HOST}\r\n",);
    if let Some(value) = authorization {
        request.push_str(&format!("Authorization: {value}\r\n"));
    }
    if let Some(value) = origin {
        request.push_str(&format!("Origin: {value}\r\n"));
    }
    request.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    let mut bytes = request.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

#[test]
fn authenticated_draft_mutations_replace_only_requested_field() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let cases = [
        ("/api/session/task", DraftField::Task, "format these notes"),
        ("/api/session/source", DraftField::Source, "área = πr²"),
        (
            "/api/session/candidate",
            DraftField::Candidate,
            "untrusted model response",
        ),
    ];
    let mut draft = SessionDraftService::default();
    for (target, field, value) in cases {
        let request = draft_replace_request(
            target,
            Some(&authorization),
            Some(EXPECTED_ORIGIN),
            value.as_bytes(),
        );
        let response = route_with_draft(&request, EXPECTED_HOST, &mut draft);
        let (head, body) = response_parts(&response);
        assert!(head.starts_with("HTTP/1.1 204 No Content\r\n"));
        assert!(!head.contains("Access-Control-Allow-Origin"));
        assert!(body.is_empty());
        assert_eq!(draft.value(field), value);
    }
}

#[test]
fn browser_forgery_cannot_mutate_session_draft_state() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let wrong_authorization = format!("Bearer {}", "b".repeat(64));
    let cases = [
        (None, Some(EXPECTED_ORIGIN)),
        (Some(wrong_authorization.as_str()), Some(EXPECTED_ORIGIN)),
        (Some(authorization.as_str()), None),
        (
            Some(authorization.as_str()),
            Some("http://attacker.example"),
        ),
        (Some(authorization.as_str()), Some("http://localhost:43123")),
    ];
    let mut draft = SessionDraftService::default();
    assert_eq!(
        draft.replace(DraftField::Task, String::from("trusted current")),
        atrament_session_draft_port::DraftMutation::Applied,
    );
    let mut reference_response = None;
    for (credential, origin) in cases {
        let request = draft_replace_request(
            "/api/session/task",
            credential,
            origin,
            b"attacker replacement",
        );
        let response = route_with_draft(&request, EXPECTED_HOST, &mut draft);
        assert_eq!(status_line(&response), "HTTP/1.1 401 Unauthorized");
        assert_eq!(draft.value(DraftField::Task), "trusted current");
        if let Some(reference) = &reference_response {
            assert_eq!(&response, reference);
        } else {
            reference_response = Some(response);
        }
    }
}

#[test]
fn malformed_draft_body_framing_never_mutates_state() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let prefix = format!(
        concat!(
            "POST /api/session/source HTTP/1.1\r\n",
            "Host: {}\r\n",
            "Authorization: {}\r\n",
            "Origin: {}\r\n",
        ),
        EXPECTED_HOST, authorization, EXPECTED_ORIGIN,
    );
    let malformed = [
        format!("{prefix}\r\nbody").into_bytes(),
        format!("{prefix}Content-Length: 3\r\n\r\nbody").into_bytes(),
        format!("{prefix}Content-Length: 4\r\nContent-Length: 4\r\n\r\nbody",)
            .into_bytes(),
        format!(
            "{prefix}Transfer-Encoding: chunked\r\n\r\n4\r\nbody\r\n0\r\n\r\n",
        )
        .into_bytes(),
    ];
    let mut draft = SessionDraftService::default();
    for request in malformed {
        let response = route_with_draft(&request, EXPECTED_HOST, &mut draft);
        assert_eq!(status_line(&response), "HTTP/1.1 400 Bad Request");
        assert_eq!(draft.value(DraftField::Source), "");
    }

    let invalid_utf8 = draft_replace_request(
        "/api/session/source",
        Some(&authorization),
        Some(EXPECTED_ORIGIN),
        &[0xff, 0xfe],
    );
    let response = route_with_draft(&invalid_utf8, EXPECTED_HOST, &mut draft);
    assert_eq!(status_line(&response), "HTTP/1.1 400 Bad Request");
    assert_eq!(draft.value(DraftField::Source), "");
}

#[test]
fn draft_resource_limit_rejects_without_truncating_current_value() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let mut draft = SessionDraftService::default();
    assert_eq!(
        draft.replace(DraftField::Candidate, String::from("current")),
        atrament_session_draft_port::DraftMutation::Applied,
    );
    let body = vec![b'a'; MAX_DRAFT_FIELD_BYTES + 1];
    let request = draft_replace_request(
        "/api/session/candidate",
        Some(&authorization),
        Some(EXPECTED_ORIGIN),
        &body,
    );
    let response = route_with_draft(&request, EXPECTED_HOST, &mut draft);
    assert_eq!(status_line(&response), "HTTP/1.1 413 Content Too Large");
    let (_, response_body) = response_parts(&response);
    let response_body =
        std::str::from_utf8(response_body).expect("resource JSON is UTF-8");
    assert!(response_body.contains(DIAGNOSTIC_VERSION));
    assert!(response_body.contains("atrament.session-draft.resource-limit"));
    assert_eq!(draft.value(DraftField::Candidate), "current");
}

fn draft_read_request(
    target: &str,
    authorization: Option<&str>,
    origin: Option<&str>,
) -> Vec<u8> {
    let mut request =
        format!("GET {target} HTTP/1.1\r\nHost: {EXPECTED_HOST}\r\n",);
    if let Some(value) = authorization {
        request.push_str(&format!("Authorization: {value}\r\n"));
    }
    if let Some(value) = origin {
        request.push_str(&format!("Origin: {value}\r\n"));
    }
    request.push_str("\r\n");
    request.into_bytes()
}

#[test]
fn authenticated_draft_reads_return_exact_private_text() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let mut draft = SessionDraftService::default();
    let cases = [
        ("/api/session/task", DraftField::Task, "task α"),
        ("/api/session/source", DraftField::Source, "fuente ñ"),
        (
            "/api/session/candidate",
            DraftField::Candidate,
            "respuesta π",
        ),
    ];
    for (_, field, value) in cases {
        assert_eq!(
            draft.replace(field, String::from(value)),
            atrament_session_draft_port::DraftMutation::Applied,
        );
    }
    for (target, _, expected) in cases {
        for origin in [None, Some(EXPECTED_ORIGIN)] {
            let request =
                draft_read_request(target, Some(&authorization), origin);
            let response =
                route_with_draft(&request, EXPECTED_HOST, &mut draft);
            let (head, body) = response_parts(&response);
            assert!(head.starts_with("HTTP/1.1 200 OK\r\n"));
            assert!(head.contains("Content-Type: text/plain; charset=utf-8"));
            assert!(head.contains("Cache-Control: no-store"));
            assert!(!head.contains("Access-Control-Allow-Origin"));
            assert_eq!(body, expected.as_bytes());
        }
    }
}

#[test]
fn draft_read_admission_failure_is_uniform_and_private() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let wrong_authorization = format!("Bearer {}", "b".repeat(64));
    let mut draft = SessionDraftService::default();
    assert_eq!(
        draft.replace(DraftField::Task, String::from("private task")),
        atrament_session_draft_port::DraftMutation::Applied,
    );
    let requests = [
        draft_read_request("/api/session/task", None, None),
        draft_read_request(
            "/api/session/task",
            Some(&wrong_authorization),
            None,
        ),
        draft_read_request(
            "/api/session/task",
            Some(&authorization),
            Some("http://attacker.example"),
        ),
        {
            let mut request = format!(
                concat!(
                    "GET /api/session/task HTTP/1.1\r\n",
                    "Host: {}\r\n",
                    "Authorization: {}\r\n",
                    "Origin: {}\r\n",
                    "Origin: http://attacker.example\r\n\r\n",
                ),
                EXPECTED_HOST, authorization, EXPECTED_ORIGIN,
            );
            request.shrink_to_fit();
            request.into_bytes()
        },
    ];
    let reference = route_with_draft(&requests[0], EXPECTED_HOST, &mut draft);
    assert_eq!(status_line(&reference), "HTTP/1.1 401 Unauthorized");
    assert!(!String::from_utf8_lossy(&reference).contains("private task"));
    for request in requests.iter().skip(1) {
        let response = route_with_draft(request, EXPECTED_HOST, &mut draft);
        assert_eq!(response, reference);
    }
}

struct EmptyDiagnosticDraft;

impl SessionDraft for EmptyDiagnosticDraft {
    fn replace(&mut self, _field: DraftField, _value: String) -> DraftMutation {
        DraftMutation::ResourceLimit {
            diagnostics: DiagnosticSet {
                completeness: Completeness::Complete,
                diagnostics: vec![],
            },
        }
    }

    fn value(&self, _field: DraftField) -> &str {
        ""
    }
}

struct EmptyDiagnosticHandshake;

impl SessionHandshake for EmptyDiagnosticHandshake {
    fn evaluate<'version>(
        &self,
        versions: Versions<'version>,
    ) -> HandshakeResult<'version> {
        HandshakeResult::Incompatible {
            diagnostics: DiagnosticSet {
                completeness: Completeness::Complete,
                diagnostics: vec![],
            },
            dimension: VersionDimension::Prompt,
            expected: PROMPT_VERSION,
            observed: versions.prompt,
        }
    }
}

#[test]
fn adapter_never_invents_a_missing_application_diagnostic() {
    let authorization = format!("Bearer {EXPECTED_SECRET}");
    let handshake_request = format!(
        concat!(
            "POST /api/handshake HTTP/1.1\r\n",
            "Host: {}\r\nAuthorization: {}\r\nOrigin: {}\r\n",
            "X-Atrament-Capability-Version: {}\r\n",
            "X-Atrament-Product-Version: {}\r\n",
            "X-Atrament-Profile-Version: {}\r\n",
            "X-Atrament-Prompt-Version: {}\r\n",
            "X-Atrament-Protocol-Version: {}\r\n",
            "X-Atrament-Renderer-Version: {}\r\n\r\n",
        ),
        EXPECTED_HOST,
        authorization,
        EXPECTED_ORIGIN,
        CAPABILITY_VERSION,
        PRODUCT_VERSION,
        PROFILE_VERSION,
        PROMPT_VERSION,
        PROTOCOL_VERSION,
        RENDERER_VERSION,
    );
    let mut ordinary_draft = SessionDraftService::default();
    let response = runtime::route_request(
        handshake_request.as_bytes(),
        EXPECTED_HOST,
        EXPECTED_ORIGIN,
        EXPECTED_SECRET,
        &EmptyDiagnosticHandshake,
        &mut ordinary_draft,
    );
    let response_text = String::from_utf8(response).expect("response is UTF-8");
    assert!(
        response_text.starts_with("HTTP/1.1 500 Internal Server Error\r\n")
    );
    assert!(!response_text.contains("atrament.handshake.version-mismatch"));

    let request = draft_replace_request(
        "/api/session/task",
        Some(&authorization),
        Some(EXPECTED_ORIGIN),
        b"ordinary text",
    );
    let mut empty_diagnostic_draft = EmptyDiagnosticDraft;
    let response = runtime::route_request(
        &request,
        EXPECTED_HOST,
        EXPECTED_ORIGIN,
        EXPECTED_SECRET,
        &HANDSHAKE,
        &mut empty_diagnostic_draft,
    );
    let response_text = String::from_utf8(response).expect("response is UTF-8");
    assert!(
        response_text.starts_with("HTTP/1.1 500 Internal Server Error\r\n")
    );
    assert!(!response_text.contains("atrament.session-draft.resource-limit"));
}
