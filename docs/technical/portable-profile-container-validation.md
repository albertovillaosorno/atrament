# Portable profile container validation

## Status

Verified implementation guide for the current `.atrament` container boundary.

## Purpose

This guide explains what Atrament currently validates and projects at its
portable-profile boundaries. It connects the container ADR to typed admission,
source-attached ZIP/JSON readers, complete in-memory artifact writing and
bounded verification of host-opened profile files.

The authoritative design remains
`docs/adr/handwriting/portable-atrament-profile.md`. This guide documents
executable behavior only.

## Scope

This guide covers the transport-neutral container domain and read-only
file-source and archive verification evidence. The Linux selected-path
composition can open an already classified profile pathname, but cannot
authenticate the caller gesture or host policy. It does not authorize a
persistent commit, migration, session acceptance or CLI inspection command.

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

### Bounded host-opened Import verification

`src/backend/profile-file-reader/adapter-outbound/lib.rs` accepts an
**already-opened** regular-file handle and a hard source byte ceiling.
It rewinds to offset zero, validates metadata size and actual read
length, and probes the real EOF before returning bounded owned bytes.
Directories, oversized sources, read failures and length drift refuse.
This is a read-only file transport, not a file picker or a stable
disk snapshot across concurrent same-length rewrites.

`src/backend/profile-open-file-import/composition/lib.rs` accepts
classified `ImportProfile` intent and a host-supplied reader callback.
It rejects any other purpose before reading and enforces the strictest
ZIP, typed-manifest and Import member-index ceiling before ZIP parsing.
Only after source-bound ZIP CRC, bounded root JSON, typed manifest,
exact inventory and every declared member SHA-256 pass does the composition
check the media type and bounded JSON object syntax of **every** `sections/`
payload.

Each section must declare exact `application/json` media type and has
independent byte, depth, object-key and value limits. Only then can the scoped
inspection callback access any verified member bytes; a bad second section
cannot expose the first. Assets remain opaque (but SHA-verified).
No typed handwriting section is decoded, no profile is accepted into a session
and no file is written.

A second `inspect_selected_profile_import` entrypoint instead
opens exactly the classified path through the secure Linux opener, then
invokes the same bounded source reader and complete verification gate.
It has no caller-supplied file-descriptor callback, so an absent selected
path cannot be replaced by a different valid host-opened source. Purpose
refusal precedes even the attempted open, and failure never invokes inspection.

`src/backend/profile-selected-file-open/adapter-outbound/lib.rs`
can securely open the **exact classified** ImportProfile pathname on
Linux with `openat2`, forbidding symlinks in every component and parent
traversal. It also refuses directories and other nonregular sources;
kernels lacking this policy and non-Linux platforms fail closed rather
than using a racy filesystem precheck. A returned descriptor remains
tied to its opened file despite subsequent path renames, but concurrent
file writes can still change its bytes. The host must independently
authenticate the selected path's original caller gesture or host-policy
admission; the classified intent cannot prove that provenance itself.

Twenty-four composition tests exercise open-descriptor and selected-path
paths, source transport, malformed archives, and complete integrity gates.
SHA-valid invalid section JSON or mislabeled JSON media type, duplicate
decoded keys, independent section byte, depth, key, and value ceilings, and
later invalid sections all refuse before inspection. A separately admitted
JSON section may coexist with an opaque, non-UTF-8 binary asset, whose bytes
remain available without parsing.

Five path-open fixtures cover link policy, relative paths, nonregular sources
and replacement after open. Missing selected sources, links and wrong purposes
refuse before inspection; a CRC-valid same-length forged member still fails
SHA-256. Typed profile sections, session Import acceptance, authentic caller
selection and atomic Export remain open.

### Local read-only profile inspection CLI (Linux)

`atrament profile inspect PATH` is an opt-in, local CLI operation. With no
arguments `atrament` still starts its usual disposable browser session;
unknown or incomplete arguments refuse before that session starts. The
inspection path itself never launches a browser, publishes a credential,
extracts a member to disk, or modifies session state.

The command passes its explicit path through the selected-file opener and
complete bounded ZIP, JSON manifest, inventory, SHA-256 and section-syntax
gate. The CLI reuses the central bounded section check instead of separately
parsing verified payloads. Only after full admission does stdout receive
canonical manifest JSON and verbatim section JSON.

Opaque asset names remain visible in manifest declarations, but asset bodies
are never printed. Section output uses path order independent of the original
manifest declaration order. A malformed section or mislabeled JSON section
fails before any stdout content; section JSON is not yet admitted to a
versioned handwriting schema.

