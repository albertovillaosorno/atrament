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
//   - Regression evidence for assignment organization over semantic blocks.
// - Must-Not:
//   - Generate assignment content, infer facts/provenance, lay out, or render.
// - Allows:
//   - Inputs: Deterministic notebook and assignment-role fixtures.
//   - Outputs: Assertions over role vocabulary and no-invention admission.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Assignment generation gains independent acceptance fixtures.
// - Merge-When:
//   - Structure evidence moves into a complete candidate-construction harness.
// - Summary:
//   - Proves missing facts remain explicitly unresolved instead of invented.
// - Description:
//   - Also proves organizational roles do not redefine semantic block kinds.
// - Usage:
//   - Compile against assignment-notebook-structure and semantic-notebook.
// - Defaults:
//   - Empty and repeated organizational roles remain caller-owned choices.
//

//! Assignment notebook semantic structure regression tests.

use std::ptr;

use atrament_assignment_notebook_structure::{
    ASSIGNMENT_NOTEBOOK_ROLES, AssignmentNotebookDisposition,
    AssignmentNotebookRole, AssignmentNotebookStructureEntry,
    AssignmentNotebookStructureError, AssignmentNotebookStructurePlan,
    project_assignment_notebook_structure_blocks,
    validate_assignment_notebook_structure,
    validate_assignment_notebook_structure_view,
};
use atrament_semantic_notebook::{
    Block, BlockContent, Flow, InlineSpan, Notebook, Page, UnresolvedBlock,
    UnresolvedReason,
};

// Typed test guards retain errors without unchecked extraction.
macro_rules! require_ok {
    ($expression:expr) => {{
        let outcome = $expression;
        assert!(
            outcome.is_ok(),
            "expected projection: {:?}",
            outcome.as_ref().err()
        );
        let Ok(value) = outcome else {
            return;
        };
        value
    }};
}

const MATRIX_STATES: [FixtureState; 6] = [
    FixtureState::Represented,
    FixtureState::MissingFact,
    FixtureState::WrongRepresented,
    FixtureState::WrongMissingFact,
    FixtureState::NonBlock,
    FixtureState::UnknownIdentity,
];

#[derive(Clone, Copy, Debug)]
enum FixtureState {
    MissingFact,
    NonBlock,
    Represented,
    UnknownIdentity,
    WrongMissingFact,
    WrongRepresented,
}

const fn fixture_disposition(
    state: FixtureState,
) -> AssignmentNotebookDisposition<u32> {
    match state {
        FixtureState::Represented => {
            AssignmentNotebookDisposition::Represented { block: 11 }
        }
        FixtureState::MissingFact => {
            AssignmentNotebookDisposition::UnresolvedMissingFact { block: 12 }
        }
        FixtureState::WrongRepresented => {
            AssignmentNotebookDisposition::Represented { block: 12 }
        }
        FixtureState::WrongMissingFact => {
            AssignmentNotebookDisposition::UnresolvedMissingFact { block: 11 }
        }
        FixtureState::NonBlock => {
            AssignmentNotebookDisposition::Represented { block: 3 }
        }
        FixtureState::UnknownIdentity => {
            AssignmentNotebookDisposition::Represented { block: 99 }
        }
    }
}

const fn fixture_error(
    state: FixtureState,
    entry_index: usize,
) -> Option<AssignmentNotebookStructureError> {
    match state {
        FixtureState::Represented | FixtureState::MissingFact => None,
        FixtureState::WrongRepresented => {
            Some(AssignmentNotebookStructureError::RepresentedIsUnresolved {
                entry_index,
            })
        }
        FixtureState::WrongMissingFact => Some(
            AssignmentNotebookStructureError::UnresolvedMissingFactIsResolved {
                entry_index,
            },
        ),
        FixtureState::NonBlock => {
            Some(AssignmentNotebookStructureError::NotBlock { entry_index })
        }
        FixtureState::UnknownIdentity => {
            Some(AssignmentNotebookStructureError::UnknownIdentity {
                entry_index,
            })
        }
    }
}

fn span(id: u32, text: &str) -> InlineSpan<u32> {
    InlineSpan {
        id,
        provenance: None,
        style: None,
        text: String::from(text),
    }
}

