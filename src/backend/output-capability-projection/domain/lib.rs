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
//   - Transport-neutral Digital/Live capability projection review.
// - Must-Not:
//   - Perform conversions, mutate semantic source, render, compile geometry,
//     emit diagnostics prose, plan motion, or control hardware.
// - Allows:
//   - Inputs: Source-linked frozen capabilities and optional accepted
//     conversion
//     choice/provenance evidence.
//   - Outputs: Ordered direct/converted/blocked review entries for one mode.
//   - Side effects: None.
// - Split-When:
//   - Conversion execution or blocking-diagnostic projection gains independent
//     authority.
// - Merge-When:
//   - Capability review becomes inseparable from one application compiler.
// - Summary:
//   - Enumerates every output capability use before an accepted projection.
// - Description:
//   - Keeps Convert blocked without explicit evidence and Reject/Future
//     blocked.
// - Usage:
//   - Review source capability uses before Render or live Plan compilation.
// - Defaults:
//   - No implicit conversion, omission, replacement, or best-effort fallback.
//

//! Ordered source-linked review over the frozen Digital/Live capability matrix.

use atrament_output_capability_matrix::{
    CapabilityDisposition, ColorCapability, HandwritingDecorationCapability,
    HardwareActionCapability, ImageTreatmentCapability, OutputMode,
    PagePaperCapability, SemanticCapability, color_capability_disposition,
    handwriting_decoration_capability_disposition,
    hardware_action_capability_disposition,
    image_treatment_capability_disposition, page_paper_capability_disposition,
    semantic_capability_disposition,
};

/// One explicit caller/user-accepted conversion plus its provenance evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedCapabilityConversion<Choice, Provenance> {
    /// Caller-owned accepted conversion choice.
    pub choice: Choice,
    /// Caller-owned provenance recording that explicit acceptance.
    pub provenance: Provenance,
}

/// Frozen first-release kind of explicit single-pen live conversion.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LiveConversionKind {
    /// Raster/image content uses an explicitly accepted line-art projection.
    AcceptedLineArtProjection,
    /// Color meaning maps to one calibrated physical ink identity.
    CalibratedInk,
    /// Paper marks are deliberately drawn with the same physical pen.
    DrawWithSamePen,
    /// A highlight becomes an explicit one-pen box.
    HighlightBox,
    /// A highlight becomes explicit one-pen spacing hierarchy.
    HighlightSpacing,
    /// A highlight becomes an admitted one-pen stroke-weight change.
    HighlightStrokeWeight,
    /// A highlight becomes an explicit one-pen underline.
    HighlightUnderline,
    /// Digital tone/opacity resolves to explicit one-pen geometry.
    OnePenGeometry,
    /// Decorative title treatment becomes a sober one-pen title role.
    SoberOnePenTitle,
}

/// One typed live conversion kind plus caller-owned conversion details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveConversionChoice<Details> {
    /// Caller-owned details such as ink identity or projection evidence.
    pub details: Details,
    /// Frozen conversion family admitted for the source capability.
    pub kind: LiveConversionKind,
}

/// One capability from any frozen first-release matrix family.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputCapability {
    /// Color or physical-pen color capability.
    Color(ColorCapability),
    /// Handwriting role or decoration capability.
    HandwritingDecoration(HandwritingDecorationCapability),
    /// Device-neutral or physical hardware-action capability.
    HardwareAction(HardwareActionCapability),
    /// Image source, placement, or treatment capability.
    ImageTreatment(ImageTreatmentCapability),
    /// Page geometry or paper appearance capability.
    PagePaper(PagePaperCapability),
    /// Semantic notebook object capability.
    Semantic(SemanticCapability),
}

/// Ordered projection entries for one capability review.
pub type OutputCapabilityProjectionEntries<Choice, Provenance, SourceIdentity> =
    Vec<OutputCapabilityProjectionEntry<Choice, Provenance, SourceIdentity>>;

/// Complete ordered review for one requested output mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputCapabilityProjection<Choice, Provenance, SourceIdentity> {
    /// Every source capability use in caller-supplied order.
    entries:
        OutputCapabilityProjectionEntries<Choice, Provenance, SourceIdentity>,
    /// Output mode under review.
    mode: OutputMode,
}

impl<Choice, Provenance, SourceIdentity>
    OutputCapabilityProjection<Choice, Provenance, SourceIdentity>
{
    /// Return blocked source entries in original caller order.
    #[must_use]
    pub fn blocking_entries(
        &self,
    ) -> Vec<
        &OutputCapabilityProjectionEntry<Choice, Provenance, SourceIdentity>,
    > {
        self.entries
            .iter()
            .filter(|entry| !entry.status.is_ready())
            .collect()
    }

    /// Return reviewed entries in caller-supplied source order.
    #[must_use]
    pub fn entries(
        &self,
    ) -> &[
        OutputCapabilityProjectionEntry<Choice, Provenance, SourceIdentity>
    ] {
        &self.entries
    }

    /// Whether every source capability use is currently admissible for output.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.entries.iter().all(|entry| entry.status.is_ready())
    }

    /// Return the exact output mode used for this review.
    #[must_use]
    pub const fn mode(&self) -> OutputMode {
        self.mode
    }
}

