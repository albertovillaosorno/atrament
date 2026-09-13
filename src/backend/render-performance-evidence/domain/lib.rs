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

/// Borrowed observation sequence consumed by coverage validation.
pub type RenderPerformanceObservations<Latency, MachineIdentity, PeakMemory> =
    [RenderPerformanceObservation<Latency, MachineIdentity, PeakMemory>];

/// Constructor-sealed evidence that one exact benchmark observation sequence
/// covers every required workload and both quality roles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedRenderPerformanceCoverage<
    'observations,
    Latency,
    MachineIdentity,
    PeakMemory,
> {
    observations: &'observations RenderPerformanceObservations<
        Latency,
        MachineIdentity,
        PeakMemory,
    >,
}

impl<'observations, Latency, MachineIdentity, PeakMemory>
    ValidatedRenderPerformanceCoverage<
        'observations,
        Latency,
        MachineIdentity,
        PeakMemory,
    >
{
    /// Return the exact caller-owned observation sequence that was admitted.
    #[must_use]
    pub const fn observations(
        &self,
    ) -> &'observations RenderPerformanceObservations<
        Latency,
        MachineIdentity,
        PeakMemory,
    > {
        self.observations
    }
}

/// Missing evidence required for a complete first-release CPU benchmark set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderPerformanceCoverageError {
    /// No Final-quality observation is present.
    FinalQualityMissing,
    /// No Preview-quality observation is present.
    PreviewNotObserved,
    /// One required first-release workload scenario has no observation.
    ScenarioAbsent(RenderPerformanceScenario),
}

/// Validate scenario and quality-role completeness without applying budgets.
///
/// # Errors
///
/// Returns [`RenderPerformanceCoverageError`] when any of the six required
/// workload scenarios is absent, or when the evidence has no Preview or no
/// Final-quality observation.
pub fn validate_render_performance_coverage<
    Latency,
    MachineIdentity,
    PeakMemory,
>(
    observations: &RenderPerformanceObservations<
        Latency,
        MachineIdentity,
        PeakMemory,
    >,
) -> Result<(), RenderPerformanceCoverageError> {
    for scenario in [
        RenderPerformanceScenario::DenseEquations,
        RenderPerformanceScenario::FinalExport,
        RenderPerformanceScenario::LongPage,
        RenderPerformanceScenario::ManyImages,
        RenderPerformanceScenario::RapidEdits,
        RenderPerformanceScenario::Zoom,
    ] {
        if !observations
            .iter()
            .any(|observation| observation.scenario == scenario)
        {
            return Err(RenderPerformanceCoverageError::ScenarioAbsent(
                scenario,
            ));
        }
    }
    if !observations
        .iter()
        .any(|observation| observation.quality_mode == RenderQualityMode::Final)
    {
        return Err(RenderPerformanceCoverageError::FinalQualityMissing);
    }
    if !observations.iter().any(|observation| {
        observation.quality_mode == RenderQualityMode::Preview
    }) {
        return Err(RenderPerformanceCoverageError::PreviewNotObserved);
    }
    Ok(())
}

/// Validate and seal one complete first-release CPU benchmark observation set.
///
/// # Errors
///
/// Returns the same scenario or quality-role omission as
/// [`validate_render_performance_coverage`].
pub fn validate_render_performance_coverage_view<
    Latency,
    MachineIdentity,
    PeakMemory,
>(
    observations: &RenderPerformanceObservations<
        Latency,
        MachineIdentity,
        PeakMemory,
    >,
) -> Result<
    ValidatedRenderPerformanceCoverage<
        '_,
        Latency,
        MachineIdentity,
        PeakMemory,
    >,
    RenderPerformanceCoverageError,
> {
    validate_render_performance_coverage(observations)?;
    Ok(ValidatedRenderPerformanceCoverage { observations })
}
