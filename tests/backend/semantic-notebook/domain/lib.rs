// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Regression evidence for semantic identity and notebook value invariants.
// - Must-Not:
//   - Freeze wire encodings, storage paths, layout pixels, or adapter behavior.
// - Allows:
//   - Inputs: Deterministic in-memory candidate and accepted semantic values.
//   - Outputs: Assertions over typed identity and semantic preservation.
//   - Side effects: None beyond process-local test allocation.
// - Split-When:
//   - Model families gain independently executable contract fixtures.
// - Merge-When:
//   - Semantic notebook invariants move into another direct domain harness.
// - Summary:
//   - Verifies semantic identity, value, and table-grid domain invariants.
// - Description:
//   - Exercises opaque identities, merged tables, and semantic preservation.
// - Usage:
//   - Compile directly against the semantic-notebook domain crate.
// - Defaults:
//   - Treats serialization syntax as intentionally unspecified.
//

//! Semantic notebook model regression tests.

use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroU64};
use std::ptr;
use std::slice::from_ref;

use atrament_mathematics_source as _;
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation,
    PageProfile as PhysicalPageProfile, PaperMarkAppearance, PaperMarkJoin,
    PaperMarkLayer, PaperPattern, Rect, SheetSize,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, Asset, Block, BlockContent, CandidateIdentity,
    Constraint, ConstraintKind, ExtensionData, Figure, Flow, Formula,
    FormulaMode, IdentityAllocator, InlineSpan, List, ListItem, Notebook,
    OutputProfile, Page, PaperProfile, Provenance, ProvenanceKind,
    SemanticBlockKind, SemanticIdentityDescriptor, SemanticIdentityKind,
    SemanticIdentityPathEntry, Style, Table, TableCell, TableCellSpan,
    TableGridError, TableLogicalCellPlacement, TableRow, TableRowRole,
    UnresolvedBlock, UnresolvedReason, semantic_block,
    semantic_block_identity_is_ambiguous, semantic_block_page,
    semantic_duplicate_owner_among, semantic_identity_descriptor,
    semantic_identity_kind, semantic_identity_path,
};

type FixtureResult<T> = Result<T, &'static str>;

