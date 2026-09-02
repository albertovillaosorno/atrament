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
    PageProfile as PhysicalPageProfile, PageProfileError, PaperMarkAppearance,
    PaperMarkJoin, PaperMarkLayer, PaperPattern, Rect, SheetSize,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, Asset, Block, BlockContent, CandidateIdentity,
    Constraint, ConstraintKind, ExtensionData, Figure, Flow, Formula,
    FormulaMode, IdentityAllocator, InlineSpan, List, ListItem,
    MathSyntaxError, MathSyntaxErrorKind, Notebook, OutputProfile, Page,
    PaperProfile, Provenance, ProvenanceKind, SemanticBlockKind,
    SemanticIdentityDescriptor, SemanticIdentityKind, Style, Table, TableCell,
    TableRow, TableRowRole, UnresolvedBlock, UnresolvedReason,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CandidateGraphError, CandidateReferenceKind,
    FormulaEditOutcome, IdentityInspectOutcome, IdentityKindInspectOutcome,
    IdentityOwnerExpectation, IdentityPrecondition,
    IdentityPreconditionOutcome, PageProfileEditOutcome,
    SemanticNotebookSession, TableRowRoleEditOutcome, TextEditOutcome,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;

fn physical_page_profile() -> PhysicalPageProfile {
    PhysicalPageProfile {
        binding_edge: BindingEdge::Left,
        border_shape: BorderShape::RoundedRectangle,
        corner_roundness: Length::from_micrometres(5_000),
        orientation: Orientation::Portrait,
        outer_margin: Length::from_micrometres(20_000),
        paper_mark_appearance: PaperMarkAppearance {
            join: PaperMarkJoin::Rounded {
                radius: Length::from_micrometres(250),
            },
            maximum_ruler_error: Length::from_micrometres(200),
        },
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

fn candidate_math_notebook(
    identities: &IdentityAllocator,
    source: &str,
    mode: FormulaMode,
) -> (Notebook<CandidateIdentity>, CandidateIdentity) {
    let notebook_id = candidate_id(identities);
    let page_id = candidate_id(identities);
    let page_profile_id = candidate_id(identities);
    let flow_id = candidate_id(identities);
    let block_id = candidate_id(identities);
    let formula_id = candidate_id(identities);
    let notebook = Notebook {
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
                blocks: vec![Block {
                    content: BlockContent::Mathematics(Formula {
                        id: formula_id,
                        mode,
                        source: source.to_owned(),
                    }),
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
    (notebook, formula_id)
}

fn candidate_table_notebook(
    identities: &IdentityAllocator,
    role: TableRowRole,
) -> (
    Notebook<CandidateIdentity>,
    CandidateIdentity,
    CandidateIdentity,
) {
    let mut notebook = candidate_notebook(identities, "table cell text");
    let table_id = candidate_id(identities);
    let row_id = candidate_id(identities);
    let cell_id = candidate_id(identities);
    let nested_block_id = candidate_id(identities);
    let outer = &mut notebook.pages[0].flows[0].blocks[0];
    let cell_content =
        std::mem::replace(&mut outer.content, BlockContent::Rule);
    outer.content = BlockContent::Table(Table {
        id: table_id,
        rows: vec![TableRow {
            cells: vec![TableCell {
                blocks: vec![Block {
                    content: cell_content,
                    extensions: vec![],
                    id: nested_block_id,
                    provenance: None,
                    style: None,
                }],
                id: cell_id,
            }],
            id: row_id,
            role,
        }],
    });
    (notebook, row_id, table_id)
}

#[test]
fn accepted_aligned_mathematics_preserves_exact_source_and_mode() {
    let ids = IdentityAllocator::new();
    let source = r"y &= (3x^2 + 1)^5 \\ y' &= 30x(3x^2 + 1)^4";
    let (candidate, formula) =
        candidate_math_notebook(&ids, source, FormulaMode::Aligned);
    let mut session = SemanticNotebookSessionService::default();
    let outcome = session.accept(candidate);
    let AcceptanceOutcome::Accepted { mapping, .. } = outcome else {
        panic!("supported aligned formula must be accepted: {outcome:?}");
    };
    let accepted_formula = accepted_for(&mapping, formula);
    let current = session.current().expect("accepted revision");
    let BlockContent::Mathematics(stored) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("formula block must remain mathematics");
    };
    assert_eq!(stored.id, accepted_formula);
    assert_eq!(stored.mode, FormulaMode::Aligned);
    assert_eq!(stored.source, source);
}

#[test]
fn unsupported_mathematics_rejects_atomically_instead_of_substituting() {
    let ids = IdentityAllocator::new();
    let valid = candidate_notebook(&ids, "accepted text");
    let (unsupported, formula) =
        candidate_math_notebook(&ids, r"x + \mystery{y}", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let _ = session.accept(valid);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.accept(unsupported),
        AcceptanceOutcome::InvalidCandidate {
            reason: CandidateGraphError::UnsupportedMathematics {
                candidate: formula,
            },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn malformed_mathematics_rejects_atomically_with_typed_syntax_failure() {
    let ids = IdentityAllocator::new();
    let valid = candidate_notebook(&ids, "accepted text");
    let (malformed, formula) =
        candidate_math_notebook(&ids, r"\frac{1}", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let _ = session.accept(valid);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.accept(malformed),
        AcceptanceOutcome::InvalidCandidate {
            reason: CandidateGraphError::InvalidMathematics {
                candidate: formula,
                reason: MathSyntaxError {
                    byte_offset: 8,
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                },
            },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn unsupported_mathematics_can_be_preserved_as_exact_unresolved_source() {
    let ids = IdentityAllocator::new();
    let mut candidate = candidate_notebook(&ids, "placeholder");
    let source = r"x + \mystery{y}";
    candidate.pages[0].flows[0].blocks[0].content =
        BlockContent::Unresolved(UnresolvedBlock {
            extensions: vec![],
            reason: UnresolvedReason::Unsupported,
            source: source.to_owned(),
        });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { .. } = session.accept(candidate) else {
        panic!("typed unresolved source must remain admissible");
    };
    let current = session.current().expect("accepted revision");
    let BlockContent::Unresolved(unresolved) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("unsupported source must remain unresolved");
    };
    assert_eq!(unresolved.reason, UnresolvedReason::Unsupported);
    assert_eq!(unresolved.source, source);
}

#[test]
fn exact_revision_identity_inspection_exposes_kind_and_structural_owner() {
    let ids = IdentityAllocator::new();
    let (candidate, row, table) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let notebook = candidate.id;
    let flow = candidate.pages[0].flows[0].id;
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let notebook = accepted_for(&mapping, notebook);
    let flow = accepted_for(&mapping, flow);
    let block = accepted_for(&mapping, block);
    let row = accepted_for(&mapping, row);
    let table = accepted_for(&mapping, table);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.inspect_identity(revision, notebook),
        IdentityInspectOutcome::Inspected {
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Notebook,
                owner: None,
            },
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.inspect_identity(revision, block),
        IdentityInspectOutcome::Inspected {
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Block(SemanticBlockKind::Table),
                owner: Some(flow),
            },
            revision,
            target: block,
        },
    );
    assert_eq!(
        session.inspect_identity(revision, table),
        IdentityInspectOutcome::Inspected {
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Table,
                owner: Some(block),
            },
            revision,
            target: table,
        },
    );
    assert_eq!(
        session.inspect_identity(revision, row),
        IdentityInspectOutcome::Inspected {
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::TableRow,
                owner: Some(table),
            },
            revision,
            target: row,
        },
    );
    assert_eq!(
        session.inspect_identity_kind(revision, row),
        IdentityKindInspectOutcome::Inspected {
            kind: SemanticIdentityKind::TableRow,
            revision,
            target: row,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn identity_kind_inspection_rejects_stale_absent_and_empty_session() {
    let ids = IdentityAllocator::new();
    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let row = accepted_for(&mapping, row);
    let replacement = candidate_notebook(&ids, "replacement");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement must be accepted");
    };
    assert_eq!(
        session.inspect_identity(revision, row),
        IdentityInspectOutcome::StaleBase { current },
    );
    assert_eq!(
        session.inspect_identity_kind(revision, row),
        IdentityKindInspectOutcome::StaleBase { current },
    );
    assert_eq!(
        session.inspect_identity(current, row),
        IdentityInspectOutcome::TargetNotFound {
            revision: current,
            target: row,
        },
    );
    assert_eq!(
        session.inspect_identity_kind(current, row),
        IdentityKindInspectOutcome::TargetNotFound {
            revision: current,
            target: row,
        },
    );
    let empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.inspect_identity(current, row),
        IdentityInspectOutcome::NoAcceptedRevision,
    );
    assert_eq!(
        empty.inspect_identity_kind(current, row),
        IdentityKindInspectOutcome::NoAcceptedRevision,
    );
}

#[test]
fn local_identity_precondition_accepts_exact_kind_and_owner_read_only() {
    let ids = IdentityAllocator::new();
    let (candidate, row, table) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let notebook = candidate.id;
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let notebook = accepted_for(&mapping, notebook);
    let block = accepted_for(&mapping, block);
    let flow = accepted_for(&mapping, flow);
    let row = accepted_for(&mapping, row);
    let table = accepted_for(&mapping, table);
    let before = session.current().expect("accepted revision").clone();

    assert_eq!(
        session.check_identity_precondition(
            revision,
            block,
            IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::Block(
                    SemanticBlockKind::Table,
                )),
                expected_owner: IdentityOwnerExpectation::Direct(flow),
            },
        ),
        IdentityPreconditionOutcome::Satisfied {
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Block(SemanticBlockKind::Table),
                owner: Some(flow),
            },
            revision,
            target: block,
        },
    );
    assert_eq!(
        session.check_identity_precondition(
            revision,
            row,
            IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::TableRow),
                expected_owner: IdentityOwnerExpectation::Direct(table),
            },
        ),
        IdentityPreconditionOutcome::Satisfied {
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::TableRow,
                owner: Some(table),
            },
            revision,
            target: row,
        },
    );
    assert_eq!(
        session.check_identity_precondition(
            revision,
            notebook,
            IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::Notebook),
                expected_owner: IdentityOwnerExpectation::Root,
            },
        ),
        IdentityPreconditionOutcome::Satisfied {
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Notebook,
                owner: None,
            },
            revision,
            target: notebook,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn local_identity_precondition_reports_kind_and_owner_mismatch() {
    let ids = IdentityAllocator::new();
    let (candidate, row, table) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let block = accepted_for(&mapping, block);
    let flow = accepted_for(&mapping, flow);
    let row = accepted_for(&mapping, row);
    let table = accepted_for(&mapping, table);
    let before = session.current().expect("accepted revision").clone();

    assert_eq!(
        session.check_identity_precondition(
            revision,
            block,
            IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::Block(
                    SemanticBlockKind::Paragraph,
                )),
                expected_owner: IdentityOwnerExpectation::Direct(flow),
            },
        ),
        IdentityPreconditionOutcome::KindMismatch {
            actual: SemanticIdentityKind::Block(SemanticBlockKind::Table),
            expected: SemanticIdentityKind::Block(SemanticBlockKind::Paragraph),
            revision,
            target: block,
        },
    );
    assert_eq!(
        session.check_identity_precondition(
            revision,
            row,
            IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::TableRow),
                expected_owner: IdentityOwnerExpectation::Direct(block),
            },
        ),
        IdentityPreconditionOutcome::OwnerMismatch {
            actual: Some(table),
            expected: IdentityOwnerExpectation::Direct(block),
            revision,
            target: row,
        },
    );
    assert_eq!(
        session.check_identity_precondition(
            revision,
            row,
            IdentityPrecondition {
                expected_kind: None,
                expected_owner: IdentityOwnerExpectation::Root,
            },
        ),
        IdentityPreconditionOutcome::OwnerMismatch {
            actual: Some(table),
            expected: IdentityOwnerExpectation::Root,
            revision,
            target: row,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn local_identity_precondition_stale_missing_and_empty_are_typed() {
    let ids = IdentityAllocator::new();
    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let row = accepted_for(&mapping, row);
    let replacement = candidate_notebook(&ids, "replacement");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    let any = IdentityPrecondition {
        expected_kind: None,
        expected_owner: IdentityOwnerExpectation::Any,
    };
    assert_eq!(
        session.check_identity_precondition(revision, row, any),
        IdentityPreconditionOutcome::StaleBase { current },
    );
    assert_eq!(
        session.check_identity_precondition(current, row, any),
        IdentityPreconditionOutcome::TargetNotFound {
            revision: current,
            target: row,
        },
    );
    let empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.check_identity_precondition(current, row, any),
        IdentityPreconditionOutcome::NoAcceptedRevision,
    );
}

#[test]
fn direct_formula_edit_preserves_formula_identity_and_commits_one_revision() {
    let ids = IdentityAllocator::new();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "E = mc^2", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    let source = r"E &= K + U \\ K &= \frac{1}{2}mv^2";
    let outcome = session.replace_formula(
        revision,
        formula,
        FormulaMode::Aligned,
        source.to_owned(),
    );
    let FormulaEditOutcome::Applied {
        base,
        revision: edited,
        target,
    } = outcome
    else {
        panic!("supported formula edit must apply: {outcome:?}");
    };
    assert_eq!(base, revision);
    assert_ne!(edited, revision);
    assert_eq!(target, formula);
    let current = session.current().expect("edited revision");
    let stored = formula_value_for_test(current, formula);
    assert_eq!(stored.id, formula);
    assert_eq!(stored.mode, FormulaMode::Aligned);
    assert_eq!(stored.source, source);
}

#[test]
fn direct_formula_edit_same_value_is_noop_without_revision_churn() {
    let ids = IdentityAllocator::new();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "E = mc^2", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    assert_eq!(
        session.replace_formula(
            revision,
            formula,
            FormulaMode::Display,
            String::from("E = mc^2"),
        ),
        FormulaEditOutcome::NoOp {
            revision,
            target: formula,
        },
    );
    assert_eq!(session.current().expect("revision").id, revision);
}

#[test]
fn direct_formula_edit_rejects_invalid_and_unsupported_without_mutation() {
    let ids = IdentityAllocator::new();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "E = mc^2", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.replace_formula(
            revision,
            formula,
            FormulaMode::Display,
            String::from(r"\frac{1}"),
        ),
        FormulaEditOutcome::InvalidMathematics {
            reason: MathSyntaxError {
                byte_offset: 8,
                kind: MathSyntaxErrorKind::MissingRequiredGroup,
            },
            revision,
            target: formula,
        },
    );
    assert_eq!(session.current(), Some(&before));
    assert_eq!(
        session.replace_formula(
            revision,
            formula,
            FormulaMode::Display,
            String::from(r"\mystery{x}"),
        ),
        FormulaEditOutcome::UnsupportedMathematics {
            revision,
            target: formula,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_formula_edit_stale_nonformula_and_absent_targets_are_no_effect() {
    let ids = IdentityAllocator::new();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "E = mc^2", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    let page =
        session.current().expect("accepted revision").notebook.pages[0].id;
    let FormulaEditOutcome::Applied { revision: edited, .. } = session
        .replace_formula(
            revision,
            formula,
            FormulaMode::Display,
            String::from("E = mc^3"),
        )
    else {
        panic!("first formula edit must apply");
    };
    let before = session.current().expect("edited revision").clone();
    assert_eq!(
        session.replace_formula(
            revision,
            formula,
            FormulaMode::Display,
            String::from("E = mc^4"),
        ),
        FormulaEditOutcome::StaleBase { current: edited },
    );
    assert_eq!(
        session.replace_formula(
            edited,
            page,
            FormulaMode::Display,
            String::from("x = 1"),
        ),
        FormulaEditOutcome::TargetNotFormula {
            revision: edited,
            target: page,
        },
    );
    assert_eq!(session.current(), Some(&before));

    let replacement = candidate_notebook(&ids, "replacement");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.replace_formula(
            current,
            formula,
            FormulaMode::Display,
            String::from("x = 1"),
        ),
        FormulaEditOutcome::TargetNotFound {
            revision: current,
            target: formula,
        },
    );
}

#[test]
fn direct_formula_edit_without_accepted_revision_is_typed_no_effect() {
    let ids = IdentityAllocator::new();
    let mut seed = SemanticNotebookSessionService::default();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "x = 1", FormulaMode::Display);
    let AcceptanceOutcome::Accepted { mapping, revision } =
        seed.accept(candidate)
    else {
        panic!("seed candidate must be accepted");
    };
    let accepted = accepted_for(&mapping, formula);
    let mut empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.replace_formula(
            revision,
            accepted,
            FormulaMode::Display,
            String::from("x = 2"),
        ),
        FormulaEditOutcome::NoAcceptedRevision,
    );
}

