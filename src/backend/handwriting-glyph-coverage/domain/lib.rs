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
//   - Profile-scoped handwriting coverage declarations and classification.
// - Must-Not:
//   - Normalize text, segment graphemes, decide compositional-rule semantics,
//     choose fallback styles, render glyphs, or define portable wire fields.
// - Allows:
//   - Inputs: Caller-owned profile identity, exact grapheme declarations,
//     compositional-rule identities, and one externally matched rule.
//   - Outputs: Exact, compositional, or missing handwriting coverage.
//   - Side effects: None.
// - Split-When:
//   - Rule evaluation, fallback admission, or coverage diagnostics gain
//     independent executable authority.
// - Merge-When:
//   - Coverage declarations become inseparable from one typed profile section.
// - Summary:
//   - Makes profile handwriting coverage explicit without renderer inference.
// - Description:
//   - Accepts only exact declarations or caller-matched rules declared by the
//     same profile.
// - Usage:
//   - Classify one already-segmented grapheme before handwriting projection.
// - Defaults:
//   - Missing coverage remains missing; no fallback is selected implicitly.
//

//! Explicit handwriting-profile grapheme coverage before rendering.

/// Handwriting coverage declarations owned by one profile identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandwritingCoverageProfile<ProfileIdentity, Grapheme, Rule> {
    /// Compositional-rule identities declared by this handwriting profile.
    pub compositional_rules: Vec<Rule>,
    /// Exact grapheme values declared directly by this handwriting profile.
    pub exact_graphemes: Vec<Grapheme>,
    /// Stable caller-owned handwriting profile identity.
    pub profile_identity: ProfileIdentity,
}

/// Coverage admitted for one already-segmented grapheme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandwritingCoverage<'profile, Rule> {
    /// An externally matched rule is declared by this profile.
    Compositional {
        /// Profile-owned rule identity that admits the grapheme composition.
        rule: &'profile Rule,
    },
    /// The profile declares this exact grapheme directly.
    Exact,
    /// Neither exact nor admitted compositional coverage is available.
    Missing,
}

/// One caller-ordered grapheme coverage query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandwritingCoverageQuery<'query, Grapheme, Rule> {
    /// Exact already-segmented grapheme being checked.
    pub grapheme: &'query Grapheme,
    /// Optional externally matched compositional rule for this grapheme.
    pub matched_compositional_rule: Option<&'query Rule>,
}

/// One missing handwriting-coverage result in caller query order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MissingHandwritingCoverage<'query, Grapheme> {
    /// Exact grapheme whose handwriting coverage is missing.
    pub grapheme: &'query Grapheme,
    /// Zero-based index of the caller-supplied coverage query.
    pub query_index: usize,
}

/// Classify one grapheme against one handwriting profile's declarations.
///
/// `matched_compositional_rule` is evidence from a separate rule evaluator.
/// This domain verifies only that the supplied rule is declared by the profile;
/// it does not decide whether the rule correctly applies to the grapheme.
#[must_use]
pub fn classify_handwriting_coverage<'profile, ProfileIdentity, Grapheme, Rule>(
    profile: &'profile HandwritingCoverageProfile<
        ProfileIdentity, Grapheme, Rule,
    >,
    grapheme: &Grapheme,
    matched_compositional_rule: Option<&Rule>,
) -> HandwritingCoverage<'profile, Rule>
where
    Grapheme: PartialEq,
    Rule: PartialEq,
{
    if profile
        .exact_graphemes
        .iter()
        .any(|declared| declared == grapheme)
    {
        return HandwritingCoverage::Exact;
    }
    let Some(matched_rule) = matched_compositional_rule else {
        return HandwritingCoverage::Missing;
    };
    profile
        .compositional_rules
        .iter()
        .find(|declared| *declared == matched_rule)
        .map_or(HandwritingCoverage::Missing, |rule| {
            HandwritingCoverage::Compositional { rule }
        })
}

/// Report every caller-supplied grapheme whose handwriting coverage is missing.
///
/// The function preserves query order and occurrences. It performs no Unicode
/// normalization, deduplication, semantic-location mapping, or fallback choice.
#[must_use]
pub fn missing_handwriting_coverage<'query, ProfileIdentity, Grapheme, Rule>(
    profile: &HandwritingCoverageProfile<
        ProfileIdentity, Grapheme, Rule,
    >,
    queries: &[HandwritingCoverageQuery<'query, Grapheme, Rule>],
) -> Vec<MissingHandwritingCoverage<'query, Grapheme>>
where
    Grapheme: PartialEq,
    Rule: PartialEq,
{
    queries
        .iter()
        .enumerate()
        .filter_map(|(query_index, query)| {
            matches!(
                classify_handwriting_coverage(
                    profile,
                    query.grapheme,
                    query.matched_compositional_rule,
                ),
                HandwritingCoverage::Missing,
            )
            .then_some(MissingHandwritingCoverage {
                grapheme: query.grapheme,
                query_index,
            })
        })
        .collect()
}
