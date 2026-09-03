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
//   - Regression evidence for accepted semantic authority and history.
// - Must-Not:
//   - Parse model output, persist revisions, or define layout/render behavior.
// - Allows:
//   - Inputs: Candidate notebooks, direct edits, and history traversals.
//   - Outputs: Assertions over acceptance, edits, history, and stable IDs.
//   - Side effects: Process-local test allocation and accepted-state mutation.
// - Split-When:
//   - History retry or command Apply needs independent transaction fixtures.
// - Merge-When:
//   - A broader semantic transaction suite subsumes these application fixtures.
// - Summary:
//   - Verifies accepted semantic changes and history remain transactional.
// - Description:
//   - Covers acceptance, direct edits, history traversal, and graph rejection.
// - Usage:
//   - Compile directly against semantic notebook application components.
// - Defaults:
//   - Rejected candidates leave the previously accepted revision unchanged.
//
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::num::NonZeroU32;
use std::sync::{Arc, Barrier, Mutex};
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
    TableCellSpan, TableGridError, TableRow, TableRowRole, UnresolvedBlock,
    UnresolvedReason,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CANDIDATE_BLOCK_NESTING_LIMIT, CandidateGraphError,
    CandidateReferenceKind, CommandBehaviorVersion,
    CommandCapabilityCompatibilityOutcome, CommandFamilyAdmissionOutcome,
    CommandFamilyCapability, CommandResourceLimits, CommandTargetMaterial,
    CommandTargetMaterialOutcome, CommandTargetPreconditionOutcome,
    CommandTargetPreconditions, DirectEditBatchApplyOutcome,
    DirectEditBatchCommand, DirectEditBatchCommandPrediction,
    DirectEditBatchCommandRejection,
    DirectEditBatchGraphLimitsOutcome, DirectEditBatchGraphSizeOutcome,
    DirectEditBatchProposal, DirectEditBatchSelectionBoundedOutcome,
    DirectEditBatchSelectionRequirementsOutcome,
    DirectEditBatchSelectionSummaryOutcome, DirectEditBatchSimulationOutcome,
    DirectEditChangePreviewOutcome, DirectEditDerivedAuthority,
    DirectEditEffectClass, DirectEditImpactScope, DirectEditImpactSeed,
    DirectEditProposal, DirectEditProposalOutcome, DirectEditSemanticChange,
    DirectEditSimulationOutcome, EditableSemanticValue,
    EditableSemanticValueKind, EditableValuePreconditionOutcome,
    FormulaEditOutcome, HistoryAvailability, HistoryAvailabilityOutcome,
    HistoryDirection, HistoryTraversalOutcome, IdentityAncestryCompleteness,
    IdentityAncestryEntry, IdentityAncestryInspectOutcome,
    IdentityInspectOutcome, IdentityKindInspectOutcome,
    IdentityOwnerExpectation, IdentityPrecondition,
    IdentityPreconditionOutcome, PageProfileEditOutcome, SemanticCommandFamily,
    SemanticNotebookHistory, SemanticNotebookSession,
    TableCellSpanEditOutcome, TableRowRoleEditOutcome, TextEditOutcome,
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

struct NonOrdCommandIdentity;

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

#[derive(Clone, Copy)]
struct FigureAssetCandidateIds {
    asset_one: CandidateIdentity,
    asset_two: CandidateIdentity,
    block: CandidateIdentity,
    figure: CandidateIdentity,
    flow: CandidateIdentity,
    page: CandidateIdentity,
}

fn candidate_notebook_with_figure_assets(
    identities: &IdentityAllocator,
) -> (Notebook<CandidateIdentity>, FigureAssetCandidateIds) {
    let notebook = candidate_id(identities);
    let profile = candidate_id(identities);
    let page = candidate_id(identities);
    let flow = candidate_id(identities);
    let block = candidate_id(identities);
    let figure = candidate_id(identities);
    let asset_one = candidate_id(identities);
    let asset_two = candidate_id(identities);
    let candidate = Notebook {
        assets: vec![
            Asset {
                id: asset_one,
                media_type: String::from("image/png"),
            },
            Asset {
                id: asset_two,
                media_type: String::from("image/webp"),
            },
        ],
        constraints: vec![],
        extensions: vec![],
        id: notebook,
        output_profiles: vec![],
        page_profiles: vec![PaperProfile {
            geometry: physical_page_profile(),
            id: profile,
        }],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![Block {
                    content: BlockContent::Figure(Figure {
                        asset: Some(asset_one),
                        caption: vec![],
                        id: figure,
                    }),
                    extensions: vec![],
                    id: block,
                    provenance: None,
                    style: None,
                }],
                id: flow,
            }],
            id: page,
            page_profile: profile,
        }],
        provenance: vec![],
        styles: vec![],
    };
    (candidate, FigureAssetCandidateIds {
        asset_one,
        asset_two,
        block,
        figure,
        flow,
        page,
    })
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

fn span_batch_command(
    id: u32,
    dependencies: &[u32],
    target: AcceptedIdentity,
    expected: TableCellSpan,
    requested: TableCellSpan,
) -> DirectEditBatchCommand<u32> {
    DirectEditBatchCommand {
        dependencies: dependencies.to_vec(),
        id,
        preconditions: CommandTargetPreconditions {
            expected_value: Some(EditableSemanticValue::TableCellSpan(
                expected,
            )),
            identity: IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::TableCell),
                expected_owner: IdentityOwnerExpectation::Any,
            },
            requested_family: SemanticCommandFamily::StructuredContent,
        },
        requested: EditableSemanticValue::TableCellSpan(requested),
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

fn table_cell_span(columns: u32, rows: u32) -> TableCellSpan {
    let Some(columns) = NonZeroU32::new(columns) else {
        panic!("fixture column span must be nonzero");
    };
    let Some(rows) = NonZeroU32::new(rows) else {
        panic!("fixture row span must be nonzero");
    };
    TableCellSpan { columns, rows }
}

fn empty_table_cell(
    identities: &IdentityAllocator,
    span: TableCellSpan,
) -> TableCell<CandidateIdentity> {
    TableCell {
        blocks: vec![],
        id: candidate_id(identities),
        span,
    }
}

fn table_row(
    identities: &IdentityAllocator,
    cells: Vec<TableCell<CandidateIdentity>>,
) -> TableRow<CandidateIdentity> {
    TableRow {
        cells,
        id: candidate_id(identities),
        role: TableRowRole::Body,
    }
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
                span: TableCellSpan::SINGLE,
            }],
            id: row_id,
            role,
        }],
    });
    (notebook, row_id, table_id)
}

#[test]
fn candidate_merged_table_spans_promote_without_rewriting() {
    let ids = IdentityAllocator::new();
    let (mut candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let BlockContent::Table(table) =
        &mut candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let first_span = table_cell_span(2, 2);
    table.rows[0].cells[0].span = first_span;
    table.rows[0]
        .cells
        .push(empty_table_cell(&ids, TableCellSpan::SINGLE));
    table.rows.push(table_row(
        &ids,
        vec![empty_table_cell(&ids, TableCellSpan::SINGLE)],
    ));
    let last_span = table_cell_span(2, 1);
    table.rows.push(table_row(
        &ids,
        vec![
            empty_table_cell(&ids, TableCellSpan::SINGLE),
            empty_table_cell(&ids, last_span),
        ],
    ));

    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { .. } = session.accept(candidate) else {
        panic!("rectangular merged table must be accepted");
    };
    let current = session.current().expect("accepted merged table");
    let BlockContent::Table(table) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("accepted block must remain a table");
    };
    assert_eq!(table.rows[0].cells[0].span, first_span);
    assert_eq!(table.rows[0].cells[1].span, TableCellSpan::SINGLE);
    assert_eq!(table.rows[1].cells.len(), 1);
    assert_eq!(table.rows[2].cells[1].span, last_span);
}

#[test]
fn candidate_table_row_span_cannot_extend_beyond_table() {
    let ids = IdentityAllocator::new();
    let baseline = candidate_notebook(&ids, "accepted baseline");
    let (mut invalid, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(table) =
        &mut invalid.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let cell = table.rows[0].cells[0].id;
    table.rows[0].cells[0].span = table_cell_span(1, 2);
    let mut session = SemanticNotebookSessionService::default();
    assert!(matches!(
        session.accept(baseline),
        AcceptanceOutcome::Accepted { .. }
    ));
    let before = session.current().expect("baseline revision").clone();

    assert_eq!(
        session.accept(invalid),
        AcceptanceOutcome::InvalidCandidate {
            reason: CandidateGraphError::InvalidTableRowSpan {
                candidate: cell,
            },
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn candidate_table_cell_cannot_cross_inherited_row_span() {
    let ids = IdentityAllocator::new();
    let (mut candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(table) =
        &mut candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    table.rows[0].cells[0].span = table_cell_span(1, 2);
    table.rows[0]
        .cells
        .push(empty_table_cell(&ids, TableCellSpan::SINGLE));
    let crossing = empty_table_cell(&ids, table_cell_span(2, 1));
    let crossing_id = crossing.id;
    table.rows.push(table_row(&ids, vec![crossing]));
    let mut session = SemanticNotebookSessionService::default();

    assert_eq!(
        session.accept(candidate),
        AcceptanceOutcome::InvalidCandidate {
            reason: CandidateGraphError::InvalidTableColumnSpan {
                candidate: crossing_id,
            },
        },
    );
    assert!(session.current().is_none());
}

#[test]
fn candidate_table_rows_must_cover_the_established_width() {
    let ids = IdentityAllocator::new();
    let (mut candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(table) =
        &mut candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    table.rows[0]
        .cells
        .push(empty_table_cell(&ids, TableCellSpan::SINGLE));
    let incomplete = table_row(
        &ids,
        vec![empty_table_cell(&ids, TableCellSpan::SINGLE)],
    );
    let incomplete_id = incomplete.id;
    table.rows.push(incomplete);
    let mut session = SemanticNotebookSessionService::default();

    assert_eq!(
        session.accept(candidate),
        AcceptanceOutcome::InvalidCandidate {
            reason: CandidateGraphError::InvalidTableRowWidth {
                candidate: incomplete_id,
            },
        },
    );
    assert!(session.current().is_none());
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
fn bounded_identity_ancestry_reports_explicit_complete_and_incomplete_chains() {
    let ids = IdentityAllocator::new();
    let (candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let notebook = candidate.id;
    let page = candidate.pages[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let BlockContent::Table(table) =
        &candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain table");
    };
    let table_id = table.id;
    let row = table.rows[0].id;
    let cell = table.rows[0].cells[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let notebook = accepted_for(&mapping, notebook);
    let page = accepted_for(&mapping, page);
    let flow = accepted_for(&mapping, flow);
    let block = accepted_for(&mapping, block);
    let table = accepted_for(&mapping, table_id);
    let row = accepted_for(&mapping, row);
    let cell = accepted_for(&mapping, cell);
    let before = session.current().expect("accepted revision").clone();

    assert_eq!(
        session.inspect_identity_ancestry_bounded(revision, cell, 0),
        IdentityAncestryInspectOutcome::Inspected {
            completeness: IdentityAncestryCompleteness::Incomplete {
                remaining_identity: cell,
            },
            entries: Vec::new(),
            revision,
            target: cell,
        },
    );
    assert_eq!(
        session.inspect_identity_ancestry_bounded(revision, cell, 3),
        IdentityAncestryInspectOutcome::Inspected {
            completeness: IdentityAncestryCompleteness::Incomplete {
                remaining_identity: block,
            },
            entries: vec![
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::TableCell,
                        owner: Some(row),
                    },
                    identity: cell,
                },
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::TableRow,
                        owner: Some(table),
                    },
                    identity: row,
                },
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::Table,
                        owner: Some(block),
                    },
                    identity: table,
                },
            ],
            revision,
            target: cell,
        },
    );
    assert_eq!(
        session.inspect_identity_ancestry_bounded(revision, cell, 7),
        IdentityAncestryInspectOutcome::Inspected {
            completeness: IdentityAncestryCompleteness::Complete,
            entries: vec![
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::TableCell,
                        owner: Some(row),
                    },
                    identity: cell,
                },
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::TableRow,
                        owner: Some(table),
                    },
                    identity: row,
                },
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::Table,
                        owner: Some(block),
                    },
                    identity: table,
                },
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::Block(
                            SemanticBlockKind::Table,
                        ),
                        owner: Some(flow),
                    },
                    identity: block,
                },
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::Flow,
                        owner: Some(page),
                    },
                    identity: flow,
                },
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::Page,
                        owner: Some(notebook),
                    },
                    identity: page,
                },
                IdentityAncestryEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::Notebook,
                        owner: None,
                    },
                    identity: notebook,
                },
            ],
            revision,
            target: cell,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn bounded_identity_ancestry_reaches_the_maximum_accepted_nesting_depth() {
    let ids = IdentityAllocator::new();
    let (candidate, _, span) = candidate_nested_text_notebook(
        &ids,
        CANDIDATE_BLOCK_NESTING_LIMIT.saturating_sub(1),
    );
    let notebook = candidate.id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate at the nesting bound must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let notebook = accepted_for(&mapping, notebook);

    let IdentityAncestryInspectOutcome::Inspected {
        completeness,
        entries,
        ..
    } = session.inspect_identity_ancestry_bounded(revision, span, 4)
    else {
        panic!("bounded deep ancestry must inspect");
    };
    assert!(matches!(
        completeness,
        IdentityAncestryCompleteness::Incomplete { .. }
    ));
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].identity, span);

    let expected_entries = CANDIDATE_BLOCK_NESTING_LIMIT + 4;
    let IdentityAncestryInspectOutcome::Inspected {
        completeness,
        entries,
        ..
    } = session.inspect_identity_ancestry_bounded(
        revision,
        span,
        expected_entries,
    ) else {
        panic!("complete deep ancestry must inspect");
    };
    assert_eq!(completeness, IdentityAncestryCompleteness::Complete);
    assert_eq!(entries.len(), expected_entries);
    assert_eq!(entries.first().map(|entry| entry.identity), Some(span));
    assert_eq!(entries.last().map(|entry| entry.identity), Some(notebook));
    assert_eq!(
        entries.last().map(|entry| entry.descriptor.kind),
        Some(SemanticIdentityKind::Notebook),
    );
}

