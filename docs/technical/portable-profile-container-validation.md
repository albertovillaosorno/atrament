# Portable profile container validation

## Status

Verified implementation guide for the current `.atrament` container boundary.

## Purpose

This guide explains what Atrament currently validates after a `.atrament`
manifest and archive metadata have been parsed. It connects the portable-
profile ADR to the dependency-free container domain and its new limited ZIP
byte reader without claiming that manifest JSON parsing already exists.

The authoritative design remains
`docs/adr/handwriting/portable-atrament-profile.md`. This guide documents
executable behavior only.

## Scope

This guide covers the current transport-neutral profile-container domain and its
checked-in regression evidence. It does not define a wire schema, archive
file-reading adapter, migration, or inspection command.

The transport-neutral domain is
`src/backend/handwriting-profile-container/domain/lib.rs`. It owns:

- `atrament.profile/1` as the current container behavior identity;
- the root manifest name `manifest.json`;
- safe non-manifest entry paths below `sections/` or `assets/`;
- required-versus-optional feature admission;
- declared byte length and SHA-256 evidence comparison;
- exact archive inventory comparison;
- canonical path ordering;
- typed section-versus-asset classification;
- canonical ZIP evidence for stored entries, platform extras, and ZIP64 use;
- unchanged-versus-changed rewrite disposition.

The domain performs no file I/O and has no ZIP, JSON, hashing, migration, or
renderer dependency.

### Bounded stored-ZIP byte reader

`src/backend/profile-zip-reader/adapter-inbound/lib.rs` is a separate,
read-only adapter. It parses **already-loaded** ZIP bytes rather than file
paths. The caller supplies maximum archive bytes, member count, per-entry
stored bytes, and cumulative stored bytes before the adapter allocates a
member index. It supports only an ordinary, single-disk ZIP archive with
stored (uncompressed) entries and a comment-free terminal directory.

The byte reader validates both the ZIP central directory and every local entry
header, requiring exact agreement of names, sizes, timestamps, CRC-32, flags,
method and version, as well as contiguous nonoverlapping members, canonical
lexicographic order, a supported UTF-8 name interpretation, and no prefixes or
unclaimed trailers. It rejects encrypted, compressed, descriptor-based, ZIP64,
extra-field, archive-comment, foreign-platform and other unsupported encodings
rather than trying alternate decoders. Unsafe paths are passed through the
shared portable-profile path validator before member bytes are exposed.

The reader recomputes ZIP CRC-32 over each bounded stored entry. This checks
transport framing only: **CRC-32 does not establish manifest SHA-256
integrity**. An additional root-manifest accessor checks a caller-chosen byte
limit and strict UTF-8 before yielding borrowed text; it does not parse JSON.

Before allocating member indices, the parser also verifies that declared
entry counts fit within the measured central-directory byte length. This
prevents a tiny forged trailer from triggering large index allocations.

The typed manifest must still be parsed, checked and matched against those
observations, and SHA-256 must be independently measured before any section
interpretation or profile acceptance. ZIP64, archive-file I/O, canonical JSON,
integration of digest evidence into actual Import, and section decoding remain
unimplemented.

### Bounded root JSON syntax admission

The independent `profile-json-syntax` domain now validates one complete
UTF-8 JSON root object with no heap allocations or field interpretation.
Callers set byte, value, per-object member, and container-nesting budgets.
Fixed ceilings of 128 keys per object and 64 containers bound key storage and
parser stack depth even when the caller requests higher limits.

Its grammar rejects malformed JSON strings, escapes, surrogate pairs,
invalid number syntax, missing separators, trailing tokens and root
arrays or primitives. It rejects duplicate **decoded** object keys at any
nesting depth, comparing scalar values so escapes cannot hide collisions.
The reader preserves the **exact** admitted byte slice and value count.

The ZIP inbound adapter exposes `manifest_json_object` to bind these syntax
checks to the actual root `manifest.json` entry after ordinary ZIP and CRC
validation. Missing or invalid UTF-8 roots and root byte-budget excess retain
the existing ZIP errors; malformed JSON and node/depth excess return separate
syntax errors. Real ZIP fixtures prove that valid CRC-32 cannot establish
JSON syntax, object-key uniqueness or typed profile acceptance.

