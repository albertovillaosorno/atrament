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

use std::iter::FusedIterator;
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

/// The syntactic kind of one already validated JSON value.
///
/// Type admission alone cannot decide a profile manifest field's semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileJsonValueKind {
    /// Ordered values, possibly empty.
    Array,
    /// `true` or `false`.
    Boolean,
    /// JSON `null`.
    Null,
    /// A JSON numeric token with its original decimal notation retained.
    Number,
    /// An object with unique decoded member names.
    Object,
    /// A JSON string including its original escape notation.
    String,
}

/// One caller-selected step through a syntax-admitted JSON value.
///
/// Keys compare decoded Unicode scalars without normalization; indices are
/// zero-based and never silently interpreted as object member names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileJsonPathSegment<'key> {
    /// Select an array element using a zero-based index.
    Index(usize),
    /// Select an exact decoded object member name.
    Key(&'key str),
}

/// Failure to copy decoded JSON string or member-name UTF-8 scalars.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileJsonStringCopyError {
    /// The caller supplied fewer bytes than are needed for the decoded UTF-8.
    BufferTooSmall {
        /// Exact required number of UTF-8 destination bytes.
        required: usize,
    },
    /// The sealed selected JSON value is not a string.
    NotString,
}

/// Constructor-sealed borrowed view into an already admitted JSON document.
///
/// Traversal never confers manifest field names, canonical encoding, or
/// Import authority. The original document had its complete grammar, Unicode,
/// duplicate-key, byte, member, depth, and value budgets checked first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedProfileJsonValue<'bytes> {
    bytes: &'bytes [u8],
    document_values: usize,
}

/// One-pass ordered iterator over a constructor-sealed JSON array.
///
/// The iterator borrows source bytes and keeps no collection of JSON values.
/// It is available only after the complete parent JSON root was validated.
#[derive(Clone, Debug)]
pub struct ValidatedProfileJsonArrayElements<'bytes> {
    bytes: &'bytes [u8],
    cursor: usize,
    document_values: usize,
    finished: bool,
}

// Iterator has many default combinators; implementing those individually
// would replace the standard semantics rather than strengthen the boundary.
#[expect(
    clippy::missing_trait_methods,
    reason = "standard Iterator combinators retain their default behavior"
)]
impl<'bytes> Iterator for ValidatedProfileJsonArrayElements<'bytes> {
    type Item = ValidatedProfileJsonValue<'bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let mut reader =
            SyntaxCursor::for_admitted(self.bytes, self.document_values);
        reader.cursor = self.cursor;
        reader.whitespace();
        if reader.consume(b']') {
            self.finished = true;
            return None;
        }
        let start = reader.cursor;
        if reader.value(0).is_err() {
            self.finished = true;
            return None;
        }
        let selected = self.bytes.get(start..reader.cursor)?;
        reader.whitespace();
        if reader.consume(b',') {
            reader.whitespace();
            self.cursor = reader.cursor;
        } else {
            self.finished = true;
        }
        Some(ValidatedProfileJsonValue {
            bytes: selected,
            document_values: self.document_values,
        })
    }
}

impl FusedIterator for ValidatedProfileJsonArrayElements<'_> {}

/// Allocation-free Unicode scalar iterator over a validated JSON string.
///
/// This includes decoded escapes and surrogate pairs without normalizing
/// Unicode, reading files, or creating a heap-backed string.
#[derive(Clone, Debug)]
pub struct ValidatedProfileJsonStringChars<'bytes> {
    cursor: usize,
    interior: &'bytes [u8],
}

// Retaining Iterator's standard combinators avoids introducing custom
// character-ordering, exhaustion, or numeric conversion semantics.
#[expect(
    clippy::missing_trait_methods,
    reason = "standard Iterator combinators retain their default behavior"
)]
impl Iterator for ValidatedProfileJsonStringChars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.interior.len() {
            return None;
        }
        decoded_key_scalar(self.interior, &mut self.cursor)
    }
}

impl FusedIterator for ValidatedProfileJsonStringChars<'_> {}

/// Borrowed key and value of one already validated JSON object member.
///
/// The original key spelling remains available for opaque preservation;
/// decoded scalar comparison is exact, with no implicit normalization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedProfileJsonField<'bytes> {
    key_bytes: &'bytes [u8],
    member_bytes: &'bytes [u8],
    value: ValidatedProfileJsonValue<'bytes>,
}

impl<'bytes> ValidatedProfileJsonField<'bytes> {
    /// Copy one decoded object key into caller-owned bounded UTF-8 storage.
    ///
    /// Failure on inadequate space leaves every destination byte unchanged.
    /// `NotString` cannot occur for a constructor-sealed JSON field name.
    /// Original name spelling remains available through `key_json`.
    ///
    /// # Errors
    ///
    /// Reports exact required UTF-8 bytes on insufficient capacity.
    pub fn copy_key_utf8(
        self,
        destination: &mut [u8],
    ) -> Result<usize, ProfileJsonStringCopyError> {
        copy_decoded_utf8(self.decoded_key_chars(), destination)
    }

