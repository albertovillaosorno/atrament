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
//   - Exhaustive first-release Digital/Live capability dispositions.
// - Must-Not:
//   - Perform conversions, mutate semantic source, compile geometry, control
//     hardware, choose devices, or treat Future as a fallback.
// - Allows:
//   - Inputs: One frozen first-release capability and output mode.
//   - Outputs: Exact Accept/Convert/Reject/Future disposition.
//   - Side effects: None.
// - Split-When:
//   - Conversion planning or capability diagnostics gains executable authority.
// - Merge-When:
//   - Matrix lookup becomes inseparable from one capability compiler service.
// - Summary:
//   - Prevents unsupported output features from disappearing or changing mode.
// - Description:
//   - Implements every row of the frozen Digital/Live first-release matrix.
// - Usage:
//   - Query before compilation; Convert stays blocked until explicitly chosen.
// - Defaults:
//   - No missing row, implicit conversion, or best-effort fallback is allowed.
//

//! Exhaustive first-release Digital/Live output capability matrix.

/// Frozen disposition for one capability in one output mode.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CapabilityDisposition {
    /// Preserve requested semantics directly in this mode.
    Accept,
    /// Require an explicit recorded conversion before acceptance.
    Convert,
    /// Reserved for a later capability profile, never a fallback.
    Future,
    /// Block the mode for the capability as requested.
    Reject,
}

/// Color and physical-pen color capabilities.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ColorCapability {
    /// Frozen capability row `AutomaticPhysicalPenChange`.
    AutomaticPhysicalPenChange,
    /// Frozen capability row `ColoredDiagramStrokes`.
    ColoredDiagramStrokes,
    /// Frozen capability row `ColoredTitleLayers`.
    ColoredTitleLayers,
    /// Frozen capability row `FullColorPhotograph`.
    FullColorPhotograph,
    /// Frozen capability row `GrayscalePhotograph`.
    GrayscalePhotograph,
    /// Frozen capability row `MarkerColor`.
    MarkerColor,
    /// Frozen capability row `MultiplePhysicalPenColors`.
    MultiplePhysicalPenColors,
    /// Frozen capability row `MultipleSimulatedInkColors`.
    MultipleSimulatedInkColors,
    /// Frozen capability row `OneInkColor`.
    OneInkColor,
    /// Frozen capability row `TransparentAlpha`.
    TransparentAlpha,
}

/// Handwriting roles and decoration styles.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HandwritingDecorationCapability {
    /// Frozen capability row `AnnotationHandwritingRole`.
    AnnotationHandwritingRole,
    /// Frozen capability row `BodyHandwritingRole`.
    BodyHandwritingRole,
    /// Frozen capability row `BoundedBaselineDrift`.
    BoundedBaselineDrift,
    /// Frozen capability row `BoundedRulerError`.
    BoundedRulerError,
    /// Frozen capability row `CaptionHandwritingRole`.
    CaptionHandwritingRole,
    /// Frozen capability row `DecorativeTitleLayering`.
    DecorativeTitleLayering,
    /// Frozen capability row `DigitalPaperShadow`.
    DigitalPaperShadow,
    /// Frozen capability row `FilledHighlight`.
    FilledHighlight,
    /// Frozen capability row `FormulaHandwritingRole`.
    FormulaHandwritingRole,
    /// Frozen capability row `HandDrawnLine`.
    HandDrawnLine,
    /// Frozen capability row `LabelHandwritingRole`.
    LabelHandwritingRole,
    /// Frozen capability row `LooseNoteFold`.
    LooseNoteFold,
    /// Frozen capability row `MarginHandwritingRole`.
    MarginHandwritingRole,
    /// Frozen capability row `MarkerHighlight`.
    MarkerHighlight,
    /// Frozen capability row `RulerLikeLine`.
    RulerLikeLine,
    /// Frozen capability row `SimulatedShadow`.
    SimulatedShadow,
    /// Frozen capability row `SubtitleHandwritingRole`.
    SubtitleHandwritingRole,
    /// Frozen capability row `TexturedPaperFill`.
    TexturedPaperFill,
    /// Frozen capability row `TitleHandwritingRole`.
    TitleHandwritingRole,
    /// Frozen capability row `TitleOutline`.
    TitleOutline,
}

