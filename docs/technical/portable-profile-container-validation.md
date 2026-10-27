# Portable profile container validation

## Status

Verified implementation guide for the current `.atrament` container boundary.

## Purpose

This guide explains what Atrament currently validates and projects at its
portable-profile boundaries. It connects the container ADR to typed admission,
limited ZIP/JSON readers and byte-only framing output without claiming that
a typed manifest has been decoded from any ZIP root.

The authoritative design remains
`docs/adr/handwriting/portable-atrament-profile.md`. This guide documents
executable behavior only.

## Scope

This guide covers the transport-neutral profile-container domain and its
checked-in regression evidence. It does not define a JSON wire schema, trusted
file-open and commit adapters, migration, or inspection command.

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

### Deterministic ordinary stored-ZIP byte writer

The separate `atrament_profile_stored_zip_writer` application frames
already-selected member names and complete byte slices into an ordinary,
stored-only ZIP **in memory**. It does not write files, encode typed JSON,
choose profile field names, or establish Export authority. The required root
is the exact `manifest.json` name; other paths use the shared validated
section-or-asset path contract without normalization or URI decoding.

All member counts, names, strict raw UTF-8 byte order, individual and total
payload sizes, ordinary ZIP32 offsets, directory sizes and complete output
byte budget are checked **before any output allocation**. Only one bounded
`Vec` allocation is reserved; local headers and the central directory are
then emitted into the same vector, without a second directory byte buffer.
Failed allocation returns a typed error, never partially exported data.

Framing always fixes version to 20, UTF-8 flag, zero timestamps, stored method,
zero attributes and empty comments/extras. It computes CRC-32 from the exact
member payload and emits a comment-free end-of-directory record. The writer
never uses data descriptors, compression, encryption or ZIP64, and rejects
ordinary-ZIP sentinel values rather than misrepresenting large archives.

An independent Python `struct`/`zlib`/`zipfile` golden fixture establishes
byte-for-byte output for a root-only archive. Cross-component tests round-trip
empty and Unicode-named members through Atrament's strict ZIP reader, including
the 65,535-byte filename boundary and 65,535-entry ZIP64 sentinel refusal.
These results establish deterministic **ZIP framing only**: manifest JSON
canonicalization, unknown-field preservation, typed serialization, ZIP64,
trusted file commit and accepted profile Export remain unresolved.

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
validation.

The root lookup uses the archive's already validated sorted index for exact-
name binary search, matching verified payload lookup rather than scanning
independently. CRC-correct ZIP fixtures prove the root is found between asset
and section entries. Nested `assets/manifest.json` or `sections/manifest.json`
cannot substitute for a missing exact root.

Missing or invalid UTF-8 roots and root byte-budget excess retain the
existing ZIP errors; malformed JSON and node/depth excess return separate
syntax errors. Real ZIP fixtures prove that valid CRC-32 cannot establish
JSON syntax, object-key uniqueness or typed profile acceptance.

Decoded-key comparison never case-folds or normalizes Unicode; raw and escaped
spellings compare by exact Unicode scalars. The fixed-size key index holds
source ranges and a fast fingerprint, always checking actual decoded names
when fingerprints match. No object member text is copied to the heap.

A generated regression checks more than 4,000 escaped-versus-plain name
pairs across varied key lengths and character positions. Separate sealed
projection tests traverse nested objects and arrays at the exact hard 64-level
ceiling, verify source-pointer provenance at the deepest scalar, and confirm
that an additional object level is rejected rather than silently admitted.
Independent real-ZIP fixtures prove that valid CRC-32 framing cannot hide
duplicate names or evade
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

### Sealed nested JSON views and ordered array projection

A `ValidatedProfileJsonObject` now projects one selected root member into a
constructor-sealed borrowed `ValidatedProfileJsonValue`. Its six possible
syntactic kinds are object, array, string, number, boolean, or null. No type
choice implies any required manifest field or admits a typed profile.

