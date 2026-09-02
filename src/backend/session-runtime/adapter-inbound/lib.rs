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
//   - Bind non-loopback addresses or expose session-private application state.
// - Allows:
//   - Inputs: Local HTTP requests and in-memory expected session credentials.
//   - Outputs: Public resources plus admitted handshake results.
//   - Side effects: Loopback binding and HTTP response writes.
// - Split-When:
//   - Authenticated routing needs an independently testable adapter.
// - Merge-When:
//   - Runtime admission no longer has independent transport ownership.
// - Summary:
//   - Starts the disposable Atrament localhost session runtime.
// - Description:
//   - Owns exact loopback endpoint admission before application services exist.
// - Usage:
//   - Run the atrament binary to serve the browser shell on one loopback
//     endpoint.
// - Defaults:
//   - Binds 127.0.0.1 on an operating-system assigned port.
//

//! Disposable loopback transport for the Atrament browser session runtime.
//!
//! This adapter owns listener admission, public frontend resources, health
//! routing, and authenticated handshake transport. Later runtime slices reuse
//! these admission checks without widening this transport boundary.

use std::io::{self, Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::str;

use atrament_session_handshake_port::{
    HandshakeResult, SessionHandshake, VersionDimension, Versions,
};

const ENCODED_SECRET_BYTES: usize = 64;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";
const CSS_CONTENT_TYPE: &str = "text/css; charset=utf-8";
const JAVASCRIPT_CONTENT_TYPE: &str = "text/javascript; charset=utf-8";
const JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";
const RESPONSE_TRAILERS: &str = concat!(
    "Cache-Control: no-store\r\n",
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
const SESSION_FRAGMENT_JAVASCRIPT: &[u8] = include_bytes!(
    "../../../browser/workspace/adapter-inbound/generated/session-fragment.js"
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
    ) {
        for incoming in self.listener.incoming() {
            match incoming {
                Ok(mut connection) => {
                    drop(serve_connection(
                        &mut connection,
                        &self.expected_host,
                        &self.origin,
                        expected_secret,
                        handshake,
                    ));
                },
                Err(_) => break,
            }
        }
    }
}

fn read_request_head(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        let Some(read_bytes) = chunk.get(..read) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "socket read exceeded the receive buffer",
            ));
        };
        bytes.extend_from_slice(read_bytes);
        if bytes.len() > MAX_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request headers exceed runtime limit",
            ));
        }
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(bytes);
        }
    }
    Ok(bytes)
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
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case(expected_name) {
            if matched_value.is_some() {
                return None;
            }
            matched_value = Some(value.trim());
        }
    }
    matched_value
}

fn authorization_bearer(request: &[u8]) -> Option<&str> {
    single_header_value(request, "authorization")?.strip_prefix("Bearer ")
}

fn fixed_work_secret_match(expected: &str, candidate: &str) -> bool {
    let valid_length = expected.len() == ENCODED_SECRET_BYTES
        && candidate.len() == ENCODED_SECRET_BYTES;
    let mut expected_bytes = [0u8; ENCODED_SECRET_BYTES];
    let mut candidate_bytes = [0u8; ENCODED_SECRET_BYTES];
    for (slot, byte) in expected_bytes.iter_mut().zip(expected.bytes()) {
        *slot = byte;
    }
    for (slot, byte) in candidate_bytes.iter_mut().zip(candidate.bytes()) {
        *slot = byte;
    }
    let mut difference = 0u8;
    for (expected_byte, candidate_byte) in
        expected_bytes.iter().zip(candidate_bytes.iter())
    {
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

fn handshake_incompatible_response(
    dimension: VersionDimension,
    expected: &str,
) -> Vec<u8> {
    let body = format!(
        concat!(
            "{{\"result\":\"incompatible\",",
            "\"diagnostic\":{{",
            "\"code\":\"atrament.handshake.version-mismatch\",",
            "\"dimension\":\"{}\",",
            "\"expected\":\"{}\"}}}}",
        ),
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
        HandshakeResult::Incompatible { dimension, expected, .. } => {
            handshake_incompatible_response(dimension, expected)
        },
    }
}

fn request_method_host_and_target(
    request: &[u8],
) -> Option<(&str, &str, &str)> {
    let text = str::from_utf8(request).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut request_parts = request_line.split_ascii_whitespace();
    let method = request_parts.next()?;
    let target = request_parts.next()?;
    let version = request_parts.next()?;
    let extra_part = request_parts.next();
    if !matches!(method, "GET" | "POST")
        || version != "HTTP/1.1"
        || extra_part.is_some()
    {
        return None;
    }

    let mut host = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("host") {
            if host.is_some() {
                return None;
            }
            host = Some(value.trim());
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

fn json_response(status: &str, body: &[u8]) -> Vec<u8> {
    response(status, JSON_CONTENT_TYPE, body)
}

/// Route one parsed HTTP request after exact canonical `Host` admission.
#[must_use]
pub fn route_request(
    request: &[u8],
    expected_host: &str,
    expected_origin: &str,
    expected_secret: &str,
    handshake: &dyn SessionHandshake,
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
    match (method, target) {
        ("GET", "/" | "/index.html") => {
            response("200 OK", HTML_CONTENT_TYPE, INDEX_HTML)
        },
        ("GET", "/generated/main.js") => {
            response("200 OK", JAVASCRIPT_CONTENT_TYPE, MAIN_JAVASCRIPT)
        },
        ("GET", "/generated/session-fragment.js") => response(
            "200 OK",
            JAVASCRIPT_CONTENT_TYPE,
            SESSION_FRAGMENT_JAVASCRIPT,
        ),
        ("GET", "/health") => json_response(
            "200 OK",
            br#"{"product":"atrament","state":"listening"}"#,
        ),
        ("GET", "/workspace.css") => {
            response("200 OK", CSS_CONTENT_TYPE, WORKSPACE_CSS)
        },
        ("POST", "/api/handshake") => route_handshake(
            request,
            expected_origin,
            expected_secret,
            handshake,
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
) -> io::Result<()> {
    let request = read_request_head(stream)?;
    let response = route_request(
        &request,
        expected_host,
        expected_origin,
        expected_secret,
        handshake,
    );
    stream.write_all(&response)?;
    stream.flush()
}