fn formula_value_for_test(
    revision: &atrament_semantic_notebook::AcceptedRevision,
    target: AcceptedIdentity,
) -> &Formula<AcceptedIdentity> {
    for page in &revision.notebook.pages {
        for flow in &page.flows {
            for block in &flow.blocks {
                if let BlockContent::Mathematics(formula) = &block.content
                    && formula.id == target
                {
                    return formula;
                }
            }
        }
    }
    panic!("accepted formula target must exist");
}

#[test]
fn direct_formula_edit_reaches_nested_structures_across_revisions() {
    let ids = IdentityAllocator::new();
    let notebook_id = candidate_id(&ids);
    let profile_id = candidate_id(&ids);
    let page_id = candidate_id(&ids);
    let flow_id = candidate_id(&ids);
    let callout_owner = candidate_id(&ids);
    let callout_block = candidate_id(&ids);
    let callout_formula = candidate_id(&ids);
    let list_owner = candidate_id(&ids);
    let list_id = candidate_id(&ids);
    let list_item = candidate_id(&ids);
    let list_block = candidate_id(&ids);
    let list_formula = candidate_id(&ids);
    let table_owner = candidate_id(&ids);
    let table_id = candidate_id(&ids);
    let table_row = candidate_id(&ids);
    let table_cell = candidate_id(&ids);
    let table_block = candidate_id(&ids);
    let table_formula = candidate_id(&ids);
    let candidate = Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: notebook_id,
        output_profiles: vec![],
        page_profiles: vec![PaperProfile {
            geometry: physical_page_profile(),
            id: profile_id,
        }],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![
                    Block {
                        content: BlockContent::Callout(vec![Block {
                            content: BlockContent::Mathematics(Formula {
                                id: callout_formula,
                                mode: FormulaMode::Display,
                                source: String::from("a = 1"),
                            }),
                            extensions: vec![],
                            id: callout_block,
                            provenance: None,
                            style: None,
                        }]),
                        extensions: vec![],
                        id: callout_owner,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::List(List {
                            id: list_id,
                            items: vec![ListItem {
                                blocks: vec![Block {
                                    content: BlockContent::Mathematics(
                                        Formula {
                                            id: list_formula,
                                            mode: FormulaMode::Inline,
                                            source: String::from("b = 2"),
                                        },
                                    ),
                                    extensions: vec![],
                                    id: list_block,
                                    provenance: None,
                                    style: None,
                                }],
                                id: list_item,
                            }],
                            ordered: false,
                        }),
                        extensions: vec![],
                        id: list_owner,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Table(Table {
                            id: table_id,
                            rows: vec![TableRow {
                                cells: vec![TableCell {
                                    blocks: vec![Block {
                                        content: BlockContent::Mathematics(
                                            Formula {
                                                id: table_formula,
                                                mode: FormulaMode::Display,
                                                source: String::from("c = 3"),
                                            },
                                        ),
                                        extensions: vec![],
                                        id: table_block,
                                        provenance: None,
                                        style: None,
                                    }],
                                    id: table_cell,
                                }],
                                id: table_row,
                                role: TableRowRole::Body,
                            }],
                        }),
                        extensions: vec![],
                        id: table_owner,
                        provenance: None,
                        style: None,
                    },
                ],
                id: flow_id,
            }],
            id: page_id,
            page_profile: profile_id,
        }],
        provenance: vec![],
        styles: vec![],
    };
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, mut revision } =
        session.accept(candidate)
    else {
        panic!("nested formula candidate must be accepted");
    };
    for (candidate_formula, source) in [
        (callout_formula, "a = 10"),
        (list_formula, "b = 20"),
        (table_formula, "c = 30"),
    ] {
        let target = accepted_for(&mapping, candidate_formula);
        let FormulaEditOutcome::Applied { revision: next, .. } = session
            .replace_formula(
                revision,
                target,
                FormulaMode::Display,
                source.to_owned(),
            )
        else {
            panic!("nested formula edit must apply");
        };
        assert_ne!(next, revision);
        revision = next;
    }
    let current = session.current().expect("nested formula revision");
    let blocks = &current.notebook.pages[0].flows[0].blocks;
    let BlockContent::Callout(callout) = &blocks[0].content else {
        panic!("callout must remain a callout");
    };
    let BlockContent::Mathematics(callout_math) = &callout[0].content else {
        panic!("callout child must remain mathematics");
    };
    assert_eq!(callout_math.source, "a = 10");
    let BlockContent::List(list) = &blocks[1].content else {
        panic!("list must remain a list");
    };
    let BlockContent::Mathematics(list_math) = &list.items[0].blocks[0].content
    else {
        panic!("list child must remain mathematics");
    };
    assert_eq!(list_math.mode, FormulaMode::Display);
    assert_eq!(list_math.source, "b = 20");
    let BlockContent::Table(table) = &blocks[2].content else {
        panic!("table must remain a table");
    };
    let BlockContent::Mathematics(table_math) =
        &table.rows[0].cells[0].blocks[0].content
    else {
        panic!("table child must remain mathematics");
    };
    assert_eq!(table_math.source, "c = 30");
    assert_eq!(current.id, revision);
}

