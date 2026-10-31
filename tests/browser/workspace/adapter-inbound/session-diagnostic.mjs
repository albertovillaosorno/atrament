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

test("diagnostic parser rejects arrays posing as records", () => {
    const ordinaryItem = { code: "atrament.example.condition" };
    const arrayRoot = Object.assign([], {
        version: DIAGNOSTIC_VERSION,
        completeness: "complete",
        items: [ordinaryItem],
    });
    const arrayItem = Object.assign([], {
        code: "atrament.example.condition",
    });
    const objectWithArrayItem = {
        version: DIAGNOSTIC_VERSION,
        completeness: "complete",
        items: [arrayItem],
    };
    for (const value of [arrayRoot, objectWithArrayItem]) {
        assert.equal(referenceDiagnosticSet(value), null);
        assert.equal(parseDiagnosticSet(value), null);
    }
});

test("diagnostic parser rejects sparse and inherited diagnostic slots", () => {
    const validItem = { code: "atrament.example.condition" };
    const sparse = Array(1);
    const trailingHole = [validItem, ,];
    const inherited = Array(1);
    const prototype = Object.create(Array.prototype);
    prototype[0] = validItem;
    Object.setPrototypeOf(inherited, prototype);
    for (const items of [sparse, trailingHole, inherited]) {
        const value = {
            version: DIAGNOSTIC_VERSION,
            completeness: "complete",
            items,
        };
        assert.equal(referenceDiagnosticSet(value), null);
        assert.equal(parseDiagnosticSet(value), null);
    }
});

test("diagnostic admission limits item work before inspecting entries", () => {
    const item = { code: "atrament.example.condition" };
    const payload = (items) => ({
        version: DIAGNOSTIC_VERSION,
        completeness: "complete",
        items,
    });
    const atLimit = Array.from({ length: 128 }, () => item);
    assert.equal(parseDiagnosticSet(payload(atLimit))?.items.length, 128);
    assert.equal(parseDiagnosticSet(payload([...atLimit, item])), null);

    // The descriptor trap stands in for an in-process array whose reported
    // length is huge. Rejection must not walk its attacker-supplied slots.
    let inspected = 0;
    const hugeItems = new Proxy(new Array(1_000_000_000), {
        getOwnPropertyDescriptor(_target, key) {
            if (key !== "length") {
                inspected += 1;
                if (inspected > 8) {
                    throw new Error("diagnostic parser walked oversized items");
                }
                return {
                    configurable: true,
                    enumerable: true,
                    writable: true,
                    value: item,
                };
            }
            return Reflect.getOwnPropertyDescriptor(_target, key);
        },
    });
    assert.equal(parseDiagnosticSet(payload(hugeItems)), null);
    assert.equal(inspected, 0, "oversize must refuse before reading slots");

    // An actual Array has an own data `length`, even when wrapped in an
    // in-process Proxy that would execute code on ordinary property access.
    let lengthReads = 0;
    const guardedLength = new Proxy([item], {
        get(target, key, receiver) {
            if (key === "length") {
                lengthReads += 1;
                return Reflect.get(target, key, receiver);
            }
            return Reflect.get(target, key, receiver);
        },
    });
    const guardedMetadata = parseDiagnosticSet(payload(guardedLength));
    assert.equal(guardedMetadata?.completeness, "complete");
    assert.notEqual(guardedMetadata?.items, guardedLength,
        "admitted metadata must not retain a caller-owned array");
    assert.equal(guardedMetadata?.items.length, 1);
    assert.notEqual(guardedMetadata?.items[0], item);
    assert.deepEqual(guardedMetadata?.items[0], item);
    assert.equal(lengthReads, 0, "array length admission must read own data");

    // Changes to an original array slot after validation cannot change the
    // route-specific diagnostic observed by handshake or draft consumers.
    const mutableItems = [item];
    const admitted = parseDiagnosticSet(payload(mutableItems));
    mutableItems[0] = { code: "unrelated" };
    assert.notEqual(admitted?.items[0], item);
    assert.deepEqual(admitted?.items[0], item);
});

test("diagnostic admission snapshots each verified item object", () => {
    const item = {
        code: "atrament.example.condition",
        dimension: "prompt",
        expected: "atrament.prompt/1",
        detail: 42,
    };
    const parsed = parseDiagnosticSet({
        version: DIAGNOSTIC_VERSION,
        completeness: "complete",
        items: [item],
    });
    assert.notEqual(parsed?.items[0], item);
    item.code = "changed";
    item.dimension = "renderer";
    item.expected = "changed";
    item.detail = 0;
    assert.deepEqual(parsed?.items[0], {
        code: "atrament.example.condition",
        dimension: "prompt",
        expected: "atrament.prompt/1",
        detail: 42,
    });
});

