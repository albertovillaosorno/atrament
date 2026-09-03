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
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation,
    PageProfile as PhysicalPageProfile, PageProfileError, PaperMarkAppearance,
    PaperMarkJoin, PaperMarkLayer, PaperPattern, Rect, SheetSize,
};
use atrament_semantic_command_graph::{
    CommandGraphError, CommandGraphLimitError, CommandGraphLimits,
    CommandGraphSize, DependencySelectionSummary, MissingDependencyRequirement,
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
    AcceptanceOutcome, CANDIDATE_BLOCK_NESTING_LIMIT, CandidateGraphError,
    CandidateReferenceKind, CommandBehaviorVersion,
    CommandCapabilityCompatibilityOutcome, CommandFamilyAdmissionOutcome,
    CommandFamilyCapability, CommandResourceLimits, CommandTargetMaterial,
    CommandTargetMaterialOutcome, CommandTargetPreconditionOutcome,
    CommandTargetPreconditions, DirectEditBatchCommand,
    DirectEditBatchCommandPrediction, DirectEditBatchCommandRejection,
    DirectEditBatchGraphLimitsOutcome, DirectEditBatchGraphSizeOutcome,
    DirectEditBatchProposal, DirectEditBatchSelectionBoundedOutcome,
    DirectEditBatchSelectionRequirementsOutcome,
    DirectEditBatchSelectionSummaryOutcome, DirectEditBatchSimulationOutcome,
    DirectEditChangePreviewOutcome, DirectEditDerivedAuthority,
    DirectEditEffectClass, DirectEditImpactScope, DirectEditImpactSeed,
    DirectEditProposal, DirectEditProposalOutcome, DirectEditSemanticChange,
    DirectEditSimulationOutcome, EditableSemanticValue,
    EditableSemanticValueKind, EditableValuePreconditionOutcome,
    FormulaEditOutcome, IdentityInspectOutcome, IdentityKindInspectOutcome,
    IdentityOwnerExpectation, IdentityPrecondition,
    IdentityPreconditionOutcome, PageProfileEditOutcome, SemanticCommandFamily,
    SemanticNotebookSession, TableRowRoleEditOutcome, TextEditOutcome,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;

#[derive(Debug)]
struct CountingCommandIdentity {
    clones: Arc<AtomicUsize>,
    id: u32,
}

impl CountingCommandIdentity {
    fn new(clones: &Arc<AtomicUsize>, id: u32) -> Self {
        Self {
            clones: Arc::clone(clones),
            id,
        }
    }
}

impl Clone for CountingCommandIdentity {
    fn clone(&self) -> Self {
        let _previous = self.clones.fetch_add(1, AtomicOrdering::Relaxed);
        Self::new(&self.clones, self.id)
    }
}

impl Eq for CountingCommandIdentity {}

impl Ord for CountingCommandIdentity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialEq for CountingCommandIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for CountingCommandIdentity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

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

fn candidate_notebook_with_three_spans(
    identities: &IdentityAllocator,
) -> (
    Notebook<CandidateIdentity>,
    CandidateIdentity,
    CandidateIdentity,
    CandidateIdentity,
) {
    let (mut notebook, first) = candidate_notebook_with_span(identities, "one");
    let second = candidate_id(identities);
    let third = candidate_id(identities);
    let BlockContent::Paragraph(spans) =
        &mut notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain one paragraph");
    };
    spans.push(InlineSpan {
        id: second,
        provenance: None,
        style: None,
        text: String::from("two"),
    });
    spans.push(InlineSpan {
        id: third,
        provenance: None,
        style: None,
        text: String::from("three"),
    });
    (notebook, first, second, third)
}

fn text_batch_command(
    id: u32,
    dependencies: &[u32],
    target: AcceptedIdentity,
    expected: &str,
    requested: &str,
) -> DirectEditBatchCommand<u32> {
    DirectEditBatchCommand {
        dependencies: dependencies.to_vec(),
        id,
        preconditions: CommandTargetPreconditions {
            expected_value: Some(EditableSemanticValue::Text(
                expected.to_owned(),
            )),
            identity: IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::InlineSpan),
                expected_owner: IdentityOwnerExpectation::Any,
            },
            requested_family: SemanticCommandFamily::TextContent,
        },
        requested: EditableSemanticValue::Text(requested.to_owned()),
        target,
    }
}

fn candidate_nested_text_notebook(
    identities: &IdentityAllocator,
    wrappers: usize,
) -> (
    Notebook<CandidateIdentity>,
    CandidateIdentity,
    CandidateIdentity,
) {
    let (mut notebook, span) =
        candidate_notebook_with_span(identities, "nested text");
    let mut block = notebook.pages[0].flows[0]
        .blocks
        .pop()
        .expect("paragraph block");
    let leaf = block.id;
    for _ in 0..wrappers {
        block = Block {
            content: BlockContent::Callout(vec![block]),
            extensions: vec![],
            id: candidate_id(identities),
            provenance: None,
            style: None,
        };
    }
    notebook.pages[0].flows[0].blocks.push(block);
    (notebook, leaf, span)
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
fn candidate_nesting_limit_accepts_the_exact_boundary() {
    let ids = IdentityAllocator::new();
    let (candidate, _, span) = candidate_nested_text_notebook(
        &ids,
        CANDIDATE_BLOCK_NESTING_LIMIT.saturating_sub(1),
    );
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate at the nesting bound must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let expected = EditableSemanticValue::Text(String::from("nested text"));
    assert_eq!(
        session.check_editable_value_precondition(
            revision,
            span,
            expected.clone(),
        ),
        EditableValuePreconditionOutcome::Satisfied {
            actual: expected,
            revision,
            target: span,
        },
    );
}

#[test]
fn candidate_nesting_limit_rejects_before_mutation_and_drops_safely() {
    let ids = IdentityAllocator::new();
    let baseline = candidate_notebook(&ids, "accepted baseline");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { .. } = session.accept(baseline) else {
        panic!("baseline must be accepted");
    };
    let before = session.current().expect("accepted baseline").clone();
    let (over_limit, leaf, _) =
        candidate_nested_text_notebook(&ids, CANDIDATE_BLOCK_NESTING_LIMIT);
    assert_eq!(
        session.accept(over_limit),
        AcceptanceOutcome::InvalidCandidate {
            reason: CandidateGraphError::NestingLimitExceeded {
                candidate: leaf,
                limit: CANDIDATE_BLOCK_NESTING_LIMIT,
            },
        },
    );
    assert_eq!(session.current(), Some(&before));

    let (extreme, _, _) = candidate_nested_text_notebook(&ids, 20_000);
    let AcceptanceOutcome::InvalidCandidate {
        reason: CandidateGraphError::NestingLimitExceeded { limit, .. },
    } = session.accept(extreme)
    else {
        panic!("extreme candidate must reject by resource limit");
    };
    assert_eq!(limit, CANDIDATE_BLOCK_NESTING_LIMIT);
    assert_eq!(session.current(), Some(&before));
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
fn command_target_preconditions_cover_local_mismatch_fixture_read_only() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "Idea base");
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let block = accepted_for(&mapping, block);
    let flow = accepted_for(&mapping, flow);
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let exact = CommandTargetPreconditions {
        expected_value: Some(EditableSemanticValue::Text(String::from(
            "Idea base",
        ))),
        identity: IdentityPrecondition {
            expected_kind: Some(SemanticIdentityKind::InlineSpan),
            expected_owner: IdentityOwnerExpectation::Direct(block),
        },
        requested_family: SemanticCommandFamily::TextContent,
    };
    let CommandTargetPreconditionOutcome::Satisfied { material } = session
        .check_command_target_preconditions(revision, span, exact.clone())
    else {
        panic!("exact local target preconditions must be satisfied");
    };
    assert_eq!(material.target, span);

    let mut wrong_family = exact.clone();
    wrong_family.requested_family = SemanticCommandFamily::StructuredContent;
    assert_eq!(
        session.check_command_target_preconditions(
            revision,
            span,
            wrong_family,
        ),
        CommandTargetPreconditionOutcome::FamilyNotExecutable {
            available: Some(SemanticCommandFamily::TextContent),
            requested: SemanticCommandFamily::StructuredContent,
            revision,
            target: span,
        },
    );
    let mut wrong_kind = exact.clone();
    wrong_kind.identity.expected_kind =
        Some(SemanticIdentityKind::Block(SemanticBlockKind::Paragraph));
    assert_eq!(
        session.check_command_target_preconditions(revision, span, wrong_kind),
        CommandTargetPreconditionOutcome::KindMismatch {
            actual: SemanticIdentityKind::InlineSpan,
            expected: SemanticIdentityKind::Block(SemanticBlockKind::Paragraph),
            revision,
            target: span,
        },
    );
    let mut wrong_owner = exact.clone();
    wrong_owner.identity.expected_owner =
        IdentityOwnerExpectation::Direct(flow);
    assert_eq!(
        session.check_command_target_preconditions(revision, span, wrong_owner),
        CommandTargetPreconditionOutcome::OwnerMismatch {
            actual: Some(block),
            expected: IdentityOwnerExpectation::Direct(flow),
            revision,
            target: span,
        },
    );
    let mut wrong_value = exact;
    wrong_value.expected_value =
        Some(EditableSemanticValue::Text(String::from("different base")));
    assert_eq!(
        session.check_command_target_preconditions(revision, span, wrong_value),
        CommandTargetPreconditionOutcome::ValueMismatch {
            actual: EditableSemanticValue::Text(String::from("Idea base")),
            expected: EditableSemanticValue::Text(String::from(
                "different base",
            )),
            revision,
            target: span,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn command_target_preconditions_do_not_invent_unicode_normalization() {
    let ids = IdentityAllocator::new();
    let nfc = "caf\u{e9}";
    let nfd = "cafe\u{301}";
    let (candidate, span) = candidate_notebook_with_span(&ids, nfc);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("Unicode text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let preconditions = CommandTargetPreconditions {
        expected_value: Some(EditableSemanticValue::Text(nfd.to_owned())),
        identity: IdentityPrecondition {
            expected_kind: Some(SemanticIdentityKind::InlineSpan),
            expected_owner: IdentityOwnerExpectation::Any,
        },
        requested_family: SemanticCommandFamily::TextContent,
    };
    assert_eq!(
        session.check_command_target_preconditions(
            revision,
            span,
            preconditions,
        ),
        CommandTargetPreconditionOutcome::ValueMismatch {
            actual: EditableSemanticValue::Text(nfc.to_owned()),
            expected: EditableSemanticValue::Text(nfd.to_owned()),
            revision,
            target: span,
        },
    );
}

#[test]
fn command_target_preconditions_stale_base_wins_before_local_checks() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "old base");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let replacement = candidate_notebook(&ids, "new current");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    let deliberately_wrong = CommandTargetPreconditions {
        expected_value: Some(EditableSemanticValue::Text(String::from(
            "wrong",
        ))),
        identity: IdentityPrecondition {
            expected_kind: Some(SemanticIdentityKind::TableRow),
            expected_owner: IdentityOwnerExpectation::Root,
        },
        requested_family: SemanticCommandFamily::StructuredContent,
    };
    assert_eq!(
        session.check_command_target_preconditions(
            revision,
            span,
            deliberately_wrong,
        ),
        CommandTargetPreconditionOutcome::StaleBase { current },
    );
}

#[test]
fn command_capability_snapshot_is_deterministic_and_does_not_overclaim() {
    let mut session = SemanticNotebookSessionService::default();
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CommandBehaviorVersion(1));
    assert_eq!(snapshot.typed_result_version, CommandBehaviorVersion(1));
    assert!(snapshot.admitted_applications.is_empty());
    assert!(snapshot.protocol_versions.is_empty());
    assert_eq!(snapshot.normalization_version, None);
    assert_eq!(snapshot.resource_limits, CommandResourceLimits {
        commands_per_batch: None,
        dependency_edges: None,
        envelope_bytes: None,
        readable_context_bytes: None,
        writable_targets: None,
    },);
    assert_eq!(snapshot.family_capabilities, [
        CommandFamilyCapability {
            behavior_version: CommandBehaviorVersion(1),
            family: SemanticCommandFamily::DocumentConstraint,
        },
        CommandFamilyCapability {
            behavior_version: CommandBehaviorVersion(1),
            family: SemanticCommandFamily::StructuredContent,
        },
        CommandFamilyCapability {
            behavior_version: CommandBehaviorVersion(1),
            family: SemanticCommandFamily::TextContent,
        },
    ],);
    assert_eq!(session.command_capability_snapshot(), snapshot);

    let ids = IdentityAllocator::new();
    let candidate =
        candidate_notebook(&ids, "capability metadata is not state");
    let AcceptanceOutcome::Accepted { .. } = session.accept(candidate) else {
        panic!("candidate must be accepted");
    };
    assert_eq!(session.command_capability_snapshot(), snapshot);
}