#[test]
fn bounded_identity_ancestry_does_not_preallocate_the_caller_bound() {
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

    let IdentityAncestryInspectOutcome::Inspected {
        completeness,
        entries,
        ..
    } = session.inspect_identity_ancestry_bounded(revision, row, usize::MAX)
    else {
        panic!("huge caller bound must inspect the finite owner chain");
    };
    assert_eq!(completeness, IdentityAncestryCompleteness::Complete);
    assert_eq!(entries.len(), 6);
    assert_eq!(entries.first().map(|entry| entry.identity), Some(row));
    assert_eq!(
        entries.last().map(|entry| entry.descriptor.kind),
        Some(SemanticIdentityKind::Notebook),
    );
}

#[test]
fn bounded_identity_ancestry_is_read_only_at_an_undo_position() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "zero");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let TextEditOutcome::Applied { revision: first, .. } =
        session.replace_text(base, target, String::from("one"))
    else {
        panic!("first edit must apply");
    };
    let TextEditOutcome::Applied { revision: second, .. } =
        session.replace_text(first, target, String::from("two"))
    else {
        panic!("second edit must apply");
    };
    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        session.traverse_history(second, HistoryDirection::Undo)
    else {
        panic!("second edit must Undo");
    };
    let expected_history = session.history_availability();
    let before = session.current().expect("undone revision").clone();

    assert!(matches!(
        session.inspect_identity_ancestry_bounded(undone, target, 2),
        IdentityAncestryInspectOutcome::Inspected { .. }
    ));
    assert_eq!(session.current(), Some(&before));
    assert_eq!(session.history_availability(), expected_history);
}

#[test]
fn bounded_identity_ancestry_preserves_global_inspection_precedence() {
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
        session.inspect_identity_ancestry_bounded(revision, row, 0),
        IdentityAncestryInspectOutcome::StaleBase { current },
    );
    assert_eq!(
        session.inspect_identity_ancestry_bounded(current, row, 0),
        IdentityAncestryInspectOutcome::TargetNotFound {
            revision: current,
            target: row,
        },
    );
    let empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.inspect_identity_ancestry_bounded(current, row, 0),
        IdentityAncestryInspectOutcome::NoAcceptedRevision,
    );
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
fn asset_reference_review_accepts_only_current_asset_identities() {
    let ids = IdentityAllocator::new();
    let (candidate, candidate_ids) =
        candidate_notebook_with_figure_assets(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("figure candidate must be accepted");
    };
    let figure = accepted_for(&mapping, candidate_ids.figure);
    let asset_one = accepted_for(&mapping, candidate_ids.asset_one);
    let asset_two = accepted_for(&mapping, candidate_ids.asset_two);
    let block = accepted_for(&mapping, candidate_ids.block);
    let flow = accepted_for(&mapping, candidate_ids.flow);
    let page = accepted_for(&mapping, candidate_ids.page);

    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(revision, figure)
    else {
        panic!("figure command material must be prepared");
    };
    assert_eq!(material.descriptor, SemanticIdentityDescriptor {
        kind: SemanticIdentityKind::Figure,
        owner: Some(block),
    });
    assert_eq!(material.direct_edit_family, Some(
        SemanticCommandFamily::AssetReference,
    ));
    assert_eq!(material.editable_value, Some(
        EditableSemanticValue::AssetReference(Some(asset_one)),
    ));
    assert!(matches!(
        session.check_command_family_admission(
            revision,
            figure,
            SemanticCommandFamily::AssetReference,
        ),
        CommandFamilyAdmissionOutcome::Admitted { .. }
    ));
    assert_eq!(
        session.check_command_family_admission(
            revision,
            figure,
            SemanticCommandFamily::StructuredContent,
        ),
        CommandFamilyAdmissionOutcome::FamilyNotExecutable {
            available: Some(SemanticCommandFamily::AssetReference),
            requested: SemanticCommandFamily::StructuredContent,
            revision,
            target: figure,
        },
    );
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            figure,
            EditableSemanticValue::Text(String::from("not an asset")),
        ),
        DirectEditSimulationOutcome::ValueFamilyMismatch {
            actual: EditableSemanticValueKind::AssetReference,
            requested: EditableSemanticValueKind::Text,
            revision,
            target: figure,
        },
    );
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            figure,
            EditableSemanticValue::AssetReference(Some(asset_one)),
        ),
        DirectEditSimulationOutcome::NoOp {
            family: SemanticCommandFamily::AssetReference,
            revision,
            target: figure,
        },
    );
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            figure,
            EditableSemanticValue::AssetReference(None),
        ),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::AssetReference,
            requested: EditableSemanticValue::AssetReference(None),
            revision,
            target: figure,
        },
    );
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            figure,
            EditableSemanticValue::AssetReference(Some(flow)),
        ),
        DirectEditSimulationOutcome::InvalidAssetReference {
            actual: Some(SemanticIdentityKind::Flow),
            reference: flow,
            revision,
            target: figure,
        },
    );
    assert_eq!(
        session.preview_direct_edit_changes(
            revision,
            figure,
            EditableSemanticValue::AssetReference(Some(flow)),
        ),
        DirectEditChangePreviewOutcome::Rejected {
            outcome: Box::new(
                DirectEditSimulationOutcome::InvalidAssetReference {
                    actual: Some(SemanticIdentityKind::Flow),
                    reference: flow,
                    revision,
                    target: figure,
                },
            ),
        },
    );
    assert_eq!(
        session.preview_direct_edit_changes(
            revision,
            figure,
            EditableSemanticValue::AssetReference(Some(asset_two)),
        ),
        DirectEditChangePreviewOutcome::Predicted {
            changes: vec![DirectEditSemanticChange {
                after: EditableSemanticValue::AssetReference(Some(asset_two)),
                before: EditableSemanticValue::AssetReference(Some(asset_one)),
                family: SemanticCommandFamily::AssetReference,
                target: figure,
            }],
            effect: DirectEditEffectClass::Mutation,
            impact_seeds: vec![DirectEditImpactSeed {
                authorities: vec![DirectEditDerivedAuthority::AllDerived],
                scope: DirectEditImpactScope::BlockFlow { block, flow, page },
            }],
            revision,
        },
    );
}

#[test]
fn asset_reference_batch_applies_atomically_and_undo_restores_reference() {
    let ids = IdentityAllocator::new();
    let (candidate, candidate_ids) =
        candidate_notebook_with_figure_assets(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("figure candidate must be accepted");
    };
    let figure = accepted_for(&mapping, candidate_ids.figure);
    let asset_one = accepted_for(&mapping, candidate_ids.asset_one);
    let asset_two = accepted_for(&mapping, candidate_ids.asset_two);
    let block = accepted_for(&mapping, candidate_ids.block);
    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::AssetReference(
                    Some(asset_one),
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Figure),
                    expected_owner: IdentityOwnerExpectation::Direct(block),
                },
                requested_family: SemanticCommandFamily::AssetReference,
            },
            requested: EditableSemanticValue::AssetReference(Some(asset_two)),
            target: figure,
        }],
    };
    let simulation = session.simulate_direct_edit_batch(batch.clone());
    let DirectEditBatchSimulationOutcome::Predicted {
        changes,
        impact_seeds,
        ..
    } = simulation
    else {
        panic!("admitted asset-reference batch must simulate");
    };
    assert_eq!(changes.len(), 1);
    assert_eq!(impact_seeds.len(), 1);
    assert_eq!(impact_seeds[0].scope, DirectEditImpactScope::BlockFlow {
        block,
        flow: accepted_for(&mapping, candidate_ids.flow),
        page: accepted_for(&mapping, candidate_ids.page),
    });
    assert_eq!(
        impact_seeds[0].authorities,
        vec![DirectEditDerivedAuthority::AllDerived],
    );

    let DirectEditBatchApplyOutcome::Applied { revision, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("admitted asset-reference batch must apply");
    };
    let current = session.current().expect("asset-reference revision");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must remain a figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(asset_two));
    assert_eq!(current.notebook.assets.len(), 2);

    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        session.traverse_history(revision, HistoryDirection::Undo)
    else {
        panic!("asset-reference batch must Undo");
    };
    let current = session.current().expect("asset-reference Undo revision");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo fixture must remain a figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(asset_one));
    assert_ne!(undone, revision);
}

#[test]
fn asset_reference_batch_attaches_to_figure_without_reference() {
    let ids = IdentityAllocator::new();
    let (mut candidate, candidate_ids) =
        candidate_notebook_with_figure_assets(&ids);
    let BlockContent::Figure(candidate_figure) =
        &mut candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a figure");
    };
    candidate_figure.asset = None;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("empty-reference figure candidate must be accepted");
    };
    let figure = accepted_for(&mapping, candidate_ids.figure);
    let first_asset = accepted_for(&mapping, candidate_ids.asset_one);
    assert_eq!(
        session.check_editable_value_precondition(
            base,
            figure,
            EditableSemanticValue::AssetReference(None),
        ),
        EditableValuePreconditionOutcome::Satisfied {
            actual: EditableSemanticValue::AssetReference(None),
            revision: base,
            target: figure,
        },
    );
    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::AssetReference(
                    None,
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Figure),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::AssetReference,
            },
            requested: EditableSemanticValue::AssetReference(Some(first_asset)),
            target: figure,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision: applied, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("empty figure reference must admit an existing asset");
    };
    let current = session.current().expect("attached asset revision");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must remain figure");
    };
    assert_eq!(current_figure.asset, Some(first_asset));

    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("asset attachment must Undo");
    };
    let current = session.current().expect("asset attachment Undo");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo fixture must remain figure");
    };
    assert_eq!(current_figure.asset, None);
}

#[test]
fn invalid_asset_reference_in_mixed_batch_is_atomic() {
    let ids = IdentityAllocator::new();
    let (mut candidate, candidate_ids) =
        candidate_notebook_with_figure_assets(&ids);
    let text_block = candidate_id(&ids);
    let text_span = candidate_id(&ids);
    candidate.pages[0].flows[0].blocks.insert(0, Block {
        content: BlockContent::Paragraph(vec![InlineSpan {
            id: text_span,
            provenance: None,
            style: None,
            text: String::from("before"),
        }]),
        extensions: vec![],
        id: text_block,
        provenance: None,
        style: None,
    });
    let candidate_flow = candidate.pages[0].flows[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("mixed asset candidate must be accepted");
    };
    let span = accepted_for(&mapping, text_span);
    let figure = accepted_for(&mapping, candidate_ids.figure);
    let first_asset = accepted_for(&mapping, candidate_ids.asset_one);
    let flow = accepted_for(&mapping, candidate_flow);
    let before = session.current().expect("mixed asset base").clone();
    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], span, "before", "after"),
            DirectEditBatchCommand {
                dependencies: vec![],
                id: 2_u32,
                preconditions: CommandTargetPreconditions {
                    expected_value: Some(EditableSemanticValue::AssetReference(
                        Some(first_asset),
                    )),
                    identity: IdentityPrecondition {
                        expected_kind: Some(SemanticIdentityKind::Figure),
                        expected_owner: IdentityOwnerExpectation::Any,
                    },
                    requested_family: SemanticCommandFamily::AssetReference,
                },
                requested: EditableSemanticValue::AssetReference(Some(flow)),
                target: figure,
            },
        ],
    };
    let DirectEditBatchSimulationOutcome::Rejected { reason, .. } =
        session.simulate_direct_edit_batch(batch.clone())
    else {
        panic!("wrong-kind asset reference must reject mixed simulation");
    };
    assert!(matches!(
        *reason,
        DirectEditBatchCommandRejection::Simulation { outcome }
            if *outcome == DirectEditSimulationOutcome::InvalidAssetReference {
                actual: Some(SemanticIdentityKind::Flow),
                reference: flow,
                revision: base,
                target: figure,
            }
    ));
    assert!(matches!(
        session.apply_direct_edit_batch(batch),
        DirectEditBatchApplyOutcome::Rejected { .. }
    ));
    assert_eq!(session.current(), Some(&before));
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: false,
            revision: base,
        }),
    );
}

