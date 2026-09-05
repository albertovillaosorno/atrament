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
//   - Loopback listener, public resources, and credential header admission.
// - Must-Not:
//   - Bind non-loopback addresses or own session-private application state.
// - Allows:
//   - Inputs: Local HTTP requests and in-memory expected session credentials.
//   - Outputs: Public resources, handshake results, and draft mutation status.
//   - Side effects: Loopback binding and HTTP response writes.
// - Split-When:
//   - Authenticated routing needs an independently testable adapter.
// - Merge-When:
//   - Runtime admission no longer has independent transport ownership.
// - Summary:
//   - Starts the disposable Atrament localhost session runtime.
// - Description:
//   - Enforces loopback and HTTP admission before invoking application
//     services.
// - Usage:
//   - Run the atrament binary to serve the browser shell on one loopback
//     endpoint.
// - Defaults:
//   - Binds 127.0.0.1 on an operating-system assigned port.
//

//! Disposable loopback transport for the Atrament browser session runtime.
//!
//! This adapter owns listener admission, public frontend resources, health
//! routing, authenticated handshake transport, and protected draft mutation
//! transport without owning the application state those routes mutate.

use std::io::{self, Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::str;
use std::time::Duration;

use atrament_diagnostic::{Completeness, DIAGNOSTIC_VERSION, DiagnosticSet};
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};
use atrament_session_handshake_port::{
    HandshakeResult, SessionHandshake, VersionDimension, Versions,
};

const ENCODED_SECRET_BYTES: usize = 64;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;
const REQUEST_IO_TIMEOUT: Duration = Duration::from_secs(2);
const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";
const CSS_CONTENT_TYPE: &str = "text/css; charset=utf-8";
const JAVASCRIPT_CONTENT_TYPE: &str = "text/javascript; charset=utf-8";
const TEXT_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
const JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";
const RESPONSE_TRAILERS: &str = concat!(
    "Cache-Control: no-store\r\n",
    "Content-Security-Policy: frame-ancestors 'none'\r\n",
    "Referrer-Policy: no-referrer\r\n",
    "X-Content-Type-Options: nosniff\r\n",
    "Connection: close\r\n\r\n",
);
const INDEX_HTML: &[u8] =
    include_bytes!("../../../browser/workspace/adapter-inbound/index.html");
const WORKSPACE_CSS: &[u8] =
    include_bytes!("../../../browser/workspace/adapter-inbound/workspace.css");
const MAIN_JAVASCRIPT: &[u8] = include_bytes!(
    "../../../browser/workspace/adapter-inbound/generated/main.js"
);
const SESSION_DIAGNOSTIC_JAVASCRIPT: &[u8] = include_bytes!(
    "../../../browser/workspace/adapter-inbound/generated/session-diagnostic.js"
);
const SESSION_DRAFT_JAVASCRIPT: &[u8] = include_bytes!(
    "../../../browser/workspace/adapter-inbound/generated/session-draft.js"
);
const SESSION_FRAGMENT_JAVASCRIPT: &[u8] = include_bytes!(
    "../../../browser/workspace/adapter-inbound/generated/session-fragment.js"
);
const SESSION_HANDSHAKE_JAVASCRIPT: &[u8] = include_bytes!(
    "../../../browser/workspace/adapter-inbound/generated/session-handshake.js"
);

/// A listener bound to one operating-system-assigned IPv4 loopback endpoint.
#[derive(Debug)]
pub struct Runtime {
    expected_host: String,
    listener: TcpListener,
    origin: String,
}