#[test]
fn command_capability_version_detects_drift_independently_of_revision() {
    let mut session = SemanticNotebookSessionService::default();
    let snapshot = session.command_capability_snapshot();
    assert_eq!(
        session
            .check_command_capability_compatibility(snapshot.behavior_version,),
        CommandCapabilityCompatibilityOutcome::Compatible { snapshot },
    );
    assert_eq!(
        session
            .check_command_capability_compatibility(CommandBehaviorVersion(0),),
        CommandCapabilityCompatibilityOutcome::Mismatch {
            current: CommandBehaviorVersion(1),
            expected: CommandBehaviorVersion(0),
        },
    );

    let ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&ids, "revision changes are separate");
    let AcceptanceOutcome::Accepted { .. } = session.accept(candidate) else {
        panic!("candidate must be accepted");
    };
    assert_eq!(
        session
            .check_command_capability_compatibility(snapshot.behavior_version,),
        CommandCapabilityCompatibilityOutcome::Compatible { snapshot },
    );
}

#[test]
fn command_family_admission_is_exact_target_scope_and_read_only() {
    let ids = IdentityAllocator::new();
    let (candidate, span) =
        candidate_notebook_with_span(&ids, "bounded writable text");
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let block = accepted_for(&mapping, block);
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();

    let CommandFamilyAdmissionOutcome::Admitted { material } = session
        .check_command_family_admission(
            revision,
            span,
            SemanticCommandFamily::TextContent,
        )
    else {
        panic!("text family must be admitted for text span");
    };
    assert_eq!(material.target, span);
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::TextContent),
    );
    assert_eq!(
        session.check_command_family_admission(
            revision,
            span,
            SemanticCommandFamily::StructuredContent,
        ),
        CommandFamilyAdmissionOutcome::FamilyNotExecutable {
            available: Some(SemanticCommandFamily::TextContent),
            requested: SemanticCommandFamily::StructuredContent,
            revision,
            target: span,
        },
    );
    assert_eq!(
        session.check_command_family_admission(
            revision,
            block,
            SemanticCommandFamily::TextContent,
        ),
        CommandFamilyAdmissionOutcome::FamilyNotExecutable {
            available: None,
            requested: SemanticCommandFamily::TextContent,
            revision,
            target: block,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn command_family_admission_stale_missing_and_empty_are_typed() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let replacement = candidate_notebook(&ids, "replacement");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.check_command_family_admission(
            revision,
            span,
            SemanticCommandFamily::TextContent,
        ),
        CommandFamilyAdmissionOutcome::StaleBase { current },
    );
    assert_eq!(
        session.check_command_family_admission(
            current,
            span,
            SemanticCommandFamily::TextContent,
        ),
        CommandFamilyAdmissionOutcome::TargetNotFound {
            revision: current,
            target: span,
        },
    );
    let empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.check_command_family_admission(
            current,
            span,
            SemanticCommandFamily::TextContent,
        ),
        CommandFamilyAdmissionOutcome::NoAcceptedRevision,
    );
}

#[test]
fn command_target_material_combines_owner_and_editable_value_read_only() {
    let ids = IdentityAllocator::new();
    let (candidate, span) =
        candidate_notebook_with_span(&ids, "No cambies esta fuente.");
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let block = accepted_for(&mapping, block);
    let flow = accepted_for(&mapping, flow);
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let expected_span = CommandTargetMaterialOutcome::Prepared {
        material: CommandTargetMaterial {
            direct_edit_family: Some(SemanticCommandFamily::TextContent),
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::InlineSpan,
                owner: Some(block),
            },
            editable_value: Some(EditableSemanticValue::Text(String::from(
                "No cambies esta fuente.",
            ))),
            revision,
            target: span,
        },
    };
    assert_eq!(
        session.command_target_material(revision, span),
        expected_span
    );
    assert_eq!(
        session.command_target_material(revision, span),
        expected_span
    );
    assert_eq!(
        session.command_target_material(revision, block),
        CommandTargetMaterialOutcome::Prepared {
            material: CommandTargetMaterial {
                direct_edit_family: None,
                descriptor: SemanticIdentityDescriptor {
                    kind: SemanticIdentityKind::Block(
                        SemanticBlockKind::Paragraph,
                    ),
                    owner: Some(flow),
                },
                editable_value: None,
                revision,
                target: block,
            },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn command_target_material_maps_current_direct_edit_families() {
    let ids = IdentityAllocator::new();
    let (formula_candidate, formula) =
        candidate_math_notebook(&ids, "x^2", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(formula_candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(revision, formula)
    else {
        panic!("formula target material must be prepared");
    };
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::StructuredContent),
    );

    let (table_candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let page_profile = table_candidate.page_profiles[0].id;
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(table_candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let row = accepted_for(&mapping, row);
    let page_profile = accepted_for(&mapping, page_profile);
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(revision, row)
    else {
        panic!("row target material must be prepared");
    };
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::StructuredContent),
    );
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(revision, page_profile)
    else {
        panic!("page-profile target material must be prepared");
    };
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::DocumentConstraint),
    );
}

