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
    HandwritingRole, HandwritingRoleLookupError, HandwritingRolePresentation,
    HandwritingRoleProfile, handwriting_role_presentation,
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

#[test]
fn role_lookup_returns_exact_presentation_without_synthesizing_missing_roles() {
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
    let title = handwriting_role_presentation(&profile, HandwritingRole::Title)
        .expect("one title role is unambiguous")
        .expect("title presentation exists");
    assert_eq!(title.size, "title-size");
    assert_eq!(title.style, "title-style");
    assert_eq!(
        handwriting_role_presentation(&profile, HandwritingRole::Caption),
        Ok(None),
    );
}

#[test]
fn duplicate_requested_role_reports_earliest_and_later_indices() {
    let profile = HandwritingRoleProfile {
        profile_identity: 7_u8,
        roles: vec![
            HandwritingRolePresentation {
                role: HandwritingRole::Body,
                size: 10_u8,
                style: 20_u8,
            },
            HandwritingRolePresentation {
                role: HandwritingRole::Title,
                size: 11_u8,
                style: 21_u8,
            },
            HandwritingRolePresentation {
                role: HandwritingRole::Body,
                size: 12_u8,
                style: 22_u8,
            },
        ],
    };
    assert_eq!(
        handwriting_role_presentation(&profile, HandwritingRole::Body),
        Err(HandwritingRoleLookupError {
            duplicate_index: 2,
            first_index: 0,
            role: HandwritingRole::Body,
        }),
    );
}

#[test]
fn all_31_two_role_sequences_match_independent_lookup_oracle() {
    let roles = [HandwritingRole::Body, HandwritingRole::Title];
    let mut cases = 0_u8;
    let mut saw_missing = false;
    let mut saw_unique = false;
    let mut saw_ambiguous = false;
    for length in 0_u32..=4 {
        for encoded in 0_u32..2_u32.pow(length) {
            let mut state = encoded;
            let mut presentations = Vec::new();
            for index in 0..length {
                let role = roles[(state % 2) as usize];
                state /= 2;
                presentations.push(HandwritingRolePresentation {
                    role,
                    size: index,
                    style: index + 10,
                });
            }
            let profile = HandwritingRoleProfile {
                profile_identity: 7_u8,
                roles: presentations,
            };
            for requested in roles {
                let matches = profile
                    .roles
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.role == requested)
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                let expected = match matches.as_slice() {
                    [] => {
                        saw_missing = true;
                        Ok(None)
                    },
                    [index] => {
                        saw_unique = true;
                        Ok(Some(&profile.roles[*index]))
                    },
                    [first_index, duplicate_index, ..] => {
                        saw_ambiguous = true;
                        Err(HandwritingRoleLookupError {
                            duplicate_index: *duplicate_index,
                            first_index: *first_index,
                            role: requested,
                        })
                    },
                };
                assert_eq!(
                    handwriting_role_presentation(&profile, requested),
                    expected,
                    "length {length}, encoded {encoded}, role {requested:?}",
                );
            }
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 31);
    assert!(saw_missing);
    assert!(saw_unique);
    assert!(saw_ambiguous);
}