The value view can select an exact decoded member name inside an object or a
zero-based element inside an array. Missing names, array overruns, and wrong
value kinds return no view. Each successful projection reuses the already
validated root's complete grammar, unique decoded key names, and resource
budget; its returned slice is borrowed from the same original input bytes,
including numeric and string escape spelling. The decoded name comparison
remains Unicode-scalar exact without case folding or normalization.

Arrays additionally expose a fused, one-pass iterator returning sealed value
views in source order without allocating a JSON value collection. The test
corpus checks empty/nested arrays, wrong-kind selection, extreme indices,
source pointer provenance, every syntactic value kind and 256-element ordered
iteration. A CRC-checked stored ZIP fixture demonstrates that nested values
are borrowed directly from the actual ZIP root, not from reference literals.

**This is still structural JSON projection, not typed schema parsing:** there
is no canonical JSON writer, selection of manifest field names, semantics for
unknown fields or numbers, or proof connecting a typed manifest to the ZIP root.

### Field-neutral scalar JSON interpretation

One sealed JSON value additionally offers allocation-free Unicode-scalar
iteration for string contents, distinguishing escaped characters, control
escapes and valid UTF-16 surrogate pairs from their unchanged source spelling.
The iterator never case-folds or normalizes distinct Unicode sequences.

The same sealed value can read exact JSON booleans and null, or interpret a
plain nonnegative decimal integer only if every byte is a digit and the
result fits `u64`. A negative, fractional, exponent-form, or overflowing
number remains valid JSON syntax but cannot silently become integer metadata.
A string containing digits cannot be coerced into a JSON number, either.

These helpers are deliberately not field-level validators: the future typed
manifest codec must choose which properties allow strings, booleans, null, or
numbers, plus canonical writer rules. CRC-checked ZIP integration fixtures
show the decoded value still borrows its source from the real manifest entry.

### Lossless object-member enumeration

Constructor-sealed JSON object views also support ordered member iteration,
including root fields and fields nested inside array elements or objects.
Each member exposes the exact source JSON key token, an allocation-free stream
of its decoded Unicode scalars, and a borrowed, sealed JSON value. Names are
compared without case folding, Unicode normalization, or URI-decoding.

The iterator does not prescribe required or optional wire fields. Its original
key spelling and associated value bytes can be inspected without discarding
unknown keys or pretending they match known names. A sealed field can
also borrow the exact source span from the original key opening quote through
its value end, retaining the colon and intervening whitespace but excluding
neighboring commas and trailing object whitespace. This evidence is still not
a canonical output writer or an accepted optional-field preservation policy.

Nested siblings remain separate objects, empty objects exhaust safely, and
128-member boundary tests
preserve original order. A CRC-checked ZIP fixture checks that member key and
value bytes come from the admitted ZIP root, not independent reference text.

A named object field can now be selected as a complete sealed key/member/value
triple (`root_field` or `object_field`). Both lookups share the source-order
iterator's decoded-name comparison and eliminate the previous independent
object-member scanner. Existing value-only projection delegates to the same
field evidence, ensuring raw member spans and extracted values cannot choose
different keys.

Unknown-field retention semantics, canonical JSON rewriting, manifest schema
admission, and complete profile import remain tasks for a future typed codec.
A deterministic 128-field corpus alternates raw and Unicode-escaped key tokens,
then compares every streamed member with exact decoded-name lookup and verifies
that all key and value slices point back into the original input bytes.
A separate single-byte mutation corpus exercises safe traversal only when the
complete changed document still passes strict syntax admission.

Unicode scalar boundaries receive an additional deterministic matrix covering
all 1,024 possible high-surrogate code units combined with six representative
low-surrogate boundaries, decoded as both object keys and string values.
Invalid high-surrogate followers and every isolated low-surrogate code unit
are rejected in both key and value positions. A separate temporary
differential run compared 13,784 bounded inputs with Python's independent
JSON decoder, configured for strict duplicate-key and surrogate admission,
and found no disagreements. This is corroborating test evidence, not an
additional runtime dependency or a frozen manifest wire schema.

### Constructor-sealed structural JSON path selection