#[test]
fn command_target_material_stale_missing_and_empty_are_typed() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let replacement = candidate_notebook(&ids, "new revision");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.command_target_material(revision, span),
        CommandTargetMaterialOutcome::StaleBase { current },
    );
    assert_eq!(
        session.command_target_material(current, span),
        CommandTargetMaterialOutcome::TargetNotFound {
            revision: current,
            target: span,
        },
    );
    let empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.command_target_material(current, span),
        CommandTargetMaterialOutcome::NoAcceptedRevision,
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
fn editable_value_precondition_accepts_exact_text_and_formula_read_only() {
    let ids = IdentityAllocator::new();
    let (text_candidate, span) =
        candidate_notebook_with_span(&ids, "energía cinética");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(text_candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let text = EditableSemanticValue::Text(String::from("energía cinética"));
    assert_eq!(
        session
            .check_editable_value_precondition(revision, span, text.clone(),),
        EditableValuePreconditionOutcome::Satisfied {
            actual: text,
            revision,
            target: span,
        },
    );
    assert_eq!(session.current(), Some(&before));

    let source = r"E &= K + U \\ K &= \frac{1}{2}mv^2";
    let (formula_candidate, formula) =
        candidate_math_notebook(&ids, source, FormulaMode::Aligned);
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(formula_candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    let before = session.current().expect("accepted revision").clone();
    let expected = EditableSemanticValue::Formula {
        mode: FormulaMode::Aligned,
        source: source.to_owned(),
    };
    assert_eq!(
        session.check_editable_value_precondition(
            revision,
            formula,
            expected.clone(),
        ),
        EditableValuePreconditionOutcome::Satisfied {
            actual: expected,
            revision,
            target: formula,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn editable_value_precondition_accepts_row_role_and_page_profile() {
    let ids = IdentityAllocator::new();
    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let page_profile = candidate.page_profiles[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let row = accepted_for(&mapping, row);
    let page_profile = accepted_for(&mapping, page_profile);
    let before = session.current().expect("accepted revision").clone();
    let role = EditableSemanticValue::TableRowRole(TableRowRole::Header);
    assert_eq!(
        session.check_editable_value_precondition(revision, row, role.clone(),),
        EditableValuePreconditionOutcome::Satisfied {
            actual: role,
            revision,
            target: row,
        },
    );
    let geometry = EditableSemanticValue::PageProfile(physical_page_profile());
    assert_eq!(
        session.check_editable_value_precondition(
            revision,
            page_profile,
            geometry.clone(),
        ),
        EditableValuePreconditionOutcome::Satisfied {
            actual: geometry,
            revision,
            target: page_profile,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn editable_value_precondition_reports_mismatch_and_noneditable_target() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let notebook = candidate.id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let notebook = accepted_for(&mapping, notebook);
    let before = session.current().expect("accepted revision").clone();
    let expected = EditableSemanticValue::Text(String::from("other text"));
    assert_eq!(
        session.check_editable_value_precondition(
            revision,
            span,
            expected.clone(),
        ),
        EditableValuePreconditionOutcome::ValueMismatch {
            actual: EditableSemanticValue::Text(String::from("base text")),
            expected,
            revision,
            target: span,
        },
    );
    let wrong_family = EditableSemanticValue::TableRowRole(TableRowRole::Body);
    assert_eq!(
        session.check_editable_value_precondition(
            revision,
            span,
            wrong_family.clone(),
        ),
        EditableValuePreconditionOutcome::ValueMismatch {
            actual: EditableSemanticValue::Text(String::from("base text")),
            expected: wrong_family,
            revision,
            target: span,
        },
    );
    assert_eq!(
        session.check_editable_value_precondition(
            revision,
            notebook,
            EditableSemanticValue::Text(String::new()),
        ),
        EditableValuePreconditionOutcome::TargetNotEditableValue {
            kind: SemanticIdentityKind::Notebook,
            revision,
            target: notebook,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn editable_value_precondition_stale_missing_and_empty_are_typed() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let replacement = candidate_notebook(&ids, "replacement");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    let expected = EditableSemanticValue::Text(String::from("base text"));
    assert_eq!(
        session.check_editable_value_precondition(
            revision,
            span,
            expected.clone(),
        ),
        EditableValuePreconditionOutcome::StaleBase { current },
    );
    assert_eq!(
        session.check_editable_value_precondition(
            current,
            span,
            expected.clone(),
        ),
        EditableValuePreconditionOutcome::TargetNotFound {
            revision: current,
            target: span,
        },
    );
    let empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.check_editable_value_precondition(current, span, expected),
        EditableValuePreconditionOutcome::NoAcceptedRevision,
    );
}

#[test]
fn direct_edit_proposal_binds_capability_preconditions_and_simulation() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "Idea base");
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let block = accepted_for(&mapping, block);
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let preconditions = CommandTargetPreconditions {
        expected_value: Some(EditableSemanticValue::Text(String::from(
            "Idea base",
        ))),
        identity: IdentityPrecondition {
            expected_kind: Some(SemanticIdentityKind::InlineSpan),
            expected_owner: IdentityOwnerExpectation::Direct(block),
        },
        requested_family: SemanticCommandFamily::TextContent,
    };
    let requested = EditableSemanticValue::Text(String::from("Idea final"));
    assert_eq!(
        session.simulate_direct_edit_proposal(DirectEditProposal {
            capability_version: CommandBehaviorVersion(1),
            preconditions: preconditions.clone(),
            requested: requested.clone(),
            revision,
            target: span,
        }),
        DirectEditProposalOutcome::Simulated {
            outcome: DirectEditSimulationOutcome::Applicable {
                family: SemanticCommandFamily::TextContent,
                requested,
                revision,
                target: span,
            },
        },
    );

    let mut wrong_value = preconditions.clone();
    wrong_value.expected_value =
        Some(EditableSemanticValue::Text(String::from("wrong base")));
    assert_eq!(
        session.simulate_direct_edit_proposal(DirectEditProposal {
            capability_version: CommandBehaviorVersion(1),
            preconditions: wrong_value,
            requested: EditableSemanticValue::Text(String::from("Idea final")),
            revision,
            target: span,
        }),
        DirectEditProposalOutcome::PreconditionRejected {
            outcome: CommandTargetPreconditionOutcome::ValueMismatch {
                actual: EditableSemanticValue::Text(String::from("Idea base")),
                expected: EditableSemanticValue::Text(String::from(
                    "wrong base",
                )),
                revision,
                target: span,
            },
        },
    );

    let mut wrong_family = preconditions.clone();
    wrong_family.requested_family = SemanticCommandFamily::StructuredContent;
    assert_eq!(
        session.simulate_direct_edit_proposal(DirectEditProposal {
            capability_version: CommandBehaviorVersion(1),
            preconditions: wrong_family,
            requested: EditableSemanticValue::Text(String::from("Idea final")),
            revision,
            target: span,
        }),
        DirectEditProposalOutcome::PreconditionRejected {
            outcome: CommandTargetPreconditionOutcome::FamilyNotExecutable {
                available: Some(SemanticCommandFamily::TextContent),
                requested: SemanticCommandFamily::StructuredContent,
                revision,
                target: span,
            },
        },
    );

    assert_eq!(
        session.simulate_direct_edit_proposal(DirectEditProposal {
            capability_version: CommandBehaviorVersion(1),
            preconditions: preconditions.clone(),
            requested: EditableSemanticValue::TableRowRole(TableRowRole::Body),
            revision,
            target: span,
        }),
        DirectEditProposalOutcome::Simulated {
            outcome: DirectEditSimulationOutcome::ValueFamilyMismatch {
                actual: EditableSemanticValueKind::Text,
                requested: EditableSemanticValueKind::TableRowRole,
                revision,
                target: span,
            },
        },
    );

    assert_eq!(
        session.simulate_direct_edit_proposal(DirectEditProposal {
            capability_version: CommandBehaviorVersion(0),
            preconditions,
            requested: EditableSemanticValue::Text(String::from("Idea final")),
            revision,
            target: span,
        }),
        DirectEditProposalOutcome::CapabilityMismatch {
            current: CommandBehaviorVersion(1),
            expected: CommandBehaviorVersion(0),
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_edit_proposal_stale_base_precedes_local_simulation() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let replacement = candidate_notebook(&ids, "replacement");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    let before = session.current().expect("current revision").clone();
    assert_eq!(
        session.simulate_direct_edit_proposal(DirectEditProposal {
            capability_version: CommandBehaviorVersion(1),
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::Text(
                    String::from("base",)
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::InlineSpan),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::TextContent,
            },
            requested: EditableSemanticValue::Text(String::from("after")),
            revision,
            target: span,
        }),
        DirectEditProposalOutcome::PreconditionRejected {
            outcome: CommandTargetPreconditionOutcome::StaleBase { current },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_edit_batch_selection_reports_transitive_requirements_read_only() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], span, "base text", "one"),
            text_batch_command(2, &[1], span, "one", "two"),
            text_batch_command(3, &[2], span, "two", "three"),
        ],
    };
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.direct_edit_batch_selection_requirements(
            &batch,
            &BTreeSet::from([3]),
        ),
        DirectEditBatchSelectionRequirementsOutcome::Requirements {
            missing: vec![
                MissingDependencyRequirement {
                    command: 2,
                    dependency: 1,
                },
                MissingDependencyRequirement {
                    command: 3,
                    dependency: 2,
                },
            ],
            revision,
        },
    );
    assert_eq!(
        session.direct_edit_batch_selection_requirements(
            &batch,
            &BTreeSet::from([1, 2, 3]),
        ),
        DirectEditBatchSelectionRequirementsOutcome::Requirements {
            missing: Vec::new(),
            revision,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_edit_batch_selection_preserves_global_failure_precedence() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let valid = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![text_batch_command(
            1,
            &[],
            span,
            "base text",
            "selected text",
        )],
    };
    let incompatible = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(0),
        commands: valid.commands.clone(),
    };
    assert_eq!(
        session.direct_edit_batch_selection_requirements(
            &incompatible,
            &BTreeSet::from([99]),
        ),
        DirectEditBatchSelectionRequirementsOutcome::CapabilityMismatch {
            current: CommandBehaviorVersion(1),
            expected: CommandBehaviorVersion(0),
        },
    );
    let empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.direct_edit_batch_selection_requirements(
            &valid,
            &BTreeSet::from([1]),
        ),
        DirectEditBatchSelectionRequirementsOutcome::NoAcceptedRevision,
    );

    let replacement = candidate_notebook(&ids, "new revision");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.direct_edit_batch_selection_requirements(
            &valid,
            &BTreeSet::from([99]),
        ),
        DirectEditBatchSelectionRequirementsOutcome::StaleBase { current },
    );

    let (_, current_span) = candidate_notebook_with_span(&ids, "unused");
    let _ = current_span;
    let current_target =
        session.current().expect("current revision").notebook.pages[0].flows[0]
            .blocks[0]
            .id;
    let invalid_graph = DirectEditBatchProposal {
        base: current,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[2], current_target, "new revision", "one"),
            text_batch_command(2, &[], current_target, "new revision", "two"),
        ],
    };
    assert_eq!(
        session.direct_edit_batch_selection_requirements(
            &invalid_graph,
            &BTreeSet::from([1]),
        ),
        DirectEditBatchSelectionRequirementsOutcome::DependencyGraphRejected {
            reason: CommandGraphError::DependencyAfterCommand {
                command: 1,
                dependency: 2,
            },
        },
    );
    let valid_current = DirectEditBatchProposal {
        base: current,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![text_batch_command(
            1,
            &[],
            current_target,
            "new revision",
            "selected text",
        )],
    };
    assert_eq!(
        session.direct_edit_batch_selection_requirements(
            &valid_current,
            &BTreeSet::from([99]),
        ),
        DirectEditBatchSelectionRequirementsOutcome::UnknownSelection {
            command: 99,
        },
    );
}

#[test]
fn direct_edit_batch_selection_summary_matches_transitive_requirements() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], span, "base text", "one"),
            text_batch_command(2, &[1], span, "one", "two"),
            text_batch_command(3, &[2], span, "two", "three"),
        ],
    };
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session
            .direct_edit_batch_selection_summary(&batch, &BTreeSet::from([3]),),
        DirectEditBatchSelectionSummaryOutcome::Summarized {
            revision,
            summary: DependencySelectionSummary {
                missing_dependency_edges: 2,
                required_commands: 3,
                selected_commands: 1,
            },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_edit_batch_selection_summary_preserves_global_precedence() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let valid = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![text_batch_command(
            1,
            &[],
            span,
            "base text",
            "selected text",
        )],
    };
    let incompatible = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(0),
        commands: valid.commands.clone(),
    };
    assert_eq!(
        session.direct_edit_batch_selection_summary(
            &incompatible,
            &BTreeSet::from([99]),
        ),
        DirectEditBatchSelectionSummaryOutcome::CapabilityMismatch {
            current: CommandBehaviorVersion(1),
            expected: CommandBehaviorVersion(0),
        },
    );
    let replacement = candidate_notebook(&ids, "new revision");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.direct_edit_batch_selection_summary(
            &valid,
            &BTreeSet::from([99]),
        ),
        DirectEditBatchSelectionSummaryOutcome::StaleBase { current },
    );
}