impl Runtime {
    /// Bind a new runtime to `127.0.0.1` on an operating-system-assigned port.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when loopback binding or address inspection fails.
    pub fn bind() -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let address = listener.local_addr()?;
        if address.ip() != Ipv4Addr::LOCALHOST {
            return Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "runtime listener is not bound to IPv4 loopback",
            ));
        }
        let expected_host = format!("127.0.0.1:{}", address.port());
        let origin = format!("http://{expected_host}");
        Ok(Self {
            expected_host,
            listener,
            origin,
        })
    }

    /// Return the exact canonical `Host` value admitted by this runtime.
    #[must_use]
    pub fn expected_host(&self) -> &str {
        &self.expected_host
    }

    /// Return the socket address assigned to this runtime.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the listener address cannot be inspected.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Return the canonical HTTP origin published for this runtime.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Serve admitted HTTP requests until listener acceptance stops.
    pub fn serve(
        self,
        expected_secret: &str,
        handshake: &dyn SessionHandshake,
        draft: &mut dyn SessionDraft,
    ) {
        for incoming in self.listener.incoming() {
            match incoming {
                Ok(mut connection) => {
                    drop(connection.set_read_timeout(Some(REQUEST_IO_TIMEOUT)));
                    drop(
                        connection.set_write_timeout(Some(REQUEST_IO_TIMEOUT)),
                    );
                    drop(serve_connection(
                        &mut connection,
                        &self.expected_host,
                        &self.origin,
                        expected_secret,
                        handshake,
                        draft,
                    ));
                },
                Err(_) => break,
            }
        }
    }
}

fn request_head_end(request: &[u8]) -> Option<usize> {
    request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .and_then(|index| index.checked_add(4))
}

fn is_http_field_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*'
                        | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|'
                        | b'~'
                )
        })
}

fn is_http_field_value(value: &str) -> bool {
    value.bytes().all(|byte| {
        byte == b'\t' || (byte >= b' ' && byte != 0x7f)
    })
}

fn split_header_line(line: &str) -> Option<(&str, &str)> {
    let (name, value) = line.split_once(':')?;
    (is_http_field_name(name) && is_http_field_value(value))
        .then_some((name, value))
}

fn header_is_present(request_head: &[u8], expected_name: &str) -> bool {
    let Ok(text) = str::from_utf8(request_head) else {
        return false;
    };
    text.split("\r\n")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .any(|line| {
            split_header_line(line).is_some_and(|(name, _)| {
                name.eq_ignore_ascii_case(expected_name)
            })
        })
}

fn declared_content_length(request_head: &[u8]) -> io::Result<usize> {
    if header_is_present(request_head, "transfer-encoding") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "request transfer encoding is not admitted",
        ));
    }
    if !header_is_present(request_head, "content-length") {
        return Ok(0);
    }
    let Some(value) = single_header_value(request_head, "content-length")
    else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "request content length is malformed or duplicated",
        ));
    };
    parse_content_length_value(value)
}

fn parse_content_length_value(value: &str) -> io::Result<usize> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "request content length is not an ASCII decimal byte count",
        ));
    }
    let length = value.parse::<usize>().map_err(|_parse_error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "request content length is not representable",
        )
    })?;
    if length > MAX_REQUEST_BODY_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "request body exceeds runtime transport limit",
        ));
    }
    Ok(length)
}

fn request_head_line_endings_are_valid_so_far(bytes: &[u8]) -> bool {
    let mut index = 0usize;
    while let Some(byte) = bytes.get(index).copied() {
        match byte {
            b'\r' => {
                let Some(next) = bytes
                    .get(index.saturating_add(1))
                    .copied()
                else {
                    return true;
                };
                if next != b'\n' {
                    return false;
                }
                index = index.saturating_add(2);
            },
            b'\n' => return false,
            _ => index = index.saturating_add(1),
        }
    }
    true
}

