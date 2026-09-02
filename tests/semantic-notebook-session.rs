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
//   - Regression evidence for transactional candidate notebook acceptance.
// - Must-Not:
//   - Parse model output, persist revisions, or define layout/render behavior.
// - Allows:
//   - Inputs: Deterministic candidate semantic notebooks and invalid ID graphs.
//   - Outputs: Assertions over atomic acceptance and stable identity promotion.
//   - Side effects: Process-local test allocation and accepted-state mutation.
// - Split-When:
//   - Candidate acceptance needs independently versioned negative fixtures.
// - Merge-When:
//   - Semantic acceptance is covered by a broader application transaction test.
// - Summary:
//   - Verifies candidate identities become accepted only through one commit.
// - Description:
//   - Covers first acceptance, replacement, duplicate IDs, and dangling refs.
// - Usage:
//   - Compile directly against semantic notebook application components.
// - Defaults:
//   - Rejected candidates leave the previously accepted revision unchanged.
//
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation,
    PageProfile as PhysicalPageProfile, PageProfileError, PaperMarkLayer,
    PaperPattern, Rect, SheetSize,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, Asset, Block, BlockContent, CandidateIdentity,
    Constraint, ConstraintKind, ExtensionData, Figure, Flow, Formula,
    IdentityAllocator, InlineSpan, List, ListItem, Notebook, OutputProfile,
    Page, PaperProfile, Provenance, ProvenanceKind, Style, Table, TableCell,
    TableRow, UnresolvedBlock, UnresolvedReason,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CandidateGraphError, CandidateReferenceKind,
    SemanticNotebookSession, TextEditOutcome,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;

fn physical_page_profile() -> PhysicalPageProfile {
    PhysicalPageProfile {
        binding_edge: BindingEdge::Left,
        border_shape: BorderShape::RoundedRectangle,
        corner_roundness: Length::from_micrometres(5_000),
        orientation: Orientation::Portrait,
        outer_margin: Length::from_micrometres(20_000),
        paper_mark_layer: PaperMarkLayer::BelowInk,
        paper_pattern: PaperPattern::Squared {
            spacing: Length::from_micrometres(5_000),
        },
        printable_region: Rect {
            height: Length::from_micrometres(277_000),
            width: Length::from_micrometres(190_000),
            x: Length::from_micrometres(10_000),
            y: Length::from_micrometres(10_000),
        },
        sheet: SheetSize {
            height: Length::from_micrometres(297_000),
            width: Length::from_micrometres(210_000),
        },
        top_clearance: Length::from_micrometres(10_000),
        writing_inset: Length::from_micrometres(5_000),
    }
}

fn accepted_for(
    mapping: &[atrament_semantic_notebook_port::IdentityMapping],
    candidate: CandidateIdentity,
) -> AcceptedIdentity {
    mapping
        .iter()
        .find(|entry| entry.candidate == candidate)
        .expect("candidate identity is mapped")
        .accepted
}

fn candidate_id(identities: &IdentityAllocator) -> CandidateIdentity {
    identities.allocate_candidate().expect("candidate id")
}

fn candidate_notebook(
    identities: &IdentityAllocator,
    text: &str,
) -> Notebook<CandidateIdentity> {
    candidate_notebook_with_span(identities, text).0
}