#[test]
fn asset_reference_batch_reaches_deeply_nested_figure() {
    let ids = IdentityAllocator::new();
    let (mut candidate, candidate_ids) =
        candidate_notebook_with_figure_assets(&ids);
    let figure_block = candidate.pages[0].flows[0].blocks.remove(0);
    let freeform_block = candidate_id(&ids);
    let table_block = candidate_id(&ids);
    let table_id = candidate_id(&ids);
    let row_id = candidate_id(&ids);
    let cell_id = candidate_id(&ids);
    let list_block = candidate_id(&ids);
    let list_id = candidate_id(&ids);
    let list_item = candidate_id(&ids);
    let callout_block = candidate_id(&ids);
    candidate.pages[0].flows[0].blocks = vec![Block {
        content: BlockContent::Callout(vec![Block {
            content: BlockContent::List(List {
                id: list_id,
                items: vec![ListItem {
                    blocks: vec![Block {
                        content: BlockContent::Table(Table {
                            id: table_id,
                            rows: vec![TableRow {
                                cells: vec![TableCell {
                                    blocks: vec![Block {
                                        content: BlockContent::Freeform(vec![
                                            figure_block,
                                        ]),
                                        extensions: vec![],
                                        id: freeform_block,
                                        provenance: None,
                                        style: None,
                                    }],
                                    id: cell_id,
                                    span: TableCellSpan::SINGLE,
                                }],
                                id: row_id,
                                role: TableRowRole::Body,
                            }],
                        }),
                        extensions: vec![],
                        id: table_block,
                        provenance: None,
                        style: None,
                    }],
                    id: list_item,
                }],
                ordered: false,
            }),
            extensions: vec![],
            id: list_block,
            provenance: None,
            style: None,
        }]),
        extensions: vec![],
        id: callout_block,
        provenance: None,
        style: None,
    }];

    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("nested figure candidate must be accepted");
    };
    let figure = accepted_for(&mapping, candidate_ids.figure);
    let first_asset = accepted_for(&mapping, candidate_ids.asset_one);
    let second_asset = accepted_for(&mapping, candidate_ids.asset_two);
    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::AssetReference(
                    Some(first_asset),
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Figure),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::AssetReference,
            },
            requested: EditableSemanticValue::AssetReference(
                Some(second_asset),
            ),
            target: figure,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("nested figure asset reference must apply");
    };
    let current = session.current().expect("nested figure revision");
    let BlockContent::Callout(callout) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("outer block must remain callout");
    };
    let BlockContent::List(list) = &callout[0].content else {
        panic!("callout child must remain list");
    };
    let BlockContent::Table(table) = &list.items[0].blocks[0].content else {
        panic!("list child must remain table");
    };
    let BlockContent::Freeform(freeform) =
        &table.rows[0].cells[0].blocks[0].content
    else {
        panic!("table child must remain freeform");
    };
    let BlockContent::Figure(current_figure) = &freeform[0].content else {
        panic!("freeform child must remain figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(second_asset));
    assert_eq!(current.id, revision);
}

#[test]
fn ordered_asset_reference_chain_replaces_then_removes_one_figure_reference() {
    let ids = IdentityAllocator::new();
    let (candidate, candidate_ids) =
        candidate_notebook_with_figure_assets(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("asset chain candidate must be accepted");
    };
    let figure = accepted_for(&mapping, candidate_ids.figure);
    let first_asset = accepted_for(&mapping, candidate_ids.asset_one);
    let second_asset = accepted_for(&mapping, candidate_ids.asset_two);
    let command = |
        id,
        dependencies,
        expected,
        requested,
    | DirectEditBatchCommand {
        dependencies,
        id,
        preconditions: CommandTargetPreconditions {
            expected_value: Some(EditableSemanticValue::AssetReference(
                expected,
            )),
            identity: IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::Figure),
                expected_owner: IdentityOwnerExpectation::Any,
            },
            requested_family: SemanticCommandFamily::AssetReference,
        },
        requested: EditableSemanticValue::AssetReference(requested),
        target: figure,
    };
    let commands = vec![
        command(1_u32, vec![], Some(first_asset), Some(second_asset)),
        command(2_u32, vec![1], Some(second_asset), None),
    ];
    assert_eq!(
        session.simulate_direct_edit_batch(DirectEditBatchProposal {
            base,
            capability_version: CommandBehaviorVersion(2),
            commands: commands.clone(),
        }),
        DirectEditBatchSimulationOutcome::CapabilityMismatch {
            current: CommandBehaviorVersion(3),
            expected: CommandBehaviorVersion(2),
        },
    );

    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands,
    };
    let DirectEditBatchSimulationOutcome::Predicted {
        changes,
        commands: predictions,
        effect: DirectEditEffectClass::Mutation,
        ..
    } = session.simulate_direct_edit_batch(batch.clone())
    else {
        panic!("dependent asset-reference chain must simulate");
    };
    assert_eq!(predictions.len(), 2);
    assert_eq!(changes, vec![DirectEditSemanticChange {
        after: EditableSemanticValue::AssetReference(None),
        before: EditableSemanticValue::AssetReference(Some(first_asset)),
        family: SemanticCommandFamily::AssetReference,
        target: figure,
    }]);

    let DirectEditBatchApplyOutcome::Applied { revision: applied, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("dependent asset-reference chain must apply");
    };
    let current = session.current().expect("asset chain revision");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("asset chain fixture must remain figure");
    };
    assert_eq!(current_figure.asset, None);
    assert_eq!(current.notebook.assets.len(), 2);
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == first_asset));
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == second_asset));

    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("asset-reference chain must Undo as one transaction");
    };
    let current = session.current().expect("asset chain Undo revision");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("asset chain Undo fixture must remain figure");
    };
    assert_eq!(current_figure.asset, Some(first_asset));
    assert_eq!(current.notebook.assets.len(), 2);
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == first_asset));
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == second_asset));
}

#[test]
fn asset_reference_rejects_identity_not_present_in_current_revision() {
    let ids = IdentityAllocator::new();
    let (first, first_ids) = candidate_notebook_with_figure_assets(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, .. } = session.accept(first)
    else {
        panic!("first figure candidate must be accepted");
    };
    let old_asset = accepted_for(&mapping, first_ids.asset_one);

    let (second, second_ids) = candidate_notebook_with_figure_assets(&ids);
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(second)
    else {
        panic!("replacement figure candidate must be accepted");
    };
    let figure = accepted_for(&mapping, second_ids.figure);
    let before = session.current().expect("replacement revision").clone();
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            figure,
            EditableSemanticValue::AssetReference(Some(old_asset)),
        ),
        DirectEditSimulationOutcome::InvalidAssetReference {
            actual: None,
            reference: old_asset,
            revision,
            target: figure,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn command_capability_snapshot_is_deterministic_and_does_not_overclaim() {
    let mut session = SemanticNotebookSessionService::default();
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CommandBehaviorVersion(3));
    assert_eq!(snapshot.typed_result_version, CommandBehaviorVersion(3));
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
            family: SemanticCommandFamily::AssetReference,
        },
        CommandFamilyCapability {
            behavior_version: CommandBehaviorVersion(1),
            family: SemanticCommandFamily::DocumentConstraint,
        },
        CommandFamilyCapability {
            behavior_version: CommandBehaviorVersion(2),
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
fn internal_batch_apply_does_not_advertise_protocol_apply_capability() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "before");
    let mut service = SemanticNotebookSessionService::default();
    let before_capability = service.command_capability_snapshot();
    assert!(before_capability.admitted_applications.is_empty());
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let batch = DirectEditBatchProposal {
        base,
        capability_version: before_capability.behavior_version,
        commands: vec![text_batch_command(
            1,
            &[],
            target,
            "before",
            "after",
        )],
    };
    assert!(matches!(
        service.apply_direct_edit_batch(batch),
        DirectEditBatchApplyOutcome::Applied { .. }
    ));

    let after_capability = service.command_capability_snapshot();
    assert_eq!(after_capability, before_capability);
    assert!(after_capability.admitted_applications.is_empty());
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
            .check_command_capability_compatibility(CommandBehaviorVersion(2),),
        CommandCapabilityCompatibilityOutcome::Mismatch {
            current: CommandBehaviorVersion(3),
            expected: CommandBehaviorVersion(2),
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
            capability_version: CommandBehaviorVersion(3),
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
            capability_version: CommandBehaviorVersion(3),
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
            capability_version: CommandBehaviorVersion(3),
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
            capability_version: CommandBehaviorVersion(3),
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
            current: CommandBehaviorVersion(3),
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
            capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
            current: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
            current: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
fn graph_resource_preflight_does_not_require_ordered_command_ids() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let command = |dependencies| DirectEditBatchCommand {
        dependencies,
        id: NonOrdCommandIdentity,
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
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            command(Vec::new()),
            command(vec![NonOrdCommandIdentity]),
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
    assert_eq!(
        session.direct_edit_batch_graph_limits(&batch, CommandGraphLimits {
            commands: 2,
            dependency_edges: 1,
        }),
        DirectEditBatchGraphLimitsOutcome::Admitted { revision, size },
    );
}

#[test]
fn batch_read_only_apis_share_capability_and_stale_authority_precedence() {
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
    let selected = BTreeSet::from([1_u32]);
    let mismatch = CommandBehaviorVersion(0);
    let current_behavior = CommandBehaviorVersion(3);
    assert_eq!(
        session.direct_edit_batch_graph_size(&incompatible),
        DirectEditBatchGraphSizeOutcome::CapabilityMismatch {
            current: current_behavior,
            expected: mismatch,
        },
    );
    assert_eq!(
        session.direct_edit_batch_graph_limits(
            &incompatible,
            CommandGraphLimits {
                commands: 0,
                dependency_edges: 0,
            },
        ),
        DirectEditBatchGraphLimitsOutcome::CapabilityMismatch {
            current: current_behavior,
            expected: mismatch,
        },
    );
    assert_eq!(
        session.direct_edit_batch_selection_requirements(
            &incompatible,
            &selected,
        ),
        DirectEditBatchSelectionRequirementsOutcome::CapabilityMismatch {
            current: current_behavior,
            expected: mismatch,
        },
    );
    assert_eq!(
        session.direct_edit_batch_selection_requirements_bounded(
            &incompatible,
            &selected,
            0,
        ),
        DirectEditBatchSelectionBoundedOutcome::CapabilityMismatch {
            current: current_behavior,
            expected: mismatch,
        },
    );
    assert_eq!(
        session.direct_edit_batch_selection_summary(&incompatible, &selected),
        DirectEditBatchSelectionSummaryOutcome::CapabilityMismatch {
            current: current_behavior,
            expected: mismatch,
        },
    );
    assert_eq!(
        session.simulate_direct_edit_batch(incompatible.clone()),
        DirectEditBatchSimulationOutcome::CapabilityMismatch {
            current: current_behavior,
            expected: mismatch,
        },
    );
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(
            incompatible,
            CommandGraphLimits {
                commands: 0,
                dependency_edges: 0,
            },
        ),
        DirectEditBatchSimulationOutcome::CapabilityMismatch {
            current: current_behavior,
            expected: mismatch,
        },
    );

    let stale = DirectEditBatchProposal {
        base: revision,
        capability_version: current_behavior,
        commands: vec![text_batch_command(
            1,
            &[],
            span,
            "base text",
            "changed",
        )],
    };
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(candidate_notebook(&ids, "new revision"))
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.direct_edit_batch_graph_size(&stale),
        DirectEditBatchGraphSizeOutcome::StaleBase { current },
    );
    assert_eq!(
        session.direct_edit_batch_graph_limits(&stale, CommandGraphLimits {
            commands: 0,
            dependency_edges: 0,
        },),
        DirectEditBatchGraphLimitsOutcome::StaleBase { current },
    );
    assert_eq!(
        session.direct_edit_batch_selection_requirements(&stale, &selected),
        DirectEditBatchSelectionRequirementsOutcome::StaleBase { current },
    );
    assert_eq!(
        session.direct_edit_batch_selection_requirements_bounded(
            &stale, &selected, 0,
        ),
        DirectEditBatchSelectionBoundedOutcome::StaleBase { current },
    );
    assert_eq!(
        session.direct_edit_batch_selection_summary(&stale, &selected),
        DirectEditBatchSelectionSummaryOutcome::StaleBase { current },
    );
    assert_eq!(
        session.simulate_direct_edit_batch(stale.clone()),
        DirectEditBatchSimulationOutcome::StaleBase { current },
    );
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(stale, CommandGraphLimits {
            commands: 0,
            dependency_edges: 0,
        },),
        DirectEditBatchSimulationOutcome::StaleBase { current },
    );
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
            current: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
            current: CommandBehaviorVersion(3),
            expected: CommandBehaviorVersion(0),
        },
    );
    let stale = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
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
fn bounded_apply_resource_rejection_borrows_command_ids() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        service.accept(candidate)
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
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            command(1, Vec::new()),
            command(2, vec![CountingCommandIdentity::new(&clones, 1)]),
        ],
    };
    clones.store(0, AtomicOrdering::Relaxed);

    assert_eq!(
        service.apply_direct_edit_batch_bounded(
            batch,
            CommandGraphLimits {
                commands: 1,
                dependency_edges: 1,
            },
        ),
        DirectEditBatchApplyOutcome::ResourceRejected {
            reason: CommandGraphLimitError::CommandCountExceeded {
                actual: 2,
                limit: 1,
            },
        },
    );
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0);
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
        capability_version: CommandBehaviorVersion(3),
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
fn ordered_empty_batch_is_read_only_no_op_at_zero_limits() {
    let ids = IdentityAllocator::new();
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { revision, .. } =
        session.accept(candidate_notebook(&ids, "unchanged"))
    else {
        panic!("candidate must be accepted");
    };
    let before = session.current().expect("accepted revision").clone();
    let batch = DirectEditBatchProposal::<u32> {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: Vec::new(),
    };
    let expected = DirectEditBatchSimulationOutcome::Predicted {
        changes: Vec::new(),
        commands: Vec::new(),
        effect: DirectEditEffectClass::NoOp,
        impact_seeds: Vec::new(),
        revision,
    };
    assert_eq!(session.simulate_direct_edit_batch(batch.clone()), expected,);
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(batch, CommandGraphLimits {
            commands: 0,
            dependency_edges: 0,
        },),
        expected,
    );
    assert_eq!(session.current(), Some(&before));
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
        capability_version: CommandBehaviorVersion(3),
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
fn ordered_batch_index_reaches_table_cell_text_in_document_order() {
    let ids = IdentityAllocator::new();
    let (candidate, _, _) = candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(table) =
        &candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let BlockContent::Paragraph(spans) =
        &table.rows[0].cells[0].blocks[0].content
    else {
        panic!("table cell must contain paragraph text");
    };
    let candidate_span = spans[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("nested table candidate must be accepted");
    };
    let span = accepted_for(&mapping, candidate_span);
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(
            1,
            &[],
            span,
            "table cell text",
            "changed cell text",
        )],
    });
    let DirectEditBatchSimulationOutcome::Predicted { changes, .. } = outcome
    else {
        panic!("nested table text must simulate");
    };
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].target, span);
}