pub(crate) fn read_request(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let mut expected_total = None;
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            if expected_total.is_some_and(|total| bytes.len() < total) {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "request body ended before declared content length",
                ));
            }
            if expected_total.is_none() && !bytes.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "request head ended before header terminator",
                ));
            }
            return Ok(bytes);
        }
        let Some(read_bytes) = chunk.get(..read) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "socket read exceeded the receive buffer",
            ));
        };
        bytes.extend_from_slice(read_bytes);
        if expected_total.is_none() {
            if let Some(head_end) = request_head_end(&bytes) {
                if head_end > MAX_HEADER_BYTES {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "request headers exceed runtime limit",
                    ));
                }
                let Some(request_head) = bytes.get(..head_end) else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "request header boundary is invalid",
                    ));
                };
                if !request_head_line_endings_are_valid_so_far(request_head) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "request headers require CRLF line endings",
                    ));
                }
                let content_length = declared_content_length(request_head)?;
                expected_total = Some(
                    head_end.checked_add(content_length).ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "request byte length overflowed",
                        )
                    })?,
                );
            } else {
                if !request_head_line_endings_are_valid_so_far(&bytes) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "request headers require CRLF line endings",
                    ));
                }
                if bytes.len() > MAX_HEADER_BYTES {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "request headers exceed runtime limit",
                    ));
                }
            }
        }
        if let Some(total) = expected_total {
            if bytes.len() > total {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "request exceeds its declared content length",
                ));
            }
            if bytes.len() == total {
                return Ok(bytes);
            }
        }
    }
}


fn trim_http_ows(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t'))
}

fn single_header_value<'request>(
    request: &'request [u8],
    expected_name: &str,
) -> Option<&'request str> {
    let text = str::from_utf8(request).ok()?;
    let mut matched_value = None;
    for line in text.split("\r\n").skip(1) {
        if line.is_empty() {
            break;
        }
        let (name, value) = split_header_line(line)?;
        if name.eq_ignore_ascii_case(expected_name) {
            if matched_value.is_some() {
                return None;
            }
            matched_value = Some(trim_http_ows(value));
        }
    }
    matched_value
}

fn authorization_bearer(request: &[u8]) -> Option<&str> {
    single_header_value(request, "authorization")?.strip_prefix("Bearer ")
}

fn fixed_work_secret_match(expected: &str, candidate: &str) -> bool {
    let expected_bytes = expected.as_bytes();
    let candidate_bytes = candidate.as_bytes();
    let valid_length = expected_bytes.len() == ENCODED_SECRET_BYTES
        && candidate_bytes.len() == ENCODED_SECRET_BYTES;
    let mut difference = 0u8;
    for index in 0..ENCODED_SECRET_BYTES {
        let expected_byte = expected_bytes.get(index).copied().unwrap_or(0);
        let candidate_byte = candidate_bytes.get(index).copied().unwrap_or(0);
        difference |= expected_byte ^ candidate_byte;
    }
    valid_length && difference == 0
}

/// Check browser origin metadata against the exact canonical startup origin.
#[must_use]
pub fn request_has_exact_origin(request: &[u8], expected_origin: &str) -> bool {
    single_header_value(request, "origin") == Some(expected_origin)
}

/// Check one Bearer credential without data-dependent comparison exit.
#[must_use]
pub fn request_has_session_credential(
    request: &[u8],
    expected_secret: &str,
) -> bool {
    let candidate = authorization_bearer(request).unwrap_or("");
    fixed_work_secret_match(expected_secret, candidate)
}

fn handshake_versions(request: &[u8]) -> Versions<'_> {
    Versions {
        capability: single_header_value(
            request,
            "x-atrament-capability-version",
        )
        .unwrap_or(""),
        product: single_header_value(request, "x-atrament-product-version")
            .unwrap_or(""),
        profile: single_header_value(request, "x-atrament-profile-version")
            .unwrap_or(""),
        prompt: single_header_value(request, "x-atrament-prompt-version")
            .unwrap_or(""),
        protocol: single_header_value(request, "x-atrament-protocol-version")
            .unwrap_or(""),
        renderer: single_header_value(request, "x-atrament-renderer-version")
            .unwrap_or(""),
    }
}

const fn handshake_dimension_name(dimension: VersionDimension) -> &'static str {
    match dimension {
        VersionDimension::Capability => "capability",
        VersionDimension::Product => "product",
        VersionDimension::Profile => "profile",
        VersionDimension::Prompt => "prompt",
        VersionDimension::Protocol => "protocol",
        VersionDimension::Renderer => "renderer",
    }
}

const fn completeness_name(completeness: Completeness) -> &'static str {
    match completeness {
        Completeness::Complete => "complete",
        Completeness::Incomplete => "incomplete",
    }
}