fn candidate_notebook_with_span(
    identities: &IdentityAllocator,
    text: &str,
) -> (Notebook<CandidateIdentity>, CandidateIdentity) {
    let notebook_id = identities.allocate_candidate().expect("notebook id");
    let page_id = identities.allocate_candidate().expect("page id");
    let page_profile_id =
        identities.allocate_candidate().expect("page profile id");
    let flow_id = identities.allocate_candidate().expect("flow id");
    let block_id = identities.allocate_candidate().expect("block id");
    let span_id = identities.allocate_candidate().expect("span id");
    let notebook = Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![ExtensionData {
            namespace: String::from("fixture.extension/1"),
            payload: vec![4, 2],
        }],
        id: notebook_id,
        output_profiles: vec![],
        page_profiles: vec![PaperProfile {
            geometry: physical_page_profile(),
            id: page_profile_id,
        }],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![Block {
                    content: BlockContent::Paragraph(vec![InlineSpan {
                        id: span_id,
                        provenance: None,
                        style: None,
                        text: String::from(text),
                    }]),
                    extensions: vec![],
                    id: block_id,
                    provenance: None,
                    style: None,
                }],
                id: flow_id,
            }],
            id: page_id,
            page_profile: page_profile_id,
        }],
        provenance: vec![],
        styles: vec![],
    };
    (notebook, span_id)
}

#[test]
fn first_candidate_acceptance_commits_one_revision_and_identity_mapping() {
    let candidate_ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&candidate_ids, "candidate text");
    let mut session = SemanticNotebookSessionService::default();
    assert!(session.current().is_none());

    let outcome = session.accept(candidate);
    let AcceptanceOutcome::Accepted { mapping, revision } = outcome else {
        panic!("valid candidate must be accepted");
    };
    assert_eq!(mapping.len(), 6);
    let current = session.current().expect("accepted revision");
    assert_eq!(current.id, revision);
    assert_eq!(current.notebook.extensions[0].payload, [4, 2]);
    assert_eq!(current.notebook.page_profiles.len(), 1);
    assert_eq!(
        current.notebook.page_profiles[0].geometry,
        physical_page_profile()
    );
    assert_eq!(
        current.notebook.pages[0].page_profile,
        current.notebook.page_profiles[0].id,
    );
}

#[test]
fn repeated_candidate_acceptance_allocates_new_revision_and_semantic_ids() {
    let candidate_ids = IdentityAllocator::new();
    let first = candidate_notebook(&candidate_ids, "same semantics");
    let second = candidate_notebook(&candidate_ids, "same semantics");
    let mut session = SemanticNotebookSessionService::default();

    let AcceptanceOutcome::Accepted {
        mapping: first_mapping,
        revision: first_revision,
    } = session.accept(first)
    else {
        panic!("first candidate must be accepted");
    };
    let AcceptanceOutcome::Accepted {
        mapping: second_mapping,
        revision: second_revision,
    } = session.accept(second)
    else {
        panic!("second candidate must be accepted");
    };

    assert_ne!(first_revision, second_revision);
    for first_identity in first_mapping {
        assert!(
            second_mapping
                .iter()
                .all(|second| second.accepted != first_identity.accepted),
        );
    }
}

