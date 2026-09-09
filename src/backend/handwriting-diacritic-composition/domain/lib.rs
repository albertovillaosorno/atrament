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
//   - Admission of one profile-evidenced compositional diacritic intent.
// - Must-Not:
//   - Normalize Unicode, infer decomposition, evaluate rule semantics, choose
//     accent geometry, calculate collision, render strokes, or select fallback.
// - Allows:
//   - Inputs: Exact profile coverage declarations, one externally matched rule,
//     and caller-owned placement, scale, collision, and language-form evidence.
//   - Outputs: Admitted composition retaining the profile-declared rule, or a
//     typed rejection.
//   - Side effects: None.
// - Split-When:
//   - Rule evaluation, collision solving, or accent geometry gains independent
//     executable authority.
// - Merge-When:
//   - Diacritic admission becomes inseparable from contextual stroke planning.
// - Summary:
//   - Prevents accent reuse without explicit profile composition evidence.
// - Description:
//   - Accepts only a compositional coverage result and preserves presentation
//     evidence without interpreting it.
// - Usage:
//   - Validate one already matched composition before stroke planning.
// - Defaults:
//   - Exact or missing coverage never becomes implicit compositional reuse.
//

//! Profile-evidenced compositional diacritic admission before stroke planning.

use atrament_handwriting_glyph_coverage::{
    HandwritingCoverage, HandwritingCoverageProfile,
    classify_handwriting_coverage,
};

/// Caller-owned presentation evidence that must survive one diacritic reuse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiacriticPresentation<
    CollisionEvidence, LanguageForm, Placement, Scale,
> {
    /// Caller-owned collision evidence or constraint state.
    pub collision_evidence: CollisionEvidence,
    /// Caller-owned language-specific form identity or value.
    pub language_form: LanguageForm,
    /// Caller-owned placement intent for the diacritic.
    pub placement: Placement,
    /// Caller-owned diacritic scale intent.
    pub scale: Scale,
}

/// One requested compositional diacritic reuse before profile admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiacriticCompositionIntent<Grapheme, Presentation, Rule> {
    /// Externally matched compositional rule presented for admission.
    pub matched_rule: Rule,
    /// Placement, scale, collision, and language-form evidence.
    pub presentation: Presentation,
    /// Exact already-segmented grapheme whose composition is requested.
    pub target_grapheme: Grapheme,
}

/// Admitted composition retaining both request evidence and profile rule proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedDiacriticComposition<'profile, Intent, Rule> {
    /// Complete caller-owned composition intent.
    pub intent: Intent,
    /// Exact rule declaration from the admitting handwriting profile.
    pub profile_rule: &'profile Rule,
}

/// Why one requested diacritic composition cannot be admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiacriticCompositionError {
    /// The exact grapheme is already directly covered, so composition is not
    /// the admitted coverage path.
    ExactCoverage,
    /// The profile does not admit the supplied compositional rule.
    MissingCompositionalCoverage,
}

/// Admit one compositional diacritic only through profile-declared coverage.
///
/// Placement, scale, collision evidence, and language form are retained but not
/// interpreted here. Rule applicability is external evidence; this function
/// verifies only that profile coverage classifies the request as compositional.
///
/// # Errors
///
/// Returns `ExactCoverage` when the grapheme has direct profile coverage and
/// `MissingCompositionalCoverage` when no declared composition admits it.
pub fn admit_diacritic_composition<
    ProfileIdentity,
    Grapheme,
    Presentation,
    Rule,
>(
    profile: &HandwritingCoverageProfile<
        ProfileIdentity, Grapheme, Rule,
    >,
    intent: DiacriticCompositionIntent<Grapheme, Presentation, Rule>,
) -> Result<
    AdmittedDiacriticComposition<
        '_,
        DiacriticCompositionIntent<Grapheme, Presentation, Rule>,
        Rule,
    >,
    DiacriticCompositionError,
>
where
    Grapheme: PartialEq,
    Rule: PartialEq,
{
    match classify_handwriting_coverage(
        profile,
        &intent.target_grapheme,
        Some(&intent.matched_rule),
    ) {
        HandwritingCoverage::Compositional { rule } => {
            Ok(AdmittedDiacriticComposition {
                intent,
                profile_rule: rule,
            })
        },
        HandwritingCoverage::Exact => {
            Err(DiacriticCompositionError::ExactCoverage)
        },
        HandwritingCoverage::Missing => {
            Err(DiacriticCompositionError::MissingCompositionalCoverage)
        },
    }
}