/// Device-neutral and physical hardware actions.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HardwareActionCapability {
    /// Frozen capability row `AccelerationValue`.
    AccelerationValue,
    /// Frozen capability row `ArmDevice`.
    ArmDevice,
    /// Frozen capability row `AutomaticToolChange`.
    AutomaticToolChange,
    /// Frozen capability row `CalibratedPressure`.
    CalibratedPressure,
    /// Frozen capability row `CancelPlan`.
    CancelPlan,
    /// Frozen capability row `Checkpoint`.
    Checkpoint,
    /// Frozen capability row `ConnectDevice`.
    ConnectDevice,
    /// Frozen capability row `EstimatedDuration`.
    EstimatedDuration,
    /// Frozen capability row `HomeDevice`.
    HomeDevice,
    /// Frozen capability row `IdentifyDevice`.
    IdentifyDevice,
    /// Frozen capability row `MultipleActivePens`.
    MultipleActivePens,
    /// Frozen capability row `Pause`.
    Pause,
    /// Frozen capability row `PausePlan`.
    PausePlan,
    /// Frozen capability row `PenDownStroke`.
    PenDownStroke,
    /// Frozen capability row `PenUpTravel`.
    PenUpTravel,
    /// Frozen capability row `PressureProxy`.
    PressureProxy,
    /// Frozen capability row `RasterPrintingAction`.
    RasterPrintingAction,
    /// Frozen capability row `ResumeKnownState`.
    ResumeKnownState,
    /// Frozen capability row `ResumeUncertainState`.
    ResumeUncertainState,
    /// Frozen capability row `SafeBounds`.
    SafeBounds,
    /// Frozen capability row `SafeStop`.
    SafeStop,
    /// Frozen capability row `SpeedValue`.
    SpeedValue,
    /// Frozen capability row `StartPlan`.
    StartPlan,
}

/// Image-source and placement/treatment capabilities.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ImageTreatmentCapability {
    /// Frozen capability row `AboveTextPlacement`.
    AboveTextPlacement,
    /// Frozen capability row `BelowTextPlacement`.
    BelowTextPlacement,
    /// Frozen capability row `ClippedRegionPlacement`.
    ClippedRegionPlacement,
    /// Frozen capability row `ConfigurableLineArtLevels`.
    ConfigurableLineArtLevels,
    /// Frozen capability row `Crop`.
    Crop,
    /// Frozen capability row `InlinePlacement`.
    InlinePlacement,
    /// Frozen capability row `JpegSource`.
    JpegSource,
    /// Frozen capability row `Opacity`.
    Opacity,
    /// Frozen capability row `OriginalRasterPixels`.
    OriginalRasterPixels,
    /// Frozen capability row `PngSource`.
    PngSource,
    /// Frozen capability row `Position`.
    Position,
    /// Frozen capability row `Scale`.
    Scale,
    /// Frozen capability row `SingleColorLineArt`.
    SingleColorLineArt,
    /// Frozen capability row `SourceIdentity`.
    SourceIdentity,
    /// Frozen capability row `WebpSource`.
    WebpSource,
    /// Frozen capability row `ZOrder`.
    ZOrder,
}

