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
//   - Checked-in English/Spanish corpus text for the accepted language ADR.
// - Must-Not:
//   - Normalize Unicode, segment arbitrary input, measure, wrap, edit, render,
//     define mathematical glyph coverage, or exercise CLI/MCP transport.
// - Allows:
//   - Inputs: Frozen bilingual scenario and visible-text grapheme authorities.
//   - Outputs: Exact scenario fixtures, one exact grapheme sweep, and
//     deterministic fixture-completeness validation.
//   - Side effects: None.
// - Split-When:
//   - Cross-surface language acceptance gains executable behavior fixtures.
// - Merge-When:
//   - Corpus content becomes inseparable from a complete language harness.
// - Summary:
//   - Authors the concrete bilingual corpus without claiming downstream use.
// - Description:
//   - Provides exact text for every ADR scenario plus the frozen text
//     inventory.
// - Usage:
//   - Validate this fixture before using it as input to downstream acceptance
//     evidence.
// - Defaults:
//   - Precomposed and decomposed forms remain exact authored alternatives.
//

//! Checked-in text fixtures for the first English/Spanish language baseline.

use EnglishSpanishCorpusFixtureError as FixtureError;
use atrament_english_spanish_corpus_evidence::{
    EnglishSpanishCorpusEvidenceError, EnglishSpanishCorpusObservation,
    EnglishSpanishCorpusScenario, validate_english_spanish_corpus_evidence,
};
use atrament_english_spanish_grapheme_inventory::REQUIRED_TEXT_GRAPHEMES;

/// One exact checked-in text fixture for an ADR-required corpus scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnglishSpanishCorpusFixture {
    /// Stable fixture identity for test and acceptance evidence.
    pub artifact_identity: &'static str,
    /// ADR scenario represented by this fixture.
    pub scenario: EnglishSpanishCorpusScenario,
    /// Exact authored UTF-8 content retained without normalization.
    pub text: &'static str,
}

/// Why the checked-in bilingual corpus no longer matches its frozen
/// authorities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnglishSpanishCorpusFixtureError {
    /// The scenario checklist rejects the checked-in fixture set.
    ScenarioEvidence(EnglishSpanishCorpusEvidenceError),
    /// The grapheme sweep ends before one required inventory position.
    TextInventoryEndedEarly(usize),
    /// One sweep token differs from the exact grapheme at its position.
    TextInventoryMismatch(usize),
    /// The grapheme sweep has content after the complete required inventory.
    TextInventoryExtra(usize),
}

/// Exact checked-in text for every English/Spanish corpus scenario in the ADR.
pub const REQUIRED_SCENARIO_FIXTURES: [EnglishSpanishCorpusFixture; 11] = [
    EnglishSpanishCorpusFixture {
        artifact_identity: "english-prose",
        scenario: EnglishSpanishCorpusScenario::EnglishProse,
        text: "Careful notes connect evidence, examples, and conclusions.",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "spanish-prose",
        scenario: EnglishSpanishCorpusScenario::SpanishProse,
        text: "La niña explicó cómo cambió el año después del pingüino.",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "names",
        scenario: EnglishSpanishCorpusScenario::Names,
        text: "Álvaro Muñoz, María Pérez, Íñigo Núñez.",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "quotations",
        scenario: EnglishSpanishCorpusScenario::Quotations,
        text: "“Read this,” she said; «Lee esto», respondió él; ‘exacto’.",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "questions",
        scenario: EnglishSpanishCorpusScenario::Questions,
        text: "Why did it change? ¿Por qué cambió?",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "exclamations",
        scenario: EnglishSpanishCorpusScenario::Exclamations,
        text: "Great! ¡Qué útil!",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "en-dash",
        scenario: EnglishSpanishCorpusScenario::EnDash,
        text: "Pages 10–12 compare both cases.",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "em-dash",
        scenario: EnglishSpanishCorpusScenario::EmDash,
        text: "The result—still provisional—needs review.",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "combining-marks",
        scenario: EnglishSpanishCorpusScenario::CombiningMarks,
        text: concat!(
            "A\u{301} E\u{301} I\u{301} N\u{303} O\u{301} U\u{301} U\u{308} ",
            "a\u{301} e\u{301} i\u{301} n\u{303} o\u{301} u\u{301} u\u{308}",
        ),
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "normalized-equivalents",
        scenario: EnglishSpanishCorpusScenario::NormalizedEquivalents,
        text: concat!(
            "Á/A\u{301} É/E\u{301} Í/I\u{301} Ñ/N\u{303} Ó/O\u{301} ",
            "Ú/U\u{301} Ü/U\u{308}; á/a\u{301} é/e\u{301} í/i\u{301} ",
            "ñ/n\u{303} ó/o\u{301} ú/u\u{301} ü/u\u{308}",
        ),
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "mixed-mathematics",
        scenario: EnglishSpanishCorpusScenario::MixedMathematics,
        text: "English y español: x^2 + y^2 = z^2; \\frac{1}{2} + \\sqrt{x}.",
    },
];

/// Exact visible-text inventory sweep used by bilingual acceptance fixtures.
///
/// ASCII whitespace is only a delimiter between exact required graphemes; it is
/// not part of the handwriting glyph inventory.
pub const REQUIRED_TEXT_INVENTORY_SWEEP: &str = concat!(
    "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z ",
    "a b c d e f g h i j k l m n o p q r s t u v w x y z\n",
    "Á É Í Ñ Ó Ú Ü á é í ñ ó ú ü\n",
    "A\u{301} E\u{301} I\u{301} N\u{303} O\u{301} U\u{301} U\u{308} ",
    "a\u{301} e\u{301} i\u{301} n\u{303} o\u{301} u\u{301} u\u{308}\n",
    "0 1 2 3 4 5 6 7 8 9\n",
    ". , ; : ? ! ¿ ¡ \" ' “ ” ‘ ’ « » … – — - ( ) [ ]",
);

/// Result of validating the checked-in corpus against its frozen authorities.
pub type CorpusFixtureValidation = Result<(), EnglishSpanishCorpusFixtureError>;

/// Validate the exact checked-in corpus against both frozen language
/// authorities.
///
/// # Errors
///
/// Returns the first scenario or exact inventory mismatch in deterministic
/// order.
pub fn validate_checked_in_english_spanish_corpus() -> CorpusFixtureValidation {
    let observations = REQUIRED_SCENARIO_FIXTURES.map(|fixture| {
        let observation = EnglishSpanishCorpusObservation {
            artifact_identity: fixture.artifact_identity,
            evidence: fixture.text,
            scenario: fixture.scenario,
        };
        observation
    });
    validate_english_spanish_corpus_evidence(&observations)
        .map_err(FixtureError::ScenarioEvidence)?;

    let mut actual = REQUIRED_TEXT_INVENTORY_SWEEP.split_ascii_whitespace();
    for (index, required) in REQUIRED_TEXT_GRAPHEMES.iter().enumerate() {
        let Some(actual_grapheme) = actual.next() else {
            let error = FixtureError::TextInventoryEndedEarly(index);
            return Err(error);
        };
        if actual_grapheme != required.grapheme {
            let error = FixtureError::TextInventoryMismatch(index);
            return Err(error);
        }
    }
    if actual.next().is_some() {
        return Err(FixtureError::TextInventoryExtra(
            REQUIRED_TEXT_GRAPHEMES.len(),
        ));
    }
    Ok(())
}