fn invalid_diagnostic_response() -> Vec<u8> {
    json_response(
        "500 Internal Server Error",
        br#"{"error":"invalid_diagnostic"}"#,
    )
}

fn handshake_incompatible_response(
    diagnostics: &DiagnosticSet,
    dimension: VersionDimension,
    expected: &str,
) -> Vec<u8> {
    let Some(diagnostic) = diagnostics.diagnostics.first() else {
        return invalid_diagnostic_response();
    };
    let body = format!(
        concat!(
            "{{\"result\":\"incompatible\",",
            "\"diagnostics\":{{",
            "\"version\":\"{}\",",
            "\"completeness\":\"{}\",",
            "\"items\":[{{",
            "\"code\":\"{}\",",
            "\"dimension\":\"{}\",",
            "\"expected\":\"{}\"}}]}}}}",
        ),
        DIAGNOSTIC_VERSION,
        completeness_name(diagnostics.completeness),
        diagnostic.code.stable_name(),
        handshake_dimension_name(dimension),
        expected,
    );
    response("409 Conflict", JSON_CONTENT_TYPE, body.as_bytes())
}

fn handshake_success_response(versions: Versions<'_>) -> Vec<u8> {
    let body = format!(
        concat!(
            "{{\"result\":\"compatible\",\"versions\":{{",
            "\"capability\":\"{}\",",
            "\"product\":\"{}\",",
            "\"profile\":\"{}\",",
            "\"prompt\":\"{}\",",
            "\"protocol\":\"{}\",",
            "\"renderer\":\"{}\"}}}}",
        ),
        versions.capability,
        versions.product,
        versions.profile,
        versions.prompt,
        versions.protocol,
        versions.renderer,
    );
    response("200 OK", JSON_CONTENT_TYPE, body.as_bytes())
}

fn route_handshake(
    request: &[u8],
    expected_origin: &str,
    expected_secret: &str,
    handshake: &dyn SessionHandshake,
) -> Vec<u8> {
    let credential_valid =
        request_has_session_credential(request, expected_secret);
    let origin_valid = request_has_exact_origin(request, expected_origin);
    if !credential_valid || !origin_valid {
        return json_response(
            "401 Unauthorized",
            br#"{"error":"unauthenticated"}"#,
        );
    }
    match handshake.evaluate(handshake_versions(request)) {
        HandshakeResult::Compatible { versions } => {
            handshake_success_response(versions)
        },
        HandshakeResult::Incompatible {
            diagnostics,
            dimension,
            expected,
            ..
        } => handshake_incompatible_response(&diagnostics, dimension, expected),
    }
}

fn request_body(request: &[u8]) -> Option<&[u8]> {
    let head_end = request_head_end(request)?;
    let request_head = request.get(..head_end)?;
    if header_is_present(request_head, "transfer-encoding")
        || !header_is_present(request_head, "content-length")
    {
        return None;
    }
    let declared = parse_content_length_value(single_header_value(
        request_head,
        "content-length",
    )?)
    .ok()?;
    let body = request.get(head_end..)?;
    (body.len() == declared).then_some(body)
}

fn request_origin_is_admitted(request: &[u8], expected_origin: &str) -> bool {
    if !header_is_present(request, "origin") {
        return true;
    }
    request_has_exact_origin(request, expected_origin)
}

fn route_draft_read(
    request: &[u8],
    field: DraftField,
    expected_origin: &str,
    expected_secret: &str,
    draft: &dyn SessionDraft,
) -> Vec<u8> {
    let credential_valid =
        request_has_session_credential(request, expected_secret);
    let origin_valid = request_origin_is_admitted(request, expected_origin);
    if !credential_valid || !origin_valid {
        return json_response(
            "401 Unauthorized",
            br#"{"error":"unauthenticated"}"#,
        );
    }
    response("200 OK", TEXT_CONTENT_TYPE, draft.value(field).as_bytes())
}