test("diagnostic item snapshots reject unbounded or accessor fields", () => {
    const payload = (item) => ({
        version: DIAGNOSTIC_VERSION,
        completeness: "complete",
        items: [item],
    });
    const atLimit = { code: "atrament.example.condition" };
    for (let index = 1; index < 128; index += 1) {
        atLimit[`detail-${index}`] = index;
    }
    assert.equal(Object.keys(atLimit).length, 128);
    assert.equal(parseDiagnosticSet(payload(atLimit))?.items.length, 1);
    assert.equal(
        parseDiagnosticSet(payload({ ...atLimit, overflow: true })), null,
    );

    let getterReads = 0;
    const withAccessor = { code: "atrament.example.condition" };
    Object.defineProperty(withAccessor, "detail", {
        enumerable: true,
        get() {
            getterReads += 1;
            return "not decoded JSON";
        },
    });
    assert.equal(parseDiagnosticSet(payload(withAccessor)), null);
    assert.equal(getterReads, 0);
    assert.equal(parseDiagnosticSet(payload({
        code: "atrament.example.condition",
        [Symbol("not JSON")]: 1,
    })), null);

    const throwingKeys = new Proxy({
        code: "atrament.example.condition",
    }, {
        ownKeys() { throw Error("invalid item keys"); },
    });
    assert.equal(parseDiagnosticSet(payload(throwingKeys)), null);
});

test("diagnostic parser refuses descriptor and revoked Proxy traps", () => {
    const throwingRoot = new Proxy({}, {
        getOwnPropertyDescriptor() {
            throw Error("unexpected root descriptor");
        },
    });
    const throwingSlot = new Proxy([{ code: "valid" }], {
        getOwnPropertyDescriptor(target, field) {
            if (field === "0") { throw Error("unexpected array slot"); }
            return Reflect.getOwnPropertyDescriptor(target, field);
        },
    });
    const { proxy: revoked, revoke } = Proxy.revocable({}, {});
    revoke();
    for (const payload of [
        throwingRoot,
        revoked,
        {
            version: DIAGNOSTIC_VERSION,
            completeness: "complete",
            items: throwingSlot,
        },
    ]) {
        assert.equal(parseDiagnosticSet(payload), null);
    }
});

test("diagnostic metadata requires own wire fields", () => {
    const validItem = { code: "atrament.example.condition" };
    const valid = {
        version: DIAGNOSTIC_VERSION,
        completeness: "complete",
        items: [validItem],
    };
    const inheritedRoot = Object.create(valid);
    const inheritedCompleteness = Object.assign(
        Object.create({ completeness: "complete" }),
        { version: DIAGNOSTIC_VERSION, items: [validItem] },
    );
    const inheritedItems = Object.assign(
        Object.create({ items: [validItem] }),
        { version: DIAGNOSTIC_VERSION, completeness: "complete" },
    );
    const inheritedItem = Object.assign(
        Object.create(validItem), { detail: 42 },
    );
    for (const malformed of [
        inheritedRoot,
        inheritedCompleteness,
        inheritedItems,
        { ...valid, items: [inheritedItem] },
    ]) {
        assert.equal(parseDiagnosticSet(malformed), null);
    }
    const ownNullPrototype = Object.assign(Object.create(null), valid);
    assert.deepEqual(parseDiagnosticSet(ownNullPrototype), {
        completeness: "complete",
        items: [validItem],
    });
});

test("diagnostic parser refuses accessor-backed wire fields", () => {
    const item = { code: "atrament.example.condition" };
    const valid = {
        version: DIAGNOSTIC_VERSION,
        completeness: "complete",
        items: [item],
    };
    let getterCalls = 0;
    const accessor = (object, field, value = valid[field]) => {
        Object.defineProperty(object, field, {
            enumerable: true,
            get() {
                getterCalls += 1;
                return value;
            },
        });
        return object;
    };
    for (const field of ["version", "completeness", "items"]) {
        const candidate = { ...valid };
        assert.equal(parseDiagnosticSet(accessor(candidate, field)), null);
    }
    const itemWithGetter = accessor({}, "code", item.code);
    assert.equal(
        parseDiagnosticSet({ ...valid, items: [itemWithGetter] }), null,
    );
    const itemsWithGetter = [item];
    Object.defineProperty(itemsWithGetter, "0", {
        configurable: true,
        get() {
            getterCalls += 1;
            return item;
        },
    });
    assert.equal(
        parseDiagnosticSet({ ...valid, items: itemsWithGetter }), null,
    );
    assert.equal(getterCalls, 0, "wire accessors must never execute");
});

function referenceDiagnosticSet(value) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        return null;
    }
    if (
        !Object.hasOwn(value, "version")
        || !Object.hasOwn(value, "completeness")
        || !Object.hasOwn(value, "items")
        || value.version !== DIAGNOSTIC_VERSION
        || !["complete", "incomplete"].includes(value.completeness)
        || !Array.isArray(value.items)
    ) {
        return null;
    }
    for (const [index, item] of value.items.entries()) {
        if (!Object.hasOwn(value.items, index)) {
            return null;
        }
        if (
            typeof item !== "object"
            || item === null
            || Array.isArray(item)
            || !Object.hasOwn(item, "code")
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
