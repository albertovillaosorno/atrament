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
    AcceptedCapabilityConversion, LiveConversionChoice, LiveConversionKind,
    OutputCapability, OutputCapabilityProjectionStatus,
    OutputCapabilityRequest, live_conversion_kind_admitted,
    output_capability_disposition, review_live_output_capabilities,
    review_output_capabilities,
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
    assert_eq!(projection.mode(), OutputMode::Live);
    assert_eq!(projection.entries().len(), 4);
    assert_eq!(projection.entries()[0].source_identity, "paragraph-1");
    assert_eq!(
        projection.entries()[0].status,
        OutputCapabilityProjectionStatus::AcceptedDirect,
    );
    assert_eq!(projection.entries()[1].source_identity, "photo-2");
    assert_eq!(
        projection.entries()[1].status,
        OutputCapabilityProjectionStatus::ConversionRequired,
    );
    let accepted = projection.entries()[1]
        .accepted_conversion
        .as_ref()
        .unwrap();
    assert_eq!(accepted.choice, "photo-to-line-art");
    assert_eq!(accepted.provenance, "user-confirmed-conversion");
    assert_eq!(
        projection.entries()[2].status,
        OutputCapabilityProjectionStatus::Rejected,
    );
    assert_eq!(
        projection.entries()[3].status,
        OutputCapabilityProjectionStatus::FutureUnavailable,
    );
    assert!(!projection.is_ready());
}

#[test]
fn blocking_projection_preserves_every_non_ready_source_in_order() {
    let projection = review_output_capabilities(
        OutputMode::Live,
        vec![
            OutputCapabilityRequest::<&str, &str, &str> {
                accepted_conversion: None,
                capability: OutputCapability::Semantic(
                    SemanticCapability::Paragraph,
                ),
                source_identity: "ready-direct",
            },
            OutputCapabilityRequest {
                accepted_conversion: None,
                capability: OutputCapability::Semantic(
                    SemanticCapability::Photograph,
                ),
                source_identity: "needs-conversion",
            },
            OutputCapabilityRequest {
                accepted_conversion: None,
                capability: OutputCapability::Semantic(
                    SemanticCapability::LoosePaperNote,
                ),
                source_identity: "rejected",
            },
            OutputCapabilityRequest {
                accepted_conversion: None,
                capability: OutputCapability::HardwareAction(
                    HardwareActionCapability::AutomaticToolChange,
                ),
                source_identity: "future",
            },
        ],
    );
    let blocked = projection.blocking_entries();
    assert_eq!(blocked.len(), 3);
    assert_eq!(blocked[0].source_identity, "needs-conversion");
    assert_eq!(blocked[1].source_identity, "rejected");
    assert_eq!(blocked[2].source_identity, "future");
    assert!(blocked.iter().all(|entry| !entry.status.is_ready()));
}

#[test]
fn readiness_predicate_covers_all_projection_statuses() {
    let cases = [
        (OutputCapabilityProjectionStatus::AcceptedDirect, true),
        (OutputCapabilityProjectionStatus::ConversionRequired, false),
        (OutputCapabilityProjectionStatus::Converted, true),
        (OutputCapabilityProjectionStatus::FutureUnavailable, false),
        (OutputCapabilityProjectionStatus::Rejected, false),
        (OutputCapabilityProjectionStatus::UnexpectedConversion, false),
        (
            OutputCapabilityProjectionStatus::UnsupportedConversionChoice,
            false,
        ),
    ];
    for (status, expected) in cases {
        assert_eq!(status.is_ready(), expected, "status {status:?}");
    }
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
        projection.entries()[0].status,
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
        projection.entries()[0].status,
        OutputCapabilityProjectionStatus::Rejected,
    );
    assert_eq!(
        projection.entries()[1].status,
        OutputCapabilityProjectionStatus::FutureUnavailable,
    );
    assert_eq!(
        projection.entries()[2].status,
        OutputCapabilityProjectionStatus::UnexpectedConversion,
    );
    assert!(!projection.is_ready());
}

#[test]
fn generic_live_review_cannot_admit_unvalidated_conversion_evidence() {
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
                accepted_conversion: Some(conversion("arbitrary-choice")),
                capability: OutputCapability::Semantic(
                    SemanticCapability::Photograph,
                ),
                source_identity: "photo-2",
            },
        ],
    );
    assert_eq!(
        projection.entries()[1].status,
        OutputCapabilityProjectionStatus::ConversionRequired,
    );
    assert!(!projection.is_ready());
}

fn live_conversion(
    kind: LiveConversionKind,
    details: &'static str,
) -> AcceptedCapabilityConversion<
    LiveConversionChoice<&'static str>,
    &'static str,
> {
    AcceptedCapabilityConversion {
        choice: LiveConversionChoice { details, kind },
        provenance: "user-confirmed-live-conversion",
    }
}