fn notebook() -> Notebook<u32> {
    Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: 1,
        output_profiles: vec![],
        page_profiles: vec![],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![
                    Block {
                        content: BlockContent::Heading(vec![span(20, "Title")]),
                        extensions: vec![],
                        id: 10,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Paragraph(vec![span(
                            21,
                            "Evidence-backed explanation",
                        )]),
                        extensions: vec![],
                        id: 11,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Unresolved(UnresolvedBlock {
                            extensions: vec![],
                            reason: UnresolvedReason::Ambiguous,
                            source: String::from("missing task fact"),
                        }),
                        extensions: vec![],
                        id: 12,
                        provenance: None,
                        style: None,
                    },
                ],
                id: 4,
            }],
            id: 3,
            paper_profile: 2,
        }],
        provenance: vec![],
        styles: vec![],
    }
}

#[test]
fn all_nine_assignment_organization_roles_are_explicit() {
    assert_eq!(
        ASSIGNMENT_NOTEBOOK_ROLES,
        [
            AssignmentNotebookRole::Title,
            AssignmentNotebookRole::Explanation,
            AssignmentNotebookRole::Derivation,
            AssignmentNotebookRole::Table,
            AssignmentNotebookRole::Equation,
            AssignmentNotebookRole::Diagram,
            AssignmentNotebookRole::Citation,
            AssignmentNotebookRole::Example,
            AssignmentNotebookRole::Conclusion,
        ],
    );
}

#[test]
fn represented_and_missing_fact_entries_validate_without_role_inference() {
    let plan = AssignmentNotebookStructurePlan {
        entries: vec![
            AssignmentNotebookStructureEntry {
                disposition: AssignmentNotebookDisposition::Represented {
                    block: 10,
                },
                role: AssignmentNotebookRole::Title,
            },
            AssignmentNotebookStructureEntry {
                disposition: AssignmentNotebookDisposition::Represented {
                    block: 11,
                },
                role: AssignmentNotebookRole::Example,
            },
            AssignmentNotebookStructureEntry {
                disposition:
                    AssignmentNotebookDisposition::UnresolvedMissingFact {
                        block: 12,
                    },
                role: AssignmentNotebookRole::Conclusion,
            },
        ],
    };
    assert_eq!(
        validate_assignment_notebook_structure(&notebook(), &plan),
        Ok(())
    );
    assert_eq!(
        plan.entries.first().map(|entry| entry.role),
        Some(AssignmentNotebookRole::Title)
    );
    assert_eq!(
        plan.entries.get(1).map(|entry| entry.role),
        Some(AssignmentNotebookRole::Example)
    );
    let notebook = notebook();
    let projected = require_ok!(project_assignment_notebook_structure_blocks(
        &notebook, &plan,
    ));
    assert_eq!(
        projected.iter().map(|block| block.id).collect::<Vec<_>>(),
        [10, 11, 12],
    );
    assert!(matches!(
        projected.first().map(|block| &block.content),
        Some(BlockContent::Heading(_))
    ));
    assert!(matches!(
        projected.get(2).map(|block| &block.content),
        Some(BlockContent::Unresolved(_))
    ));
}

#[test]
fn empty_plan_and_repeated_roles_do_not_synthesize_structure_policy() {
    assert_eq!(
        validate_assignment_notebook_structure(
            &notebook(),
            &AssignmentNotebookStructurePlan { entries: vec![] },
        ),
        Ok(()),
    );
    let source = notebook();
    assert_eq!(
        project_assignment_notebook_structure_blocks(
            &source,
            &AssignmentNotebookStructurePlan { entries: vec![] },
        ),
        Ok(vec![]),
    );
    let repeated = AssignmentNotebookStructurePlan {
        entries: vec![
            AssignmentNotebookStructureEntry {
                disposition: AssignmentNotebookDisposition::Represented {
                    block: 11,
                },
                role: AssignmentNotebookRole::Explanation,
            },
            AssignmentNotebookStructureEntry {
                disposition: AssignmentNotebookDisposition::Represented {
                    block: 11,
                },
                role: AssignmentNotebookRole::Explanation,
            },
        ],
    };
    assert_eq!(
        validate_assignment_notebook_structure(&notebook(), &repeated),
        Ok(()),
    );
    let notebook = notebook();
    let projected = require_ok!(project_assignment_notebook_structure_blocks(
        &notebook, &repeated,
    ));
    assert_eq!(projected.len(), 2);
    let first_candidate = projected.first();
    let second_candidate = projected.get(1);
    assert!(matches!(
        (first_candidate, second_candidate),
        (Some(first), Some(second)) if ptr::eq(*first, *second)
    ));
}

