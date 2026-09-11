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
//   - Regression evidence for English/Spanish corpus-scenario completeness.
// - Must-Not:
//   - Enumerate graphemes, author language fixtures, normalize, measure, wrap,
//     edit, serialize, render, or exercise CLI/MCP transport.
// - Allows:
//   - Inputs: Deterministic scenario, artifact, and supporting-evidence
//     fixtures.
//   - Outputs: Assertions over required scenario presence and retention.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Concrete corpus or cross-surface language behavior gains fixtures.
// - Merge-When:
//   - Scenario evidence moves into a complete language acceptance harness.
// - Summary:
//   - Proves every ADR-named bilingual corpus scenario stays explicit.
// - Description:
//   - Exhausts all scenario-presence masks without inventing corpus contents.
// - Usage:
//   - Compile directly against the English/Spanish corpus-evidence domain.
// - Defaults:
//   - Artifact identities and supporting evidence remain caller-owned.
//
use atrament_english_spanish_corpus_evidence::{
    EnglishSpanishCorpusEvidenceError, EnglishSpanishCorpusObservation,
    EnglishSpanishCorpusScenario, validate_english_spanish_corpus_evidence,
};

const SCENARIOS: [EnglishSpanishCorpusScenario; 11] = [
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

fn observation(
    scenario: EnglishSpanishCorpusScenario,
) -> EnglishSpanishCorpusObservation<String, String> {
    EnglishSpanishCorpusObservation {
        artifact_identity: format!("artifact-{scenario:?}"),
        evidence: format!("evidence-{scenario:?}"),
        scenario,
    }
}

#[test]
fn complete_adr_scenario_set_is_structurally_admitted() {
    let observations = SCENARIOS.map(observation);
    assert_eq!(
        validate_english_spanish_corpus_evidence(&observations),
        Ok(()),
    );
    assert_eq!(observations.len(), 11);
}

#[test]
fn duplicate_scenarios_and_owned_evidence_remain_caller_owned() {
    let mut observations = SCENARIOS.map(observation).to_vec();
    observations.push(EnglishSpanishCorpusObservation {
        artifact_identity: String::from("extra: México — ‘¿qué?’"),
        evidence: String::from("caller-reviewed-extra-prose"),
        scenario: EnglishSpanishCorpusScenario::SpanishProse,
    });
    assert_eq!(
        validate_english_spanish_corpus_evidence(&observations),
        Ok(()),
    );
    assert_eq!(observations[11].artifact_identity, "extra: México — ‘¿qué?’");
    assert_eq!(observations[11].evidence, "caller-reviewed-extra-prose");
}

#[test]
fn all_2048_scenario_presence_masks_match_first_missing_scenario_oracle() {
    let mut cases = 0_u16;
    let mut saw_complete = false;
    let mut saw_each_missing = [false; 11];
    for mask in 0_u16..2_048 {
        let observations = SCENARIOS
            .into_iter()
            .enumerate()
            .filter(|(index, _)| mask & (1_u16 << index) != 0)
            .map(|(_, scenario)| observation(scenario))
            .collect::<Vec<_>>();
        let expected = SCENARIOS
            .into_iter()
            .enumerate()
            .find(|(index, _)| mask & (1_u16 << index) == 0)
            .map_or_else(
                || {
                    saw_complete = true;
                    Ok(())
                },
                |(index, scenario)| {
                    saw_each_missing[index] = true;
                    Err(EnglishSpanishCorpusEvidenceError::ScenarioAbsent(
                        scenario,
                    ))
                },
            );
        assert_eq!(
            validate_english_spanish_corpus_evidence(&observations),
            expected,
            "scenario presence mask {mask:#013b}",
        );
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 2_048);
    assert!(saw_complete);
    assert!(saw_each_missing.into_iter().all(|seen| seen));
}