A caller-provided path can select a JSON value by ordered decoded-name `Key`
and zero-based `Index` segments, without allocating or transforming the parsed
JSON tree. Root traversal requires a first object key and refuses an empty
path; traversal from an already selected value accepts an empty path as the
same sealed view. Wrong kinds, absent decoded names and array overruns return
no value, including `usize::MAX` indices.

The path grammar is not JSON Pointer: a numeric object key remains a `Key`,
never an implied array index, and no slash or tilde decoding is performed.
Complete input syntax, decoded-key uniqueness, Unicode validity and resource
ceilings are admitted *before* a path is evaluated. The selected value still
borrows original source bytes with unchanged JSON escape and number spelling.

Integration fixtures traverse a nested path into real CRC-verified ZIP root
bytes and check source-pointer provenance. Further tests cover escaped root
names, mixed nested object and array segments, sparse/wrong-kind paths and
the hard JSON nesting ceiling. No path is a manifest schema, filesystem path,
permission, accepted profile, or canonical encoding decision.

### Caller-bounded decoded JSON string copies

A sealed JSON string value can copy decoded Unicode scalars into caller-owned
UTF-8 storage using `copy_string_utf8`. A two-pass, allocation-free copy first
computes the exact decoded byte count and refuses a short destination without
writing any bytes. The distinct errors are wrong JSON kind and too-small
buffer, with the latter reporting the number of bytes required.

Successful copying preserves embedded NUL characters, escaped controls and
supplementary Unicode scalars without case folding or normalization. Returned
length is the number of written bytes, not a C terminator; any unused caller
buffer remains unchanged. Test cases cover exact and zero-size buffers,
UTF-8 width transitions, UTF-16 surrogate pairs and distinct Unicode scalar
sequences. A real CRC-verified ZIP fixture proves that decoding does not
rewrite or separate the sealed original manifest value bytes.

The same bounded Unicode copy engine now powers `copy_key_utf8` for sealed
object fields. A future decoder can preserve a key's exact JSON token while
copying its decoded name into caller-owned storage, even when the original
key uses escapes or a UTF-16 surrogate pair. Caller capacity is preflighted
before any mutation, matching the value-copy behavior, and a CRC-verified ZIP
fixture checks root-key copies with no change to original manifest bytes.

Tests also cover empty, composed/decomposed, astral, and embedded-NUL keys,
plus all 128 source-order object members at the hard ceiling. String keys are
still not normalized or interpreted as manifest feature and path authorities.

An additional deterministic 5,120-scalar sweep crosses BMP and
supplementary Unicode planes using a nonrepeating stride. For each valid
scalar, the name and value use equivalent escaped JSON source bytes; decoded
field lookup, key copy and value copy must agree byte-for-byte. The test
preflights a one-byte-short buffer for each value and confirms both caller
buffers remain unmodified before accepting exactly sized outputs.

String copying is not canonical JSON serialization or profile field
validation. Future schema codecs must choose field names, string budgets,
allowed characters and normalized identities, and must preserve unknown data
according to the still-unfrozen wire contract.

### Archive-bound root JSON syntax evidence

The ZIP reader also exposes `admitted_root_json` to pair the exact supplying
CRC-checked, stored-only ZIP archive with its resource-bounded, syntax-checked
root JSON object in one constructor-sealed borrowed view. A caller can inspect
`archive()` and `root()` without constructing an independently supplied archive
association or guessing that similarly named members share manifest authority.

A two-archive fixture keeps the same member names but changes the root JSON
payload and its valid CRC. Each sealed view retains only its own archive and
points into that archive's exact stored root bytes; malformed JSON and root
byte-budget failures return the same typed errors before any paired view exists.

The pairing **does not** establish provenance of a separately constructed typed
`ProfileManifest`, prove SHA-256 over declared entries, or authorize any file or
session effect. Typed field decoding from the paired root remains necessary
before Import can claim manifest-to-archive correspondence.

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

### Incremental, fixed-memory entry SHA-256 verification

