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
//   - Allocation-free, bounded UTF-8 and JSON object syntax admission.
// - Must-Not:
//   - Choose manifest field names, accept duplicate-key semantics, construct
//     typed manifests, read files, parse ZIP, or mutate session state.
// - Allows:
//   - Inputs: A byte slice and caller-selected structural resource limits.
//   - Outputs: Constructor-sealed borrowed syntax evidence or typed failure.
//   - Side effects: None.
// - Split-When:
//   - Canonical manifest schema and field mapping become a separate codec.
// - Merge-When:
//   - Trusted bounded manifest decoding subsumes syntax admission.
// - Summary:
//   - Rejects invalid JSON grammar, Unicode escapes, depth and token excess.
// - Description:
//   - Valid JSON alone cannot bind typed values to source bytes.
// - Usage:
//   - Validate root JSON syntax before any typed field-level interpretation.
// - Defaults:
//   - Bytes and nesting must fit both caller and hard stack-safe limits.
//

//! Resource-bounded JSON object syntax checks for profile manifest bytes.

use std::str::from_utf8;

/// Maximum container nesting allowed, even if a caller requests more.
pub const MAXIMUM_JSON_NESTING: usize = 64;

/// Caller resource limits, applied before and during JSON parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileJsonSyntaxLimits {
    /// Maximum raw UTF-8 bytes including whitespace.
    pub maximum_bytes: usize,
    /// Maximum object and array nesting, counting the root object as one.
    pub maximum_nesting: usize,
    /// Maximum JSON values including the root; object keys do not count.
    pub maximum_values: usize,
}

/// Failure before any typed manifest field interpretation is permitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileJsonSyntaxError {
    /// Input is not valid UTF-8.
    InvalidUtf8,
    /// A JSON string or numeric/literal/object/array grammar rule failed.
    Malformed,
    /// Caller byte limit is exceeded before interpreting syntax.
    MaximumBytesExceeded,
    /// Caller limit or fixed stack-safe nesting ceiling is exceeded.
    MaximumNestingExceeded,
    /// Caller JSON-value limit is exceeded.
    MaximumValuesExceeded,
    /// The root is a primitive or array, not a JSON object.
    RootNotObject,
}

/// Borrowed evidence that exact input bytes contain one JSON object.
///
/// Object member semantics, duplicate decoded names, wire field mapping,
/// digest decoding and a typed manifest's provenance are not checked here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedProfileJsonObject<'bytes> {
    bytes: &'bytes [u8],
    values: usize,
}

impl<'bytes> ValidatedProfileJsonObject<'bytes> {
    /// Exact original UTF-8 bytes, including any insignificant whitespace.
    #[must_use]
    pub const fn bytes(self) -> &'bytes [u8] {
        self.bytes
    }

    /// Number of syntactic JSON values consumed including the root object.
    #[must_use]
    pub const fn value_count(self) -> usize {
        self.values
    }
}

struct SyntaxCursor<'bytes> {
    bytes: &'bytes [u8],
    cursor: usize,
    maximum_nesting: usize,
    remaining_values: usize,
    values: usize,
}

