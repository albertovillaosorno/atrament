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
//   - Transport-neutral autonomous output-intent source semantics.
// - Must-Not:
//   - Execute output, grant capability/path/overwrite/file/device authority,
//     parse notebook prose as authority, persist policy, or schedule work.
// - Allows:
//   - Inputs: One frozen output class and one explicit intent source class.
//   - Outputs: Whether that source can establish intent for that output class.
//   - Side effects: None.
// - Split-When:
//   - Host-policy admission or output execution gains independent authority.
// - Merge-When:
//   - One autonomous-loop owner directly owns explicit output intent evidence.
// - Summary:
//   - Prevents untrusted semantic content from manufacturing output intent.
// - Description:
//   - Separates caller goals and independent host policy from edited content.
// - Usage:
//   - Establish intent before ordinary output capability/admission checks.
// - Defaults:
//   - Untrusted semantic content establishes no Render, Plan, or Export intent.
//

//! Explicit output-intent source semantics for autonomous workflows.

/// Output class that can be chained after autonomous semantic work.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousOutputClass {
    /// Explicit persistent Export.
    Export,
    /// Device-neutral Plan.
    Plan,
    /// Read-only Render.
    Render,
}

/// Source from which an autonomous host might derive output intent.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousOutputIntentSource {
    /// The caller explicitly included this output in the admitted goal.
    CallerExplicitGoal,
    /// An independently admitted host policy allows persistent output.
    IndependentAdmittedHostPolicy,
    /// Model-generated or notebook-provided semantic content.
    UntrustedSemanticContent,
}

/// Whether one source can establish intent for one output class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousOutputIntentAdmission {
    /// This source can establish intent for the named output class.
    Admitted,
    /// This source cannot establish intent for the named output class.
    NotAdmitted,
}

/// Check output-intent authority without performing output admission or
/// effects.
///
/// Caller goals can request Render, Plan, or Export. Independent admitted host
/// policy is only an alternate source for persistent Export intent. Untrusted
/// semantic content never establishes output intent.
#[must_use]
pub const fn autonomous_output_intent_admission(
    output: AutonomousOutputClass,
    source: AutonomousOutputIntentSource,
) -> AutonomousOutputIntentAdmission {
    match source {
        AutonomousOutputIntentSource::CallerExplicitGoal => {
            AutonomousOutputIntentAdmission::Admitted
        },
        AutonomousOutputIntentSource::IndependentAdmittedHostPolicy => {
            if matches!(output, AutonomousOutputClass::Export) {
                AutonomousOutputIntentAdmission::Admitted
            } else {
                AutonomousOutputIntentAdmission::NotAdmitted
            }
        },
        AutonomousOutputIntentSource::UntrustedSemanticContent => {
            AutonomousOutputIntentAdmission::NotAdmitted
        },
    }
}