#[test]
fn direct_edit_batch_graph_resource_preflight_is_exact_and_read_only() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], span, "base text", "one"),
            text_batch_command(2, &[1], span, "one", "two"),
            text_batch_command(3, &[2], span, "two", "three"),
        ],
    };
    let size = CommandGraphSize {
        commands: 3,
        dependency_edges: 2,
    };
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.direct_edit_batch_graph_size(&batch),
        DirectEditBatchGraphSizeOutcome::Sized { revision, size },
    );
    assert_eq!(
        session.direct_edit_batch_graph_limits(&batch, CommandGraphLimits {
            commands: 3,
            dependency_edges: 2,
        },),
        DirectEditBatchGraphLimitsOutcome::Admitted { revision, size },
    );
    assert_eq!(
        session.direct_edit_batch_graph_limits(&batch, CommandGraphLimits {
            commands: 2,
            dependency_edges: 2,
        },),
        DirectEditBatchGraphLimitsOutcome::Rejected {
            reason: CommandGraphLimitError::CommandCountExceeded {
                actual: 3,
                limit: 2,
            },
        },
    );
    assert_eq!(
        session.direct_edit_batch_graph_limits(&batch, CommandGraphLimits {
            commands: 3,
            dependency_edges: 1,
        },),
        DirectEditBatchGraphLimitsOutcome::Rejected {
            reason: CommandGraphLimitError::DependencyEdgeCountExceeded {
                actual: 2,
                limit: 1,
            },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_edit_batch_graph_resources_preserve_authority_and_structure_layers() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let mut revision_source = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted {
        revision: unavailable_revision,
        ..
    } = revision_source.accept(candidate_notebook(&ids, "revision source"))
    else {
        panic!("revision source candidate must be accepted");
    };
    let unavailable = DirectEditBatchProposal::<u32> {
        base: unavailable_revision,
        capability_version: CommandBehaviorVersion(1),
        commands: Vec::new(),
    };
    assert_eq!(
        session.direct_edit_batch_graph_size(&unavailable),
        DirectEditBatchGraphSizeOutcome::NoAcceptedRevision,
    );
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let invalid_graph = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[2], span, "base text", "one"),
            text_batch_command(2, &[], span, "base text", "two"),
        ],
    };
    let size = CommandGraphSize {
        commands: 2,
        dependency_edges: 1,
    };
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.direct_edit_batch_graph_size(&invalid_graph),
        DirectEditBatchGraphSizeOutcome::Sized { revision, size },
    );
    assert_eq!(
        session.direct_edit_batch_graph_limits(
            &invalid_graph,
            CommandGraphLimits {
                commands: 2,
                dependency_edges: 1,
            },
        ),
        DirectEditBatchGraphLimitsOutcome::Admitted { revision, size },
    );
    let incompatible = DirectEditBatchProposal {
        capability_version: CommandBehaviorVersion(0),
        ..invalid_graph.clone()
    };
    assert_eq!(
        session.direct_edit_batch_graph_size(&incompatible),
        DirectEditBatchGraphSizeOutcome::CapabilityMismatch {
            current: CommandBehaviorVersion(1),
            expected: CommandBehaviorVersion(0),
        },
    );
    let replacement = candidate_notebook(&ids, "new revision");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.direct_edit_batch_graph_size(&invalid_graph),
        DirectEditBatchGraphSizeOutcome::StaleBase { current },
    );
    assert_ne!(session.current(), Some(&before));
}