This first CLI policy limits input archives to 4 MiB, including at most 64
ZIP members and 2 MiB per member. It limits manifest root JSON to 128 KiB,
requires object-shaped section JSON, and applies independent nesting, value,
feature and name ceilings. Unsupported required features and unsupported
platforms still fail closed. These are command-specific protective limits,
not portable format maxima or proof of a trusted picker gesture.

The eleven runtime-bootstrap integration tests exercise the real CLI process:
verified sections without opaque asset extraction, CRC-valid forged members,
invalid section syntax, missing or symlinked selected paths, and bounded
argument rejection. Linux fixtures additionally prove that opaque non-UTF-8
caller-selected path bytes survive CLI argv and that a later invalid JSON
section cannot publish even a previously valid section or manifest to stdout.
Browser launch recovery error behavior remains independently covered.

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

The reader projects `ProfileZipArchive::encoding_evidence()` from an already
constructor-sealed ZIP archive. Successful parsing proves stored-only members,
absent extras, ordinary ZIP32 sufficiency and absent ZIP64 records; the Import
preflight test now receives those facts from the parsed source rather than
reconstructing them as an independent caller claim. CRC-valid legacy header
versions and timestamps retain the same encoding evidence even when they fail
the stricter deterministic-writer metadata comparison.

The parser must reject compression, encryption, ZIP64 and platform extras
before such evidence exists. A negative fixture separately changes those
header fields to ensure no parsed archive can supply a misleading encoding
projection. This still does not prove manifest-to-root provenance or file path
selection authority.

The reader also records whether every checked ZIP entry has the deterministic
writer's fixed metadata: extraction/creator versions 20, the UTF-8 filename
flag, and zero DOS timestamps. `matches_fixed_writer_metadata()` is a
read-only, allocation-free indicator; legacy ordinary stored ZIP variants may
remain readable while returning `false`. Exact content SHA-256 fingerprints
still include those different metadata bytes. This indicator does **not**
prove canonical manifest JSON, typed profile identity, or Import authority.

`matches_stored_writer_framing()` strengthens that indicator with the exact
presence of `manifest.json` in the validated sorted archive. An empty or
rootless stored ZIP may have all fixed metadata fields but still cannot have
been emitted by the current writer, which requires that root.

The check does not allocate or mutate bytes, and its `true` result can coexist
with malformed root JSON because writer framing never interprets its opaque
root payload.

A cross-component test starts with a CRC-valid legacy DOS-timestamp variant,
recovers each original member from the reader and feeds it to the independent
stored writer. The result matches the original fixed-metadata ZIP byte-for-byte,
while the legacy complete-archive fingerprint remains different. This is a
**read-only proof of envelope compatibility**, not a deployed rewrite action.

Fifteen additional writer/reader differential fixtures alter creator version,
extraction version, the ASCII UTF-8 flag, DOS time and DOS date independently
on each of three member positions. All remain structurally readable, return
`false` for writer framing, retain the exact same member evidence and produce
the byte-identical fixed-metadata archive when deliberately reframed in memory.

A further 1,024-case seeded byte writer/reader corpus varies present entry
names, Unicode archive paths, stored payload lengths and opaque root bytes.
Each result must match the exact preflight count, retain borrowed members and
SHA-256 source identity, report fixed writer framing and serialize again into
byte-for-byte identical ZIP records. Changing only one matching local/central
DOS date preserves every member but must clear that writer-framing marker.

The reader recomputes ZIP CRC-32 over each bounded stored entry. This checks
transport framing only: **CRC-32 does not establish manifest SHA-256
integrity**. An additional root-manifest accessor checks a caller-chosen byte
limit and strict UTF-8 before yielding borrowed text; it does not parse JSON.

Before allocating member indices, the parser also verifies that declared
entry counts fit within the measured central-directory byte length. It then
uses a fallible exact-size reservation, returning `AllocationFailed` if that
bounded index cannot be allocated instead of relying on an infallible reserve.
A tiny forged trailer cannot trigger a large index allocation.

A writer-to-reader boundary test proves that all 65,534 ordinary ZIP entries
fit and round-trip without ZIP64, including both end-record counts, sorted
member identity, planned byte length, and the full ZIP source fingerprint.
The next count (65,535) remains the ZIP64 sentinel and is refused without
allocating output bytes. Python's independent `struct`/`zlib` framing and
standard `zipfile` reader also verified the 65,534-entry boundary archive.
A permanent 7,077,690-byte SHA-256 golden protects the exact ZIP32 framing.