/// One reviewed source capability use retained in projection order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputCapabilityProjectionEntry<Choice, Provenance, SourceIdentity> {
    /// Optional explicit conversion evidence supplied with this source use.
    pub accepted_conversion:
        Option<AcceptedCapabilityConversion<Choice, Provenance>>,
    /// Frozen capability represented by this source use.
    pub capability: OutputCapability,
    /// Caller-owned semantic/source identity for the originating object.
    pub source_identity: SourceIdentity,
    /// Admissibility result for this exact source capability use.
    pub status: OutputCapabilityProjectionStatus,
}

/// Source-level capability status after applying explicit conversion evidence.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputCapabilityProjectionStatus {
    /// Capability is directly accepted in this mode with no conversion.
    AcceptedDirect,
    /// Capability needs an explicit accepted conversion before output.
    ConversionRequired,
    /// Capability is admitted through explicit conversion evidence.
    Converted,
    /// Capability belongs to a future profile and remains unavailable.
    FutureUnavailable,
    /// Capability is rejected in this first-release output mode.
    Rejected,
    /// A conversion was supplied for a capability already accepted directly.
    UnexpectedConversion,
    /// Explicit conversion evidence names a kind not admitted for this source.
    UnsupportedConversionChoice,
}

impl OutputCapabilityProjectionStatus {
    /// Whether this exact status is ready for the requested output mode.
    #[must_use]
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::AcceptedDirect | Self::Converted)
    }
}

/// One source-linked capability use before output projection review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputCapabilityRequest<Choice, Provenance, SourceIdentity> {
    /// Optional explicit accepted conversion choice and provenance.
    pub accepted_conversion:
        Option<AcceptedCapabilityConversion<Choice, Provenance>>,
    /// Frozen capability required by the source object.
    pub capability: OutputCapability,
    /// Caller-owned semantic/source identity for the originating object.
    pub source_identity: SourceIdentity,
}

/// Ordered source capability requests for one generic review.
pub type OutputCapabilityRequests<Choice, Provenance, SourceIdentity> =
    Vec<OutputCapabilityRequest<Choice, Provenance, SourceIdentity>>;

/// Live projection after typed conversion-choice review.
pub type LiveOutputCapabilityProjection<Details, Provenance, SourceIdentity> =
    OutputCapabilityProjection<
        LiveConversionChoice<Details>,
        Provenance,
        SourceIdentity,
    >;

/// Ordered source requests carrying typed Live conversion choices.
pub type LiveOutputCapabilityRequests<Details, Provenance, SourceIdentity> =
    OutputCapabilityRequests<
        LiveConversionChoice<Details>,
        Provenance,
        SourceIdentity,
    >;

/// Return the frozen matrix disposition for one wrapped capability.
#[must_use]
pub const fn output_capability_disposition(
    capability: OutputCapability,
    mode: OutputMode,
) -> CapabilityDisposition {
    match capability {
        OutputCapability::Color(value) => {
            color_capability_disposition(value, mode)
        }
        OutputCapability::HandwritingDecoration(value) => {
            handwriting_decoration_capability_disposition(value, mode)
        }
        OutputCapability::HardwareAction(value) => {
            hardware_action_capability_disposition(value, mode)
        }
        OutputCapability::ImageTreatment(value) => {
            image_treatment_capability_disposition(value, mode)
        }
        OutputCapability::PagePaper(value) => {
            page_paper_capability_disposition(value, mode)
        }
        OutputCapability::Semantic(value) => {
            semantic_capability_disposition(value, mode)
        }
    }
}