/// Page geometry and paper appearance capabilities.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PagePaperCapability {
    /// Frozen capability row `BindingEdge`.
    BindingEdge,
    /// Frozen capability row `BlankSheet`.
    BlankSheet,
    /// Frozen capability row `BorderGeometry`.
    BorderGeometry,
    /// Frozen capability row `CustomDigitalPaper`.
    CustomDigitalPaper,
    /// Frozen capability row `DottedPaper`.
    DottedPaper,
    /// Frozen capability row `GridOrRuleGeometry`.
    GridOrRuleGeometry,
    /// Frozen capability row `Orientation`.
    Orientation,
    /// Frozen capability row `OuterMargin`.
    OuterMargin,
    /// Frozen capability row `PageBackgroundColor`.
    PageBackgroundColor,
    /// Frozen capability row `PaperTexture`.
    PaperTexture,
    /// Frozen capability row `PrintableRegion`.
    PrintableRegion,
    /// Frozen capability row `RuledPaper`.
    RuledPaper,
    /// Frozen capability row `SheetSize`.
    SheetSize,
    /// Frozen capability row `SimulatedLooseSheet`.
    SimulatedLooseSheet,
    /// Frozen capability row `SquaredPaper`.
    SquaredPaper,
    /// Frozen capability row `TopClearance`.
    TopClearance,
    /// Frozen capability row `WritingInset`.
    WritingInset,
}

/// Semantic blocks and semantic output objects.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticCapability {
    /// Frozen capability row `AlignedMathematics`.
    AlignedMathematics,
    /// Frozen capability row `Arrow`.
    Arrow,
    /// Frozen capability row `Box`.
    Box,
    /// Frozen capability row `Callout`.
    Callout,
    /// Frozen capability row `Citation`.
    Citation,
    /// Frozen capability row `Date`.
    Date,
    /// Frozen capability row `Definition`.
    Definition,
    /// Frozen capability row `DisplayedMathematics`.
    DisplayedMathematics,
    /// Frozen capability row `Divider`.
    Divider,
    /// Frozen capability row `Footnote`.
    Footnote,
    /// Frozen capability row `FreeformRegion`.
    FreeformRegion,
    /// Frozen capability row `Heading`.
    Heading,
    /// Frozen capability row `InlineMathematics`.
    InlineMathematics,
    /// Frozen capability row `LoosePaperNote`.
    LoosePaperNote,
    /// Frozen capability row `MarginNote`.
    MarginNote,
    /// Frozen capability row `MatrixMathematics`.
    MatrixMathematics,
    /// Frozen capability row `MergedTableCells`.
    MergedTableCells,
    /// Frozen capability row `OrderedList`.
    OrderedList,
    /// Frozen capability row `PageReference`.
    PageReference,
    /// Frozen capability row `Paragraph`.
    Paragraph,
    /// Frozen capability row `Photograph`.
    Photograph,
    /// Frozen capability row `Quotation`.
    Quotation,
    /// Frozen capability row `RasterIllustration`.
    RasterIllustration,
    /// Frozen capability row `SemanticDiagram`.
    SemanticDiagram,
    /// Frozen capability row `SourceNote`.
    SourceNote,
    /// Frozen capability row `Table`.
    Table,
    /// Frozen capability row `TextLabel`.
    TextLabel,
    /// Frozen capability row `UnitsInMathematics`.
    UnitsInMathematics,
    /// Frozen capability row `UnorderedList`.
    UnorderedList,
    /// Frozen capability row `UnresolvedClaim`.
    UnresolvedClaim,
    /// Frozen capability row `UnresolvedUnsupportedBlock`.
    UnresolvedUnsupportedBlock,
    /// Frozen capability row `VectorLineArt`.
    VectorLineArt,
}

/// First-release output modes covered by the capability matrix.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputMode {
    /// Digital render/export capability.
    Digital,
    /// Initial physical single-pen live capability.
    Live,
}