#[test]
fn ordered_batch_overlay_indexes_profile_and_text_targets_together() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "before");
    let profile = candidate.page_profiles[0].id;
    let page = candidate.pages[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let profile = accepted_for(&mapping, profile);
    let page = accepted_for(&mapping, page);
    let flow = accepted_for(&mapping, flow);
    let span = accepted_for(&mapping, span);
    let mut changed_profile = physical_page_profile();
    changed_profile.top_clearance = Length::from_micrometres(14_000);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            DirectEditBatchCommand {
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
                requested: EditableSemanticValue::PageProfile(changed_profile),
                target: profile,
            },
            text_batch_command(2, &[], span, "before", "after"),
        ],
    };
    let DirectEditBatchSimulationOutcome::Predicted {
        changes,
        impact_seeds,
        ..
    } = session.simulate_direct_edit_batch(batch)
    else {
        panic!("mixed profile and text batch must simulate");
    };
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().any(|change| change.target == profile));
    assert!(changes.iter().any(|change| change.target == span));
    assert!(impact_seeds.contains(&DirectEditImpactSeed {
        authorities: vec![DirectEditDerivedAuthority::AllDerived],
        scope: DirectEditImpactScope::Pages { pages: vec![page] },
    }));
    assert!(impact_seeds.contains(&DirectEditImpactSeed {
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
    }));
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
fn ordered_batch_index_waits_until_every_requested_target_is_resolved() {
    let ids = IdentityAllocator::new();
    let (mut candidate, first) = candidate_notebook_with_span(&ids, "first");
    let second_block = candidate_id(&ids);
    let second = candidate_id(&ids);
    candidate.pages[0].flows[0].blocks.push(Block {
        content: BlockContent::Paragraph(vec![InlineSpan {
            id: second,
            provenance: None,
            style: None,
            text: String::from("second"),
        }]),
        extensions: Vec::new(),
        id: second_block,
        provenance: None,
        style: None,
    });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("two-block candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], first, "first", "FIRST"),
            text_batch_command(2, &[], second, "second", "SECOND"),
        ],
    });
    let DirectEditBatchSimulationOutcome::Predicted { changes, .. } = outcome
    else {
        panic!("both requested targets must resolve before indexing stops");
    };
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().any(|change| change.target == first));
    assert!(changes.iter().any(|change| change.target == second));
}

#[test]
fn bounded_apply_preserves_authority_and_graph_rejection_precedence() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let before = service.current().expect("accepted revision").clone();
    let zero_limits = CommandGraphLimits {
        commands: 0,
        dependency_edges: 0,
    };
    assert_eq!(
        service.apply_direct_edit_batch_bounded(
            DirectEditBatchProposal {
                base,
                capability_version: CommandBehaviorVersion(0),
                commands: vec![text_batch_command(
                    1,
                    &[],
                    target,
                    "base text",
                    "changed",
                )],
            },
            zero_limits,
        ),
        DirectEditBatchApplyOutcome::CapabilityMismatch {
            current: CommandBehaviorVersion(3),
            expected: CommandBehaviorVersion(0),
        },
    );
    assert_eq!(service.current(), Some(&before));

    let stale = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(
            1,
            &[],
            target,
            "base text",
            "changed",
        )],
    };
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        service.accept(candidate_notebook(&ids, "replacement"))
    else {
        panic!("replacement candidate must be accepted");
    };
    let before_stale = service.current().expect("replacement revision").clone();
    assert_eq!(
        service.apply_direct_edit_batch_bounded(stale, zero_limits),
        DirectEditBatchApplyOutcome::StaleBase { current },
    );
    assert_eq!(service.current(), Some(&before_stale));

    let (candidate, span) = candidate_notebook_with_span(&ids, "graph base");
    let AcceptanceOutcome::Accepted { mapping, revision } =
        service.accept(candidate)
    else {
        panic!("graph candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let invalid_graph = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(
            1,
            &[1],
            target,
            "graph base",
            "never applied",
        )],
    };
    let before_graph = service.current().expect("graph revision").clone();
    assert_eq!(
        service.apply_direct_edit_batch_bounded(
            invalid_graph,
            CommandGraphLimits {
                commands: 1,
                dependency_edges: 1,
            },
        ),
        DirectEditBatchApplyOutcome::DependencyGraphRejected {
            reason: CommandGraphError::SelfDependency { command: 1 },
        },
    );
    assert_eq!(service.current(), Some(&before_graph));

    let empty_ids = IdentityAllocator::new();
    let empty_base = empty_ids.allocate_revision().expect("synthetic revision");
    let empty_target = empty_ids.allocate_accepted().expect("synthetic target");
    let mut empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.apply_direct_edit_batch_bounded(
            DirectEditBatchProposal {
                base: empty_base,
                capability_version: CommandBehaviorVersion(3),
                commands: vec![text_batch_command(
                    1,
                    &[],
                    empty_target,
                    "missing",
                    "changed",
                )],
            },
            zero_limits,
        ),
        DirectEditBatchApplyOutcome::NoAcceptedRevision,
    );
}

#[test]
fn bounded_direct_edit_batch_apply_rejects_resources_before_commit() {
    let ids = IdentityAllocator::new();
    let (candidate, first, second, _) =
        candidate_notebook_with_three_spans(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("three-span candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], first, "one", "ONE"),
            text_batch_command(2, &[1], second, "two", "TWO"),
        ],
    };
    let before = session.current().expect("accepted revision").clone();

    assert_eq!(
        session.apply_direct_edit_batch_bounded(
            batch.clone(),
            CommandGraphLimits {
                commands: 1,
                dependency_edges: 1,
            },
        ),
        DirectEditBatchApplyOutcome::ResourceRejected {
            reason: CommandGraphLimitError::CommandCountExceeded {
                actual: 2,
                limit: 1,
            },
        },
    );
    assert_eq!(session.current(), Some(&before));
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: false,
            revision: base,
        }),
    );

    let outcome = session.apply_direct_edit_batch_bounded(
        batch,
        CommandGraphLimits {
            commands: 2,
            dependency_edges: 1,
        },
    );
    assert!(matches!(
        outcome,
        DirectEditBatchApplyOutcome::Applied {
            base: applied_base,
            ..
        } if applied_base == base
    ));
}

