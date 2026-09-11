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
//   - Regression evidence for explicit handwriting fallback admission.
// - Must-Not:
//   - Choose styles, normalize text, classify profile declarations, infer user
//     consent, render glyphs, or emit diagnostics.
// - Allows:
//   - Inputs: Deterministic profile-coverage and fallback-evidence fixtures.
//   - Outputs: Assertions over profile coverage, explicit fallback, and block.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Fallback selection, UI consent, or diagnostic behavior gains fixtures.
// - Merge-When:
//   - Fallback admission moves into glyph projection acceptance tests.
// - Summary:
//   - Proves missing handwriting never receives silent fallback.
// - Description:
//   - Exhausts declaration, visibility, and acceptance states independently.
// - Usage:
//   - Compile directly against the handwriting-fallback-admission domain.
// - Defaults:
//   - Missing coverage stays blocked without complete explicit evidence.
//
use atrament_handwriting_fallback_admission::{
    FallbackStyleDeclaration, FallbackStyleVisibility, FallbackUserAcceptance,
    HandwritingCoverageAdmission, HandwritingFallbackAdmissionError,
    HandwritingFallbackEvidence, admit_handwriting_fallback,
};
use atrament_handwriting_glyph_coverage::HandwritingCoverage;

fn fallback(
    declaration: FallbackStyleDeclaration,
    visibility: FallbackStyleVisibility,
    user_acceptance: FallbackUserAcceptance,
) -> HandwritingFallbackEvidence<&'static str> {
    HandwritingFallbackEvidence {
        declaration,
        style_identity: "declared-readable-fallback",
        user_acceptance,
        visibility,
    }
}

#[test]
fn profile_coverage_never_requires_or_selects_fallback() {
    let declared_rule = "profile-rule";
    for coverage in [
        HandwritingCoverage::Exact,
        HandwritingCoverage::Compositional {
            rule: &declared_rule,
        },
    ] {
        assert_eq!(
            admit_handwriting_fallback::<_, &str>(coverage, None),
            Ok(HandwritingCoverageAdmission::ProfileCoverage),
        );
    }
}

#[test]
fn missing_coverage_without_fallback_remains_blocked() {
    assert_eq!(
        admit_handwriting_fallback::<u8, &str>(
            HandwritingCoverage::Missing,
            None,
        ),
        Err(HandwritingFallbackAdmissionError::MissingCoverageWithoutFallback),
    );
}

#[test]
fn all_eight_fallback_evidence_states_match_three_requirement_oracle() {
    let mut cases = 0_u8;
    let mut saw_admitted = false;
    let mut saw_rejected = false;
    for declaration in [
        FallbackStyleDeclaration::Undeclared,
        FallbackStyleDeclaration::Declared,
    ] {
        for visibility in [
            FallbackStyleVisibility::NotEstablished,
            FallbackStyleVisibility::Visible,
        ] {
            for user_acceptance in [
                FallbackUserAcceptance::NotAccepted,
                FallbackUserAcceptance::Accepted,
            ] {
                let evidence = fallback(
                    declaration,
                    visibility,
                    user_acceptance,
                );
                let expected = if declaration
                    == FallbackStyleDeclaration::Declared
                    && visibility == FallbackStyleVisibility::Visible
                    && user_acceptance == FallbackUserAcceptance::Accepted
                {
                    saw_admitted = true;
                    Ok(HandwritingCoverageAdmission::VisibleAcceptedFallback {
                        style_identity: &evidence.style_identity,
                    })
                } else {
                    saw_rejected = true;
                    Err(
                        HandwritingFallbackAdmissionError::
                            RequirementsNotEstablished {
                                declaration,
                                user_acceptance,
                                visibility,
                            },
                    )
                };
                assert_eq!(
                    admit_handwriting_fallback(
                        HandwritingCoverage::<u8>::Missing,
                        Some(&evidence),
                    ),
                    expected,
                );
                cases = cases.saturating_add(1);
            }
        }
    }
    assert_eq!(cases, 8);
    assert!(saw_admitted);
    assert!(saw_rejected);
}

#[test]
fn admitted_fallback_retains_exact_caller_owned_style_identity() {
    let mut evidence = fallback(
        FallbackStyleDeclaration::Declared,
        FallbackStyleVisibility::Visible,
        FallbackUserAcceptance::Accepted,
    );
    evidence.style_identity = "fallback-style-17";
    assert_eq!(
        admit_handwriting_fallback(
            HandwritingCoverage::<u8>::Missing,
            Some(&evidence),
        ),
        Ok(HandwritingCoverageAdmission::VisibleAcceptedFallback {
            style_identity: &"fallback-style-17",
        }),
    );
}
#[test]
fn all_27_coverage_and_fallback_states_match_independent_admission_oracle() {
    let declared_rule = "profile-rule";
    let mut cases = 0_u8;
    let mut saw_profile = false;
    let mut saw_fallback = false;
    let mut saw_missing_without_fallback = false;
    let mut saw_incomplete_fallback = false;
    for coverage_index in 0_u8..3 {
        for fallback_index in 0_u8..9 {
            let coverage = match coverage_index {
                0 => HandwritingCoverage::Exact,
                1 => HandwritingCoverage::Compositional {
                    rule: &declared_rule,
                },
                _ => HandwritingCoverage::Missing,
            };
            let evidence = (fallback_index != 0).then(|| {
                let bits = fallback_index - 1;
                fallback(
                    if bits & 1 == 0 {
                        FallbackStyleDeclaration::Undeclared
                    } else {
                        FallbackStyleDeclaration::Declared
                    },
                    if bits & 2 == 0 {
                        FallbackStyleVisibility::NotEstablished
                    } else {
                        FallbackStyleVisibility::Visible
                    },
                    if bits & 4 == 0 {
                        FallbackUserAcceptance::NotAccepted
                    } else {
                        FallbackUserAcceptance::Accepted
                    },
                )
            });
            let expected = if coverage_index != 2 {
                saw_profile = true;
                Ok(HandwritingCoverageAdmission::ProfileCoverage)
            } else if evidence.is_none() {
                saw_missing_without_fallback = true;
                Err(
                    HandwritingFallbackAdmissionError::
                        MissingCoverageWithoutFallback,
                )
            } else {
                let supplied = evidence
                    .as_ref()
                    .expect("nonzero fallback state supplies evidence");
                if supplied.declaration == FallbackStyleDeclaration::Declared
                    && supplied.visibility == FallbackStyleVisibility::Visible
                    && supplied.user_acceptance
                        == FallbackUserAcceptance::Accepted
                {
                    saw_fallback = true;
                    Ok(HandwritingCoverageAdmission::VisibleAcceptedFallback {
                        style_identity: &supplied.style_identity,
                    })
                } else {
                    saw_incomplete_fallback = true;
                    Err(
                        HandwritingFallbackAdmissionError::
                            RequirementsNotEstablished {
                                declaration: supplied.declaration,
                                user_acceptance: supplied.user_acceptance,
                                visibility: supplied.visibility,
                            },
                    )
                }
            };
            assert_eq!(
                admit_handwriting_fallback(coverage, evidence.as_ref()),
                expected,
                "coverage {coverage_index}, fallback {fallback_index}",
            );
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 27);
    assert!(saw_profile);
    assert!(saw_fallback);
    assert!(saw_missing_without_fallback);
    assert!(saw_incomplete_fallback);
}
