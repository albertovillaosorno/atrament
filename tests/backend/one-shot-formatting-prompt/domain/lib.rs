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
//   - Regression evidence for self-contained initial formatting-prompt inputs.
// - Must-Not:
//   - Serialize text, compute identity, parse responses, access clipboard, or
//     define semantic command behavior.
// - Allows:
//   - Inputs: Deterministic caller-owned formatting-prompt fixtures.
//   - Outputs: Assertions that every frozen request input remains explicit.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Prompt serialization or response parsing gains independent fixtures.
// - Merge-When:
//   - Prompt evidence moves into an application-level formatting harness.
// - Summary:
//   - Proves the one-shot request is structurally self-contained.
// - Description:
//   - Covers task/source, constraints, protocol, prompt identity, and version.
// - Usage:
//   - Compile directly against the one-shot-formatting-prompt domain.
// - Defaults:
//   - No hidden chat state or browser state participates in the value.
//
use atrament_one_shot_formatting_prompt::{
    FormattingPromptConstraintInputs, FormattingPromptProtocolInputs,
    FORMATTING_PROMPT_VERSION, FormattingPromptSourceInputs,
    OneShotFormattingPrompt,
};

#[test]
fn current_prompt_version_is_owned_by_the_prompt_domain() {
    assert_eq!(FORMATTING_PROMPT_VERSION, "atrament.prompt/1");
}

#[test]
fn one_shot_prompt_retains_every_frozen_self_contained_input() {
    let prompt = OneShotFormattingPrompt {
        constraints: FormattingPromptConstraintInputs {
            output_targets: ["pdf", "live"],
            paper_constraints: "blank-a4",
            style_constraints: "sober-single-pen",
        },
        identity: "prompt-identity-9",
        protocol: FormattingPromptProtocolInputs {
            diagnostic_expectations: "report-ambiguity-and-invalid-structure",
            provenance_rules: "retain-source-and-claim-provenance",
            return_envelope: "candidate-envelope-v3",
            semantic_format: "semantic-notebook-format-v8",
            source_rules: "provided-derived-cited-unverified",
        },
        source: FormattingPromptSourceInputs {
            source_material: "complete source\nwith formula: x^2",
            task: "organize into study notes",
        },
        version: "prompt-v4",
    };

    assert_eq!(prompt.source.task, "organize into study notes");
    assert_eq!(
        prompt.source.source_material,
        "complete source\nwith formula: x^2",
    );
    assert_eq!(prompt.constraints.paper_constraints, "blank-a4");
    assert_eq!(prompt.constraints.style_constraints, "sober-single-pen");
    assert_eq!(prompt.constraints.output_targets, ["pdf", "live"]);
    assert_eq!(prompt.protocol.semantic_format, "semantic-notebook-format-v8");
    assert_eq!(prompt.protocol.return_envelope, "candidate-envelope-v3");
    assert_eq!(
        prompt.protocol.source_rules,
        "provided-derived-cited-unverified",
    );
    assert_eq!(
        prompt.protocol.provenance_rules,
        "retain-source-and-claim-provenance",
    );
    assert_eq!(
        prompt.protocol.diagnostic_expectations,
        "report-ambiguity-and-invalid-structure",
    );
    assert_eq!(prompt.identity, "prompt-identity-9");
    assert_eq!(prompt.version, "prompt-v4");
}

#[test]
fn prompt_identity_and_version_are_data_not_browser_side_effects() {
    let prompt = OneShotFormattingPrompt {
        constraints: (),
        identity: 41_u64,
        protocol: (),
        source: (),
        version: 7_u16,
    };
    let unchanged = prompt.clone();
    assert_eq!(unchanged, prompt);
    assert_eq!(unchanged.identity, 41);
    assert_eq!(unchanged.version, 7);
}
