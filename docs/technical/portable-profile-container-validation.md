# Portable profile container validation

## Status

Verified implementation guide for the current `.atrament` container boundary.

## Purpose

This guide explains what Atrament currently validates after a `.atrament`
manifest and archive metadata have been parsed by a future adapter. It connects
the portable-profile ADR to the dependency-free container domain without
claiming that ZIP or JSON parsing already exists.

The authoritative design remains
`docs/adr/handwriting/portable-atrament-profile.md`. This guide documents
executable behavior only.

## Scope

This guide covers the current transport-neutral profile-container domain and its
checked-in regression evidence. It does not define a wire schema, archive
adapter, migration, or inspection command.

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

The current domain compares a supplied 256-bit digest value. It does not compute
SHA-256 itself.

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
- contains an empty segment;
- contains a `.` or `..` segment.

The domain treats path strings as archive-relative names. It does not URI-decode
percent-like text and does not perform filesystem path resolution.

The generated regression corpus currently exercises 162 combinations across
both admitted roots with ordinary, Unicode, space-containing, percent-like,
empty, traversal, and backslash-containing segments.

### Canonical archive evidence

The domain does not write ZIP bytes, but it validates adapter-observed canonical
encoding facts through `validate_profile_archive_encoding`.

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

ZIP/JSON parse failures, archive resource limits, hash-computation failures,
section-schema failures, and migration failures belong to future adapters or
section owners and are not manufactured by this pure domain.

## Verification

### Regression evidence

The executable profile-container fixture is
`tests/backend/handwriting-profile-container/domain/lib.rs`. It currently
covers:

- canonical ZIP storage/extras/ZIP64 relationships;
- archive inventory agreement and mismatch cases;
- path safety and duplicate declarations;
- deterministic archive ordering;
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
- SHA-256 computation over real entry bytes;
- numeric archive and section resource limits;
- typed section schemas and decoders;
- supported migrations and loss reporting;
- golden byte-stable ZIP and ZIP64 fixtures;
- a human-readable inspection command.
