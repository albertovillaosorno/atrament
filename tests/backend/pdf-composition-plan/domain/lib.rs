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
//   - Regression evidence for vector-preserving PDF composition intent.
// - Must-Not:
//   - Emit PDF bytes, choose fonts/codecs/compression, write files, or infer
//     searchable-text or asset compatibility.
// - Allows:
//   - Inputs: Deterministic caller-owned page/resource/manifest fixtures.
//   - Outputs: Assertions over page order, physical boxes, vectors, and
//     resources.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - PDF serialization or embedding gains independent executable fixtures.
// - Merge-When:
//   - Composition evidence moves into a PDF adapter harness.
// - Summary:
//   - Proves PDF composition stays explicit before byte generation.
// - Description:
//   - Covers ordered pages, vector authority, assets, color, text, and
//     manifest.
// - Usage:
//   - Compile directly against the pdf-composition-plan domain.
// - Defaults:
//   - No accepted content disappears implicitly.
//
use atrament_pdf_composition_plan::{
    PdfAssetDisposition, PdfAssetResource, PdfCompositionPlan, PdfPagePlan,
    SearchableSemanticText,
};

#[test]
fn pages_preserve_order_physical_boxes_vectors_and_color_intent() {
    let plan = PdfCompositionPlan {
        assets: Vec::<PdfAssetResource<&str>>::new(),
        pages: vec![
            PdfPagePlan {
                color_intent: "srgb",
                physical_page_box: (210_000_u64, 297_000_u64),
                searchable_text: SearchableSemanticText::Preserved("page-one"),
                vector_authority: "vector-page-1",
            },
            PdfPagePlan {
                color_intent: "srgb",
                physical_page_box: (210_000_u64, 297_000_u64),
                searchable_text:
                    SearchableSemanticText::<&str>::OmittedAsIncompatible,
                vector_authority: "vector-page-2",
            },
        ],
        render_manifest_identity: "manifest-4",
    };
    assert_eq!(plan.pages[0].vector_authority, "vector-page-1");
    assert_eq!(plan.pages[1].vector_authority, "vector-page-2");
    assert_eq!(plan.pages[0].physical_page_box, (210_000, 297_000));
    assert_eq!(plan.pages[0].color_intent, "srgb");
    assert_eq!(plan.render_manifest_identity, "manifest-4");
}

#[test]
fn searchable_text_disposition_is_explicit_instead_of_silent() {
    let preserved = SearchableSemanticText::Preserved("semantic-text");
    let omitted = SearchableSemanticText::<&str>::OmittedAsIncompatible;
    assert_eq!(
        preserved,
        SearchableSemanticText::Preserved("semantic-text"),
    );
    assert_eq!(omitted, SearchableSemanticText::OmittedAsIncompatible);
}

#[test]
fn asset_embedding_and_safe_reference_are_distinct() {
    let assets = [
        PdfAssetResource {
            asset_identity: "photo-1",
            disposition: PdfAssetDisposition::Embedded,
        },
        PdfAssetResource {
            asset_identity: "photo-2",
            disposition: PdfAssetDisposition::SafelyReferenced,
        },
    ];
    assert_ne!(assets[0].disposition, assets[1].disposition);
    assert_eq!(assets[0].asset_identity, "photo-1");
    assert_eq!(assets[1].asset_identity, "photo-2");
}

fn next_index_permutation(values: &mut [usize]) -> bool {
    let Some(pivot) = (0..values.len().saturating_sub(1))
        .rev()
        .find(|&index| values[index] < values[index + 1])
    else {
        return false;
    };
    let successor = (pivot + 1..values.len())
        .rev()
        .find(|&index| values[pivot] < values[index])
        .expect("nonterminal permutation has a successor");
    values.swap(pivot, successor);
    values[pivot + 1..].reverse();
    true
}

#[test]
fn every_page_resource_order_and_disposition_state_is_preserved() {
    type Page = PdfPagePlan<u16, (u32, u32), u16, u16>;
    type Asset = PdfAssetResource<u16>;
    let mut page_order = [0_usize, 1, 2, 3];
    let mut page_permutations = 0_u8;
    let mut cases = 0_u32;
    loop {
        page_permutations = page_permutations.saturating_add(1);
        let mut asset_order = [0_usize, 1, 2];
        loop {
            for text_mask in 0_u8..16 {
                for asset_mask in 0_u8..8 {
                    let pages = page_order
                        .iter()
                        .map(|&source_index| {
                            let source = source_index as u16;
                            let searchable_text = if text_mask
                                & (1_u8 << source_index)
                                == 0
                            {
                                SearchableSemanticText::OmittedAsIncompatible
                            } else {
                                SearchableSemanticText::Preserved(
                                    3_000 + source,
                                )
                            };
                            Page {
                                color_intent: 1_000 + source,
                                physical_page_box: (
                                    210_000 + u32::from(source),
                                    297_000 + u32::from(source),
                                ),
                                searchable_text,
                                vector_authority: 2_000 + source,
                            }
                        })
                        .collect::<Vec<_>>();
                    let assets = asset_order
                        .iter()
                        .map(|&source_index| {
                            let source = source_index as u16;
                            Asset {
                                asset_identity: 4_000 + source,
                                disposition: if asset_mask
                                    & (1_u8 << source_index)
                                    == 0
                                {
                                    PdfAssetDisposition::Embedded
                                } else {
                                    PdfAssetDisposition::SafelyReferenced
                                },
                            }
                        })
                        .collect::<Vec<_>>();
                    let plan = PdfCompositionPlan {
                        assets,
                        pages,
                        render_manifest_identity: 5_000_u16,
                    };
                    assert_eq!(plan.render_manifest_identity, 5_000);
                    for (position, &source_index) in
                        page_order.iter().enumerate()
                    {
                        let source = source_index as u16;
                        let page = &plan.pages[position];
                        assert_eq!(page.color_intent, 1_000 + source);
                        assert_eq!(
                            page.physical_page_box,
                            (
                                210_000 + u32::from(source),
                                297_000 + u32::from(source),
                            ),
                        );
                        assert_eq!(page.vector_authority, 2_000 + source);
                        match &page.searchable_text {
                            SearchableSemanticText::OmittedAsIncompatible => {
                                assert_eq!(
                                    text_mask & (1_u8 << source_index),
                                    0,
                                );
                            },
                            SearchableSemanticText::Preserved(value) => {
                                assert_ne!(
                                    text_mask & (1_u8 << source_index),
                                    0,
                                );
                                assert_eq!(*value, 3_000 + source);
                            },
                        }
                    }
                    for (position, &source_index) in
                        asset_order.iter().enumerate()
                    {
                        let source = source_index as u16;
                        let asset = &plan.assets[position];
                        assert_eq!(asset.asset_identity, 4_000 + source);
                        let expected = if asset_mask
                            & (1_u8 << source_index)
                            == 0
                        {
                            PdfAssetDisposition::Embedded
                        } else {
                            PdfAssetDisposition::SafelyReferenced
                        };
                        assert_eq!(asset.disposition, expected);
                    }
                    cases = cases.saturating_add(1);
                }
            }
            if !next_index_permutation(&mut asset_order) {
                break;
            }
        }
        if !next_index_permutation(&mut page_order) {
            break;
        }
    }
    assert_eq!(page_permutations, 24);
    assert_eq!(cases, 18_432);
}