#[test]
fn every_live_convert_row_has_a_frozen_supported_conversion_kind() {
    let cases = [
        (
            OutputCapability::Semantic(SemanticCapability::Photograph),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::Semantic(SemanticCapability::RasterIllustration),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::MarkerHighlight,
            ),
            LiveConversionKind::HighlightUnderline,
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::FilledHighlight,
            ),
            LiveConversionKind::HighlightBox,
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::DecorativeTitleLayering,
            ),
            LiveConversionKind::SoberOnePenTitle,
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::TitleOutline,
            ),
            LiveConversionKind::SoberOnePenTitle,
        ),
        (
            OutputCapability::Color(
                ColorCapability::MultipleSimulatedInkColors,
            ),
            LiveConversionKind::CalibratedInk,
        ),
        (
            OutputCapability::Color(ColorCapability::MarkerColor),
            LiveConversionKind::CalibratedInk,
        ),
        (
            OutputCapability::Color(ColorCapability::ColoredTitleLayers),
            LiveConversionKind::CalibratedInk,
        ),
        (
            OutputCapability::Color(ColorCapability::ColoredDiagramStrokes),
            LiveConversionKind::CalibratedInk,
        ),
        (
            OutputCapability::Color(ColorCapability::FullColorPhotograph),
            LiveConversionKind::CalibratedInk,
        ),
        (
            OutputCapability::Color(ColorCapability::GrayscalePhotograph),
            LiveConversionKind::CalibratedInk,
        ),
        (
            OutputCapability::Color(ColorCapability::TransparentAlpha),
            LiveConversionKind::CalibratedInk,
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::PngSource,
            ),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::JpegSource,
            ),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::WebpSource,
            ),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::BelowTextPlacement,
            ),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::InlinePlacement,
            ),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::AboveTextPlacement,
            ),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::ClippedRegionPlacement,
            ),
            LiveConversionKind::AcceptedLineArtProjection,
        ),
        (
            OutputCapability::ImageTreatment(ImageTreatmentCapability::Opacity),
            LiveConversionKind::OnePenGeometry,
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::RuledPaper),
            LiveConversionKind::DrawWithSamePen,
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::DottedPaper),
            LiveConversionKind::DrawWithSamePen,
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::SquaredPaper),
            LiveConversionKind::DrawWithSamePen,
        ),
        (
            OutputCapability::PagePaper(
                PagePaperCapability::CustomDigitalPaper,
            ),
            LiveConversionKind::DrawWithSamePen,
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::BorderGeometry),
            LiveConversionKind::DrawWithSamePen,
        ),
        (
            OutputCapability::PagePaper(
                PagePaperCapability::GridOrRuleGeometry,
            ),
            LiveConversionKind::DrawWithSamePen,
        ),
    ];
    assert_eq!(cases.len(), 27);
    for (capability, kind) in cases {
        assert_eq!(
            output_capability_disposition(capability, OutputMode::Live),
            CapabilityDisposition::Convert,
        );
        assert!(live_conversion_kind_admitted(capability, kind));
    }
}

#[test]
fn highlight_conversion_kind_is_limited_to_four_frozen_one_pen_choices() {
    let capability = OutputCapability::HandwritingDecoration(
        HandwritingDecorationCapability::MarkerHighlight,
    );
    for kind in [
        LiveConversionKind::HighlightBox,
        LiveConversionKind::HighlightSpacing,
        LiveConversionKind::HighlightStrokeWeight,
        LiveConversionKind::HighlightUnderline,
    ] {
        assert!(live_conversion_kind_admitted(capability, kind));
    }
    assert!(!live_conversion_kind_admitted(
        capability,
        LiveConversionKind::CalibratedInk,
    ));
}

#[test]
fn live_review_keeps_supported_conversion_details_and_provenance() {
    let projection =
        review_live_output_capabilities(vec![OutputCapabilityRequest {
            accepted_conversion: Some(live_conversion(
                LiveConversionKind::AcceptedLineArtProjection,
                "line-art-projection-17",
            )),
            capability: OutputCapability::Semantic(
                SemanticCapability::Photograph,
            ),
            source_identity: "photo-7",
        }]);
    assert!(projection.is_ready());
    assert_eq!(
        projection.entries()[0].status,
        OutputCapabilityProjectionStatus::Converted,
    );
    let accepted = projection.entries()[0]
        .accepted_conversion
        .as_ref()
        .unwrap();
    assert_eq!(accepted.choice.details, "line-art-projection-17");
    assert_eq!(accepted.provenance, "user-confirmed-live-conversion",);
}