trait FixtureChecked<T> {
    fn checked(self, reason: &'static str) -> FixtureResult<T>;
}

impl<T> FixtureChecked<T> for Option<T> {
    fn checked(self, reason: &'static str) -> FixtureResult<T> {
        self.ok_or(reason)
    }
}

impl<T, E> FixtureChecked<T> for Result<T, E> {
    fn checked(self, reason: &'static str) -> FixtureResult<T> {
        self.map_err(|_error| reason)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct GeneratedPathShape {
    callout_freeform_positions: [bool; 3],
    depth: usize,
    kinds: [bool; 4],
    list_ordered_states: [bool; 2],
    list_slots: [bool; 2],
    table_slots: [bool; 2],
}

fn grid_cell(id: u32, columns: u32, rows: u32) -> TableCell<u32> {
    assert_ne!(columns, 0, "fixture column span must be nonzero");
    assert_ne!(rows, 0, "fixture row span must be nonzero");
    TableCell {
        blocks: vec![],
        id,
        span: TableCellSpan {
            columns: NonZeroU32::new(columns).unwrap_or(NonZeroU32::MIN),
            rows: NonZeroU32::new(rows).unwrap_or(NonZeroU32::MIN),
        },
    }
}

const fn grid_row(id: u32, cells: Vec<TableCell<u32>>) -> TableRow<u32> {
    TableRow {
        cells,
        id,
        role: TableRowRole::Body,
    }
}

// Separate positional oracle for table-grid geometry. All bounds are
// checked against fixture-owned rows and a typed grid error is returned;
// malformed spans must not panic before the domain is compared.
fn grid_oracle_placements(
    table: &Table<u32>,
) -> Result<Vec<TableLogicalCellPlacement<u32>>, TableGridError<u32>> {
    let Some(first) = table.rows.first() else {
        return Ok(vec![]);
    };
    let width = first.cells.iter().try_fold(0usize, |total, cell| {
        let columns =
            usize::try_from(cell.span.columns.get()).map_err(|_overflow| {
                TableGridError::ColumnSpan { cell: cell.id }
            })?;
        total
            .checked_add(columns)
            .ok_or(TableGridError::ColumnSpan { cell: cell.id })
    })?;
    let mut occupied = vec![vec![false; width]; table.rows.len()];
    let mut placements = Vec::new();
    for (row_index, row) in table.rows.iter().enumerate() {
        let mut cursor = 0usize;
        for cell in &row.cells {
            while cursor < width
                && occupied
                    .get(row_index)
                    .and_then(|cells| cells.get(cursor))
                    .copied()
                    .unwrap_or(false)
            {
                cursor = cursor.saturating_add(1);
            }
            let columns = usize::try_from(cell.span.columns.get()).map_err(
                |_overflow| TableGridError::ColumnSpan { cell: cell.id },
            )?;
            let rows =
                usize::try_from(cell.span.rows.get()).map_err(|_overflow| {
                    TableGridError::RowSpan { cell: cell.id }
                })?;
            let Some(end_column) = cursor.checked_add(columns) else {
                return Err(TableGridError::ColumnSpan { cell: cell.id });
            };
            let current_row_slots = occupied
                .get(row_index)
                .and_then(|cells| cells.get(cursor..end_column));
            if end_column > width
                || current_row_slots
                    .is_none_or(|slots| slots.iter().any(|slot| *slot))
            {
                return Err(TableGridError::ColumnSpan { cell: cell.id });
            }
            let Some(end_row) = row_index.checked_add(rows) else {
                return Err(TableGridError::RowSpan { cell: cell.id });
            };
            let Some(row_range) = occupied.get(row_index..end_row) else {
                return Err(TableGridError::RowSpan { cell: cell.id });
            };
            if row_range.iter().any(|row_slots| {
                row_slots
                    .get(cursor..end_column)
                    .is_none_or(|slots| slots.iter().any(|slot| *slot))
            }) {
                return Err(TableGridError::ColumnSpan { cell: cell.id });
            }
            placements.push(TableLogicalCellPlacement {
                cell: cell.id,
                column_start: u64::try_from(cursor).map_err(|_overflow| {
                    TableGridError::ColumnSpan { cell: cell.id }
                })?,
                row: row.id,
                row_start: u64::try_from(row_index).map_err(|_overflow| {
                    TableGridError::RowSpan { cell: cell.id }
                })?,
                span: cell.span,
            });
            let Some(rows_to_mark) = occupied.get_mut(row_index..end_row)
            else {
                return Err(TableGridError::RowSpan { cell: cell.id });
            };
            for occupied_row in rows_to_mark {
                let Some(cells_to_mark) =
                    occupied_row.get_mut(cursor..end_column)
                else {
                    return Err(TableGridError::ColumnSpan { cell: cell.id });
                };
                for slot in cells_to_mark {
                    *slot = true;
                }
            }
            cursor = end_column;
        }
        if occupied
            .get(row_index)
            .is_none_or(|cells| cells.iter().any(|slot| !*slot))
        {
            return Err(TableGridError::RowWidth { row: row.id });
        }
    }
    Ok(placements)
}

fn grid_oracle_result(table: &Table<u32>) -> Result<(), TableGridError<u32>> {
    grid_oracle_placements(table).map(|_placements| ())
}

fn grid_oracle_column_count(
    table: &Table<u32>,
) -> Result<u64, TableGridError<u32>> {
    drop(grid_oracle_placements(table)?);
    let count = table.rows.first().map_or_else(
        || 0u64,
        |row| {
            row.cells
                .iter()
                .map(|cell| NonZeroU64::from(cell.span.columns).get())
                .sum()
        },
    );
    Ok(count)
}

fn assert_validated_table_topology_matches_oracle(table: &Table<u32>) {
    let outcome = (|| -> FixtureResult<()> {
        match table.validated_logical_topology() {
            Ok(topology) => {
                let placements = grid_oracle_placements(table)
                    .checked("oracle-valid topology")?;
                assert_eq!(
                    placements.as_slice(),
                    topology.cell_placements(),
                    "oracle cells and topology agree"
                );
                assert_eq!(
                    grid_oracle_column_count(table),
                    Ok(topology.logical_column_count()),
                    "logical columns agree with oracle",
                );
                assert!(
                    ptr::eq(topology.table(), table),
                    "topology binds source table"
                );
                for row in
                    0..u64::try_from(table.rows.len()).checked("small rows")?
                {
                    for column in 0..topology.logical_column_count() {
                        let expected = placements.iter().find(|placement| {
                            let row_end = placement.row_start.saturating_add(
                                NonZeroU64::from(placement.span.rows).get(),
                            );
                            let column_end =
                                placement.column_start.saturating_add(
                                    NonZeroU64::from(placement.span.columns)
                                        .get(),
                                );
                            placement.row_start <= row
                                && row < row_end
                                && placement.column_start <= column
                                && column < column_end
                        });
                        assert_eq!(
                            topology.cell_covering(row, column),
                            expected,
                            "cell at ({row},{column})"
                        );
                    }
                }
                assert_eq!(
                    topology.cell_covering(
                        u64::try_from(table.rows.len())
                            .checked("small rows")?,
                        0,
                    ),
                    None,
                    "outer range is empty",
                );
                assert_eq!(
                    topology.cell_covering(0, topology.logical_column_count()),
                    None,
                    "outer range is empty",
                );
            },
            Err(actual) => {
                assert_eq!(
                    grid_oracle_placements(table),
                    Err(actual),
                    "matching oracle placement failure"
                );
                assert_eq!(
                    grid_oracle_column_count(table),
                    Err(actual),
                    "matching oracle count failure"
                );
            },
        }
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "table topology oracle setup");
}

const fn next_grid_seed(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    u32::from_be_bytes({
        let [first, second, third, fourth, _, _, _, _] = seed.to_be_bytes();
        [first, second, third, fourth]
    })
}

const fn physical_page_profile() -> PhysicalPageProfile {
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

fn allocate_path_identity(
    next: &mut u32,
    targets: &mut Vec<u32>,
) -> FixtureResult<u32> {
    let identity = *next;
    *next = next
        .checked_add(1)
        .checked("small generated identity space")?;
    targets.push(identity);
    Ok(identity)
}

fn path_rule_block(
    next: &mut u32,
    targets: &mut Vec<u32>,
) -> FixtureResult<Block<u32>> {
    Ok(Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id: allocate_path_identity(next, targets)?,
        provenance: None,
        style: None,
    })
}

fn wrap_generated_table(
    block: Block<u32>,
    wrapper: u32,
    seed: &mut u64,
    next: &mut u32,
    targets: &mut Vec<u32>,
    shape: &mut GeneratedPathShape,
) -> FixtureResult<Block<u32>> {
    let table = allocate_path_identity(next, targets)?;
    let first_row = allocate_path_identity(next, targets)?;
    let second_row = allocate_path_identity(next, targets)?;
    let first_cell = allocate_path_identity(next, targets)?;
    let second_cell = allocate_path_identity(next, targets)?;
    let first_rule = path_rule_block(next, targets)?;
    let second_rule = path_rule_block(next, targets)?;
    let slot = usize::try_from(next_grid_seed(seed) & 1)
        .checked("generated slot within bounds")?;
    *shape.table_slots.get_mut(slot).checked("table slot")? = true;
    let (first_blocks, second_blocks) = if slot == 0 {
        (vec![first_rule], vec![second_rule, block])
    } else {
        (vec![first_rule, block], vec![second_rule])
    };
    Ok(Block {
        content: BlockContent::Table(Table {
            id: table,
            rows: vec![
                TableRow {
                    cells: vec![TableCell {
                        blocks: first_blocks,
                        id: first_cell,
                        span: TableCellSpan::SINGLE,
                    }],
                    id: first_row,
                    role: TableRowRole::Body,
                },
                TableRow {
                    cells: vec![TableCell {
                        blocks: second_blocks,
                        id: second_cell,
                        span: TableCellSpan::SINGLE,
                    }],
                    id: second_row,
                    role: TableRowRole::Body,
                },
            ],
        }),
        extensions: vec![],
        id: wrapper,
        provenance: None,
        style: None,
    })
}

fn wrap_generated_path(
    block: Block<u32>,
    kind: u32,
    wrapper: u32,
    seed: &mut u64,
    next: &mut u32,
    targets: &mut Vec<u32>,
    shape: &mut GeneratedPathShape,
) -> FixtureResult<Block<u32>> {
    Ok(match kind {
        0 | 1 => {
            let before = path_rule_block(next, targets)?;
            let after = path_rule_block(next, targets)?;
            let position = next_grid_seed(seed).rem_euclid(3);
            *shape
                .callout_freeform_positions
                .get_mut(
                    usize::try_from(position)
                        .checked("small child position")?,
                )
                .checked("child position")? = true;
            let children = match position {
                0 => vec![block, before, after],
                1 => vec![before, block, after],
                _ => vec![before, after, block],
            };
            Block {
                content: if kind == 0 {
                    BlockContent::Callout(children)
                } else {
                    BlockContent::Freeform(children)
                },
                extensions: vec![],
                id: wrapper,
                provenance: None,
                style: None,
            }
        },
        2 => {
            let list = allocate_path_identity(next, targets)?;
            let first_item = allocate_path_identity(next, targets)?;
            let second_item = allocate_path_identity(next, targets)?;
            let first_rule = path_rule_block(next, targets)?;
            let second_rule = path_rule_block(next, targets)?;
            let slot = usize::try_from(next_grid_seed(seed) & 1)
                .checked("generated slot within bounds")?;
            *shape.list_slots.get_mut(slot).checked("list slot")? = true;
            let (first_blocks, second_blocks) = if slot == 0 {
                (vec![first_rule], vec![second_rule, block])
            } else {
                (vec![first_rule, block], vec![second_rule])
            };
            let ordered_index = usize::try_from(next_grid_seed(seed) & 1)
                .checked("generated ordered index within bounds")?;
            *shape
                .list_ordered_states
                .get_mut(ordered_index)
                .checked("ordered list state")? = true;
            Block {
                content: BlockContent::List(List {
                    id: list,
                    items: vec![
                        ListItem {
                            blocks: first_blocks,
                            id: first_item,
                        },
                        ListItem {
                            blocks: second_blocks,
                            id: second_item,
                        },
                    ],
                    ordered: ordered_index == 0,
                }),
                extensions: vec![],
                id: wrapper,
                provenance: None,
                style: None,
            }
        },
        _ => wrap_generated_table(block, wrapper, seed, next, targets, shape)?,
    })
}

fn generated_path_notebook(
    seed: &mut u64,
) -> FixtureResult<(Notebook<u32>, Vec<u32>, GeneratedPathShape)> {
    let mut next = 1u32;
    let mut targets = Vec::new();
    let notebook = allocate_path_identity(&mut next, &mut targets)?;
    let profile = allocate_path_identity(&mut next, &mut targets)?;
    let page = allocate_path_identity(&mut next, &mut targets)?;
    let flow = allocate_path_identity(&mut next, &mut targets)?;
    let span = allocate_path_identity(&mut next, &mut targets)?;
    let leaf = allocate_path_identity(&mut next, &mut targets)?;
    let mut block = Block {
        content: BlockContent::Paragraph(vec![InlineSpan {
            id: span,
            provenance: None,
            style: None,
            text: String::from("generated path leaf"),
        }]),
        extensions: vec![],
        id: leaf,
        provenance: None,
        style: None,
    };
    let depth = (next_grid_seed(seed).rem_euclid(8)).saturating_add(1);
    let mut shape = GeneratedPathShape {
        depth: usize::try_from(depth).checked("small generated depth")?,
        ..GeneratedPathShape::default()
    };
    for _ in 0..depth {
        let wrapper = allocate_path_identity(&mut next, &mut targets)?;
        let kind = next_grid_seed(seed).rem_euclid(4);
        *shape
            .kinds
            .get_mut(usize::try_from(kind).checked("small wrapper kind")?)
            .checked("kind slot")? = true;
        block = wrap_generated_path(
            block,
            kind,
            wrapper,
            seed,
            &mut next,
            &mut targets,
            &mut shape,
        )?;
    }
    Ok((
        Notebook {
            assets: vec![],
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
                    blocks: vec![block],
                    id: flow,
                }],
                id: page,
                paper_profile: profile,
            }],
            provenance: vec![],
            styles: vec![],
        },
        targets,
        shape,
    ))
}