/// Return the frozen disposition for this capability family.
#[must_use]
pub const fn color_capability_disposition(
    capability: ColorCapability,
    mode: OutputMode,
) -> CapabilityDisposition {
    use CapabilityDisposition as Disposition;
    use ColorCapability as Capability;
    match mode {
        OutputMode::Digital => match capability {
            Capability::ColoredDiagramStrokes
            | Capability::ColoredTitleLayers
            | Capability::FullColorPhotograph
            | Capability::GrayscalePhotograph
            | Capability::MarkerColor
            | Capability::MultipleSimulatedInkColors
            | Capability::OneInkColor
            | Capability::TransparentAlpha
            => Disposition::Accept,
            Capability::AutomaticPhysicalPenChange
            | Capability::MultiplePhysicalPenColors
            => Disposition::Reject,
        },
        OutputMode::Live => match capability {
            Capability::OneInkColor
            => Disposition::Accept,
            Capability::ColoredDiagramStrokes
            | Capability::ColoredTitleLayers
            | Capability::FullColorPhotograph
            | Capability::GrayscalePhotograph
            | Capability::MarkerColor
            | Capability::MultipleSimulatedInkColors
            | Capability::TransparentAlpha
            => Disposition::Convert,
            Capability::AutomaticPhysicalPenChange
            | Capability::MultiplePhysicalPenColors
            => Disposition::Future,
        },
    }
}

/// Return the frozen disposition for this capability family.
#[must_use]
pub const fn handwriting_decoration_capability_disposition(
    capability: HandwritingDecorationCapability,
    mode: OutputMode,
) -> CapabilityDisposition {
    use CapabilityDisposition as Disposition;
    use HandwritingDecorationCapability as Capability;
    match mode {
        OutputMode::Digital => match capability {
            Capability::AnnotationHandwritingRole
            | Capability::BodyHandwritingRole
            | Capability::BoundedBaselineDrift
            | Capability::BoundedRulerError
            | Capability::CaptionHandwritingRole
            | Capability::DecorativeTitleLayering
            | Capability::DigitalPaperShadow
            | Capability::FilledHighlight
            | Capability::FormulaHandwritingRole
            | Capability::HandDrawnLine
            | Capability::LabelHandwritingRole
            | Capability::LooseNoteFold
            | Capability::MarginHandwritingRole
            | Capability::MarkerHighlight
            | Capability::RulerLikeLine
            | Capability::SimulatedShadow
            | Capability::SubtitleHandwritingRole
            | Capability::TexturedPaperFill
            | Capability::TitleHandwritingRole
            | Capability::TitleOutline
            => Disposition::Accept,
        },
        OutputMode::Live => match capability {
            Capability::AnnotationHandwritingRole
            | Capability::BodyHandwritingRole
            | Capability::BoundedBaselineDrift
            | Capability::BoundedRulerError
            | Capability::CaptionHandwritingRole
            | Capability::FormulaHandwritingRole
            | Capability::HandDrawnLine
            | Capability::LabelHandwritingRole
            | Capability::MarginHandwritingRole
            | Capability::RulerLikeLine
            | Capability::SubtitleHandwritingRole
            | Capability::TitleHandwritingRole
            => Disposition::Accept,
            Capability::DecorativeTitleLayering
            | Capability::FilledHighlight
            | Capability::MarkerHighlight
            | Capability::TitleOutline
            => Disposition::Convert,
            Capability::DigitalPaperShadow
            | Capability::LooseNoteFold
            | Capability::SimulatedShadow
            | Capability::TexturedPaperFill
            => Disposition::Reject,
        },
    }
}

