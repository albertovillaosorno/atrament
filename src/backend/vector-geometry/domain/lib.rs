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
//   - Transport-neutral authoritative vector primitive taxonomy and provenance.
// - Must-Not:
//   - Generate geometry, choose units, expand contours, tessellate, rasterize,
//     blend materials, emit PDF, or plan machine motion.
// - Allows:
//   - Inputs: Caller-owned physical bounds, geometry, and semantic provenance.
//   - Outputs: Ordered authoritative vector primitives grouped by one page.
//   - Side effects: None.
// - Split-When:
//   - Geometry generation, material projection, or PDF output gains independent
//     executable authority.
// - Merge-When:
//   - Vector geometry becomes inseparable from one renderer implementation.
// - Summary:
//   - Keeps vector topology and semantic provenance separate from rendering.
// - Description:
//   - Classifies accepted vector primitive families without inventing geometry.
// - Usage:
//   - Carry generated geometry into preview, PDF, or live-compatible
//     projection.
// - Defaults:
//   - No primitive kind implies a renderer or material behavior.
//

//! Projection-neutral vector geometry authority with semantic provenance.

/// Accepted authoritative vector primitive families.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum VectorPrimitiveKind {
    /// Measured path belonging to a diagram.
    DiagramPath,
    /// Vector geometry for one mathematical equation.
    Equation,
    /// Expanded ink contour around authoritative handwriting centerlines.
    ExpandedInkContour,
    /// Layout box emitted by semantic layout.
    LayoutBox,
    /// Measured rule or ruler-like path.
    RulePath,
    /// Authoritative handwriting stroke centerline.
    StrokeCenterline,
    /// Vector geometry for one table.
    Table,
}

/// One authoritative vector primitive tied to semantic provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorPrimitive<SemanticOrigin, Geometry> {
    /// Caller-owned physical vector geometry.
    pub geometry: Geometry,
    /// Primitive family that owns interpretation of the geometry.
    pub kind: VectorPrimitiveKind,
    /// Semantic object or span from which this geometry was derived.
    pub semantic_origin: SemanticOrigin,
}

/// Complete ordered vector geometry for one caller-owned physical page region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorGeometryPage<PhysicalBounds, Primitive> {
    /// Caller-owned physical page bounds.
    pub physical_bounds: PhysicalBounds,
    /// Authoritative primitives in accepted composition order.
    pub primitives: Vec<Primitive>,
}
/// Return composition-order primitive indices for one exact semantic origin.
///
/// This projection preserves page geometry and primitive order unchanged. It
/// identifies an existing provenance dependency region only; no geometry is
/// generated, validated, or interpreted.
#[must_use]
pub fn semantic_origin_primitive_indices<
    PhysicalBounds,
    SemanticOrigin,
    Geometry,
>(
    page: &VectorGeometryPage<
        PhysicalBounds,
        VectorPrimitive<SemanticOrigin, Geometry>,
    >,
    semantic_origin: &SemanticOrigin,
) -> Vec<usize>
where
    SemanticOrigin: PartialEq,
{
    page.primitives
        .iter()
        .enumerate()
        .filter_map(|(primitive_index, primitive)| {
            (primitive.semantic_origin == *semantic_origin)
                .then_some(primitive_index)
        })
        .collect()
}