fn draft_resource_limit_response(diagnostics: &DiagnosticSet) -> Vec<u8> {
    let Some(diagnostic) = diagnostics.diagnostics.first() else {
        return invalid_diagnostic_response();
    };
    let body = format!(
        concat!(
            "{{\"error\":\"resource_limit\",",
            "\"diagnostics\":{{",
            "\"version\":\"{}\",",
            "\"completeness\":\"{}\",",
            "\"items\":[{{\"code\":\"{}\"}}]}}}}",
        ),
        DIAGNOSTIC_VERSION,
        completeness_name(diagnostics.completeness),
        diagnostic.code.stable_name(),
    );
    json_response("413 Content Too Large", body.as_bytes())
}

fn route_draft_replace(
    request: &[u8],
    field: DraftField,
    expected_origin: &str,
    expected_secret: &str,
    draft: &mut dyn SessionDraft,
) -> Vec<u8> {
    let credential_valid =
        request_has_session_credential(request, expected_secret);
    let origin_valid = request_has_exact_origin(request, expected_origin);
    if !credential_valid || !origin_valid {
        return json_response(
            "401 Unauthorized",
            br#"{"error":"unauthenticated"}"#,
        );
    }
    let Some(body) = request_body(request) else {
        return json_response(
            "400 Bad Request",
            br#"{"error":"invalid_request"}"#,
        );
    };
    let Ok(value) = str::from_utf8(body) else {
        return json_response(
            "400 Bad Request",
            br#"{"error":"invalid_request"}"#,
        );
    };
    match draft.replace(field, value.to_owned()) {
        DraftMutation::Applied => empty_response("204 No Content"),
        DraftMutation::ResourceLimit { diagnostics } => {
            draft_resource_limit_response(&diagnostics)
        },
    }
}

const fn is_origin_form_target_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'.' | b'_' | b'~' | b'!' | b'$' | b'&' | b'\''
                | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b':'
                | b'@' | b'/' | b'?'
        )
}

fn is_origin_form_target(target: &str) -> bool {
    if !target.starts_with('/') {
        return false;
    }
    let bytes = target.as_bytes();
    let mut index = 0usize;
    while let Some(byte) = bytes.get(index).copied() {
        if byte == b'%' {
            let Some(first) = bytes.get(index.saturating_add(1)) else {
                return false;
            };
            let Some(second) = bytes.get(index.saturating_add(2)) else {
                return false;
            };
            if !first.is_ascii_hexdigit() || !second.is_ascii_hexdigit() {
                return false;
            }
            index = index.saturating_add(3);
        } else {
            if !is_origin_form_target_character(byte) {
                return false;
            }
            index = index.saturating_add(1);
        }
    }
    true
}

fn request_method_host_and_target(
    request: &[u8],
) -> Option<(&str, &str, &str)> {
    let text = str::from_utf8(request).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let (method, remainder) = request_line.split_once(' ')?;
    let (target, version) = remainder.split_once(' ')?;
    if !matches!(method, "GET" | "POST")
        || !is_origin_form_target(target)
        || version != "HTTP/1.1"
    {
        return None;
    }

    let mut host = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = split_header_line(line)?;
        if name.eq_ignore_ascii_case("host") {
            if host.is_some() {
                return None;
            }
            host = Some(trim_http_ows(value));
        }
    }
    Some((method, host?, target))
}