Decoded-key comparison never case-folds or normalizes Unicode; raw and escaped
spellings compare by exact Unicode scalars. The fixed-size key index holds
source ranges and a fast fingerprint, always checking actual decoded names
when fingerprints match. No object member text is copied to the heap.

A generated regression checks more than 4,000 escaped-versus-plain name
pairs across varied key lengths and character positions. Independent real-ZIP
fixtures prove that valid CRC-32 framing cannot hide duplicate names or evade
caller object-member limits.

**This gate still does not parse manifest fields, enforce canonical JSON wire
encoding, or prove correspondence between an independently supplied typed
manifest and this archive's root bytes.** Those remaining obligations belong
to the future typed profile JSON codec and canonical writer.

### Read-only root JSON value projection

A constructor-sealed `ValidatedProfileJsonObject` now exposes
`root_member_json` for exact caller-selected top-level JSON member names.
The lookup compares decoded Unicode scalars, including surrogate-pair and
escaped aliases, after the complete document has passed all syntax and
unique-key checks. It cannot select a nested key as a root member.

The result is a borrowed source-byte slice of the original JSON value, not
a decoded or normalized JSON value. Whitespace inside containers, original
string escapes, and numeric spelling remain unchanged; missing names return
`None`. An empty string key is valid when explicitly requested.

The ZIP reader's `manifest_json_object` result can make that same projection
from the actual CRC-checked root bytes, without allocating values or reading
from a filesystem. Neither this generic lookup nor the sample fixture names
freeze manifest wire fields, prove a typed manifest's provenance, or authorize
profile acceptance. A future typed codec still owns those responsibilities.

### Exact entry SHA-256 verifier

The separate profile-entry-sha256 domain computes NIST FIPS 180-4 SHA-256 over
caller-bounded in-memory member bytes, with fixed-size working arrays and no
ZIP, JSON, filesystem, or session effects. Its verifier checks observed byte
length before computing the digest, then checks the exact digest against one
typed manifest entry.

The verifier does not grant path authority, parse the manifest, or accept an
import. Its regression suite checks known SHA-256 vectors (including one
million bytes), block and padding boundaries against independent reference
digests, unchanged bytes, changed bytes, and failure precedence. A
cross-component ZIP fixture additionally proves that a matching CRC-32 over
altered bytes cannot satisfy an unchanged manifest SHA-256 digest.

### Complete stored member SHA-256 projection

The ZIP inbound adapter can now take a constructor-sealed declared-versus-
observed inventory and verify every stored, CRC-checked member against its
declared byte length and SHA-256 before exposing any non-manifest payload
bytes through a separate borrowed, sealed result. It rechecks the exact ZIP
member-name sequence against the admitted inventory, including order, to
prevent substituting a different archive after preflight.

Each entry is resolved against the ZIP reader's sorted index, rather than
using a quadratic name scan. Failure of one entry blocks the complete
projection. The root manifest is deliberately excluded from the verified
section/asset accessor. This accessor now also uses the sorted ZIP index for
exact logarithmic-time member lookup, including Unicode names, while refusing
missing names, prefixes, percent-encoded aliases, and case changes.

The regression suite additionally proves that an admitted zero-byte asset
returns an actual empty byte slice rather than being confused with absence,
and that a later member's length mismatch precedes its incorrect digest.

The caller still must prove the supplied typed manifest originated in
this archive's root JSON, enforce caller or host path authority, and decode
the typed sections before session acceptance. The adapter has no filesystem
or session effect and the profile Import TODO remains open.

The integration fixture proves that the reader's exact observed entry names
can be passed to the existing explicitly authorized `ImportProfile` preflight.
It intentionally uses a synthetic, unverified manifest digest to demonstrate
that a successful archive framing check is not a completed import.

## Contract

### Validation order

A future archive adapter should keep the current fail-closed order visible.

### 1. Parse without interpreting section content

The adapter first obtains a root manifest value and archive-entry metadata. It
must not decode a typed handwriting section merely because ZIP or JSON syntax
was parseable.

The domain expects a typed `ProfileManifest` containing:

- `container_version`;
- `profile_identity`;
- required and optional feature identifiers;
- one declaration for every non-manifest archive entry.

