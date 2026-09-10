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
//   - Transport-neutral digital loose-paper-note composition intent.
// - Must-Not:
//   - Render pixels, choose colors, folds, shadows, stack order, contrast
//     metrics, contrast thresholds, geometry, or live conversion behavior.
// - Allows:
//   - Inputs: Caller-owned fill, fold, stacking, shadow, and contrast evidence.
//   - Outputs: Preserved note intent and structural readability admission.
//   - Side effects: None.
// - Split-When:
//   - Contrast measurement, note layout, or shadow rendering gains executable
//     authority.
// - Merge-When:
//   - Paper-note composition becomes inseparable from one renderer layer plan.
// - Summary:
//   - Freezes digital paper-note effects without implementing their renderer.
// - Description:
//   - Retains note appearance inputs and requires caller-provided readability.
// - Usage:
//   - Validate digital paper-note intent before later layout and rendering.
// - Defaults:
//   - No style, geometry, contrast threshold, or live fallback is inferred.
//

//! Digital loose-paper-note intent before geometry or raster composition.

/// Caller-computed contrast evidence for one digital paper note.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DigitalPaperNoteContrast {
    /// Caller-owned evidence says readable contrast is not established.
    NotEstablished,
    /// Caller-owned evidence says note content remains readable.
    Readable,
}

/// One digital-only loose-paper-note composition intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigitalPaperNotePlan<Fill, Fold, Shadow, StackOrder> {
    /// Caller-owned note fill or material intent.
    pub fill: Fill,
    /// Caller-owned fold intent.
    pub fold: Fold,
    /// Caller-computed contrast evidence.
    pub readable_contrast: DigitalPaperNoteContrast,
    /// Caller-owned soft-shadow intent.
    pub shadow: Shadow,
    /// Caller-owned stacking or z-order intent.
    pub stack_order: StackOrder,
}

/// Why one digital paper-note plan is not structurally admissible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigitalPaperNotePlanError {
    /// Caller evidence does not establish readable content contrast.
    ReadableContrastNotEstablished,
}

/// Require caller-provided readable-contrast evidence for one digital note.
///
/// This function does not calculate contrast or infer a threshold. Live output
/// incompatibility remains owned by the frozen output-capability matrix.
///
/// # Errors
///
/// Returns [`DigitalPaperNotePlanError::ReadableContrastNotEstablished`] when
/// caller-owned evidence does not establish readable contrast.
pub fn validate_digital_paper_note_plan<Fill, Fold, Shadow, StackOrder>(
    plan: &DigitalPaperNotePlan<Fill, Fold, Shadow, StackOrder>,
) -> Result<(), DigitalPaperNotePlanError> {
    if plan.readable_contrast == DigitalPaperNoteContrast::NotEstablished {
        return Err(DigitalPaperNotePlanError::ReadableContrastNotEstablished);
    }
    Ok(())
}