Adjacent negative tests reduce the entry allowance by one, the archive-byte
budget by one byte and the total payload allowance below the nonempty root.
They verify typed, fail-closed errors rather than silently admitting a partial
archive or allocating output beyond the selected bounds.

The reader also exposes `copy_observed_paths` for bounded caller-owned storage
of exact borrowed ZIP member names. A too-small destination reports the
required count before mutating any slot; surplus slots remain untouched and
no name is copied, allocated, normalized or resolved against a filesystem.
The existing allocating `observed_paths` helper remains for compatibility.

Tests exercise Unicode, empty and rootless ZIPs, and an exact 65,534-entry
ordinary ZIP32 boundary, including pointer identity into source archive bytes.
The existing read-only explicit Import preflight integration now consumes
this caller-owned borrowed path slice, including a missing-root rejection,
without separately allocating a list of observed archive names.

For callers that already have strictly ordered ZIP names, the typed
`ValidatedProfileManifest::admit_sorted_entry_inventory` performs its own
order/path checks and uses binary searches to prove every declared member.
It constructs the same sealed inventory without allocating new membership sets.
An opt-in `preflight_explicit_profile_import_sorted` shares all existing path,
encoding, resource-budget and manifest gates with the general Import preflight.

The sorted comparison rejects duplicates, decreasing order, missing root,
missing declared members and unexpected extra names with typed diagnostics.
A further 128-case matrix confirms sorted and general Import preflight agree
on purpose, compression, platform extras, ZIP64 and resource-budget outcomes
before the two inventory admission strategies can diverge.
An 8,192-case generated comparison checks agreement on acceptance with the
existing order-agnostic validator. A 65,534-name fixture exercises the ZIP32
entry ceiling, and the read-only real-ZIP fixture compares both preflights.

One additional CRC-valid ZIP fixture exercises that sorted Import path through
real SHA-256 verification of every declared member before exposing its bytes.
A second ZIP preserves the same names and admitted root but changes the member
and updates its framing CRC: structural preflight still succeeds, while exact
SHA-256 payload verification rejects the altered bytes. This demonstrates that
ordered inventory admission cannot substitute for cryptographic evidence.

The typed manifest must still be parsed, checked and matched against those
observations, and SHA-256 must be independently measured before any section
interpretation or profile acceptance. ZIP64, archive-file I/O, canonical JSON,
integration of digest evidence into actual Import, and section decoding remain
unimplemented.

### Source-backed root manifest SHA-256 fingerprint

After an ordinary stored ZIP is CRC-checked and its exact root object is
syntax-admitted, `ValidatedProfileZipRootJson::source_sha256()` fingerprints
those **unchanged UTF-8 JSON bytes** with SHA-256. The digest is derived from
the sealed archive-backed root slice without parsing a typed manifest, editing
JSON, normalizing Unicode, or reserializing the source.

Python `hashlib` goldens distinguish `{"a":1}` from `{ "a" : 1 }`, even though
both syntax views expose the same scalar. Another fixture changes a CRC-valid
non-root ZIP member without changing the root digest, explicitly proving this
is a **root-only fingerprint** and not a digest of the entire archive. It
cannot serve as verification of a declared root hash or proof that a typed
manifest was parsed, and confers no Import authorization.

### Complete stored-ZIP source fingerprint

The constructor-sealed `ProfileZipArchive` now retains an immutable borrow of
the *entire* checked ZIP byte slice, not just its individual stored member
payloads. Its `source_bytes()` points at the exact caller-supplied archive and
`source_sha256()` hashes that complete slice without copying or canonicalizing
it. This includes all local headers, payloads, CRC fields, directory records
and the end-of-directory marker.

An independent Python `hashlib` golden fixes the archive SHA-256 for a
root-only 126-byte ZIP. A second CRC-valid fixture changes just one non-root
member: the **archive fingerprint changes**, while the **root JSON fingerprint
remains identical**. This establishes exact scope, not a cryptographic
signature, selected filesystem source authority or typed profile admission.

A deterministic Python `zlib` fixture also supplies two different eight-byte
assets with identical CRC-32. Both remain valid stored ZIP frames, but their
archive-wide SHA-256 fingerprints differ while their root fingerprints stay
equal. CRC-32 protects ZIP framing against accidental damage; it is not a
cryptographic identity or authenticity guarantee.

### Deterministic ordinary stored-ZIP byte writer

