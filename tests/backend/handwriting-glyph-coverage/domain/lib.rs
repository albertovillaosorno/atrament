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
    HandwritingCoverage, HandwritingCoverageProfile,
    classify_handwriting_coverage,
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