#[test]
fn duplicate_candidate_identity_rejects_without_changing_current_revision() {
    let candidate_ids = IdentityAllocator::new();
    let valid = candidate_notebook(&candidate_ids, "accepted");
    let mut invalid = candidate_notebook(&candidate_ids, "duplicate");
    invalid.pages[0].id = invalid.id;
    let mut session = SemanticNotebookSessionService::default();
    let _ = session.accept(valid);
    let before = session.current().expect("accepted revision").clone();

    let outcome = session.accept(invalid);
    assert!(matches!(outcome, AcceptanceOutcome::InvalidCandidate {
        reason: CandidateGraphError::Duplicate { .. },
    }));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn dangling_candidate_reference_rejects_without_changing_current_revision() {
    let candidate_ids = IdentityAllocator::new();
    let valid = candidate_notebook(&candidate_ids, "accepted");
    let mut invalid = candidate_notebook(&candidate_ids, "dangling");
    let missing = candidate_ids.allocate_candidate().expect("dangling id");
    let constraint_id = candidate_ids.allocate_candidate().expect("constraint");
    invalid.constraints.push(Constraint {
        id: constraint_id,
        kind: ConstraintKind::Placement,
        target: missing,
    });
    let mut session = SemanticNotebookSessionService::default();
    let _ = session.accept(valid);
    let before = session.current().expect("accepted revision").clone();

    let outcome = session.accept(invalid);
    assert!(matches!(outcome, AcceptanceOutcome::InvalidCandidate {
        reason: CandidateGraphError::MissingReference { .. },
    }));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn invalid_page_profile_rejects_without_changing_current_revision() {
    let candidate_ids = IdentityAllocator::new();
    let valid = candidate_notebook(&candidate_ids, "accepted");
    let mut invalid = candidate_notebook(&candidate_ids, "invalid paper");
    invalid.page_profiles[0].geometry.printable_region.width = Length::ZERO;
    let invalid_profile = invalid.page_profiles[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let _ = session.accept(valid);
    let before = session.current().expect("accepted revision").clone();

    let outcome = session.accept(invalid);
    assert_eq!(outcome, AcceptanceOutcome::InvalidCandidate {
        reason: CandidateGraphError::InvalidPageProfile {
            candidate: invalid_profile,
            reason: PageProfileError::PrintableRegionIsEmpty,
        },
    },);
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn page_profile_reference_kind_rejects_without_changing_current_revision() {
    let candidate_ids = IdentityAllocator::new();
    let valid = candidate_notebook(&candidate_ids, "accepted");
    let mut invalid = candidate_notebook(&candidate_ids, "wrong paper owner");
    invalid.pages[0].page_profile = invalid.pages[0].id;
    let wrong_owner = invalid.pages[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let _ = session.accept(valid);
    let before = session.current().expect("accepted revision").clone();

    let outcome = session.accept(invalid);
    assert_eq!(outcome, AcceptanceOutcome::InvalidCandidate {
        reason: CandidateGraphError::ReferenceKindMismatch {
            candidate: wrong_owner,
            expected: CandidateReferenceKind::PageProfile,
        },
    },);
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn debug_output_never_exposes_accepted_notebook_text() {
    let candidate_ids = IdentityAllocator::new();
    let candidate = candidate_notebook(
        &candidate_ids,
        "private accepted notebook sentence",
    );
    let mut session = SemanticNotebookSessionService::default();
    let _ = session.accept(candidate);

    let debug = format!("{session:?}");
    assert!(!debug.contains("private accepted notebook sentence"));
    assert!(debug.contains("SemanticNotebookSessionService"));
}

#[test]
fn nested_semantic_families_promote_all_owned_and_referenced_identities() {
    let ids = IdentityAllocator::new();
    let asset_id = candidate_id(&ids);
    let block_callout_id = candidate_id(&ids);
    let block_figure_id = candidate_id(&ids);
    let block_formula_id = candidate_id(&ids);
    let block_freeform_id = candidate_id(&ids);
    let block_list_id = candidate_id(&ids);
    let block_rule_id = candidate_id(&ids);
    let block_table_id = candidate_id(&ids);
    let block_unresolved_id = candidate_id(&ids);
    let callout_child_id = candidate_id(&ids);
    let constraint_id = candidate_id(&ids);
    let figure_id = candidate_id(&ids);
    let flow_id = candidate_id(&ids);
    let formula_id = candidate_id(&ids);
    let freeform_child_id = candidate_id(&ids);
    let list_id = candidate_id(&ids);
    let list_item_id = candidate_id(&ids);
    let list_span_id = candidate_id(&ids);
    let notebook_id = candidate_id(&ids);
    let page_id = candidate_id(&ids);
    let page_profile_id = candidate_id(&ids);
    let profile_id = candidate_id(&ids);
    let provenance_id = candidate_id(&ids);
    let style_id = candidate_id(&ids);
    let table_cell_id = candidate_id(&ids);
    let table_id = candidate_id(&ids);
    let table_row_id = candidate_id(&ids);
    let table_span_id = candidate_id(&ids);
    let figure_span_id = candidate_id(&ids);

    let candidate = Notebook {
        assets: vec![Asset {
            id: asset_id,
            media_type: String::from("image/png"),
        }],
        constraints: vec![Constraint {
            id: constraint_id,
            kind: ConstraintKind::Placement,
            target: block_figure_id,
        }],
        extensions: vec![],
        id: notebook_id,
        output_profiles: vec![OutputProfile {
            id: profile_id,
            name: String::from("digital"),
        }],
        page_profiles: vec![PaperProfile {
            geometry: physical_page_profile(),
            id: page_profile_id,
        }],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![
                    Block {
                        content: BlockContent::Figure(Figure {
                            asset: Some(asset_id),
                            caption: vec![InlineSpan {
                                id: figure_span_id,
                                provenance: Some(provenance_id),
                                style: Some(style_id),
                                text: String::from("semantic figure"),
                            }],
                            id: figure_id,
                        }),
                        extensions: vec![],
                        id: block_figure_id,
                        provenance: Some(provenance_id),
                        style: Some(style_id),
                    },
                    Block {
                        content: BlockContent::Mathematics(Formula {
                            id: formula_id,
                            source: String::from("E = mc^2"),
                        }),
                        extensions: vec![],
                        id: block_formula_id,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::List(List {
                            id: list_id,
                            items: vec![ListItem {
                                blocks: vec![Block {
                                    content: BlockContent::Paragraph(vec![
                                        InlineSpan {
                                            id: list_span_id,
                                            provenance: None,
                                            style: Some(style_id),
                                            text: String::from("list item"),
                                        },
                                    ]),
                                    extensions: vec![],
                                    id: block_list_id,
                                    provenance: None,
                                    style: None,
                                }],
                                id: list_item_id,
                            }],
                            ordered: true,
                        }),
                        extensions: vec![],
                        id: block_rule_id,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Table(Table {
                            id: table_id,
                            rows: vec![TableRow {
                                cells: vec![TableCell {
                                    blocks: vec![Block {
                                        content: BlockContent::Paragraph(vec![
                                            InlineSpan {
                                                id: table_span_id,
                                                provenance: None,
                                                style: None,
                                                text: String::from("cell"),
                                            },
                                        ]),
                                        extensions: vec![],
                                        id: block_table_id,
                                        provenance: None,
                                        style: None,
                                    }],
                                    id: table_cell_id,
                                }],
                                id: table_row_id,
                            }],
                        }),
                        extensions: vec![],
                        id: block_callout_id,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Callout(vec![Block {
                            content: BlockContent::Rule,
                            extensions: vec![],
                            id: callout_child_id,
                            provenance: None,
                            style: None,
                        }]),
                        extensions: vec![],
                        id: block_freeform_id,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Freeform(vec![Block {
                            content: BlockContent::Unresolved(
                                UnresolvedBlock {
                                    extensions: vec![],
                                    reason: UnresolvedReason::Ambiguous,
                                    source: String::from("ambiguous fragment"),
                                },
                            ),
                            extensions: vec![],
                            id: freeform_child_id,
                            provenance: None,
                            style: None,
                        }]),
                        extensions: vec![],
                        id: block_unresolved_id,
                        provenance: None,
                        style: None,
                    },
                ],
                id: flow_id,
            }],
            id: page_id,
            page_profile: page_profile_id,
        }],
        provenance: vec![Provenance {
            id: provenance_id,
            kind: ProvenanceKind::Supplied,
            reference: Some(String::from("source note 1")),
        }],
        styles: vec![Style {
            id: style_id,
            name: String::from("body"),
        }],
    };

    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, .. } = session.accept(candidate)
    else {
        panic!("rich candidate must be accepted");
    };
    let current = session.current().expect("accepted rich revision");
    let figure_block = &current.notebook.pages[0].flows[0].blocks[0];
    let BlockContent::Figure(figure) = &figure_block.content else {
        panic!("first block must remain a figure");
    };
    assert_eq!(figure.asset, Some(accepted_for(&mapping, asset_id)));
    assert_eq!(figure.id, accepted_for(&mapping, figure_id));
    assert_eq!(figure_block.style, Some(accepted_for(&mapping, style_id)));
    assert_eq!(
        figure.caption[0].provenance,
        Some(accepted_for(&mapping, provenance_id)),
    );
    assert_eq!(
        current.notebook.constraints[0].target,
        accepted_for(&mapping, block_figure_id),
    );
    assert_eq!(
        current.notebook.styles[0].id,
        accepted_for(&mapping, style_id)
    );
    assert_eq!(
        current.notebook.provenance[0].id,
        accepted_for(&mapping, provenance_id),
    );
    assert_eq!(
        current.notebook.output_profiles[0].id,
        accepted_for(&mapping, profile_id),
    );
    assert_eq!(
        current.notebook.page_profiles[0].id,
        accepted_for(&mapping, page_profile_id),
    );
    assert_eq!(
        current.notebook.pages[0].page_profile,
        accepted_for(&mapping, page_profile_id),
    );
}

#[test]
fn wrong_reference_kind_rejects_without_changing_current_revision() {
    let candidate_ids = IdentityAllocator::new();
    let valid = candidate_notebook(&candidate_ids, "accepted");
    let mut invalid = candidate_notebook(&candidate_ids, "wrong reference");
    let page_id = invalid.pages[0].id;
    let BlockContent::Paragraph(spans) =
        &mut invalid.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture block must be a paragraph");
    };
    spans[0].style = Some(page_id);
    let mut session = SemanticNotebookSessionService::default();
    let _ = session.accept(valid);
    let before = session.current().expect("accepted revision").clone();

    let outcome = session.accept(invalid);
    assert!(matches!(outcome, AcceptanceOutcome::InvalidCandidate {
        reason: CandidateGraphError::ReferenceKindMismatch {
            expected: CandidateReferenceKind::Style,
            ..
        },
    }));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn dropped_semantic_session_does_not_seed_a_fresh_service() {
    let candidate_ids = IdentityAllocator::new();
    let candidate =
        candidate_notebook(&candidate_ids, "ephemeral accepted text");
    {
        let mut first = SemanticNotebookSessionService::default();
        assert!(matches!(
            first.accept(candidate),
            AcceptanceOutcome::Accepted { .. }
        ));
        assert!(first.current().is_some());
    }

    let fresh = SemanticNotebookSessionService::default();
    assert!(fresh.current().is_none());
}

#[test]
fn direct_text_edit_changes_one_span_and_preserves_all_semantic_identities() {
    let candidate_ids = IdentityAllocator::new();
    let original = concat!(
        "Si y = f(g(x)), entonces la derivada exterior se evalúa en g(x) y ",
        "se multiplica por la derivada interior.",
    );
    let corrected = concat!(
        "Si y = f(g(x)), la derivada exterior se evalúa en g(x) y se ",
        "multiplica por la derivada interior.",
    );
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&candidate_ids, original);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    let before = session.current().expect("base revision").clone();

    let outcome = session.replace_text(base, target, String::from(corrected));
    let TextEditOutcome::Applied {
        base: actual_base,
        revision,
        target: actual_target,
    } = outcome
    else {
        panic!("changed text must create a new revision");
    };
    assert_eq!(actual_base, base);
    assert_eq!(actual_target, target);
    assert_ne!(revision, base);

    let after = session.current().expect("edited revision");
    assert_eq!(after.id, revision);
    assert_eq!(after.notebook.id, before.notebook.id);
    assert_eq!(after.notebook.pages[0].id, before.notebook.pages[0].id);
    assert_eq!(
        after.notebook.pages[0].flows[0].id,
        before.notebook.pages[0].flows[0].id,
    );
    let before_block = &before.notebook.pages[0].flows[0].blocks[0];
    let after_block = &after.notebook.pages[0].flows[0].blocks[0];
    assert_eq!(after_block.id, before_block.id);
    let BlockContent::Paragraph(before_spans) = &before_block.content else {
        panic!("base block must be paragraph");
    };
    let BlockContent::Paragraph(after_spans) = &after_block.content else {
        panic!("edited block must remain paragraph");
    };
    assert_eq!(before_spans[0].id, target);
    assert_eq!(after_spans[0].id, target);
    assert_eq!(before_spans[0].text, original);
    assert_eq!(after_spans[0].text, corrected);
    assert_eq!(after_spans[0].provenance, before_spans[0].provenance);
    assert_eq!(after_spans[0].style, before_spans[0].style);
    assert_eq!(after.notebook.extensions, before.notebook.extensions);
}