The separate `atrament_profile_stored_zip_writer` application frames
already-selected member names and complete byte slices into an ordinary,
stored-only ZIP **in memory**. It does not write files, encode typed JSON,
choose profile field names, or establish Export authority. The required root
is the exact `manifest.json` name; other paths use the shared validated
section-or-asset path contract without normalization or URI decoding.

All member counts, names, strict raw UTF-8 byte order, individual and total
payload sizes, ordinary ZIP32 offsets, directory sizes and complete output
byte budget are checked **before any output allocation or output write**.
The original writer reserves one bounded `Vec` and emits local headers and the
central directory directly into it, without a second directory byte buffer.
Failed allocation returns a typed error, never partially exported data.

The new `write_stored_profile_zip_into` uses the same exact byte-emission path
but writes into a caller-owned, pre-sized `&mut [u8]`, allocating no output
storage. It rejects a short destination with the precise required byte count
before modifying any output slot, and leaves surplus destination bytes intact.
Both forms reuse the exact planned length, path checks, local-header CRC-32,
ZIP32 directory and end record; neither writes to a filesystem or grants Export.

A 1,024-case generated fixture compares the caller-owned output slice with
the existing owned byte vector for varying Unicode names, opaque root JSON
bytes and binary payloads. Additional tests cover empty/one-byte-short buffers,
preflight error precedence before any destination mutation and untouched extra
capacity. The full 65,534-entry ordinary ZIP32 golden is reproduced exactly in
caller-owned memory, including SHA-256 and its seven-megabyte framing budget.

The cross-component read-only writer/reader/typed-inventory test now starts
with caller-owned ZIP bytes, checks the sealed ZIP-derived encoding evidence,
copies exact member-name references into bounded storage and admits sorted
inventory without a new membership set. It verifies all declared member
SHA-256 digests against those unchanged output bytes before exposing any
member payload to downstream code. A ten-case error matrix also confirms that
planning, allocating writing and caller-owned writing return identical input
preflight errors before modifying caller storage.

`planned_stored_profile_zip_bytes` also returns the exact archive byte count
without allocating or computing CRC-32. It shares every path, order, budget and
ZIP32-limit check with actual writing; tests compare accepted output lengths
and all preflight error classes across bounded fixtures. This preview does not
reserve memory, bind a source or grant permission to write a destination.

Framing always fixes version to 20, UTF-8 flag, zero timestamps, stored method,
zero attributes and empty comments/extras. It computes CRC-32 once from each
member's exact payload, then reuses that local-header checksum for the central
directory without rescanning potentially large member bytes. It emits a
comment-free end-of-directory record.

The writer never uses data descriptors, compression, encryption or ZIP64.
It rejects ordinary-ZIP sentinel values rather than misrepresenting large
archives.

An independent Python `struct`/`zlib`/`zipfile` golden fixture establishes
byte-for-byte output for a root-only archive. Python's standard `zipfile`
reader also verifies a four-member archive's stored method, UTF-8 flags,
payloads and CRC checks; its independent SHA-256 digest is now a permanent
multi-member output regression. An additional one-megabyte deterministic
binary fixture checks local and central directory CRC fields against an
independently computed Python `zlib` oracle. Cross-component tests round-trip
empty and Unicode-named members through Atrament's strict ZIP reader, including
the 65,535-byte filename boundary and 65,535-entry ZIP64 sentinel refusal.

Additional integration tests use the generated ZIP with an independently
admitted typed entry inventory and SHA-256 verification. Conversely, framing
still admits an opaque malformed JSON root that the strict root JSON reader
rejects. This makes the separation between CRC-correct ZIP structure and
actual typed manifest acceptance executable rather than merely descriptive.

A deterministic 1,024-case filename corpus checks the writer's decisions
against the existing portable-profile path domain across Unicode, escapes,
control characters, traversal and literal percent signs. A separate 4,096-
mutation byte corpus flips one bit in emitted ZIP records and either obtains a
typed reader failure or confirms that admitted names and payload bytes remain
identical. Permissible irrelevant metadata changes do not become new content.

The strict ZIP reader also exercises an 8,192-case seeded mutation corpus
against a three-entry archive with independent Python `zlib` CRC values.
Replacement, insertion, deletion and duplication operations must never panic,
and any surviving stored-ZIP admission must preserve all declared member bytes,
names, CRCs and the syntax-admitted manifest root. Branch guards ensure
multiple malformed framing and checksum outcomes occur.

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

