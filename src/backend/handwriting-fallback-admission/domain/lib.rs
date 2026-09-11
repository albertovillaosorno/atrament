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
//   - Admission of an explicit visible handwriting fallback after profile
//     coverage classification.
// - Must-Not:
//   - Choose fallback styles, normalize text, classify profile coverage, infer
//     user consent, render glyphs, emit diagnostics, or define wire fields.
// - Allows:
//   - Inputs: Existing profile-coverage classification plus caller-owned
//     fallback style declaration, visibility, and user-acceptance evidence.
//   - Outputs: Profile-covered or explicitly accepted fallback admission.
//   - Side effects: None.
// - Split-When:
//   - Fallback style selection, UI consent, or diagnostic projection gains
//     executable authority.
// - Merge-When:
//   - Fallback admission becomes inseparable from glyph projection policy.
// - Summary:
//   - Keeps missing handwriting coverage blocked without explicit fallback.
// - Description:
//   - Admits only a declared, visible style explicitly accepted by the user.
// - Usage:
//   - Apply after profile-scoped glyph coverage has already been classified.
// - Defaults:
//   - Missing profile coverage remains blocked when fallback evidence is absent
//     or incomplete.
//

//! Explicit handwriting fallback admission after profile coverage
//! classification.

use atrament_handwriting_glyph_coverage::HandwritingCoverage;

/// Whether the fallback style is explicitly declared by an owning authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackStyleDeclaration {
    /// The fallback style is explicitly declared.
    Declared,
    /// No declaration establishes this fallback style.
    Undeclared,
}

/// Whether the fallback will remain visible rather than silently substituted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackStyleVisibility {
    /// Caller evidence does not establish visible fallback presentation.
    NotEstablished,
    /// Caller evidence establishes that fallback use remains visible.
    Visible,
}

/// Whether the user explicitly accepted this declared fallback style.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackUserAcceptance {
    /// The user explicitly accepted this fallback style.
    Accepted,
    /// User acceptance is absent or declined.
    NotAccepted,
}

/// Caller-owned evidence required to admit one fallback style.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandwritingFallbackEvidence<StyleIdentity> {
    /// Whether the fallback style is explicitly declared.
    pub declaration: FallbackStyleDeclaration,
    /// Caller-owned stable fallback style identity.
    pub style_identity: StyleIdentity,
    /// Whether the user explicitly accepted this fallback style.
    pub user_acceptance: FallbackUserAcceptance,
    /// Whether fallback use remains visible to the user.
    pub visibility: FallbackStyleVisibility,
}

/// Handwriting projection admission after profile coverage and fallback review.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandwritingCoverageAdmission<
    'coverage,
    'evidence,
    Rule,
    StyleIdentity,
> {
    /// Exact or compositional profile coverage is already sufficient.
    ProfileCoverage {
        /// Original exact/compositional coverage, including any declared rule.
        coverage: HandwritingCoverage<'coverage, Rule>,
    },
    /// Missing profile coverage is replaced only by explicit fallback evidence.
    VisibleAcceptedFallback {
        /// Exact caller-owned fallback style identity that was admitted.
        style_identity: &'evidence StyleIdentity,
    },
}

/// Why missing profile coverage cannot proceed through fallback admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandwritingFallbackAdmissionError {
    /// Profile coverage is missing and no fallback evidence was supplied.
    MissingCoverageWithoutFallback,
    /// A fallback was supplied but one or more required facts are absent.
    RequirementsNotEstablished {
        /// Exact declaration state supplied by the caller.
        declaration: FallbackStyleDeclaration,
        /// Exact user-acceptance state supplied by the caller.
        user_acceptance: FallbackUserAcceptance,
        /// Exact visibility state supplied by the caller.
        visibility: FallbackStyleVisibility,
    },
}

/// Admit existing profile coverage or one explicitly reviewed fallback style.
///
/// Exact and compositional profile coverage do not require fallback evidence.
/// Missing coverage remains blocked unless the supplied fallback is declared,
/// visible, and explicitly accepted by the user. This function never chooses a
/// style or interprets how visibility or consent were established.
///
/// # Errors
///
/// Returns [`HandwritingFallbackAdmissionError`] when missing coverage has no
/// fallback or when supplied fallback evidence does not establish all three
/// accepted requirements.
pub fn admit_handwriting_fallback<
    'coverage,
    'evidence,
    Rule,
    StyleIdentity,
>(
    coverage: HandwritingCoverage<'coverage, Rule>,
    fallback: Option<&'evidence HandwritingFallbackEvidence<StyleIdentity>>,
) -> Result<
    HandwritingCoverageAdmission<'coverage, 'evidence, Rule, StyleIdentity>,
    HandwritingFallbackAdmissionError,
> {
    if !matches!(coverage, HandwritingCoverage::Missing) {
        return Ok(HandwritingCoverageAdmission::ProfileCoverage { coverage });
    }
    let Some(fallback_evidence) = fallback else {
        return Err(
            HandwritingFallbackAdmissionError::MissingCoverageWithoutFallback,
        );
    };
    if fallback_evidence.declaration != FallbackStyleDeclaration::Declared
        || fallback_evidence.visibility != FallbackStyleVisibility::Visible
        || fallback_evidence.user_acceptance != FallbackUserAcceptance::Accepted
    {
        return Err(
            HandwritingFallbackAdmissionError::RequirementsNotEstablished {
                declaration: fallback_evidence.declaration,
                user_acceptance: fallback_evidence.user_acceptance,
                visibility: fallback_evidence.visibility,
            },
        );
    }
    Ok(HandwritingCoverageAdmission::VisibleAcceptedFallback {
        style_identity: &fallback_evidence.style_identity,
    })
}