#[test]
fn direct_table_row_role_edit_preserves_row_identity_and_cells() {
    let ids = IdentityAllocator::new();
    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let target = accepted_for(&mapping, row);
    let before = session.current().expect("base revision").clone();
    let BlockContent::Table(before_table) =
        &before.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must remain a table");
    };
    let cells = before_table.rows[0].cells.clone();

    let outcome =
        session.replace_table_row_role(revision, target, TableRowRole::Body);
    let TableRowRoleEditOutcome::Applied {
        base,
        revision: edited,
        target: actual_target,
    } = outcome
    else {
        panic!("row role edit must apply: {outcome:?}");
    };
    assert_eq!(base, revision);
    assert_ne!(edited, revision);
    assert_eq!(actual_target, target);
    let current = session.current().expect("edited revision");
    let BlockContent::Table(table) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("edited fixture must remain a table");
    };
    assert_eq!(table.rows[0].id, target);
    assert_eq!(table.rows[0].role, TableRowRole::Body);
    assert_eq!(table.rows[0].cells, cells);
}

#[test]
fn direct_table_row_role_same_value_is_noop_without_revision_churn() {
    let ids = IdentityAllocator::new();
    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let target = accepted_for(&mapping, row);
    assert_eq!(
        session.replace_table_row_role(revision, target, TableRowRole::Header,),
        TableRowRoleEditOutcome::NoOp { revision, target },
    );
    assert_eq!(session.current().expect("revision").id, revision);
}

