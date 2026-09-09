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
//   - Regression evidence for ordered source-linked capability review.
// - Must-Not:
//   - Perform conversions, omit incompatible sources, render, plan, or control
//     hardware.
// - Allows:
//   - Inputs: Deterministic frozen capabilities and explicit conversion
//     evidence.
//   - Outputs: Assertions over direct, converted, and blocked review statuses.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Conversion execution or blocking diagnostics gain independent fixtures.
// - Merge-When:
//   - Capability review moves into one application compiler harness.
// - Summary:
//   - Proves incompatibilities remain visible until explicitly resolved.
// - Description:
//   - Covers all six matrix families and explicit conversion evidence handling.
// - Usage:
//   - Compile directly against the output-capability-projection domain.
// - Defaults:
//   - No conversion, omission, or Future fallback is inferred.
//
use atrament_output_capability_matrix::CapabilityDisposition;
use atrament_output_capability_matrix::{
    ColorCapability, HandwritingDecorationCapability, HardwareActionCapability,
    ImageTreatmentCapability, OutputMode, PagePaperCapability,
    SemanticCapability,
};
use atrament_output_capability_projection::{
    AcceptedCapabilityConversion, OutputCapability,
    OutputCapabilityProjectionStatus, OutputCapabilityRequest,
    output_capability_disposition, review_output_capabilities,
};

fn conversion(
    choice: &'static str,
) -> AcceptedCapabilityConversion<&'static str, &'static str> {
    AcceptedCapabilityConversion {
        choice,
        provenance: "user-confirmed-conversion",
    }
}

#[test]
fn wrapper_dispatches_to_all_six_frozen_capability_families() {
    let cases = [
        (
            OutputCapability::Color(ColorCapability::OneInkColor),
            CapabilityDisposition::Accept,
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::TitleOutline,
            ),
            CapabilityDisposition::Convert,
        ),
        (
            OutputCapability::HardwareAction(
                HardwareActionCapability::StartPlan,
            ),
            CapabilityDisposition::Accept,
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::OriginalRasterPixels,
            ),
            CapabilityDisposition::Reject,
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::BlankSheet),
            CapabilityDisposition::Accept,
        ),
        (
            OutputCapability::Semantic(SemanticCapability::Paragraph),
            CapabilityDisposition::Accept,
        ),
    ];
    for (capability, expected) in cases {
        assert_eq!(
            output_capability_disposition(capability, OutputMode::Live),
            expected
        );
    }
}

#[test]
fn review_preserves_every_source_and_explicit_conversion_evidence() {
    let projection = review_output_capabilities(
        OutputMode::Live,
        vec![
            OutputCapabilityRequest {
                accepted_conversion: None,
                capability: OutputCapability::Semantic(
                    SemanticCapability::Paragraph,
                ),
                source_identity: "paragraph-1",
            },
            OutputCapabilityRequest {
                accepted_conversion: Some(conversion("photo-to-line-art")),
                capability: OutputCapability::Semantic(
                    SemanticCapability::Photograph,
                ),
                source_identity: "photo-2",
            },
            OutputCapabilityRequest {
                accepted_conversion: None,
                capability: OutputCapability::Semantic(
                    SemanticCapability::LoosePaperNote,
                ),
                source_identity: "note-3",
            },
            OutputCapabilityRequest {
                accepted_conversion: None,
                capability: OutputCapability::HardwareAction(
                    HardwareActionCapability::AutomaticToolChange,
                ),
                source_identity: "tool-change-4",
            },
        ],
    );
    assert_eq!(projection.entries.len(), 4);
    assert_eq!(projection.entries[0].source_identity, "paragraph-1");
    assert_eq!(
        projection.entries[0].status,
        OutputCapabilityProjectionStatus::AcceptedDirect,
    );
    assert_eq!(projection.entries[1].source_identity, "photo-2");
    assert_eq!(
        projection.entries[1].status,
        OutputCapabilityProjectionStatus::Converted,
    );
    let accepted = projection.entries[1].accepted_conversion.as_ref().unwrap();
    assert_eq!(accepted.choice, "photo-to-line-art");
    assert_eq!(accepted.provenance, "user-confirmed-conversion");
    assert_eq!(
        projection.entries[2].status,
        OutputCapabilityProjectionStatus::Rejected,
    );
    assert_eq!(
        projection.entries[3].status,
        OutputCapabilityProjectionStatus::FutureUnavailable,
    );
    assert!(!projection.is_ready());
}

#[test]
fn convert_without_explicit_acceptance_remains_blocked() {
    let projection = review_output_capabilities(
        OutputMode::Live,
        vec![OutputCapabilityRequest::<&str, &str, &str> {
            accepted_conversion: None,
            capability: OutputCapability::Semantic(
                SemanticCapability::Photograph,
            ),
            source_identity: "photo-9",
        }],
    );
    assert_eq!(
        projection.entries[0].status,
        OutputCapabilityProjectionStatus::ConversionRequired,
    );
    assert!(!projection.is_ready());
}

#[test]
fn conversion_cannot_override_reject_future_or_direct_acceptance() {
    let projection = review_output_capabilities(
        OutputMode::Live,
        vec![
            OutputCapabilityRequest {
                accepted_conversion: Some(conversion("remove-shadow")),
                capability: OutputCapability::HandwritingDecoration(
                    HandwritingDecorationCapability::DigitalPaperShadow,
                ),
                source_identity: "shadow-1",
            },
            OutputCapabilityRequest {
                accepted_conversion: Some(conversion("change-tool")),
                capability: OutputCapability::HardwareAction(
                    HardwareActionCapability::AutomaticToolChange,
                ),
                source_identity: "tool-change-2",
            },
            OutputCapabilityRequest {
                accepted_conversion: Some(conversion("rewrite-paragraph")),
                capability: OutputCapability::Semantic(
                    SemanticCapability::Paragraph,
                ),
                source_identity: "paragraph-3",
            },
        ],
    );
    assert_eq!(
        projection.entries[0].status,
        OutputCapabilityProjectionStatus::Rejected,
    );
    assert_eq!(
        projection.entries[1].status,
        OutputCapabilityProjectionStatus::FutureUnavailable,
    );
    assert_eq!(
        projection.entries[2].status,
        OutputCapabilityProjectionStatus::UnexpectedConversion,
    );
    assert!(!projection.is_ready());
}

#[test]
fn projection_is_ready_only_when_every_entry_is_direct_or_converted() {
    let projection = review_output_capabilities(
        OutputMode::Live,
        vec![
            OutputCapabilityRequest {
                accepted_conversion: None,
                capability: OutputCapability::Semantic(
                    SemanticCapability::Paragraph,
                ),
                source_identity: "paragraph-1",
            },
            OutputCapabilityRequest {
                accepted_conversion: Some(conversion("photo-to-line-art")),
                capability: OutputCapability::Semantic(
                    SemanticCapability::Photograph,
                ),
                source_identity: "photo-2",
            },
        ],
    );
    assert!(projection.is_ready());
}
