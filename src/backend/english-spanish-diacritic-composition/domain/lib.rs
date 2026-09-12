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
//   - English/Spanish rule lookup before generic profile-evidenced diacritic
//     composition admission.
// - Must-Not:
//   - Normalize Unicode, infer decomposition, choose accent geometry, evaluate
//     collision, select language forms, render strokes, or select fallback.
// - Allows:
//   - Inputs: One exact already-segmented grapheme, generic presentation
//     evidence, and a profile declaring English/Spanish diacritic rules.
//   - Outputs: One admitted generic composition or typed language/profile
//     rejection.
//   - Side effects: None.
// - Split-When:
//   - Another language gains its own executable compositional-rule matcher.
// - Merge-When:
//   - Language rule matching becomes inseparable from generic composition
//     admission.
// - Summary:
//   - Connects the frozen bilingual rule matcher to profile composition proof.
// - Description:
//   - Rejects spellings outside the 14 decomposed bilingual requirements before
//     asking generic profile coverage to admit composition.
// - Usage:
//   - Apply to one exact English/Spanish grapheme before stroke planning.
// - Defaults:
//   - Unsupported or precomposed spellings never gain implicit decomposition.
//

//! English/Spanish diacritic-rule lookup plus generic profile admission.

use atrament_english_spanish_diacritic_rule::{
    EnglishSpanishDiacriticRule, applicable_english_spanish_diacritic_rule,
};
use atrament_handwriting_diacritic_composition::{
    AdmittedDiacriticComposition, DiacriticCompositionError,
    DiacriticCompositionIntent, admit_diacritic_composition,
};
use atrament_handwriting_glyph_coverage::HandwritingCoverageProfile;

/// Complete bilingual composition admission result.
pub type EnglishSpanishDiacriticCompositionAdmission<
    'profile,
    Grapheme,
    Presentation,
    ProfileIdentity,
> = Result<
    AdmittedDiacriticComposition<
        'profile,
        DiacriticCompositionIntent<
            Grapheme,
            Presentation,
            EnglishSpanishDiacriticRule,
        >,
        ProfileIdentity,
        EnglishSpanishDiacriticRule,
    >,
    EnglishSpanishDiacriticCompositionError,
>;

/// Why one English/Spanish compositional request cannot be admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnglishSpanishDiacriticCompositionError {
    /// Exact spelling has no admitted bilingual decomposed-diacritic rule.
    NoApplicableLanguageRule,
    /// The language rule exists but generic profile composition rejects it.
    ProfileAdmission(DiacriticCompositionError),
}

/// Match and admit one exact English/Spanish decomposed diacritic grapheme.
///
/// Rule matching is language-owned and performs no Unicode normalization.
/// Generic profile admission still decides whether exact coverage wins or the
/// matched compositional rule is actually declared by the selected profile.
/// Presentation evidence is retained unchanged and remains uninterpreted.
///
/// # Errors
///
/// Returns
/// [`EnglishSpanishDiacriticCompositionError::NoApplicableLanguageRule`]
/// when the exact spelling is outside the frozen 14 decomposed requirements, or
/// wraps the generic profile-admission rejection after a language rule matches.
pub fn admit_english_spanish_diacritic_composition<
    ProfileIdentity,
    Grapheme,
    Presentation,
>(
    profile: &HandwritingCoverageProfile<
        ProfileIdentity,
        Grapheme,
        EnglishSpanishDiacriticRule,
    >,
    target_grapheme: Grapheme,
    presentation: Presentation,
) -> EnglishSpanishDiacriticCompositionAdmission<
    '_,
    Grapheme,
    Presentation,
    ProfileIdentity,
>
where
    Grapheme: AsRef<str> + PartialEq,
{
    let Some(matched_rule) =
        applicable_english_spanish_diacritic_rule(target_grapheme.as_ref())
    else {
        return Err(
            EnglishSpanishDiacriticCompositionError::NoApplicableLanguageRule,
        );
    };
    let intent = DiacriticCompositionIntent {
        matched_rule,
        presentation,
        target_grapheme,
    };
    admit_diacritic_composition(profile, intent)
        .map_err(EnglishSpanishDiacriticCompositionError::ProfileAdmission)
}
