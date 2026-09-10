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
//   - Regression evidence for browser handshake headers and payload validation.
// - Must-Not:
//   - Perform network requests, persist credentials, or mutate browser state.
// - Allows:
//   - Inputs: Deterministic credentials and decoded handshake payload fixtures.
//   - Outputs: Assertions over exact headers and typed compatibility outcomes.
//   - Side effects: None.
// - Split-When:
//   - Browser compatibility gains independently versioned optional features.
// - Merge-When:
//   - Browser startup no longer performs a compatibility handshake.
// - Summary:
//   - Verifies the browser side of the six-version handshake contract.
// - Description:
//   - Runs directly under Node against the tracked generated browser module.
// - Usage:
//   - Execute through the repository frontend test script.
// - Defaults:
//   - Rejects any drift in one of the six required backend version identities.
//
import assert from "node:assert/strict";
import test from "node:test";

const GENERATED_ROOT =
    "../../../../src/browser/workspace/adapter-inbound/generated/";
const { handshakeHeaders, parseHandshakePayload } =
    await import(`${GENERATED_ROOT}session-handshake.js`);

const CURRENT_VERSIONS = {
    capability: "atrament.capability/1",
    product: "0.1.0",
    profile: "atrament.profile/1",
    prompt: "atrament.prompt/1",
    protocol: "atrament.runtime/1",
    renderer: "atrament.renderer/1",
};

test("handshake headers bind the credential and six versions", () => {
    const secret = "a".repeat(64);
    const headers = handshakeHeaders(secret);
    assert.deepEqual(headers, {
        Authorization: `Bearer ${secret}`,
        "X-Atrament-Capability-Version": CURRENT_VERSIONS.capability,
        "X-Atrament-Product-Version": CURRENT_VERSIONS.product,
        "X-Atrament-Profile-Version": CURRENT_VERSIONS.profile,
        "X-Atrament-Prompt-Version": CURRENT_VERSIONS.prompt,
        "X-Atrament-Protocol-Version": CURRENT_VERSIONS.protocol,
        "X-Atrament-Renderer-Version": CURRENT_VERSIONS.renderer,
    });
});

test("compatible payload requires all six exact backend versions", () => {
    assert.deepEqual(
        parseHandshakePayload({
            result: "compatible",
            versions: CURRENT_VERSIONS,
        }),
        { kind: "compatible" },
    );
    for (const dimension of Object.keys(CURRENT_VERSIONS)) {
        const versions = {
            ...CURRENT_VERSIONS,
            [dimension]: `${CURRENT_VERSIONS[dimension]}-drift`,
        };
        assert.deepEqual(
            parseHandshakePayload({ result: "compatible", versions }),
            { kind: "invalid" },
        );
    }
});

test("typed incompatibility requires the stable diagnostic shape", () => {
    assert.deepEqual(
        parseHandshakePayload({
            result: "incompatible",
            diagnostics: {
                version: "atrament.diagnostic/1",
                completeness: "complete",
                items: [{
                    code: "atrament.handshake.version-mismatch",
                    dimension: "prompt",
                    expected: CURRENT_VERSIONS.prompt,
                }],
            },
        }),
        {
            kind: "incompatible",
            dimension: "prompt",
            expected: CURRENT_VERSIONS.prompt,
        },
    );
    assert.deepEqual(
        parseHandshakePayload({
            result: "incompatible",
            diagnostics: {
                version: "atrament.diagnostic/1",
                completeness: "complete",
                items: [{
                    code: "unknown",
                    dimension: "prompt",
                    expected: CURRENT_VERSIONS.prompt,
                }],
            },
        }),
        { kind: "invalid" },
    );
});

test("typed incompatibility requires the dimension's current version", () => {
    for (const expected of [
        "arbitrary-version",
        CURRENT_VERSIONS.renderer,
    ]) {
        assert.deepEqual(
            parseHandshakePayload({
                result: "incompatible",
                diagnostics: {
                    version: "atrament.diagnostic/1",
                    completeness: "complete",
                    items: [{
                        code: "atrament.handshake.version-mismatch",
                        dimension: "prompt",
                        expected,
                    }],
                },
            }),
            { kind: "invalid" },
        );
    }
});

