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
//   - Structural completeness of English/Spanish corpus-scenario evidence.
// - Must-Not:
//   - Enumerate required graphemes, author corpus text, normalize Unicode,
//     measure, wrap, edit, serialize, render, or define CLI/MCP transport.
// - Allows:
//   - Inputs: Caller-owned artifact identity and supporting evidence for one
//     ADR-required corpus scenario.
//   - Outputs: Deterministic required-scenario completeness validation.
//   - Side effects: None.
// - Split-When:
//   - Concrete corpus construction or cross-surface grapheme verification gains
//     executable authority.
// - Merge-When:
//   - Corpus evidence becomes inseparable from language acceptance fixtures.
// - Summary:
//   - Prevents the accepted bilingual corpus scenarios from being omitted.
// - Description:
//   - Freezes the ADR verification checklist without inventing corpus content.
// - Usage:
//   - Validate scenario coverage before claiming the bilingual corpus
//     checklist.
// - Defaults:
//   - No grapheme inventory, normalization form, or content fixture is
//     inferred.
//

//! English/Spanish corpus-scenario evidence without text-behavior authority.

const REQUIRED_SCENARIOS: [EnglishSpanishCorpusScenario; 11] = [
    EnglishSpanishCorpusScenario::EnglishProse,
    EnglishSpanishCorpusScenario::SpanishProse,
    EnglishSpanishCorpusScenario::Names,
    EnglishSpanishCorpusScenario::Quotations,
    EnglishSpanishCorpusScenario::Questions,
    EnglishSpanishCorpusScenario::Exclamations,
    EnglishSpanishCorpusScenario::EnDash,
    EnglishSpanishCorpusScenario::EmDash,
    EnglishSpanishCorpusScenario::CombiningMarks,
    EnglishSpanishCorpusScenario::NormalizedEquivalents,
    EnglishSpanishCorpusScenario::MixedMathematics,
];

/// Corpus scenarios explicitly required by the accepted language ADR.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EnglishSpanishCorpusScenario {
    /// Corpus evidence exercising combining marks.
    CombiningMarks,
    /// Corpus evidence exercising an em dash.
    EmDash,
    /// Corpus evidence exercising an en dash.
    EnDash,
    /// English prose evidence.
    EnglishProse,
    /// Exclamation evidence.
    Exclamations,
    /// Mixed prose and mathematics evidence.
    MixedMathematics,
    /// Mixed-language or ordinary personal-name evidence.
    Names,
    /// Equivalent normalized spellings represented by caller evidence.
    NormalizedEquivalents,
    /// Question evidence.
    Questions,
    /// Quotation evidence.
    Quotations,
    /// Spanish prose evidence.
    SpanishProse,
}

/// One caller-produced observation for an ADR-required corpus scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnglishSpanishCorpusObservation<ArtifactIdentity, Evidence> {
    /// Exact caller-owned corpus artifact or fixture identity.
    pub artifact_identity: ArtifactIdentity,
    /// Caller-owned supporting evidence for this scenario.
    pub evidence: Evidence,
    /// Required corpus scenario exercised by this observation.
    pub scenario: EnglishSpanishCorpusScenario,
}

/// Constructor-sealed evidence that one exact caller-owned corpus checklist
/// contains every required scenario.
#[derive(Debug)]
pub struct ValidatedEnglishSpanishCorpusEvidence<
    'observations,
    ArtifactIdentity,
    Evidence,
> {
    observations: &'observations [
        EnglishSpanishCorpusObservation<ArtifactIdentity, Evidence>
    ],
}

impl<'observations, ArtifactIdentity, Evidence>
    ValidatedEnglishSpanishCorpusEvidence<
        'observations,
        ArtifactIdentity,
        Evidence,
    >
{
    /// Return the exact caller-owned observations that passed completeness
    /// admission.
    #[must_use]
    pub const fn observations(
        &self,
    ) -> &'observations [
        EnglishSpanishCorpusObservation<ArtifactIdentity, Evidence>
    ] {
        self.observations
    }
}

/// Why the structural English/Spanish corpus checklist is incomplete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnglishSpanishCorpusEvidenceError {
    /// No caller observation represents one required corpus scenario.
    ScenarioAbsent(EnglishSpanishCorpusScenario),
}

/// Require evidence for every corpus scenario named by the accepted ADR.
///
/// Multiple observations for one scenario remain valid caller-owned evidence.
/// This function does not inspect corpus text or claim that every required
/// grapheme or downstream render/measure/edit surface has been verified.
///
/// # Errors
///
/// Returns the first missing scenario in the accepted ADR requirement order.
pub fn validate_english_spanish_corpus_evidence<ArtifactIdentity, Evidence>(
    observations: &[
        EnglishSpanishCorpusObservation<ArtifactIdentity, Evidence>
    ],
) -> Result<(), EnglishSpanishCorpusEvidenceError> {
    for scenario in REQUIRED_SCENARIOS {
        if !observations
            .iter()
            .any(|observation| observation.scenario == scenario)
        {
            return Err(EnglishSpanishCorpusEvidenceError::ScenarioAbsent(
                scenario,
            ));
        }
    }
    Ok(())
}

/// Validate and seal one exact corpus-evidence checklist for read-only reuse.
///
/// # Errors
///
/// Returns the same first missing scenario as
/// [`validate_english_spanish_corpus_evidence`].
pub fn validate_english_spanish_corpus_evidence_view<
    ArtifactIdentity,
    Evidence,
>(
    observations: &[
        EnglishSpanishCorpusObservation<ArtifactIdentity, Evidence>
    ],
) -> Result<
    ValidatedEnglishSpanishCorpusEvidence<'_, ArtifactIdentity, Evidence>,
    EnglishSpanishCorpusEvidenceError,
> {
    validate_english_spanish_corpus_evidence(observations)?;
    Ok(ValidatedEnglishSpanishCorpusEvidence { observations })
}
