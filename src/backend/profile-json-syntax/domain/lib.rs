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
//   - Exact duplicate decoded-key rejection for every nested object.
// - Must-Not:
//   - Choose manifest field names, normalize Unicode, construct
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
//   - Rejects bad JSON grammar, ambiguous keys, depth and token excess.
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

/// Maximum keys admitted in each JSON object without a heap allocation.
pub const MAXIMUM_JSON_OBJECT_MEMBERS: usize = 128;

/// Caller resource limits, applied before and during JSON parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileJsonSyntaxLimits {
    /// Maximum raw UTF-8 bytes including whitespace.
    pub maximum_bytes: usize,
    /// Maximum object and array nesting, counting the root object as one.
    pub maximum_nesting: usize,
    /// Maximum keys in each JSON object (also subject to a fixed ceiling).
    pub maximum_object_members: usize,
    /// Maximum JSON values including the root; object keys do not count.
    pub maximum_values: usize,
}

/// Failure before any typed manifest field interpretation is permitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileJsonSyntaxError {
    /// Two keys in one object decode to the exact same Unicode scalar stream.
    DuplicateObjectKey,
    /// Input is not valid UTF-8.
    InvalidUtf8,
    /// A JSON string or numeric/literal/object/array grammar rule failed.
    Malformed,
    /// Caller byte limit is exceeded before interpreting syntax.
    MaximumBytesExceeded,
    /// Caller limit or fixed stack-safe nesting ceiling is exceeded.
    MaximumNestingExceeded,
    /// Caller or fixed per-object key limit is exceeded.
    MaximumObjectMembersExceeded,
    /// Caller JSON-value limit is exceeded.
    MaximumValuesExceeded,
    /// The root is a primitive or array, not a JSON object.
    RootNotObject,
}

/// Borrowed evidence that exact input bytes contain one JSON object.
///
/// Duplicate decoded keys have been rejected in every object. Wire field
/// mapping, digest decoding and a typed manifest's provenance are not checked.
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
    maximum_object_members: usize,
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

    fn key_fingerprint(
        &self,
        key: (usize, usize),
    ) -> Result<u64, ProfileJsonSyntaxError> {
        let interior = self
            .bytes
            .get(key.0.saturating_add(1)..key.1.saturating_sub(1))
            .ok_or(ProfileJsonSyntaxError::Malformed)?;
        let mut position = 0usize;
        let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
        while position < interior.len() {
            let scalar = decoded_key_scalar(interior, &mut position)
                .ok_or(ProfileJsonSyntaxError::Malformed)?;
            fingerprint ^= u64::from(u32::from(scalar));
            fingerprint = fingerprint.wrapping_mul(0x100_0000_01b3u64);
        }
        Ok(fingerprint)
    }

    fn keys_equal(
        &self,
        previous: (usize, usize),
        current: (usize, usize),
    ) -> bool {
        let first_candidate = self
            .bytes
            .get(previous.0.saturating_add(1)..previous.1.saturating_sub(1));
        let second_candidate = self
            .bytes
            .get(current.0.saturating_add(1)..current.1.saturating_sub(1));
        let (Some(first), Some(second)) = (first_candidate, second_candidate)
        else {
            return false;
        };
        let (mut left, mut right) = (0usize, 0usize);
        while left < first.len() && right < second.len() {
            match (
                decoded_key_scalar(first, &mut left),
                decoded_key_scalar(second, &mut right),
            ) {
                (Some(left_scalar), Some(right_scalar))
                    if left_scalar == right_scalar => {}
                _ => return false,
            }
        }
        left == first.len() && right == second.len()
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
        // Only source spans are retained; no key is copied or normalized.
        let mut keys = [(0usize, 0usize, 0u64); MAXIMUM_JSON_OBJECT_MEMBERS];
        let mut count = 0usize;
        loop {
            if count >= self.maximum_object_members
                || count >= MAXIMUM_JSON_OBJECT_MEMBERS
            {
                return Err(
                    ProfileJsonSyntaxError::MaximumObjectMembersExceeded,
                );
            }
            let start = self.cursor;
            self.string()?;
            let key = (start, self.cursor);
            let fingerprint = self.key_fingerprint(key)?;
            for previous in keys.iter().take(count) {
                if previous.2 == fingerprint
                    && self.keys_equal((previous.0, previous.1), key)
                {
                    return Err(ProfileJsonSyntaxError::DuplicateObjectKey);
                }
            }
            if let Some(slot) = keys.get_mut(count) {
                *slot = (start, self.cursor, fingerprint);
            }
            count = count.saturating_add(1);
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

// Decode one scalar from a previously validated JSON string interior.
// Comparing scalars avoids treating escape spellings as different object keys.
fn decoded_key_scalar(bytes: &[u8], cursor: &mut usize) -> Option<char> {
    let byte = *bytes.get(*cursor)?;
    if byte != b'\\' {
        let width = match byte {
            0..=0x7f => 1usize,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 4,
        };
        let end = cursor.checked_add(width)?;
        let scalar =
            from_utf8(bytes.get(*cursor..end)?).ok()?.chars().next()?;
        *cursor = end;
        return Some(scalar);
    }
    *cursor = cursor.saturating_add(1);
    let escape = *bytes.get(*cursor)?;
    *cursor = cursor.saturating_add(1);
    match escape {
        b'"' => Some('"'),
        b'\\' => Some('\\'),
        b'/' => Some('/'),
        b'b' => Some('\u{0008}'),
        b'f' => Some('\u{000c}'),
        b'n' => Some('\n'),
        b'r' => Some('\r'),
        b't' => Some('\t'),
        b'u' => {
            let high = hexadecimal_key_quad(bytes, cursor)?;
            let scalar = if (0xd800..=0xdbff).contains(&high) {
                let slash_u = bytes.get(*cursor..cursor.saturating_add(2))?;
                if slash_u != b"\\u" {
                    return None;
                }
                *cursor = cursor.saturating_add(2);
                let low = hexadecimal_key_quad(bytes, cursor)?;
                if !(0xdc00..=0xdfff).contains(&low) {
                    return None;
                }
                0x1_0000u32
                    .saturating_add(
                        u32::from(high.saturating_sub(0xd800))
                            .saturating_mul(1024),
                    )
                    .saturating_add(u32::from(low.saturating_sub(0xdc00)))
            } else {
                u32::from(high)
            };
            char::from_u32(scalar)
        }
        _ => None,
    }
}

fn hexadecimal_key_quad(bytes: &[u8], cursor: &mut usize) -> Option<u16> {
    let mut code = 0u16;
    for _ in 0u8..4u8 {
        let digit = char::from(*bytes.get(*cursor)?).to_digit(16)?;
        code = code
            .saturating_mul(16)
            .saturating_add(u16::try_from(digit).ok()?);
        *cursor = cursor.saturating_add(1);
    }
    Some(code)
}

/// Validate a bounded UTF-8 JSON root object without allocating or decoding.
///
/// This gate also rejects duplicate decoded keys in each JSON object, even
/// when escaped and unescaped names differ in their source spelling.
/// It does not normalize Unicode, establish canonical JSON encoding, or map
/// keys into a typed profile manifest.
///
/// # Errors
///
/// Rejects invalid UTF-8, excess bytes, nesting, values, or object keys,
/// duplicate decoded keys, malformed JSON, or a non-object root.
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
        maximum_object_members: limits.maximum_object_members,
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
