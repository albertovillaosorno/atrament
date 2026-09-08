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