#[test]
fn direct_edit_batch_bounded_selection_enforces_exact_report_limit() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], span, "base text", "one"),
            text_batch_command(2, &[1], span, "one", "two"),
            text_batch_command(3, &[2], span, "two", "three"),
        ],
    };
    let selected = BTreeSet::from([3]);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.direct_edit_batch_selection_requirements_bounded(
            &batch, &selected, 2,
        ),
        DirectEditBatchSelectionBoundedOutcome::Requirements {
            missing: vec![
                MissingDependencyRequirement {
                    command: 2,
                    dependency: 1,
                },
                MissingDependencyRequirement {
                    command: 3,
                    dependency: 2,
                },
            ],
            revision,
        },
    );
    assert_eq!(
        session.direct_edit_batch_selection_requirements_bounded(
            &batch, &selected, 1,
        ),
        DirectEditBatchSelectionBoundedOutcome::RequirementCountExceeded {
            actual: 2,
            limit: 1,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn graph_resource_preflight_borrows_command_identities() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let clones = Arc::new(AtomicUsize::new(0));
    let command = |id, dependencies| DirectEditBatchCommand {
        dependencies,
        id: CountingCommandIdentity::new(&clones, id),
        preconditions: CommandTargetPreconditions {
            expected_value: Some(EditableSemanticValue::Text(
                "base text".to_owned(),
            )),
            identity: IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::InlineSpan),
                expected_owner: IdentityOwnerExpectation::Any,
            },
            requested_family: SemanticCommandFamily::TextContent,
        },
        requested: EditableSemanticValue::Text("changed".to_owned()),
        target: span,
    };
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            command(1, Vec::new()),
            command(2, vec![CountingCommandIdentity::new(&clones, 1)]),
        ],
    };
    let size = CommandGraphSize {
        commands: 2,
        dependency_edges: 1,
    };
    assert_eq!(
        session.direct_edit_batch_graph_size(&batch),
        DirectEditBatchGraphSizeOutcome::Sized { revision, size },
    );
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0);
    assert_eq!(
        session.direct_edit_batch_graph_limits(&batch, CommandGraphLimits {
            commands: 1,
            dependency_edges: 1,
        },),
        DirectEditBatchGraphLimitsOutcome::Rejected {
            reason: CommandGraphLimitError::CommandCountExceeded {
                actual: 2,
                limit: 1,
            },
        },
    );
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0);
}

#[test]
fn selection_summary_and_bounded_rejection_borrow_command_identities() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let clones = Arc::new(AtomicUsize::new(0));
    let command = |id, dependencies, expected: &str, requested: &str| {
        DirectEditBatchCommand {
            dependencies,
            id: CountingCommandIdentity::new(&clones, id),
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::Text(
                    expected.to_owned(),
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::InlineSpan),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::TextContent,
            },
            requested: EditableSemanticValue::Text(requested.to_owned()),
            target: span,
        }
    };
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            command(1, Vec::new(), "base text", "one"),
            command(
                2,
                vec![CountingCommandIdentity::new(&clones, 1)],
                "one",
                "two",
            ),
            command(
                3,
                vec![CountingCommandIdentity::new(&clones, 2)],
                "two",
                "three",
            ),
        ],
    };
    let selected = BTreeSet::from([CountingCommandIdentity::new(&clones, 3)]);
    assert_eq!(
        session.direct_edit_batch_selection_summary(&batch, &selected),
        DirectEditBatchSelectionSummaryOutcome::Summarized {
            revision,
            summary: DependencySelectionSummary {
                missing_dependency_edges: 2,
                required_commands: 3,
                selected_commands: 1,
            },
        },
    );
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0,);
    assert_eq!(
        session.direct_edit_batch_selection_requirements_bounded(
            &batch, &selected, 1,
        ),
        DirectEditBatchSelectionBoundedOutcome::RequirementCountExceeded {
            actual: 2,
            limit: 1,
        },
    );
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0,);
}

#[test]
fn invalid_batch_graph_clones_only_reported_command_identities() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let clones = Arc::new(AtomicUsize::new(0));
    let command = |id, dependencies| DirectEditBatchCommand {
        dependencies,
        id: CountingCommandIdentity::new(&clones, id),
        preconditions: CommandTargetPreconditions {
            expected_value: Some(EditableSemanticValue::Text(
                "base text".to_owned(),
            )),
            identity: IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::InlineSpan),
                expected_owner: IdentityOwnerExpectation::Any,
            },
            requested_family: SemanticCommandFamily::TextContent,
        },
        requested: EditableSemanticValue::Text("changed".to_owned()),
        target: span,
    };
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            command(1, vec![CountingCommandIdentity::new(&clones, 2)]),
            command(2, Vec::new()),
            command(3, Vec::new()),
        ],
    };
    assert_eq!(
        session.simulate_direct_edit_batch(batch),
        DirectEditBatchSimulationOutcome::DependencyGraphRejected {
            reason: CommandGraphError::DependencyAfterCommand {
                command: CountingCommandIdentity::new(&clones, 1),
                dependency: CountingCommandIdentity::new(&clones, 2),
            },
        },
    );
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 2,);
}

#[test]
fn bounded_ordered_batch_matches_unbounded_at_exact_graph_limits() {
    let ids = IdentityAllocator::new();
    let (candidate, first, second, _) =
        candidate_notebook_with_three_spans(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("three-span candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], first, "one", "ONE"),
            text_batch_command(2, &[1], second, "two", "TWO"),
        ],
    };
    let before = session.current().expect("accepted revision").clone();
    let expected = session.simulate_direct_edit_batch(batch.clone());
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(batch, CommandGraphLimits {
            commands: 2,
            dependency_edges: 1,
        },),
        expected,
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn bounded_ordered_batch_rejects_resources_before_graph_or_semantics() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[2], span, "wrong base", "one"),
            text_batch_command(2, &[], span, "base text", "two"),
        ],
    };
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(batch, CommandGraphLimits {
            commands: 1,
            dependency_edges: 1,
        },),
        DirectEditBatchSimulationOutcome::ResourceRejected {
            reason: CommandGraphLimitError::CommandCountExceeded {
                actual: 2,
                limit: 1,
            },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn bounded_ordered_batch_counts_repeated_dependency_edges() {
    let ids = IdentityAllocator::new();
    let (candidate, first, second, _) =
        candidate_notebook_with_three_spans(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("three-span candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], first, "one", "ONE"),
            text_batch_command(2, &[1, 1], second, "two", "TWO"),
        ],
    };
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(
            batch.clone(),
            CommandGraphLimits {
                commands: 2,
                dependency_edges: 1,
            },
        ),
        DirectEditBatchSimulationOutcome::ResourceRejected {
            reason: CommandGraphLimitError::DependencyEdgeCountExceeded {
                actual: 2,
                limit: 1,
            },
        },
    );
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(
            batch.clone(),
            CommandGraphLimits {
                commands: 2,
                dependency_edges: 2,
            },
        ),
        session.simulate_direct_edit_batch(batch),
    );
}

#[test]
fn bounded_ordered_batch_preserves_capability_and_stale_precedence() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let incompatible = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(0),
        commands: vec![text_batch_command(
            1,
            &[],
            span,
            "base text",
            "changed",
        )],
    };
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(
            incompatible,
            CommandGraphLimits {
                commands: 0,
                dependency_edges: 0,
            },
        ),
        DirectEditBatchSimulationOutcome::CapabilityMismatch {
            current: CommandBehaviorVersion(1),
            expected: CommandBehaviorVersion(0),
        },
    );
    let stale = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![text_batch_command(
            1,
            &[],
            span,
            "base text",
            "changed",
        )],
    };
    let replacement = candidate_notebook(&ids, "new revision");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(stale, CommandGraphLimits {
            commands: 0,
            dependency_edges: 0,
        },),
        DirectEditBatchSimulationOutcome::StaleBase { current },
    );
}

#[test]
fn bounded_ordered_batch_resource_rejection_borrows_command_ids() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let clones = Arc::new(AtomicUsize::new(0));
    let command = |id, dependencies| DirectEditBatchCommand {
        dependencies,
        id: CountingCommandIdentity::new(&clones, id),
        preconditions: CommandTargetPreconditions {
            expected_value: Some(EditableSemanticValue::Text(
                "base text".to_owned(),
            )),
            identity: IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::InlineSpan),
                expected_owner: IdentityOwnerExpectation::Any,
            },
            requested_family: SemanticCommandFamily::TextContent,
        },
        requested: EditableSemanticValue::Text("changed".to_owned()),
        target: span,
    };
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            command(1, Vec::new()),
            command(2, vec![CountingCommandIdentity::new(&clones, 1)]),
        ],
    };
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(batch, CommandGraphLimits {
            commands: 1,
            dependency_edges: 1,
        },),
        DirectEditBatchSimulationOutcome::ResourceRejected {
            reason: CommandGraphLimitError::CommandCountExceeded {
                actual: 2,
                limit: 1,
            },
        },
    );
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0);
}