The FIPS maximum message length now has an explicit, testable
`checked_sha256_message_bytes` helper shared by the streaming hasher and
entry verifier. Synthetic arithmetic tests exercise the largest admissible
byte count, its zero-length edge and the first overflowing byte without
allocating unrealistic multi-exabyte buffers. This is a cryptographic bit-
length limit only, never an acceptable application resource budget.

The same dependency-free SHA-256 domain now offers a 64-byte incremental
`ProfileSha256State` for already bounded chunks, independent of filesystem or
ZIP transport. Updates preserve pending partial compression blocks, exact
byte counts and FIPS message-length limits, rejecting overflow before any
state mutation. Finishing consumes one state value and pads using its exact
accumulated bit count.

`verify_profile_entry_chunks` accepts already admitted ordered byte slices and
one existing typed declaration. It rejects the first known length overrun
before hashing that chunk, reports complete-stream underflow after iteration,
and compares SHA-256 only for a matching declared byte length. If an overrun
occurs, the reported observed length is the first exceeded prefix, not a claim
that unseen later chunks were consumed.

Regression cases include the million-byte SHA-256 vector; every split of all
lengths 0 through 256; padding boundaries, empty chunks and varied strides;
copyable prefix state divergence; length-before-digest failure precedence; and
early stream consumption termination. A CRC-correct stored ZIP fixture proves
that a modified payload still fails its declared SHA-256 under chunked reads.
A companion `verify_profile_entry_chunks_fallible` API accepts chunks that
can report a separate upstream error. It propagates the original read failure
without reinterpreting it as an ordinary truncated stream, length mismatch or
completed digest—even if a complete byte prefix arrived first.

Ordinary and fallible iterators share one internal length-before-digest
accumulator, and overlong streams still stop without requesting subsequent
input. This remains a pure in-memory primitive, not an I/O adapter or
path-authority grant.

None of this freezes typed manifest field names or introduces actual file I/O.

### Bounded verification from already opened byte readers

The separate `atrament_profile_entry_stream_reader` inbound adapter now accepts
an already-open `std::io::Read` source, one typed entry declaration, and an
independent maximum byte budget. It refuses oversized or SHA-unrepresentable
declarations *before the first read*, then hashes using a fixed 8 KiB stack
buffer without retaining another copy of the source payload.

The adapter reads no more than the declared length plus one probe byte. An
exact prefix is not admitted without an actual successful EOF observation;
extra bytes fail with the first observed overrun, and early EOF reports the
short observed count before digest comparison. Errors from the original
reader remain `std::io::Error` values, including errors during EOF probing.

This EOF proof is scoped to the provided `Read` view, not automatically to its
underlying filesystem source. In particular, `Read::take` can manufacture
an apparently complete stream even when the original reader contains extra
bytes. Regression tests document that distinction, and future trusted
adapters must bind reader scope, source authority and actual transport length
independently before any Import acceptance.

Synthetic reader fixtures exercise short reads, the 8 KiB boundary, empty
entries, changed digests, injected read errors and refused byte budgets.
Malformed `Read` implementations that report more bytes than the supplied
buffer are rejected as `InvalidData` without unsafe slicing. Premature zero
reads remain EOF failures rather than permitting a retry loop to consume later
hidden bytes.

A CRC-checked ZIP fixture confirms that a changed payload remains detectable
when ZIP framing and CRC have been updated consistently. The adapter never
opens a path, establishes trusted source selection, decodes a section or
accepts an Import; blocking, cancellation and host deadlines remain external.

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

The sealed result also exposes `verified_entry_chunks` with a typed nonzero
chunk size. Consumers borrow original, consecutive source slices of a fully
verified member without allocating a second payload or claiming fresh source
authority. Tests compare the original pointer for each chunk at nine different
sizes and reject root, absent, near-matching and case-altered member names.

A present zero-byte verified asset yields a *present*, empty chunk iterator;
an absent asset yields no iterator. The future typed section decoder must still
provide its own format, size, streaming and Import acceptance policy.

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
