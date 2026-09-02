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
//   - Validation of the browser-only session-secret fragment representation.
// - Must-Not:
//   - Persist credentials, access browser storage, or perform network requests.
// - Allows:
//   - Inputs: One URL fragment string.
//   - Outputs: A validated in-memory credential string or null.
//   - Side effects: None.
// - Split-When:
//   - Browser credential transport supports more than one representation.
// - Merge-When:
//   - The browser no longer receives credentials through a URL fragment.
// - Summary:
//   - Parses the disposable browser session credential fragment.
// - Description:
//   - Keeps credential syntax validation pure and independently executable.
// - Usage:
//   - Validate the startup fragment before storing the credential in memory.
// - Defaults:
//   - Accepts exactly 64 lowercase hexadecimal characters after `#session=`.
//
const SESSION_FRAGMENT_PREFIX = "#session=";
const SESSION_SECRET_PATTERN = /^[0-9a-f]{64}$/u;

export function sessionSecretFromFragment(hash: string): string | null {
    if (!hash.startsWith(SESSION_FRAGMENT_PREFIX)) {
        return null;
    }
    const candidate = hash.slice(SESSION_FRAGMENT_PREFIX.length);
    return SESSION_SECRET_PATTERN.test(candidate) ? candidate : null;
}