    /// Iterate decoded Unicode scalars of the admitted member name.
    #[must_use]
    pub fn decoded_key_chars(self) -> ValidatedProfileJsonStringChars<'bytes> {
        ValidatedProfileJsonStringChars {
            cursor: 0,
            interior: self
                .key_bytes
                .get(1..self.key_bytes.len().saturating_sub(1))
                .unwrap_or_default(),
        }
    }

    /// Exact original JSON key token including quotes and character escapes.
    #[must_use]
    pub const fn key_json(self) -> &'bytes [u8] {
        self.key_bytes
    }

    /// Compare one name without allocation, case folding or normalization.
    #[must_use]
    pub fn key_matches(self, expected: &str) -> bool {
        self.decoded_key_chars().eq(expected.chars())
    }

    /// Return the exact source span from key opening quote through the
    /// member value, including the original colon and intervening whitespace.
    ///
    /// Excludes separators between members and surrounding object whitespace.
    /// This is preservation evidence, not canonical JSON re-encoding.
    #[must_use]
    pub const fn source_member_json(self) -> &'bytes [u8] {
        self.member_bytes
    }

    /// Return the source-byte value associated with this original key token.
    #[must_use]
    pub const fn value(self) -> ValidatedProfileJsonValue<'bytes> {
        self.value
    }
}

/// Fused one-pass member iterator from a syntax-admitted JSON object.
///
/// Member names and values remain borrowed and unchanged in source order.
#[derive(Clone, Debug)]
pub struct ValidatedProfileJsonObjectFields<'bytes> {
    bytes: &'bytes [u8],
    cursor: usize,
    document_values: usize,
    finished: bool,
    started: bool,
}

impl<'bytes> ValidatedProfileJsonObjectFields<'bytes> {
    const fn for_object(bytes: &'bytes [u8], document_values: usize) -> Self {
        Self {
            bytes,
            cursor: 0,
            document_values,
            finished: false,
            started: false,
        }
    }
}

// Preserve standard iterator semantics instead of reimplementing combinators.
#[expect(
    clippy::missing_trait_methods,
    reason = "standard Iterator combinators retain their default behavior"
)]
impl<'bytes> Iterator for ValidatedProfileJsonObjectFields<'bytes> {
    type Item = ValidatedProfileJsonField<'bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let mut reader =
            SyntaxCursor::for_admitted(self.bytes, self.document_values);
        reader.cursor = self.cursor;
        if !self.started {
            reader.whitespace();
            reader.required(b'{').ok()?;
            self.started = true;
        }
        reader.whitespace();
        if reader.consume(b'}') {
            self.finished = true;
            return None;
        }
        let key_start = reader.cursor;
        if reader.string().is_err() {
            self.finished = true;
            return None;
        }
        let key_end = reader.cursor;
        reader.whitespace();
        if reader.required(b':').is_err() {
            self.finished = true;
            return None;
        }
        reader.whitespace();
        let value_start = reader.cursor;
        if reader.value(0).is_err() {
            self.finished = true;
            return None;
        }
        let value_end = reader.cursor;
        let key_bytes = self.bytes.get(key_start..key_end)?;
        let member_bytes = self.bytes.get(key_start..value_end)?;
        let value_bytes = self.bytes.get(value_start..value_end)?;
        reader.whitespace();
        if reader.consume(b',') {
            self.cursor = reader.cursor;
        } else {
            self.finished = true;
        }
        Some(ValidatedProfileJsonField {
            key_bytes,
            member_bytes,
            value: ValidatedProfileJsonValue {
                bytes: value_bytes,
                document_values: self.document_values,
            },
        })
    }
}

impl FusedIterator for ValidatedProfileJsonObjectFields<'_> {}

impl<'bytes> ValidatedProfileJsonValue<'bytes> {
    /// Select a zero-based array element without copying or normalizing it.
    ///
    /// Missing indices and non-array values return `None`.
    #[must_use]
    pub fn array_element(self, index: usize) -> Option<Self> {
        self.array_elements()?.nth(index)
    }

