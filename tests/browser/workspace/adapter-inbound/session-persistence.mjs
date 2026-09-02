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
//   - Regression evidence that session UI code avoids browser persistence APIs.
// - Must-Not:
//   - Execute browser storage, perform network requests, or inspect user data.
// - Allows:
//   - Inputs: The tracked generated workspace JavaScript module.
//   - Outputs: Assertions that persistence-capable browser APIs are absent.
//   - Side effects: Reads one repository test artifact.
// - Split-When:
//   - Browser persistence policy gains independently admitted storage classes.
// - Merge-When:
//   - Another executable browser policy fixture subsumes this static evidence.
// - Summary:
//   - Guards the disposable-session browser against accidental persistence.
// - Description:
//   - Fails when generated workspace code starts using persistent browser APIs.
// - Usage:
//   - Execute through the repository frontend test script after generation.
// - Defaults:
//   - Treats Web Storage, IndexedDB, cookies, caches, and workers as forbidden.
//
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const MAIN_MODULE = new URL(
    "../../../../src/browser/workspace/adapter-inbound/generated/main.js",
    import.meta.url,
);

test("workspace module contains no browser persistence API", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const forbidden = [
        "localStorage",
        "sessionStorage",
        "indexedDB",
        "document.cookie",
        "serviceWorker",
        "CacheStorage",
        "caches.open",
    ];
    for (const capability of forbidden) {
        assert.equal(
            source.includes(capability),
            false,
            `generated workspace uses ${capability}`,
        );
    }
});