#[test]
fn direct_edit_batch_apply_replays_every_established_value_family() {
    let ids = IdentityAllocator::new();
    let (mut candidate, text) =
        candidate_notebook_with_span(&ids, "before text");
    let profile = candidate.page_profiles[0].id;
    let formula = candidate_id(&ids);
    let formula_block = candidate_id(&ids);
    let table = candidate_id(&ids);
    let row = candidate_id(&ids);
    let cell = candidate_id(&ids);
    let table_block = candidate_id(&ids);
    let asset_one = candidate_id(&ids);
    let asset_two = candidate_id(&ids);
    let figure = candidate_id(&ids);
    let figure_block = candidate_id(&ids);
    candidate.assets.extend([
        Asset {
            id: asset_one,
            media_type: String::from("image/png"),
        },
        Asset {
            id: asset_two,
            media_type: String::from("image/webp"),
        },
    ]);
    candidate.pages[0].flows[0].blocks.push(Block {
        content: BlockContent::Mathematics(Formula {
            id: formula,
            mode: FormulaMode::Display,
            source: String::from("x"),
        }),
        extensions: vec![],
        id: formula_block,
        provenance: None,
        style: None,
    });
    candidate.pages[0].flows[0].blocks.push(Block {
        content: BlockContent::Table(Table {
            id: table,
            rows: vec![TableRow {
                cells: vec![TableCell {
                    blocks: vec![],
                    id: cell,
                    span: TableCellSpan::SINGLE,
                }],
                id: row,
                role: TableRowRole::Header,
            }],
        }),
        extensions: vec![],
        id: table_block,
        provenance: None,
        style: None,
    });
    candidate.pages[0].flows[0].blocks.push(Block {
        content: BlockContent::Figure(Figure {
            asset: Some(asset_one),
            caption: vec![],
            id: figure,
        }),
        extensions: vec![],
        id: figure_block,
        provenance: None,
        style: None,
    });
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("mixed editable candidate must be accepted");
    };
    let text = accepted_for(&mapping, text);
    let profile = accepted_for(&mapping, profile);
    let formula = accepted_for(&mapping, formula);
    let row = accepted_for(&mapping, row);
    let cell = accepted_for(&mapping, cell);
    let asset_one = accepted_for(&mapping, asset_one);
    let asset_two = accepted_for(&mapping, asset_two);
    let figure = accepted_for(&mapping, figure);
    let mut changed_profile = physical_page_profile();
    changed_profile.top_clearance = Length::from_micrometres(12_000);
    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], text, "before text", "after text"),
            DirectEditBatchCommand {
                dependencies: vec![],
                id: 2_u32,
                preconditions: CommandTargetPreconditions {
                    expected_value: Some(EditableSemanticValue::Formula {
                        mode: FormulaMode::Display,
                        source: String::from("x"),
                    }),
                    identity: IdentityPrecondition {
                        expected_kind: Some(SemanticIdentityKind::Formula),
                        expected_owner: IdentityOwnerExpectation::Any,
                    },
                    requested_family: SemanticCommandFamily::StructuredContent,
                },
                requested: EditableSemanticValue::Formula {
                    mode: FormulaMode::Display,
                    source: String::from("x^2"),
                },
                target: formula,
            },
            DirectEditBatchCommand {
                dependencies: vec![],
                id: 3_u32,
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
                requested: EditableSemanticValue::PageProfile(changed_profile),
                target: profile,
            },
            DirectEditBatchCommand {
                dependencies: vec![],
                id: 4_u32,
                preconditions: CommandTargetPreconditions {
                    expected_value: Some(EditableSemanticValue::TableRowRole(
                        TableRowRole::Header,
                    )),
                    identity: IdentityPrecondition {
                        expected_kind: Some(SemanticIdentityKind::TableRow),
                        expected_owner: IdentityOwnerExpectation::Any,
                    },
                    requested_family: SemanticCommandFamily::StructuredContent,
                },
                requested: EditableSemanticValue::TableRowRole(
                    TableRowRole::Body,
                ),
                target: row,
            },
            span_batch_command(
                5,
                &[],
                cell,
                TableCellSpan::SINGLE,
                table_cell_span(2, 1),
            ),
            DirectEditBatchCommand {
                dependencies: vec![],
                id: 6_u32,
                preconditions: CommandTargetPreconditions {
                    expected_value: Some(
                        EditableSemanticValue::AssetReference(Some(asset_one)),
                    ),
                    identity: IdentityPrecondition {
                        expected_kind: Some(SemanticIdentityKind::Figure),
                        expected_owner: IdentityOwnerExpectation::Any,
                    },
                    requested_family: SemanticCommandFamily::AssetReference,
                },
                requested: EditableSemanticValue::AssetReference(
                    Some(asset_two),
                ),
                target: figure,
            },
        ],
    };

    let outcome = service.apply_direct_edit_batch(batch);
    let DirectEditBatchApplyOutcome::Applied {
        changes,
        revision: applied,
        ..
    } = outcome
    else {
        panic!("mixed-family batch must apply: {outcome:?}");
    };
    assert_eq!(changes.len(), 6);
    assert_ne!(applied, base);
    let current = service.current().expect("mixed-family revision");
    assert_eq!(current.notebook.page_profiles[0].geometry, changed_profile);
    let blocks = &current.notebook.pages[0].flows[0].blocks;
    let BlockContent::Paragraph(spans) = &blocks[0].content else {
        panic!("first block must remain paragraph");
    };
    assert_eq!(spans[0].id, text);
    assert_eq!(spans[0].text, "after text");
    let BlockContent::Mathematics(math) = &blocks[1].content else {
        panic!("second block must remain mathematics");
    };
    assert_eq!(math.id, formula);
    assert_eq!(math.source, "x^2");
    let BlockContent::Table(table) = &blocks[2].content else {
        panic!("third block must remain table");
    };
    assert_eq!(table.rows[0].id, row);
    assert_eq!(table.rows[0].role, TableRowRole::Body);
    assert_eq!(table.rows[0].cells[0].id, cell);
    assert_eq!(table.rows[0].cells[0].span, table_cell_span(2, 1));
    let BlockContent::Figure(current_figure) = &blocks[3].content else {
        panic!("fourth block must remain figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(asset_two));

    let HistoryTraversalOutcome::Traversed { .. } =
        service.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("mixed-family Apply must Undo as one transaction");
    };
    let current = service.current().expect("mixed-family Undo revision");
    assert_eq!(
        current.notebook.page_profiles[0].geometry,
        physical_page_profile(),
    );
    let blocks = &current.notebook.pages[0].flows[0].blocks;
    let BlockContent::Paragraph(spans) = &blocks[0].content else {
        panic!("Undo first block must remain paragraph");
    };
    assert_eq!(spans[0].text, "before text");
    let BlockContent::Mathematics(math) = &blocks[1].content else {
        panic!("Undo second block must remain mathematics");
    };
    assert_eq!(math.source, "x");
    let BlockContent::Table(table) = &blocks[2].content else {
        panic!("Undo third block must remain table");
    };
    assert_eq!(table.rows[0].role, TableRowRole::Header);
    assert_eq!(table.rows[0].cells[0].span, TableCellSpan::SINGLE);
    let BlockContent::Figure(current_figure) = &blocks[3].content else {
        panic!("Undo fourth block must remain figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(asset_one));
}

#[test]
fn direct_edit_batch_apply_matches_prediction_and_undoes_as_one_transaction() {
    let ids = IdentityAllocator::new();
    let (candidate, first, second, _) =
        candidate_notebook_with_three_spans(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("three-span candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], first, "one", "ONE"),
            text_batch_command(2, &[], second, "two", "TWO"),
        ],
    };
    let predicted = session.simulate_direct_edit_batch(batch.clone());
    let DirectEditBatchSimulationOutcome::Predicted {
        changes: predicted_changes,
        commands: predicted_commands,
        effect: DirectEditEffectClass::Mutation,
        impact_seeds: predicted_impact,
        revision: predicted_base,
    } = predicted
    else {
        panic!("valid batch must predict a mutation");
    };
    assert_eq!(predicted_base, base);

    let applied = session.apply_direct_edit_batch(batch);
    let DirectEditBatchApplyOutcome::Applied {
        base: applied_base,
        changes,
        commands,
        impact_seeds,
        revision: result,
    } = applied
    else {
        panic!("valid batch must apply: {applied:?}");
    };
    assert_eq!(applied_base, base);
    assert_eq!(changes, predicted_changes);
    assert_eq!(commands, predicted_commands);
    assert_eq!(impact_seeds, predicted_impact);
    assert_ne!(result, base);
    let current = session.current().expect("applied revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must remain a paragraph");
    };
    assert_eq!(spans[0].id, first);
    assert_eq!(spans[0].text, "ONE");
    assert_eq!(spans[1].id, second);
    assert_eq!(spans[1].text, "TWO");

    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        session.traverse_history(result, HistoryDirection::Undo)
    else {
        panic!("batch application must enter history as one transaction");
    };
    let current = session.current().expect("Undo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo fixture must remain a paragraph");
    };
    assert_eq!(spans[0].text, "one");
    assert_eq!(spans[1].text, "two");
    assert_ne!(undone, base);
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: true,
            can_undo: false,
            revision: undone,
        }),
    );
}

#[test]
fn direct_edit_batch_apply_net_noop_keeps_revision_and_history_position() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let before = session.current().expect("accepted revision").clone();
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], target, "base text", "changed"),
            text_batch_command(2, &[1], target, "changed", "base text"),
        ],
    };

    let outcome = session.apply_direct_edit_batch(batch);
    let DirectEditBatchApplyOutcome::NoOp {
        commands,
        revision: unchanged,
    } = outcome
    else {
        panic!("change-then-revert must apply as a no-op: {outcome:?}");
    };
    assert_eq!(commands.len(), 2);
    assert_eq!(unchanged, revision);
    assert_eq!(session.current(), Some(&before));
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: false,
            revision,
        }),
    );
}

#[test]
fn noncommitting_direct_and_candidate_attempts_preserve_redo_branch() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "zero");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let TextEditOutcome::Applied { revision: first, .. } =
        service.replace_text(base, target, String::from("one"))
    else {
        panic!("first edit must apply");
    };
    let TextEditOutcome::Applied { revision: second, .. } =
        service.replace_text(first, target, String::from("two"))
    else {
        panic!("second edit must apply");
    };
    let HistoryTraversalOutcome::Traversed {
        revision: branch_base,
        ..
    } = service.traverse_history(second, HistoryDirection::Undo)
    else {
        panic!("second edit must Undo");
    };
    let expected_history = HistoryAvailabilityOutcome::Available(
        HistoryAvailability {
            can_redo: true,
            can_undo: true,
            revision: branch_base,
        },
    );

    assert_eq!(
        service.replace_text(branch_base, target, String::from("one")),
        TextEditOutcome::NoOp {
            revision: branch_base,
            target,
        },
    );
    assert_eq!(service.history_availability(), expected_history);
    assert_eq!(
        service.replace_text(second, target, String::from("stale")),
        TextEditOutcome::StaleBase {
            current: branch_base,
        },
    );
    assert_eq!(service.history_availability(), expected_history);

    let mut invalid = candidate_notebook(&ids, "invalid replacement");
    invalid.pages[0].id = invalid.id;
    assert!(matches!(
        service.accept(invalid),
        AcceptanceOutcome::InvalidCandidate { .. }
    ));
    assert_eq!(service.history_availability(), expected_history);

    let HistoryTraversalOutcome::Traversed { revision: redone, .. } =
        service.traverse_history(branch_base, HistoryDirection::Redo)
    else {
        panic!("preserved direct redo branch must remain traversable");
    };
    let current = service.current().expect("redone revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("redo fixture must remain paragraph");
    };
    assert_eq!(spans[0].id, target);
    assert_eq!(spans[0].text, "two");
    assert_eq!(current.id, redone);
}

#[test]
fn noncommitting_batch_attempts_preserve_an_existing_redo_branch() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "zero");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let TextEditOutcome::Applied { revision: first, .. } =
        service.replace_text(base, target, String::from("one"))
    else {
        panic!("first edit must apply");
    };
    let TextEditOutcome::Applied { revision: second, .. } =
        service.replace_text(first, target, String::from("two"))
    else {
        panic!("second edit must apply");
    };
    let HistoryTraversalOutcome::Traversed {
        revision: branch_base,
        ..
    } = service.traverse_history(second, HistoryDirection::Undo)
    else {
        panic!("second edit must Undo");
    };
    let expected_history = HistoryAvailabilityOutcome::Available(
        HistoryAvailability {
            can_redo: true,
            can_undo: true,
            revision: branch_base,
        },
    );
    assert_eq!(service.history_availability(), expected_history);

    let no_op = DirectEditBatchProposal {
        base: branch_base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(1, &[], target, "one", "one")],
    };
    assert!(matches!(
        service.apply_direct_edit_batch(no_op),
        DirectEditBatchApplyOutcome::NoOp {
            revision,
            ..
        } if revision == branch_base
    ));
    assert_eq!(service.history_availability(), expected_history);

    let rejected = DirectEditBatchProposal {
        base: branch_base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(2, &[], target, "wrong", "changed")],
    };
    assert!(matches!(
        service.apply_direct_edit_batch(rejected),
        DirectEditBatchApplyOutcome::Rejected { revision, .. }
            if revision == branch_base
    ));
    assert_eq!(service.history_availability(), expected_history);

    let over_limit = DirectEditBatchProposal {
        base: branch_base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(3, &[], target, "one", "temporary"),
            text_batch_command(4, &[3], target, "temporary", "changed"),
        ],
    };
    assert_eq!(
        service.apply_direct_edit_batch_bounded(
            over_limit,
            CommandGraphLimits {
                commands: 1,
                dependency_edges: 1,
            },
        ),
        DirectEditBatchApplyOutcome::ResourceRejected {
            reason: CommandGraphLimitError::CommandCountExceeded {
                actual: 2,
                limit: 1,
            },
        },
    );
    assert_eq!(service.history_availability(), expected_history);

    let HistoryTraversalOutcome::Traversed { revision: redone, .. } =
        service.traverse_history(branch_base, HistoryDirection::Redo)
    else {
        panic!("preserved redo branch must remain traversable");
    };
    let current = service.current().expect("redone revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("redo fixture must remain paragraph");
    };
    assert_eq!(spans[0].id, target);
    assert_eq!(spans[0].text, "two");
    assert_eq!(current.id, redone);
}

#[test]
fn repeated_batch_apply_history_round_trip_preserves_every_snapshot() {
    const TRANSACTIONS: u32 = 64;

    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "value-0");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let mut revisions = vec![base];
    for index in 1..=TRANSACTIONS {
        let prior = format!("value-{}", index.saturating_sub(1));
        let next = format!("value-{index}");
        let batch = DirectEditBatchProposal {
            base: *revisions.last().expect("current revision"),
            capability_version: CommandBehaviorVersion(3),
            commands: vec![text_batch_command(
                index,
                &[],
                target,
                &prior,
                &next,
            )],
        };
        let DirectEditBatchApplyOutcome::Applied { revision, .. } =
            service.apply_direct_edit_batch(batch)
        else {
            panic!("transaction {index} must apply");
        };
        assert!(!revisions.contains(&revision));
        revisions.push(revision);
    }

    let mut current = *revisions.last().expect("last applied revision");
    for index in (1..=TRANSACTIONS).rev() {
        let HistoryTraversalOutcome::Traversed { revision, .. } =
            service.traverse_history(current, HistoryDirection::Undo)
        else {
            panic!("transaction {index} must Undo");
        };
        current = revision;
        let accepted = service.current().expect("Undo revision");
        let BlockContent::Paragraph(spans) =
            &accepted.notebook.pages[0].flows[0].blocks[0].content
        else {
            panic!("Undo fixture must remain paragraph");
        };
        assert_eq!(spans[0].id, target);
        assert_eq!(spans[0].text, format!("value-{}", index - 1));
    }
    assert_eq!(
        service.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: true,
            can_undo: false,
            revision: current,
        }),
    );

    for index in 1..=TRANSACTIONS {
        let HistoryTraversalOutcome::Traversed { revision, .. } =
            service.traverse_history(current, HistoryDirection::Redo)
        else {
            panic!("transaction {index} must Redo");
        };
        assert!(!revisions.contains(&revision));
        revisions.push(revision);
        current = revision;
        let accepted = service.current().expect("Redo revision");
        let BlockContent::Paragraph(spans) =
            &accepted.notebook.pages[0].flows[0].blocks[0].content
        else {
            panic!("Redo fixture must remain paragraph");
        };
        assert_eq!(spans[0].id, target);
        assert_eq!(spans[0].text, format!("value-{index}"));
    }
    assert_eq!(
        service.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: true,
            revision: current,
        }),
    );
}

