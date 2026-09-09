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
//   - Regression evidence for personal stroke-vocabulary entry structure.
// - Must-Not:
//   - Extract geometry, infer forms, choose confidence policy, plan strokes, or
//     render output.
// - Allows:
//   - Inputs: Deterministic geometry, connection, form, confidence, and sample
//     provenance fixtures.
//   - Outputs: Assertions over exact evidence retention.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Extraction or candidate-generation policy gains independent fixtures.
// - Merge-When:
//   - Vocabulary evidence moves into a profile-section harness.
// - Summary:
//   - Proves extracted stroke evidence remains explicit and provenance-rich.
// - Description:
//   - Covers every stroke-vocabulary evidence family named by the TODO.
// - Usage:
//   - Compile directly against the handwriting-stroke-vocabulary domain.
// - Defaults:
//   - No missing evidence family receives an implicit value.
//
use atrament_handwriting_stroke_vocabulary::{
    StrokeVocabularyConnection, StrokeVocabularyEntry, StrokeVocabularyForms,
    StrokeVocabularyGeometry, StrokeVocabularyProvenance,
};

#[test]
fn vocabulary_entry_retains_geometry_and_connection_evidence() {
    let entry = StrokeVocabularyEntry {
        connection: StrokeVocabularyConnection {
            entry_condition: "entry-left-low",
            exit_condition: "exit-right-mid",
            pen_lifts: ["lift-before-dot"],
        },
        forms: StrokeVocabularyForms {
            contextual_forms: ["initial", "medial", "final"],
            diacritics: ["acute"],
            ligatures: ["n-to-d"],
        },
        geometry: StrokeVocabularyGeometry {
            centerline: "centerline-evidence-7",
            contour: "contour-evidence-7",
        },
        provenance: StrokeVocabularyProvenance {
            confidence: 91_u8,
            sample_provenance: ["sample-region-4", "sample-region-8"],
        },
    };
    assert_eq!(entry.geometry.centerline, "centerline-evidence-7");
    assert_eq!(entry.geometry.contour, "contour-evidence-7");
    assert_eq!(entry.connection.entry_condition, "entry-left-low");
    assert_eq!(entry.connection.exit_condition, "exit-right-mid");
    assert_eq!(entry.connection.pen_lifts, ["lift-before-dot"]);
}

#[test]
fn vocabulary_entry_retains_form_confidence_and_sample_provenance() {
    let entry = StrokeVocabularyEntry {
        connection: StrokeVocabularyConnection {
            entry_condition: (),
            exit_condition: (),
            pen_lifts: Vec::<u8>::new(),
        },
        forms: StrokeVocabularyForms {
            contextual_forms: vec!["isolated", "medial"],
            diacritics: vec!["tilde"],
            ligatures: vec!["n-to-a", "n-to-o"],
        },
        geometry: StrokeVocabularyGeometry {
            centerline: vec![(0_i16, 0_i16), (1_i16, 2_i16)],
            contour: vec![(0_i16, 1_i16), (1_i16, 3_i16)],
        },
        provenance: StrokeVocabularyProvenance {
            confidence: "observed-confidence-4",
            sample_provenance: vec!["sample-11", "sample-12"],
        },
    };
    assert_eq!(entry.forms.contextual_forms, ["isolated", "medial"]);
    assert_eq!(entry.forms.diacritics, ["tilde"]);
    assert_eq!(entry.forms.ligatures, ["n-to-a", "n-to-o"]);
    assert_eq!(entry.provenance.confidence, "observed-confidence-4");
    assert_eq!(entry.provenance.sample_provenance, ["sample-11", "sample-12"]);
}
