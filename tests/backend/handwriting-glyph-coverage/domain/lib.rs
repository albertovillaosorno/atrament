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
//   - Regression evidence for explicit handwriting-profile glyph coverage.
// - Must-Not:
//   - Normalize Unicode, evaluate composition semantics, choose fallback style,
//     render glyphs, or define profile wire schema.
// - Allows:
//   - Inputs: Deterministic profile, grapheme, and rule fixtures.
//   - Outputs: Assertions over exact, compositional, and missing coverage.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Rule evaluation or fallback admission gains independent fixtures.
// - Merge-When:
//   - Coverage validation moves into another handwriting profile harness.
// - Summary:
//   - Proves unsupported graphemes cannot become implicit renderer fallback.
// - Description:
//   - Covers exact precedence, admitted rules, undeclared rules, and missing
//     coverage.
// - Usage:
//   - Compile directly against the handwriting-glyph-coverage domain.
// - Defaults:
//   - Missing coverage has no implicit fallback.
//
use atrament_handwriting_glyph_coverage::{
    HandwritingCoverage, HandwritingCoverageProfile, HandwritingCoverageQuery,
    MissingHandwritingCoverage, classify_handwriting_coverage,
    missing_handwriting_coverage,
};

fn profile(
) -> HandwritingCoverageProfile<&'static str, &'static str, &'static str> {
    HandwritingCoverageProfile {
        compositional_rules: vec![
            "latin-base-plus-acute",
            "latin-base-plus-tilde",
        ],
        exact_graphemes: vec!["a", "á", "¿", "—"],
        profile_identity: "writer-profile-7",
    }
}

#[test]
fn profile_identity_and_declarations_remain_explicit() {
    let profile = profile();
    assert_eq!(profile.profile_identity, "writer-profile-7");
    assert_eq!(profile.exact_graphemes, ["a", "á", "¿", "—"]);
    assert_eq!(profile.compositional_rules.len(), 2);
}

#[test]
fn exact_grapheme_declaration_takes_precedence_over_rule_evidence() {
    let profile = profile();
    assert_eq!(
        classify_handwriting_coverage(
            &profile,
            &"á",
            Some(&"latin-base-plus-acute"),
        ),
        HandwritingCoverage::Exact,
    );
}

#[test]
fn exact_grapheme_precedes_undeclared_rule_evidence() {
    let profile = profile();
    assert_eq!(
        classify_handwriting_coverage(
            &profile,
            &"á",
            Some(&"generic-font-fallback"),
        ),
        HandwritingCoverage::Exact,
    );
}

#[test]
fn declared_compositional_rule_can_admit_non_exact_grapheme() {
    let profile = profile();
    assert_eq!(
        classify_handwriting_coverage(
            &profile,
            &"n\u{303}",
            Some(&"latin-base-plus-tilde"),
        ),
        HandwritingCoverage::Compositional {
            rule: &"latin-base-plus-tilde",
        },
    );
}

#[test]
fn absent_or_undeclared_rule_evidence_leaves_coverage_missing() {
    let profile = profile();
    assert_eq!(
        classify_handwriting_coverage(&profile, &"ø", None),
        HandwritingCoverage::Missing,
    );
    assert_eq!(
        classify_handwriting_coverage(
            &profile,
            &"ø",
            Some(&"generic-font-fallback"),
        ),
        HandwritingCoverage::Missing,
    );
}


#[test]
fn missing_report_preserves_query_order_and_duplicate_occurrences() {
    let profile = profile();
    let first = "ø";
    let covered = "a";
    let second = "ø";
    let queries = [
        HandwritingCoverageQuery {
            grapheme: &first,
            matched_compositional_rule: None,
        },
        HandwritingCoverageQuery {
            grapheme: &covered,
            matched_compositional_rule: None,
        },
        HandwritingCoverageQuery {
            grapheme: &second,
            matched_compositional_rule: None,
        },
    ];
    assert_eq!(
        missing_handwriting_coverage(&profile, &queries),
        [
            MissingHandwritingCoverage {
                grapheme: &first,
                query_index: 0,
            },
            MissingHandwritingCoverage {
                grapheme: &second,
                query_index: 2,
            },
        ],
    );
}

