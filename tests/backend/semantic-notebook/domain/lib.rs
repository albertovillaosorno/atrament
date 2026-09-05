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
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation,
    PageProfile as PhysicalPageProfile, PaperMarkAppearance, PaperMarkJoin,
    PaperMarkLayer, PaperPattern, Rect, SheetSize,
};
use std::num::NonZeroU32;

use atrament_semantic_notebook::{
    Asset, Block, BlockContent, Constraint, ConstraintKind, ExtensionData,
    Figure, Flow, IdentityAllocator, InlineSpan, List, ListItem, Notebook,
    OutputProfile, Page, PaperProfile, Provenance, ProvenanceKind,
    SemanticBlockKind, SemanticIdentityDescriptor, SemanticIdentityKind,
    SemanticIdentityPathEntry, Style,
    Table, TableCell, TableCellSpan, TableGridError, TableRow, TableRowRole,
    UnresolvedBlock, UnresolvedReason, semantic_identity_descriptor,
    semantic_identity_kind, semantic_identity_path,
};

fn grid_cell(
    id: u32,
    columns: u32,
    rows: u32,
) -> TableCell<u32> {
    let Some(columns) = NonZeroU32::new(columns) else {
        panic!("fixture column span must be nonzero");
    };
    let Some(rows) = NonZeroU32::new(rows) else {
        panic!("fixture row span must be nonzero");
    };
    TableCell {
        blocks: vec![],
        id,
        span: TableCellSpan { columns, rows },
    }
}

fn grid_row(id: u32, cells: Vec<TableCell<u32>>) -> TableRow<u32> {
    TableRow {
        cells,
        id,
        role: TableRowRole::Body,
    }
}

fn grid_oracle_result(
    table: &Table<u32>,
) -> Result<(), TableGridError<u32>> {
    let Some(first) = table.rows.first() else {
        return Ok(());
    };
    let width: usize = first
        .cells
        .iter()
        .map(|cell| {
            usize::try_from(cell.span.columns.get()).expect("small width")
        })
        .sum();
    let mut occupied = vec![vec![false; width]; table.rows.len()];
    for (row_index, row) in table.rows.iter().enumerate() {
        let mut cursor = 0usize;
        for cell in &row.cells {
            while cursor < width && occupied[row_index][cursor] {
                cursor = cursor.saturating_add(1);
            }
            let columns = usize::try_from(cell.span.columns.get())
                .expect("small columns");
            let rows =
                usize::try_from(cell.span.rows.get()).expect("small rows");
            let Some(end_column) = cursor.checked_add(columns) else {
                return Err(TableGridError::ColumnSpan { cell: cell.id });
            };
            if end_column > width
                || occupied[row_index][cursor..end_column]
                    .iter()
                    .any(|slot| *slot)
            {
                return Err(TableGridError::ColumnSpan { cell: cell.id });
            }
            let Some(end_row) = row_index.checked_add(rows) else {
                return Err(TableGridError::RowSpan { cell: cell.id });
            };
            if end_row > table.rows.len() {
                return Err(TableGridError::RowSpan { cell: cell.id });
            }
            if occupied[row_index..end_row].iter().any(|occupied_row| {
                occupied_row[cursor..end_column]
                    .iter()
                    .any(|slot| *slot)
            }) {
                return Err(TableGridError::ColumnSpan { cell: cell.id });
            }
            for occupied_row in &mut occupied[row_index..end_row] {
                for slot in &mut occupied_row[cursor..end_column] {
                    *slot = true;
                }
            }
            cursor = end_column;
        }
        if occupied[row_index].iter().any(|slot| !*slot) {
            return Err(TableGridError::RowWidth { row: row.id });
        }
    }
    Ok(())
}

fn next_grid_seed(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    u32::try_from(*seed >> 32).expect("upper half fits u32")
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


fn allocate_path_identity(next: &mut u32, targets: &mut Vec<u32>) -> u32 {
    let identity = *next;
    *next = next.checked_add(1).expect("small generated identity space");
    targets.push(identity);
    identity
}

fn path_rule_block(next: &mut u32, targets: &mut Vec<u32>) -> Block<u32> {
    Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id: allocate_path_identity(next, targets),
        provenance: None,
        style: None,
    }
}