fn response(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n\
         Content-Length: {}\r\n{RESPONSE_TRAILERS}",
        body.len(),
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

fn empty_response(status: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\n{RESPONSE_TRAILERS}")
        .into_bytes()
}

fn json_response(status: &str, body: &[u8]) -> Vec<u8> {
    response(status, JSON_CONTENT_TYPE, body)
}

fn public_get_response(target: &str) -> Option<Vec<u8>> {
    match target {
        "/" | "/index.html" => {
            Some(response("200 OK", HTML_CONTENT_TYPE, INDEX_HTML))
        },
        "/generated/main.js" => {
            Some(response("200 OK", JAVASCRIPT_CONTENT_TYPE, MAIN_JAVASCRIPT))
        },
        "/generated/session-diagnostic.js" => Some(response(
            "200 OK",
            JAVASCRIPT_CONTENT_TYPE,
            SESSION_DIAGNOSTIC_JAVASCRIPT,
        )),
        "/generated/session-draft.js" => Some(response(
            "200 OK",
            JAVASCRIPT_CONTENT_TYPE,
            SESSION_DRAFT_JAVASCRIPT,
        )),
        "/generated/session-fragment.js" => Some(response(
            "200 OK",
            JAVASCRIPT_CONTENT_TYPE,
            SESSION_FRAGMENT_JAVASCRIPT,
        )),
        "/generated/session-handshake.js" => Some(response(
            "200 OK",
            JAVASCRIPT_CONTENT_TYPE,
            SESSION_HANDSHAKE_JAVASCRIPT,
        )),
        "/health" => Some(json_response(
            "200 OK",
            br#"{"product":"atrament","state":"ready"}"#,
        )),
        "/workspace.css" => {
            Some(response("200 OK", CSS_CONTENT_TYPE, WORKSPACE_CSS))
        },
        _ => None,
    }
}

/// Route one parsed HTTP request after exact canonical `Host` admission.
#[must_use]
pub fn route_request(
    request: &[u8],
    expected_host: &str,
    expected_origin: &str,
    expected_secret: &str,
    handshake: &dyn SessionHandshake,
    draft: &mut dyn SessionDraft,
) -> Vec<u8> {
    let Some((method, host, target)) = request_method_host_and_target(request)
    else {
        return json_response(
            "400 Bad Request",
            br#"{"error":"invalid_request"}"#,
        );
    };
    if host != expected_host {
        return json_response(
            "421 Misdirected Request",
            br#"{"error":"invalid_host"}"#,
        );
    }
    if method == "GET"
        && let Some(public_response) = public_get_response(target)
    {
        return public_response;
    }
    match (method, target) {
        ("GET", "/api/session/candidate") => route_draft_read(
            request,
            DraftField::Candidate,
            expected_origin,
            expected_secret,
            draft,
        ),
        ("GET", "/api/session/source") => route_draft_read(
            request,
            DraftField::Source,
            expected_origin,
            expected_secret,
            draft,
        ),
        ("GET", "/api/session/task") => route_draft_read(
            request,
            DraftField::Task,
            expected_origin,
            expected_secret,
            draft,
        ),
        ("POST", "/api/handshake") => route_handshake(
            request,
            expected_origin,
            expected_secret,
            handshake,
        ),
        ("POST", "/api/session/candidate") => route_draft_replace(
            request,
            DraftField::Candidate,
            expected_origin,
            expected_secret,
            draft,
        ),
        ("POST", "/api/session/source") => route_draft_replace(
            request,
            DraftField::Source,
            expected_origin,
            expected_secret,
            draft,
        ),
        ("POST", "/api/session/task") => route_draft_replace(
            request,
            DraftField::Task,
            expected_origin,
            expected_secret,
            draft,
        ),
        ("POST", _) | ("GET", "/api/handshake") => {
            json_response("400 Bad Request", br#"{"error":"invalid_request"}"#)
        },
        ("GET", _) => {
            json_response("404 Not Found", br#"{"error":"not_found"}"#)
        },
        _ => {
            json_response("400 Bad Request", br#"{"error":"invalid_request"}"#)
        },
    }
}

fn serve_connection(
    stream: &mut TcpStream,
    expected_host: &str,
    expected_origin: &str,
    expected_secret: &str,
    handshake: &dyn SessionHandshake,
    draft: &mut dyn SessionDraft,
) -> io::Result<()> {
    let request = match read_request(stream) {
        Ok(request) => request,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof
            ) =>
        {
            let response = json_response(
                "400 Bad Request",
                br#"{"error":"invalid_request"}"#,
            );
            stream.write_all(&response)?;
            return stream.flush();
        },
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
            ) =>
        {
            let response = json_response(
                "408 Request Timeout",
                br#"{"error":"request_timeout"}"#,
            );
            stream.write_all(&response)?;
            return stream.flush();
        },
        Err(error) => return Err(error),
    };
    let response = route_request(
        &request,
        expected_host,
        expected_origin,
        expected_secret,
        handshake,
        draft,
    );
    stream.write_all(&response)?;
    stream.flush()
}
