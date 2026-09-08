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
//   - Regression evidence for one-profile handwriting role presentations.
// - Must-Not:
//   - Choose size units, role defaults, style policy, glyph behavior, or
//     semantic-document mappings.
// - Allows:
//   - Inputs: Deterministic caller-owned profile, size, and style fixtures.
//   - Outputs: Assertions over role vocabulary and shared profile identity.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Role selection or semantic-style mapping gains independent fixtures.
// - Merge-When:
//   - Role presentation moves into another handwriting profile harness.
// - Summary:
//   - Proves role variants remain presentations of one handwriting identity.
// - Description:
//   - Covers all first-release roles and caller-owned role size/style metadata.
// - Usage:
//   - Compile directly against the handwriting-role-profile domain.
// - Defaults:
//   - Missing roles remain absent instead of receiving implicit defaults.
//
use atrament_handwriting_role_profile::{
    HandwritingRole, HandwritingRolePresentation, HandwritingRoleProfile,
};

#[test]
fn first_release_role_vocabulary_is_explicit() {
    let roles = [
        HandwritingRole::Annotation,
        HandwritingRole::Body,
        HandwritingRole::Caption,
        HandwritingRole::Formula,
        HandwritingRole::Label,
        HandwritingRole::Margin,
        HandwritingRole::Subtitle,
        HandwritingRole::Title,
    ];
    assert_eq!(roles.len(), 8);
}

#[test]
fn role_presentations_share_one_profile_identity_by_structure() {
    let profile = HandwritingRoleProfile {
        profile_identity: "writer-profile-7",
        roles: vec![
            HandwritingRolePresentation {
                role: HandwritingRole::Body,
                size: "body-size",
                style: "body-style",
            },
            HandwritingRolePresentation {
                role: HandwritingRole::Title,
                size: "title-size",
                style: "title-style",
            },
        ],
    };
    assert_eq!(profile.profile_identity, "writer-profile-7");
    assert_eq!(profile.roles[0].role, HandwritingRole::Body);
    assert_eq!(profile.roles[1].role, HandwritingRole::Title);
}

#[test]
fn size_and_style_vocabularies_remain_caller_owned_without_defaults() {
    let profile = HandwritingRoleProfile {
        profile_identity: 17_u64,
        roles: vec![HandwritingRolePresentation {
            role: HandwritingRole::Formula,
            size: 42_u32,
            style: ("formula-style", 3_u8),
        }],
    };
    assert_eq!(profile.roles[0].size, 42);
    assert_eq!(profile.roles[0].style, ("formula-style", 3));
    assert_eq!(profile.roles.len(), 1);
}