    /// Traverse all items of one admitted array in their exact source order.
    ///
    /// The iterator yields borrowed views without storing items on the heap.
    /// This remains syntactic projection, not typed manifest admission.
    #[must_use]
    pub fn array_elements(
        self,
    ) -> Option<ValidatedProfileJsonArrayElements<'bytes>> {
        if self.kind() != Some(ProfileJsonValueKind::Array) {
            return None;
        }
        Some(ValidatedProfileJsonArrayElements {
            bytes: self.bytes,
            cursor: 1,
            document_values: self.document_values,
            finished: false,
        })
    }

    /// Follow a caller-supplied structural path from this admitted value.
    ///
    /// An empty path returns this same sealed value. Key and index segments
    /// must match their container kinds; missing members, wrong kinds and
    /// out-of-range indices return `None` without coercion or allocation.
    /// This is JSON syntax selection, never a typed manifest policy.
    #[must_use]
    pub fn at_path(self, path: &[ProfileJsonPathSegment<'_>]) -> Option<Self> {
        let mut current = self;
        for segment in path {
            current = match segment {
                ProfileJsonPathSegment::Key(key) => {
                    current.object_member(key)?
                }
                ProfileJsonPathSegment::Index(index) => {
                    current.array_element(*index)?
                }
            };
        }
        Some(current)
    }

    /// Interpret an admitted JSON `true` or `false` token without coercion.
    ///
    /// Wrong kinds return `None`, including strings containing `"true"`.
    #[must_use]
    pub const fn boolean(self) -> Option<bool> {
        match self.bytes {
            b"true" => Some(true),
            b"false" => Some(false),
            _ => None,
        }
    }

    /// Exact original bytes of this selected JSON value, without outer spaces.
    #[must_use]
    pub const fn bytes(self) -> &'bytes [u8] {
        self.bytes
    }

    /// Copy the Unicode scalar contents of one JSON string into a bounded,
    /// caller-owned UTF-8 buffer without allocating an owned string.
    ///
    /// JSON escape spellings are decoded, but normalization, case folding
    /// and schema-specific length or character policies are not performed.
    /// If the value is not a string or capacity is insufficient, the output
    /// remains completely unchanged. The returned length excludes any NUL
    /// terminator; embedded JSON `\u0000` decodes as an ordinary NUL byte.
    ///
    /// # Errors
    ///
    /// Distinguishes a wrong JSON kind from inadequate caller buffer space.
    pub fn copy_string_utf8(
        self,
        destination: &mut [u8],
    ) -> Result<usize, ProfileJsonStringCopyError> {
        let scalars = self
            .string_chars()
            .ok_or(ProfileJsonStringCopyError::NotString)?;
        copy_decoded_utf8(scalars, destination)
    }

    /// Return whether the admitted token is exactly JSON `null`.
    #[must_use]
    pub fn is_null(self) -> bool {
        self.bytes == b"null"
    }

    /// Syntactic kind, not an admitted typed profile field.
    ///
    /// Returns `Some` for every constructor-sealed valid value; `None` would
    /// indicate a failed internal invariant, never a new manifest field kind.
    #[must_use]
    pub const fn kind(self) -> Option<ProfileJsonValueKind> {
        match self.bytes.first() {
            Some(b'[') => Some(ProfileJsonValueKind::Array),
            Some(b't' | b'f') => Some(ProfileJsonValueKind::Boolean),
            Some(b'n') => Some(ProfileJsonValueKind::Null),
            Some(b'-' | b'0'..=b'9') => Some(ProfileJsonValueKind::Number),
            Some(b'{') => Some(ProfileJsonValueKind::Object),
            Some(b'"') => Some(ProfileJsonValueKind::String),
            _ => None,
        }
    }

    /// Select a named field while retaining its exact source member syntax.
    ///
    /// Lookup compares decoded Unicode scalars and never treats two distinct
    /// member spellings as separate fields after duplicate-key validation.
    #[must_use]
    pub fn object_field(
        self,
        key: &str,
    ) -> Option<ValidatedProfileJsonField<'bytes>> {
        self.object_fields()?.find(|field| field.key_matches(key))
    }

    /// Traverse admitted object members in their original source order.
    ///
    /// The iterator preserves both key spelling and value source bytes.
    #[must_use]
    pub fn object_fields(
        self,
    ) -> Option<ValidatedProfileJsonObjectFields<'bytes>> {
        if self.kind() != Some(ProfileJsonValueKind::Object) {
            return None;
        }
        Some(ValidatedProfileJsonObjectFields::for_object(
            self.bytes,
            self.document_values,
        ))
    }

    /// Select one decoded member name only when this value is an object.
    #[must_use]
    pub fn object_member(self, key: &str) -> Option<Self> {
        Some(self.object_field(key)?.value())
    }
    /// Iterate the Unicode scalar values of one admitted JSON string.
    ///
    /// The original escape spelling stays available through `bytes`.
    /// This does not choose manifest character normalization or identity rules.
    #[must_use]
    pub fn string_chars(
        self,
    ) -> Option<ValidatedProfileJsonStringChars<'bytes>> {
        if self.kind() != Some(ProfileJsonValueKind::String) {
            return None;
        }
        let interior = self.bytes.get(1..self.bytes.len().checked_sub(1)?)?;
        Some(ValidatedProfileJsonStringChars {
            cursor: 0,
            interior,
        })
    }

    /// Interpret a JSON number only if its entire spelling is a plain,
    /// nonnegative base-10 integer fitting `u64` with no sign or exponent.
    ///
    /// Decimal and exponent spellings remain valid JSON but are not silently
    /// rounded, coerced or normalized into integer metadata here.
    #[must_use]
    pub fn unsigned_integer(self) -> Option<u64> {
        if self.kind() != Some(ProfileJsonValueKind::Number) {
            return None;
        }
        let spelling = from_utf8(self.bytes).ok()?;
        if spelling.bytes().any(|byte| !byte.is_ascii_digit()) {
            return None;
        }
        spelling.parse::<u64>().ok()
    }
}

