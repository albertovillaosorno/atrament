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
    isResourceLimit,
} = await import(`${GENERATED_ROOT}session-draft.js`);

test("draft targets are same-origin and contain no credential", () => {
    const secret = "a".repeat(64);
    for (const field of ["candidate", "source", "task"]) {
        const target = draftMutationTarget(field);
        assert.equal(target, `./api/session/${field}`);
        assert.equal(target.includes(secret), false);
        assert.equal(target.includes("session="), false);
    }
});

test("draft headers carry only bearer credential and text media type", () => {
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
