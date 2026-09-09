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
//   - Regression evidence for CPU render-performance observations.
// - Must-Not:
//   - Choose budgets, units, hardware tiers, render algorithms, or pass/fail
//     policy.
// - Allows:
//   - Inputs: Deterministic Preview/Final measurements and scenario fixtures.
//   - Outputs: Assertions over evidence retention and scenario coverage.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Budget validation or benchmark execution gains independent fixtures.
// - Merge-When:
//   - Performance evidence moves into a renderer benchmark harness.
// - Summary:
//   - Proves CPU benchmark measurements remain inspectable before budgeting.
// - Description:
//   - Covers required workloads and no-discrete-GPU machine evidence.
// - Usage:
//   - Compile directly against the render-performance-evidence domain.
// - Defaults:
//   - No performance threshold is implicit.
//
use atrament_render_performance_evidence::{
    NoDiscreteGpuMachine, RenderPerformanceObservation,
    RenderPerformanceScenario,
};
use atrament_render_quality_profile::RenderQualityMode;

#[test]
fn first_release_cpu_benchmark_scenarios_are_explicit() {
    let scenarios = [
        RenderPerformanceScenario::DenseEquations,
        RenderPerformanceScenario::FinalExport,
        RenderPerformanceScenario::LongPage,
        RenderPerformanceScenario::ManyImages,
        RenderPerformanceScenario::RapidEdits,
        RenderPerformanceScenario::Zoom,
    ];
    assert_eq!(scenarios.len(), 6);
}

#[test]
fn observations_retain_preview_and_final_measurements_without_budgeting() {
    let preview = RenderPerformanceObservation {
        latency: (14_u64, "ms"),
        machine: NoDiscreteGpuMachine {
            machine_identity: "integrated-host-4",
        },
        peak_memory: (180_u64, "MiB"),
        quality_mode: RenderQualityMode::Preview,
        scenario: RenderPerformanceScenario::RapidEdits,
    };
    let final_render = RenderPerformanceObservation {
        latency: (430_u64, "ms"),
        machine: NoDiscreteGpuMachine {
            machine_identity: "integrated-host-4",
        },
        peak_memory: (420_u64, "MiB"),
        quality_mode: RenderQualityMode::Final,
        scenario: RenderPerformanceScenario::FinalExport,
    };
    assert_eq!(preview.quality_mode, RenderQualityMode::Preview);
    assert_eq!(final_render.quality_mode, RenderQualityMode::Final);
    assert_eq!(preview.machine.machine_identity, "integrated-host-4");
    assert_eq!(preview.latency, (14, "ms"));
    assert_eq!(final_render.peak_memory, (420, "MiB"));
}

#[test]
fn measurement_units_and_values_remain_caller_owned() {
    let observation = RenderPerformanceObservation {
        latency: "caller-latency-evidence",
        machine: NoDiscreteGpuMachine {
            machine_identity: 77_u64,
        },
        peak_memory: vec![1_u8, 2_u8, 3_u8],
        quality_mode: RenderQualityMode::Preview,
        scenario: RenderPerformanceScenario::Zoom,
    };
    assert_eq!(observation.latency, "caller-latency-evidence");
    assert_eq!(observation.peak_memory, [1, 2, 3]);
    assert_eq!(observation.machine.machine_identity, 77);
}