#[test]
fn direct_table_row_role_stale_nonrow_and_absent_targets_are_no_effect() {
    let ids = IdentityAllocator::new();
    let (candidate, row, table) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let row = accepted_for(&mapping, row);
    let table = accepted_for(&mapping, table);
    let TableRowRoleEditOutcome::Applied { revision: edited, .. } =
        session.replace_table_row_role(revision, row, TableRowRole::Body)
    else {
        panic!("initial role edit must apply");
    };
    let before = session.current().expect("edited revision").clone();
    assert_eq!(
        session.replace_table_row_role(revision, row, TableRowRole::Header,),
        TableRowRoleEditOutcome::StaleBase { current: edited },
    );
    assert_eq!(
        session.replace_table_row_role(edited, table, TableRowRole::Header,),
        TableRowRoleEditOutcome::TargetNotTableRow {
            revision: edited,
            target: table,
        },
    );
    assert_eq!(session.current(), Some(&before));

    let replacement = candidate_notebook(&ids, "replacement");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.replace_table_row_role(current, row, TableRowRole::Header,),
        TableRowRoleEditOutcome::TargetNotFound {
            revision: current,
            target: row,
        },
    );
}

#[test]
fn direct_table_row_role_without_accepted_revision_is_typed_no_effect() {
    let ids = IdentityAllocator::new();
    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let mut seed = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        seed.accept(candidate)
    else {
        panic!("seed table must be accepted");
    };
    let target = accepted_for(&mapping, row);
    let mut empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.replace_table_row_role(revision, target, TableRowRole::Header,),
        TableRowRoleEditOutcome::NoAcceptedRevision,
    );
}

