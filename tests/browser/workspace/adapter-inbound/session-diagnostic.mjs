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

function referenceDiagnosticSet(value) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        return null;
    }
    if (
        value.version !== DIAGNOSTIC_VERSION
        || !["complete", "incomplete"].includes(value.completeness)
        || !Array.isArray(value.items)
    ) {
        return null;
    }
    for (const item of value.items) {
        if (
            typeof item !== "object"
            || item === null
            || Array.isArray(item)
            || typeof item.code !== "string"
            || item.code === ""
        ) {
            return null;
        }
    }
    return {
        completeness: value.completeness,
        items: value.items,
    };
}

function nextDiagnosticMutation(state) {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return state >>> 0;
}

test("generated diagnostic sets match fail-closed reference", () => {
    const versions = [DIAGNOSTIC_VERSION, "atrament.diagnostic/0", "", null];
    const completeness = ["complete", "incomplete", "unknown", "", null];
    const itemShapes = [
        { code: "atrament.example.condition" },
        { code: "another.code", detail: 42 },
        { code: "" },
        { code: 7 },
        {},
        null,
        [],
        "text",
    ];
    const seenVersions = new Set();
    const seenCompleteness = new Set();
    const seenItemShapes = new Set();
    const seenItemCounts = new Set();
    let state = 0x5eed_d1a6;
    for (let caseIndex = 0; caseIndex < 4_096; caseIndex += 1) {
        state = nextDiagnosticMutation(state);
        const versionIndex = state % versions.length;
        seenVersions.add(versionIndex);
        state = nextDiagnosticMutation(state);
        const completenessIndex = state % completeness.length;
        seenCompleteness.add(completenessIndex);
        state = nextDiagnosticMutation(state);
        const itemCount = state % 4;
        seenItemCounts.add(itemCount);
        const items = [];
        for (let itemIndex = 0; itemIndex < itemCount; itemIndex += 1) {
            state = nextDiagnosticMutation(state);
            const shapeIndex = state % itemShapes.length;
            seenItemShapes.add(shapeIndex);
            const shape = itemShapes[shapeIndex];
            items.push(
                shape !== null
                    && typeof shape === "object"
                    && !Array.isArray(shape)
                    ? { ...shape }
                    : shape,
            );
        }
        const payload = {
            version: versions[versionIndex],
            completeness: completeness[completenessIndex],
            items,
            ignored: caseIndex,
        };
        assert.deepEqual(
            parseDiagnosticSet(payload),
            referenceDiagnosticSet(payload),
            `generated diagnostic case ${caseIndex}`,
        );
    }
    assert.equal(seenVersions.size, versions.length);
    assert.equal(seenCompleteness.size, completeness.length);
    assert.equal(seenItemShapes.size, itemShapes.length);
    assert.equal(seenItemCounts.size, 4);
});
