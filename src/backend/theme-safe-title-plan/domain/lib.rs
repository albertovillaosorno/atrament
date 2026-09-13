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
//   - Shared title identity/hierarchy across digital and live treatments.
// - Must-Not:
//   - Derive title styles, choose typography, colors, motifs, one-pen geometry,
//     hierarchy semantics, layout, conversion details, or rendering behavior.
// - Allows:
//   - Inputs: Caller-owned digital title treatments, live one-pen treatment,
//     stable title identity, and hierarchy identity/value.
//   - Outputs: Exact cross-mode identity/hierarchy compatibility validation.
//   - Side effects: None.
// - Split-When:
//   - Title derivation, layout, or rendering gains executable authority.
// - Merge-When:
//   - Title projection becomes inseparable from one renderer/theme service.
// - Summary:
//   - Preserves title hierarchy while digital and live media treatments differ.
// - Description:
//   - Requires one semantic title/hierarchy across decorative and sober forms.
// - Usage:
//   - Validate caller-produced digital/live title projections before rendering.
// - Defaults:
//   - No title treatment, hierarchy mapping, or conversion is inferred.
//

//! Cross-mode title projection invariants before theme derivation or rendering.

/// Digital-only decorative treatment required by the first-release title task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigitalTitleTreatment<Highlight, Layering, Motif, Outline> {
    /// Caller-owned title highlight treatment.
    pub highlight: Highlight,
    /// Caller-owned layered-lettering treatment.
    pub layering: Layering,
    /// Caller-owned decorative motif treatment.
    pub motif: Motif,
    /// Caller-owned title outline treatment.
    pub outline: Outline,
}

/// One title projection in either output mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TitleProjection<Hierarchy, TitleIdentity, Treatment> {
    /// Caller-owned hierarchy identity or exact hierarchy value.
    pub hierarchy: Hierarchy,
    /// Stable semantic title identity shared across modes.
    pub title_identity: TitleIdentity,
    /// Mode-specific title treatment.
    pub treatment: Treatment,
}

/// One digital decorative title paired with its live one-pen treatment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeSafeTitlePlan<DigitalProjection, LiveProjection> {
    /// Digital projection retaining layered decorative treatment.
    pub digital: DigitalProjection,
    /// Live projection retaining caller-owned sober one-pen treatment.
    pub live: LiveProjection,
}

/// Concrete cross-mode title plan shape admitted by this domain.
pub type CrossModeTitlePlan<
    Hierarchy,
    TitleIdentity,
    DigitalTreatment,
    LiveTreatment,
> = ThemeSafeTitlePlan<
    TitleProjection<Hierarchy, TitleIdentity, DigitalTreatment>,
    TitleProjection<Hierarchy, TitleIdentity, LiveTreatment>,
>;

/// Constructor-sealed evidence that one digital/live title pair preserves the
/// exact semantic title identity and hierarchy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedThemeSafeTitlePlan<'plan, Plan> {
    plan: &'plan Plan,
}

impl<'plan, Plan> ValidatedThemeSafeTitlePlan<'plan, Plan> {
    /// Return the exact caller-owned title plan that was admitted.
    #[must_use]
    pub const fn plan(&self) -> &'plan Plan {
        self.plan
    }
}

/// Why digital/live title projections do not preserve semantic hierarchy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeSafeTitlePlanError {
    /// Digital and live projections disagree on hierarchy.
    HierarchyMismatch,
    /// Digital and live projections refer to different semantic titles.
    TitleIdentityMismatch,
}

/// Validate one caller-produced digital/live title projection pair.
///
/// Treatment values are deliberately not compared because digital decoration
/// and live one-pen media are expected to differ.
///
/// # Errors
///
/// Returns title identity mismatch first, then hierarchy mismatch.
pub fn validate_theme_safe_title_plan<
    DigitalTreatment,
    Hierarchy,
    LiveTreatment,
    TitleIdentity,
>(
    plan: &ThemeSafeTitlePlan<
        TitleProjection<Hierarchy, TitleIdentity, DigitalTreatment>,
        TitleProjection<Hierarchy, TitleIdentity, LiveTreatment>,
    >,
) -> Result<(), ThemeSafeTitlePlanError>
where
    Hierarchy: PartialEq,
    TitleIdentity: PartialEq,
{
    if plan.digital.title_identity != plan.live.title_identity {
        return Err(ThemeSafeTitlePlanError::TitleIdentityMismatch);
    }
    if plan.digital.hierarchy != plan.live.hierarchy {
        return Err(ThemeSafeTitlePlanError::HierarchyMismatch);
    }
    Ok(())
}

/// Validate and seal one caller-produced digital/live title projection pair.
///
/// # Errors
///
/// Returns the same identity-first or hierarchy failure as
/// [`validate_theme_safe_title_plan`].
pub fn validate_theme_safe_title_plan_view<
    DigitalTreatment,
    Hierarchy,
    LiveTreatment,
    TitleIdentity,
>(
    plan: &CrossModeTitlePlan<
        Hierarchy,
        TitleIdentity,
        DigitalTreatment,
        LiveTreatment,
    >,
) -> Result<
    ValidatedThemeSafeTitlePlan<
        '_,
        CrossModeTitlePlan<
            Hierarchy,
            TitleIdentity,
            DigitalTreatment,
            LiveTreatment,
        >,
    >,
    ThemeSafeTitlePlanError,
>
where
    Hierarchy: PartialEq,
    TitleIdentity: PartialEq,
{
    validate_theme_safe_title_plan(plan)?;
    Ok(ValidatedThemeSafeTitlePlan { plan })
}
