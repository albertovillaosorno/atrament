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
//   - Cryptographically secure per-process browser session secret generation.
// - Must-Not:
//   - Persist, log, reuse, or derive the secret from predictable process state.
// - Allows:
//   - Inputs: Operating-system cryptographic randomness only.
//   - Outputs: One in-memory 256-bit secret encoded as lowercase hexadecimal.
//   - Side effects: Reading the operating system's preferred random source.
// - Split-When:
//   - Secret derivation requires independently governed cryptographic policy.
// - Merge-When:
//   - Browser admission no longer requires a per-process secret.
// - Summary:
//   - Generates the disposable browser-session credential.
// - Description:
//   - Keeps operating-system random-source authority outside request handling.
// - Usage:
//   - Generate exactly once while composing a new Atrament process session.
// - Defaults:
//   - Uses 32 random bytes and lowercase hexadecimal transport encoding.
//

//! Operating-system-backed secret generation for one Atrament browser session.

use std::error::Error;
use std::fmt::{self, Write as _};

const SECRET_BYTES: usize = 32;
const ENCODED_BYTES: usize = SECRET_BYTES * 2;

/// Failure to acquire cryptographically secure operating-system randomness.
#[derive(Clone, Copy, Debug)]
pub struct SecretGenerationError(getrandom::Error);

impl fmt::Display for SecretGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("operating-system session randomness is unavailable")
    }
}

impl Error for SecretGenerationError {
    fn cause(&self) -> Option<&dyn Error> {
        Some(&self.0)
    }

    fn description(&self) -> &'static str {
        "operating-system session randomness is unavailable"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

/// An in-memory 256-bit session secret encoded for browser transport.
pub struct SessionSecret {
    encoded: String,
}

impl fmt::Debug for SessionSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionSecret([REDACTED])")
    }
}

impl SessionSecret {
    /// Return the URL-fragment-safe credential text.
    #[must_use]
    pub fn encoded(&self) -> &str {
        &self.encoded
    }

    /// Generate a fresh session secret from the operating-system random source.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the operating system cannot supply all 32
    /// required random bytes.
    pub fn generate() -> Result<Self, SecretGenerationError> {
        let mut bytes = [0u8; SECRET_BYTES];
        getrandom::fill(&mut bytes).map_err(SecretGenerationError)?;
        let mut encoded = String::with_capacity(ENCODED_BYTES);
        for byte in bytes {
            write!(&mut encoded, "{byte:02x}").map_err(|_| {
                SecretGenerationError(getrandom::Error::UNEXPECTED)
            })?;
        }
        Ok(Self { encoded })
    }
}