#[test]
fn direct_edit_batch_history_redoes_transaction_and_discards_old_branch() {
    let ids = IdentityAllocator::new();
    let (candidate, first, second, third) =
        candidate_notebook_with_three_spans(&ids);
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("three-span candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let third = accepted_for(&mapping, third);
    let initial = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], first, "one", "ONE"),
            text_batch_command(2, &[], second, "two", "TWO"),
        ],
    };
    let DirectEditBatchApplyOutcome::Applied {
        revision: applied, ..
    } = service.apply_direct_edit_batch(initial)
    else {
        panic!("initial batch must apply");
    };
    let HistoryTraversalOutcome::Traversed {
        revision: undone, ..
    } = service.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("initial batch must Undo");
    };
    let HistoryTraversalOutcome::Traversed {
        revision: redone, ..
    } = service.traverse_history(undone, HistoryDirection::Redo)
    else {
        panic!("initial batch must Redo");
    };
    let current = service.current().expect("Redo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Redo fixture must remain paragraph");
    };
    assert_eq!(spans[0].text, "ONE");
    assert_eq!(spans[1].text, "TWO");
    assert_ne!(redone, applied);

    let HistoryTraversalOutcome::Traversed {
        revision: branch_base,
        ..
    } = service.traverse_history(redone, HistoryDirection::Undo)
    else {
        panic!("redone batch must Undo again");
    };
    let branch = DirectEditBatchProposal {
        base: branch_base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(
            3,
            &[],
            third,
            "three",
            "branch",
        )],
    };
    let DirectEditBatchApplyOutcome::Applied {
        revision: branched, ..
    } = service.apply_direct_edit_batch(branch)
    else {
        panic!("branch batch must apply");
    };
    assert_eq!(
        service.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: true,
            revision: branched,
        }),
    );
    assert_eq!(
        service.traverse_history(branched, HistoryDirection::Redo),
        HistoryTraversalOutcome::Boundary {
            direction: HistoryDirection::Redo,
            revision: branched,
        },
    );
}

#[test]
fn direct_edit_batch_apply_middle_failure_is_atomic() {
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
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], first, "one", "ONE"),
            text_batch_command(2, &[], second, "wrong", "TWO"),
            text_batch_command(3, &[], third, "three", "THREE"),
        ],
    };
    let before = session.current().expect("accepted revision").clone();
    let predicted = session.simulate_direct_edit_batch(batch.clone());

    let applied = session.apply_direct_edit_batch(batch);
    let DirectEditBatchApplyOutcome::Rejected {
        command,
        evaluated,
        not_evaluated,
        reason,
        revision: rejected_revision,
    } = applied
    else {
        panic!("middle failure must reject application: {applied:?}");
    };
    let DirectEditBatchSimulationOutcome::Rejected {
        command: predicted_command,
        evaluated: predicted_evaluated,
        not_evaluated: predicted_not_evaluated,
        reason: predicted_reason,
        revision: predicted_revision,
    } = predicted
    else {
        panic!("middle failure must also reject simulation");
    };
    assert_eq!(command, predicted_command);
    assert_eq!(evaluated, predicted_evaluated);
    assert_eq!(not_evaluated, predicted_not_evaluated);
    assert_eq!(reason, predicted_reason);
    assert_eq!(rejected_revision, predicted_revision);
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn concurrent_history_undo_and_batch_apply_allow_one_commit_from_one_base() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let TextEditOutcome::Applied { revision: shared, .. } =
        service.replace_text(base, target, String::from("first edit"))
    else {
        panic!("first edit must create the shared race base");
    };
    let batch = DirectEditBatchProposal {
        base: shared,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(
            1,
            &[],
            target,
            "first edit",
            "applied edit",
        )],
    };
    let service = Arc::new(Mutex::new(service));
    let barrier = Arc::new(Barrier::new(3));

    let undo_service = Arc::clone(&service);
    let undo_barrier = Arc::clone(&barrier);
    let undo = std::thread::spawn(move || {
        undo_barrier.wait();
        undo_service
            .lock()
            .expect("Undo race lock")
            .traverse_history(shared, HistoryDirection::Undo)
    });
    let apply_service = Arc::clone(&service);
    let apply_barrier = Arc::clone(&barrier);
    let apply = std::thread::spawn(move || {
        apply_barrier.wait();
        apply_service
            .lock()
            .expect("Apply race lock")
            .apply_direct_edit_batch(batch)
    });
    barrier.wait();
    let undo = undo.join().expect("Undo race thread");
    let apply = apply.join().expect("Apply race thread");
    let service = service.lock().expect("final race lock");

    match (undo, apply) {
        (
            HistoryTraversalOutcome::Traversed {
                base: undo_base,
                direction: HistoryDirection::Undo,
                revision: winner,
            },
            DirectEditBatchApplyOutcome::StaleBase { current },
        ) => {
            assert_eq!(undo_base, shared);
            assert_eq!(current, winner);
            assert_eq!(
                service.current().map(|revision| revision.id),
                Some(winner),
            );
            let current = service.current().expect("Undo-winning revision");
            let BlockContent::Paragraph(spans) =
                &current.notebook.pages[0].flows[0].blocks[0].content
            else {
                panic!("Undo-winning fixture must remain a paragraph");
            };
            assert_eq!(spans[0].id, target);
            assert_eq!(spans[0].text, "base text");
            assert_eq!(
                service.history_availability(),
                HistoryAvailabilityOutcome::Available(HistoryAvailability {
                    can_redo: true,
                    can_undo: false,
                    revision: winner,
                }),
            );
        },
        (
            HistoryTraversalOutcome::StaleBase { current, requested },
            DirectEditBatchApplyOutcome::Applied {
                base: applied_base,
                revision: winner,
                ..
            },
        ) => {
            assert_eq!(applied_base, shared);
            assert_eq!(requested, shared);
            assert_eq!(current, winner);
            assert_eq!(
                service.current().map(|revision| revision.id),
                Some(winner),
            );
            let current = service.current().expect("Apply-winning revision");
            let BlockContent::Paragraph(spans) =
                &current.notebook.pages[0].flows[0].blocks[0].content
            else {
                panic!("Apply-winning fixture must remain a paragraph");
            };
            assert_eq!(spans[0].id, target);
            assert_eq!(spans[0].text, "applied edit");
            assert_eq!(
                service.history_availability(),
                HistoryAvailabilityOutcome::Available(HistoryAvailability {
                    can_redo: false,
                    can_undo: true,
                    revision: winner,
                }),
            );
        },
        other => panic!("unexpected Undo/Apply race outcomes: {other:?}"),
    }
}

#[test]
fn concurrent_direct_edit_batch_apply_allows_one_commit_from_one_base() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let first_batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(
            1,
            &[],
            target,
            "base text",
            "first winner",
        )],
    };
    let second_batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(
            2,
            &[],
            target,
            "base text",
            "second winner",
        )],
    };
    let service = Arc::new(Mutex::new(service));
    let barrier = Arc::new(Barrier::new(3));

    let first_service = Arc::clone(&service);
    let first_barrier = Arc::clone(&barrier);
    let first = std::thread::spawn(move || {
        first_barrier.wait();
        first_service
            .lock()
            .expect("first application lock")
            .apply_direct_edit_batch(first_batch)
    });
    let second_service = Arc::clone(&service);
    let second_barrier = Arc::clone(&barrier);
    let second = std::thread::spawn(move || {
        second_barrier.wait();
        second_service
            .lock()
            .expect("second application lock")
            .apply_direct_edit_batch(second_batch)
    });
    barrier.wait();
    let outcomes = [
        first.join().expect("first application thread"),
        second.join().expect("second application thread"),
    ];

    let mut applied_revision = None;
    let mut stale_revision = None;
    for outcome in outcomes {
        match outcome {
            DirectEditBatchApplyOutcome::Applied { revision, .. } => {
                assert!(applied_revision.replace(revision).is_none());
            },
            DirectEditBatchApplyOutcome::StaleBase { current } => {
                assert!(stale_revision.replace(current).is_none());
            },
            other => panic!("unexpected concurrent Apply outcome: {other:?}"),
        }
    }
    let applied = applied_revision.expect("one application must commit");
    assert_eq!(stale_revision, Some(applied));

    let mut service = service.lock().expect("final application lock");
    assert_eq!(service.current().map(|current| current.id), Some(applied));
    let current = service.current().expect("winning revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must remain a paragraph");
    };
    assert!(matches!(
        spans[0].text.as_str(),
        "first winner" | "second winner"
    ));

    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        service.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("winning Apply must be one Undo transaction");
    };
    assert_ne!(undone, base);
    let current = service.current().expect("Undo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo fixture must remain a paragraph");
    };
    assert_eq!(spans[0].id, target);
    assert_eq!(spans[0].text, "base text");
}

#[test]
fn direct_edit_batch_apply_refuses_stale_base_after_another_commit() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base text");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, span);
    let batch = DirectEditBatchProposal {
        base,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![text_batch_command(
            1,
            &[],
            target,
            "base text",
            "batch text",
        )],
    };
    assert!(matches!(
        session.simulate_direct_edit_batch(batch.clone()),
        DirectEditBatchSimulationOutcome::Predicted {
            effect: DirectEditEffectClass::Mutation,
            ..
        }
    ));
    let TextEditOutcome::Applied { revision: current, .. } =
        session.replace_text(base, target, String::from("concurrent edit"))
    else {
        panic!("intervening direct edit must apply");
    };
    let before = session.current().expect("intervening revision").clone();

    assert_eq!(
        session.apply_direct_edit_batch(batch),
        DirectEditBatchApplyOutcome::StaleBase { current },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn table_cell_span_is_structured_editable_material() {
    let ids = IdentityAllocator::new();
    let (candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(table) =
        &candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let candidate_cell = table.rows[0].cells[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let cell = accepted_for(&mapping, candidate_cell);
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(revision, cell)
    else {
        panic!("table cell material must be prepared");
    };
    assert_eq!(
        material.editable_value,
        Some(EditableSemanticValue::TableCellSpan(TableCellSpan::SINGLE)),
    );
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::StructuredContent),
    );
    let requested = EditableSemanticValue::TableCellSpan(table_cell_span(2, 1));
    assert_eq!(
        session.simulate_direct_edit(revision, cell, requested.clone()),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::StructuredContent,
            requested,
            revision,
            target: cell,
        },
    );
}

#[test]
fn table_cell_span_simulation_rejects_invalid_owning_grid() {
    let ids = IdentityAllocator::new();
    let (mut candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(table) =
        &mut candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let candidate_cell = table.rows[0].cells[0].id;
    let second_row = table_row(
        &ids,
        vec![empty_table_cell(&ids, TableCellSpan::SINGLE)],
    );
    let candidate_second_row = second_row.id;
    table.rows.push(second_row);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("two-row table candidate must be accepted");
    };
    let cell = accepted_for(&mapping, candidate_cell);
    let second_row = accepted_for(&mapping, candidate_second_row);
    let before = session.current().expect("accepted revision").clone();
    let requested = table_cell_span(2, 1);
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            cell,
            EditableSemanticValue::TableCellSpan(requested),
        ),
        DirectEditSimulationOutcome::InvalidTableGrid {
            reason: TableGridError::RowWidth { row: second_row },
            revision,
            target: cell,
        },
    );
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![span_batch_command(
            1,
            &[],
            cell,
            TableCellSpan::SINGLE,
            requested,
        )],
    };
    let DirectEditBatchSimulationOutcome::Rejected { reason, .. } =
        session.simulate_direct_edit_batch(batch.clone())
    else {
        panic!("invalid table span batch must reject");
    };
    assert!(matches!(
        *reason,
        DirectEditBatchCommandRejection::Simulation { outcome }
            if *outcome == DirectEditSimulationOutcome::InvalidTableGrid {
                reason: TableGridError::RowWidth { row: second_row },
                revision,
                target: cell,
            }
    ));
    assert!(matches!(
        session.apply_direct_edit_batch(batch),
        DirectEditBatchApplyOutcome::Rejected { .. }
    ));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn ordered_table_span_overlay_observes_dependent_candidate_value() {
    let ids = IdentityAllocator::new();
    let (candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(table) =
        &candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let candidate_cell = table.rows[0].cells[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let cell = accepted_for(&mapping, candidate_cell);
    let middle = table_cell_span(2, 1);
    let final_span = table_cell_span(3, 1);
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            span_batch_command(1, &[], cell, TableCellSpan::SINGLE, middle),
            span_batch_command(2, &[1], cell, middle, final_span),
        ],
    };
    let DirectEditBatchSimulationOutcome::Predicted { changes, .. } =
        session.simulate_direct_edit_batch(batch.clone())
    else {
        panic!("dependent table span batch must simulate");
    };
    assert_eq!(changes, vec![DirectEditSemanticChange {
        after: EditableSemanticValue::TableCellSpan(final_span),
        before: EditableSemanticValue::TableCellSpan(TableCellSpan::SINGLE),
        family: SemanticCommandFamily::StructuredContent,
        target: cell,
    }]);
    let DirectEditBatchApplyOutcome::Applied { revision: applied, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("dependent table span batch must apply");
    };
    let current = session.current().expect("applied table revision");
    let BlockContent::Table(table) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("applied fixture must remain table");
    };
    assert_eq!(table.rows[0].cells[0].span, final_span);
    assert!(matches!(
        session.traverse_history(applied, HistoryDirection::Undo),
        HistoryTraversalOutcome::Traversed { .. }
    ));
    let current = session.current().expect("Undo table revision");
    let BlockContent::Table(table) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo fixture must remain table");
    };
    assert_eq!(table.rows[0].cells[0].span, TableCellSpan::SINGLE);
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
fn applied_same_target_chain_moves_command_ids_without_cloning() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        service.accept(candidate)
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
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            command(1, Vec::new(), "base", "one"),
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
    clones.store(0, AtomicOrdering::Relaxed);

    let DirectEditBatchApplyOutcome::Applied { commands, .. } =
        service.apply_direct_edit_batch(batch)
    else {
        panic!("dependent same-target chain must apply");
    };
    assert_eq!(commands.len(), 3);
    assert_eq!(commands[0].command.id, 1);
    assert_eq!(commands[1].command.id, 2);
    assert_eq!(commands[2].command.id, 3);
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0);
}

