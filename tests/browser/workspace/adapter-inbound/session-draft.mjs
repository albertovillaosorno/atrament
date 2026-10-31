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
    isDraftReadContentType,
    isDraftDiagnosticContentType,
    decodeDraftReadBytes,
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

test("draft reads admit only declared UTF-8 plain text", () => {
    for (const valid of [
        "text/plain; charset=utf-8",
        "Text/Plain; Charset=UTF-8",
        " text/plain; charset=utf-8 ",
    ]) {
        assert.equal(isDraftReadContentType(valid), true, valid);
    }
    for (const invalid of [
        null,
        "",
        "text/plain",
        "text/html; charset=utf-8",
        "application/json; charset=utf-8",
        "text/plain; charset=iso-8859-1",
        "text/plain; charset=utf-8; extra=yes",
    ]) {
        assert.equal(isDraftReadContentType(invalid), false, String(invalid));
    }
});

test("draft byte decoder preserves BOM and refuses corrupt UTF-8", () => {
    const enc = new TextEncoder();
    for (const value of ["", "plain", "\uFEFFprefix", "niño 📝", "a\uFFFDb"]) {
        assert.equal(decodeDraftReadBytes(enc.encode(value).buffer), value);
    }
    for (const bytes of [
        Uint8Array.of(0xc3, 0x28),
        Uint8Array.of(0xe2, 0x82),
        Uint8Array.of(0xed, 0xa0, 0x80),
    ]) {
        assert.equal(decodeDraftReadBytes(bytes.buffer), null);
    }
});

test("draft resource-limit diagnostics require UTF-8 JSON media", () => {
    for (const valid of [
        "application/json; charset=utf-8",
        "Application/JSON; Charset=UTF-8",
        " application/json; charset=utf-8 ",
    ]) {
        assert.equal(isDraftDiagnosticContentType(valid), true, valid);
    }
    for (const invalid of [
        null,
        "",
        "application/json",
        "text/html; charset=utf-8",
        "text/plain; charset=utf-8",
        "application/json; charset=latin1",
        "application/json; charset=utf-8; extra=yes",
    ]) {
        assert.equal(
            isDraftDiagnosticContentType(invalid), false, String(invalid),
        );
    }
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

test("draft parses diagnostic item fields without rereading proxies", () => {
    let codeReads = 0;
    const item = new Proxy({
        code: "atrament.session-draft.resource-limit",
    }, {
        getOwnPropertyDescriptor(target, key) {
            if (key === "code") {
                codeReads += 1;
                if (codeReads > 1) {
                    throw Error("draft reread untrusted diagnostic code");
                }
            }
            return Reflect.getOwnPropertyDescriptor(target, key);
        },
    });
    assert.equal(isResourceLimit({
        error: "resource_limit",
        diagnostics: {
            version: "atrament.diagnostic/1",
            completeness: "complete",
            items: [item],
        },
    }), true);
    assert.equal(codeReads, 1);
});

test("draft resource-limit parser refuses throwing Proxy metadata", () => {
    const throwingError = new Proxy({}, {
        getOwnPropertyDescriptor() { throw Error("draft error trap"); },
    });
    const { proxy: revoked, revoke } = Proxy.revocable({}, {});
    revoke();
    const throwingItems = new Proxy([{
        code: "atrament.session-draft.resource-limit",
    }], {
        getOwnPropertyDescriptor(target, key) {
            if (key === "0") { throw Error("draft diagnostic item trap"); }
            return Reflect.getOwnPropertyDescriptor(target, key);
        },
    });
    for (const value of [
        throwingError,
        revoked,
        {
            error: "resource_limit",
            diagnostics: {
                version: "atrament.diagnostic/1",
                completeness: "complete",
                items: throwingItems,
            },
        },
    ]) {
        assert.equal(isResourceLimit(value), false);
    }
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

test("draft resource-limit ignores input-array property getters", () => {
    const item = { code: "atrament.session-draft.resource-limit" };
    let reads = 0;
    const items = new Proxy([item], {
        get(target, key, receiver) {
            if (key === "0" || key === "length") {
                reads += 1;
            }
            return Reflect.get(target, key, receiver);
        },
    });
    assert.equal(isResourceLimit({
        error: "resource_limit",
        diagnostics: {
            version: "atrament.diagnostic/1",
            completeness: "complete",
            items,
        },
    }), true);
    assert.equal(reads, 0, "validated array slots must be owned snapshots");
});

test("draft resource-limit response refuses array-shaped roots", () => {
    const forged = Object.assign([], {
        error: "resource_limit",
        diagnostics: {
            version: "atrament.diagnostic/1",
            completeness: "complete",
            items: [{ code: "atrament.session-draft.resource-limit" }],
        },
    });
    assert.equal(referenceResourceLimit(forged), false);
    assert.equal(isResourceLimit(forged), false);
});

test("draft resource-limit requires own response error identity", () => {
    const metadata = {
        version: "atrament.diagnostic/1",
        completeness: "complete",
        items: [{ code: "atrament.session-draft.resource-limit" }],
    };
    const inheritedError = Object.assign(
        Object.create({ error: "resource_limit" }),
        { diagnostics: metadata },
    );
    const inheritedDiagnostics = Object.assign(
        Object.create({ diagnostics: metadata }),
        { error: "resource_limit" },
    );
    for (const value of [inheritedError, inheritedDiagnostics]) {
        assert.equal(referenceResourceLimit(value), false);
        assert.equal(isResourceLimit(value), false);
    }
});

test("draft resource-limit parser refuses accessor-backed fields", () => {
    const diagnostic = { code: "atrament.session-draft.resource-limit" };
    const metadata = {
        version: "atrament.diagnostic/1",
        completeness: "complete",
        items: [diagnostic],
    };
    const valid = { error: "resource_limit", diagnostics: metadata };
    let getterCalls = 0;
    const accessor = (object, field, value) => {
        Object.defineProperty(object, field, {
            enumerable: true,
            get() {
                getterCalls += 1;
                return value;
            },
        });
        return object;
    };
    const candidates = [
        accessor({ ...valid }, "error", "resource_limit"),
        accessor({ error: "resource_limit" }, "diagnostics", metadata),
        { ...valid, diagnostics: accessor(
            { ...metadata }, "version", "atrament.diagnostic/1",
        ) },
        { ...valid, diagnostics: {
            ...metadata,
            items: [accessor({ ...diagnostic }, "code", diagnostic.code)],
        } },
    ];
    for (const candidate of candidates) {
        assert.equal(isResourceLimit(candidate), false);
    }
    assert.equal(getterCalls, 0, "wire accessors must never execute");
});

function referenceResourceLimit(value) {
    if (
        typeof value !== "object"
        || value === null
        || Array.isArray(value)
        || !Object.hasOwn(value, "error")
        || !Object.hasOwn(value, "diagnostics")
        || value.error !== "resource_limit"
    ) {
        return false;
    }
    const diagnostics = value.diagnostics;
    if (
        typeof diagnostics !== "object"
        || diagnostics === null
        || Array.isArray(diagnostics)
        || !["version", "completeness", "items"].every(
            (field) => Object.hasOwn(diagnostics, field),
        )
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
        && Object.hasOwn(item, "code")
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