fn candidate_table_block(
    identities: &IdentityAllocator,
    role: TableRowRole,
) -> (Block<CandidateIdentity>, CandidateIdentity) {
    let block_id = candidate_id(identities);
    let table_id = candidate_id(identities);
    let row_id = candidate_id(identities);
    let cell_id = candidate_id(identities);
    let child_id = candidate_id(identities);
    (
        Block {
            content: BlockContent::Table(Table {
                id: table_id,
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        blocks: vec![Block {
                            content: BlockContent::Rule,
                            extensions: vec![],
                            id: child_id,
                            provenance: None,
                            style: None,
                        }],
                        id: cell_id,
                    }],
                    id: row_id,
                    role,
                }],
            }),
            extensions: vec![],
            id: block_id,
            provenance: None,
            style: None,
        },
        row_id,
    )
}

#[test]
fn direct_table_row_role_edit_reaches_nested_structures_across_revisions() {
    let ids = IdentityAllocator::new();
    let mut candidate = candidate_notebook(&ids, "discarded seed");
    let (callout_table, callout_row) =
        candidate_table_block(&ids, TableRowRole::Body);
    let (list_table, list_row) =
        candidate_table_block(&ids, TableRowRole::Body);
    let (cell_table, cell_row) =
        candidate_table_block(&ids, TableRowRole::Body);
    let callout_id = candidate_id(&ids);
    let list_block_id = candidate_id(&ids);
    let list_id = candidate_id(&ids);
    let list_item_id = candidate_id(&ids);
    let outer_table_block_id = candidate_id(&ids);
    let outer_table_id = candidate_id(&ids);
    let outer_row_id = candidate_id(&ids);
    let outer_cell_id = candidate_id(&ids);
    candidate.pages[0].flows[0].blocks = vec![
        Block {
            content: BlockContent::Callout(vec![callout_table]),
            extensions: vec![],
            id: callout_id,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::List(List {
                id: list_id,
                items: vec![ListItem {
                    blocks: vec![list_table],
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
                id: outer_table_id,
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        blocks: vec![cell_table],
                        id: outer_cell_id,
                    }],
                    id: outer_row_id,
                    role: TableRowRole::Body,
                }],
            }),
            extensions: vec![],
            id: outer_table_block_id,
            provenance: None,
            style: None,
        },
    ];
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, mut revision } =
        session.accept(candidate)
    else {
        panic!("nested table candidate must be accepted");
    };
    for candidate_row in [callout_row, list_row, cell_row] {
        let target = accepted_for(&mapping, candidate_row);
        let TableRowRoleEditOutcome::Applied {
            revision: next,
            target: actual_target,
            ..
        } = session.replace_table_row_role(
            revision,
            target,
            TableRowRole::Header,
        )
        else {
            panic!("nested row-role edit must apply");
        };
        assert_eq!(actual_target, target);
        assert_ne!(next, revision);
        revision = next;
    }
    let current = session.current().expect("nested row-role revision");
    let blocks = &current.notebook.pages[0].flows[0].blocks;
    let BlockContent::Callout(callout) = &blocks[0].content else {
        panic!("first block must remain callout");
    };
    let BlockContent::Table(callout_table) = &callout[0].content else {
        panic!("callout child must remain table");
    };
    assert_eq!(callout_table.rows[0].role, TableRowRole::Header);
    let BlockContent::List(list) = &blocks[1].content else {
        panic!("second block must remain list");
    };
    let BlockContent::Table(list_table) = &list.items[0].blocks[0].content
    else {
        panic!("list child must remain table");
    };
    assert_eq!(list_table.rows[0].role, TableRowRole::Header);
    let BlockContent::Table(outer_table) = &blocks[2].content else {
        panic!("third block must remain table");
    };
    let BlockContent::Table(cell_table) =
        &outer_table.rows[0].cells[0].blocks[0].content
    else {
        panic!("table-cell child must remain table");
    };
    assert_eq!(cell_table.rows[0].role, TableRowRole::Header);
    assert_eq!(outer_table.rows[0].role, TableRowRole::Body);
    assert_eq!(current.id, revision);
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
                            mode: FormulaMode::Display,
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
                                role: TableRowRole::Header,
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
    let table_block = &current.notebook.pages[0].flows[0].blocks[3];
    let BlockContent::Table(table) = &table_block.content else {
        panic!("fourth block must remain a table");
    };
    assert_eq!(table.rows[0].id, accepted_for(&mapping, table_row_id));
    assert_eq!(table.rows[0].role, TableRowRole::Header);
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
                                role: TableRowRole::Body,
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