impl<'bytes> ValidatedProfileJsonObject<'bytes> {
    /// Exact original UTF-8 bytes, including any insignificant whitespace.
    #[must_use]
    pub const fn bytes(self) -> &'bytes [u8] {
        self.bytes
    }

    /// Select one decoded root field with its original key and member span.
    ///
    /// Missing names return `None`; no required wire field is assumed here.
    #[must_use]
    pub fn root_field(
        self,
        key: &str,
    ) -> Option<ValidatedProfileJsonField<'bytes>> {
        self.root_fields().find(|field| field.key_matches(key))
    }

    /// Traverse unique decoded root fields in their original JSON order.
    ///
    /// Unknown fields can be inspected and their raw names retained without
    /// choosing or accepting any portable-profile wire schema.
    #[must_use]
    pub const fn root_fields(self) -> ValidatedProfileJsonObjectFields<'bytes> {
        ValidatedProfileJsonObjectFields::for_object(self.bytes, self.values)
    }

    /// Borrow the exact JSON bytes of one unique top-level member by its
    /// decoded key name, without interpreting the member's value.
    ///
    /// The complete object was already admitted, so this lookup cannot
    /// bypass duplicate-key rejection or resource limits. Unicode scalar
    /// equality is exact, without URI decoding, case folding or NFC.
    /// Returned bytes include the original JSON escapes and internal spacing.
    #[must_use]
    pub fn root_member_json(self, key: &str) -> Option<&'bytes [u8]> {
        Some(self.root_member_value(key)?.bytes())
    }

    /// Select a root JSON value through its exact decoded object member name.
    ///
    /// The sealed result may be traversed without copying any source bytes.
    /// Nested objects and arrays remain checked against the same original
    /// document's syntax admission. Unknown names do not confer field policy.
    #[must_use]
    pub fn root_member_value(
        self,
        key: &str,
    ) -> Option<ValidatedProfileJsonValue<'bytes>> {
        Some(self.root_field(key)?.value())
    }

    /// Follow a structural path into the admitted root without creating a
    /// typed manifest or accepting an arbitrary raw JSON value.
    ///
    /// The first segment must be an object member key. A following index may
    /// select array elements, and subsequent keys may select nested members.
    /// An empty path returns `None` because the root object retains outer
    /// whitespace and is not itself represented by a child value slice.
    #[must_use]
    pub fn value_at_path(
        self,
        path: &[ProfileJsonPathSegment<'_>],
    ) -> Option<ValidatedProfileJsonValue<'bytes>> {
        let (first, remainder) = path.split_first()?;
        let ProfileJsonPathSegment::Key(name) = first else {
            return None;
        };
        self.root_member_value(name)?.at_path(remainder)
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

impl<'bytes> SyntaxCursor<'bytes> {
    const fn for_admitted(bytes: &'bytes [u8], values: usize) -> Self {
        Self {
            bytes,
            cursor: 0,
            maximum_nesting: MAXIMUM_JSON_NESTING,
            maximum_object_members: MAXIMUM_JSON_OBJECT_MEMBERS,
            remaining_values: values,
            values: 0,
        }
    }
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

// Two passes prevent partial writes on wrong-size caller buffers.
// Valid JSON decoding never expands beyond the already admitted source token.
fn copy_decoded_utf8(
    scalars: ValidatedProfileJsonStringChars<'_>,
    destination: &mut [u8],
) -> Result<usize, ProfileJsonStringCopyError> {
    let required = scalars.clone().map(char::len_utf8).sum::<usize>();
    if destination.len() < required {
        return Err(ProfileJsonStringCopyError::BufferTooSmall { required });
    }
    let mut cursor = 0usize;
    for scalar in scalars {
        let next = cursor.saturating_add(scalar.len_utf8());
        let Some(target) = destination.get_mut(cursor..next) else {
            // The full capacity preflight proves this branch unreachable.
            return Err(ProfileJsonStringCopyError::BufferTooSmall {
                required,
            });
        };
        let _encoded = scalar.encode_utf8(target);
        cursor = next;
    }
    Ok(cursor)
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