#[test]
fn admitted_compositional_rule_removes_query_from_missing_report() {
    let profile = profile();
    let composed = "n\u{303}";
    let rule = "latin-base-plus-tilde";
    let queries = [HandwritingCoverageQuery {
        grapheme: &composed,
        matched_compositional_rule: Some(&rule),
    }];
    assert!(missing_handwriting_coverage(&profile, &queries).is_empty());
}

#[test]
fn missing_report_preserves_normalization_spelling_without_equivalence() {
    let profile = profile();
    let nfd = "a\u{301}";
    let queries = [HandwritingCoverageQuery {
        grapheme: &nfd,
        matched_compositional_rule: None,
    }];
    assert_eq!(
        missing_handwriting_coverage(&profile, &queries),
        [MissingHandwritingCoverage {
            grapheme: &nfd,
            query_index: 0,
        }],
    );
}
#[test]
fn compact_classification_states_match_exact_rule_missing_oracle() {
    let grapheme = "target";
    let declared_rule = "declared-rule";
    let other_rule = "other-rule";
    let mut cases = 0_u8;
    let mut outcomes = [false; 3];
    for exact_declared in [false, true] {
        for rule_declared in [false, true] {
            for matched_rule in [None, Some(declared_rule), Some(other_rule)] {
                let profile = HandwritingCoverageProfile {
                    compositional_rules: rule_declared
                        .then_some(declared_rule)
                        .into_iter()
                        .collect(),
                    exact_graphemes: exact_declared
                        .then_some(grapheme)
                        .into_iter()
                        .collect(),
                    profile_identity: 7_u8,
                };
                let expected = if exact_declared {
                    outcomes[0] = true;
                    HandwritingCoverage::Exact
                } else if rule_declared && matched_rule == Some(declared_rule) {
                    outcomes[1] = true;
                    HandwritingCoverage::Compositional {
                        rule: &profile.compositional_rules[0],
                    }
                } else {
                    outcomes[2] = true;
                    HandwritingCoverage::Missing
                };
                assert_eq!(
                    classify_handwriting_coverage(
                        &profile,
                        &grapheme,
                        matched_rule.as_ref(),
                    ),
                    expected,
                    "exact {}, rule {}, matched {:?}",
                    exact_declared,
                    rule_declared,
                    matched_rule,
                );
                cases = cases.saturating_add(1);
            }
        }
    }
    assert_eq!(cases, 12);
    assert!(outcomes.into_iter().all(|seen| seen));
}

#[test]
fn all_64_six_query_missing_masks_preserve_exact_occurrences_and_order() {
    let profile = HandwritingCoverageProfile {
        compositional_rules: Vec::<u8>::new(),
        exact_graphemes: vec![1_u8],
        profile_identity: 7_u8,
    };
    let covered = 1_u8;
    let missing = 2_u8;
    let mut cases = 0_u8;
    for mask in 0_u8..64 {
        let values = (0..6)
            .map(|index| {
                if mask & (1_u8 << index) == 0 {
                    &covered
                } else {
                    &missing
                }
            })
            .collect::<Vec<_>>();
        let queries = values
            .iter()
            .map(|grapheme| HandwritingCoverageQuery {
                grapheme: *grapheme,
                matched_compositional_rule: None,
            })
            .collect::<Vec<_>>();
        let expected = values
            .iter()
            .enumerate()
            .filter_map(|(query_index, grapheme)| {
                (**grapheme == missing).then_some(MissingHandwritingCoverage {
                    grapheme: *grapheme,
                    query_index,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(missing_handwriting_coverage(&profile, &queries), expected);
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 64);
}
