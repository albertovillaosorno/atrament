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
//   - Transport-neutral personal handwriting stroke-vocabulary evidence.
// - Must-Not:
//   - Extract geometry, choose units, infer joins/forms, score confidence,
//     synthesize strokes, or render output.
// - Allows:
//   - Inputs: Caller-owned centerline/contour, connection behavior,
//     ligature/diacritic/contextual-form evidence, confidence, and provenance.
//   - Outputs: Inspectable stroke-vocabulary entries.
//   - Side effects: None.
// - Split-When:
//   - Stroke extraction or contextual candidate generation gains executable
//     policy.
// - Merge-When:
//   - Vocabulary evidence becomes inseparable from profile-section authority.
// - Summary:
//   - Freezes extracted stroke evidence without implementing extraction.
// - Description:
//   - Preserves geometry, connection, form, confidence, and sample provenance.
// - Usage:
//   - Feed validated vocabulary evidence into later contextual candidates.
// - Defaults:
//   - No geometry, form, confidence, or provenance value is inferred.
//

//! Personal handwriting stroke-vocabulary evidence before candidate synthesis.

/// Caller-owned centerline and contour evidence for one vocabulary entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokeVocabularyGeometry<Centerline, Contour> {
    /// Caller-owned centerline evidence.
    pub centerline: Centerline,
    /// Caller-owned contour evidence.
    pub contour: Contour,
}

/// Caller-owned entry, exit, and pen-lift behavior for one vocabulary entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokeVocabularyConnection<EntryCondition, ExitCondition, PenLifts> {
    /// Caller-owned entry condition evidence.
    pub entry_condition: EntryCondition,
    /// Caller-owned exit condition evidence.
    pub exit_condition: ExitCondition,
    /// Caller-owned pen-lift evidence.
    pub pen_lifts: PenLifts,
}

/// Caller-owned contextual form evidence for one vocabulary entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokeVocabularyForms<ContextualForms, Diacritics, Ligatures> {
    /// Caller-owned contextual-form evidence.
    pub contextual_forms: ContextualForms,
    /// Caller-owned diacritic evidence.
    pub diacritics: Diacritics,
    /// Caller-owned ligature evidence.
    pub ligatures: Ligatures,
}

/// Caller-owned evidentiary quality and sample provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokeVocabularyProvenance<Confidence, SampleProvenance> {
    /// Caller-owned confidence evidence or value.
    pub confidence: Confidence,
    /// Caller-owned sample provenance supporting this entry.
    pub sample_provenance: SampleProvenance,
}

/// One complete personal handwriting stroke-vocabulary entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokeVocabularyEntry<Connection, Forms, Geometry, Provenance> {
    /// Entry/exit and pen-lift behavior.
    pub connection: Connection,
    /// Ligature, diacritic, and contextual-form evidence.
    pub forms: Forms,
    /// Centerline and contour evidence.
    pub geometry: Geometry,
    /// Confidence and source-sample provenance.
    pub provenance: Provenance,
}