#[test]
fn disposition_must_match_resolved_or_unresolved_block_authority() {
    let represented_unresolved = AssignmentNotebookStructurePlan {
        entries: vec![AssignmentNotebookStructureEntry {
            disposition: AssignmentNotebookDisposition::Represented {
                block: 12,
            },
            role: AssignmentNotebookRole::Conclusion,
        }],
    };
    assert_eq!(
        validate_assignment_notebook_structure(
            &notebook(),
            &represented_unresolved,
        ),
        Err(AssignmentNotebookStructureError::RepresentedIsUnresolved {
            entry_index: 0,
        }),
    );
    let unresolved_resolved = AssignmentNotebookStructurePlan {
        entries: vec![AssignmentNotebookStructureEntry {
            disposition: AssignmentNotebookDisposition::UnresolvedMissingFact {
                block: 11,
            },
            role: AssignmentNotebookRole::Conclusion,
        }],
    };
    assert_eq!(
        validate_assignment_notebook_structure(
            &notebook(),
            &unresolved_resolved
        ),
        Err(
            AssignmentNotebookStructureError::UnresolvedMissingFactIsResolved {
                entry_index: 0,
            }
        ),
    );
}

#[test]
fn unknown_and_non_block_identities_reject_at_exact_entry() {
    let unknown = AssignmentNotebookStructurePlan {
        entries: vec![AssignmentNotebookStructureEntry {
            disposition: AssignmentNotebookDisposition::Represented {
                block: 99,
            },
            role: AssignmentNotebookRole::Explanation,
        }],
    };
    assert_eq!(
        validate_assignment_notebook_structure(&notebook(), &unknown),
        Err(AssignmentNotebookStructureError::UnknownIdentity {
            entry_index: 0,
        }),
    );
    let page_identity = AssignmentNotebookStructurePlan {
        entries: vec![AssignmentNotebookStructureEntry {
            disposition: AssignmentNotebookDisposition::Represented {
                block: 3,
            },
            role: AssignmentNotebookRole::Explanation,
        }],
    };
    assert_eq!(
        validate_assignment_notebook_structure(&notebook(), &page_identity),
        Err(AssignmentNotebookStructureError::NotBlock { entry_index: 0 }),
    );
}
#[test]
fn projection_rejects_complete_plan_before_exposing_any_blocks() {
    let notebook = notebook();
    let plan = AssignmentNotebookStructurePlan {
        entries: vec![
            AssignmentNotebookStructureEntry {
                disposition: AssignmentNotebookDisposition::Represented {
                    block: 11,
                },
                role: AssignmentNotebookRole::Explanation,
            },
            AssignmentNotebookStructureEntry {
                disposition: AssignmentNotebookDisposition::Represented {
                    block: 99,
                },
                role: AssignmentNotebookRole::Conclusion,
            },
        ],
    };
    assert_eq!(
        project_assignment_notebook_structure_blocks(&notebook, &plan),
        Err(AssignmentNotebookStructureError::UnknownIdentity {
            entry_index: 1,
        }),
    );
}

