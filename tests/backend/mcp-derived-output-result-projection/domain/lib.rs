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
//   - Exhaustive regression evidence for MCP derived/output result projection.
// - Must-Not:
//   - Execute MCP or output operations, infer tool admission, touch files, or
//     simulate retry/transport outcomes.
// - Allows:
//   - Inputs: All eight MCP capabilities crossed with all 11 output results.
//   - Outputs: Exact operation mapping and compatible effect disposition.
//   - Side effects: None.
// - Split-When:
//   - Executable adapter parity tests gain independent result authority.
// - Merge-When:
//   - MCP adapter result tests fully subsume this structural cross-product.
// - Summary:
//   - Pins three output capabilities and all 88 capability/result pairs.
// - Description:
//   - Keeps non-output capabilities and incompatible result classes
//     unprojected.
// - Usage:
//   - Compare the complete cross-product against an independent oracle.
// - Defaults:
//   - Unsupported capability/result combinations produce no disposition.
//
use atrament_derived_output_result::{
    DerivedOutputEffectDisposition, DerivedOutputOperation,
    DerivedOutputResultClass,
};
use atrament_mcp_capability_effect::McpApplicationCapabilityClass;
use atrament_mcp_derived_output_result_projection::{
    mcp_derived_output_effect_disposition, mcp_derived_output_operation,
};

const CAPABILITIES: [McpApplicationCapabilityClass; 8] = [
    McpApplicationCapabilityClass::Apply,
    McpApplicationCapabilityClass::CommandContext,
    McpApplicationCapabilityClass::Export,
    McpApplicationCapabilityClass::HistoryTraversal,
    McpApplicationCapabilityClass::Inspect,
    McpApplicationCapabilityClass::Plan,
    McpApplicationCapabilityClass::Render,
    McpApplicationCapabilityClass::Validate,
];

const RESULTS: [DerivedOutputResultClass; 11] = [
    DerivedOutputResultClass::CancelledBeforeResultOrEffect,
    DerivedOutputResultClass::CapabilityOrValidationRejection,
    DerivedOutputResultClass::CompletedProjection,
    DerivedOutputResultClass::Exported,
    DerivedOutputResultClass::ExportOverwriteConflict,
    DerivedOutputResultClass::ExportPathRejection,
    DerivedOutputResultClass::ExportRetryConflict,
    DerivedOutputResultClass::ExternalTargetDriftConflict,
    DerivedOutputResultClass::IdempotentExportReplay,
    DerivedOutputResultClass::InternalFailureKnownNoEffect,
    DerivedOutputResultClass::StaleOrUnavailableRevision,
];

fn expected_operation(
    capability: McpApplicationCapabilityClass,
) -> Option<DerivedOutputOperation> {
    match capability {
        McpApplicationCapabilityClass::Export => {
            Some(DerivedOutputOperation::Export)
        },
        McpApplicationCapabilityClass::Plan => {
            Some(DerivedOutputOperation::Plan)
        },
        McpApplicationCapabilityClass::Render => {
            Some(DerivedOutputOperation::Render)
        },
        _ => None,
    }
}

fn expected_disposition(
    operation: DerivedOutputOperation,
    result: DerivedOutputResultClass,
) -> Option<DerivedOutputEffectDisposition> {
    let common = matches!(
        result,
        DerivedOutputResultClass::CancelledBeforeResultOrEffect
            | DerivedOutputResultClass::CapabilityOrValidationRejection
            | DerivedOutputResultClass::InternalFailureKnownNoEffect
            | DerivedOutputResultClass::StaleOrUnavailableRevision
    );
    if common {
        return Some(DerivedOutputEffectDisposition::KnownNoNewEffect);
    }
    match (operation, result) {
        (
            DerivedOutputOperation::Export,
            DerivedOutputResultClass::Exported,
        ) => Some(DerivedOutputEffectDisposition::CommittedFileThisCall),
        (
            DerivedOutputOperation::Export,
            DerivedOutputResultClass::IdempotentExportReplay,
        ) => Some(DerivedOutputEffectDisposition::RecoveredPriorFileCommit),
        (
            DerivedOutputOperation::Export,
            DerivedOutputResultClass::ExportOverwriteConflict
            | DerivedOutputResultClass::ExportPathRejection
            | DerivedOutputResultClass::ExportRetryConflict
            | DerivedOutputResultClass::ExternalTargetDriftConflict,
        ) => Some(DerivedOutputEffectDisposition::KnownNoNewEffect),
        (
            DerivedOutputOperation::Plan | DerivedOutputOperation::Render,
            DerivedOutputResultClass::CompletedProjection,
        ) => Some(DerivedOutputEffectDisposition::CompletedReadOnlyProjection),
        _ => None,
    }
}

#[test]
fn all_eight_capabilities_have_exact_output_operation_mapping() {
    for capability in CAPABILITIES {
        assert_eq!(
            mcp_derived_output_operation(capability),
            expected_operation(capability),
        );
    }
}

#[test]
fn all_88_capability_result_pairs_match_independent_oracle() {
    let mut cases = 0_usize;
    for capability in CAPABILITIES {
        for result in RESULTS {
            let expected = expected_operation(capability)
                .and_then(|operation| expected_disposition(operation, result));
            assert_eq!(
                mcp_derived_output_effect_disposition(capability, result),
                expected,
                "capability={capability:?} result={result:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 88);
}
