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
    NoDiscreteGpuMachine, RenderPerformanceCoverageError,
    RenderPerformanceObservation, RenderPerformanceScenario,
    validate_render_performance_coverage,
};
use atrament_render_quality_profile::RenderQualityMode;

const SCENARIOS: [RenderPerformanceScenario; 6] = [
    RenderPerformanceScenario::DenseEquations,
    RenderPerformanceScenario::FinalExport,
    RenderPerformanceScenario::LongPage,
    RenderPerformanceScenario::ManyImages,
    RenderPerformanceScenario::RapidEdits,
    RenderPerformanceScenario::Zoom,
];

#[test]
fn first_release_cpu_benchmark_scenarios_are_explicit() {
    assert_eq!(SCENARIOS.len(), 6);
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

fn observation(
    scenario: RenderPerformanceScenario,
    quality_mode: RenderQualityMode,
) -> RenderPerformanceObservation<u64, &'static str, u64> {
    RenderPerformanceObservation {
        latency: 1,
        machine: NoDiscreteGpuMachine {
            machine_identity: "no-discrete-gpu-host",
        },
        peak_memory: 2,
        quality_mode,
        scenario,
    }
}

#[test]
fn complete_evidence_covers_all_scenarios_and_both_quality_roles() {
    let observations = [
        observation(
            RenderPerformanceScenario::DenseEquations,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::FinalExport,
            RenderQualityMode::Final,
        ),
        observation(
            RenderPerformanceScenario::LongPage,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::ManyImages,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::RapidEdits,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::Zoom,
            RenderQualityMode::Preview,
        ),
    ];
    assert_eq!(validate_render_performance_coverage(&observations), Ok(()));
}

#[test]
fn every_workload_scenario_is_independently_required() {
    for missing in SCENARIOS {
        let observations = SCENARIOS
            .into_iter()
            .filter(|scenario| *scenario != missing)
            .map(|scenario| {
                let quality_mode = if scenario
                    == RenderPerformanceScenario::FinalExport
                {
                    RenderQualityMode::Final
                } else {
                    RenderQualityMode::Preview
                };
                observation(scenario, quality_mode)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            validate_render_performance_coverage(&observations),
            Err(RenderPerformanceCoverageError::ScenarioAbsent(missing)),
            "missing scenario {missing:?}",
        );
    }
}

#[test]
fn scenario_coverage_does_not_substitute_for_final_quality_evidence() {
    let observations = [
        observation(
            RenderPerformanceScenario::DenseEquations,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::FinalExport,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::LongPage,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::ManyImages,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::RapidEdits,
            RenderQualityMode::Preview,
        ),
        observation(
            RenderPerformanceScenario::Zoom,
            RenderQualityMode::Preview,
        ),
    ];
    assert_eq!(
        validate_render_performance_coverage(&observations),
        Err(RenderPerformanceCoverageError::FinalQualityMissing),
    );
}

#[test]
fn scenario_coverage_does_not_substitute_for_preview_quality_evidence() {
    let observations = [
        observation(
            RenderPerformanceScenario::DenseEquations,
            RenderQualityMode::Final,
        ),
        observation(
            RenderPerformanceScenario::FinalExport,
            RenderQualityMode::Final,
        ),
        observation(
            RenderPerformanceScenario::LongPage,
            RenderQualityMode::Final,
        ),
        observation(
            RenderPerformanceScenario::ManyImages,
            RenderQualityMode::Final,
        ),
        observation(
            RenderPerformanceScenario::RapidEdits,
            RenderQualityMode::Final,
        ),
        observation(RenderPerformanceScenario::Zoom, RenderQualityMode::Final),
    ];
    assert_eq!(
        validate_render_performance_coverage(&observations),
        Err(RenderPerformanceCoverageError::PreviewNotObserved),
    );
}

#[test]
fn every_complete_scenario_quality_mask_matches_role_coverage_oracle() {
    let mut cases = 0_u8;
    let mut saw = [false; 3];
    for final_mask in 0_u8..64 {
        let observations = SCENARIOS
            .into_iter()
            .enumerate()
            .map(|(index, scenario)| {
                let quality_mode = if final_mask & (1_u8 << index) == 0 {
                    RenderQualityMode::Preview
                } else {
                    RenderQualityMode::Final
                };
                observation(scenario, quality_mode)
            })
            .collect::<Vec<_>>();
        let expected = if final_mask == 0 {
            saw[0] = true;
            Err(RenderPerformanceCoverageError::FinalQualityMissing)
        } else if final_mask == 0b11_1111 {
            saw[1] = true;
            Err(RenderPerformanceCoverageError::PreviewNotObserved)
        } else {
            saw[2] = true;
            Ok(())
        };
        assert_eq!(
            validate_render_performance_coverage(&observations),
            expected,
            "final-quality assignment mask {final_mask:#08b}",
        );
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 64);
    assert!(saw.into_iter().all(|seen| seen));
}