/// Return the frozen disposition for this capability family.
#[must_use]
pub const fn hardware_action_capability_disposition(
    capability: HardwareActionCapability,
    mode: OutputMode,
) -> CapabilityDisposition {
    use CapabilityDisposition as Disposition;
    use HardwareActionCapability as Capability;
    match mode {
        OutputMode::Digital => match capability {
            Capability::AccelerationValue
            | Capability::ArmDevice
            | Capability::AutomaticToolChange
            | Capability::CalibratedPressure
            | Capability::CancelPlan
            | Capability::Checkpoint
            | Capability::ConnectDevice
            | Capability::EstimatedDuration
            | Capability::HomeDevice
            | Capability::IdentifyDevice
            | Capability::MultipleActivePens
            | Capability::Pause
            | Capability::PausePlan
            | Capability::PenDownStroke
            | Capability::PenUpTravel
            | Capability::PressureProxy
            | Capability::RasterPrintingAction
            | Capability::ResumeKnownState
            | Capability::ResumeUncertainState
            | Capability::SafeBounds
            | Capability::SafeStop
            | Capability::SpeedValue
            | Capability::StartPlan
            => Disposition::Reject,
        },
        OutputMode::Live => match capability {
            Capability::AccelerationValue
            | Capability::ArmDevice
            | Capability::CalibratedPressure
            | Capability::CancelPlan
            | Capability::Checkpoint
            | Capability::ConnectDevice
            | Capability::EstimatedDuration
            | Capability::HomeDevice
            | Capability::IdentifyDevice
            | Capability::Pause
            | Capability::PausePlan
            | Capability::PenDownStroke
            | Capability::PenUpTravel
            | Capability::PressureProxy
            | Capability::ResumeKnownState
            | Capability::SafeBounds
            | Capability::SafeStop
            | Capability::SpeedValue
            | Capability::StartPlan
            => Disposition::Accept,
            Capability::AutomaticToolChange
            | Capability::MultipleActivePens
            => Disposition::Future,
            Capability::RasterPrintingAction
            | Capability::ResumeUncertainState
            => Disposition::Reject,
        },
    }
}

/// Return the frozen disposition for this capability family.
#[must_use]
pub const fn image_treatment_capability_disposition(
    capability: ImageTreatmentCapability,
    mode: OutputMode,
) -> CapabilityDisposition {
    use CapabilityDisposition as Disposition;
    use ImageTreatmentCapability as Capability;
    match mode {
        OutputMode::Digital => match capability {
            Capability::AboveTextPlacement
            | Capability::BelowTextPlacement
            | Capability::ClippedRegionPlacement
            | Capability::ConfigurableLineArtLevels
            | Capability::Crop
            | Capability::InlinePlacement
            | Capability::JpegSource
            | Capability::Opacity
            | Capability::OriginalRasterPixels
            | Capability::PngSource
            | Capability::Position
            | Capability::Scale
            | Capability::SingleColorLineArt
            | Capability::SourceIdentity
            | Capability::WebpSource
            | Capability::ZOrder
            => Disposition::Accept,
        },
        OutputMode::Live => match capability {
            Capability::ConfigurableLineArtLevels
            | Capability::Crop
            | Capability::Position
            | Capability::Scale
            | Capability::SingleColorLineArt
            | Capability::SourceIdentity
            | Capability::ZOrder
            => Disposition::Accept,
            Capability::AboveTextPlacement
            | Capability::BelowTextPlacement
            | Capability::ClippedRegionPlacement
            | Capability::InlinePlacement
            | Capability::JpegSource
            | Capability::Opacity
            | Capability::PngSource
            | Capability::WebpSource
            => Disposition::Convert,
            Capability::OriginalRasterPixels
            => Disposition::Reject,
        },
    }
}

