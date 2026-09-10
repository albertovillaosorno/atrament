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
//   - Regression evidence for browser session credential fragment syntax.
// - Must-Not:
//   - Persist credentials, access browser state, or perform network requests.
// - Allows:
//   - Inputs: Deterministic URL fragment fixtures.
//   - Outputs: Assertions over exact credential admission and rejection.
//   - Side effects: None.
// - Split-When:
//   - Browser credential handoff supports another transport representation.
// - Merge-When:
//   - Browser startup no longer receives a URL-fragment credential.
// - Summary:
//   - Verifies exact parsing of the disposable browser credential fragment.
// - Description:
//   - Runs directly under Node against the tracked generated parser module.
// - Usage:
//   - Execute through the repository frontend test script.
// - Defaults:
//   - Accepts only 64 lowercase hexadecimal characters after `#session=`.
//
import assert from "node:assert/strict";
import test from "node:test";

const GENERATED_ROOT =
    "../../../../src/browser/workspace/adapter-inbound/generated/";
const { sessionSecretFromFragment } =
    await import(`${GENERATED_ROOT}session-fragment.js`);

test("session fragment accepts one exact lowercase credential", () => {
    const secret = "a".repeat(64);
    assert.equal(sessionSecretFromFragment(`#session=${secret}`), secret);
});

test("session fragment rejects malformed or alternate representations", () => {
    for (const fragment of [
        "",
        "#session=",
        `#session=${"a".repeat(63)}`,
        `#session=${"a".repeat(65)}`,
        `#session=${"A".repeat(64)}`,
        `#other=${"a".repeat(64)}`,
    ]) {
        assert.equal(sessionSecretFromFragment(fragment), null);
    }
});


function referenceSessionSecret(fragment) {
    const prefix = "#session=";
    if (!fragment.startsWith(prefix)) {
        return null;
    }
    const candidate = fragment.slice(prefix.length);
    if (candidate.length !== 64) {
        return null;
    }
    for (let index = 0; index < candidate.length; index += 1) {
        const code = candidate.charCodeAt(index);
        const decimal = code >= 0x30 && code <= 0x39;
        const lowerHex = code >= 0x61 && code <= 0x66;
        if (!decimal && !lowerHex) {
            return null;
        }
    }
    return candidate;
}

function nextMutationValue(state) {
    return (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
}

test("generated fragment mutations match exact credential oracle", () => {
    const canonical = `#session=${"0123456789abcdef".repeat(4)}`;
    const replacements = [
        "0", "a", "f", "A", "F", "&", "=", "%", "#", "é", "😀", "\0",
    ];
    let state = 0x5eed_2026;
    for (let caseIndex = 0; caseIndex < 4_096; caseIndex += 1) {
        state = nextMutationValue(state);
        const operation = state % 4;
        state = nextMutationValue(state);
        const position = state % (canonical.length + 1);
        state = nextMutationValue(state);
        const token = replacements[state % replacements.length];
        let fragment;
        if (operation === 0 && position < canonical.length) {
            fragment = canonical.slice(0, position)
                + token
                + canonical.slice(position + 1);
        } else if (operation === 1) {
            fragment = canonical.slice(0, position)
                + token
                + canonical.slice(position);
        } else if (operation === 2 && position < canonical.length) {
            fragment = canonical.slice(0, position)
                + canonical.slice(position + 1);
        } else {
            const width = Math.min(3, canonical.length - position);
            fragment = canonical.slice(0, position)
                + canonical.slice(position, position + width)
                + canonical.slice(position);
        }
        const expected = referenceSessionSecret(fragment);
        assert.equal(
            sessionSecretFromFragment(fragment),
            expected,
            `generated fragment case ${caseIndex}`,
        );
        assert.equal(
            sessionSecretFromFragment(fragment),
            expected,
            `repeat generated fragment case ${caseIndex}`,
        );
    }
});
