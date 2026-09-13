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
//   - Regression evidence for vector primitive taxonomy and semantic
//     provenance.
// - Must-Not:
//   - Generate geometry, choose units, expand contours, rasterize, blend, emit
//     PDF, or plan machine motion.
// - Allows:
//   - Inputs: Deterministic caller-owned vector geometry fixtures.
//   - Outputs: Assertions over primitive kinds, order, bounds, and provenance.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Geometry generation or rendering gains independent executable fixtures.
// - Merge-When:
//   - Vector authority moves into another pure projection harness.
// - Summary:
//   - Proves vector authority remains semantic and projection-neutral.
// - Description:
//   - Covers accepted primitive kinds, physical bounds, order, and provenance.
// - Usage:
//   - Compile directly against the vector-geometry domain.
// - Defaults:
//   - No material or renderer semantics are inferred from primitive kinds.
//
use atrament_vector_geometry::{
    VectorGeometryPage, VectorPrimitive, VectorPrimitiveKind,
    semantic_origin_primitive_indices,
};

#[test]
fn accepted_vector_primitive_families_are_explicit() {
    let kinds = [
        VectorPrimitiveKind::DiagramPath,
        VectorPrimitiveKind::Equation,
        VectorPrimitiveKind::ExpandedInkContour,
        VectorPrimitiveKind::LayoutBox,
        VectorPrimitiveKind::RulePath,
        VectorPrimitiveKind::StrokeCenterline,
        VectorPrimitiveKind::Table,
    ];
    assert_eq!(kinds.len(), 7);
}

#[test]
fn every_vector_primitive_retains_semantic_provenance() {
    let primitive = VectorPrimitive {
        geometry: "caller-owned-centerline",
        kind: VectorPrimitiveKind::StrokeCenterline,
        semantic_origin: "span-42",
    };
    assert_eq!(primitive.semantic_origin, "span-42");
    assert_eq!(primitive.geometry, "caller-owned-centerline");
}

#[test]
fn page_keeps_caller_owned_physical_bounds_and_composition_order() {
    let page = VectorGeometryPage {
        physical_bounds: (210_000_u64, 297_000_u64),
        primitives: vec![
            VectorPrimitive {
                geometry: "box-a",
                kind: VectorPrimitiveKind::LayoutBox,
                semantic_origin: "block-a",
            },
            VectorPrimitive {
                geometry: "equation-a",
                kind: VectorPrimitiveKind::Equation,
                semantic_origin: "formula-a",
            },
        ],
    };
    assert_eq!(page.physical_bounds, (210_000, 297_000));
    assert_eq!(page.primitives[0].semantic_origin, "block-a");
    assert_eq!(page.primitives[1].semantic_origin, "formula-a");
}

#[test]
fn vector_primitive_kind_does_not_rewrite_caller_geometry() {
    let contour = VectorPrimitive {
        geometry: vec![(1_i32, 2_i32), (3_i32, 4_i32)],
        kind: VectorPrimitiveKind::ExpandedInkContour,
        semantic_origin: 7_u64,
    };
    assert_eq!(contour.geometry, [(1, 2), (3, 4)]);
    assert_eq!(contour.semantic_origin, 7);
}
#[test]
fn semantic_origin_projection_preserves_composition_order_and_duplicates() {
    let page = VectorGeometryPage {
        physical_bounds: "page-a",
        primitives: vec![
            VectorPrimitive {
                geometry: "box-a",
                kind: VectorPrimitiveKind::LayoutBox,
                semantic_origin: "block-a",
            },
            VectorPrimitive {
                geometry: "stroke-a",
                kind: VectorPrimitiveKind::StrokeCenterline,
                semantic_origin: "span-b",
            },
            VectorPrimitive {
                geometry: "contour-a",
                kind: VectorPrimitiveKind::ExpandedInkContour,
                semantic_origin: "span-b",
            },
            VectorPrimitive {
                geometry: "rule-a",
                kind: VectorPrimitiveKind::RulePath,
                semantic_origin: "block-a",
            },
        ],
    };
    assert_eq!(semantic_origin_primitive_indices(&page, &"block-a"), [0, 3],);
    assert_eq!(semantic_origin_primitive_indices(&page, &"span-b"), [1, 2],);
    assert_eq!(
        semantic_origin_primitive_indices(&page, &"missing"),
        Vec::<usize>::new(),
    );
    assert_eq!(page.primitives.len(), 4);
    assert_eq!(page.physical_bounds, "page-a");
}


#[test]
fn every_twelve_primitive_origin_mask_projects_exact_composition_indices() {
    const PRIMITIVE_COUNT: usize = 12;
    const MASK_COUNT: usize = 1 << PRIMITIVE_COUNT;
    const KINDS: [VectorPrimitiveKind; 7] = [
        VectorPrimitiveKind::DiagramPath,
        VectorPrimitiveKind::Equation,
        VectorPrimitiveKind::ExpandedInkContour,
        VectorPrimitiveKind::LayoutBox,
        VectorPrimitiveKind::RulePath,
        VectorPrimitiveKind::StrokeCenterline,
        VectorPrimitiveKind::Table,
    ];

    let mut cases = 0_usize;
    for mask in 0..MASK_COUNT {
        let primitives = (0..PRIMITIVE_COUNT)
            .map(|index| VectorPrimitive {
                geometry: index,
                kind: KINDS[index % KINDS.len()],
                semantic_origin: if mask & (1 << index) == 0 {
                    "other"
                } else {
                    "target"
                },
            })
            .collect::<Vec<_>>();
        let page = VectorGeometryPage {
            physical_bounds: (210_000_u64, 297_000_u64),
            primitives,
        };
        let before = page.clone();
        let expected = (0..PRIMITIVE_COUNT)
            .filter(|index| mask & (1 << index) != 0)
            .collect::<Vec<_>>();
        assert_eq!(
            semantic_origin_primitive_indices(&page, &"target"),
            expected,
            "origin mask {mask:#014b}",
        );
        assert_eq!(
            semantic_origin_primitive_indices(&page, &"missing"),
            Vec::<usize>::new(),
            "missing origin mask {mask:#014b}",
        );
        assert_eq!(page, before, "projection mutated mask {mask:#014b}");
        cases += 1;
    }
    assert_eq!(cases, MASK_COUNT);
}
