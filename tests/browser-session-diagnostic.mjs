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
    "../src/browser/workspace/adapter-inbound/generated/";
const { DIAGNOSTIC_VERSION, parseDiagnosticMetadata } =
    await import(`${GENERATED_ROOT}session-diagnostic.js`);

test("diagnostic metadata admits current version and preserves code", () => {
    assert.equal(DIAGNOSTIC_VERSION, "atrament.diagnostic/1");
    assert.deepEqual(
        parseDiagnosticMetadata({
            version: DIAGNOSTIC_VERSION,
            code: "atrament.example.condition",
        }),
        { code: "atrament.example.condition" },
    );
});

test("diagnostic metadata rejects missing or drifted namespace", () => {
    for (const value of [
        null,
        {},
        { code: "atrament.example.condition" },
        {
            version: "atrament.diagnostic/0",
            code: "atrament.example.condition",
        },
        { version: DIAGNOSTIC_VERSION, code: "" },
        { version: DIAGNOSTIC_VERSION, code: 42 },
    ]) {
        assert.equal(parseDiagnosticMetadata(value), null);
    }
});