/// Whether this frozen live conversion kind is admitted for the capability.
#[must_use]
pub const fn live_conversion_kind_admitted(
    capability: OutputCapability,
    kind: LiveConversionKind,
) -> bool {
    match capability {
        OutputCapability::HandwritingDecoration(
            HandwritingDecorationCapability::FilledHighlight
            | HandwritingDecorationCapability::MarkerHighlight,
        ) => matches!(
            kind,
            LiveConversionKind::HighlightBox
                | LiveConversionKind::HighlightSpacing
                | LiveConversionKind::HighlightStrokeWeight
                | LiveConversionKind::HighlightUnderline
        ),
        OutputCapability::HandwritingDecoration(
            HandwritingDecorationCapability::DecorativeTitleLayering
            | HandwritingDecorationCapability::TitleOutline,
        ) => matches!(kind, LiveConversionKind::SoberOnePenTitle),
        OutputCapability::Color(
            ColorCapability::ColoredDiagramStrokes
            | ColorCapability::ColoredTitleLayers
            | ColorCapability::FullColorPhotograph
            | ColorCapability::GrayscalePhotograph
            | ColorCapability::MarkerColor
            | ColorCapability::MultipleSimulatedInkColors
            | ColorCapability::TransparentAlpha,
        ) => matches!(kind, LiveConversionKind::CalibratedInk),
        OutputCapability::ImageTreatment(
            ImageTreatmentCapability::AboveTextPlacement
            | ImageTreatmentCapability::BelowTextPlacement
            | ImageTreatmentCapability::ClippedRegionPlacement
            | ImageTreatmentCapability::InlinePlacement
            | ImageTreatmentCapability::JpegSource
            | ImageTreatmentCapability::PngSource
            | ImageTreatmentCapability::WebpSource,
        )
        | OutputCapability::Semantic(
            SemanticCapability::Photograph
            | SemanticCapability::RasterIllustration,
        ) => matches!(kind, LiveConversionKind::AcceptedLineArtProjection),
        OutputCapability::ImageTreatment(ImageTreatmentCapability::Opacity) => {
            matches!(kind, LiveConversionKind::OnePenGeometry)
        }
        OutputCapability::PagePaper(
            PagePaperCapability::BorderGeometry
            | PagePaperCapability::CustomDigitalPaper
            | PagePaperCapability::DottedPaper
            | PagePaperCapability::GridOrRuleGeometry
            | PagePaperCapability::RuledPaper
            | PagePaperCapability::SquaredPaper,
        ) => matches!(kind, LiveConversionKind::DrawWithSamePen),
        OutputCapability::Color(_)
        | OutputCapability::HandwritingDecoration(_)
        | OutputCapability::HardwareAction(_)
        | OutputCapability::ImageTreatment(_)
        | OutputCapability::PagePaper(_)
        | OutputCapability::Semantic(_) => false,
    }
}

/// Review every source capability use without dropping blocked entries.
///
/// Conversion evidence is retained but does not by itself admit a `Convert`
/// row. Mode-specific validation must prove the conversion choice before the
/// projection can become ready.
#[must_use]
pub fn review_output_capabilities<Choice, Provenance, SourceIdentity>(
    mode: OutputMode,
    requests: OutputCapabilityRequests<Choice, Provenance, SourceIdentity>,
) -> OutputCapabilityProjection<Choice, Provenance, SourceIdentity> {
    let entries = requests
        .into_iter()
        .map(|request| {
            let disposition =
                output_capability_disposition(request.capability, mode);
            let status =
                match (disposition, request.accepted_conversion.is_some()) {
                    (CapabilityDisposition::Accept, false) => {
                        OutputCapabilityProjectionStatus::AcceptedDirect
                    }
                    (CapabilityDisposition::Accept, true) => {
                        OutputCapabilityProjectionStatus::UnexpectedConversion
                    }
                    (CapabilityDisposition::Convert, _) => {
                        OutputCapabilityProjectionStatus::ConversionRequired
                    }
                    (CapabilityDisposition::Future, _) => {
                        OutputCapabilityProjectionStatus::FutureUnavailable
                    }
                    (CapabilityDisposition::Reject, _) => {
                        OutputCapabilityProjectionStatus::Rejected
                    }
                };
            OutputCapabilityProjectionEntry {
                accepted_conversion: request.accepted_conversion,
                capability: request.capability,
                source_identity: request.source_identity,
                status,
            }
        })
        .collect();
    OutputCapabilityProjection { entries, mode }
}

/// Review Live output and reject mismatched explicit conversion kinds.
#[must_use]
pub fn review_live_output_capabilities<Details, Provenance, SourceIdentity>(
    requests: LiveOutputCapabilityRequests<Details, Provenance, SourceIdentity>,
) -> LiveOutputCapabilityProjection<Details, Provenance, SourceIdentity> {
    let mut projection = review_output_capabilities(OutputMode::Live, requests);
    for entry in &mut projection.entries {
        if entry.status
            != OutputCapabilityProjectionStatus::ConversionRequired
        {
            continue;
        }
        let Some(conversion) = entry.accepted_conversion.as_ref() else {
            continue;
        };
        entry.status = if live_conversion_kind_admitted(
            entry.capability,
            conversion.choice.kind,
        ) {
            OutputCapabilityProjectionStatus::Converted
        } else {
            OutputCapabilityProjectionStatus::UnsupportedConversionChoice
        };
    }
    projection
}
