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
//   - Browser validation of shared diagnostic namespace and code metadata.
// - Must-Not:
//   - Reimplement diagnostic semantics, choose blocking policy, or perform I/O.
// - Allows:
//   - Inputs: One decoded adapter diagnostic value.
//   - Outputs: Version-admitted diagnostic code metadata or null.
//   - Side effects: None.
// - Split-When:
//   - Browser diagnostic transport gains independently versioned projections.
// - Merge-When:
//   - All browser transports consume diagnostic metadata through another
//     module.
// - Summary:
//   - Admits only current-version diagnostic metadata in the browser adapter.
// - Description:
//   - Keeps namespace validation shared without duplicating backend policy.
// - Usage:
//   - Parse adapter diagnostic metadata before route-specific field validation.
// - Defaults:
//   - Accepts only the first-release diagnostic namespace identity.
//
export const DIAGNOSTIC_VERSION = "atrament.diagnostic/1";
// Browser metadata is explanatory, not an unbounded decoded JSON work queue.
// Current handshake and draft transports emit exactly one diagnostic each.
const MAX_DIAGNOSTIC_ITEMS = 128;
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
function isDiagnosticItem(value) {
    if (!isRecord(value)) {
        return false;
    }
    const code = ownDataField(value, "code");
    return typeof code === "string" && code !== "";
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
    // The route consumers read `items[0]` directly. Return a snapshot so
    // subsequent access cannot invoke a Proxy getter on the input array or
    // observe a changed source slot after validation.
    const admitted = [];
    for (let index = 0; index < length; index += 1) {
        const item = ownDataField(items, String(index));
        if (!isDiagnosticItem(item)) {
            return null;
        }
        admitted.push(item);
    }
    return admitted;
}
export function parseDiagnosticSet(value) {
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