fn descriptor_owner_chain(
    notebook: &Notebook<u32>,
    target: u32,
) -> FixtureResult<Vec<(u32, SemanticIdentityDescriptor<u32>)>> {
    let mut chain = Vec::new();
    let mut current = target;
    loop {
        let descriptor = semantic_identity_descriptor(notebook, current)
            .checked("generated target must have descriptor")?;
        chain.push((current, descriptor));
        let Some(owner) = descriptor.owner else {
            break;
        };
        current = owner;
    }
    Ok(chain)
}

#[test]
fn accepted_candidate_and_revision_sequences_never_reuse_within_authority() {
    let outcome = (|| -> FixtureResult<()> {
        let identities = IdentityAllocator::new();
        let accepted_a =
            identities.allocate_accepted().checked("accepted id")?;
        let accepted_b =
            identities.allocate_accepted().checked("accepted id")?;
        let candidate_a =
            identities.allocate_candidate().checked("candidate id")?;
        let candidate_b =
            identities.allocate_candidate().checked("candidate id")?;
        let revision_a =
            identities.allocate_revision().checked("revision id")?;
        let revision_b =
            identities.allocate_revision().checked("revision id")?;

        assert_ne!(accepted_a, accepted_b);
        assert_ne!(candidate_a, candidate_b);
        assert_ne!(revision_a, revision_b);
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

fn cloned_identity_notebook(
    ids: [AcceptedIdentity; 6],
) -> Notebook<AcceptedIdentity> {
    let [
        notebook_id,
        page_id,
        page_profile_id,
        flow_id,
        block_id,
        span_id,
    ] = ids;
    Notebook {
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
                    content: BlockContent::Paragraph(vec![InlineSpan {
                        id: span_id,
                        provenance: None,
                        style: None,
                        text: String::from("stable semantic text"),
                    }]),
                    extensions: vec![],
                    id: block_id,
                    provenance: None,
                    style: None,
                }],
                id: flow_id,
            }],
            id: page_id,
            paper_profile: page_profile_id,
        }],
        provenance: vec![],
        styles: vec![],
    }
}

#[test]
fn cloned_semantic_state_preserves_stable_identity() {
    let outcome = (|| -> FixtureResult<()> {
        let identities = IdentityAllocator::new();
        let notebook_id =
            identities.allocate_accepted().checked("notebook id")?;
        let page_id = identities.allocate_accepted().checked("page id")?;
        let page_profile_id =
            identities.allocate_accepted().checked("page profile id")?;
        let flow_id = identities.allocate_accepted().checked("flow id")?;
        let block_id = identities.allocate_accepted().checked("block id")?;
        let span_id = identities.allocate_accepted().checked("span id")?;
        let notebook = cloned_identity_notebook([
            notebook_id,
            page_id,
            page_profile_id,
            flow_id,
            block_id,
            span_id,
        ]);

        assert_eq!(notebook.clone(), notebook);
        assert_eq!(
            semantic_identity_kind(&notebook, notebook_id),
            Some(SemanticIdentityKind::Notebook),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, page_profile_id),
            Some(SemanticIdentityKind::PageProfile),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, page_id),
            Some(SemanticIdentityKind::Page),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, flow_id),
            Some(SemanticIdentityKind::Flow),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, block_id),
            Some(SemanticIdentityKind::Block(SemanticBlockKind::Paragraph)),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, span_id),
            Some(SemanticIdentityKind::InlineSpan),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, notebook_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Notebook,
                owner: None,
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, page_profile_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::PageProfile,
                owner: Some(notebook_id),
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, page_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Page,
                owner: Some(notebook_id),
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, flow_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Flow,
                owner: Some(page_id),
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, block_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Block(SemanticBlockKind::Paragraph),
                owner: Some(flow_id),
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, span_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::InlineSpan,
                owner: Some(block_id),
            }),
        );
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn simple_inline_block_kinds_keep_span_ownership() {
    type InlineBlockConstructor = fn(Vec<InlineSpan<u32>>) -> BlockContent<u32>;
    let cases: &[(SemanticBlockKind, InlineBlockConstructor)] = &[
        (SemanticBlockKind::Citation, BlockContent::Citation),
        (SemanticBlockKind::Date, BlockContent::Date),
        (SemanticBlockKind::Footnote, BlockContent::Footnote),
        (SemanticBlockKind::Definition, BlockContent::Definition),
        (SemanticBlockKind::Heading, BlockContent::Heading),
        (SemanticBlockKind::Label, BlockContent::Label),
        (SemanticBlockKind::MarginNote, BlockContent::MarginNote),
        (SemanticBlockKind::Paragraph, BlockContent::Paragraph),
        (SemanticBlockKind::Quotation, BlockContent::Quotation),
        (SemanticBlockKind::SourceNote, BlockContent::SourceNote),
    ];

    for (kind, constructor) in cases {
        let notebook = Notebook {
            assets: vec![],
            constraints: vec![],
            extensions: vec![],
            id: 1u32,
            output_profiles: vec![],
            page_profiles: vec![PaperProfile {
                geometry: physical_page_profile(),
                id: 2,
            }],
            pages: vec![Page {
                flows: vec![Flow {
                    blocks: vec![Block {
                        content: constructor(vec![InlineSpan {
                            id: 6,
                            provenance: None,
                            style: None,
                            text: String::from("semantic inline text"),
                        }]),
                        extensions: vec![],
                        id: 5,
                        provenance: None,
                        style: None,
                    }],
                    id: 4,
                }],
                id: 3,
                paper_profile: 2,
            }],
            provenance: vec![],
            styles: vec![],
        };

        assert_eq!(
            semantic_identity_descriptor(&notebook, 5),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Block(*kind),
                owner: Some(4),
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, 6),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::InlineSpan,
                owner: Some(5),
            }),
        );
        let looked_up_block =
            semantic_block(&notebook, 5).map(|block| block.id);
        assert_eq!(looked_up_block, Some(5));
        assert_eq!(semantic_block(&notebook, 6), None);
        assert_eq!(semantic_block_page(&notebook, 5), Some(3));
        assert_eq!(semantic_block_page(&notebook, 6), None);
    }
}

