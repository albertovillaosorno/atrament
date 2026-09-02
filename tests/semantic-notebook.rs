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
//   - Verifies first semantic notebook domain authority invariants.
// - Description:
//   - Exercises opaque identity sequences and unresolved semantic preservation.
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
use atrament_semantic_notebook::{
    Block, BlockContent, ExtensionData, Flow, IdentityAllocator, InlineSpan,
    Notebook, Page, PaperProfile, SemanticBlockKind, SemanticIdentityKind,
    Table, TableCell, TableRow, TableRowRole, UnresolvedBlock,
    UnresolvedReason, semantic_identity_kind,
};

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
    assert_eq!(semantic_identity_kind(&notebook, missing), None);
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