Each entry declaration retains path, media type, uncompressed byte length, and
one exact SHA-256 digest value.

### 2. Admit the manifest

`validate_profile_manifest` rejects before section decoding when:

- the container version is not exactly `atrament.profile/1`;
- a required feature is not in the caller-supplied supported feature set;
- a non-manifest path is unsafe, noncanonical, reserved, or outside the two
  admitted roots;
- two entries declare the same canonical path;
- an entry has no media-type identity.

Unknown optional feature identifiers are retained and do not block admission.
The domain does not normalize, reinterpret, or discard them.

### 3. Compare archive inventory

`validate_profile_entry_inventory` compares the observed archive names with the
already validated manifest. The archive must contain exactly one root
`manifest.json` and every declared non-manifest entry exactly once.

The inventory rejects:

- a missing root manifest;
- duplicate observed names;
- unsafe observed names;
- undeclared observed entries;
- declared entries that are absent from the archive.

This step operates on entry names only. It does not trust or decode entry
contents.

### 4. Verify each entry before decoding

`verify_profile_entry` receives independently observed byte-length and digest
evidence for one manifest declaration. It checks byte length before digest.

A length mismatch rejects even when the supplied digest happens to equal the
manifest digest. A matching length with a different digest also rejects. Only
matching framing and digest evidence may proceed to a future section decoder.

After exact inventory admission, `ValidatedProfileEntryInventory::verify_entry`
can seal that successful comparison together with the exact manifest
declaration. The returned evidence carries only declaration plus observed
length/digest facts; it does not carry or decode entry bytes. Missing or
near-match paths remain absent rather than being normalized, URI-decoded, or
resolved through filesystem rules.

The container domain compares a supplied 256-bit digest value; it does not
compute SHA-256 itself. The separate verifier above can supply the independently
computed digest, but callers still must bind it to the exact admitted member.

### 5. Classify section and asset paths

`profile_entry_kind` classifies a validated path as either:

- `Section` for `sections/...`; or
- `Asset` for `assets/...`.

`profile_section_entries` returns only validated section declarations in
canonical path order. Opaque assets remain in the manifest but are deliberately
excluded from this default section projection.

This preserves the ADR rule that ordinary inspection need not extract or
interpret opaque assets.

### Path rules

A declared non-manifest path is rejected when it:

- equals `manifest.json`;
- contains a backslash separator;
- is outside `sections/` and `assets/`;
- contains a control character, including NUL, CR, LF or TAB;
- contains an empty segment;
- contains a `.` or `..` segment.

The domain treats path strings as archive-relative names. It does not URI-decode
percent-like text and does not perform filesystem path resolution.

The generated regression corpus now exercises 242 combinations across both
admitted roots with ordinary, Unicode, space-containing, percent-like, empty,
traversal, backslash-containing and control-bearing segments. Separate fixtures
verify that malformed declared and observed archive entry names cannot reach
section or asset decoding.

Manifest entry media-type identifiers also fail before decoding if the value
is empty or whitespace-only, or if it embeds a Unicode control character.
The typed `InvalidMediaType` failure preserves the affected entry path but does
not echo potentially unsafe media-type text. Printable parameterized values
such as `text/plain; charset=utf-8` remain accepted pending the eventual wire
schema and adapter MIME checks.

### Canonical archive evidence

The domain does not write ZIP bytes, but it validates adapter-observed canonical
encoding facts through `validate_profile_archive_encoding`. Successful admission
can also seal the exact observed evidence for downstream read-only use.

A canonical archive requires:

- every member stored without compression;
- no platform-specific archive extras;
- ZIP64 absent when ordinary ZIP limits are sufficient;
- ZIP64 present when the adapter reports that ordinary limits are insufficient.

The adapter remains responsible for actual ZIP thresholds, fixed timestamp and
attribute values, ZIP parsing, and ZIP64 record construction.

Canonical writer path order is lexicographic across the complete archive,
including `manifest.json`. `canonical_profile_archive_paths` and
`canonical_profile_entry_order` expose deterministic ordering without encoding
bytes.

### Rewrite behavior

`profile_rewrite_disposition` consumes application-owned complete-profile
change state.

- `Unchanged` permits preservation of the original archive bytes.
- `Changed` requires a canonical rewrite.