fn generated_path_notebook(seed: &mut u64) -> (Notebook<u32>, Vec<u32>) {
    let mut next = 1u32;
    let mut targets = Vec::new();
    let notebook = allocate_path_identity(&mut next, &mut targets);
    let profile = allocate_path_identity(&mut next, &mut targets);
    let page = allocate_path_identity(&mut next, &mut targets);
    let flow = allocate_path_identity(&mut next, &mut targets);
    let span = allocate_path_identity(&mut next, &mut targets);
    let leaf = allocate_path_identity(&mut next, &mut targets);
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
    let depth = (next_grid_seed(seed) % 8).saturating_add(1);
    for _ in 0..depth {
        let wrapper = allocate_path_identity(&mut next, &mut targets);
        let kind = next_grid_seed(seed) % 4;
        block = match kind {
            0 | 1 => {
                let before = path_rule_block(&mut next, &mut targets);
                let after = path_rule_block(&mut next, &mut targets);
                let children = match next_grid_seed(seed) % 3 {
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
                let list = allocate_path_identity(&mut next, &mut targets);
                let first_item =
                    allocate_path_identity(&mut next, &mut targets);
                let second_item =
                    allocate_path_identity(&mut next, &mut targets);
                let first_rule = path_rule_block(&mut next, &mut targets);
                let second_rule = path_rule_block(&mut next, &mut targets);
                let (first_blocks, second_blocks) =
                    if next_grid_seed(seed) & 1 == 0 {
                        (vec![first_rule], vec![second_rule, block])
                    } else {
                        (vec![first_rule, block], vec![second_rule])
                    };
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
                        ordered: next_grid_seed(seed) & 1 == 0,
                    }),
                    extensions: vec![],
                    id: wrapper,
                    provenance: None,
                    style: None,
                }
            },
            _ => {
                let table = allocate_path_identity(&mut next, &mut targets);
                let first_row = allocate_path_identity(&mut next, &mut targets);
                let second_row =
                    allocate_path_identity(&mut next, &mut targets);
                let first_cell =
                    allocate_path_identity(&mut next, &mut targets);
                let second_cell =
                    allocate_path_identity(&mut next, &mut targets);
                let first_rule = path_rule_block(&mut next, &mut targets);
                let second_rule = path_rule_block(&mut next, &mut targets);
                let (first_blocks, second_blocks) =
                    if next_grid_seed(seed) & 1 == 0 {
                        (vec![first_rule], vec![second_rule, block])
                    } else {
                        (vec![first_rule, block], vec![second_rule])
                    };
                Block {
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
                }
            },
        };
    }
    (
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
                page_profile: profile,
            }],
            provenance: vec![],
            styles: vec![],
        },
        targets,
    )
}

fn descriptor_owner_chain(
    notebook: &Notebook<u32>,
    target: u32,
) -> Vec<(u32, SemanticIdentityDescriptor<u32>)> {
    let mut chain = Vec::new();
    let mut current = target;
    loop {
        let descriptor = semantic_identity_descriptor(notebook, current)
            .expect("generated target must have descriptor");
        chain.push((current, descriptor));
        let Some(owner) = descriptor.owner else {
            break;
        };
        current = owner;
    }
    chain
}

#[test]
fn accepted_candidate_and_revision_sequences_never_reuse_within_authority() {
    let identities = IdentityAllocator::new();
    let accepted_a = identities.allocate_accepted().expect("accepted id");
    let accepted_b = identities.allocate_accepted().expect("accepted id");
    let candidate_a = identities.allocate_candidate().expect("candidate id");
    let candidate_b = identities.allocate_candidate().expect("candidate id");
    let revision_a = identities.allocate_revision().expect("revision id");
    let revision_b = identities.allocate_revision().expect("revision id");

    assert_ne!(accepted_a, accepted_b);
    assert_ne!(candidate_a, candidate_b);
    assert_ne!(revision_a, revision_b);
}

#[test]
fn cloned_semantic_state_preserves_stable_identity() {
    let identities = IdentityAllocator::new();
    let notebook_id = identities.allocate_accepted().expect("notebook id");
    let page_id = identities.allocate_accepted().expect("page id");
    let page_profile_id =
        identities.allocate_accepted().expect("page profile id");
    let flow_id = identities.allocate_accepted().expect("flow id");
    let block_id = identities.allocate_accepted().expect("block id");
    let span_id = identities.allocate_accepted().expect("span id");
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
            page_profile: page_profile_id,
        }],
        provenance: vec![],
        styles: vec![],
    };

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
}

#[test]
fn simple_inline_block_kinds_keep_span_ownership() {
    type InlineBlockConstructor =
        fn(Vec<InlineSpan<u32>>) -> BlockContent<u32>;
    let cases: &[(SemanticBlockKind, InlineBlockConstructor)] = &[
        (SemanticBlockKind::Date, BlockContent::Date),
        (SemanticBlockKind::Definition, BlockContent::Definition),
        (SemanticBlockKind::Heading, BlockContent::Heading),
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
                page_profile: 2,
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
    }
}

#[test]
fn notebook_root_families_have_exact_two_entry_paths() {
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
            .expect("root-owned identity must have a semantic path");
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], SemanticIdentityPathEntry {
            descriptor: SemanticIdentityDescriptor {
                kind,
                owner: Some(notebook.id),
            },
            identity: target,
        });
        assert_eq!(path[1], SemanticIdentityPathEntry {
            descriptor: SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Notebook,
                owner: None,
            },
            identity: notebook.id,
        });
    }
}

