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

//! Pdf composition plan domain regression tests.

use atrament_pdf_composition_plan::{
    PdfAssetDisposition, PdfAssetResource, PdfCompositionPlan, PdfPagePlan,
    SearchableSemanticText,
};

type Page = PdfPagePlan<u16, (u32, u32), u16, u16>;
type Asset = PdfAssetResource<u16>;
type Composition = PdfCompositionPlan<Page, Asset, u16>;

#[test]
fn pages_preserve_order_physical_boxes_vectors_and_color_intent() {
    let plan = PdfCompositionPlan {
        assets: Vec::<PdfAssetResource<&str>>::new(),
        pages: vec![
            PdfPagePlan {
                color_intent: "srgb",
                physical_page_box: (210_000u64, 297_000u64),
                searchable_text: SearchableSemanticText::Preserved("page-one"),
                vector_authority: "vector-page-1",
            },
            PdfPagePlan {
                color_intent: "srgb",
                physical_page_box: (210_000u64, 297_000u64),
                searchable_text:
                    SearchableSemanticText::<&str>::OmittedAsIncompatible,
                vector_authority: "vector-page-2",
            },
        ],
        render_manifest_identity: "manifest-4",
    };
    assert_eq!(
        plan.pages.first().map(|page| page.vector_authority),
        Some("vector-page-1"),
    );
    assert_eq!(
        plan.pages.get(1).map(|page| page.vector_authority),
        Some("vector-page-2"),
    );
    assert_eq!(
        plan.pages.first().map(|page| page.physical_page_box),
        Some((210_000, 297_000)),
    );
    assert_eq!(
        plan.pages.first().map(|page| page.color_intent),
        Some("srgb"),
    );
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
    assert_ne!(
        assets.first().map(|asset| asset.disposition),
        assets.get(1).map(|asset| asset.disposition),
    );
    assert_eq!(
        assets.first().map(|asset| asset.asset_identity),
        Some("photo-1"),
    );
    assert_eq!(
        assets.get(1).map(|asset| asset.asset_identity),
        Some("photo-2"),
    );
}

fn next_index_permutation(values: &mut [u16]) -> bool {
    let Some((pivot, pivot_value)) =
        values.windows(2).enumerate().rev().find_map(
            |(index, pair)| match pair {
                [left, right] if left < right => Some((index, *left)),
                _ => None,
            },
        )
    else {
        return false;
    };
    let Some((successor, _)) = values
        .iter()
        .enumerate()
        .rev()
        .find(|(index, value)| *index > pivot && **value > pivot_value)
    else {
        return false;
    };
    values.swap(pivot, successor);
    let Some(tail) = values.get_mut(pivot.saturating_add(1)..) else {
        return false;
    };
    tail.reverse();
    true
}

fn page_for(source: u16, text_mask: u8) -> Page {
    let searchable_text = if text_mask & (1u8 << source) == 0 {
        SearchableSemanticText::OmittedAsIncompatible
    } else {
        SearchableSemanticText::Preserved(3_000u16.saturating_add(source))
    };
    Page {
        color_intent: 1_000u16.saturating_add(source),
        physical_page_box: (
            210_000u32.saturating_add(u32::from(source)),
            297_000u32.saturating_add(u32::from(source)),
        ),
        searchable_text,
        vector_authority: 2_000u16.saturating_add(source),
    }
}

const fn asset_for(source: u16, asset_mask: u8) -> Asset {
    Asset {
        asset_identity: 4_000u16.saturating_add(source),
        disposition: if asset_mask & (1u8 << source) == 0 {
            PdfAssetDisposition::Embedded
        } else {
            PdfAssetDisposition::SafelyReferenced
        },
    }
}

fn assert_page_order(plan: &Composition, order: [u16; 4], text_mask: u8) {
    assert_eq!(plan.pages.len(), order.len(), "all pages retained");
    for (&source, page) in order.iter().zip(&plan.pages) {
        assert_eq!(
            page.color_intent,
            1_000u16.saturating_add(source),
            "page {source} color intent",
        );
        assert_eq!(
            page.physical_page_box,
            (
                210_000u32.saturating_add(u32::from(source)),
                297_000u32.saturating_add(u32::from(source)),
            ),
            "page {source} physical box",
        );
        assert_eq!(
            page.vector_authority,
            2_000u16.saturating_add(source),
            "page {source} vector authority",
        );
        match &page.searchable_text {
            SearchableSemanticText::OmittedAsIncompatible => {
                assert_eq!(
                    text_mask & (1u8 << source),
                    0,
                    "page {source} omitted search text",
                );
            }
            SearchableSemanticText::Preserved(value) => {
                assert_ne!(
                    text_mask & (1u8 << source),
                    0,
                    "page {source} preserved search text",
                );
                assert_eq!(
                    *value,
                    3_000u16.saturating_add(source),
                    "page {source} search text identity",
                );
            }
        }
    }
}

fn assert_asset_order(plan: &Composition, order: [u16; 3], asset_mask: u8) {
    assert_eq!(plan.assets.len(), order.len(), "all assets retained");
    for (&source, asset) in order.iter().zip(&plan.assets) {
        assert_eq!(
            asset.asset_identity,
            4_000u16.saturating_add(source),
            "asset {source} identity",
        );
        let expected = if asset_mask & (1u8 << source) == 0 {
            PdfAssetDisposition::Embedded
        } else {
            PdfAssetDisposition::SafelyReferenced
        };
        assert_eq!(asset.disposition, expected, "asset {source} disposition");
    }
}

#[test]
fn every_page_resource_order_and_disposition_state_is_preserved() {
    let mut page_order = [0u16, 1, 2, 3];
    let mut page_permutations = 0u8;
    let mut cases = 0u32;
    loop {
        page_permutations = page_permutations.saturating_add(1);
        let mut asset_order = [0u16, 1, 2];
        loop {
            for text_mask in 0u8..16 {
                for asset_mask in 0u8..8 {
                    let plan = Composition {
                        assets: asset_order
                            .into_iter()
                            .map(|source| asset_for(source, asset_mask))
                            .collect(),
                        pages: page_order
                            .into_iter()
                            .map(|source| page_for(source, text_mask))
                            .collect(),
                        render_manifest_identity: 5_000,
                    };
                    assert_eq!(plan.render_manifest_identity, 5_000);
                    assert_page_order(&plan, page_order, text_mask);
                    assert_asset_order(&plan, asset_order, asset_mask);
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
