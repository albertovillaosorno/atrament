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

import {
    sessionSecretFromFragment,
} from "../src/browser/workspace/adapter-inbound/generated/session-fragment.js";

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
