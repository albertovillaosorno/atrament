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
//   - Regression evidence for browser diagnostic metadata version admission.
// - Must-Not:
//   - Reimplement backend diagnostic meaning or perform browser/network I/O.
// - Allows:
//   - Inputs: Decoded diagnostic metadata fixtures.
//   - Outputs: Assertions over current namespace and opaque code preservation.
//   - Side effects: None.
// - Split-When:
//   - Browser diagnostic projections gain independently versioned metadata.
// - Merge-When:
//   - Diagnostic metadata admission is covered by another browser fixture.
// - Summary:
//   - Verifies browser admission of the shared diagnostic namespace identity.
// - Description:
//   - Rejects version drift while leaving code meaning to route-specific logic.
// - Usage:
//   - Execute through the repository frontend test script.
// - Defaults:
//   - Accepts atrament.diagnostic/1 and one non-empty opaque stable code.
//
import assert from "node:assert/strict";
import test from "node:test";

const GENERATED_ROOT =
    "../../../../src/browser/workspace/adapter-inbound/generated/";
const { DIAGNOSTIC_VERSION, parseDiagnosticSet } =
    await import(`${GENERATED_ROOT}session-diagnostic.js`);

test("diagnostic set admits current version and explicit completeness", () => {
    assert.equal(DIAGNOSTIC_VERSION, "atrament.diagnostic/1");
    const complete = parseDiagnosticSet({
        version: DIAGNOSTIC_VERSION,
        completeness: "complete",
        items: [{ code: "atrament.example.condition", detail: 42 }],
    });
    assert.deepEqual(complete, {
        completeness: "complete",
        items: [{ code: "atrament.example.condition", detail: 42 }],
    });
    const incomplete = parseDiagnosticSet({
        version: DIAGNOSTIC_VERSION,
        completeness: "incomplete",
        items: [{ code: "atrament.example.condition" }],
    });
    assert.equal(incomplete?.completeness, "incomplete");
});

test("diagnostic set rejects invalid namespace, completeness, or items", () => {
    const validItem = { code: "atrament.example.condition" };
    for (const value of [
        null,
        {},
        { version: DIAGNOSTIC_VERSION, completeness: "complete", items: [] },
        {
            version: "atrament.diagnostic/0",
            completeness: "complete",
            items: [validItem],
        },
        {
            version: DIAGNOSTIC_VERSION,
            completeness: "unknown",
            items: [validItem],
        },
        {
            version: DIAGNOSTIC_VERSION,
            completeness: "complete",
            items: [{ code: "" }],
        },
    ]) {
        const parsed = parseDiagnosticSet(value);
        if (
            value !== null
            && typeof value === "object"
            && "items" in value
            && Array.isArray(value.items)
            && value.items.length === 0
        ) {
            assert.deepEqual(parsed, { completeness: "complete", items: [] });
        } else {
            assert.equal(parsed, null);
        }
    }
});