#[test]
fn direct_page_profile_edit_preserves_profile_and_page_identity() {
    let candidate_ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&candidate_ids, "paper edit");
    let candidate_profile = candidate.page_profiles[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_profile);
    let before = session.current().expect("base revision").clone();
    let mut geometry = physical_page_profile();
    geometry.outer_margin = Length::from_micrometres(25_000);

    let outcome = session.replace_page_profile(base, target, geometry);
    let PageProfileEditOutcome::Applied {
        base: actual_base,
        revision,
        target: actual_target,
    } = outcome
    else {
        panic!("changed physical profile must create a revision");
    };
    assert_eq!(actual_base, base);
    assert_eq!(actual_target, target);
    assert_ne!(revision, base);
    let after = session.current().expect("profile-edited revision");
    assert_eq!(after.id, revision);
    assert_eq!(after.notebook.id, before.notebook.id);
    assert_eq!(after.notebook.pages[0].id, before.notebook.pages[0].id);
    assert_eq!(after.notebook.page_profiles[0].id, target);
    assert_eq!(after.notebook.pages[0].page_profile, target);
    assert_eq!(after.notebook.page_profiles[0].geometry, geometry);
    assert_eq!(
        after.notebook.pages[0].flows,
        before.notebook.pages[0].flows,
    );
}

