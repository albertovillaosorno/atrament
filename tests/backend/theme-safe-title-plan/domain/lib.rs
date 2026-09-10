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
//   - Regression evidence for hierarchy-safe digital/live title projections.
// - Must-Not:
//   - Derive title styles, choose typography, render, or infer conversion
//     details.
// - Allows:
//   - Inputs: Deterministic caller-owned title, hierarchy, and treatment data.
//   - Outputs: Assertions over required digital treatment and shared identity.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Title derivation, layout, or rendering gains independent fixtures.
// - Merge-When:
//   - Title evidence moves into a renderer/theme harness.
// - Summary:
//   - Proves media-specific titles keep one semantic title hierarchy.
// - Description:
//   - Covers digital layers/outlines/highlights/motifs and live one-pen intent.
// - Usage:
//   - Compile directly against the theme-safe-title-plan domain.
// - Defaults:
//   - No title style, hierarchy mapping, or conversion is synthesized.
//
use atrament_theme_safe_title_plan::{
    DigitalTitleTreatment, ThemeSafeTitlePlan, ThemeSafeTitlePlanError,
    TitleProjection, validate_theme_safe_title_plan,
};

type DigitalTreatment =
    DigitalTitleTreatment<&'static str, &'static str, u8, u8>;
type Projection<Treatment> = TitleProjection<u8, &'static str, Treatment>;
type Plan =
    ThemeSafeTitlePlan<Projection<DigitalTreatment>, Projection<&'static str>>;

fn plan() -> Plan {
    ThemeSafeTitlePlan {
        digital: TitleProjection {
            hierarchy: 2,
            title_identity: "title-17",
            treatment: DigitalTitleTreatment {
                highlight: "marker-strip",
                layering: "two-layer-lettering",
                motif: 3,
                outline: 4,
            },
        },
        live: TitleProjection {
            hierarchy: 2,
            title_identity: "title-17",
            treatment: "caller-owned-sober-one-pen-title",
        },
    }
}

#[test]
fn digital_treatment_structurally_retains_every_required_decorative_input() {
    let plan = plan();
    assert_eq!(plan.digital.treatment.highlight, "marker-strip");
    assert_eq!(plan.digital.treatment.layering, "two-layer-lettering");
    assert_eq!(plan.digital.treatment.motif, 3);
    assert_eq!(plan.digital.treatment.outline, 4);
    assert_eq!(plan.live.treatment, "caller-owned-sober-one-pen-title");
    assert_eq!(validate_theme_safe_title_plan(&plan), Ok(()));
}

#[test]
fn title_identity_mismatch_rejects_before_hierarchy_mismatch() {
    let mut plan = plan();
    plan.live.title_identity = "title-18";
    plan.live.hierarchy = 7;
    assert_eq!(
        validate_theme_safe_title_plan(&plan),
        Err(ThemeSafeTitlePlanError::TitleIdentityMismatch),
    );
}

#[test]
fn hierarchy_mismatch_rejects_when_title_identity_matches() {
    let mut plan = plan();
    plan.live.hierarchy = 7;
    assert_eq!(
        validate_theme_safe_title_plan(&plan),
        Err(ThemeSafeTitlePlanError::HierarchyMismatch),
    );
}

#[test]
fn mode_specific_treatment_may_differ_without_losing_shared_hierarchy() {
    for hierarchy in 0_u8..=63 {
        let mut plan = plan();
        plan.digital.hierarchy = hierarchy;
        plan.live.hierarchy = hierarchy;
        plan.digital.treatment.motif = hierarchy;
        plan.live.treatment = if hierarchy % 2 == 0 {
            "one-pen-large"
        } else {
            "one-pen-underlined"
        };
        assert_eq!(validate_theme_safe_title_plan(&plan), Ok(()));
    }
}