The provisional Linux CLI above exposes the fully verified manifest and
syntax-checked section objects without printing opaque asset bodies.
Versioned typed handwriting section decoding remains unimplemented. The
complete accepted design still requires typed and cross-platform inspection;
read-only operation must never bypass profile byte admission.

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

### Typed manifest JSON decoding (read-only)

`atrament_profile_manifest_json_codec` now consumes constructor-sealed JSON
root evidence and produces the existing typed `ProfileManifest` without opening
a file or mutating any accepted profile. The first-release decoder requires
`container_version`, `profile_identity`, `required_features`,
`optional_features`, and `entries` as exact decoded object names. Every entry
requires `path`, `media_type`, `byte_length`, and `sha256`; digests must have
exactly 64 lowercase hexadecimal scalars and lengths must be plain `u64`
integer tokens. Unknown wire keys reject rather than silently losing data;
unknown optional feature identifiers remain typed and unchanged.

Caller-owned ceilings separately bound entry count, each feature-list count,
and each decoded string's UTF-8 bytes. The upstream JSON syntax gate already
bounds total source bytes, nesting, object keys and JSON values, rejecting
ambiguous duplicate decoded keys before this decoder runs. These independent
limits prevent a valid but oversized manifest from bypassing resource policy.

The regression in
`tests/backend/profile-manifest-json-codec/adapter-inbound/lib.rs` checks all
wire fields, Unicode escapes, alias collisions, missing/unknown/wrongly typed
keys, limits, numeric ambiguities and malformed SHA-256. Its real stored-ZIP
fixture passes the decoded manifest to sorted Import preflight and complete
member SHA-256 verification. A CRC-correct modified member fails digest
admission even though the original root remains syntactically valid.

A 4,096-case deterministic mutation fixture also exercises raw JSON byte
replacement, insertion, deletion and duplication plus value-level integer,
digest, whitespace and decoded NUL-string variations. Repeated syntax and
typed decoding results must match exactly; each operation is exercised 512
times, and the corpus requires both rejection stages and accepted typed values.
This is regression fuzzing, not a claim of complete coverage-guided fuzzing.

The decoder also accepts the source-sealed `ValidatedProfileZipRootJson`
into a `DecodedProfileZipManifest` with a private constructor. That result
borrows the exact checked ZIP that supplied the manifest, retains its typed
fields and binds the source-root SHA-256 fingerprint. Callers cannot associate
unrelated ZIPs and typed manifests by constructing this wrapper themselves.
The original standalone syntax decoder is still available but carries no ZIP
association, so downstream Import should prefer the sealed ZIP entry point.

This is a reader-side schema contract, not canonical manifest JSON writing or
a selected-file Import. Selected-file provenance, filesystem admission, real
section schemas, profile acceptance and canonical writing remain outstanding.
No path inside the manifest conveys file read or write authorization.

## Verification

### Current adapter implementation status (2026-10-27)

The codebase now has deterministic stored-only ZIP32 bytes and a checked ZIP
reader, exact SHA-256 verification of declared members, a source-sealed typed
JSON manifest decoder, and canonical JSON serialization. Export assembly now
combines admitted typed manifests with every verified supplied payload into an
owned full ZIP artifact, or writes it into caller-owned memory after length,
digest and archive-budget checks. An independent Import verification fixture
reopens that exact emitted archive and accepts all declared members.

The preceding milestones describe how these boundaries were established; old
phrases stating that a ZIP or JSON writer is missing are historical, not the
present implementation status. Portable byte production does not establish
file authority, persistent Export, accepted-session Import, section schemas,
ZIP64 support, migration or a human-readable inspection command.

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

- typed handwriting section schemas, encoders and decoders;
- section-specific resource limits and admitted profile semantics;
- supported migrations, compatibility and loss reporting;
- ZIP64 implementation and golden byte-stable ZIP64 fixtures;
- versioned typed and cross-platform profile inspection;
- trusted selected-file origin and target admission;
- durable atomic Export, retry recovery and accepted-session Import.

The complete profile artifact assembly refuses JSON section declarations
whose media type differs from exact `application/json`. After verifying all
member lengths and SHA-256 digests, it also validates every section as an
independently bounded JSON object before emitting any artifact. The owned
writer, byte-count planner, and caller-owned writer share the same gate, while
opaque asset bodies remain uninterpreted; typed handwriting schemas remain
open.

Canonical JSON, ordinary stored-ZIP32 reading and writing, fixed writer ZIP
metadata, whole-archive hashing and source-bound Import payload verification
are already covered by the current adapters and integration tests.