#[test]
fn semantic_identity_path_matches_descriptor_chain_across_nested_families() {
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
            page_profile: 2,
        }],
        provenance: vec![],
        styles: vec![],
    };

    for target in [1, 2, 20, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] {
        let path = semantic_identity_path(&notebook, target)
            .expect("fixture identity must have a semantic path");
        let first = path.first().expect("path contains target");
        assert_eq!(first.identity, target);
        assert_eq!(
            first.descriptor,
            semantic_identity_descriptor(&notebook, target)
                .expect("fixture identity must have a descriptor"),
        );
        for adjacent in path.windows(2) {
            let Some(current) = adjacent.first() else {
                panic!("path window has current entry");
            };
            let Some(owner) = adjacent.get(1) else {
                panic!("path window has owner entry");
            };
            assert_eq!(current.descriptor.owner, Some(owner.identity));
        }
        let root = path.last().expect("path reaches notebook root");
        assert_eq!(root.identity, notebook.id);
        assert_eq!(root.descriptor.kind, SemanticIdentityKind::Notebook);
        assert_eq!(root.descriptor.owner, None);
    }
    assert_eq!(semantic_identity_path(&notebook, u32::MAX), None);
}

#[test]
fn semantic_identity_path_matches_descriptor_walk_on_generated_trees() {
    let mut seed = 0x9e37_79b9_7f4a_7c15;
    for case in 0..5_000u32 {
        let (notebook, targets) = generated_path_notebook(&mut seed);
        for target in targets {
            let path = semantic_identity_path(&notebook, target)
                .expect("generated target must have semantic path");
            let actual = path
                .iter()
                .map(|entry| (entry.identity, entry.descriptor))
                .collect::<Vec<_>>();
            assert_eq!(
                actual,
                descriptor_owner_chain(&notebook, target),
                "generated path mismatch in case {case} target {target}",
            );
        }
    }
}

#[test]
fn logical_table_validator_matches_naive_occupancy_oracle() {
    let mut seed = 0x5eed_1234_9876_abcd;
    for case in 0..20_000u32 {
        let row_count = usize::try_from((next_grid_seed(&mut seed) % 4) + 1)
            .expect("small row count");
        let mut rows = Vec::with_capacity(row_count);
        let mut identity = case.saturating_mul(100).saturating_add(1);
        for _ in 0..row_count {
            let cell_count = usize::try_from(next_grid_seed(&mut seed) % 4)
                .expect("small cell count");
            let mut cells = Vec::with_capacity(cell_count);
            for _ in 0..cell_count {
                let columns = (next_grid_seed(&mut seed) % 3) + 1;
                let span_rows = (next_grid_seed(&mut seed) % 3) + 1;
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
        assert_eq!(
            table.validate_grid(),
            grid_oracle_result(&table),
            "typed occupancy oracle mismatch in generated case {case}",
        );
    }
}

#[test]
fn maximum_logical_colspan_stays_compact() {
    let maximum = NonZeroU32::MAX.get();
    let table = Table {
        id: 100u32,
        rows: vec![grid_row(1, vec![grid_cell(10, maximum, 1)])],
    };

    assert_eq!(table.validate_grid(), Ok(()));
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

#[test]
fn semantic_identity_kind_reaches_nested_table_owners() {
    let identities = IdentityAllocator::new();
    let notebook_id = identities.allocate_candidate().expect("notebook id");
    let profile_id = identities.allocate_candidate().expect("profile id");
    let page_id = identities.allocate_candidate().expect("page id");
    let flow_id = identities.allocate_candidate().expect("flow id");
    let block_id = identities.allocate_candidate().expect("block id");
    let table_id = identities.allocate_candidate().expect("table id");
    let row_id = identities.allocate_candidate().expect("row id");
    let cell_id = identities.allocate_candidate().expect("cell id");
    let child_id = identities.allocate_candidate().expect("child id");
    let missing = identities.allocate_candidate().expect("missing id");
    let notebook = Notebook {
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
            page_profile: profile_id,
        }],
        provenance: vec![],
        styles: vec![],
    };
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
}

#[test]
fn semantic_identity_descriptor_handles_deep_nesting_iteratively() {
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
            page_profile: 4,
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
    let path = semantic_identity_path(&notebook, TARGET)
        .expect("deep target must have a structural path");
    assert_eq!(path.len(), usize::try_from(DEPTH).expect("depth fits") + 4);
    assert_eq!(path.first().map(|entry| entry.identity), Some(TARGET));
    assert_eq!(path.last().map(|entry| entry.identity), Some(notebook.id));
    std::mem::forget(notebook);
}

#[test]
fn unresolved_semantics_and_extensions_are_preserved_exactly() {
    let identities = IdentityAllocator::new();
    let block_id = identities.allocate_candidate().expect("block id");
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
        panic!("fixture must remain unresolved");
    };
    assert_eq!(unresolved.source, "future semantic object");
    assert_eq!(
        unresolved.extensions.as_slice(),
        std::slice::from_ref(&extension),
    );
    assert_eq!(block.extensions, [extension]);
}