#[test]
fn live_review_blocks_explicit_but_mismatched_conversion_kind() {
    let projection =
        review_live_output_capabilities(vec![OutputCapabilityRequest {
            accepted_conversion: Some(live_conversion(
                LiveConversionKind::SoberOnePenTitle,
                "wrong-kind-for-photo",
            )),
            capability: OutputCapability::Semantic(
                SemanticCapability::Photograph,
            ),
            source_identity: "photo-8",
        }]);
    assert_eq!(
        projection.entries()[0].status,
        OutputCapabilityProjectionStatus::UnsupportedConversionChoice,
    );
    assert!(!projection.is_ready());
}

#[test]
fn every_live_convert_row_admits_only_its_frozen_conversion_kinds() {
    const ALL_KINDS: [LiveConversionKind; 9] = [
        LiveConversionKind::AcceptedLineArtProjection,
        LiveConversionKind::CalibratedInk,
        LiveConversionKind::DrawWithSamePen,
        LiveConversionKind::HighlightBox,
        LiveConversionKind::HighlightSpacing,
        LiveConversionKind::HighlightStrokeWeight,
        LiveConversionKind::HighlightUnderline,
        LiveConversionKind::OnePenGeometry,
        LiveConversionKind::SoberOnePenTitle,
    ];
    let cases: [(OutputCapability, &[LiveConversionKind]); 27] = [
        (
            OutputCapability::Semantic(SemanticCapability::Photograph),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::Semantic(SemanticCapability::RasterIllustration),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::MarkerHighlight,
            ),
            &[
                LiveConversionKind::HighlightBox,
                LiveConversionKind::HighlightSpacing,
                LiveConversionKind::HighlightStrokeWeight,
                LiveConversionKind::HighlightUnderline,
            ],
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::FilledHighlight,
            ),
            &[
                LiveConversionKind::HighlightBox,
                LiveConversionKind::HighlightSpacing,
                LiveConversionKind::HighlightStrokeWeight,
                LiveConversionKind::HighlightUnderline,
            ],
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::DecorativeTitleLayering,
            ),
            &[LiveConversionKind::SoberOnePenTitle],
        ),
        (
            OutputCapability::HandwritingDecoration(
                HandwritingDecorationCapability::TitleOutline,
            ),
            &[LiveConversionKind::SoberOnePenTitle],
        ),
        (
            OutputCapability::Color(
                ColorCapability::MultipleSimulatedInkColors,
            ),
            &[LiveConversionKind::CalibratedInk],
        ),
        (
            OutputCapability::Color(ColorCapability::MarkerColor),
            &[LiveConversionKind::CalibratedInk],
        ),
        (
            OutputCapability::Color(ColorCapability::ColoredTitleLayers),
            &[LiveConversionKind::CalibratedInk],
        ),
        (
            OutputCapability::Color(ColorCapability::ColoredDiagramStrokes),
            &[LiveConversionKind::CalibratedInk],
        ),
        (
            OutputCapability::Color(ColorCapability::FullColorPhotograph),
            &[LiveConversionKind::CalibratedInk],
        ),
        (
            OutputCapability::Color(ColorCapability::GrayscalePhotograph),
            &[LiveConversionKind::CalibratedInk],
        ),
        (
            OutputCapability::Color(ColorCapability::TransparentAlpha),
            &[LiveConversionKind::CalibratedInk],
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::PngSource,
            ),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::JpegSource,
            ),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::WebpSource,
            ),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::BelowTextPlacement,
            ),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::InlinePlacement,
            ),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::AboveTextPlacement,
            ),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::ImageTreatment(
                ImageTreatmentCapability::ClippedRegionPlacement,
            ),
            &[LiveConversionKind::AcceptedLineArtProjection],
        ),
        (
            OutputCapability::ImageTreatment(ImageTreatmentCapability::Opacity),
            &[LiveConversionKind::OnePenGeometry],
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::RuledPaper),
            &[LiveConversionKind::DrawWithSamePen],
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::DottedPaper),
            &[LiveConversionKind::DrawWithSamePen],
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::SquaredPaper),
            &[LiveConversionKind::DrawWithSamePen],
        ),
        (
            OutputCapability::PagePaper(
                PagePaperCapability::CustomDigitalPaper,
            ),
            &[LiveConversionKind::DrawWithSamePen],
        ),
        (
            OutputCapability::PagePaper(PagePaperCapability::BorderGeometry),
            &[LiveConversionKind::DrawWithSamePen],
        ),
        (
            OutputCapability::PagePaper(
                PagePaperCapability::GridOrRuleGeometry,
            ),
            &[LiveConversionKind::DrawWithSamePen],
        ),
    ];

    for (capability, expected) in cases {
        assert_eq!(
            output_capability_disposition(capability, OutputMode::Live),
            CapabilityDisposition::Convert,
        );
        let admitted = ALL_KINDS
            .into_iter()
            .filter(|kind| live_conversion_kind_admitted(capability, *kind))
            .collect::<Vec<_>>();
        assert_eq!(
            admitted,
            expected,
            "unexpected conversion set for {capability:?}",
        );
    }
}
