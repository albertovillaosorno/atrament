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
//   - Transport-neutral CPU render benchmark evidence.
// - Must-Not:
//   - Choose latency or memory budgets, units, hardware tiers, render
//     scheduling, algorithms, or pass/fail policy.
// - Allows:
//   - Inputs: Caller-owned machine identity, latency, peak memory, and accepted
//     benchmark scenario.
//   - Outputs: Preview/final benchmark observations from no-discrete-GPU hosts.
//   - Side effects: None.
// - Split-When:
//   - Budget admission or benchmark execution gains independent authority.
// - Merge-When:
//   - Performance evidence becomes inseparable from one renderer benchmark.
// - Summary:
//   - Records required CPU preview/final benchmark evidence without budgets.
// - Description:
//   - Covers first-release workload scenarios on no-discrete-GPU machines.
// - Usage:
//   - Retain measurements before a future product budget evaluates them.
// - Defaults:
//   - No measurement value or budget is inferred.
//

//! CPU render benchmark evidence independent of product performance budgets.

use atrament_render_quality_profile::RenderQualityMode;

/// CPU benchmark scenarios named by the first-release performance TODO.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RenderPerformanceScenario {
    /// Dense mathematical expressions and equation layout.
    DenseEquations,
    /// Final output/export rendering workload.
    FinalExport,
    /// Long page with substantial semantic/render content.
    LongPage,
    /// Page containing many admitted images.
    ManyImages,
    /// Rapid edit/re-render workload.
    RapidEdits,
    /// Interactive zoom workload.
    Zoom,
}

/// Machine evidence for CPU rendering without a discrete GPU.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoDiscreteGpuMachine<MachineIdentity> {
    /// Caller-owned identity of the measured machine/configuration.
    pub machine_identity: MachineIdentity,
}

/// One measured CPU render-performance observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderPerformanceObservation<Latency, MachineIdentity, PeakMemory> {
    /// Caller-owned latency measurement.
    pub latency: Latency,
    /// Machine/configuration with no discrete GPU.
    pub machine: NoDiscreteGpuMachine<MachineIdentity>,
    /// Caller-owned peak-memory measurement.
    pub peak_memory: PeakMemory,
    /// Preview or Final quality role exercised by this observation.
    pub quality_mode: RenderQualityMode,
    /// First-release workload scenario exercised by this observation.
    pub scenario: RenderPerformanceScenario,
}