#[test]
fn direct_page_profile_edit_same_value_is_noop() {
    let candidate_ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&candidate_ids, "paper no-op");
    let candidate_profile = candidate.page_profiles[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_profile);
    let before = session.current().expect("base revision").clone();

    assert_eq!(
        session.replace_page_profile(revision, target, physical_page_profile()),
        PageProfileEditOutcome::NoOp { revision, target },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_page_profile_edit_invalid_geometry_is_no_effect() {
    let candidate_ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&candidate_ids, "paper invalid");
    let candidate_profile = candidate.page_profiles[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_profile);
    let before = session.current().expect("base revision").clone();
    let mut invalid = physical_page_profile();
    invalid.paper_pattern = PaperPattern::Ruled { spacing: Length::ZERO };

    assert_eq!(
        session.replace_page_profile(revision, target, invalid),
        PageProfileEditOutcome::InvalidProfile {
            reason: PageProfileError::PatternSpacingIsZero,
            revision,
            target,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_page_profile_edit_stale_base_rejects_without_mutation() {
    let candidate_ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&candidate_ids, "paper stale");
    let candidate_profile = candidate.page_profiles[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted {
        mapping,
        revision: stale_base,
    } = session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_profile);
    let mut geometry = physical_page_profile();
    geometry.outer_margin = Length::from_micrometres(25_000);
    let PageProfileEditOutcome::Applied { revision: current, .. } =
        session.replace_page_profile(stale_base, target, geometry)
    else {
        panic!("first profile edit must apply");
    };
    let before = session.current().expect("current revision").clone();
    geometry.outer_margin = Length::from_micrometres(30_000);

    assert_eq!(
        session.replace_page_profile(stale_base, target, geometry),
        PageProfileEditOutcome::StaleBase { current },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_page_profile_edit_nonprofile_identity_rejects_without_mutation() {
    let candidate_ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&candidate_ids, "paper target");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { revision, .. } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let before = session.current().expect("base revision").clone();
    let page_target = before.notebook.pages[0].id;

    assert_eq!(
        session.replace_page_profile(
            revision,
            page_target,
            physical_page_profile(),
        ),
        PageProfileEditOutcome::TargetNotPageProfile {
            revision,
            target: page_target,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_page_profile_edit_absent_prior_profile_is_not_found() {
    let candidate_ids = IdentityAllocator::new();
    let first = candidate_notebook(&candidate_ids, "first paper");
    let first_profile = first.page_profiles[0].id;
    let second = candidate_notebook(&candidate_ids, "second paper");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted {
        mapping: first_mapping, ..
    } = session.accept(first)
    else {
        panic!("first candidate must be accepted");
    };
    let absent_target = accepted_for(&first_mapping, first_profile);
    let AcceptanceOutcome::Accepted { revision, .. } = session.accept(second)
    else {
        panic!("second candidate must be accepted");
    };
    let before = session.current().expect("second revision").clone();

    assert_eq!(
        session.replace_page_profile(
            revision,
            absent_target,
            physical_page_profile(),
        ),
        PageProfileEditOutcome::TargetNotFound {
            revision,
            target: absent_target,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_page_profile_edit_without_accepted_revision_is_typed_no_effect() {
    let ids = IdentityAllocator::new();
    let base = ids.allocate_revision().expect("synthetic revision");
    let target = ids.allocate_accepted().expect("synthetic accepted id");
    let mut session = SemanticNotebookSessionService::default();

    assert_eq!(
        session.replace_page_profile(base, target, physical_page_profile()),
        PageProfileEditOutcome::NoAcceptedRevision,
    );
    assert!(session.current().is_none());
}
