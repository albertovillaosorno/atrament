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
        text: concat!(
            "La ni\u{f1}a explic\u{f3} c\u{f3}mo cambi\u{f3} el a\u{f1}o ",
            "despu\u{e9}s del ping\u{fc}ino.",
        ),
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "names",
        scenario: EnglishSpanishCorpusScenario::Names,
        text: concat!(
            "\u{c1}lvaro Mu\u{f1}oz, Mar\u{ed}a P\u{e9}rez, ",
            "\u{cd}\u{f1}igo N\u{fa}\u{f1}ez.",
        ),
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "quotations",
        scenario: EnglishSpanishCorpusScenario::Quotations,
        text: concat!(
            "\u{201c}Read this,\u{201d} she said; \u{ab}Lee esto\u{bb}, ",
            "respondi\u{f3} \u{e9}l; \u{2018}exacto\u{2019}.",
        ),
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "questions",
        scenario: EnglishSpanishCorpusScenario::Questions,
        text: "Why did it change? \u{bf}Por qu\u{e9} cambi\u{f3}?",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "exclamations",
        scenario: EnglishSpanishCorpusScenario::Exclamations,
        text: "Great! \u{a1}Qu\u{e9} \u{fa}til!",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "en-dash",
        scenario: EnglishSpanishCorpusScenario::EnDash,
        text: "Pages 10\u{2013}12 compare both cases.",
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "em-dash",
        scenario: EnglishSpanishCorpusScenario::EmDash,
        text: "The result\u{2014}still provisional\u{2014}needs review.",
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
            "\u{c1}/A\u{301} \u{c9}/E\u{301} \u{cd}/I\u{301} ",
            "\u{d1}/N\u{303} \u{d3}/O\u{301} ",
            "\u{da}/U\u{301} \u{dc}/U\u{308}; \u{e1}/a\u{301} ",
            "\u{e9}/e\u{301} \u{ed}/i\u{301} ",
            "\u{f1}/n\u{303} \u{f3}/o\u{301} \u{fa}/u\u{301} \u{fc}/u\u{308}",
        ),
    },
    EnglishSpanishCorpusFixture {
        artifact_identity: "mixed-mathematics",
        scenario: EnglishSpanishCorpusScenario::MixedMathematics,
        text: concat!(
            "English y espa\u{f1}ol: x^2 + y^2 = z^2; ",
            "\\frac{1}{2} + \\sqrt{x}.",
        ),
    },
];

/// Exact visible-text inventory sweep used by bilingual acceptance fixtures.
///
/// ASCII whitespace is only a delimiter between exact required graphemes; it is
/// not part of the handwriting glyph inventory.
pub const REQUIRED_TEXT_INVENTORY_SWEEP: &str = concat!(
    "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z ",
    "a b c d e f g h i j k l m n o p q r s t u v w x y z\n",
    "\u{c1} \u{c9} \u{cd} \u{d1} \u{d3} \u{da} \u{dc} ",
    "\u{e1} \u{e9} \u{ed} \u{f1} \u{f3} \u{fa} \u{fc}\n",
    "A\u{301} E\u{301} I\u{301} N\u{303} O\u{301} U\u{301} U\u{308} ",
    "a\u{301} e\u{301} i\u{301} n\u{303} o\u{301} u\u{301} u\u{308}\n",
    "0 1 2 3 4 5 6 7 8 9\n",
    ". , ; : ? ! \u{bf} \u{a1} \" ' \u{201c} \u{201d} ",
    "\u{2018} \u{2019} \u{ab} \u{bb} \u{2026} \u{2013} ",
    "\u{2014} - ( ) [ ]",
);

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
    /// The grapheme sweep has content after the complete required inventory.
    TextInventoryExtra(usize),
    /// One sweep token differs from the exact grapheme at its position.
    TextInventoryMismatch(usize),
}

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
        EnglishSpanishCorpusObservation {
            artifact_identity: fixture.artifact_identity,
            evidence: fixture.text,
            scenario: fixture.scenario,
        }
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