#[test]
fn ordered_direct_edit_batch_material_overlay_reaches_nested_text() {
    let ids = IdentityAllocator::new();
    let (candidate, _, span) = candidate_nested_text_notebook(
        &ids,
        CANDIDATE_BLOCK_NESTING_LIMIT.saturating_sub(1),
    );
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("nested candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![text_batch_command(
            1,
            &[],
            span,
            "nested text",
            "indexed nested text",
        )],
    });
    let DirectEditBatchSimulationOutcome::Predicted { changes, effect, .. } =
        outcome
    else {
        panic!("nested indexed target must simulate");
    };
    assert_eq!(changes.len(), 1);
    assert_eq!(effect, DirectEditEffectClass::Mutation);
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_batch_overlay_keeps_unreferenced_profile_impact() {
    let ids = IdentityAllocator::new();
    let mut candidate = candidate_notebook(&ids, "unreferenced profile");
    let notebook = candidate.id;
    let profile = candidate_id(&ids);
    candidate.page_profiles.push(PaperProfile {
        geometry: physical_page_profile(),
        id: profile,
    });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate with unused profile must be accepted");
    };
    let notebook = accepted_for(&mapping, notebook);
    let profile = accepted_for(&mapping, profile);
    let mut changed = physical_page_profile();
    changed.top_clearance = Length::from_micrometres(13_000);
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![DirectEditBatchCommand {
            dependencies: Vec::<u32>::new(),
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::PageProfile(
                    physical_page_profile(),
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::PageProfile),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::DocumentConstraint,
            },
            requested: EditableSemanticValue::PageProfile(changed),
            target: profile,
        }],
    });
    let DirectEditBatchSimulationOutcome::Predicted { impact_seeds, .. } =
        outcome
    else {
        panic!("unused profile edit must simulate");
    };
    assert_eq!(impact_seeds, vec![DirectEditImpactSeed {
        authorities: vec![DirectEditDerivedAuthority::AllDerived],
        scope: DirectEditImpactScope::Notebook { notebook },
    }]);
}

#[test]
fn ordered_direct_edit_batch_material_overlay_preserves_noneditable_rejection()
{
    let ids = IdentityAllocator::new();
    let (candidate, block, _) = candidate_nested_text_notebook(&ids, 1);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let block = accepted_for(&mapping, block);
    let before = session.current().expect("accepted revision").clone();
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![text_batch_command(
            1,
            &[],
            block,
            "nested text",
            "not permitted",
        )],
    });
    let DirectEditBatchSimulationOutcome::Rejected {
        reason,
        evaluated,
        not_evaluated,
        ..
    } = outcome
    else {
        panic!("non-editable block must reject");
    };
    assert!(evaluated.is_empty());
    assert!(not_evaluated.is_empty());
    assert!(matches!(
        *reason,
        DirectEditBatchCommandRejection::Precondition {
            outcome: boxed,
        } if matches!(
            *boxed,
            CommandTargetPreconditionOutcome::FamilyNotExecutable {
                available: None,
                requested: SemanticCommandFamily::TextContent,
                target,
                ..
            } if target == block
        )
    ));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_direct_edit_batch_simulates_independent_changes_read_only() {
    let ids = IdentityAllocator::new();
    let (candidate, first, second, _) =
        candidate_notebook_with_three_spans(&ids);
    let page = candidate.pages[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("three-span candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let flow = accepted_for(&mapping, flow);
    let page = accepted_for(&mapping, page);
    let before = session.current().expect("accepted revision").clone();
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], first, "one", "ONE"),
            text_batch_command(2, &[], second, "two", "TWO"),
        ],
    });
    let DirectEditBatchSimulationOutcome::Predicted {
        changes,
        commands,
        effect,
        impact_seeds,
        revision: predicted_revision,
    } = outcome
    else {
        panic!("independent direct edits must simulate");
    };
    assert_eq!(predicted_revision, revision);
    assert_eq!(changes.len(), 2);
    assert_eq!(commands.len(), 2);
    assert_eq!(effect, DirectEditEffectClass::Mutation);
    assert_eq!(commands[0].command, 1);
    assert_eq!(commands[1].command, 2);
    assert_eq!(impact_seeds, vec![DirectEditImpactSeed {
        authorities: vec![
            DirectEditDerivedAuthority::Diagnostics,
            DirectEditDerivedAuthority::FlowGeometry,
            DirectEditDerivedAuthority::Handwriting,
            DirectEditDerivedAuthority::Motion,
            DirectEditDerivedAuthority::Rendering,
            DirectEditDerivedAuthority::Shaping,
            DirectEditDerivedAuthority::Wrapping,
        ],
        scope: DirectEditImpactScope::Flow { flow, page },
    }]);
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_direct_edit_batch_rejects_atomic_middle_failure_read_only() {
    let ids = IdentityAllocator::new();
    let (candidate, first, second, third) =
        candidate_notebook_with_three_spans(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("three-span candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let third = accepted_for(&mapping, third);
    let before = session.current().expect("accepted revision").clone();
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], first, "one", "ONE"),
            text_batch_command(2, &[], second, "wrong", "TWO"),
            text_batch_command(3, &[], third, "three", "THREE"),
        ],
    });
    let DirectEditBatchSimulationOutcome::Rejected {
        command,
        evaluated,
        not_evaluated,
        reason,
        revision: rejected_revision,
    } = outcome
    else {
        panic!("middle precondition must reject the complete simulation");
    };
    assert_eq!(command, 2);
    assert_eq!(rejected_revision, revision);
    assert_eq!(evaluated.len(), 1);
    assert_eq!(evaluated[0].command, 1);
    assert_eq!(not_evaluated, vec![3]);
    assert!(matches!(
        *reason,
        DirectEditBatchCommandRejection::Precondition { .. }
    ));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_direct_edit_batch_requires_dependency_for_repeated_target() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let rejected =
        session.simulate_direct_edit_batch(DirectEditBatchProposal {
            base: revision,
            capability_version: CommandBehaviorVersion(1),
            commands: vec![
                text_batch_command(1, &[], span, "base", "middle"),
                text_batch_command(2, &[], span, "middle", "final"),
            ],
        });
    assert!(matches!(
        rejected,
        DirectEditBatchSimulationOutcome::Rejected {
            command: 2,
            reason,
            ..
        } if matches!(
            *reason,
            DirectEditBatchCommandRejection::MissingPriorTargetDependency {
                dependency: 1,
                target,
            } if target == span
        )
    ));

    let predicted =
        session.simulate_direct_edit_batch(DirectEditBatchProposal {
            base: revision,
            capability_version: CommandBehaviorVersion(1),
            commands: vec![
                text_batch_command(1, &[], span, "base", "middle"),
                text_batch_command(2, &[1], span, "middle", "final"),
            ],
        });
    let DirectEditBatchSimulationOutcome::Predicted {
        changes, commands, ..
    } = predicted
    else {
        panic!("explicitly dependent repeated target must simulate");
    };
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0], DirectEditBatchCommandPrediction {
        change: Some(DirectEditSemanticChange {
            after: EditableSemanticValue::Text(String::from("middle")),
            before: EditableSemanticValue::Text(String::from("base")),
            family: SemanticCommandFamily::TextContent,
            target: span,
        }),
        command: 1,
        family: SemanticCommandFamily::TextContent,
        target: span,
    });
    assert_eq!(
        commands[1].change,
        Some(DirectEditSemanticChange {
            after: EditableSemanticValue::Text(String::from("final")),
            before: EditableSemanticValue::Text(String::from("middle")),
            family: SemanticCommandFamily::TextContent,
            target: span,
        })
    );
    assert_eq!(changes, vec![DirectEditSemanticChange {
        after: EditableSemanticValue::Text(String::from("final")),
        before: EditableSemanticValue::Text(String::from("base")),
        family: SemanticCommandFamily::TextContent,
        target: span,
    }]);
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_direct_edit_batch_graph_rejects_before_candidate_replay() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.simulate_direct_edit_batch(DirectEditBatchProposal {
            base: revision,
            capability_version: CommandBehaviorVersion(1),
            commands: vec![
                text_batch_command(7, &[], span, "base", "one"),
                text_batch_command(7, &[], span, "base", "two"),
            ],
        }),
        DirectEditBatchSimulationOutcome::DependencyGraphRejected {
            reason: CommandGraphError::DuplicateIdentity { command: 7 },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_direct_edit_batch_noop_does_not_manufacture_dependency() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], span, "base", "base"),
            text_batch_command(2, &[], span, "base", "final"),
        ],
    });
    let DirectEditBatchSimulationOutcome::Predicted {
        changes, commands, ..
    } = outcome
    else {
        panic!("no-op predecessor must not require a dependency");
    };
    assert_eq!(commands.len(), 2);
    assert!(commands[0].change.is_none());
    assert!(commands[1].change.is_some());
    assert_eq!(changes.len(), 1);
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_direct_edit_batch_coalesces_net_noop_across_commands() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![
            text_batch_command(1, &[], span, "base", "temporary"),
            text_batch_command(2, &[1], span, "temporary", "base"),
        ],
    });
    let DirectEditBatchSimulationOutcome::Predicted {
        changes,
        commands,
        effect,
        impact_seeds,
        ..
    } = outcome
    else {
        panic!("dependent revert chain must simulate");
    };
    assert!(changes.is_empty());
    assert_eq!(effect, DirectEditEffectClass::NoOp);
    assert!(impact_seeds.is_empty());
    assert_eq!(commands.len(), 2);
    assert!(commands.iter().all(|command| command.change.is_some()));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_direct_edit_batch_preserves_global_rejection_precedence() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    assert_eq!(
        session.simulate_direct_edit_batch(DirectEditBatchProposal {
            base: revision,
            capability_version: CommandBehaviorVersion(0),
            commands: vec![text_batch_command(1, &[], span, "base", "after")],
        }),
        DirectEditBatchSimulationOutcome::CapabilityMismatch {
            current: CommandBehaviorVersion(1),
            expected: CommandBehaviorVersion(0),
        },
    );
    let replacement = candidate_notebook(&ids, "new revision");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.simulate_direct_edit_batch(DirectEditBatchProposal {
            base: revision,
            capability_version: CommandBehaviorVersion(1),
            commands: vec![text_batch_command(1, &[], span, "base", "after")],
        }),
        DirectEditBatchSimulationOutcome::StaleBase { current },
    );
}

