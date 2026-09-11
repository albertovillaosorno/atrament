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
