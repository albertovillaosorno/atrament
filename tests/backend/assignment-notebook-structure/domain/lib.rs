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
use atrament_assignment_notebook_structure::{
    ASSIGNMENT_NOTEBOOK_ROLES, AssignmentNotebookDisposition,
    AssignmentNotebookRole, AssignmentNotebookStructureEntry,
    AssignmentNotebookStructureError, AssignmentNotebookStructurePlan,
    project_assignment_notebook_structure_blocks,
    validate_assignment_notebook_structure,
};
use atrament_semantic_notebook::{
    Block, BlockContent, Flow, InlineSpan, Notebook, Page, UnresolvedBlock,
    UnresolvedReason,
};

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
    assert_eq!(plan.entries[0].role, AssignmentNotebookRole::Title);
    assert_eq!(plan.entries[1].role, AssignmentNotebookRole::Example);
    let notebook = notebook();
    let projected = project_assignment_notebook_structure_blocks(
        &notebook,
        &plan,
    )
    .expect("validated structure projects exact source blocks");
    assert_eq!(
        projected.iter().map(|block| block.id).collect::<Vec<_>>(),
        [10, 11, 12],
    );
    assert!(matches!(projected[0].content, BlockContent::Heading(_)));
    assert!(matches!(projected[2].content, BlockContent::Unresolved(_)));
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
    let projected = project_assignment_notebook_structure_blocks(
        &notebook,
        &repeated,
    )
    .expect("repeated review entries remain valid");
    assert_eq!(projected.len(), 2);
    assert!(std::ptr::eq(projected[0], projected[1]));
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
    let mut cases = 0_u8;
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
            let projection = project_assignment_notebook_structure_blocks(
                &notebook,
                &plan,
            );
            match expected {
                Ok(()) => assert_eq!(
                    projection
                        .expect("valid case projects one exact block")[0]
                        .id,
                    match disposition {
                        AssignmentNotebookDisposition::Represented { block }
                        | AssignmentNotebookDisposition::UnresolvedMissingFact {
                            block,
                        } => block,
                    },
                ),
                Err(error) => assert_eq!(projection, Err(error)),
            }
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 54);
}