#[test]
fn ordered_same_target_success_moves_command_ids_without_cloning() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
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
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            command(1, Vec::new(), "base", "one"),
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
    clones.store(0, AtomicOrdering::Relaxed);
    let DirectEditBatchSimulationOutcome::Predicted { commands, .. } =
        session.simulate_direct_edit_batch(batch)
    else {
        panic!("dependent same-target chain must simulate");
    };
    assert_eq!(commands.len(), 3);
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0);
}

#[test]
fn ordered_semantic_rejection_moves_result_command_ids_without_cloning() {
    let ids = IdentityAllocator::new();
    let (candidate, first, second, third) =
        candidate_notebook_with_three_spans(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let first = accepted_for(&mapping, first);
    let second = accepted_for(&mapping, second);
    let third = accepted_for(&mapping, third);
    let clones = Arc::new(AtomicUsize::new(0));
    let command =
        |id, target, expected: &str, requested: &str| DirectEditBatchCommand {
            dependencies: Vec::new(),
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
            target,
        };
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            command(1, first, "one", "ONE"),
            command(2, second, "wrong", "TWO"),
            command(3, third, "three", "THREE"),
        ],
    };
    clones.store(0, AtomicOrdering::Relaxed);
    let DirectEditBatchSimulationOutcome::Rejected {
        command,
        evaluated,
        not_evaluated,
        ..
    } = session.simulate_direct_edit_batch(batch)
    else {
        panic!("middle semantic mismatch must reject");
    };
    assert_eq!(command.id, 2);
    assert_eq!(evaluated.len(), 1);
    assert_eq!(evaluated[0].command.id, 1);
    assert_eq!(not_evaluated.len(), 1);
    assert_eq!(not_evaluated[0].id, 3);
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0);
}

#[test]
fn ordered_missing_target_dependency_clones_only_dependency_evidence() {
    let ids = IdentityAllocator::new();
    let (candidate, span) = candidate_notebook_with_span(&ids, "base");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let span = accepted_for(&mapping, span);
    let clones = Arc::new(AtomicUsize::new(0));
    let command =
        |id, expected: &str, requested: &str| DirectEditBatchCommand {
            dependencies: Vec::new(),
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
        };
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            command(1, "base", "middle"),
            command(2, "middle", "final"),
        ],
    };
    clones.store(0, AtomicOrdering::Relaxed);
    let DirectEditBatchSimulationOutcome::Rejected { reason, .. } =
        session.simulate_direct_edit_batch(batch)
    else {
        panic!("missing same-target dependency must reject");
    };
    assert!(matches!(
        *reason,
        DirectEditBatchCommandRejection::MissingPriorTargetDependency {
            dependency,
            target,
        } if dependency.id == 1 && target == span
    ));
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 1);
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
            capability_version: CommandBehaviorVersion(3),
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
            capability_version: CommandBehaviorVersion(3),
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
fn ordered_batch_index_coalescing_preserves_first_change_order() {
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
    let outcome = session.simulate_direct_edit_batch(DirectEditBatchProposal {
        base: revision,
        capability_version: CommandBehaviorVersion(3),
        commands: vec![
            text_batch_command(1, &[], second, "two", "TWO-1"),
            text_batch_command(2, &[], first, "one", "ONE-1"),
            text_batch_command(3, &[1], second, "TWO-1", "TWO-2"),
            text_batch_command(4, &[2], first, "ONE-1", "ONE-2"),
            text_batch_command(5, &[], third, "three", "temporary"),
            text_batch_command(6, &[5], third, "temporary", "three"),
        ],
    });
    let DirectEditBatchSimulationOutcome::Predicted {
        changes,
        commands,
        effect,
        ..
    } = outcome
    else {
        panic!("interleaved dependent edits must simulate");
    };
    assert_eq!(commands.len(), 6);
    assert!(commands.iter().all(|command| command.change.is_some()));
    assert_eq!(effect, DirectEditEffectClass::Mutation);
    assert_eq!(changes, vec![
        DirectEditSemanticChange {
            after: EditableSemanticValue::Text(String::from("TWO-2")),
            before: EditableSemanticValue::Text(String::from("two")),
            family: SemanticCommandFamily::TextContent,
            target: second,
        },
        DirectEditSemanticChange {
            after: EditableSemanticValue::Text(String::from("ONE-2")),
            before: EditableSemanticValue::Text(String::from("one")),
            family: SemanticCommandFamily::TextContent,
            target: first,
        },
    ]);
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
            capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
            current: CommandBehaviorVersion(3),
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
            capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
        capability_version: CommandBehaviorVersion(3),
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
                                    span: TableCellSpan::SINGLE,
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
fn direct_table_cell_span_edit_commits_and_enters_history() {
    let ids = IdentityAllocator::new();
    let (candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Header);
    let BlockContent::Table(candidate_table) =
        &candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let candidate_cell = candidate_table.rows[0].cells[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_cell);
    let before = session.current().expect("base table revision");
    let BlockContent::Table(before_table) =
        &before.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("base block must remain a table");
    };
    let child_blocks = before_table.rows[0].cells[0].blocks.clone();
    let replacement = table_cell_span(2, 1);

    let outcome = session.replace_table_cell_span(base, target, replacement);
    let TableCellSpanEditOutcome::Applied { revision: edited, .. } = outcome
    else {
        panic!("valid cell span edit must apply: {outcome:?}");
    };
    let current = session.current().expect("edited table revision");
    let BlockContent::Table(table) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("edited block must remain a table");
    };
    assert_eq!(table.rows[0].cells[0].blocks, child_blocks);
    assert_eq!(table.rows[0].cells[0].id, target);
    assert_eq!(table.rows[0].cells[0].span, replacement);
    assert!(matches!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_undo: true,
            revision,
            ..
        }) if revision == edited
    ));

    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        session.traverse_history(edited, HistoryDirection::Undo)
    else {
        panic!("cell span edit must be undoable");
    };
    let current = session.current().expect("Undo table revision");
    let BlockContent::Table(table) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo block must remain a table");
    };
    assert_eq!(table.rows[0].cells[0].id, target);
    assert_eq!(table.rows[0].cells[0].span, TableCellSpan::SINGLE);
    assert_ne!(undone, base);
}