The container domain does not infer semantic equality from the manifest. The
application owning the complete profile must decide whether profile semantics
changed.

### Compatibility outcomes

The current first-release compatibility evidence is intentionally conservative.

| Input condition | Current outcome |
| --- | --- |
| `atrament.profile/1` with admitted required features | validate normally |
| prior container version | reject without mutation |
| future container version | reject without mutation |
| unknown required feature | reject without mutation |
| unknown optional feature | preserve without interpretation |
| corrupt digest evidence | reject before decoding |
| truncated entry evidence | reject before decoding |
| missing declared entry | reject before decoding |

No prior-version migration is claimed. The accepted ADR requires migrations to
preserve source evidence and report lossy transformations, but no concrete prior
profile schema has yet been frozen for implementation.

### Canonical JSON and section schemas

The ADR requires canonical UTF-8 JSON, typed versioned sections, and stable
identity independent of whitespace or object insertion order. Those requirements
are not yet executable in this repository.

In particular, the current container domain does not choose:

- manifest JSON field names beyond the in-memory Rust value names;
- canonical JSON number syntax;
- key ordering rules;
- concrete typed-section JSON schemas;
- section behavior versions;
- unknown JSON-field preservation mechanics;
- migration envelopes or migration history encoding.

Do not treat the current Rust structs as an accidental frozen wire schema.

### Inspection command boundary

The accepted design requires a human-readable inspection command that exposes
the manifest and typed sections without extracting opaque assets by default.
That command does not yet exist.

An eventual implementation should consume the same validated inventory and
entry evidence described above. It must not bypass container admission merely
because the operation is read-only.

## Failure Modes

The current domain fails closed for unsupported container versions, unknown
required features, unsafe or duplicate declared paths, missing media-type
identity, duplicate or unsafe observed archive paths, missing or undeclared
entries, byte-length mismatch, digest mismatch, compressed canonical entries,
platform-specific ZIP extras, and incorrect ZIP64 use.

These failures do not decode a typed section or mutate the supplied manifest.
Unsupported prior/future versions are rejected rather than silently migrated.
Unknown optional feature identifiers and opaque assets remain preserved data.

ZIP/JSON parse failures, archive resource limits, section-schema failures,
and migration failures belong to adapters or section owners and are not
manufactured by the pure container domain. SHA-256 computation is now available
as a separate byte-slice primitive, not as a complete Import adapter.

## Verification

### Regression evidence

The executable profile-container fixture is
`tests/backend/handwriting-profile-container/domain/lib.rs`. It currently
covers:

- canonical ZIP storage/extras/ZIP64 relationships across all 16 states through
  direct and sealed admission;
- archive inventory agreement and mismatch cases;
- path safety and duplicate declarations;
- deterministic archive ordering, including all 720 declaration orders for a
  six-entry fixture without mutating caller order;
- typed section projection without opaque asset extraction;
- current/non-current/partially unsupported compatibility outcomes;
- digest and truncation evidence;
- non-destructive rejection behavior;
- generated path-boundary coverage.

These are domain-level fixtures. They do not replace future byte-level golden
ZIP fixtures, canonical JSON fixtures, parser fuzzing, migration round trips, or
inspection-command tests.

### Developer rules

When extending this boundary:

1. Keep ZIP, JSON, hashing, and filesystem effects in adapters rather than this
   pure domain.
2. Validate names and inventory before typed section decoding.
3. Treat unknown required features as blocking and optional features as opaque
   preserved data.
4. Do not infer complete-profile equality from manifest metadata alone.
5. Do not silently migrate an unsupported version.
6. Add explicit compatibility evidence before admitting another container or
   section behavior version.
7. Keep new resource limits owned by a documented adapter/application contract
   rather than inventing numbers inside this domain.

### Open implementation work

The profile-container TODO remains open for:

- actual ZIP parsing and deterministic writing;
- fixed canonical ZIP metadata values;
- canonical JSON encoding and parsing;
- wiring verified SHA-256 bytes to a parsed, admitted profile manifest;
- numeric archive and section resource limits;
- typed section schemas and decoders;
- supported migrations and loss reporting;
- golden byte-stable ZIP and ZIP64 fixtures;
- a human-readable inspection command.