#[test]
fn every_role_obeys_only_block_resolution_authority() {
    let notebook = notebook();
    let mut cases = 0u8;
    for role in ASSIGNMENT_NOTEBOOK_ROLES {
        let checks = [
            (
                AssignmentNotebookDisposition::Represented { block: 11 },
                Ok(()),
            ),
            (
                AssignmentNotebookDisposition::UnresolvedMissingFact {
                    block: 12,
                },
                Ok(()),
            ),
            (
                AssignmentNotebookDisposition::Represented { block: 12 },
                Err(AssignmentNotebookStructureError::RepresentedIsUnresolved {
                    entry_index: 0,
                }),
            ),
            (
                AssignmentNotebookDisposition::UnresolvedMissingFact {
                    block: 11,
                },
                Err(
                    AssignmentNotebookStructureError::
                        UnresolvedMissingFactIsResolved { entry_index: 0 },
                ),
            ),
            (
                AssignmentNotebookDisposition::Represented { block: 3 },
                Err(AssignmentNotebookStructureError::NotBlock {
                    entry_index: 0,
                }),
            ),
            (
                AssignmentNotebookDisposition::Represented { block: 99 },
                Err(AssignmentNotebookStructureError::UnknownIdentity {
                    entry_index: 0,
                }),
            ),
        ];
        for (disposition, expected) in checks {
            let plan = AssignmentNotebookStructurePlan {
                entries: vec![AssignmentNotebookStructureEntry {
                    disposition,
                    role,
                }],
            };
            assert_eq!(
                validate_assignment_notebook_structure(&notebook, &plan),
                expected,
                "role {role:?}, disposition {disposition:?}",
            );
            let projection =
                project_assignment_notebook_structure_blocks(&notebook, &plan);
            match expected {
                Ok(()) => {
                    let projected = require_ok!(projection);
                    let expected_id = match disposition {
                        AssignmentNotebookDisposition::Represented { block }
                        | AssignmentNotebookDisposition::UnresolvedMissingFact {
                            block,
                        } => block,
                    };
                    assert_eq!(
                        projected.first().map(|block| block.id),
                        Some(expected_id)
                    );
                }
                Err(error) => assert_eq!(projection, Err(error)),
            }
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 54);
}

#[test]
fn every_three_entry_disposition_tuple_preserves_first_failure_and_atomicity() {
    let notebook = notebook();
    let roles = [
        AssignmentNotebookRole::Title,
        AssignmentNotebookRole::Example,
        AssignmentNotebookRole::Conclusion,
    ];

    let mut cases = 0u16;
    for first in MATRIX_STATES {
        for second in MATRIX_STATES {
            for third in MATRIX_STATES {
                let states = [first, second, third];
                let plan = AssignmentNotebookStructurePlan {
                    entries: states
                        .into_iter()
                        .zip(roles)
                        .map(|(state, role)| AssignmentNotebookStructureEntry {
                            disposition: fixture_disposition(state),
                            role,
                        })
                        .collect(),
                };
                let expected = states.into_iter().enumerate().find_map(
                    |(entry_index, state)| fixture_error(state, entry_index),
                );
                assert_eq!(
                    validate_assignment_notebook_structure(&notebook, &plan),
                    expected.map_or(Ok(()), Err),
                    "states {states:?}",
                );
                let projection = project_assignment_notebook_structure_blocks(
                    &notebook, &plan,
                );
                let validated = validate_assignment_notebook_structure_view(
                    &notebook, &plan,
                );
                if let Some(error) = expected {
                    assert_eq!(projection, Err(error), "states {states:?}");
                    assert_eq!(validated.err(), Some(error));
                } else {
                    let projected = require_ok!(projection);
                    let expected_ids = states.map(|state| {
                        match fixture_disposition(state) {
                            AssignmentNotebookDisposition::Represented { block }
                            | AssignmentNotebookDisposition::
                                UnresolvedMissingFact { block } => block,
                        }
                    });
                    let projected_ids = projected
                        .iter()
                        .map(|block| block.id)
                        .collect::<Vec<_>>();
                    assert_eq!(
                        projected_ids, expected_ids,
                        "states {states:?}",
                    );
                    let sealed = require_ok!(validated);
                    assert!(ptr::eq(sealed.notebook(), &raw const notebook));
                    assert!(ptr::eq(sealed.plan(), &raw const plan));
                    assert_eq!(
                        sealed
                            .blocks()
                            .iter()
                            .map(|block| block.id)
                            .collect::<Vec<_>>(),
                        projected_ids,
                        "sealed states {states:?}",
                    );
                }
                cases = cases.saturating_add(1);
            }
        }
    }
    assert_eq!(cases, 216);
}