/// Return the frozen disposition for this capability family.
#[must_use]
pub const fn page_paper_capability_disposition(
    capability: PagePaperCapability,
    mode: OutputMode,
) -> CapabilityDisposition {
    use CapabilityDisposition as Disposition;
    use PagePaperCapability as Capability;
    match mode {
        OutputMode::Digital => match capability {
            Capability::BindingEdge
            | Capability::BlankSheet
            | Capability::BorderGeometry
            | Capability::CustomDigitalPaper
            | Capability::DottedPaper
            | Capability::GridOrRuleGeometry
            | Capability::Orientation
            | Capability::OuterMargin
            | Capability::PageBackgroundColor
            | Capability::PaperTexture
            | Capability::PrintableRegion
            | Capability::RuledPaper
            | Capability::SheetSize
            | Capability::SimulatedLooseSheet
            | Capability::SquaredPaper
            | Capability::TopClearance
            | Capability::WritingInset
            => Disposition::Accept,
        },
        OutputMode::Live => match capability {
            Capability::BindingEdge
            | Capability::BlankSheet
            | Capability::Orientation
            | Capability::OuterMargin
            | Capability::PrintableRegion
            | Capability::SheetSize
            | Capability::TopClearance
            | Capability::WritingInset
            => Disposition::Accept,
            Capability::BorderGeometry
            | Capability::CustomDigitalPaper
            | Capability::DottedPaper
            | Capability::GridOrRuleGeometry
            | Capability::RuledPaper
            | Capability::SquaredPaper
            => Disposition::Convert,
            Capability::PageBackgroundColor
            | Capability::PaperTexture
            | Capability::SimulatedLooseSheet
            => Disposition::Reject,
        },
    }
}

/// Return the frozen disposition for this capability family.
#[must_use]
pub const fn semantic_capability_disposition(
    capability: SemanticCapability,
    mode: OutputMode,
) -> CapabilityDisposition {
    match mode {
        OutputMode::Digital => {
            semantic_capability_disposition_digital(capability)
        },
        OutputMode::Live => semantic_capability_disposition_live(capability),
    }
}

const fn semantic_capability_disposition_digital(
    capability: SemanticCapability,
) -> CapabilityDisposition {
    use CapabilityDisposition as Disposition;
    use SemanticCapability as Capability;
    match capability {
        Capability::AlignedMathematics
        | Capability::Arrow
        | Capability::Box
        | Capability::Callout
        | Capability::Citation
        | Capability::Date
        | Capability::Definition
        | Capability::DisplayedMathematics
        | Capability::Divider
        | Capability::Footnote
        | Capability::FreeformRegion
        | Capability::Heading
        | Capability::InlineMathematics
        | Capability::LoosePaperNote
        | Capability::MarginNote
        | Capability::MatrixMathematics
        | Capability::MergedTableCells
        | Capability::OrderedList
        | Capability::PageReference
        | Capability::Paragraph
        | Capability::Photograph
        | Capability::Quotation
        | Capability::RasterIllustration
        | Capability::SemanticDiagram
        | Capability::SourceNote
        | Capability::Table
        | Capability::TextLabel
        | Capability::UnitsInMathematics
        | Capability::UnorderedList
        | Capability::UnresolvedClaim
        | Capability::VectorLineArt => Disposition::Accept,
        Capability::UnresolvedUnsupportedBlock => Disposition::Reject,
    }
}

const fn semantic_capability_disposition_live(
    capability: SemanticCapability,
) -> CapabilityDisposition {
    use CapabilityDisposition as Disposition;
    use SemanticCapability as Capability;
    match capability {
        Capability::AlignedMathematics
        | Capability::Arrow
        | Capability::Box
        | Capability::Callout
        | Capability::Citation
        | Capability::Date
        | Capability::Definition
        | Capability::DisplayedMathematics
        | Capability::Divider
        | Capability::Footnote
        | Capability::FreeformRegion
        | Capability::Heading
        | Capability::InlineMathematics
        | Capability::MarginNote
        | Capability::MatrixMathematics
        | Capability::MergedTableCells
        | Capability::OrderedList
        | Capability::PageReference
        | Capability::Paragraph
        | Capability::Quotation
        | Capability::SemanticDiagram
        | Capability::SourceNote
        | Capability::Table
        | Capability::TextLabel
        | Capability::UnitsInMathematics
        | Capability::UnorderedList
        | Capability::UnresolvedClaim
        | Capability::VectorLineArt => Disposition::Accept,
        Capability::Photograph | Capability::RasterIllustration => {
            Disposition::Convert
        },
        Capability::LoosePaperNote | Capability::UnresolvedUnsupportedBlock => {
            Disposition::Reject
        },
    }
}