#[test]
fn ordered_direct_edit_batch_seeds_structured_and_profile_impacts() {
    let ids = IdentityAllocator::new();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "x", FormulaMode::Display);
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let page = candidate.pages[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let block = accepted_for(&mapping, block);
    let flow = accepted_for(&mapping, flow);
    let formula = accepted_for(&mapping, formula);
    let page = accepted_for(&mapping, page);
    let formula_batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![DirectEditBatchCommand {
            dependencies: Vec::<u32>::new(),
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::Formula {
                    mode: FormulaMode::Display,
                    source: String::from("x"),
                }),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Formula),
                    expected_owner: IdentityOwnerExpectation::Direct(block),
                },
                requested_family: SemanticCommandFamily::StructuredContent,
            },
            requested: EditableSemanticValue::Formula {
                mode: FormulaMode::Display,
                source: String::from("x^2"),
            },
            target: formula,
        }],
    };
    let DirectEditBatchSimulationOutcome::Predicted { impact_seeds, .. } =
        session.simulate_direct_edit_batch(formula_batch)
    else {
        panic!("formula batch must simulate");
    };
    assert_eq!(impact_seeds, vec![DirectEditImpactSeed {
        authorities: vec![
            DirectEditDerivedAuthority::Layout,
            DirectEditDerivedAuthority::Output,
            DirectEditDerivedAuthority::StructureValidation,
        ],
        scope: DirectEditImpactScope::BlockFlow { block, flow, page },
    }]);

    let mut candidate = candidate_notebook(&ids, "profile impact");
    let profile = candidate.page_profiles[0].id;
    let first_page = candidate.pages[0].id;
    let second_page = candidate_id(&ids);
    candidate.pages.push(Page {
        flows: vec![],
        id: second_page,
        page_profile: profile,
    });
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("shared-profile candidate must be accepted");
    };
    let first_page = accepted_for(&mapping, first_page);
    let profile = accepted_for(&mapping, profile);
    let second_page = accepted_for(&mapping, second_page);
    let mut changed = physical_page_profile();
    changed.top_clearance = Length::from_micrometres(12_000);
    let profile_batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: vec![DirectEditBatchCommand {
            dependencies: Vec::<u32>::new(),
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::PageProfile(
                    physical_page_profile(),
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::PageProfile),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::DocumentConstraint,
            },
            requested: EditableSemanticValue::PageProfile(changed),
            target: profile,
        }],
    };
    let DirectEditBatchSimulationOutcome::Predicted { impact_seeds, .. } =
        session.simulate_direct_edit_batch(profile_batch)
    else {
        panic!("profile batch must simulate");
    };
    assert_eq!(impact_seeds, vec![DirectEditImpactSeed {
        authorities: vec![DirectEditDerivedAuthority::AllDerived],
        scope: DirectEditImpactScope::Pages {
            pages: vec![first_page, second_page],
        },
    }]);
}

