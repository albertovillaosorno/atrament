// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Native CLI executable entrypoint for shared Rust paper authority.
// - Must-Not:
//   - Independently validate, render or persist paper profiles.
// - Allows:
//   - Inputs: Explicit CLI arguments through the Rust paper library.
//   - Outputs: Exact library-produced stdout bytes or refusal.
//   - Side effects: Delegated explicit profile I/O and stdout writes.
// - Split-When:
//   - Native CLI gains a separate transport contract.
// - Merge-When:
//   - Application entrypoint subsumes CLI composition.
// - Summary:
//   - Runs the canonical paper profile and PDF implementation unchanged.
// - Description:
//   - Connects CLI invocation to shared Rust notebook paper operations.
// - Usage:
//   - cargo run -q -p atrament_notebook_paper_cli -- --help
// - Defaults:
//   - A4 grid output through the shared library.
//

//! Native paper-profile CLI shell delegating to the shared Rust authority.

use std::error::Error;

// These package dependencies are exercised by the shared library target.
// The CLI binary itself delegates instead of rebuilding that authority.
use atrament_paper_mark_geometry as _;
use atrament_pdf_composition_plan as _;
use atrament_pdf_vector_serializer as _;
use atrament_physical_page_profile as _;
// The sibling print binary uses this shared package dependency.
use atrament_profile_entry_sha256 as _;
use atrament_profile_file_commit as _;
use atrament_profile_file_reader as _;
use atrament_profile_selected_file_open as _;
#[cfg(all(test, unix))]
use nix::sys::stat::Mode;
#[cfg(all(test, unix))]
const _: Mode = Mode::empty();
use atrament_profile_json_syntax as _;


/// Execute the Rust paper-profile CLI with validated output.
///
/// # Errors
///
/// Propagates input, file, or output errors without claiming success.
fn main() -> Result<(), Box<dyn Error>> {
    atrament_notebook_paper_cli::run_cli()
}
