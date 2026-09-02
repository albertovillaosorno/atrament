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
//   - Loopback listener startup, frontend resources, and public health.
// - Must-Not:
//   - Bind non-loopback addresses or expose session-private application state.
// - Allows:
//   - Inputs: Local HTTP requests addressed to the canonical startup endpoint.
//   - Outputs: Secret-free startup, frontend resources, and public health.
//   - Side effects: Loopback socket binding, stdout startup publication, and
//     HTTP response writes.
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
//! This binary owns pre-authentication listener admission, public frontend
//! resources, and health routing. Later runtime slices add authenticated
//! session services without widening this transport boundary.

use std::io::{self, Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::str;

const MAX_HEADER_BYTES: usize = 16 * 1024;
const PROCESS_VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
    Some(version) => version,
    None => "0.1.0",
};
const PROTOCOL_VERSION: &str = "atrament.runtime/1";
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

    fn serve(self) {
        for incoming in self.listener.incoming() {
            match incoming {
                Ok(mut connection) => {
                    drop(serve_connection(
                        &mut connection,
                        &self.expected_host,
                    ));
                },
                Err(_) => break,
            }
        }
    }
}

fn publish_startup(state: &str, origin: Option<&str>) -> io::Result<()> {
    let origin_json = origin
        .map_or_else(|| String::from("null"), |value| format!("\"{value}\""));
    let stdout = io::stdout();
    let mut output = stdout.lock();
    writeln!(
        output,
        concat!(
            "{{\"product\":\"atrament\",",
            "\"process_version\":\"{}\",",
            "\"protocol_version\":\"{}\",",
            "\"origin\":{},\"state\":\"{}\"}}",
        ),
        PROCESS_VERSION, PROTOCOL_VERSION, origin_json, state,
    )?;
    output.flush()
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

fn request_host_and_target(request: &[u8]) -> Option<(&str, &str)> {
    let text = str::from_utf8(request).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut request_parts = request_line.split_ascii_whitespace();
    let method = request_parts.next()?;
    let target = request_parts.next()?;
    let version = request_parts.next()?;
    let extra_part = request_parts.next();
    if method != "GET" || version != "HTTP/1.1" || extra_part.is_some() {
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
    Some((host?, target))
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

fn json_response(status: &str, body: &'static [u8]) -> Vec<u8> {
    response(status, JSON_CONTENT_TYPE, body)
}

/// Route one parsed HTTP request after exact canonical `Host` admission.
#[must_use]
pub fn route_request(request: &[u8], expected_host: &str) -> Vec<u8> {
    let Some((host, target)) = request_host_and_target(request) else {
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
    match target {
        "/" | "/index.html" => {
            response("200 OK", HTML_CONTENT_TYPE, INDEX_HTML)
        },
        "/generated/main.js" => {
            response("200 OK", JAVASCRIPT_CONTENT_TYPE, MAIN_JAVASCRIPT)
        },
        "/health" => json_response(
            "200 OK",
            br#"{"product":"atrament","state":"listening"}"#,
        ),
        "/workspace.css" => response("200 OK", CSS_CONTENT_TYPE, WORKSPACE_CSS),
        _ => json_response("404 Not Found", br#"{"error":"not_found"}"#),
    }
}

fn serve_connection(
    stream: &mut TcpStream,
    expected_host: &str,
) -> io::Result<()> {
    let request = read_request_head(stream)?;
    let response = route_request(&request, expected_host);
    stream.write_all(&response)?;
    stream.flush()
}

fn main() -> io::Result<()> {
    publish_startup("starting", None)?;
    let runtime = Runtime::bind()?;
    publish_startup("listening", Some(runtime.origin()))?;
    runtime.serve();
    Ok(())
}