#[test]
fn direct_edit_change_preview_reports_exact_change_or_empty_noop() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "before");
    let flow = candidate.pages[0].flows[0].id;
    let page = candidate.pages[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let flow = accepted_for(&mapping, flow);
    let page = accepted_for(&mapping, page);
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.preview_direct_edit_changes(
            revision,
            span,
            EditableSemanticValue::Text(String::from("after")),
        ),
        DirectEditChangePreviewOutcome::Predicted {
            changes: vec![DirectEditSemanticChange {
                after: EditableSemanticValue::Text(String::from("after")),
                before: EditableSemanticValue::Text(String::from("before")),
                family: SemanticCommandFamily::TextContent,
                target: span,
            }],
            effect: DirectEditEffectClass::Mutation,
            impact_seeds: vec![DirectEditImpactSeed {
                authorities: vec![
                    DirectEditDerivedAuthority::Diagnostics,
                    DirectEditDerivedAuthority::FlowGeometry,
                    DirectEditDerivedAuthority::Handwriting,
                    DirectEditDerivedAuthority::Motion,
                    DirectEditDerivedAuthority::Rendering,
                    DirectEditDerivedAuthority::Shaping,
                    DirectEditDerivedAuthority::Wrapping,
                ],
                scope: DirectEditImpactScope::Flow { flow, page },
            }],
            revision,
        },
    );
    assert_eq!(
        session.preview_direct_edit_changes(
            revision,
            span,
            EditableSemanticValue::Text(String::from("before")),
        ),
        DirectEditChangePreviewOutcome::Predicted {
            changes: Vec::new(),
            effect: DirectEditEffectClass::NoOp,
            impact_seeds: Vec::new(),
            revision,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_edit_change_preview_preserves_structured_before_and_after_values() {
    let ids = IdentityAllocator::new();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "x^2", FormulaMode::Display);
    let formula_block = candidate.pages[0].flows[0].blocks[0].id;
    let formula_flow = candidate.pages[0].flows[0].id;
    let formula_page = candidate.pages[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    let formula_block = accepted_for(&mapping, formula_block);
    let formula_flow = accepted_for(&mapping, formula_flow);
    let formula_page = accepted_for(&mapping, formula_page);
    assert_eq!(
        session.preview_direct_edit_changes(
            revision,
            formula,
            EditableSemanticValue::Formula {
                mode: FormulaMode::Display,
                source: String::from(r"\frac{1}{2}"),
            },
        ),
        DirectEditChangePreviewOutcome::Predicted {
            changes: vec![DirectEditSemanticChange {
                after: EditableSemanticValue::Formula {
                    mode: FormulaMode::Display,
                    source: String::from(r"\frac{1}{2}"),
                },
                before: EditableSemanticValue::Formula {
                    mode: FormulaMode::Display,
                    source: String::from("x^2"),
                },
                family: SemanticCommandFamily::StructuredContent,
                target: formula,
            }],
            effect: DirectEditEffectClass::Mutation,
            impact_seeds: vec![DirectEditImpactSeed {
                authorities: vec![
                    DirectEditDerivedAuthority::Layout,
                    DirectEditDerivedAuthority::Output,
                    DirectEditDerivedAuthority::StructureValidation,
                ],
                scope: DirectEditImpactScope::BlockFlow {
                    block: formula_block,
                    flow: formula_flow,
                    page: formula_page,
                },
            }],
            revision,
        },
    );

    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let table_block = candidate.pages[0].flows[0].blocks[0].id;
    let table_flow = candidate.pages[0].flows[0].id;
    let table_page = candidate.pages[0].id;
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let row = accepted_for(&mapping, row);
    let table_block = accepted_for(&mapping, table_block);
    let table_flow = accepted_for(&mapping, table_flow);
    let table_page = accepted_for(&mapping, table_page);
    assert_eq!(
        session.preview_direct_edit_changes(
            revision,
            row,
            EditableSemanticValue::TableRowRole(TableRowRole::Body),
        ),
        DirectEditChangePreviewOutcome::Predicted {
            changes: vec![DirectEditSemanticChange {
                after: EditableSemanticValue::TableRowRole(TableRowRole::Body),
                before: EditableSemanticValue::TableRowRole(
                    TableRowRole::Header,
                ),
                family: SemanticCommandFamily::StructuredContent,
                target: row,
            }],
            effect: DirectEditEffectClass::Mutation,
            impact_seeds: vec![DirectEditImpactSeed {
                authorities: vec![
                    DirectEditDerivedAuthority::Layout,
                    DirectEditDerivedAuthority::Output,
                    DirectEditDerivedAuthority::StructureValidation,
                ],
                scope: DirectEditImpactScope::BlockFlow {
                    block: table_block,
                    flow: table_flow,
                    page: table_page,
                },
            }],
            revision,
        },
    );

    let candidate = candidate_notebook(&ids, "profile change preview");
    let profile = candidate.page_profiles[0].id;
    let profile_page = candidate.pages[0].id;
    let mut changed_profile = physical_page_profile();
    changed_profile.top_clearance = Length::from_micrometres(22_000);
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("profile candidate must be accepted");
    };
    let profile = accepted_for(&mapping, profile);
    let profile_page = accepted_for(&mapping, profile_page);
    assert_eq!(
        session.preview_direct_edit_changes(
            revision,
            profile,
            EditableSemanticValue::PageProfile(changed_profile),
        ),
        DirectEditChangePreviewOutcome::Predicted {
            changes: vec![DirectEditSemanticChange {
                after: EditableSemanticValue::PageProfile(changed_profile),
                before: EditableSemanticValue::PageProfile(
                    physical_page_profile(),
                ),
                family: SemanticCommandFamily::DocumentConstraint,
                target: profile,
            }],
            effect: DirectEditEffectClass::Mutation,
            impact_seeds: vec![DirectEditImpactSeed {
                authorities: vec![DirectEditDerivedAuthority::AllDerived],
                scope: DirectEditImpactScope::Pages {
                    pages: vec![profile_page],
                },
            }],
            revision,
        },
    );
}

#[test]
fn direct_edit_change_preview_preserves_typed_simulation_rejection() {
    let ids = IdentityAllocator::new();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "x", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    let before = session.current().expect("accepted revision").clone();
    let preview = session.preview_direct_edit_changes(
        revision,
        formula,
        EditableSemanticValue::Formula {
            mode: FormulaMode::Display,
            source: String::from(r"\frac{1}"),
        },
    );
    let DirectEditChangePreviewOutcome::Rejected { outcome } = preview else {
        panic!("malformed mathematics must reject change preview");
    };
    assert!(matches!(
        *outcome,
        DirectEditSimulationOutcome::InvalidMathematics {
            revision: rejected_revision,
            target,
            ..
        } if rejected_revision == revision && target == formula
    ));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_edit_simulation_predicts_text_and_table_role_without_mutation() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "before");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            span,
            EditableSemanticValue::Text(String::from("before")),
        ),
        DirectEditSimulationOutcome::NoOp {
            family: SemanticCommandFamily::TextContent,
            revision,
            target: span,
        },
    );
    let requested = EditableSemanticValue::Text(String::from("after"));
    assert_eq!(
        session.simulate_direct_edit(revision, span, requested.clone()),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::TextContent,
            requested,
            revision,
            target: span,
        },
    );
    assert_eq!(session.current(), Some(&before));
    assert!(matches!(
        session.replace_text(revision, span, String::from("after")),
        TextEditOutcome::Applied { base, target, .. }
            if base == revision && target == span
    ));

    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let row = accepted_for(&mapping, row);
    let before = session.current().expect("table revision").clone();
    let requested = EditableSemanticValue::TableRowRole(TableRowRole::Body);
    assert_eq!(
        session.simulate_direct_edit(revision, row, requested.clone()),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::StructuredContent,
            requested,
            revision,
            target: row,
        },
    );
    assert_eq!(session.current(), Some(&before));
    assert!(matches!(
        session.replace_table_row_role(revision, row, TableRowRole::Body),
        TableRowRoleEditOutcome::Applied { base, target, .. }
            if base == revision && target == row
    ));
}

#[test]
fn direct_edit_simulation_matches_formula_domain_rejections() {
    let ids = IdentityAllocator::new();
    let (candidate, formula) =
        candidate_math_notebook(&ids, "x^2", FormulaMode::Display);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("formula candidate must be accepted");
    };
    let formula = accepted_for(&mapping, formula);
    let before = session.current().expect("formula revision").clone();
    let malformed = EditableSemanticValue::Formula {
        mode: FormulaMode::Display,
        source: String::from(r"\frac{1}"),
    };
    let DirectEditSimulationOutcome::InvalidMathematics { reason, .. } =
        session.simulate_direct_edit(revision, formula, malformed)
    else {
        panic!("malformed formula must be rejected by simulation");
    };
    assert_eq!(session.current(), Some(&before));
    assert_eq!(
        session.replace_formula(
            revision,
            formula,
            FormulaMode::Display,
            String::from(r"\frac{1}"),
        ),
        FormulaEditOutcome::InvalidMathematics {
            reason,
            revision,
            target: formula,
        },
    );

    let unsupported = EditableSemanticValue::Formula {
        mode: FormulaMode::Display,
        source: String::from(r"x + \mystery{y}"),
    };
    assert_eq!(
        session.simulate_direct_edit(revision, formula, unsupported),
        DirectEditSimulationOutcome::UnsupportedMathematics {
            revision,
            target: formula,
        },
    );
    assert_eq!(
        session.replace_formula(
            revision,
            formula,
            FormulaMode::Display,
            String::from(r"x + \mystery{y}"),
        ),
        FormulaEditOutcome::UnsupportedMathematics {
            revision,
            target: formula,
        },
    );
}

#[test]
fn direct_edit_simulation_matches_page_profile_validation_and_noop() {
    let ids = IdentityAllocator::new();
    let candidate = candidate_notebook(&ids, "profile simulation");
    let profile = candidate.page_profiles[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("profile candidate must be accepted");
    };
    let profile = accepted_for(&mapping, profile);
    let before = session.current().expect("profile revision").clone();
    let same = EditableSemanticValue::PageProfile(physical_page_profile());
    assert_eq!(
        session.simulate_direct_edit(revision, profile, same),
        DirectEditSimulationOutcome::NoOp {
            family: SemanticCommandFamily::DocumentConstraint,
            revision,
            target: profile,
        },
    );
    let mut invalid = physical_page_profile();
    invalid.sheet.width = Length::ZERO;
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            profile,
            EditableSemanticValue::PageProfile(invalid),
        ),
        DirectEditSimulationOutcome::InvalidPageProfile {
            reason: PageProfileError::SheetDimensionIsZero,
            revision,
            target: profile,
        },
    );
    assert_eq!(session.current(), Some(&before));
    assert_eq!(
        session.replace_page_profile(revision, profile, invalid),
        PageProfileEditOutcome::InvalidProfile {
            reason: PageProfileError::SheetDimensionIsZero,
            revision,
            target: profile,
        },
    );
}

#[test]
fn direct_edit_simulation_stale_missing_noneditable_and_family_mismatch() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let notebook = candidate.id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let notebook = accepted_for(&mapping, notebook);
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            span,
            EditableSemanticValue::TableRowRole(TableRowRole::Body),
        ),
        DirectEditSimulationOutcome::ValueFamilyMismatch {
            actual: EditableSemanticValueKind::Text,
            requested: EditableSemanticValueKind::TableRowRole,
            revision,
            target: span,
        },
    );
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            notebook,
            EditableSemanticValue::Text(String::from("x")),
        ),
        DirectEditSimulationOutcome::TargetNotEditableValue {
            kind: SemanticIdentityKind::Notebook,
            revision,
            target: notebook,
        },
    );
    let replacement = candidate_notebook(&ids, "new revision");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            span,
            EditableSemanticValue::Text(String::from("after")),
        ),
        DirectEditSimulationOutcome::StaleBase { current },
    );
    assert_eq!(
        session.simulate_direct_edit(
            current,
            span,
            EditableSemanticValue::Text(String::from("after")),
        ),
        DirectEditSimulationOutcome::TargetNotFound {
            revision: current,
            target: span,
        },
    );
    let empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.simulate_direct_edit(
            current,
            span,
            EditableSemanticValue::Text(String::from("after")),
        ),
        DirectEditSimulationOutcome::NoAcceptedRevision,
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
