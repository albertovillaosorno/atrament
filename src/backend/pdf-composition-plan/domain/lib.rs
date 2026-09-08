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
//   - Transport-neutral vector-preserving PDF composition intent.
// - Must-Not:
//   - Emit PDF bytes, choose codecs, fonts, compression, object numbering,
//     filesystem paths, searchable-text compatibility, or resource policy.
// - Allows:
//   - Inputs: Ordered pages, vector authority, physical boxes, color intent,
//     explicit semantic-text/asset disposition, and render-manifest linkage.
//   - Outputs: One inspectable PDF composition plan.
//   - Side effects: None.
// - Split-When:
//   - PDF serialization or resource embedding gains executable authority.
// - Merge-When:
//   - Composition becomes inseparable from one PDF adapter implementation.
// - Summary:
//   - Preserves vector/page/resource authority before PDF serialization.
// - Description:
//   - Makes page order and compatibility dispositions explicit without bytes.
// - Usage:
//   - Validate renderer output before a future PDF serializer or file adapter.
// - Defaults:
//   - No text or asset is silently omitted or transformed.
//

//! Vector-preserving PDF composition intent independent of PDF byte encoding.

/// How one accepted asset participates in the PDF composition.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PdfAssetDisposition {
    /// Asset bytes are embedded in the produced PDF resource set.
    Embedded,
    /// Asset is retained through an admitted safe-reference mechanism.
    SafelyReferenced,
}

/// One accepted asset resource with explicit PDF disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfAssetResource<AssetIdentity> {
    /// Accepted source asset identity.
    pub asset_identity: AssetIdentity,
    /// Explicit embedding or safe-reference choice.
    pub disposition: PdfAssetDisposition,
}

/// Whether searchable semantic text is compatible with one page projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchableSemanticText<SemanticText> {
    /// Searchable text is intentionally absent because compatibility rejected
    /// it.
    OmittedAsIncompatible,
    /// Searchable semantic text is preserved for this page projection.
    Preserved(SemanticText),
}

/// One vector-preserving PDF page in final page order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfPagePlan<
    ColorIntent,
    PhysicalPageBox,
    SemanticText,
    VectorAuthority,
> {
    /// Caller-owned output color intent.
    pub color_intent: ColorIntent,
    /// Exact physical page box or dimensions.
    pub physical_page_box: PhysicalPageBox,
    /// Explicit searchable semantic-text disposition.
    pub searchable_text: SearchableSemanticText<SemanticText>,
    /// Authoritative vector page projection.
    pub vector_authority: VectorAuthority,
}

/// Complete PDF composition before any PDF byte serialization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfCompositionPlan<Page, AssetResource, RenderManifestIdentity> {
    /// Accepted PDF asset resources in caller-owned deterministic order.
    pub assets: Vec<AssetResource>,
    /// Pages in final document order.
    pub pages: Vec<Page>,
    /// Render-manifest identity associated with the composition.
    pub render_manifest_identity: RenderManifestIdentity,
}
