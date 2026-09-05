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
//   - Process startup, browser launch orchestration, and launch recovery
//     output.
// - Must-Not:
//   - Implement HTTP admission, browser process mechanics, or session state.
// - Allows:
//   - Inputs: Runtime binding results and typed browser launch outcomes.
//   - Outputs: Secret-free machine startup records and human recovery text.
//   - Side effects: Stdout and stderr publication plus component coordination.
// - Split-When:
//   - Startup lifecycle becomes independently configurable or multi-process.
// - Merge-When:
//   - Atrament no longer composes separate runtime and launcher components.
// - Summary:
//   - Wires the localhost runtime to the operating-system browser launcher.
// - Description:
//   - Keeps cross-adapter orchestration in the process composition root.
// - Usage:
//   - Run the atrament binary to bind, publish, launch, and serve the
//     workspace.
// - Defaults:
//   - Fails startup closed when automatic browser launch is unavailable.
//   - Never publishes the session credential for manual recovery.
//

//! Atrament process composition for one disposable localhost browser session.

use std::io::{self, Write as _};

use atrament_browser_launch as browser_launch;
use atrament_session_application::SessionApplication;
use atrament_session_handshake::{
    HandshakeService, PRODUCT_VERSION, PROTOCOL_VERSION,
};
use atrament_session_runtime::Runtime;
use atrament_session_secret::SessionSecret;

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
        PRODUCT_VERSION, PROTOCOL_VERSION, origin_json, state,
    )?;
    output.flush()
}

pub(crate) fn browser_launch_failure(
    error: &browser_launch::LaunchError,
) -> io::Error {
    io::Error::other(format!(
        concat!(
            "Atrament browser launch failed: {}. ",
            "Fix automatic browser launch and restart Atrament; ",
            "the session credential is intentionally not published for ",
            "manual recovery.",
        ),
        error,
    ))
}

fn bind_runtime() -> io::Result<Runtime> {
    Runtime::bind().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Atrament loopback startup failed: {error}"),
        )
    })
}

fn launch_url(origin: &str, secret: &SessionSecret) -> String {
    format!("{origin}#session={}", secret.encoded())
}

fn new_session_secret() -> io::Result<SessionSecret> {
    SessionSecret::generate().map_err(|error| {
        io::Error::other(format!(
            "Atrament session credential generation failed: {error}",
        ))
    })
}

fn main() -> io::Result<()> {
    publish_startup("starting", None)?;
    let mut application = SessionApplication::default();
    let handshake = HandshakeService;
    let secret = new_session_secret()?;
    let runtime = bind_runtime()?;
    publish_startup("listening", Some(runtime.origin()))?;
    let initial_browser_url = launch_url(runtime.origin(), &secret);
    browser_launch::launch(&initial_browser_url)
        .map_err(|error| browser_launch_failure(&error))?;
    publish_startup("ready", Some(runtime.origin()))?;
    runtime.serve(secret.encoded(), &handshake, &mut application);
    Ok(())
}
