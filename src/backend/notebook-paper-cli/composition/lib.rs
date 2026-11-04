// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Wiring the paper application and caller-selected CLI effects.
// - Must-Not:
//   - Independently define paper geometry or silently write files.
// - Allows:
//   - Inputs: Bounded paper profile bytes and optional CLI arguments.
//   - Outputs: Native PDF/PNG and canonical profile bytes or refusal.
//   - Side effects: Selected profile reads and explicit CLI output writes.
// - Split-When:
//   - Native CLI effects require distinct transport adapters.
// - Merge-When:
//   - Unified notebook export composition owns these entrypoints.
// - Summary:
//   - Exposes the same public paper APIs while isolating external effects.
// - Description:
//   - Composition imports the pure renderer without a reverse dependency.
// - Usage:
//   - cargo test -p atrament_notebook_paper_cli
// - Defaults:
//   - No file writes or printing without caller selection.
//

//! Public notebook-paper authority and selected CLI transport.

#[path = "../application/lib.rs"]
pub mod application;
mod cli;

pub use application::*;
pub use cli::{run_cli, run_cli_args};