#[test]
fn direct_text_edit_same_value_is_noop_without_revision_churn() {
    let candidate_ids = IdentityAllocator::new();
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&candidate_ids, "same text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    let before = session.current().expect("base revision").clone();

    assert_eq!(
        session.replace_text(revision, target, String::from("same text")),
        TextEditOutcome::NoOp { revision, target },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_text_edit_stale_base_rejects_without_mutation() {
    let candidate_ids = IdentityAllocator::new();
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&candidate_ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted {
        mapping,
        revision: stale_base,
    } = session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    let TextEditOutcome::Applied { revision: current, .. } =
        session.replace_text(stale_base, target, String::from("current text"))
    else {
        panic!("first edit must apply");
    };
    let before = session.current().expect("current revision").clone();

    assert_eq!(
        session.replace_text(stale_base, target, String::from("stale text")),
        TextEditOutcome::StaleBase { current },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_text_edit_non_text_identity_rejects_without_mutation() {
    let candidate_ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&candidate_ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { revision, .. } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let before = session.current().expect("base revision").clone();
    let page_target = before.notebook.pages[0].id;

    assert_eq!(
        session.replace_text(revision, page_target, String::from("invalid")),
        TextEditOutcome::TargetNotText {
            revision,
            target: page_target,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_text_edit_without_accepted_revision_is_typed_no_effect() {
    let ids = IdentityAllocator::new();
    let base = ids.allocate_revision().expect("synthetic revision");
    let target = ids.allocate_accepted().expect("synthetic accepted id");
    let mut session = SemanticNotebookSessionService::default();

    assert_eq!(
        session.replace_text(base, target, String::from("unavailable")),
        TextEditOutcome::NoAcceptedRevision,
    );
    assert!(session.current().is_none());
}

#[test]
fn direct_text_edit_reaches_nested_text_families_across_revisions() {
    let ids = IdentityAllocator::new();
    let notebook_id = candidate_id(&ids);
    let page_id = candidate_id(&ids);
    let page_profile_id = candidate_id(&ids);
    let flow_id = candidate_id(&ids);
    let callout_id = candidate_id(&ids);
    let callout_child_id = candidate_id(&ids);
    let callout_span_id = candidate_id(&ids);
    let figure_block_id = candidate_id(&ids);
    let figure_id = candidate_id(&ids);
    let figure_span_id = candidate_id(&ids);
    let list_block_id = candidate_id(&ids);
    let list_id = candidate_id(&ids);
    let list_item_id = candidate_id(&ids);
    let list_child_id = candidate_id(&ids);
    let list_span_id = candidate_id(&ids);
    let table_block_id = candidate_id(&ids);
    let table_id = candidate_id(&ids);
    let row_id = candidate_id(&ids);
    let cell_id = candidate_id(&ids);
    let table_child_id = candidate_id(&ids);
    let table_span_id = candidate_id(&ids);
    let candidate = Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: notebook_id,
        output_profiles: vec![],
        page_profiles: vec![PaperProfile {
            geometry: physical_page_profile(),
            id: page_profile_id,
        }],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![
                    Block {
                        content: BlockContent::Callout(vec![Block {
                            content: BlockContent::Paragraph(vec![
                                InlineSpan {
                                    id: callout_span_id,
                                    provenance: None,
                                    style: None,
                                    text: String::from("callout old"),
                                },
                            ]),
                            extensions: vec![],
                            id: callout_child_id,
                            provenance: None,
                            style: None,
                        }]),
                        extensions: vec![],
                        id: callout_id,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Figure(Figure {
                            asset: None,
                            caption: vec![InlineSpan {
                                id: figure_span_id,
                                provenance: None,
                                style: None,
                                text: String::from("figure old"),
                            }],
                            id: figure_id,
                        }),
                        extensions: vec![],
                        id: figure_block_id,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::List(List {
                            id: list_id,
                            items: vec![ListItem {
                                blocks: vec![Block {
                                    content: BlockContent::Paragraph(vec![
                                        InlineSpan {
                                            id: list_span_id,
                                            provenance: None,
                                            style: None,
                                            text: String::from("list old"),
                                        },
                                    ]),
                                    extensions: vec![],
                                    id: list_child_id,
                                    provenance: None,
                                    style: None,
                                }],
                                id: list_item_id,
                            }],
                            ordered: false,
                        }),
                        extensions: vec![],
                        id: list_block_id,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Table(Table {
                            id: table_id,
                            rows: vec![TableRow {
                                cells: vec![TableCell {
                                    blocks: vec![Block {
                                        content: BlockContent::Paragraph(vec![
                                            InlineSpan {
                                                id: table_span_id,
                                                provenance: None,
                                                style: None,
                                                text: String::from("table old"),
                                            },
                                        ]),
                                        extensions: vec![],
                                        id: table_child_id,
                                        provenance: None,
                                        style: None,
                                    }],
                                    id: cell_id,
                                }],
                                id: row_id,
                            }],
                        }),
                        extensions: vec![],
                        id: table_block_id,
                        provenance: None,
                        style: None,
                    },
                ],
                id: flow_id,
            }],
            id: page_id,
            page_profile: page_profile_id,
        }],
        provenance: vec![],
        styles: vec![],
    };
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, mut revision } =
        session.accept(candidate)
    else {
        panic!("nested candidate must be accepted");
    };
    let cases = [
        (callout_span_id, "callout new"),
        (figure_span_id, "figure new"),
        (list_span_id, "list new"),
        (table_span_id, "table new"),
    ];
    for (candidate_target, replacement) in cases {
        let target = accepted_for(&mapping, candidate_target);
        let TextEditOutcome::Applied {
            revision: next,
            target: actual_target,
            ..
        } = session.replace_text(revision, target, String::from(replacement))
        else {
            panic!("nested text edit must apply");
        };
        assert_eq!(actual_target, target);
        assert_ne!(next, revision);
        revision = next;
    }
    assert_eq!(session.current().expect("nested revision").id, revision);
}

#[test]
fn direct_text_edit_absent_identity_is_distinct_from_non_text_owner() {
    let candidate_ids = IdentityAllocator::new();
    let (first, first_span) =
        candidate_notebook_with_span(&candidate_ids, "first");
    let second = candidate_notebook(&candidate_ids, "second");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted {
        mapping: first_mapping, ..
    } = session.accept(first)
    else {
        panic!("first candidate must be accepted");
    };
    let absent_target = accepted_for(&first_mapping, first_span);
    let AcceptanceOutcome::Accepted { revision, .. } = session.accept(second)
    else {
        panic!("second candidate must be accepted");
    };
    let before = session.current().expect("second revision").clone();

    assert_eq!(
        session.replace_text(revision, absent_target, String::from("missing")),
        TextEditOutcome::TargetNotFound {
            revision,
            target: absent_target,
        },
    );
    assert_eq!(session.current(), Some(&before));
}