test("typed incompatibility rejects diagnostic namespace drift", () => {
    assert.deepEqual(
        parseHandshakePayload({
            result: "incompatible",
            diagnostics: {
                version: "atrament.diagnostic/0",
                completeness: "complete",
                items: [{
                    code: "atrament.handshake.version-mismatch",
                    dimension: "prompt",
                    expected: CURRENT_VERSIONS.prompt,
                }],
            },
        }),
        { kind: "invalid" },
    );
});

test(
    "incomplete handshake diagnostics preserve known incompatibility",
    () => {
    const outcome = parseHandshakePayload({
        result: "incompatible",
        diagnostics: {
            version: "atrament.diagnostic/1",
            completeness: "incomplete",
            items: [{
                code: "atrament.handshake.version-mismatch",
                dimension: "prompt",
                expected: CURRENT_VERSIONS.prompt,
            }],
        },
    });
        assert.deepEqual(outcome, {
            kind: "incompatible",
            dimension: "prompt",
            expected: CURRENT_VERSIONS.prompt,
        });
    },
);

function referenceHandshakeOutcome(value) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        return { kind: "invalid" };
    }
    if (value.result === "compatible") {
        const versions = value.versions;
        if (
            typeof versions === "object"
            && versions !== null
            && !Array.isArray(versions)
            && Object.entries(CURRENT_VERSIONS).every(
                ([dimension, expected]) => versions[dimension] === expected,
            )
        ) {
            return { kind: "compatible" };
        }
        return { kind: "invalid" };
    }
    if (value.result !== "incompatible") {
        return { kind: "invalid" };
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
        return { kind: "invalid" };
    }
    const item = diagnostics.items[0];
    if (
        typeof item !== "object"
        || item === null
        || Array.isArray(item)
        || item.code !== "atrament.handshake.version-mismatch"
        || !Object.hasOwn(CURRENT_VERSIONS, item.dimension)
        || item.expected !== CURRENT_VERSIONS[item.dimension]
    ) {
        return { kind: "invalid" };
    }
    return {
        kind: "incompatible",
        dimension: item.dimension,
        expected: item.expected,
    };
}

function nextHandshakeMutation(state) {
    return (Math.imul(state, 1_103_515_245) + 12_345) >>> 0;
}

test("generated handshake payloads match fail-closed reference", () => {
    const dimensions = Object.keys(CURRENT_VERSIONS);
    const results = ["compatible", "incompatible", "unknown", "", null, 7];
    const completeness = ["complete", "incomplete", "unknown", null];
    const codes = [
        "atrament.handshake.version-mismatch",
        "unknown",
        "",
        null,
    ];
    let state = 0x5eed_4a11;
    for (let caseIndex = 0; caseIndex < 4_096; caseIndex += 1) {
        state = nextHandshakeMutation(state);
        const result = results[state % results.length];
        state = nextHandshakeMutation(state);
        const dimension = dimensions[state % dimensions.length];
        state = nextHandshakeMutation(state);
        const versionsMode = state % 4;
        state = nextHandshakeMutation(state);
        const expectedMode = state % 3;
        state = nextHandshakeMutation(state);
        const completenessValue = completeness[state % completeness.length];
        state = nextHandshakeMutation(state);
        const code = codes[state % codes.length];
        const versions = versionsMode === 0
            ? { ...CURRENT_VERSIONS }
            : versionsMode === 1
                ? { ...CURRENT_VERSIONS, [dimension]: "drift" }
                : versionsMode === 2
                    ? null
                    : [];
        const expected = expectedMode === 0
            ? CURRENT_VERSIONS[dimension]
            : expectedMode === 1
                ? "drift"
                : null;
        const payload = {
            result,
            versions,
            diagnostics: {
                version: caseIndex % 7 === 0
                    ? "atrament.diagnostic/0"
                    : "atrament.diagnostic/1",
                completeness: completenessValue,
                items: caseIndex % 11 === 0
                    ? []
                    : [{ code, dimension, expected }],
            },
        };
        assert.deepEqual(
            parseHandshakePayload(payload),
            referenceHandshakeOutcome(payload),
            `generated handshake case ${caseIndex}`,
        );
    }
});
