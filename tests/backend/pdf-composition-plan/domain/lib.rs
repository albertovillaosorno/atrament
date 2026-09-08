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
