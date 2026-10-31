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
//   - Bounded response decoding and shared diagnostic metadata admission.
// - Must-Not:
//   - Reimplement diagnostic semantics, choose blocking policy, or fetch data.
// - Allows:
//   - Inputs: One backend response body or decoded adapter diagnostic value.
//   - Outputs: Bounded UTF-8/JSON text or admitted diagnostic metadata.
//   - Side effects: Reads or cancels an already-fetched response stream.
// - Split-When:
//   - Response decoding gains a separately owned transport contract.
// - Merge-When:
//   - All browser transports consume diagnostic metadata through another
//     module.
// - Summary:
//   - Bounds response bytes before admitting versioned diagnostic metadata.
// - Description:
//   - Shares strict UTF-8 response decoding without duplicating domain rules.
// - Usage:
//   - Decode response bytes and parse route-specific diagnostic metadata.
// - Defaults:
//   - Accepts first-release namespace and at most 16 KiB of control JSON.
//
export const DIAGNOSTIC_VERSION = "atrament.diagnostic/1";
// Control-plane JSON is small: a compatible handshake or one diagnostic.
// Reject misrouted giant bodies before they reach JSON.parse(). Draft text
// uses a separate, larger session-owned limit.
const MAX_CONTROL_JSON_BYTES = 16_384;
// Do not use Response.text()/json(): they buffer the entire body, replace
// malformed UTF-8 and discard an initial U+FEFF. The caller chooses the
// transport byte budget; the stream is cancelled at the first excess chunk.
export async function decodeBoundedUtf8Response(response, maxBytes) {
    if (!Number.isSafeInteger(maxBytes) || maxBytes < 0) {
        return null;
    }
    const declaredLength = response.headers.get("content-length");
    if (declaredLength !== null
        && (!/^(0|[1-9][0-9]*)$/.test(declaredLength)
            || Number(declaredLength) > maxBytes)) {
        void response.body?.cancel().catch(() => { });
        return null;
    }
    const decoder = new TextDecoder("utf-8", {
        fatal: true,
        ignoreBOM: true,
    });
    if (!response.body) {
        // A real Fetch Response with a null body contains no payload.
        // Isolated transport stubs may still expose an arrayBuffer().
        const bytes = await response.arrayBuffer();
        if (bytes.byteLength > maxBytes) {
            return null;
        }
        try {
            return decoder.decode(bytes);
        }
        catch {
            return null;
        }
    }
    const reader = response.body.getReader();
    let receivedBytes = 0;
    let result = "";
    try {
        for (;;) {
            const { done, value } = await reader.read();
            if (done) {
                try {
                    return result + decoder.decode();
                }
                catch {
                    return null;
                }
            }
            if (value.byteLength > maxBytes - receivedBytes) {
                void reader.cancel().catch(() => { });
                return null;
            }
            receivedBytes += value.byteLength;
            try {
                result += decoder.decode(value, { stream: true });
            }
            catch {
                void reader.cancel().catch(() => { });
                return null;
            }
        }
    }
    finally {
        reader.releaseLock();
    }
}
export async function parseBoundedJsonResponse(response) {
    const maxBytes = MAX_CONTROL_JSON_BYTES;
    const value = await decodeBoundedUtf8Response(response, maxBytes);
    if (value === null) {
        return null;
    }
    // Fetch's Response.json() strips the UTF-8 BOM before JSON.parse().
    const withoutBom = value.startsWith("\uFEFF") ? value.slice(1) : value;
    try {
        return JSON.parse(withoutBom);
    }
    catch {
        return null;
    }
}
// Browser metadata is explanatory, not an unbounded decoded JSON work queue.
// Current handshake and draft transports emit exactly one diagnostic each.
const MAX_DIAGNOSTIC_ITEMS = 128;
const MAX_DIAGNOSTIC_ITEM_FIELDS = 128;
function isRecord(value) {
    return typeof value === "object"
        && value !== null
        && !Array.isArray(value);
}
// Decoded JSON fields are own data properties. Do not invoke getters when
// these parsers are also passed objects constructed inside JavaScript.
export function ownDataField(value, field) {
    const descriptor = Object.getOwnPropertyDescriptor(value, field);
    return descriptor !== undefined && Object.hasOwn(descriptor, "value")
        ? descriptor.value
        : undefined;
}
function isCompleteness(value) {
    return value === "complete" || value === "incomplete";
}
function admittedDiagnosticItem(value) {
    if (!isRecord(value)) {
        return null;
    }
    // Copy own data descriptors, not the caller's object. Later consumers
    // must not reenter Proxy traps or observe field changes after admission.
    // A decoded JSON item has only string-keyed data fields; bound the work
    // per item as well as the number of items.
    try {
        const fields = Reflect.ownKeys(value);
        if (fields.length > MAX_DIAGNOSTIC_ITEM_FIELDS) {
            return null;
        }
        const snapshot = {};
        for (const field of fields) {
            if (typeof field !== "string") {
                return null;
            }
            const descriptor = Object.getOwnPropertyDescriptor(value, field);
            if (descriptor === undefined
                || !Object.hasOwn(descriptor, "value")) {
                return null;
            }
            Object.defineProperty(snapshot, field, {
                value: descriptor.value,
                enumerable: descriptor.enumerable === true,
                configurable: true,
                writable: true,
            });
        }
        const code = ownDataField(snapshot, "code");
        return typeof code === "string" && code !== ""
            ? snapshot
            : null;
    }
    catch {
        // In-process traps have no authority to break diagnostic admission.
        return null;
    }
}
function admittedDiagnosticItems(items) {
    // Avoid property getters even when an in-process caller wraps its Array.
    // Ordinary decoded JSON arrays always have an own data length.
    const length = ownDataField(items, "length");
    if (typeof length !== "number"
        || !Number.isSafeInteger(length)
        || length < 0
        || length > MAX_DIAGNOSTIC_ITEMS) {
        return null;
    }
    // Both the collection and its item records must be owned snapshots.
    const admitted = [];
    for (let index = 0; index < length; index += 1) {
        const item = admittedDiagnosticItem(ownDataField(items, String(index)));
        if (item === null) {
            return null;
        }
        admitted.push(item);
    }
    return admitted;
}
export function parseDiagnosticSet(value) {
    // Descriptor traps from in-process objects are invalid payloads.
    try {
        if (!isRecord(value)) {
            return null;
        }
        const version = ownDataField(value, "version");
        const completeness = ownDataField(value, "completeness");
        const items = ownDataField(value, "items");
        if (version !== DIAGNOSTIC_VERSION
            || !isCompleteness(completeness)
            || !Array.isArray(items)) {
            return null;
        }
        const admittedItems = admittedDiagnosticItems(items);
        return admittedItems === null
            ? null
            : { completeness, items: admittedItems };
    }
    catch {
        return null;
    }
}