#[test]
fn direct_table_cell_span_edit_rejects_invalid_grid_atomically() {
    let ids = IdentityAllocator::new();
    let (candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(candidate_table) =
        &candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let candidate_cell = candidate_table.rows[0].cells[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_cell);
    let before = session.current().expect("base table revision").clone();

    assert_eq!(
        session.replace_table_cell_span(
            revision,
            target,
            table_cell_span(1, 2),
        ),
        TableCellSpanEditOutcome::InvalidTableGrid {
            reason: TableGridError::RowSpan { cell: target },
            revision,
            target,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_table_cell_span_edit_revalidates_later_rows_atomically() {
    let ids = IdentityAllocator::new();
    let (mut candidate, _, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(table) =
        &mut candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let candidate_cell = table.rows[0].cells[0].id;
    let second_row = table_row(
        &ids,
        vec![empty_table_cell(&ids, TableCellSpan::SINGLE)],
    );
    let candidate_second_row = second_row.id;
    table.rows.push(second_row);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("two-row table candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_cell);
    let second_row = accepted_for(&mapping, candidate_second_row);
    let before = session.current().expect("base table revision").clone();

    assert_eq!(
        session.replace_table_cell_span(
            revision,
            target,
            table_cell_span(2, 1),
        ),
        TableCellSpanEditOutcome::InvalidTableGrid {
            reason: TableGridError::RowWidth { row: second_row },
            revision,
            target,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn direct_table_cell_span_edit_preserves_typed_no_effects() {
    let ids = IdentityAllocator::new();
    let (candidate, row, _) =
        candidate_table_notebook(&ids, TableRowRole::Body);
    let BlockContent::Table(candidate_table) =
        &candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must contain a table");
    };
    let candidate_cell = candidate_table.rows[0].cells[0].id;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("table candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_cell);
    let row = accepted_for(&mapping, row);
    assert_eq!(
        session.replace_table_cell_span(base, target, TableCellSpan::SINGLE),
        TableCellSpanEditOutcome::NoOp { revision: base, target },
    );
    assert_eq!(
        session.replace_table_cell_span(base, row, TableCellSpan::SINGLE),
        TableCellSpanEditOutcome::TargetNotTableCell {
            revision: base,
            target: row,
        },
    );
    let TableCellSpanEditOutcome::Applied { revision: edited, .. } =
        session.replace_table_cell_span(base, target, table_cell_span(2, 1))
    else {
        panic!("valid span edit must apply");
    };
    assert_eq!(
        session.replace_table_cell_span(base, target, TableCellSpan::SINGLE),
        TableCellSpanEditOutcome::StaleBase { current: edited },
    );

    let replacement = candidate_notebook(&ids, "replacement");
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(replacement)
    else {
        panic!("replacement candidate must be accepted");
    };
    assert_eq!(
        session.replace_table_cell_span(
            current,
            target,
            TableCellSpan::SINGLE,
        ),
        TableCellSpanEditOutcome::TargetNotFound {
            revision: current,
            target,
        },
    );

    let mut empty = SemanticNotebookSessionService::default();
    assert_eq!(
        empty.replace_table_cell_span(
            current,
            target,
            TableCellSpan::SINGLE,
        ),
        TableCellSpanEditOutcome::NoAcceptedRevision,
    );
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
) -> (
    Block<CandidateIdentity>,
    CandidateIdentity,
    CandidateIdentity,
) {
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
                        span: TableCellSpan::SINGLE,
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
        cell_id,
    )
}

#[test]
fn direct_table_row_role_edit_reaches_nested_structures_across_revisions() {
    let ids = IdentityAllocator::new();
    let mut candidate = candidate_notebook(&ids, "discarded seed");
    let (callout_table, callout_row, _) =
        candidate_table_block(&ids, TableRowRole::Body);
    let (list_table, list_row, _) =
        candidate_table_block(&ids, TableRowRole::Body);
    let (cell_table, cell_row, _) =
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
                        span: TableCellSpan::SINGLE,
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
fn direct_table_cell_span_edit_reaches_nested_structures_across_revisions() {
    let ids = IdentityAllocator::new();
    let mut candidate = candidate_notebook(&ids, "discarded seed");
    let (callout_table, _, callout_cell) =
        candidate_table_block(&ids, TableRowRole::Body);
    let (list_table, _, list_cell) =
        candidate_table_block(&ids, TableRowRole::Body);
    let (cell_table, _, nested_cell) =
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
                        span: TableCellSpan::SINGLE,
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
    let replacement = table_cell_span(2, 1);
    for candidate_cell in [callout_cell, list_cell, nested_cell] {
        let target = accepted_for(&mapping, candidate_cell);
        let TableCellSpanEditOutcome::Applied {
            revision: next,
            target: actual_target,
            ..
        } = session.replace_table_cell_span(revision, target, replacement)
        else {
            panic!("nested cell-span edit must apply");
        };
        assert_eq!(actual_target, target);
        assert_ne!(next, revision);
        revision = next;
    }
    let current = session.current().expect("nested cell-span revision");
    let blocks = &current.notebook.pages[0].flows[0].blocks;
    let BlockContent::Callout(callout) = &blocks[0].content else {
        panic!("first block must remain callout");
    };
    let BlockContent::Table(callout_table) = &callout[0].content else {
        panic!("callout child must remain table");
    };
    assert_eq!(callout_table.rows[0].cells[0].span, replacement);
    let BlockContent::List(list) = &blocks[1].content else {
        panic!("second block must remain list");
    };
    let BlockContent::Table(list_table) = &list.items[0].blocks[0].content
    else {
        panic!("list child must remain table");
    };
    assert_eq!(list_table.rows[0].cells[0].span, replacement);
    let BlockContent::Table(outer_table) = &blocks[2].content else {
        panic!("third block must remain table");
    };
    let BlockContent::Table(nested_table) =
        &outer_table.rows[0].cells[0].blocks[0].content
    else {
        panic!("table-cell child must remain table");
    };
    assert_eq!(nested_table.rows[0].cells[0].span, replacement);
    assert_eq!(outer_table.rows[0].cells[0].span, TableCellSpan::SINGLE);
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
                                    span: TableCellSpan::SINGLE,
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
    let (candidate, candidate_span) = candidate_notebook_with_span(
        &candidate_ids,
        "ephemeral accepted text",
    );
    {
        let mut first = SemanticNotebookSessionService::default();
        let AcceptanceOutcome::Accepted { mapping, revision } =
            first.accept(candidate)
        else {
            panic!("candidate must be accepted");
        };
        let target = accepted_for(&mapping, candidate_span);
        assert!(matches!(
            first.replace_text(
                revision,
                target,
                String::from("ephemeral history text"),
            ),
            TextEditOutcome::Applied { .. }
        ));
        assert!(matches!(
            first.history_availability(),
            HistoryAvailabilityOutcome::Available(HistoryAvailability {
                can_undo: true,
                ..
            })
        ));
    }

    let fresh = SemanticNotebookSessionService::default();
    assert!(fresh.current().is_none());
    assert_eq!(
        fresh.history_availability(),
        HistoryAvailabilityOutcome::NoAcceptedRevision,
    );
}

#[test]
fn bilingual_unicode_punctuation_is_preserved_exactly_across_text_edits() {
    let ids = IdentityAllocator::new();
    let original = concat!(
        "«¿Qué dijo Ana?» — “It’s piñata time” – cafe\u{301}; ",
        "emoji 👩‍🔬.",
    );
    let replacement = concat!(
        "“Hello—hola”, dijo Íñigo; «¡acción!» – café; ",
        "emoji 👩‍🔬.",
    );
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&ids, original);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("bilingual Unicode candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    let before = session.current().expect("accepted Unicode revision");
    let BlockContent::Paragraph(spans) =
        &before.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must remain a paragraph");
    };
    assert_eq!(spans[0].text, original);

    let TextEditOutcome::Applied { revision, .. } =
        session.replace_text(base, target, replacement.to_owned())
    else {
        panic!("bilingual Unicode text edit must apply");
    };
    let after = session.current().expect("edited Unicode revision");
    let BlockContent::Paragraph(spans) =
        &after.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("edited fixture must remain a paragraph");
    };
    assert_eq!(spans[0].id, target);
    assert_eq!(spans[0].text, replacement);
    assert_eq!(after.id, revision);
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
fn semantic_history_traverses_candidate_replacement_snapshots() {
    let candidate_ids = IdentityAllocator::new();
    let (first_candidate, first_span) =
        candidate_notebook_with_span(&candidate_ids, "first candidate");
    let (second_candidate, second_span) =
        candidate_notebook_with_span(&candidate_ids, "second candidate");
    let mut session = SemanticNotebookSessionService::default();

    let AcceptanceOutcome::Accepted {
        mapping: first_mapping,
        revision: first_revision,
    } = session.accept(first_candidate)
    else {
        panic!("first candidate must be accepted");
    };
    let first_span = accepted_for(&first_mapping, first_span);
    let AcceptanceOutcome::Accepted {
        mapping: second_mapping,
        revision: second_revision,
    } = session.accept(second_candidate)
    else {
        panic!("second candidate must be accepted");
    };
    let second_span = accepted_for(&second_mapping, second_span);
    assert_ne!(first_span, second_span);

    let HistoryTraversalOutcome::Traversed {
        revision: undone, ..
    } = session.traverse_history(second_revision, HistoryDirection::Undo)
    else {
        panic!("candidate replacement must be undoable");
    };
    assert_ne!(undone, first_revision);
    assert_ne!(undone, second_revision);
    let current = session.current().expect("Undo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo must restore first candidate paragraph");
    };
    assert_eq!(spans[0].id, first_span);
    assert_eq!(spans[0].text, "first candidate");

    let HistoryTraversalOutcome::Traversed {
        revision: redone, ..
    } = session.traverse_history(undone, HistoryDirection::Redo)
    else {
        panic!("candidate replacement must be redoable");
    };
    assert_ne!(redone, second_revision);
    let current = session.current().expect("Redo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Redo must restore second candidate paragraph");
    };
    assert_eq!(spans[0].id, second_span);
    assert_eq!(spans[0].text, "second candidate");
}

#[test]
fn candidate_branch_after_undo_never_reuses_abandoned_redo_identities() {
    let candidate_ids = IdentityAllocator::new();
    let (first_candidate, _) =
        candidate_notebook_with_span(&candidate_ids, "first candidate");
    let (second_candidate, _) =
        candidate_notebook_with_span(&candidate_ids, "second candidate");
    let (third_candidate, third_span) =
        candidate_notebook_with_span(&candidate_ids, "third candidate");
    let mut service = SemanticNotebookSessionService::default();

    let AcceptanceOutcome::Accepted { revision: first, .. } =
        service.accept(first_candidate)
    else {
        panic!("first candidate must be accepted");
    };
    let AcceptanceOutcome::Accepted {
        mapping: second_mapping,
        revision: second,
    } = service.accept(second_candidate)
    else {
        panic!("second candidate must be accepted");
    };
    let abandoned: BTreeSet<_> = second_mapping
        .iter()
        .map(|mapping| mapping.accepted)
        .collect();
    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        service.traverse_history(second, HistoryDirection::Undo)
    else {
        panic!("second candidate must Undo");
    };
    assert_ne!(undone, first);
    assert_eq!(
        service.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: true,
            can_undo: false,
            revision: undone,
        }),
    );

    let AcceptanceOutcome::Accepted {
        mapping: third_mapping,
        revision: branched,
    } = service.accept(third_candidate)
    else {
        panic!("third candidate must create a new branch");
    };
    assert!(
        third_mapping
            .iter()
            .all(|mapping| !abandoned.contains(&mapping.accepted))
    );
    let third_span = accepted_for(&third_mapping, third_span);
    assert!(!abandoned.contains(&third_span));
    assert_eq!(
        service.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: true,
            revision: branched,
        }),
    );
    assert_eq!(
        service.traverse_history(branched, HistoryDirection::Redo),
        HistoryTraversalOutcome::Boundary {
            direction: HistoryDirection::Redo,
            revision: branched,
        },
    );
}

#[test]
fn semantic_history_undo_redo_preserves_stable_text_identity() {
    let candidate_ids = IdentityAllocator::new();
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&candidate_ids, "original");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: false,
            revision: base,
        }),
    );

    let TextEditOutcome::Applied { revision: edited, .. } =
        session.replace_text(base, target, String::from("corrected"))
    else {
        panic!("text edit must apply");
    };
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: true,
            revision: edited,
        }),
    );

    let HistoryTraversalOutcome::Traversed {
        revision: undone, ..
    } = session.traverse_history(edited, HistoryDirection::Undo)
    else {
        panic!("Undo must traverse one transaction");
    };
    assert_ne!(undone, base);
    assert_ne!(undone, edited);
    let current = session.current().expect("Undo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo must restore paragraph state");
    };
    assert_eq!(spans[0].id, target);
    assert_eq!(spans[0].text, "original");
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: true,
            can_undo: false,
            revision: undone,
        }),
    );

    let HistoryTraversalOutcome::Traversed {
        revision: redone, ..
    } = session.traverse_history(undone, HistoryDirection::Redo)
    else {
        panic!("Redo must traverse one transaction");
    };
    assert_ne!(redone, undone);
    assert_ne!(redone, edited);
    let current = session.current().expect("Redo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Redo must restore paragraph state");
    };
    assert_eq!(spans[0].id, target);
    assert_eq!(spans[0].text, "corrected");
}

#[test]
fn semantic_history_rejects_stale_base_without_traversal() {
    let candidate_ids = IdentityAllocator::new();
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&candidate_ids, "original");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    let TextEditOutcome::Applied { revision: edited, .. } =
        session.replace_text(base, target, String::from("edited"))
    else {
        panic!("text edit must apply");
    };
    let before = session.current().expect("edited revision").clone();

    assert_eq!(
        session.traverse_history(base, HistoryDirection::Undo),
        HistoryTraversalOutcome::StaleBase {
            current: edited,
            requested: base,
        },
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn history_boundary_probe_preserves_the_opposite_available_traversal() {
    let candidate_ids = IdentityAllocator::new();
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&candidate_ids, "zero");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    let TextEditOutcome::Applied { revision: edited, .. } =
        service.replace_text(base, target, String::from("one"))
    else {
        panic!("edit must apply");
    };

    assert_eq!(
        service.traverse_history(edited, HistoryDirection::Redo),
        HistoryTraversalOutcome::Boundary {
            direction: HistoryDirection::Redo,
            revision: edited,
        },
    );
    assert_eq!(
        service.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: true,
            revision: edited,
        }),
    );
    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        service.traverse_history(edited, HistoryDirection::Undo)
    else {
        panic!("Redo boundary probe must preserve Undo");
    };

    assert_eq!(
        service.traverse_history(undone, HistoryDirection::Undo),
        HistoryTraversalOutcome::Boundary {
            direction: HistoryDirection::Undo,
            revision: undone,
        },
    );
    assert_eq!(
        service.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: true,
            can_undo: false,
            revision: undone,
        }),
    );
    let HistoryTraversalOutcome::Traversed { revision: redone, .. } =
        service.traverse_history(undone, HistoryDirection::Redo)
    else {
        panic!("Undo boundary probe must preserve Redo");
    };
    let current = service.current().expect("redone revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("history boundary fixture must remain paragraph");
    };
    assert_eq!(spans[0].id, target);
    assert_eq!(spans[0].text, "one");
    assert_eq!(current.id, redone);
}

#[test]
fn repeated_completed_history_request_is_stale_without_advancing_again() {
    let candidate_ids = IdentityAllocator::new();
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&candidate_ids, "zero");
    let mut service = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        service.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    let TextEditOutcome::Applied { revision: first, .. } =
        service.replace_text(base, target, String::from("one"))
    else {
        panic!("first edit must apply");
    };
    let TextEditOutcome::Applied { revision: second, .. } =
        service.replace_text(first, target, String::from("two"))
    else {
        panic!("second edit must apply");
    };

    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        service.traverse_history(second, HistoryDirection::Undo)
    else {
        panic!("Undo must traverse once");
    };
    assert_eq!(
        service.traverse_history(second, HistoryDirection::Undo),
        HistoryTraversalOutcome::StaleBase {
            current: undone,
            requested: second,
        },
    );
    let current = service.current().expect("single Undo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo fixture must remain paragraph");
    };
    assert_eq!(spans[0].text, "one");

    let HistoryTraversalOutcome::Traversed { revision: redone, .. } =
        service.traverse_history(undone, HistoryDirection::Redo)
    else {
        panic!("Redo must traverse once");
    };
    assert_eq!(
        service.traverse_history(undone, HistoryDirection::Redo),
        HistoryTraversalOutcome::StaleBase {
            current: redone,
            requested: undone,
        },
    );
    let current = service.current().expect("single Redo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Redo fixture must remain paragraph");
    };
    assert_eq!(spans[0].id, target);
    assert_eq!(spans[0].text, "two");
}

#[test]
fn semantic_history_new_edit_after_undo_discards_redo_branch() {
    let candidate_ids = IdentityAllocator::new();
    let (candidate, candidate_span) =
        candidate_notebook_with_span(&candidate_ids, "zero");
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let target = accepted_for(&mapping, candidate_span);
    let TextEditOutcome::Applied { revision: first, .. } =
        session.replace_text(base, target, String::from("one"))
    else {
        panic!("first edit must apply");
    };
    let TextEditOutcome::Applied { revision: second, .. } =
        session.replace_text(first, target, String::from("two"))
    else {
        panic!("second edit must apply");
    };
    let HistoryTraversalOutcome::Traversed {
        revision: undone, ..
    } = session.traverse_history(second, HistoryDirection::Undo)
    else {
        panic!("Undo must restore first edit");
    };
    let TextEditOutcome::Applied { revision: branched, .. } =
        session.replace_text(undone, target, String::from("branch"))
    else {
        panic!("branch edit must apply");
    };

    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: true,
            revision: branched,
        }),
    );
    assert_eq!(
        session.traverse_history(branched, HistoryDirection::Redo),
        HistoryTraversalOutcome::Boundary {
            direction: HistoryDirection::Redo,
            revision: branched,
        },
    );
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
                                    span: TableCellSpan::SINGLE,
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