impl SyntaxCursor<'_> {
    fn array(&mut self, depth: usize) -> Result<(), ProfileJsonSyntaxError> {
        self.nesting(depth)?;
        self.required(b'[')?;
        self.whitespace();
        if self.consume(b']') {
            return Ok(());
        }
        loop {
            self.value(depth)?;
            self.whitespace();
            if self.consume(b']') {
                return Ok(());
            }
            self.required(b',')?;
            self.whitespace();
        }
    }

    fn byte(&self) -> Option<u8> {
        self.bytes.get(self.cursor).copied()
    }

    fn consume(&mut self, expected: u8) -> bool {
        if self.byte() == Some(expected) {
            self.cursor = self.cursor.saturating_add(1);
            true
        } else {
            false
        }
    }

    fn digits(&mut self) -> bool {
        let start = self.cursor;
        while matches!(self.byte(), Some(b'0'..=b'9')) {
            self.cursor = self.cursor.saturating_add(1);
        }
        self.cursor > start
    }

    fn hexadecimal_quad(&mut self) -> Result<u16, ProfileJsonSyntaxError> {
        let mut code = 0u16;
        for _ in 0u8..4u8 {
            let digit = match self.byte() {
                Some(byte @ b'0'..=b'9') => {
                    u16::from(byte.saturating_sub(b'0'))
                }
                Some(byte @ b'a'..=b'f') => {
                    u16::from(byte.saturating_sub(b'a')).saturating_add(10)
                }
                Some(byte @ b'A'..=b'F') => {
                    u16::from(byte.saturating_sub(b'A')).saturating_add(10)
                }
                _ => return Err(ProfileJsonSyntaxError::Malformed),
            };
            code = code.wrapping_mul(16).wrapping_add(digit);
            self.cursor = self.cursor.saturating_add(1);
        }
        Ok(code)
    }

    fn literal(
        &mut self,
        literal: &[u8],
    ) -> Result<(), ProfileJsonSyntaxError> {
        let Some(remaining) = self.bytes.get(self.cursor..) else {
            return Err(ProfileJsonSyntaxError::Malformed);
        };
        if !remaining.starts_with(literal) {
            return Err(ProfileJsonSyntaxError::Malformed);
        }
        self.cursor = self.cursor.saturating_add(literal.len());
        Ok(())
    }

    const fn nesting(
        &self,
        depth: usize,
    ) -> Result<(), ProfileJsonSyntaxError> {
        if depth > self.maximum_nesting || depth > MAXIMUM_JSON_NESTING {
            Err(ProfileJsonSyntaxError::MaximumNestingExceeded)
        } else {
            Ok(())
        }
    }

    fn number(&mut self) -> Result<(), ProfileJsonSyntaxError> {
        let _negative = self.consume(b'-');
        if !self.consume(b'0') {
            if !matches!(self.byte(), Some(b'1'..=b'9')) {
                return Err(ProfileJsonSyntaxError::Malformed);
            }
            let _integral = self.digits();
        }
        if self.consume(b'.') && !self.digits() {
            return Err(ProfileJsonSyntaxError::Malformed);
        }
        if self.consume(b'e') || self.consume(b'E') {
            if !self.consume(b'+') {
                let _minus = self.consume(b'-');
            }
            if !self.digits() {
                return Err(ProfileJsonSyntaxError::Malformed);
            }
        }
        Ok(())
    }

    fn object(&mut self, depth: usize) -> Result<(), ProfileJsonSyntaxError> {
        self.nesting(depth)?;
        self.required(b'{')?;
        self.whitespace();
        if self.consume(b'}') {
            return Ok(());
        }
        loop {
            self.string()?;
            self.whitespace();
            self.required(b':')?;
            self.whitespace();
            self.value(depth)?;
            self.whitespace();
            if self.consume(b'}') {
                return Ok(());
            }
            self.required(b',')?;
            self.whitespace();
        }
    }

    fn required(&mut self, expected: u8) -> Result<(), ProfileJsonSyntaxError> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(ProfileJsonSyntaxError::Malformed)
        }
    }

    fn string(&mut self) -> Result<(), ProfileJsonSyntaxError> {
        self.required(b'"')?;
        loop {
            match self.byte() {
                Some(b'"') => {
                    self.cursor = self.cursor.saturating_add(1);
                    return Ok(());
                }
                Some(b'\\') => {
                    self.cursor = self.cursor.saturating_add(1);
                    match self.byte() {
                        Some(
                            b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r'
                            | b't',
                        ) => {
                            self.cursor = self.cursor.saturating_add(1);
                        }
                        Some(b'u') => {
                            self.cursor = self.cursor.saturating_add(1);
                            let code = self.hexadecimal_quad()?;
                            if (0xd800..=0xdbff).contains(&code) {
                                self.required(b'\\')?;
                                self.required(b'u')?;
                                let low = self.hexadecimal_quad()?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return Err(
                                        ProfileJsonSyntaxError::Malformed,
                                    );
                                }
                            } else if (0xdc00..=0xdfff).contains(&code) {
                                return Err(ProfileJsonSyntaxError::Malformed);
                            }
                        }
                        _ => return Err(ProfileJsonSyntaxError::Malformed),
                    }
                }
                Some(0..=31) | None => {
                    return Err(ProfileJsonSyntaxError::Malformed);
                }
                Some(_) => self.cursor = self.cursor.saturating_add(1),
            }
        }
    }

    fn value(&mut self, depth: usize) -> Result<(), ProfileJsonSyntaxError> {
        if self.remaining_values == 0 {
            return Err(ProfileJsonSyntaxError::MaximumValuesExceeded);
        }
        self.remaining_values = self.remaining_values.saturating_sub(1);
        self.values = self.values.saturating_add(1);
        match self.byte() {
            Some(b'{') => self.object(depth.saturating_add(1)),
            Some(b'[') => self.array(depth.saturating_add(1)),
            Some(b'"') => self.string(),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(ProfileJsonSyntaxError::Malformed),
        }
    }
    fn whitespace(&mut self) {
        while matches!(self.byte(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.cursor = self.cursor.saturating_add(1);
        }
    }
}

/// Validate a bounded UTF-8 JSON root object without allocating or decoding.
///
/// This is a grammar gate only. It does not reject duplicate *decoded* keys,
/// establish canonical JSON encoding, or map keys into a profile manifest.
/// The eventual typed codec must enforce those checks independently.
///
/// # Errors
///
/// Rejects invalid UTF-8, excess bytes, nesting or values, malformed JSON,
/// or a root value other than a complete object.
pub fn validate_profile_json_object(
    bytes: &[u8],
    limits: ProfileJsonSyntaxLimits,
) -> Result<ValidatedProfileJsonObject<'_>, ProfileJsonSyntaxError> {
    if bytes.len() > limits.maximum_bytes {
        return Err(ProfileJsonSyntaxError::MaximumBytesExceeded);
    }
    if from_utf8(bytes).is_err() {
        return Err(ProfileJsonSyntaxError::InvalidUtf8);
    }
    let mut parser = SyntaxCursor {
        bytes,
        cursor: 0,
        maximum_nesting: limits.maximum_nesting,
        remaining_values: limits.maximum_values,
        values: 0,
    };
    parser.whitespace();
    if parser.byte() != Some(b'{') {
        return Err(ProfileJsonSyntaxError::RootNotObject);
    }
    parser.value(0)?;
    parser.whitespace();
    if parser.cursor != bytes.len() {
        return Err(ProfileJsonSyntaxError::Malformed);
    }
    Ok(ValidatedProfileJsonObject {
        bytes,
        values: parser.values,
    })
}
