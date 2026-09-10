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
//   - Regression evidence for browser session draft request metadata.
// - Must-Not:
//   - Perform network requests, persist credentials, or mutate browser state.
// - Allows:
//   - Inputs: Deterministic draft fields and one deterministic credential.
//   - Outputs: Assertions over same-origin targets and explicit request
//     headers.
//   - Side effects: None.
// - Split-When:
//   - Draft browser transport gains independently testable request semantics.
// - Merge-When:
//   - Session draft metadata is tested by another browser transport fixture.
// - Summary:
//   - Verifies protected task, source, and raw-response browser request
//     metadata.
// - Description:
//   - Ensures credentials remain headers and never become request-target data.
// - Usage:
//   - Execute through the repository frontend test script.
// - Defaults:
//   - Covers all three first-release pre-acceptance draft fields.
//
import assert from "node:assert/strict";
import test from "node:test";

const GENERATED_ROOT =
    "../../../../src/browser/workspace/adapter-inbound/generated/";
const {
    draftMutationHeaders,
    draftMutationTarget,
    draftReadHeaders,
    draftReadTarget,
    isResourceLimit,
} = await import(`${GENERATED_ROOT}session-draft.js`);

test("draft targets are same-origin and contain no credential", () => {
    const secret = "a".repeat(64);
    for (const field of ["candidate", "source", "task"]) {
        for (const target of [
            draftReadTarget(field),
            draftMutationTarget(field),
        ]) {
            assert.equal(target, `./api/session/${field}`);
            assert.equal(target.includes(secret), false);
            assert.equal(target.includes("session="), false);
        }
    }
});

test("draft read headers carry only the bearer credential", () => {
    const secret = "a".repeat(64);
    assert.deepEqual(draftReadHeaders(secret), {
        Authorization: `Bearer ${secret}`,
    });
});

test("draft mutation headers add only the text media type", () => {
    const secret = "a".repeat(64);
    assert.deepEqual(draftMutationHeaders(secret), {
        Authorization: `Bearer ${secret}`,
        "Content-Type": "text/plain; charset=utf-8",
    });
});

test("draft resource limit requires current shared diagnostic metadata", () => {
    const valid = {
        error: "resource_limit",
        diagnostics: {
            version: "atrament.diagnostic/1",
            completeness: "complete",
            items: [{ code: "atrament.session-draft.resource-limit" }],
        },
    };
    assert.equal(isResourceLimit(valid), true);
    assert.equal(
        isResourceLimit({
            ...valid,
            diagnostics: {
                ...valid.diagnostics,
                version: "atrament.diagnostic/0",
            },
        }),
        false,
    );
    assert.equal(
        isResourceLimit({
            ...valid,
            diagnostics: {
                ...valid.diagnostics,
                items: [{ code: "unknown" }],
            },
        }),
        false,
    );
});

test("incomplete draft diagnostics still preserve known resource limit", () => {
    assert.equal(
        isResourceLimit({
            error: "resource_limit",
            diagnostics: {
                version: "atrament.diagnostic/1",
                completeness: "incomplete",
                items: [{ code: "atrament.session-draft.resource-limit" }],
            },
        }),
        true,
    );
});

function referenceResourceLimit(value) {
    if (
        typeof value !== "object"
        || value === null
        || Array.isArray(value)
        || value.error !== "resource_limit"
    ) {
        return false;
    }
    const diagnostics = value.diagnostics;
    if (
        typeof diagnostics !== "object"
        || diagnostics === null
        || Array.isArray(diagnostics)
        || diagnostics.version !== "atrament.diagnostic/1"
        || !["complete", "incomplete"].includes(diagnostics.completeness)
        || !Array.isArray(diagnostics.items)
        || diagnostics.items.length !== 1
    ) {
        return false;
    }
    const item = diagnostics.items[0];
    return typeof item === "object"
        && item !== null
        && !Array.isArray(item)
        && item.code === "atrament.session-draft.resource-limit";
}

function nextDraftMutation(state) {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return state >>> 0;
}

test("generated resource-limit payloads match fail-closed reference", () => {
    const errors = ["resource_limit", "invalid_request", "", null, 7];
    const versions = [
        "atrament.diagnostic/1", "atrament.diagnostic/0", "", null,
    ];
    const completeness = ["complete", "incomplete", "unknown", null];
    const codes = [
        "atrament.session-draft.resource-limit",
        "atrament.handshake.version-mismatch",
        "unknown",
        "",
        null,
    ];
    let state = 0x5eed_d2af;
    const seenErrors = new Set();
    const seenVersions = new Set();
    const seenCompleteness = new Set();
    const seenCodes = new Set();
    const seenItemModes = new Set();
    const seenOutcomes = new Set();
    for (let caseIndex = 0; caseIndex < 4_096; caseIndex += 1) {
        state = nextDraftMutation(state);
        const errorIndex = state % errors.length;
        seenErrors.add(errorIndex);
        const error = errors[errorIndex];
        state = nextDraftMutation(state);
        const versionIndex = state % versions.length;
        seenVersions.add(versionIndex);
        const version = versions[versionIndex];
        state = nextDraftMutation(state);
        const completenessIndex = state % completeness.length;
        seenCompleteness.add(completenessIndex);
        const completenessValue = completeness[completenessIndex];
        state = nextDraftMutation(state);
        const codeIndex = state % codes.length;
        seenCodes.add(codeIndex);
        const code = codes[codeIndex];
        state = nextDraftMutation(state);
        const itemMode = state % 4;
        seenItemModes.add(itemMode);
        const items = itemMode === 0
            ? [{ code }]
            : itemMode === 1
                ? []
                : itemMode === 2
                    ? [{ code }, { code }]
                    : [null];
        const payload = {
            error,
            diagnostics: {
                version,
                completeness: completenessValue,
                items,
            },
        };
        const expected = referenceResourceLimit(payload);
        seenOutcomes.add(expected);
        assert.equal(
            isResourceLimit(payload),
            expected,
            `generated resource-limit case ${caseIndex}`,
        );
    }
    assert.equal(seenErrors.size, errors.length);
    assert.equal(seenVersions.size, versions.length);
    assert.equal(seenCompleteness.size, completeness.length);
    assert.equal(seenCodes.size, codes.length);
    assert.equal(seenItemModes.size, 4);
    assert.deepEqual([...seenOutcomes].sort(), [false, true]);
});