#[test]
fn semantic_block_respects_root_identity_precedence() {
    let notebook = Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: 1u32,
        output_profiles: vec![],
        page_profiles: vec![],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![Block {
                    content: BlockContent::Rule,
                    extensions: vec![],
                    id: 1,
                    provenance: None,
                    style: None,
                }],
                id: 4,
            }],
            id: 3,
            paper_profile: 2,
        }],
        provenance: vec![],
        styles: vec![],
    };
    assert_eq!(
        semantic_identity_kind(&notebook, 1),
        Some(SemanticIdentityKind::Notebook),
    );
    assert_eq!(semantic_block(&notebook, 1), None);
}

#[test]
fn notebook_root_families_have_exact_two_entry_paths() {
    let outcome = (|| -> FixtureResult<()> {
        let notebook = Notebook {
            assets: vec![Asset {
                id: 2u32,
                media_type: String::from("image/png"),
            }],
            constraints: vec![Constraint {
                id: 3,
                kind: ConstraintKind::Placement,
                target: 1,
            }],
            extensions: vec![],
            id: 1,
            output_profiles: vec![OutputProfile {
                id: 4,
                name: String::from("digital"),
            }],
            page_profiles: vec![PaperProfile {
                geometry: physical_page_profile(),
                id: 5,
            }],
            pages: vec![],
            provenance: vec![Provenance {
                id: 6,
                kind: ProvenanceKind::Supplied,
                reference: Some(String::from("source")),
            }],
            styles: vec![Style {
                id: 7,
                name: String::from("body"),
            }],
        };
        let cases = [
            (2, SemanticIdentityKind::Asset),
            (3, SemanticIdentityKind::Constraint),
            (4, SemanticIdentityKind::OutputProfile),
            (5, SemanticIdentityKind::PageProfile),
            (6, SemanticIdentityKind::Provenance),
            (7, SemanticIdentityKind::Style),
        ];

        for (target, kind) in cases {
            let path = semantic_identity_path(&notebook, target)
                .checked("root-owned identity must have a semantic path")?;
            assert_eq!(path.len(), 2);
            assert_eq!(
                path.first().checked("root-owned path target")?,
                &SemanticIdentityPathEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind,
                        owner: Some(notebook.id),
                    },
                    identity: target,
                }
            );
            assert_eq!(
                path.get(1).checked("root-owned notebook owner")?,
                &SemanticIdentityPathEntry {
                    descriptor: SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::Notebook,
                        owner: None,
                    },
                    identity: notebook.id,
                }
            );
        }
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

fn nested_path_fixture() -> Notebook<u32> {
    Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: 1u32,
        output_profiles: vec![],
        page_profiles: vec![PaperProfile {
            geometry: physical_page_profile(),
            id: 2,
        }],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![
                    Block {
                        content: BlockContent::Figure(Figure {
                            asset: None,
                            caption: vec![InlineSpan {
                                id: 6,
                                provenance: None,
                                style: None,
                                text: String::from("caption"),
                            }],
                            id: 5,
                        }),
                        extensions: vec![],
                        id: 4,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::List(List {
                            id: 8,
                            items: vec![ListItem {
                                blocks: vec![Block {
                                    content: BlockContent::Paragraph(vec![
                                        InlineSpan {
                                            id: 11,
                                            provenance: None,
                                            style: None,
                                            text: String::from("list text"),
                                        },
                                    ]),
                                    extensions: vec![],
                                    id: 10,
                                    provenance: None,
                                    style: None,
                                }],
                                id: 9,
                            }],
                            ordered: true,
                        }),
                        extensions: vec![],
                        id: 7,
                        provenance: None,
                        style: None,
                    },
                    Block {
                        content: BlockContent::Table(Table {
                            id: 13,
                            rows: vec![TableRow {
                                cells: vec![TableCell {
                                    blocks: vec![Block {
                                        content: BlockContent::Rule,
                                        extensions: vec![],
                                        id: 16,
                                        provenance: None,
                                        style: None,
                                    }],
                                    id: 15,
                                    span: TableCellSpan::SINGLE,
                                }],
                                id: 14,
                                role: TableRowRole::Body,
                            }],
                        }),
                        extensions: vec![],
                        id: 12,
                        provenance: None,
                        style: None,
                    },
                ],
                id: 3,
            }],
            id: 20,
            paper_profile: 2,
        }],
        provenance: vec![],
        styles: vec![],
    }
}

