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
//   - Handwriting role vocabulary and one-profile role-presentation structure.
// - Must-Not:
//   - Choose size units, role defaults, style vocabulary, glyph behavior,
//     rendering, planning, or semantic document style mappings.
// - Allows:
//   - Inputs: One caller-owned profile identity and caller-owned size/style
//     data.
//   - Outputs: Role presentations that remain structurally tied to that
//     profile.
//   - Side effects: None.
// - Split-When:
//   - Role selection or semantic-style mapping gains independent application
//     authority.
// - Merge-When:
//   - Role presentation becomes inseparable from the portable profile domain.
// - Summary:
//   - Keeps role-specific handwriting presentation under one writer identity.
// - Description:
//   - Defines first-release handwriting roles without inventing size semantics.
// - Usage:
//   - Attach role-specific size/style metadata to one handwriting profile.
// - Defaults:
//   - No role presentation is synthesized when the caller does not provide it.
//

//! One-writer role and size presentation vocabulary for handwriting profiles.

/// All first-release handwriting roles in stable semantic order.
pub const REQUIRED_HANDWRITING_ROLES: [HandwritingRole; 8] = [
    HandwritingRole::Annotation,
    HandwritingRole::Body,
    HandwritingRole::Caption,
    HandwritingRole::Formula,
    HandwritingRole::Label,
    HandwritingRole::Margin,
    HandwritingRole::Subtitle,
    HandwritingRole::Title,
];

/// First-release semantic presentation roles exposed by one handwriting
/// profile.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HandwritingRole {
    /// Annotation handwriting.
    Annotation,
    /// Ordinary body handwriting.
    Body,
    /// Figure or supporting caption handwriting.
    Caption,
    /// Mathematical-formula handwriting.
    Formula,
    /// Compact label handwriting.
    Label,
    /// Margin handwriting.
    Margin,
    /// Subtitle handwriting.
    Subtitle,
    /// Dominant title handwriting.
    Title,
}

/// Caller-owned presentation metadata for one handwriting role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandwritingRolePresentation<Size, Style> {
    /// Semantic handwriting role.
    pub role: HandwritingRole,
    /// Caller-owned role size in its owning unit vocabulary.
    pub size: Size,
    /// Caller-owned profile style or variation choice for this role.
    pub style: Style,
}

/// Role presentations exposed by one handwriting profile identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandwritingRoleProfile<ProfileIdentity, Size, Style> {
    /// Stable caller-owned identity shared by every role presentation.
    pub profile_identity: ProfileIdentity,
    /// Caller-supplied role presentations. Missing roles have no implicit
    /// value.
    pub roles: Vec<HandwritingRolePresentation<Size, Style>>,
}

/// Why one role cannot be projected to a single presentation safely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandwritingRoleLookupError {
    /// Later presentation carrying the same requested role.
    pub duplicate_index: usize,
    /// Earliest presentation carrying the requested role.
    pub first_index: usize,
    /// Requested role with more than one presentation.
    pub role: HandwritingRole,
}

/// Multiplicity observed for one first-release role in a profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandwritingRoleMultiplicity {
    /// More than one presentation claims the role.
    Ambiguous,
    /// No presentation claims the role.
    Missing,
    /// Exactly one presentation claims the role.
    Unique,
}

/// Read-only occurrence audit for one first-release handwriting role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandwritingRoleAudit {
    /// Caller-order presentation indices that claim this role.
    pub presentation_indices: Vec<usize>,
    /// First-release role being audited.
    pub role: HandwritingRole,
    /// Missing, unique, or ambiguous occurrence classification.
    pub status: HandwritingRoleMultiplicity,
}

/// Audit all first-release roles without choosing missing/duplicate policy.
///
/// Results follow [`REQUIRED_HANDWRITING_ROLES`] order. Presentation indices
/// preserve caller order exactly. Missing roles receive no synthesized default,
/// and ambiguous roles remain reported rather than being resolved implicitly.
#[must_use]
pub fn audit_handwriting_role_profile<ProfileIdentity, Size, Style>(
    profile: &HandwritingRoleProfile<ProfileIdentity, Size, Style>,
) -> [HandwritingRoleAudit; 8] {
    REQUIRED_HANDWRITING_ROLES.map(|role| {
        let presentation_indices = profile
            .roles
            .iter()
            .enumerate()
            .filter_map(|(index, presentation)| {
                (presentation.role == role).then_some(index)
            })
            .collect::<Vec<_>>();
        let status = match presentation_indices.len() {
            0 => HandwritingRoleMultiplicity::Missing,
            1 => HandwritingRoleMultiplicity::Unique,
            _ => HandwritingRoleMultiplicity::Ambiguous,
        };
        HandwritingRoleAudit {
            presentation_indices,
            role,
            status,
        }
    })
}

/// Return one unambiguous presentation for a requested handwriting role.
///
/// A missing role returns `None` and does not synthesize a default. Duplicate
/// role claims reject instead of silently choosing caller order. This lookup
/// does not make duplicate or missing roles globally invalid profile state.
///
/// # Errors
///
/// Returns the first duplicate for the requested role and its earliest owner.
pub fn handwriting_role_presentation<ProfileIdentity, Size, Style>(
    profile: &HandwritingRoleProfile<ProfileIdentity, Size, Style>,
    role: HandwritingRole,
) -> Result<
    Option<&HandwritingRolePresentation<Size, Style>>,
    HandwritingRoleLookupError,
> {
    let mut first = None;
    for (index, presentation) in profile.roles.iter().enumerate() {
        if presentation.role != role {
            continue;
        }
        if let Some((first_index, _)) = first {
            return Err(HandwritingRoleLookupError {
                duplicate_index: index,
                first_index,
                role,
            });
        }
        first = Some((index, presentation));
    }
    Ok(first.map(|(_, presentation)| presentation))
}
