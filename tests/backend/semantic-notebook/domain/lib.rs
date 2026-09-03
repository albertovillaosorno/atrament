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
    Block, BlockContent, ExtensionData, Flow, IdentityAllocator, InlineSpan,
    Notebook, Page, PaperProfile, SemanticBlockKind,
    SemanticIdentityDescriptor, SemanticIdentityKind, Table, TableCell,
    TableCellSpan, TableGridError, TableRow, TableRowRole, UnresolvedBlock,
    UnresolvedReason,
    semantic_identity_descriptor, semantic_identity_kind,
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
    assert_eq!(unresolved.extensions, [extension.clone()]);
    assert_eq!(block.extensions, [extension]);
}