#[test]
fn semantic_identity_path_matches_descriptor_chain_across_nested_families() {
    let outcome = (|| -> FixtureResult<()> {
        let notebook = nested_path_fixture();

        for target in
            [1, 2, 20, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        {
            let path = semantic_identity_path(&notebook, target)
                .checked("fixture identity must have a semantic path")?;
            let first = path.first().checked("path contains target")?;
            assert_eq!(first.identity, target);
            assert_eq!(
                first.descriptor,
                semantic_identity_descriptor(&notebook, target)
                    .checked("fixture identity must have a descriptor")?,
            );
            for adjacent in path.windows(2) {
                let current =
                    adjacent.first().checked("path window current")?;
                let owner = adjacent.get(1).checked("path window owner")?;
                assert_eq!(current.descriptor.owner, Some(owner.identity));
            }
            let root = path.last().checked("path reaches notebook root")?;
            assert_eq!(root.identity, notebook.id);
            assert_eq!(root.descriptor.kind, SemanticIdentityKind::Notebook);
            assert_eq!(root.descriptor.owner, None);
        }
        assert_eq!(semantic_identity_path(&notebook, u32::MAX), None);
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

fn verify_generated_tree_targets(
    case: u32,
    notebook: &Notebook<u32>,
    targets: Vec<u32>,
) {
    let outcome = (|| -> FixtureResult<()> {
        for target in targets {
            let path = semantic_identity_path(notebook, target)
                .checked("generated target must have semantic path")?;
            let actual = path
                .iter()
                .map(|entry| (entry.identity, entry.descriptor))
                .collect::<Vec<_>>();
            assert_eq!(
                actual,
                descriptor_owner_chain(notebook, target)?,
                "generated path mismatch in case {case} target {target}",
            );
            let expected_page = matches!(
                path.first().map(|entry| entry.descriptor.kind),
                Some(SemanticIdentityKind::Block(_)),
            )
            .then(|| {
                path.iter().find(|entry| {
                    entry.descriptor.kind == SemanticIdentityKind::Page
                })
            })
            .map(|entry| {
                entry.checked("generated block path must reach a page")
            })
            .transpose()?
            .map(|entry| entry.identity);
            let expected_block = matches!(
                path.first()
                    .checked("generated first entry")?
                    .descriptor
                    .kind,
                SemanticIdentityKind::Block(_),
            )
            .then_some(target);
            assert_eq!(
                semantic_block(notebook, target).map(|block| block.id),
                expected_block,
                "block lookup mismatch: {case}/{target}",
            );
            assert_eq!(
                semantic_block_page(notebook, target),
                expected_page,
                "block/page mismatch: {case}/{target}",
            );
        }
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "generated target oracle");
}

#[test]
fn logical_table_validator_matches_naive_occupancy_oracle() {
    let outcome = (|| -> FixtureResult<()> {
        let mut seed = 0x5eed_1234_9876_abcd;
        let mut seen_row_counts = [false; 4];
        let mut seen_cell_counts = [false; 4];
        let mut seen_column_spans = [false; 3];
        let mut seen_row_spans = [false; 3];
        let mut saw_valid = false;
        let mut saw_column_span = false;
        let mut saw_row_span = false;
        let mut saw_row_width = false;
        for case in 0..20_000u32 {
            let row_count =
                usize::try_from((next_grid_seed(&mut seed).rem_euclid(4)) + 1)
                    .checked("small row count")?;
            *seen_row_counts
                .get_mut(row_count.saturating_sub(1))
                .checked("generated row count slot")? = true;
            let mut rows = Vec::with_capacity(row_count);
            let mut identity = case.saturating_mul(100).saturating_add(1);
            for _ in 0..row_count {
                let cell_count =
                    usize::try_from(next_grid_seed(&mut seed).rem_euclid(4))
                        .checked("small cell count")?;
                *seen_cell_counts
                    .get_mut(cell_count)
                    .checked("generated cell count slot")? = true;
                let mut cells = Vec::with_capacity(cell_count);
                for _ in 0..cell_count {
                    let columns = (next_grid_seed(&mut seed).rem_euclid(3)) + 1;
                    let span_rows =
                        (next_grid_seed(&mut seed).rem_euclid(3)) + 1;
                    *seen_column_spans
                        .get_mut(
                            usize::try_from(columns.saturating_sub(1))
                                .checked("column index fits")?,
                        )
                        .checked("column span slot")? = true;
                    *seen_row_spans
                        .get_mut(
                            usize::try_from(span_rows.saturating_sub(1))
                                .checked("row index fits")?,
                        )
                        .checked("row span slot")? = true;
                    cells.push(grid_cell(identity, columns, span_rows));
                    identity = identity.saturating_add(1);
                }
                rows.push(TableRow {
                    cells,
                    id: identity,
                    role: TableRowRole::Body,
                });
                identity = identity.saturating_add(1);
            }
            let table = Table { id: identity, rows };
            let expected = grid_oracle_result(&table);
            match expected {
                Ok(()) => saw_valid = true,
                Err(TableGridError::ColumnSpan { .. }) => {
                    saw_column_span = true;
                },
                Err(TableGridError::RowSpan { .. }) => saw_row_span = true,
                Err(TableGridError::RowWidth { .. }) => saw_row_width = true,
            }
            assert_eq!(
                table.validate_grid(),
                expected,
                "typed occupancy oracle mismatch in generated case {case}",
            );
            assert_eq!(
                table.logical_cell_placements(),
                grid_oracle_placements(&table),
                "logical placement oracle mismatch in generated case {case}",
            );
            assert_eq!(
                table.logical_column_count(),
                grid_oracle_column_count(&table),
                "logical column-count oracle mismatch in generated case {case}",
            );
            assert_validated_table_topology_matches_oracle(&table);
        }
        assert!(seen_row_counts.into_iter().all(|seen| seen));
        assert!(seen_cell_counts.into_iter().all(|seen| seen));
        assert!(seen_column_spans.into_iter().all(|seen| seen));
        assert!(seen_row_spans.into_iter().all(|seen| seen));
        assert!(saw_valid, "valid table fixture reached");
        assert!(saw_column_span, "column-span fixture reached");
        assert!(saw_row_span, "row-span fixture reached");
        assert!(saw_row_width, "row-width fixture reached");
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

fn compact_grid_row_shapes() -> Vec<Vec<(u32, u32)>> {
    let cells = [(1, 1), (1, 2), (2, 1), (2, 2)];
    let mut shapes = vec![vec![]];
    for first in cells {
        shapes.push(vec![first]);
    }
    for first in cells {
        for second in cells {
            shapes.push(vec![first, second]);
        }
    }
    shapes
}

#[test]
fn semantic_identity_path_matches_descriptor_walk_on_generated_trees() {
    let outcome = (|| -> FixtureResult<()> {
        let mut seed = 0x9e37_79b9_7f4a_7c15;
        let mut seen_depths = [false; 8];
        let mut seen_kinds = [false; 4];
        let mut seen_callout_freeform_positions = [false; 3];
        let mut seen_list_slots = [false; 2];
        let mut seen_list_ordered_states = [false; 2];
        let mut seen_table_slots = [false; 2];
        for case in 0..5_000u32 {
            let (notebook, targets, shape) =
                generated_path_notebook(&mut seed)?;
            *seen_depths
                .get_mut(shape.depth.saturating_sub(1))
                .checked("generated depth slot")? = true;
            for (seen, present) in seen_kinds.iter_mut().zip(shape.kinds) {
                *seen |= present;
            }
            for (seen, present) in seen_callout_freeform_positions
                .iter_mut()
                .zip(shape.callout_freeform_positions)
            {
                *seen |= present;
            }
            for (seen, present) in
                seen_list_slots.iter_mut().zip(shape.list_slots)
            {
                *seen |= present;
            }
            for (seen, present) in seen_list_ordered_states
                .iter_mut()
                .zip(shape.list_ordered_states)
            {
                *seen |= present;
            }
            for (seen, present) in
                seen_table_slots.iter_mut().zip(shape.table_slots)
            {
                *seen |= present;
            }
            verify_generated_tree_targets(case, &notebook, targets);
        }
        assert!(seen_depths.into_iter().all(|seen| seen), "all depths occur");
        assert!(seen_kinds.into_iter().all(|seen| seen), "all kinds occur");
        assert!(
            seen_callout_freeform_positions.into_iter().all(|seen| seen),
            "all positions occur"
        );
        assert!(
            seen_list_slots.into_iter().all(|seen| seen),
            "all list slots occur"
        );
        assert!(
            seen_list_ordered_states.into_iter().all(|seen| seen),
            "all ordered states occur"
        );
        assert!(
            seen_table_slots.into_iter().all(|seen| seen),
            "all table slots occur"
        );
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "generated tree fixture");
}

#[test]
fn compact_table_grids_exhaustively_match_naive_occupancy_oracle() {
    let outcome = (|| -> FixtureResult<()> {
        const EXPECTED_CASES: usize = 9_724;
        let shapes = compact_grid_row_shapes();
        let mut cases = 0usize;
        for row_count in 0..=3usize {
            let combinations = shapes.len().pow(
                u32::try_from(row_count)
                    .checked("small exhaustive row count")?,
            );
            for mut combination in 0..combinations {
                let mut next_identity = 1u32;
                let mut rows = Vec::with_capacity(row_count);
                for _ in 0..row_count {
                    let shape = shapes
                        .get(combination.rem_euclid(shapes.len()))
                        .checked("exhaustive shape index")?;
                    combination = combination.div_euclid(shapes.len());
                    let mut cells = Vec::with_capacity(shape.len());
                    for &(columns, span_rows) in shape {
                        cells.push(grid_cell(
                            next_identity,
                            columns,
                            span_rows,
                        ));
                        next_identity = next_identity.saturating_add(1);
                    }
                    rows.push(grid_row(next_identity, cells));
                    next_identity = next_identity.saturating_add(1);
                }
                let table = Table { id: next_identity, rows };
                assert_eq!(
                    table.validate_grid(),
                    grid_oracle_result(&table),
                    "compact table grid mismatch in exhaustive case {cases}",
                );
                assert_eq!(
                    table.logical_cell_placements(),
                    grid_oracle_placements(&table),
                    "compact placement mismatch in case {cases}",
                );
                assert_eq!(
                    table.logical_column_count(),
                    grid_oracle_column_count(&table),
                    "compact table column count mismatch in case {cases}",
                );
                assert_validated_table_topology_matches_oracle(&table);
                cases = cases.saturating_add(1);
            }
        }
        assert_eq!(cases, EXPECTED_CASES);
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn maximum_logical_colspan_stays_compact() {
    let outcome = (|| -> FixtureResult<()> {
        let maximum = NonZeroU32::MAX.get();
        let table = Table {
            id: 100u32,
            rows: vec![grid_row(1, vec![grid_cell(10, maximum, 1)])],
        };

        assert_eq!(table.validate_grid(), Ok(()));
        assert_eq!(table.logical_column_count(), Ok(u64::from(maximum)));
        let topology = table
            .validated_logical_topology()
            .checked("maximum span topology")?;
        assert_eq!(
            topology
                .cell_covering(0, u64::from(maximum).saturating_sub(1))
                .map(|cell| cell.cell),
            Some(10),
        );
        assert_eq!(topology.cell_covering(0, u64::from(maximum)), None);
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn wide_interleaved_row_spans_preserve_compact_grid_placements() {
    let outcome = (|| -> FixtureResult<()> {
        // Each second-row cell follows a different inherited row span. Walking
        // every active span per new cell would make this fixture quadratic.
        const PAIRS: u32 = 8_192;
        let mut first = Vec::new();
        let mut second = Vec::new();
        for index in 0..PAIRS {
            first.push(grid_cell(index * 2 + 1, 1, 2));
            first.push(grid_cell(index * 2 + 2, 1, 1));
            second.push(grid_cell(PAIRS * 2 + index + 1, 1, 1));
        }
        let table = Table {
            id: 1,
            rows: vec![grid_row(1, first), grid_row(2, second)],
        };
        assert_eq!(table.logical_column_count(), Ok(u64::from(PAIRS) * 2));
        let topology = table
            .validated_logical_topology()
            .checked("interleaved valid table")?;
        assert_eq!(
            topology.cell_placements().len(),
            usize::try_from(PAIRS)
                .checked("pair count fits")?
                .saturating_mul(3)
        );
        for index in [0, 1, PAIRS.div_euclid(2), PAIRS - 1] {
            let start = u64::from(index) * 2;
            assert_eq!(
                topology.cell_covering(1, start).map(|cell| cell.cell),
                Some(index * 2 + 1),
            );
            assert_eq!(
                topology.cell_covering(1, start + 1).map(|cell| cell.cell),
                Some(PAIRS * 2 + index + 1),
            );
        }
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn tall_fully_inherited_grid_remains_compact_and_ordered() {
    let outcome = (|| -> FixtureResult<()> {
        // A large number of rows inherit the identical wide set of active
        // spans. Sorting these unchanged spans on every row is
        // unnecessary work.
        const COLUMNS: u32 = 4_096;
        const ROWS: u32 = 512;
        let first_row = (0..COLUMNS)
            .map(|column| grid_cell(column + 1, 1, ROWS))
            .collect();
        let mut rows = Vec::new();
        rows.push(grid_row(1, first_row));
        for index in 1..ROWS {
            rows.push(grid_row(index + 1, Vec::new()));
        }
        let table = Table { id: 1u32, rows };
        let topology = table
            .validated_logical_topology()
            .checked("fully inherited tall table")?;
        assert_eq!(topology.logical_column_count(), u64::from(COLUMNS));
        assert_eq!(
            topology.cell_placements().len(),
            usize::try_from(COLUMNS).checked("bounded column count")?,
        );
        for row in [0, 1, ROWS.div_euclid(2), ROWS - 1] {
            for column in [0, 1, COLUMNS.div_euclid(2), COLUMNS - 1] {
                assert_eq!(
                    topology
                        .cell_covering(u64::from(row), u64::from(column))
                        .map(|cell| cell.cell),
                    Some(column + 1),
                    "inherited cell must retain its logical owner",
                );
            }
        }
        assert_eq!(topology.cell_covering(u64::from(ROWS), 0), None);
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn adjacent_inherited_spans_skip_entire_occupied_run() {
    let outcome = (|| -> FixtureResult<()> {
        let table = Table {
            id: 1u32,
            rows: vec![
                grid_row(1, vec![
                    grid_cell(10, 1, 2),
                    grid_cell(11, 1, 2),
                    grid_cell(12, 1, 2),
                    grid_cell(13, 1, 1),
                ]),
                grid_row(2, vec![grid_cell(20, 1, 1)]),
            ],
        };
        assert_eq!(table.validate_grid(), Ok(()));
        let topology = table
            .validated_logical_topology()
            .checked("adjacent inherited spans")?;
        assert_eq!(
            topology.cell_covering(1, 2).map(|cell| cell.cell),
            Some(12)
        );
        assert_eq!(
            topology.cell_covering(1, 3).map(|cell| cell.cell),
            Some(20)
        );
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn logical_cell_placements_reuse_validated_merged_grid_topology() {
    let outcome = (|| -> FixtureResult<()> {
        let table = Table {
            id: 100u32,
            rows: vec![
                grid_row(1, vec![grid_cell(10, 2, 2), grid_cell(11, 1, 1)]),
                grid_row(2, vec![grid_cell(20, 1, 1)]),
                grid_row(3, vec![grid_cell(30, 1, 1), grid_cell(31, 2, 1)]),
            ],
        };
        let span_two_by_two = TableCellSpan {
            columns: NonZeroU32::new(2).checked("nonzero columns")?,
            rows: NonZeroU32::new(2).checked("nonzero rows")?,
        };
        let span_two_by_one = TableCellSpan {
            columns: NonZeroU32::new(2).checked("nonzero columns")?,
            rows: NonZeroU32::MIN,
        };

        assert_eq!(
            table.logical_cell_placements(),
            Ok(vec![
                TableLogicalCellPlacement {
                    cell: 10,
                    column_start: 0,
                    row: 1,
                    row_start: 0,
                    span: span_two_by_two,
                },
                TableLogicalCellPlacement {
                    cell: 11,
                    column_start: 2,
                    row: 1,
                    row_start: 0,
                    span: TableCellSpan::SINGLE,
                },
                TableLogicalCellPlacement {
                    cell: 20,
                    column_start: 2,
                    row: 2,
                    row_start: 1,
                    span: TableCellSpan::SINGLE,
                },
                TableLogicalCellPlacement {
                    cell: 30,
                    column_start: 0,
                    row: 3,
                    row_start: 2,
                    span: TableCellSpan::SINGLE,
                },
                TableLogicalCellPlacement {
                    cell: 31,
                    column_start: 1,
                    row: 3,
                    row_start: 2,
                    span: span_two_by_one,
                },
            ]),
        );
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn validated_logical_topology_binds_exact_table_and_complete_projection() {
    let outcome = (|| -> FixtureResult<()> {
        let table = Table {
            id: 100u32,
            rows: vec![
                grid_row(1, vec![grid_cell(10, 2, 2), grid_cell(11, 1, 1)]),
                grid_row(2, vec![grid_cell(20, 1, 1)]),
                grid_row(3, vec![grid_cell(30, 3, 1)]),
            ],
        };
        let topology = table
            .validated_logical_topology()
            .checked("valid topology evidence")?;

        assert!(
            ptr::eq(topology.table(), &raw const table),
            "topology binds source table"
        );
        assert_eq!(topology.logical_column_count(), 3);
        assert_eq!(
            topology.cell_covering(0, 0).map(|cell| cell.cell),
            Some(10)
        );
        assert_eq!(
            topology.cell_covering(1, 1).map(|cell| cell.cell),
            Some(10)
        );
        assert_eq!(
            topology.cell_covering(1, 2).map(|cell| cell.cell),
            Some(20)
        );
        assert_eq!(
            topology.cell_covering(2, 2).map(|cell| cell.cell),
            Some(30)
        );
        assert_eq!(topology.cell_covering(3, 0), None);
        assert_eq!(topology.cell_covering(0, 3), None);
        assert_eq!(
            topology.cell_placements(),
            table
                .logical_cell_placements()
                .checked("standalone placements")?
                .as_slice(),
        );
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn logical_cell_placements_return_no_partial_projection_for_invalid_grid() {
    let table = Table {
        id: 101u32,
        rows: vec![
            grid_row(1, vec![grid_cell(10, 1, 1), grid_cell(11, 1, 1)]),
            grid_row(2, vec![grid_cell(20, 1, 1)]),
        ],
    };

    assert_eq!(
        table.logical_cell_placements(),
        Err(TableGridError::RowWidth { row: 2 }),
    );
    assert_eq!(
        table.logical_column_count(),
        Err(TableGridError::RowWidth { row: 2 }),
    );
}

#[test]
fn merged_table_grid_validation_is_identity_generic() {
    let table = Table {
        id: 100u32,
        rows: vec![
            grid_row(1, vec![grid_cell(10, 2, 2), grid_cell(11, 1, 1)]),
            grid_row(2, vec![grid_cell(20, 1, 1)]),
            grid_row(3, vec![grid_cell(30, 1, 1), grid_cell(31, 2, 1)]),
        ],
    };

    assert_eq!(table.validate_grid(), Ok(()));
}

#[test]
fn staggered_row_spans_expire_and_fill_gaps_deterministically() {
    let table = Table {
        id: 200u32,
        rows: vec![
            grid_row(1, vec![
                grid_cell(10, 1, 3),
                grid_cell(11, 1, 1),
                grid_cell(12, 2, 2),
            ]),
            grid_row(2, vec![grid_cell(20, 1, 1)]),
            grid_row(3, vec![grid_cell(30, 3, 1)]),
        ],
    };

    assert_eq!(table.validate_grid(), Ok(()));
}

#[test]
fn table_grid_validation_reports_semantic_owner_of_each_failure() {
    let row_span = Table {
        id: 100u32,
        rows: vec![grid_row(1, vec![grid_cell(10, 1, 2)])],
    };
    assert_eq!(
        row_span.validate_grid(),
        Err(TableGridError::RowSpan { cell: 10 }),
    );

    let row_width = Table {
        id: 101u32,
        rows: vec![
            grid_row(2, vec![grid_cell(20, 1, 1), grid_cell(21, 1, 1)]),
            grid_row(3, vec![grid_cell(30, 1, 1)]),
        ],
    };
    assert_eq!(
        row_width.validate_grid(),
        Err(TableGridError::RowWidth { row: 3 }),
    );

    let column_span = Table {
        id: 102u32,
        rows: vec![
            grid_row(4, vec![grid_cell(40, 1, 2), grid_cell(41, 1, 1)]),
            grid_row(5, vec![grid_cell(50, 2, 1)]),
        ],
    };
    assert_eq!(
        column_span.validate_grid(),
        Err(TableGridError::ColumnSpan { cell: 50 }),
    );
}

fn nested_table_identity_notebook(
    ids: [CandidateIdentity; 9],
) -> Notebook<CandidateIdentity> {
    let [
        notebook_id,
        profile_id,
        page_id,
        flow_id,
        block_id,
        table_id,
        row_id,
        cell_id,
        child_id,
    ] = ids;
    Notebook {
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
                blocks: vec![Block {
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
                            role: TableRowRole::Header,
                        }],
                    }),
                    extensions: vec![],
                    id: block_id,
                    provenance: None,
                    style: None,
                }],
                id: flow_id,
            }],
            id: page_id,
            paper_profile: profile_id,
        }],
        provenance: vec![],
        styles: vec![],
    }
}

#[test]
fn semantic_identity_kind_reaches_nested_table_owners() {
    let outcome = (|| -> FixtureResult<()> {
        let identities = IdentityAllocator::new();
        let notebook_id =
            identities.allocate_candidate().checked("notebook id")?;
        let profile_id =
            identities.allocate_candidate().checked("profile id")?;
        let page_id = identities.allocate_candidate().checked("page id")?;
        let flow_id = identities.allocate_candidate().checked("flow id")?;
        let block_id = identities.allocate_candidate().checked("block id")?;
        let table_id = identities.allocate_candidate().checked("table id")?;
        let row_id = identities.allocate_candidate().checked("row id")?;
        let cell_id = identities.allocate_candidate().checked("cell id")?;
        let child_id = identities.allocate_candidate().checked("child id")?;
        let missing = identities.allocate_candidate().checked("missing id")?;
        let notebook = nested_table_identity_notebook([
            notebook_id,
            profile_id,
            page_id,
            flow_id,
            block_id,
            table_id,
            row_id,
            cell_id,
            child_id,
        ]);
        assert_eq!(
            semantic_identity_kind(&notebook, block_id),
            Some(SemanticIdentityKind::Block(SemanticBlockKind::Table)),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, table_id),
            Some(SemanticIdentityKind::Table),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, row_id),
            Some(SemanticIdentityKind::TableRow),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, cell_id),
            Some(SemanticIdentityKind::TableCell),
        );
        assert_eq!(
            semantic_identity_kind(&notebook, child_id),
            Some(SemanticIdentityKind::Block(SemanticBlockKind::Rule)),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, table_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Table,
                owner: Some(block_id),
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, row_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::TableRow,
                owner: Some(table_id),
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, cell_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::TableCell,
                owner: Some(row_id),
            }),
        );
        assert_eq!(
            semantic_identity_descriptor(&notebook, child_id),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Block(SemanticBlockKind::Rule),
                owner: Some(cell_id),
            }),
        );
        assert_eq!(semantic_identity_kind(&notebook, missing), None);
        assert_eq!(semantic_identity_descriptor(&notebook, missing), None);
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

// Unlink nested callouts from the leaves upward so the 50k-depth fixture
// can be deallocated without a recursive destructor or leaked allocations.
fn drop_deep_notebook_iteratively(mut notebook: Notebook<u64>) {
    let mut pending = Vec::new();
    for page in &mut notebook.pages {
        for flow in &mut page.flows {
            pending.append(&mut flow.blocks);
        }
    }
    while let Some(mut block) = pending.pop() {
        if let BlockContent::Callout(children) = &mut block.content {
            pending.append(children);
        }
    }
}

#[test]
fn semantic_identity_descriptor_handles_deep_nesting_iteratively() {
    let outcome = (|| -> FixtureResult<()> {
        const DEPTH: u64 = 50_000;
        const TARGET: u64 = u64::MAX;
        let mut block = Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: TARGET,
            provenance: None,
            style: None,
        };
        for offset in 0..DEPTH {
            block = Block {
                content: BlockContent::Callout(vec![block]),
                extensions: vec![],
                id: offset.saturating_add(1_000),
                provenance: None,
                style: None,
            };
        }
        let notebook = Notebook {
            assets: vec![],
            constraints: vec![],
            extensions: vec![],
            id: 1,
            output_profiles: vec![],
            page_profiles: vec![],
            pages: vec![Page {
                flows: vec![Flow {
                    blocks: vec![block],
                    id: 3,
                }],
                id: 2,
                paper_profile: 4,
            }],
            provenance: vec![],
            styles: vec![],
        };
        assert_eq!(
            semantic_identity_descriptor(&notebook, TARGET),
            Some(SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Block(SemanticBlockKind::Rule),
                owner: Some(1_000),
            }),
        );
        assert_eq!(
            semantic_block(&notebook, TARGET).map(|found| found.id),
            Some(TARGET),
        );
        let path = semantic_identity_path(&notebook, TARGET)
            .checked("deep target must have a structural path")?;
        assert_eq!(
            path.len(),
            usize::try_from(DEPTH).checked("depth fits")? + 4
        );
        assert_eq!(path.first().map(|entry| entry.identity), Some(TARGET));
        assert_eq!(path.last().map(|entry| entry.identity), Some(notebook.id));
        drop_deep_notebook_iteratively(notebook);
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

#[test]
fn unresolved_semantics_and_extensions_are_preserved_exactly() {
    let outcome = (|| -> FixtureResult<()> {
        let identities = IdentityAllocator::new();
        let block_id = identities.allocate_candidate().checked("block id")?;
        let extension = ExtensionData {
            namespace: String::from("example.future/7"),
            payload: vec![0, 1, 2, 255],
        };
        let block = Block {
            content: BlockContent::Unresolved(UnresolvedBlock {
                extensions: vec![extension.clone()],
                reason: UnresolvedReason::Unsupported,
                source: String::from("future semantic object"),
            }),
            extensions: vec![extension.clone()],
            id: block_id,
            provenance: None,
            style: None,
        };

        let BlockContent::Unresolved(unresolved) = &block.content else {
            return Err("fixture must remain unresolved");
        };
        assert_eq!(unresolved.source, "future semantic object");
        assert_eq!(unresolved.extensions.as_slice(), from_ref(&extension),);
        assert_eq!(block.extensions, [extension]);
        Ok(())
    })();
    assert_eq!(outcome, Ok(()), "fixture setup");
}

fn ambiguity_fixture_block_mut(
    notebook: &mut Notebook<u32>,
    page: usize,
    block: usize,
) -> Option<&mut Block<u32>> {
    notebook
        .pages
        .get_mut(page)?
        .flows
        .first_mut()?
        .blocks
        .get_mut(block)
}

fn verify_block_ambiguity_nested_and_cross_page() -> Result<(), &'static str> {
    let rule = |id| Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id,
        provenance: None,
        style: None,
    };
    let mut notebook = Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: 1u32,
        output_profiles: vec![],
        page_profiles: vec![],
        pages: vec![
            Page {
                flows: vec![Flow {
                    blocks: vec![rule(5), rule(6)],
                    id: 4,
                }],
                id: 3,
                paper_profile: 2,
            },
            Page {
                flows: vec![Flow {
                    blocks: vec![rule(9)],
                    id: 8,
                }],
                id: 7,
                paper_profile: 2,
            },
        ],
        provenance: vec![],
        styles: vec![],
    };
    if semantic_block_identity_is_ambiguous(&notebook, 5)
        || semantic_block_identity_is_ambiguous(&notebook, 99)
        || semantic_block_page(&notebook, 5) != Some(3)
    {
        return Err("valid block was ambiguous or missing");
    }

    ambiguity_fixture_block_mut(&mut notebook, 1, 0)
        .ok_or("second page block missing")?
        .id = 5;
    if !semantic_block_identity_is_ambiguous(&notebook, 5) {
        return Err("duplicate block was not reported");
    }
    ambiguity_fixture_block_mut(&mut notebook, 1, 0)
        .ok_or("second page block missing")?
        .id = 9;

    ambiguity_fixture_block_mut(&mut notebook, 0, 1)
        .ok_or("sibling block missing")?
        .id = 5;
    if !semantic_block_identity_is_ambiguous(&notebook, 5) {
        return Err("duplicate block was not reported");
    }
    let sibling = ambiguity_fixture_block_mut(&mut notebook, 0, 1)
        .ok_or("sibling block missing")?;
    sibling.id = 6;
    sibling.content = BlockContent::Callout(vec![rule(5)]);
    if !semantic_block_identity_is_ambiguous(&notebook, 5) {
        return Err("duplicate block was not reported");
    }
    if semantic_block_identity_is_ambiguous(&notebook, 6) {
        return Err("distinct callout owner became ambiguous");
    }
    Ok(())
}

#[test]
fn semantic_block_ambiguity_checks_nested_and_cross_page_owners() {
    assert_eq!(verify_block_ambiguity_nested_and_cross_page(), Ok(()));
}

fn verify_selected_owner_cross_kind() -> Result<(), &'static str> {
    let rule = |id| Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id,
        provenance: None,
        style: None,
    };
    let mut notebook = Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: 1u32,
        output_profiles: vec![],
        page_profiles: vec![],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![rule(5), rule(6)],
                id: 4,
            }],
            id: 3,
            paper_profile: 2,
        }],
        provenance: vec![],
        styles: vec![],
    };
    let selected = BTreeSet::from([5u32]);
    ambiguity_fixture_block_mut(&mut notebook, 0, 0)
        .ok_or("first block missing")?
        .style = Some(5);
    if semantic_duplicate_owner_among(&notebook, &selected).is_some() {
        return Err("reference incorrectly counted as semantic owner");
    }
    notebook
        .pages
        .first_mut()
        .ok_or("page missing")?
        .flows
        .push(Flow { id: 5, blocks: vec![] });
    if semantic_duplicate_owner_among(&notebook, &selected) != Some(5) {
        return Err("later flow alias was missed");
    }
    let page = notebook.pages.first_mut().ok_or("page missing")?;
    let _removed = page.flows.pop();
    for content in [
        BlockContent::Paragraph(vec![InlineSpan {
            id: 5,
            provenance: None,
            style: None,
            text: String::from("nested span"),
        }]),
        BlockContent::Figure(Figure {
            asset: None,
            caption: vec![],
            id: 5,
        }),
        BlockContent::Mathematics(Formula {
            id: 5,
            mode: FormulaMode::Display,
            source: String::from("x"),
        }),
    ] {
        ambiguity_fixture_block_mut(&mut notebook, 0, 1)
            .ok_or("sibling block missing")?
            .content = content;
        if semantic_duplicate_owner_among(&notebook, &selected) != Some(5) {
            return Err("nested leaf owner alias was missed");
        }
    }
    Ok(())
}

#[test]
fn selected_semantic_owner_collision_checks_non_block_kinds() {
    assert_eq!(verify_selected_owner_cross_kind(), Ok(()));
}
