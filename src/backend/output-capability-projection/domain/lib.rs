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

/// Complete ordered review for one requested output mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputCapabilityProjection<Choice, Provenance, SourceIdentity> {
    /// Every source capability use in caller-supplied order.
    pub entries: Vec<
        OutputCapabilityProjectionEntry<Choice, Provenance, SourceIdentity>,
    >,
    /// Output mode under review.
    pub mode: OutputMode,
}

impl<Choice, Provenance, SourceIdentity>
    OutputCapabilityProjection<Choice, Provenance, SourceIdentity>
{
    /// Whether every source capability use is currently admissible for output.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.entries.iter().all(|entry| {
            matches!(
                entry.status,
                OutputCapabilityProjectionStatus::AcceptedDirect
                    | OutputCapabilityProjectionStatus::Converted
            )
        })
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

/// Review every source capability use without dropping blocked entries.
#[must_use]
pub fn review_output_capabilities<Choice, Provenance, SourceIdentity>(
    mode: OutputMode,
    requests: Vec<OutputCapabilityRequest<Choice, Provenance, SourceIdentity>>,
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
                    (CapabilityDisposition::Convert, false) => {
                        OutputCapabilityProjectionStatus::ConversionRequired
                    }
                    (CapabilityDisposition::Convert, true) => {
                        OutputCapabilityProjectionStatus::Converted
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
