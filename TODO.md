# Atrament TODO

Only unfinished product work appears here. P1 through P8 are the ordered path to
the complete first release: a disposable localhost notebook with a Rust
backend, TypeScript frontend, deterministic CPU renderer, PDF output, and safe
single-pen hardware operation. P9 is optional future hardware and does not block
the first release.

Architecture decisions live in `docs/adr/`. A task closes only with executable,
visual, or physical acceptance evidence appropriate to the claim; attractive
screenshots are never a substitute for semantic or geometric correctness.

## P1 — Localhost runtime and ephemeral session

### TODO - Keep all notebook state in memory

Hold documents, assets, undo history, previews, and derived plans only for the
active session with no database, autosave, hidden recovery file, or cloud copy.

Current executable evidence composes source-preparation draft text, accepted
semantic notebook revisions, and semantic Undo/Redo history under one
`SessionApplication` instance owned by the live process. Transport-neutral
command review, exact and caller-bounded batch analysis and simulation,
selection analysis, and atomic batch application also route through that owner
without exposing the concrete semantic service.

The localhost runtime still consumes only the established draft inbound port,
while the same application owner retains accepted semantic and history
authority for later admitted routes. `SessionApplication` is the only production
consumer of the concrete semantic-session service. No command transport or
writable scope is published by this composition evidence.

Dropping that owner and creating a fresh application yields empty draft fields,
no accepted revision, no history position, and no retained asset bytes; its
Debug projection does not expose private draft text or raw media. The active
application can retain already-validated raw bytes only after an accepted
semantic asset identity exists. It preserves the first retained byte sequence
for that identity and keeps those bytes process-local across semantic Undo/Redo.

After Undo followed by a new candidate branch, abandoned asset bytes cannot
reattach: the old accepted asset identity is absent from the new revision, Redo
is discarded, and newly accepted assets receive distinct identities with no
retained bytes. The process owner now also prunes retained bytes whose accepted
asset identities are unreachable from current, Undo, and Redo semantic state.
No-op batches, rejected candidates, bounded command rejection, and stale direct
edits preserve both Redo and its reachable bytes. Branch cleanup is selective:
bytes still reachable through current or Undo state remain retained.

All semantic mutation entry points in `SessionApplication` now share one private
branch-cleanup gate so this lifecycle rule is not duplicated per operation.

Formula, physical-page-profile, table-cell-span, and table-row-role replacements
are exercised through that same gate after Undo with Redo-only asset bytes:
successful changes prune the discarded branch, while exact-value no-ops preserve
Redo and its retained bytes. Malformed mathematics, invalid page geometry, and
invalid table grids also reject without dropping that branch or its bytes.
Direct text and bounded-batch branches are covered separately. Grapheme-range
editing now also preserves Redo and its reachable bytes on provider rejection
or semantic no-op, while an applied grapheme mutation prunes the abandoned
Redo-only bytes through the same gate.

Read-only grapheme position, signed-step, and selection inspection also route
through `SessionApplication`; semantic revision/target rejection occurs before
the boundary provider is consulted and cannot mutate history. Edit, Undo, and
Redo each invalidate older query revision identities.

A 256-state asset-history fixture undoes halfway, then branches: cleanup removes
exactly 128 Redo-only retained byte entries and preserves all 128 identities
still reachable through older history, including their exact byte sequences.

A process fixture now retains all three private draft fields, rejects one
oversized draft replacement with the shared resource-limit diagnostic,
retains bytes for two semantic assets, commits text and asset-reference history,
computes a revision-bound direct-edit preview and version-bound direct-edit
proposal, measured pagination, fixed-region overflow diagnostics and layout-only
Export preflight. It also
owns one terminal media job with a registered waveform still requiring cleanup.
Both
orderly and forced
termination leave a fresh process with empty draft fields, no accepted revision,
history, bytes, or prior media-job authority; derived results and cleanup
bookkeeping exist only in the terminated process. Browser edits send complete
authenticated replacements and no browser persistence API is used.

This evidence establishes raw-byte ownership after a separate ingestion
boundary and one semantic direct-edit preview lifecycle; it does not implement
media decoding, format validation, clipboard or file intake, or temporary
conversion files. The task remains open until the remaining preview and
diagnostic producers, renders, plans, and media operations join the same
lifecycle invariant.

### TODO - Implement explicit import and export

Allow deliberate notebook bundle and `.atrament` profile reads or writes at
caller-selected or explicitly host-policy-admitted paths without converting them
into background persistence. Keep paths mentioned only in notebook, source,
diagnostic, clipboard, or model-response text as data rather than Export
authority.

Current export design evidence freezes accepted-revision binding, caller-owned
or
explicit host-policy output intent, explicit path and overwrite behavior,
blocking validation, temporary cleanup, file-commit semantics, output identity,
same-retry lost-receipt recovery, external target drift handling, cancellation,
typed output results, concurrency, and browser/CLI/MCP parity. File adapters and
format-specific execution remain open. Execution is also blocked on a typed
exportable-artifact payload: current backend code has semantic layout preflight
but no renderer or serializer result value whose admitted bytes a file adapter
can commit. Filesystem writes must not guess that missing artifact-ownership
boundary.

The shared derived/output result domain now freezes all 11 first-release core
result classes and their operation applicability across Render, Plan, and
Export.
It remains vocabulary only: no output operation, receipt normalization, retry
recovery, path handling, or file effect is implemented by that domain.

Its effect disposition separately distinguishes current-call read-only
projection, current-call file commit, recovered prior file commit, and known no
new effect without claiming any receipt or retry-store implementation.

A combined operation/result helper now exposes that disposition only for an
applicable Render, Plan, or Export pair, so callers cannot skip applicability
validation and accidentally project an operation-incompatible result.

### TODO - Prove session destruction and temporary cleanup

Close, refresh, cancel, crash, and restart fixtures must show that ephemeral
state and media intermediates disappear while explicit exports remain intact.

Current runtime evidence proves orderly process termination releases the old
loopback listener, restart generates a fresh session credential, and a stale
credential cannot authenticate to the new process.

The secret adapter separately pins that independent operating-system-backed
generations do not reuse the same encoded credential. Process-level draft
fixtures
also write and read task, source, and raw-response text, then verify both
orderly restart and forced process death yield empty fields in the fresh
session. A checked-in application-process fixture now also populates an accepted
semantic revision containing two semantic asset records and a figure reference,
retains distinct raw byte sequences for both assets, commits text and
asset-reference history, and leaves one terminal media job with registered
waveform cleanup pending in process memory. Orderly exit and forced process
death both leave a fresh owner with no accepted revision, history position, raw
bytes, or prior media-job authority.

Media decoding and actual temporary
conversion-file cleanup remain open because no filesystem intermediate exists
yet.

While private draft text is live, the runtime holds no writable regular-file
descriptor and changes no declared repository runtime-root file.

Checked-in browser policy tests guard one-time launch-credential fragment
consumption, credential invalidation on `pagehide`, in-flight handshake and
draft request cancellation, pending clipboard/draft invalidation, session-text
clearing, and bfcache subtree scrubbing. A `401` during handshake, draft
hydration, or draft replacement now invalidates the same page-session work and
scrubs all four session text surfaces before requiring restart.

Aborted draft requests and stale `413` diagnostic-body completions cannot
repopulate status text after invalidation. Concurrent field failures also remain
visible until the failed field succeeds; another field cannot announce
`Session ready` while work or failure remains. Refresh cannot recover the launch
credential from the rewritten browser URL or persistence API.

Handshake response admission now invalidates `401` directly from the status and
parses JSON bodies only for the two body-bearing handshake result classes,
`200` and `409`; unrelated statuses cannot delay failure on an irrelevant body.
The localhost adapter also JSON-escapes compatible and required backend version
identities, so application-owned quotes, backslashes, controls, and Unicode
cannot corrupt the handshake response syntax.

The task remains open until temporary media exist and end-to-end fixtures also
cover browser close, refresh, cancellation, media cleanup, and explicit-export
survival.

### TODO - Define one typed diagnostic envelope

Represent field, object, page, source, glyph, collision, capability, renderer,
and hardware errors with stable codes and actionable locations.

Current executable evidence now implements the versioned
`atrament.diagnostic/1` backend domain model. Handshake incompatibility and
resource-limit results preserve their application result classes while carrying
shared diagnostic sets with stable codes, severity, blocking disposition,
semantic locations, typed evidence, remediation, operation binding, and
explicit set completeness. The browser admits the same namespace and
completeness metadata; handshake mismatch projection also requires the reported
expected identity to equal the browser's current value for that exact dimension.

The localhost adapter requires exactly one route-specific application
diagnostic for the current singleton handshake-mismatch and draft-resource
response schemas.
Empty, multi-item, cross-operation codes, or wrong operation bindings fail as
`invalid_diagnostic`. Handshake projection also requires one `RequiredVersion`
evidence item that exactly matches the reported dimension and expected identity,
so transport neither invents, truncates, nor relabels application evidence.

Layout now provides the first accepted-revision producer beyond handshake and
session draft: fixed-region overflow emits stable blocking diagnostics with
semantic object/page locations, typed boundary and physical-length evidence,
operation binding, and complete-set semantics. Their current opaque object,
page, and revision identities use one in-process Debug-shaped projection only;
no browser, CLI, MCP, or file-format identity encoding is admitted by this
evidence. A layout-only Export preflight preserves those diagnostics through
that same internal projection rather than interpreting prose.

A transport-neutral semantic-command Apply producer now exists, but its typed
results do not yet carry `DiagnosticSet`. The shared envelope now reserves the
stable `atrament.semantic-command.precondition-rejected` code and typed local
precondition condition/failure evidence for family, kind, owner, target,
editability, and exact-value checks. Domain fixtures prove the code, Semantic
Validate binding, command/object/field locations, and typed evidence shape
without inventing adapter prose.

Complete external diagnostic identity projection remains open because accepted
semantic identities are intentionally opaque and command identities are still
generic in the pre-normalization batch. No stable diagnostic identity text or
context binding exists yet, so the application must not leak Debug formatting to
satisfy the fixture. Render, full Export, and Plan producers also remain open,
and CLI/MCP parity cannot yet prove cross-capability semantics.

## P2 — Semantic notebook and physical layout

### TODO - Implement the semantic notebook model

Represent notebooks, pages, flows, blocks, spans, formulas, tables, figures,
styles, assets, constraints, output profiles, and provenance with stable IDs.

Current executable evidence defines transport-independent typed values for those
semantic families, separate opaque candidate, accepted, and revision identities,
non-recycling active-session allocation, unresolved semantic blocks, and exact
extension-data preservation. Citations, definitions, footnotes, quotations,
source notes, and margin notes are now distinct semantic block kinds whose
editable inline spans retain their own identities, style references, and
provenance references rather than being flattened into paragraph text.
Explicit candidate acceptance validates
duplicate, dangling, and wrong-kind references before mutation, promotes
candidate-local identities through one backend-owned mapping, and commits one
new accepted revision atomically while preserving nested semantic references.
A specialized-reference matrix now pins wrong-kind rejection for figure assets,
block provenance/style, and figure-caption provenance/style without changing the
current accepted revision.

A direct accepted-text edit now preconditions the exact current revision,
preserves all semantic identities while replacing one admitted inline text
identity, creates one new revision only for a real change, and rejects no-op,
stale, unavailable, or non-text targets without mutating accepted state.
Mathematical blocks now carry an explicit inline, display, or aligned mode and
exact authored source; candidate acceptance validates that source before
allocating accepted authority.

A read-only semantic descriptor now classifies stable identities and their
direct structural owners without exposing storage or wire shape. Exact-revision
inspection returns that kind-and-owner descriptor with typed stale, missing, and
empty-session outcomes and no mutation. Caller-bounded owner ancestry can also
walk target-first structural owners with explicit complete/incomplete status; it
does not invent a continuation token or writable scope. A local-precondition
check can require an exact semantic kind and direct owner, including explicit
notebook-root ownership, before a future command family may mutate that target.

A second read-only precondition compares exact accepted base values for twelve
generic editable value shapes: an existing figure's optional admitted asset
reference, revision-owned constraint kind, one list's ordering-significance
flag, block or inline-span style reference, inline text, formula mode and
source, provenance-record kind and source reference, block or inline-span
provenance reference, table-row role, table-cell span, physical page-profile
geometry, and one page's admitted page-profile identity.
Text and formula checks preserve exact authored source because no Unicode
normalization form is yet frozen. Default command-target material preserves the
canonical value for one exact revision and target, while a family-specific
read-only projection can expose another admitted family on the same stable
identity without changing that default review surface.

The frozen command-family taxonomy is represented without choosing final wire
operation names. Current executable targets admit Asset reference for a figure's
existing optional asset identity, Text content for inline text, Structured
content for formulas, table-row roles, and table-cell spans, Provenance for
revision-owned source records and block/inline-span provenance references,
Ordering and grouping for one existing list's ordering-significance flag, Style
role for block- and inline-span admitted style references, and Document
constraint for page-profile geometry, per-page page-profile assignment, and
broad revision-owned constraint kinds. A combined local checker validates
family, kind, owner, and an optional exact base value in one read-only snapshot
with stale-base precedence.

Deterministic capability discovery reports only those seven family behaviors and
supports read-only behavior-version drift checks. It deliberately advertises no
command protocol, normalizer, command context, Validate, Apply, rebatching, or
numeric command/context limits yet.

Aggregate command behavior and typed-result behavior are version 94 after
admitting LaTeX-base math-mode glyph aliases as exact source. Asset
reference,
Ordering and grouping, and Text content retain family behavior version 1;
Provenance and Style role are version 2, Document constraint is version 3, and
Structured
content is version 82. The immediately previous aggregate version 93 rejects
instead of being reinterpreted.

A version-bound single-target proposal combines capability, exact local
preconditions, and direct-edit simulation read-only. All five dedicated direct
mutation paths consume that simulator before commit.

A separate generic command-graph domain validates duplicate command identities,
self-dependencies, missing dependencies, cycles, and dependency direction
without choosing command-ID syntax or changing command order. Interactive
command-ID selections can report their complete transitive omitted dependency
requirements without silently changing the caller's selection. A 20,000-case
valid-DAG oracle cross-checks exact graph size, dependency closure, omitted-edge
requirements, summaries, bounded reporting, and closed-selection results.
It also asserts coverage of every generated node-count bucket, duplicate edges,
empty/full/partial selections, and both closed and missing-requirement outcomes.

Session-level selection analysis binds those requirements to capability behavior
and the exact accepted base revision, but does not construct a replacement
batch. A non-materializing summary reports selected-command, required-command,
and omitted-edge counts before identity-pair review data is requested. Session
analysis uses lightweight read-only graph-node views and can bound detailed
requirement materialization without cloning omitted pairs first or allocating a
dependency-reference vector per command. Exact graph counts and report size can
be checked against caller-supplied bounds without choosing product limits.

Session graph-resource preflight also binds exact command/dependency counts to
capability behavior and the accepted base revision before candidate simulation;
passing those bounds does not validate graph structure. Ordered simulation can
apply the same caller-supplied bounds before graph and semantic work. Coarse
resource sizing reads command/dependency counts directly without graph-node
views or command-ID ordering. All batch read-only APIs share one capability,
accepted-state, and exact-base authority gate.

The active-process application owner now delegates these exact graph size and
caller-bounded preflight checks, exact and bounded simulation, selection
requirements and summaries, local target review, and bounded or unbounded Apply
to the same semantic authority. This adds no backend product limit, command
context identity, writable scope, or transport admission.

Direct-edit simulation now also projects exact before/after semantic changes. An
ordered in-memory direct-edit batch validates the generic dependency graph,
then consumes valid command payloads through a private targeted value overlay.
Targeted table cells additionally clone their owning table once so ordered span
commands validate the complete candidate grid. Caller command IDs and requested
values move into result/candidate evidence;
local preconditions are borrowed, and only additional typed failure evidence is
cloned.

The simulation stops atomically on the first failure and coalesces accepted-base
to final-candidate net changes. Net coalescing stores first/last mutating
prediction indexes instead of cloning every intermediate semantic change. The
same per-target-and-family index state enforces prior-writer dependencies
without coupling independent fields on one identity.

Figure asset-reference commands can only carry `None` or an accepted semantic
identity already classified as `Asset` in the same current revision. Missing or
wrong-kind identities reject before mutation; raw bytes, base64 payloads, local
paths, and remote URLs are not representable by this value. Asset ingestion
remains a separate open capability; the application owner can retain opaque raw
bytes only after an accepted asset identity already exists.
Executable fixtures now cover attach from no reference, replacement between two
admitted assets, removal to no reference, a dependent replace-then-remove chain,
and a figure nested through callout, list, table-cell, and freeform containers.

The pre-asset-reference aggregate behavior version 2 remains incompatible. A
wrong-kind reference in a later mixed-batch command leaves earlier valid
predictions and history uncommitted.

Provenance commands now distinguish record metadata from semantic linkage. One
value replaces an existing revision-owned provenance record's kind and optional
caller-visible source reference. A separate `ProvenanceReference` value can
attach, replace, or remove the admitted provenance identity on an existing block
or inline span while preserving that target's identity and authored text.
Wrong-kind or no-longer-current references reject before mutation, and Undo
restores either prior value.

The Provenance family remains behavior version 2; later admissions advance the
aggregate command behavior independently.

Revision-owned semantic constraints now expose their broad `ConstraintKind` as a
Document-constraint editable value. The constraint identity and semantic target
remain stable, a changed kind seeds notebook-wide `AllDerived`, and one Undo
restores the prior kind. Reassigning a constraint target and editing detailed
paper, style, placement, or output values remain outside this broad constraint
model. The pre-constraint aggregate behavior version 4 remains incompatible.

Block and inline-span identities now expose their optional admitted `Style`
reference through the Style-role family. Replacement may attach, replace, or
remove that reference; non-Style or no-longer-current identities reject before
mutation. A span keeps Text content as its default command-target material, but
family-specific material exposes Style role without replacing the text review
surface.

Same-span Text, Style, and Provenance batch changes are tracked independently by
target and family. Style-role behavior version remains 2; later admissions
advance the aggregate command behavior independently.

Existing semantic `List` identities now expose only their boolean ordering
significance through Ordering and grouping. The edit can switch ordered versus
unordered semantics while preserving list identity, item order, item identities,
and child content. Item moves, new grouping relationships, insertion/deletion,
and anchor semantics remain open.

Changed lists seed their containing block/flow/page region with `AllDerived`.
The pre-list-ordering aggregate behavior version 6 remains incompatible.

Existing semantic `Page` identities now expose only the accepted page-profile
identity they reference through Document constraint. Replacement must name a
`PageProfile` identity admitted in the same current revision; missing,
wrong-kind, or prior-revision identities reject before mutation.

Retargeting preserves page identity, flows, and the page-profile catalog while
changing only `Page.paper_profile`. It seeds that exact page with `AllDerived`;
downstream reflow remains derived work outside this semantic mutation service.
Profile creation/deletion and serialized command syntax remain open.

An empty batch returns the frozen NoOp prediction before semantic target
indexing. Explicit dependencies are required before a later command can observe
an earlier same-target candidate change; prior no-ops do not manufacture
dependencies.

Successful direct-edit predictions classify their net effect as Mutation or
NoOp and derive conservative impact seeds from backend-owned semantic ownership.
Text seeds identify the owning flow/page dependency region. Structured edits,
figure asset references, block style references, and list ordering-significance
edits seed the nearest block/flow/page. Page-profile geometry changes seed
referencing pages, page-profile assignment seeds the exact changed page, and
semantic constraint-kind changes seed the notebook.

Asset-reference seeds conservatively require all derived authorities.
Provenance-record edits seed the notebook for only diagnostic and output
authorities because reverse claim/source dependency expansion is not yet
implemented. Provenance-reference edits use those same authorities at the
owning block/flow region. Single-target review and ordered batches share the
same seed projection; net semantic no-ops emit no seed.

Ordered simulation consumes indexed target material and impact scopes instead of
cloning them for each evaluation. The index request is keyed by exact target and
requested family rather than unique target alone, so same-target Text, Style,
and Provenance material is collected in one document-order traversal. Block,
list, and table traversal uses borrowed slice continuation frames and stops once
every requested target/family material pair is indexed. Constraint-, page-,
profile-, and revision-owned provenance-record-only batches resolve before block
traversal.

Single-target provenance-record material also resolves before the generic
page/block descriptor walk. Family-qualified block/span material retains its
local impact scope; figure-caption references use the containing figure block as
impact owner rather than the nested Figure identity.

A release probe over 20,000 unrelated rule blocks measured 500 single-target
provenance-material reads at about 94-95 microseconds each before that fast path
and about 0.05 microseconds each after it. These measurements are implementation
evidence rather than a latency promise.

Valid root-owned semantic references now check their admitted Asset,
PageProfile,
Provenance, or Style collections before requesting generic wrong-kind evidence.
With a first-block target and 100,000 unrelated sibling blocks, 500 valid
Provenance-reference simulations measured about 302 milliseconds before this
shortcut and 0.066 milliseconds after it. This is implementation evidence, not
a product latency guarantee.

A pinned release probe with the target first measured about 20 microseconds with
100,000 unrelated top-level blocks versus 1,099 microseconds before traversal
hardening. A nested first-child probe measured about 19 microseconds with
100,000 siblings versus 186 microseconds before slice frames. These are
implementation measurements, not product latency or resource guarantees.

Single-target change preview now reuses the ordered-batch material index for its
impact scope instead of walking the semantic owner chain through repeated root
lookups. A session-local release probe at the maximum accepted 255 block
wrappers measured 2,000 Text-content previews at about 396-400 milliseconds
before indexed impact reuse and 12.4-12.9 milliseconds after it. This is
implementation evidence, not a product latency guarantee.

Bounded identity ancestry now resolves one iterative structural path instead of
rescanning the notebook for every owner hop. A 5,000-tree deterministic oracle
checks branching callout, freeform, list, and table paths against an independent
descriptor-owner walk; root-owned semantic families also pin exact two-entry
paths to the notebook. At 255 block wrappers, the same
20,000 complete release reads measured 4,206 milliseconds before this change and
about 79-88 milliseconds after it; a 2,000,000-read stress completed in about
7.81 seconds. These are implementation measurements, not latency guarantees.

The same transport-neutral direct-edit batch can now be applied atomically after
re-running its validation and simulation. Net mutations replay only the
coalesced final semantic changes into a cloned accepted notebook. Coalesced
cell-span changes are overlaid before each affected table is validated once,
then the batch commits exactly one revision and enters Undo history as one
transaction.

Net no-ops keep the current revision and history position, while middle-command
failure and stale base remain no-effects. The discoverable `Apply` capability
remains disabled.

This is still not an admitted normalized command batch. Protocol normalization,
command context and writable scope, published limits, complete impact expansion,
diagnostics, retry handling, and full Validate/Apply admission remain open.
Candidate acceptance also enforces a public 256-level block nesting resource
bound and iteratively dismantles rejected deep candidates before mutation.

The task remains open for the complete first-release semantic vocabulary,
format parsing and canonical serialization, migrations and round-trip fixtures,
adapter-level semantic command/history transport, measured layout/render
consumers, and format-compatible import/export.

### TODO - Implement calibrated grid and rule geometry

Preserve nominal spacing and square cells while allowing bounded ruler error,
rounded joins, layer order, and profile-specific appearance.

Current executable evidence compiles blank, ruled, dotted, and squared paper
into compact exact physical anchor series without allocating one object per
mark. Squared grids keep identical nominal spacing on both axes. One 20,000-
case deterministic oracle verifies repeated-mark counts, first and last anchors,
and full rule spans across dotted, ruled, and squared regions. A second 20,000-
case oracle verifies signed ruler-sample span and error-bound classification.

Those generated geometry oracles assert every paper pattern, every ruler result
class, both sheet orientations, and all four binding edges are actually reached.

Accepted page profiles own the maximum ruler-error envelope, rounded or sharp
join treatment, and mark layer.

Validated ruler samples cannot move nominal anchors. Overflow, zero spacing,
invalid rounded joins, and unspecified custom geometry fail with typed results.
Standalone ruler-sample validation now reuses the physical-profile appearance
validator, so a zero-radius rounded join rejects before sample span or error
checks instead of bypassing profile-owned appearance authority.

The task remains open for seeded calibrated ruler-path synthesis, additional
profile-specific visual appearance, and renderer or live-output consumption.
Seeded path synthesis is blocked on an executable calibrated variation model:
the accepted contracts define a maximum ruler-error envelope and deterministic
seed, but not the profile-derived distribution or correlation behavior inside
that envelope. Independent point jitter must not be invented as a substitute.

### TODO - Implement measured text flow and pagination

Wrap paragraphs, lists, quotations, and citations from actual handwriting
metrics and move complete semantic fragments to following pages predictably.

Current executable evidence paginates already-measured top-level flow blocks in
semantic order over exact writable regions derived from accepted page profiles.

The physical-profile root independently matches writable-region derivation
across 20,000 valid profiles covering both orientations and all four binding
edges. Another 120,000 mixed profiles compare validation failure precedence and
writable-region results against an independent integer-arithmetic model,
including coordinate overflow and simultaneous invalid conditions.
The mixed-profile oracle also requires every typed page-profile validation error
plus a valid outcome to occur, preventing precedence coverage from collapsing to
a subset of its nine failure classes. Its generated binding edge and orientation
selectors use upper LCG bits and require all eight combinations.

Measurements bind to one accepted revision and one accepted flow; stale,
incomplete, reordered, unknown-flow, and out-of-flow measurements reject before
layout. Repeated measured fragments may retain one block owner, and no fragment
is split by pagination. Already-measured top-level citation, date, definition,
footnote, heading, list, margin-note, paragraph, quotation, and source-note
blocks retain their accepted block identities and semantic order through
pagination.

List pagination keeps the containing top-level block as the measured owner while
preserving its accepted list, item, and child-block identities. This admits
upstream measurements without inventing metrics for those families.

Already-measured callout, figure, freeform, mathematics, rule, table, and
unresolved blocks now complete direct top-level vocabulary coverage. Pagination
preserves their accepted top-level owner order without reinterpreting nested
semantic structure or unresolved source content.

Keep-together groups move intact when a current or later page can contain them.
Groups too tall for any one remaining page fall back only to measured-fragment
boundaries, while exact-bottom fits do not manufacture page breaks. A page
profile edit invalidates old measurements, and fresh measurements reflow
deterministically against the new accepted writable geometry without backfilling
pages before the selected flow's owning page.

Measurement admission now streams contiguous block-owner runs instead of
materializing a second owner vector. Independent pagination advances through the
remaining page sequence once per fragment, and keep-together search computes one
checked total height and maximum width per group. Four reference oracles cover
17,504 independent, keep-together, and deterministic mixed-policy placement
cases, including nonzero current-page remainder and heterogeneous page skips.
Another 20,341 owner-sequence cases cover complete, incomplete, repeated,
reordered, and out-of-flow semantic measurements, including owner runs split
across empty, independent, and keep-together measurement units.

The exhaustive 341-case owner alphabet walk independently requires success,
incomplete measurement, sequence mismatch, and foreign-owner outcomes before the
larger generated corpus is considered sufficient.

The three exhaustive Cartesian placement oracles also require valid plans plus
both no-fit and no-page placement failures, while their page rectangles remain
valid by construction.

Generated pagination oracles now assert their page/unit counts, both policies,
fragment and owner grouping, owner identities, and expected success/failure
classes; the mixed-policy generator uses high LCG bits so low-bit cycling cannot
collapse those advertised branches.

Empty measured flows now avoid unused page-profile authority entirely. Nonempty
semantic pagination indexes page profiles once per call while preserving
first-match defensive behavior, and derives each writable profile region once
for all pages sharing it. Fixtures cover 10,000 ordered measured blocks, 10,000
additional distinct page/profile pairs, and 10,000 pages sharing one profile
without changing semantic order.

The live `SessionApplication` owner now admits the semantic-flow pagination
application dependency and exposes one read-only measured-flow operation. It
binds the supplied measurement to the owner's current accepted revision and
delegates page/profile derivation and measurement admission without publishing
arbitrary page rectangles or a transport route. Live page-profile geometry and
page-to-profile reference edits now have integration evidence that old
measurements become stale and fresh measurements use the current referenced
writable geometry; fresh Undo/Redo measurements follow restored profile content
and assignments under new revision identities. Undo and Redo therefore never
reactivate an older measurement merely because semantic content is restored.

The task remains open for real handwriting and formula measurement, grapheme-
aware line breaking and paragraph wrapping, quotation and citation metric
generation, column-flow policy, measurement diagnostics, and render or
live-output consumption.

### TODO - Implement fixed-region constraint solving

Place titles, figures, callouts, and freeform regions with anchors, alignment,
minimum size, collision policy, and explicit infeasibility diagnostics.

The current semantic `Constraint` owns only the broad
`ConstraintKind::Placement` classification; it has no typed anchor, alignment,
minimum-size, or collision fields. The fixed-region bounds and accepted-layout
components explicitly must not choose those semantics. Solver implementation is
therefore blocked until an executable placement authority owns the typed
constraint values and infeasibility policy.

### TODO - Prevent invisible page overflow

Block export while fixed content crosses the writable region and identify the
object, violated edge, amount, and valid move, crop, resize, or reflow choices.

Current executable evidence checks solver-derived fixed rectangles only after
binding them to an exact accepted revision, page, and semantic block. Writable
bounds come only from the accepted page's referenced physical profile. Every
crossed edge produces one complete blocking layout diagnostic with the semantic
object and page locations plus exact physical overflow evidence; the frozen
first-journey 6 mm bottom overflow is exercised directly. A 20,000-case
deterministic rectangle oracle independently verifies crossed-edge combinations,
stable edge order, and exact overflow amounts.

That pure bounds corpus and the accepted-layout diagnostic projection corpus now
both assert all 16 possible four-edge violation masks are represented, including
the no-overflow case.

A second 200,000-case oracle uses `u128` reference arithmetic over full-range
`u64` rectangles to verify writable and object coordinate-overflow precedence
plus exact surviving edge amounts.
Its representable surviving cases also assert all 16 edge masks, preventing the
full-range corpus from degenerating into overflow-only coverage.

A second 20,000-case accepted placement oracle verifies those reports project
one
complete blocking diagnostic per violation with matching typed edge and physical
amount.

Stale placements, wrong page ownership, missing or invalid profiles, and
unrepresentable coordinates fail before a diagnostic can be presented as
current. The live `SessionApplication` owner routes this same read-only
placement validation against its current accepted revision without admitting
solver or transport authority. Live page-profile geometry and page-to-profile
reference edits now have integration evidence that prior placements become stale
while fresh placements use the current referenced writable geometry; fresh
Undo/Redo placements follow restored profile content and assignments under new
revision identities. Edited overflow diagnostics block layout-only Export for
that revision, then remain stale after Undo/Redo; fresh Redo overflow evidence
must bind to the new revision before it can block Export again.

Nested semantic blocks retain their accepted page ownership, and overflow
amounts retain the full physical `u64` range without diagnostic truncation.

Fixed block/page ownership now reuses the semantic model's iterative structural
lookup instead of maintaining a recursive fixed-layout traversal. Fixtures keep
same-page non-block identities rejected and cover the maximum accepted 256-level
block nesting boundary.

Undo and Redo restore semantic content under fresh accepted revision identities.
Old fixed placements and their revision-bound layout-only Export evidence remain
stale after either traversal instead of being reactivated by restored content.

A read-only layout-only Export preflight now consumes revision-bound layout
diagnostics and refuses layout readiness when evidence is blocking or explicitly
incomplete. The live `SessionApplication` routes that gate against its current
accepted revision, so post-placement overflow evidence can be checked without
creating file authority.

Complete empty evidence remains Ready, while an incomplete set remains
Incomplete even when it already contains a blocking diagnostic. The sole current
layout code is fixed-region overflow, so callers cannot downgrade its backend-
owned blocking disposition to advisory. Stale revisions, evidence from another
revision, and non-layout diagnostic codes or operations cannot be smuggled
through that gate. It accepts no path, overwrite, retry, format, or file-commit
input and therefore cannot report `Exported`.

The task remains open until full Export preflight combines layout with semantic,
source, asset, capability, and format validation before file commit. Move, crop,
resize, and reflow remediation classes also remain open until the owning fixed
placement and asset semantics can prove which choices are valid.

### TODO - Implement mathematics as editable structure

Parse and preserve TeX-compatible inline, displayed, aligned, matrix, unit, and
derivation content while exposing unsupported constructs without rewriting.

Current executable evidence preserves exact UTF-8 mathematical source while
structurally recognizing ordinary Unicode notation, groups, scripts, plain and
styled fractions, plain and styled binomial coefficients, square and indexed
roots, grouped
mathematical
alphabets, common one-group, triple/quadruple-dot, and wide accents, upright
unit or label groups,
explicit `\text{...}` fragments, grouped custom operator names with the exact
starred limits variant, stacked annotations and relations, vector, directional
over- and under-arrows, overline, and underline decorations, escaped TeX
special characters,
standard and variant Greek control-sequence notation including italic capital
Greek forms, complete AMS ordinary, letter-like, geometric, and standalone
special symbols including the large box/diamond aliases, checkmark, registered
mark, maltese cross, and yen sign, common named and delimiter symbols including
bracket, backslash, AMS corner delimiters and fixed symbol aliases, paired
vertical-bar spellings,
parallel control-symbol,
foundational set/logic forms, and the core plain-TeX surd glyph,
real/imaginary-part and derivative-prime symbols, geometry/proof and
ordering/set relations including AMS order, equality, approximation, triangle,
and strong-containment forms,
binary-operator symbols including the AMS boxed, circled, wedge, and product
forms plus square/lattice forms, large
n-ary operators including coproduct and logical joins/meets, card-suit and
musical symbols, mathematical punctuation atoms, LaTeX-base paragraph, section,
dollar, math sterling, pound, paragraph, section, and dagger aliases, and
variant large-triangle symbols,
arrow and ellipsis
symbols including
AMS paired, loop, curve, tail, squiggle, and multimap arrows, harpoons, and
mapped arrows, positive AMS relation and logic symbols, negated AMS arrows and
order/equality, containment, divisibility, turnstile, and triangle relations,
modular arithmetic
including grouped `\pod{...}` operands,
single, multiple including fourfold, contour, and explicit integral-operator
atoms, direct/inverse and decorated limit operators, set notation, aligned row
and
column separators,
and ordered aligned, brace-delimited-matrix, bracketed-matrix, cases,
double-vertical-bar-matrix, gathered, matrix, parenthesized-matrix,
small-matrix, split, and vertical-bar-matrix environments. Paired
delimiter-sizing controls remain explicit unsupported input rather than being
inferred from admitted delimiter glyph names. Context-sensitive ellipsis forms,
the dotted multi-integral form, explicit limit-placement controls, and equation
reference, tag, intertext, display-break, primitive fraction/root, and advanced
spacing or layout controls likewise remain unsupported until their presentation,
expression-grammar, or document-reference semantics have an owning model.

Math-only alignment, script, and row-break markers remain literal inside grouped
text. Row breaks are structural inside grouped substacks without admitting
column alignment there, including when a column-capable environment surrounds
the substack; an environment opened inside the substack owns its own columns.
Inline, display, and aligned presentation modes are semantic values rather than
renderer guesses. Unknown control words remain explicit unsupported
constructs; malformed groups, required arguments,
alignment use, environment ordering, either direction of group/environment
crossing, indexed-root optional-argument closure, and aligned,
brace-delimited-matrix, bracketed-matrix, cases, double-vertical-bar-matrix,
gathered, matrix, parenthesized-matrix, small-matrix, split, or
vertical-bar-matrix boundaries return typed syntax failures.

Gathered rows do not admit column alignment unless a column-capable environment
is explicitly nested inside.

Semantic candidate acceptance analyzes every `Mathematics` block before accepted
identity allocation. Unsupported or malformed mathematical candidates reject
atomically without replacing the current revision, and the same exact bytes can
instead remain admitted through the typed unresolved-block path. Supported
formula source and mode survive candidate-to-accepted identity promotion without
rewriting.

Direct accepted formula replacement uses the same analyzer against an exact base
revision. A real supported edit preserves the formula identity and creates one
new revision; no-op, malformed, unsupported, stale, absent, non-formula, and
empty-session requests are typed no-effects.

The task remains open for a broader TeX-compatible expression vocabulary,
component-level edits such as numerator or matrix-cell operations, canonical
serialization and migration round trips, formula measurement and glyph geometry,
math-specific diagnostics, and render/live-output consumption.

### TODO - Implement tables and ruled educational blocks

Support merged cells, headers, alignment, wrapping, ruler-like borders, boxes,
dividers, arrows, labels, and page references.

Current executable evidence makes table header rows semantically distinct from
ordinary body rows without coupling that meaning to visual styling. Candidate
acceptance preserves the row role and stable row identity together with nested
cell content. An exact-base accepted edit can change only one row role and keeps
row identity and cells unchanged, creates one revision only for a real change,
and treats no-op, stale, absent, non-row, and empty-session requests as typed
no-effects. Nested tables remain addressable through callouts, list items, and
table cells across successive revisions.

Merged cells now carry nonzero logical row and column spans. The first row fixes
logical table width, later rows fill unoccupied columns around inherited row
spans, and candidate acceptance rejects horizontal conflicts, vertical overflow,
or rows that leave logical columns uncovered before accepted identity
allocation. A 20,000-case occupancy oracle now matches the validator's exact
first typed failure and semantic owner, not only valid/invalid classification.

An exhaustive 9,724-case compact corpus additionally covers zero through three
rows, zero through two cells per row, and every one- or two-row/column span
combination. Accepted promotion preserves valid spans exactly, and validation
keeps logical coverage compact instead of allocating per spanned column.

The occupancy generator also asserts every row/cell-count bucket, every 1–3
row/column span, valid grids, and all three typed grid failure classes are
actually reached.

An exact-base direct cell-span edit reuses that complete table-grid invariant. A
valid edit preserves cell identity and child blocks, creates one accepted
revision, and enters Undo history; a structurally invalid replacement is a typed
no-effect. Table-cell spans are now Structured-content editable values in the
generic simulator. Ordered span commands validate cloned owning-table candidate
grids, Apply revalidates affected final tables atomically, and one Undo restores
the complete batch.

A zero-content `Rule` block already supplies the semantic identity for a
divider.
Candidate acceptance preserves that block kind and its generic block style
reference, so divider meaning is not flattened into paragraph text or inferred
from drawn geometry. Ruler-like divider measurement and rendering remain open.

The task remains open for cell alignment and wrapping semantics, ruler-like
border geometry, table measurement/layout, command-batch cell content
operations, and the remaining structured educational block families.

### TODO - Implement English and Spanish text behavior

Normalize Unicode, edit by grapheme, apply language-aware punctuation and
wrapping, and preserve curly quotes, guillemets, accents, and en or em dashes.

Current executable evidence preserves exact authored Unicode through
candidate acceptance, exact-base text replacement, ordered batch Apply, and
Undo, including English and Spanish punctuation, curly quotes, guillemets,
accents, en/em dashes, a decomposed accent sequence, and multi-code-point emoji.
Existing precondition fixtures also keep NFC and NFD spellings distinct rather
than inventing a normalization form. A hexagonal grapheme-edit boundary now
uses pinned Unicode extended-grapheme segmentation behind an outbound port;
`SessionApplication` can replace an exact grapheme range through the existing
atomic whole-text edit authority, and Undo restores the exact authored bytes.
Invalid ranges reject before revision or history mutation.

The frozen 114-entry English/Spanish visible-text inventory now runs through the
same pinned segmentation adapter. Every entry reports exactly one extended
grapheme cluster, boundary zero at byte zero, boundary one at the exact UTF-8
length, and no second cluster boundary, including every decomposed diacritic.

The same inventory now exercises the generic grapheme-edit boundary directly.
Every required entry can be replaced as one exact grapheme unit and inserted at
an internal boundary with its authored UTF-8 bytes unchanged; this adds no
normalization or punctuation-generation policy.

A 114-entry application sweep also inserts each exact requirement through
`SessionApplication`, verifies the accepted semantic text byte-for-byte, and
Undo restores the original text under a fresh revision identity.

Missing, invalid, non-advancing, reversed, or inconsistent source-anchor
boundaries from an injected segmentation provider remain typed failures and do
not mutate accepted state. Source-anchor validation runs before caller range
bounds, so an underreported provider count cannot masquerade as a user error.
Zero-length insertion resolves an internal provider
boundary once, so a changing repeated answer cannot turn insertion into byte
replacement.

A provider that reports zero graphemes also resolves boundary zero once, so
changing answers cannot make nonempty authored text masquerade as an empty
source.

A separate transport-neutral cursor-position boundary now resolves one caller
extended-grapheme boundary index to its exact UTF-8 byte offset. It validates
provider start/end anchors before caller bounds, resolves an internal boundary
once, admits both endpoints including empty text, and returns typed failures for
missing, invalid, or out-of-range positions. All 114 frozen bilingual visible
text graphemes admit exactly start and end cursor positions under the pinned
segmentation adapter. A same-source selection resolver validates provider
anchors once, preserves caller anchor/focus order for forward and reverse
selections, and reuses one internal boundary for a collapsed selection.

A caller-supplied signed-step resolver advances only by extended-grapheme
boundaries, rejects out-of-range steps without clamping, and covers both
directions for all 114 frozen visible-text requirements.

`SessionApplication` exposes position, signed-step, and selection queries
against one exact current Text-content target without mutation; no-session,
stale-base, missing-target, and non-text outcomes are resolved before provider
access.

The application owner also sweeps all 114 frozen visible-text requirements:
exact end position, forward/backward signed steps, and forward/reverse selection
all preserve the same accepted revision and leave history unchanged.

Normalization policy, punctuation generation, language-aware wrapping, cursor
direction/step-size/clamping, selection-extension/visual-affinity policy, and
browser, CLI, or MCP grapheme-range transport remain open.

### TODO - Make missing glyph coverage impossible to miss

Report every unsupported grapheme by profile and offer only a visible declared
fallback, profile repair, content replacement, or export refusal.

Current executable evidence now gives one handwriting profile explicit exact
grapheme declarations and compositional-rule identities. Coverage classification
admits an exact grapheme first, otherwise accepts only caller-supplied rule
evidence that names a rule declared by the same profile; absent or undeclared
rule evidence remains `Missing`. A 12-case compact oracle exhausts exact
presence, declared-rule presence, and absent/declared/undeclared matched-rule
states across Exact, Compositional, and Missing outcomes.

A caller-ordered report returns every missing query with its original index and
exact grapheme value without Unicode normalization or deduplication. All 64
six-query missing masks prove that covered and missing occurrences preserve
exact caller order and duplicate positions.

The frozen 114-entry bilingual text inventory now drives that same report in
cross-component regression evidence. An empty profile returns all 114 exact
requirements in order, while a profile declaring all 114 exact graphemes returns
no missing entries.

A language-owned rule matcher now recognizes exactly the 14 decomposed Spanish
requirements: ten acute forms, two diaereses, and two tildes. Non-decomposed
requirements and deterministic near misses receive no language rule. A profile
with the other 100 requirements declared exactly plus those three rules covers
all 114 with no missing result; arbitrary profile-specific rule semantics remain
outside this baseline authority.

A separate language-owned inventory now freezes 114 visible textual grapheme
requirements before profile coverage: 52 ASCII Latin letters, 14 precomposed
Spanish diacritic letters, their 14 exact decomposed Unicode equivalents, 10
numerals, and 24 core prose punctuation forms. The punctuation set includes
ordinary sentence punctuation, Spanish opening punctuation, straight and curly
quotation/apostrophe forms, guillemets, ellipsis, en/em dash, hyphen,
parentheses, and square brackets. Precomposed and decomposed spellings remain
distinct requirements rather than being normalized into one another.

The inventory deliberately excludes whitespace/layout separators and
mathematical symbols. Whitespace remains text measurement/layout behavior; the
language ADR delegates mathematical glyph requirements to the separately
admitted mathematical symbol authority. Arbitrary profile-specific
compositional rules, that concrete mathematical grapheme inventory, semantic
locations, diagnostics, measurement, rendering, and portable section schema
remain open.
Glyph validation must not infer support from an installed font, renderer
behavior, or successful text acceptance.

A separate corpus-evidence boundary now freezes the ADR verification checklist:
English prose, Spanish prose, names, quotations, questions, exclamations, en and
em dashes, combining marks, normalized equivalents, and mixed mathematics. It
requires caller-owned evidence for all eleven scenarios, permits extra evidence,
and exhausts all 2,048 scenario-presence masks.

A checked-in corpus fixture now authors exact UTF-8 text for all eleven
scenarios and a whitespace-delimited sweep whose 114 tokens must match the
frozen textual grapheme inventory in exact order. The fixture keeps
precomposed/decomposed pairs
visible together and retains mixed mathematical source as uninterpreted text.
It does not claim normalization, render/measure/wrap/edit/serialize behavior, or
CLI/MCP round-trip verification.

A separate fallback-admission boundary now keeps missing handwriting coverage
blocked unless caller evidence identifies one declared fallback style, proves
its use remains visible, and records explicit user acceptance. Exact and
Compositional admission retain the original profile coverage, including the
profile-declared rule reference, without requiring fallback. All eight
declaration/visibility/acceptance combinations are covered independently.
A separate 27-state oracle crosses Exact/Compositional/Missing with absent and
all eight fallback evidence states.

Style selection, consent UI, semantic locations, diagnostics, and rendering
remain open.

## P3 — The dual human and LLM editor

### TODO - Build the responsive 16:9 split shell

Show both editors concurrently with adjustable division, stable selection,
keyboard navigation, zoom, page list, diagnostics, and no hidden main surface.

Current frontend evidence: both editors are visible in one 16:9 workspace. The
splitter supports pointer and keyboard adjustment from 35% through 65% without
forcing horizontal grid overflow, including at a 1024 px viewport, and exposes
a wider invisible pointer target than its visual rule. Preview zoom remains
local from 60% through 160%, with layout-aware page geometry keeping every edge
scroll-reachable at narrow, short, and wide browser sizes.

At 480 px and below, both panels stay visible at a fixed 50/50 split so the
document reflows without horizontal page scrolling; wider viewports restore the
adjustable 35-65% range. The workspace is viewport-bounded, with source and
preview overflow contained by their own scroll surfaces. At 320 by 480 px, the
Task field and page stage are both visible in the initial frame.

Very-short reflow keeps both Task and the page stage visible down to 225 px of
viewport height without document overflow. A 32-case Firefox matrix spanning
320 through 1024 px widths, 225 through 576 px heights, split extremes, and 60%
through 160% preview zoom completed without layout or reachability failures.
With JavaScript disabled, the short-height warning stays visible without taking
workspace flow, and Firefox requests only the document and stylesheet. BiDi
viewport emulation at 320 and 481 pixels confirms the static shell remains 50/50
with an inert divider, scrollable source and preview panels, and no document
overflow before `main.js` is available.

The task stays open for backend-fed page navigation and stable semantic
selection.

### TODO - Build the structured LLM editor

Edit task instructions and notebook source with structured completion,
formatting,
object navigation, diagnostics, and a safe raw-response inspection boundary.

Current frontend evidence includes task, source, permanent prompt, backend-owned
options, and a visually isolated raw model-response boundary that explicitly
remains untrusted until backend validation. After a compatible authenticated
handshake, task, source, and raw-response draft fields are read from
backend-owned session state, staged without partial DOM updates, then committed
to the UI together before editing is enabled. Complete edits replace those same
backend draft fields, while page exit aborts both reads and writes and clears
browser text.

Structured completion, object navigation, candidate diagnostics, semantic
acceptance, and semantic-command transport remain unfinished.

### TODO - Build the human page editor

Support selection, text correction, drag, resize, crop, align, group, layer,
duplicate, delete, and style changes through typed semantic commands.

### TODO - Synchronize selection between both editors

Selecting source, a diagnostic, a page object, or a table cell must focus the
same stable semantic identity in the other surface.

### TODO - Implement semantic undo and redo

Record accepted command batches rather than DOM or canvas snapshots and preserve
stable semantic identities plus deterministic recomputation across history.
Expose undo and redo as application-history commands, not semantic operations
embedded in a new batch.

Current executable evidence now stores semantic history only in the active
backend session. Accepted candidate replacement and direct semantic edits append
in-memory prior snapshots, Undo and Redo require the exact current revision and
allocate fresh revision identities, restored semantic identities remain stable,
read-only availability exposes traversal boundaries, and a new edit after Undo
discards the old redo branch. Dropping the session destroys populated history.

Transport-neutral direct-edit batch application now enters this same semantic
history as one transaction, including multi-command batches. All five
established exact-base direct mutation families also route through the active
`SessionApplication` owner; a live text edit proves commit, stale review, Undo
availability, and exact restoration share that authority.

A synchronized in-process fixture races Undo against Apply from one shared
current revision; exactly one commits and the loser reports the winner's fresh
revision as stale.

No-op, semantic-rejected, and resource-rejected batch attempts preserve an
existing Redo branch after Undo. Candidate replacement on a new branch clears
Redo while never reusing semantic identities still stored in the abandoned
branch.

Repeating a completed traversal against its old exact base now returns stale and
cannot advance history twice. That is a duplicate-effect safety property, not
lost-receipt recovery: the history protocol still owns no retry-identity type,
and base plus direction cannot distinguish a retry from a genuinely new caller.


The semantic-history-result application now freezes the six history result
classes and their commit dispositions independently from traversal execution.
Current Traversed, Boundary, StaleBase, and revision-allocation exhaustion
outcomes project only to their unambiguous frozen meanings; missing accepted
state remains unclassified. Retry replay and cancellation are not implemented.
Read-only direction availability is now centralized over the existing
`HistoryAvailabilityOutcome`, and `SessionApplication` uses it for Redo-aware
asset-byte pruning instead of reinterpreting `can_redo` independently.

Bounded history storage is also blocked on policy rather than mechanics. The
frozen history contract explicitly leaves depth limits and storage structure
unfrozen, so the application must not silently choose eviction semantics that
change which Undo/Redo steps remain admitted. Cancellation likewise awaits an
owning operation-cancellation boundary instead of being inferred from transport
disconnect.

Transaction provenance has frozen origin classes for direct human,
clipboard-assisted model, CLI, and MCP mutations, but no trusted application
entry-path authority currently reaches the semantic history service. The batch
payload must not self-attest that origin, so provenance recording remains
blocked until an admitted adapter/application boundary supplies it.

The task remains open for transaction provenance, dependency-expanded derived
impact, bounded history resource policy, retry/lost-receipt recovery,
cancellation, and browser/CLI/MCP parity.

### TODO - Implement rich clipboard intake

Accept text, structured table fragments, formulas, PNG, JPEG, and WebP while
reporting exactly which source structure or metadata cannot be retained.

Current executable evidence now freezes those six first-release clipboard
content families before adapter parsing. One intake value keeps caller-owned
content separate from source provenance, preserves unresolved fragments, and
retains ordered typed metadata-versus-structure loss details without inventing
MIME parsing or retention policy. Clipboard access, decoding, semantic candidate
construction, diagnostics, and notebook mutation remain open.

### TODO - Implement image placement and layering

Expose source identity, crop, scale, opacity, z-order, below-text, inline,
above-text, and clipped-region placement without overwriting the original.

Current executable command evidence covers source identity on an existing
semantic figure. `AssetReference` can attach, replace, or remove a reference to
an asset already admitted in the same accepted revision, including nested
figures and atomic Undo. The value cannot carry raw bytes, base64, filesystem
paths, or remote URLs, so it does not implement media ingestion.

A transport-neutral placement intent now keeps that original source identity
separate from caller-owned crop, position, size, transform, opacity, z-order,
resolution policy, color handling, and placement constraints. Below-text,
inline, above-text, and clipped-region modes are explicit. Coordinate units,
transform/crop evaluation, opacity ranges, semantic figure integration, derived
line-art, decoding, rendering, and command transport remain open.

### TODO - Keep the Copy prompt control permanently available

Generate one complete, versioned request from current task and constraints,
copy it with one action, and confirm the exact prompt identity copied.

Current frontend evidence: the single Copy prompt control lives in a compact
sticky toolbar and remains visible while source and response content scroll.
It starts disabled until a backend-presented prompt exists and reports clipboard
failure when enabled.

The same presentation surface is mode-neutral: the backend may provide a full
candidate prompt or a targeted semantic-command prompt without adding frontend
command parsing. Command-mode identity must include the accepted revision and
bounded command context so stale copied requests cannot masquerade as current.

A transport probe passed a command-like Unicode payload measuring 1,406,010
UTF-8 bytes through the presented prompt, mocked clipboard `writeText`, and raw
response surfaces with exact content equality and one write. A separate newline
probe showed that Firefox textareas normalize `CRLF` and lone `CR` to `LF`, so
the frozen browser text contract now requires backend prompts to present
canonical `LF` and treats line-ending representation as non-semantic. A control
probe with canonical `LF`, separate NFC/NFD accents, and emoji ZWJ sequences
preserved every observed code point through prompt, mocked clipboard write, and
raw response. These probes do not define a backend protocol limit or prove
operating-system clipboard capacity.

A hostile-text transport probe placed script markup, a fetch expression,
out-of-scope command prose, file-export prose, hardware-start prose, JSON-like
text, and Unicode in the prompt/response surfaces. The frontend preserved the
payload exactly, performed one mocked clipboard write, created no child
elements,
executed none of the text, and emitted no hostile `/evil` request. This proves
browser transport inertness only; backend prompt construction and semantic
validation remain authoritative for model-injection resistance.

The one-shot prompt domain now owns the shared `atrament.prompt/1` contract
identity consumed by browser/backend handshake compatibility, eliminating a
second hard-coded backend prompt-version source. Prompt serialization, identity
computation, mode selection, and browser transport still keep this task open.

### TODO - Validate pasted model responses transactionally

Parse the complete response into a candidate notebook, show all differences and
errors, and change the accepted session only after explicit acceptance.

### TODO - Make overflow and collision correction visual

Draw page-edge, collision, and unsupported-mode diagnostics on preview and link
each overlay to the owning source object and correction actions.

## P4 — Personal handwriting profiles and synthesis

### TODO - Freeze the first `.atrament` container

Choose canonical encoding, manifest, checksums, optional assets, version rules,
unknown-field behavior, migration, and a human-readable inspection command.

Current executable evidence centralizes the first portable-profile format
identity in a dependency-free container domain shared by session compatibility
discovery. Parsed manifest values can now reject future container versions,
unknown required features, duplicate non-manifest paths, missing media types,
traversal segments, noncanonical backslash separators, and entries outside
`sections/` or `assets/` before section decoding. Unknown optional features
remain opaque manifest data. Validated entry paths now
also classify typed `sections/` separately from opaque `assets/`, and a default
path-sorted section projection excludes asset entries without deleting them from
the manifest.

The same domain returns deterministic path-sorted writer order for the complete
archive, including root `manifest.json`, and verifies one entry's independently
observed byte length before its SHA-256 digest. Archive inventory validation now
also requires exactly one root manifest and exact agreement between declared and
observed non-manifest paths, rejecting missing, undeclared, duplicate, or unsafe
names before entry decoding. Canonical archive evidence separately rejects
compression and platform-specific extras and requires ZIP64 exactly when an
adapter reports ordinary ZIP limits are exceeded; the domain does not invent
those limits. Application-owned complete-profile change detection now maps
changed content to mandatory canonical rewrite while unchanged content retains
permission to preserve its original archive bytes.

It does not parse or write ZIP or JSON, choose wire field names or numeric
resource limits, compute hashes, decode typed sections, interpret opaque assets,
migrate versions, or expose the inspection command. Those adapter,
canonical-encoding, migration, and inspection surfaces remain open together with
golden byte-stable ZIP, fixed archive-metadata field values, truncation at the
ZIP adapter boundary, and real digest fixtures.

### TODO - Build the guided calibration session

Collect isolated characters, joins, words, numerals, punctuation, mathematics,
titles, labels, sizes, speeds, pressure proxies, and free writing on known
paper.

Current executable evidence now freezes transport-neutral calibration
provenance. Parameter evidence retains caller-owned source region, units,
confidence, and accepted correction history; observed evidence remains distinct
from inferred extremes. Training and held-out sample identities cannot cross
roles, and an underdetermined required behavior explicitly requests another
sample.

A 5,461-case exhaustive compact oracle covers every declaration sequence through
six samples over two identities and both roles. Repeated same-role declarations
remain valid while the first cross-role reuse rejects in declaration order. This
does not choose a minimum sample set, confidence scale, unit vocabulary,
source-region syntax, capture geometry, extraction algorithm, or calibration
workflow.

Guided-session evidence now also preserves a caller-supplied ordered prompt plan
across resumptions, rejects duplicate prompt identities, derives completion from
accepted prompt progress, and covers the ADR's isolated-character, join, word,
sentence, numeral, punctuation, mathematical-symbol, heading, and free-writing
categories. Completed prompts can be projected read-only in original plan order
without losing sample links, speed, size, or category metadata.

An exhaustive 511-state compact corpus covers every pending/completed mask from
zero through eight prompts and cross-checks completion, next-pending position,
and completed-prompt inspection against an independent bit-mask oracle. One
exact-precondition edit can replace a completed prompt's sample link while
preserving all unrelated prompt state; stale, pending, missing, ambiguous, and
no-op cases cannot alter the session. A second exhaustive 3,076-case compact
oracle crosses prompt counts through six with every progress mask, target or
missing target, current/stale expectation, and equal/different replacement.

Minimum counts, wording, sufficiency, weak-sample classification and workflow
policy, capture UI, persistence/history, and geometry semantics remain open.

### TODO - Correct photographed calibration geometry

Detect reference marks, perspective, lens distortion, scale, grid, baseline,
and capture quality before treating any observed stroke as evidence.

Current executable evidence now requires one caller-produced observation for
each of those seven photographed-capture geometry families and binds the report
to one stable capture identity. Duplicate families reject in report order and
missing families reject in canonical requirement order; a 128-mask compact
oracle exhausts every presence subset. A separate 2,801-sequence oracle covers
every axis sequence through length four, pinning duplicate-before-missing
precedence independently. Detection, image decoding, transform
fitting, distortion correction, units/tolerances, capture-quality scoring,
geometry acceptance, and handwriting extraction remain open.

### TODO - Extract the personal stroke vocabulary

Derive centerlines, contours, entry and exit conditions, pen lifts, ligatures,
diacritics, contextual forms, and confidence with sample provenance.

Current executable evidence now freezes the transport-neutral vocabulary entry
produced by later extraction. Each entry retains caller-owned centerline and
contour evidence, entry/exit conditions, pen-lift evidence, ligature, diacritic,
and contextual-form evidence, plus confidence and sample provenance. Extraction,
geometry/units, form inference, confidence calculation, profile-section schema,
and candidate generation remain open.

### TODO - Implement compositional diacritics safely

Reuse accents only when profile evidence admits the composition and preserve
placement, scale, collision, and language-specific forms.

Current executable evidence now admits one compositional diacritic intent only
when the exact handwriting profile classifies its externally matched rule as
`Compositional`. The admitted value retains the exact admitting profile
identity and profile-declared rule together with caller-owned placement, scale,
collision evidence, and language-specific form. Exact-covered graphemes and
missing or undeclared composition rules reject
instead of silently switching to accent reuse. Unicode decomposition, rule
applicability, placement/scale geometry, collision evaluation, language-form
selection, stroke planning, rendering, and fallback remain open.

Exact coverage remains the first classification even when the supplied
compositional rule is also undeclared.

### TODO - Implement continuous contextual stroke planning

Select, connect, deform, space, and lift strokes from neighboring graphemes,
word position, line geometry, semantic role, and calibrated writing style.

Current executable evidence now freezes both sides of the transport-neutral
planner boundary. Profile-provided contextual candidates retain caller-owned
candidate identity, character intent, stroke-vocabulary payload, semantic
origin, profile choice, and entry/exit conditions. One complete planning input
also retains neighboring graphemes, word position, line geometry, semantic role,
and calibrated writing style while preserving caller candidate order.


A pure identity-collision check can now reject the first repeated candidate
identity and report its earliest prior owner without selecting, ranking, or
rewriting candidates. A 31-case compact oracle exhausts every two-identity
sequence through length four; applying that check remains an explicit planner
boundary decision rather than an implicit constructor side effect.

The inspectable output plan keeps ordered samples with caller-owned position,
tangent, curvature, width or pressure proxy, velocity, and explicit contact
state; each declared stroke retains semantic origin, profile choice, and
contextual entry/exit conditions. Empty declared strokes reject before any
projection, while an empty plan remains valid for content with no handwriting.
A validated plan can project the exact planner-order stroke indices owned by one
semantic origin, providing an edit-local dependency region without mutating or
replanning the plan.

Candidate ranking/selection, invalidation orchestration, geometry, units, joins,
deformation, spacing, interpolation, rendering, PDF, and machine-motion behavior
remain open.

### TODO - Implement bounded correlated variation

Support observed or authorized minimum, maximum, distribution, and correlation
for size, slant, roundness, flattening, spacing, drift, speed, and pressure.

Current executable evidence now preserves independently observed or authorized
minimum and maximum bounds, validates central tendency inside the envelope, and
retains caller-owned typed units, distribution metadata, correlation groups,
context rules, and the profile/document/page/line/word/character/stroke scale. A
caller-owned parameter identity plus document seed and stable semantic
identity forms the typed replay key. A caller-produced sample can now be checked
against the parameter's inclusive
minimum/maximum envelope while retaining those exact replay inputs. Repeated
caller-produced observations carrying the same exact replay key must also carry
the same sampled value; the first disagreement reports both observation indices
without choosing how the sample was generated.

A compact 341-case replay oracle exhausts every sequence through length four
over two semantic keys and two values. Separate fixtures prove that changing
only the parameter identity, document seed, or semantic identity yields a
distinct replay key.

Parameter identities are also required to be unique within a supplied parameter
collection. The first duplicate reports both its caller-order index and the
earliest prior owner; a 31-case compact oracle exhausts every two-identity
sequence through length four without assigning meaning to either identity.

A separate 1,099-case integer oracle exhausts all 343
minimum/central/maximum triples in `[-3, 3]` and all 756 sample checks over
`[-4, 4]` for valid envelopes, reaching every parameter and sample validation
outcome.

A complete sample-set gate now composes those invariants in one deterministic
order: parameter validity, caller-order parameter identity, caller-order sample
bounds, then exact replay consistency. A 2,700-case compact oracle crosses all
parameter triples in `[-1, 1]`, both match/mismatch states for each sample's
parameter identity, and every two-sample value pair in `[-2, 2]`. It reaches
parameter, identity, bound, replay-conflict, and accepted outcomes while pinning
the first identity mismatch in caller order.

This does not choose parameter vocabularies, distribution families,
correlations, context semantics, random generators, fitting policy, or a
sampling algorithm.

### TODO - Implement handwriting roles and sizes

Allow one profile to expose body, title, subtitle, label, caption, formula,
margin, and annotation roles without pretending they are unrelated writers.

Current executable evidence now centralizes those eight handwriting roles and
stores the profile identity once around caller-supplied role presentations. Role
entries therefore carry size/style metadata but cannot independently claim a
different writer identity. A read-only role lookup returns `None` for a missing
role and refuses an ambiguous duplicate claim instead of silently choosing one;
a 31-case two-role oracle covers zero through four supplied presentations.

Size units, role defaults, style vocabulary, global duplicate/missing-role
policy, glyph behavior, and semantic document-style mappings remain open.

### TODO - Prevent repeated-glyph and repeated-line artifacts

Detect frozen contours, identical word rhythms, mechanical baselines, local
white noise, and configurations that leave the calibrated writer's envelope.

Current executable evidence now freezes seven independent variation-quality
axes spanning those five artifact risks plus the accepted ADR's correlation
retention and extreme-legibility checks. One caller-computed finding is required
per axis. Duplicate or omitted axes reject structurally, while detected
artifacts remain evidence and are reported in canonical axis order. All 128
combinations
of not-detected/detected findings are covered without depending on input order.

Statistical detectors, thresholds, perceptual metrics, correlation models,
legibility criteria, sampling, rendering, and release policy remain open.

### TODO - Validate against held-out writing

Measure geometry, rhythm, joins, spacing, punctuation, and perceptual fidelity
against samples excluded from extraction and publish known failure modes.

Current executable evidence now freezes a transport-neutral held-out quality
report with those six required dimensions, caller-owned measured values and
supporting evidence, and caller-owned known failure modes. Structural admission
first rejects conflicting calibration sample roles, then rejects measurements
that reference training or unknown samples in report order, and finally requires
all six quality dimensions. Repeated held-out measurements and an empty known-
failure list remain valid because aggregation and failure discovery are not
inferred here. A 793-case compact oracle exhausts all 64 required-dimension
subsets plus all 729 six-measurement held-out/training/unknown sample-reference
sequences and pins first invalid-reference precedence.

Metric units, thresholds, scoring, statistical/perceptual models, sample
sufficiency, quality-report generation, and pass/fail policy remain open.

## P5 — Deterministic CPU rendering and digital output

### TODO - Implement authoritative vector geometry

Generate layout boxes, stroke centerlines, expanded ink contours, equations,
rules, tables, and diagram paths with physical units and semantic provenance.

Current executable evidence now centralizes those seven authoritative vector
primitive families. Every primitive structurally carries caller-owned geometry
and semantic provenance, while page geometry retains caller-owned physical
bounds
and composition order. Geometry generation, physical unit vocabulary, contour
expansion, tessellation, rasterization, material behavior, PDF, and live-motion
projection remain open.

### TODO - Implement layered ink materials

Compose base deposition, edges, starvation, pooling, paper interaction, color,
and highlight layers without altering vector authority.

Current executable evidence now also freezes the six accepted material-layer
families: base ink, edge variation, deposition texture, paper interaction,
highlight, and color. Layer plans preserve caller-owned clip geometry, material
inputs, physical bounds, and blend order without evaluation. Every layer marked
stochastic carries the complete document-seed, semantic-identity, profile, and
material-preset replay key. Required-layer policy, transfer equations, blending,
texture sampling, color spaces, rasterization, and output formats remain open.

Current executable evidence now freezes transport-neutral empirical
contact-model inputs before material evaluation. Force-calibrated pressure and
named dimensionless proxies are distinct, fitted parameter evidence retains
caller-owned units, provenance, confidence, error measures, and calibrated input
range, and out-of-range inputs require explicit extrapolation.

The same boundary now retains the six accepted observable output families:
trace width, coverage, starvation, pooling, edge displacement, and absorption or
drying response. A caller-owned inclusive output envelope can classify observed
values as bounded or outside the declared range and rejects inverted
envelopes. Preset evidence also retains caller-owned pen, ink, paper,
conditions, calibrated input ranges, bounded output-range evidence, error
measures, and known failure modes. Transfer functions, concrete output
units/ranges, preset vocabularies, force units, proxy scales, and rendering
remain open.

### TODO - Implement bounded page texture and soft noise

Apply seeded low-frequency texture at declared physical scale and prove it does
not move geometry, obscure small writing, or create repeated tiles.

Current executable evidence now freezes page-level texture intent without
generating pixels. A plan retains read-only vector-geometry authority, physical
page bounds, caller-owned physical noise scale, a bounded strength envelope, and
the complete stochastic replay key reused by material layers. Strength validates
inside its inclusive envelope before generation. Frequency models, texture
generation, tile detection, legibility thresholds, units, rasterization, and
visual evaluation remain open.

### TODO - Implement fast and final CPU quality profiles

Use the same vectors, seeds, blend order, and page geometry while varying only
declared texture resolution and sampling cost.

Current executable evidence now makes Preview and Final explicit quality roles.
A pair validates only when vector geometry, calibrated material authority,
physical noise scale, render seed, material blend order, and physical bounds
remain exactly shared; caller-owned texture resolution and sampling cost may
differ. Rendering, concrete quality values, performance budgets, scheduling,
and sampling algorithms remain open.
Both wrong-role paths reject before shared-authority comparison, including when
other authority fields also drift.

Current design evidence freezes Render as a read-only application capability
bound to one accepted revision and deterministic vector/material inputs. Preview
and final profiles share geometry, seeds, physical dimensions, and blend order;
quality-only sampling may differ. Render identity, retry safety, invalidation,
read-only cancellation/progress semantics, typed projection results, no-file
behavior, and browser/CLI/MCP parity are frozen while implementation remains
open.

### TODO - Implement configurable line-art extraction

Convert images to transparent single-color paths with levels, threshold,
cleanup, detail, minimum feature, and preview controls suitable for hand color.

Current executable evidence now freezes the accepted live-compatible boundary:
one source image identity plus caller-owned levels, threshold, cleanup, detail,
minimum-feature, and preview controls can produce an ordered transparent-black
vector-path result linked back to that source. Result validation rejects drift
in any control or source identity from the exact request, with control drift
checked first. A 128-case oracle crosses every six-control drift subset with
matching or changed source identity and reaches both mismatch classes plus the
exact-match path. The control values remain generic and have no inferred
defaults.

Image decoding, control units/ranges/semantics, path extraction, vector
geometry,
acceptance diagnostics, preview rendering, and final rendering remain open.

### TODO - Implement digital paper notes and shadows

Render loose note fills, folds, stacking, and soft shadows as digital-only
objects with readable contrast and explicit live incompatibility.

Current executable evidence now freezes one digital loose-paper-note plan with
caller-owned fill, fold, stack order, and soft-shadow intent plus
caller-computed
readable-contrast evidence. Structural validation requires that readability
evidence without choosing a contrast metric or threshold. The existing output
capability matrix separately keeps loose paper notes, folds, paper shadows,
simulated shadows, and textured fills accepted only for Digital and rejected for
Live.

Contrast measurement, concrete style/geometry values, note layout, occlusion,
shadow rendering, raster composition, and digital preview/export remain open.

### TODO - Implement theme-safe decorative titles

Support layered lettering, outlines, highlights, and motifs in digital mode and
derive a sober, single-pen alternative without losing title hierarchy.

Current executable evidence now freezes a caller-produced digital/live title
pair around one stable semantic title identity and one exact hierarchy value.
The digital treatment structurally carries layered lettering, outline,
highlight,
and motif inputs while the live treatment remains caller-owned one-pen intent.
Validation rejects title-identity drift first and hierarchy drift second while
allowing the mode-specific treatments themselves to differ. A 256-case compact
oracle crosses 64 hierarchy values with both identity- and hierarchy-drift bits
while varying digital/live treatment values independently.

Title-style derivation, hierarchy semantics, typography, colors, motif
generation, one-pen geometry, layout, rendering, and conversion orchestration
remain open.

### TODO - Produce vector-preserving PDF

Embed bounded texture and image resources, preserve physical page boxes and
color, retain searchable semantics when compatible, and emit a render manifest.

Current executable evidence now freezes the transport-neutral render-manifest
input structure required by the accepted reproducibility contract. It retains
accepted revision/document/profile/paper identities, asset identities, engine
version, model choices, variation seed, material authority, renderer version,
quality profile, and output-affecting render options. Render identity
computation,
serialization, PDF bytes, resource embedding, searchable-text policy, and file
commit remain open.

A transport-neutral PDF composition plan now also preserves final page order,
caller-owned physical page boxes, vector authority, color intent, explicit
searchable-semantic-text disposition, explicit embedded-versus-safe-reference
asset disposition, and render-manifest linkage. PDF object encoding, fonts,
compression, concrete safe-reference policy, compatibility decisions, resource
bytes, serialization, and file commit remain open.

### TODO - Calibrate print and scan round trips

Measure physical scale, clipping, margins, grid registration, color shifts,
photo placement, line weight, and scanner distortion on representative devices.

Current executable evidence now names all eight physical round-trip measurement
families explicitly and keeps each caller-owned observation tied to its source,
unit, value, and rendered/exported output identity. The four ADR verification
families remain physical scale, clipping, grid registration, and photo
placement;
P5 additionally retains margins, color shift, line weight, and scanner
distortion. No observation is interpreted as calibration success.

Representative-device selection, concrete units, tolerances, statistical
analysis, correction transforms, calibration fitting, and pass/fail policy
remain open.

### TODO - Meet CPU preview latency and memory budgets

Benchmark long pages, many images, dense equations, zoom, rapid edits, and final
export on ordinary integrated-graphics computers with no compute GPU.

Current executable evidence now records transport-neutral CPU benchmark
observations for those six workload scenarios. Each observation is explicitly
Preview or Final, retains caller-owned latency and peak-memory measurements, and
names a caller-owned machine/configuration classified as having no discrete GPU.
A completeness validator requires evidence for all six scenarios and requires
both Preview and Final quality to be represented without demanding a Cartesian
scenario-by-quality matrix. Numeric budgets, units, representative machine
tiers,
benchmark execution, render scheduling, and pass/fail policy remain open.

A leave-one-scenario-out matrix proves each workload is independently required
before quality-role completeness can pass. A second 64-case oracle exhausts
every Preview/Final assignment over a complete six-scenario set and proves only
the all-Preview and all-Final assignments fail role completeness.

## P6 — LLM, CLI, MCP, and media intake

### TODO - Freeze the one-shot formatting prompt

Include the complete backend-owned return format, allowed styles, output mode,
source rules, ambiguity behavior, and exact return envelope in one copy action.

Current executable evidence now freezes the transport-neutral initial formatting
request as one self-contained value. It retains task, complete source material,
paper/style constraints, output targets, backend-owned semantic format, source
and provenance rules, diagnostic/ambiguity expectations, required return
envelope, prompt identity, and prompt version. Prompt text
wording/serialization,
identity computation, clipboard presentation, response parsing, and semantic
command refinement remain separate open authorities.

### TODO - Implement assignment-to-notebook structuring

Support titles, explanations, derivations, tables, equations, diagrams,
citations, examples, and conclusions without inventing missing task facts.

### TODO - Implement source and claim provenance

Distinguish provided, derived, cited, and unverified content and link every
citation to the exact claim and source metadata the user must review. Keep that
semantic provenance separate from application transaction provenance recording
whether an accepted mutation entered through direct human editing,
clipboard-assisted model response, CLI, or MCP.

The provenance-only acceptance case is now executable for one existing
revision-owned provenance record: kind and optional source reference can change
while claim text, claim identity, claim-to-record linkage, and unrelated source
records remain stable. Claim-level assignment and removal are also executable
for existing block and inline-span targets through admitted provenance
identities; they preserve authored content and remain atomic with other generic
families on the same target. Undo restores the prior linkage.

A transport-neutral citation-review linkage now additionally requires every
cited claim in a review set to resolve through its exact assigned revision-owned
`Provenance` record and from there to at least one exact source-metadata
identity.
The validator derives `Cited` status from that record rather than a copied claim
field. Links to unknown
claims,
unknown sources, unknown or duplicated provenance records, mismatched
provenance links, non-cited claims, duplicate claims/sources, or duplicate exact
claim/source relationships reject with typed
structural errors. Multiple distinct sources for one cited claim remain valid.

A 32-case compact oracle crosses all four semantic provenance kinds with link
presence, source presence, and provenance-identity match state to pin the same
failure precedence. Source retrieval, source-quality policy, citation
parsing/formatting, and review UI remain separate.

The task remains open for source retrieval/admission, a concrete source-metadata
schema, source-quality and claim-sufficiency policy, provenance diagnostics,
citation parsing/formatting and review UI, candidate-response integration, and
cross-adapter projection. Those owners must consume the existing revision-owned
provenance and citation linkage rather than inventing parallel source status.

### TODO - Prove complex educational coverage

Exercise bilingual mathematics, physics, chemistry, biology, history, and
language assignments with dense but readable page organization.

Current executable evidence now names all six required subject families and
requires every admitted exercise observation to carry caller-established
bilingual, dense-organization, and readable-organization evidence. Complete
coverage requires at least one fully evidenced observation for mathematics,
physics, chemistry, biology, history, and language while permitting additional
subject observations and preserving their caller-owned artifact identities.
A 64-case subject-presence oracle proves every subset rejects at the first
missing subject, while 48 property-state cases cross every observation position
with all bilingual/dense/readable evidence masks and pin property precedence.

Actual cross-subject assignment fixtures, curriculum/source correctness,
language selection, density/readability measurement and thresholds, page layout,
rendering, and pedagogical quality policy remain open.

### TODO - Freeze the semantic command-batch envelope

Version the batch, base notebook revision, command-context identity, readable
context, writable targets or insertion anchors, preconditions, retry identity,
and normalized application receipt without exposing storage paths as document
semantics.

Current design evidence freezes those semantics, revision-owned command
families, acceptance fixtures, semantic normalization, command ordering,
dependency validity, retry-safe batch-local insertion handles, and backend-owned
resource limits without choosing final wire field names or JSON Schema.

The inbound semantic application contract now also carries one backend-owned
command context with behavior identity, notebook/base revision, context
identity, readable context, explicit writable targets and insertion anchors,
admitted families, relevant constraints, local precondition material, edit
intent, and resource limits. A separate application boundary checks family and
location admission independently, so readable-only identities never become
writable by echoing them. Returned batch authority can also be checked against
that context's notebook, base revision, behavior identity, and context identity
without accepting partial matches as equivalent context. A parsed
transport-neutral batch envelope now retains its protocol-version token, retry
identity, exact context binding, and ordered direct-edit commands; context
review
preserves command identities while checking every existing target and requested
family against that same bounded context.

Batch-local insertion-handle structure now also has transport-neutral graph
evidence: one unique declared handle maps to one producing command, a consumer
must name a declared handle and carry an explicit dependency on that producer,
and missing producer/consumer commands reject before any accepted mutation. The
validator does not decide which command family may produce a handle or allocate
accepted identities.

Command-context resource admission now enforces every count that current typed
structures can measure without serialization: ordered commands, explicit
dependency edges, and the combined writable target plus insertion-anchor count.
Each axis is checked independently against its optional context-owned limit, and
an absent limit remains unspecified rather than becoming a frontend default.
Envelope/readable-context bytes, structured depth, and family payload sizes
still
need their owning representations before they can be measured here.

Protocol-token membership can now be checked exactly against one capability
snapshot without guessing a downgrade or conflating capability behavior with a
wire protocol version. The production snapshot still advertises no protocol
versions, so every serialized protocol token remains unsupported today.

Application-operation membership and family behavior compatibility can likewise
be checked exactly against one snapshot. Missing application operations reject
without becoming executable, while family checks distinguish an absent family
from behavior-version drift. These read-only checks do not widen target-specific
family admission or populate the production snapshot's empty application list.

A transport-neutral envelope preflight now composes the currently representable
facts without choosing one rejection precedence: current snapshot behavior
against the context, requested application and protocol membership, exact
batch-to-context binding, ordered family/target scope, and count-based resource
admission. Simultaneous failures remain independently inspectable. Dependency
graph validation, normalization, retry equality, and mutation stay outside this
preflight. A 64-case mask oracle crosses snapshot behavior, application and
protocol membership, family and target scope, and command-count admission to
prove those preflight axes remain independent; the existing 16-state context
binding and 27-state resource oracles cover their own internal combinations.

A separate fixture supplies duplicate command identities plus a missing
dependency and proves preflight preserves both ordered commands while counting
the explicit edge; dependency-graph rejection remains solely graph-validator
work.

Preflight failures can now also be projected into frozen result-class facts
without collapsing them to one batch result: unsupported application,
capability, protocol, or family authority; command-context mismatch, writable
scope escape, and measured resource rejection remain independently visible.
Command identities and order are retained, so later precedence policy need not
reconstruct evidence.

A transport-neutral core result taxonomy now names the 15 frozen application
classes required across Validate and Apply without assigning final wire names or
transport statuses. `Unknown transport outcome` is intentionally absent because
the frozen contract defines it as caller state when no valid core response was
received, not as an application result.

The semantic-command-result application now exposes each core result class's
frozen commit disposition. Applied is the sole class that guarantees a new
accepted revision from the current call.
Idempotent replay reports recovery of an earlier completed Apply without
applying again, and every other core class guarantees no new commit from the
current call. Replay deliberately does not guess whether the recovered earlier
result was a mutation or no-op.

The internal direct-edit simulation and Apply foundations now project only their
unambiguous outcomes into that taxonomy. Simulation prediction maps to
Successful validation; Apply mutation/no-op and shared capability, graph,
resource, semantic, and stale-base failures map directly. Candidate replay
failure mutates only an isolated clone, while revision allocation fails before
the commit point; both therefore map to Internal failure with known no-commit.
Missing accepted state remains explicitly unclassified rather than being
assigned invented final result semantics.

Actual protocol-version implementation/admission, context identity computation,
context construction/completeness policy, normalization and normalized batch
identity, retry equality/recovery, receipt normalization, concrete
insertion-command payload behavior and accepted-ID mapping, and serialized
compatibility remain open.
The exhaustive three-node dependency-mask oracle requires valid, self, missing,
cyclic, and forward-dependency outcomes to occur. Duplicate identities remain a
separate fixture because that Cartesian graph uses fixed-unique command IDs.

### TODO - Implement atomic command validation and apply

Validate the whole batch against one accepted snapshot, reject stale or invalid
commands without partial mutation, and make retry behavior idempotent enough for
CLI and MCP automation.

Current executable foundation applies the transport-neutral ordered direct-edit
batch through the same semantic simulation used for review. A successful net
mutation commits exactly one accepted revision and one Undo transaction; a net
no-op creates no revision, and stale or middle-command failure remains atomic.
Validate/Apply semantic change and impact-seed evidence match for unchanged
inputs.

An in-process synchronized race fixture now releases two Apply calls sharing one
base; exactly one commits and the other returns the winning revision as stale.
The task remains open for the normalized protocol/envelope, command-context and
writable-scope admission, retry identity and lost-receipt recovery, published
resource limits, complete dependency-expanded impact, diagnostics, transaction
provenance, cancellation and adapter-level concurrency fixtures, and
browser/CLI/MCP parity.

Retry recovery is specifically blocked on normalized batch identity: the current
`DirectEditBatchProposal` is deliberately pre-normalization, while the frozen
retry contract compares one retry identity against the normalized batch. Raw
proposal equality must not stand in for that missing normalization authority.
Until these boundaries exist, capability discovery intentionally does not
advertise `Apply`.

### TODO - Implement impact-scoped recomputation

Compute changed semantic identities and dependency-expanded invalidation so a
small command does not regenerate unrelated notebook content, while every
derived layout, handwriting, diagnostic, preview, export, or motion result that
can change is recomputed.

Current executable direct-edit simulation separates semantic changes from
conservative derived-impact seeds for text, structured content, and page-profile
edits. Those seeds identify safe starting scopes and authority families, but the
frozen command contract explicitly treats them as inputs to later expansion.

No executable dependency graph currently relates those seeds to downstream flow
regions or layout, handwriting, diagnostic, preview, export, and motion
producers. Full impact expansion and incremental recomputation are blocked on
that derived-dependency authority; seeds must not be relabeled as the final
impact set.

### TODO - Add clipboard command-mode round trips

Let the backend present a self-contained command-mode prompt for an accepted
revision and parse the pasted command envelope through the same untrusted raw
response boundary. Delimit notebook/task/source material as data, exclude
session
credentials and internal paths, and treat prompt-injection-like source prose as
untrusted content rather than application authority. Show the semantic diff and
diagnostics before interactive acceptance rather than parsing commands in
TypeScript.

If review permits selecting only some returned commands, submit that selection
to the backend as a new dependency-checked batch and revalidate it. Never splice
or partially apply the old validated envelope in TypeScript.

Current clipboard transport design evidence freezes explicit Copy, exact text
transport, prompt/context correlation, intentional external-data egress,
operating-system clipboard lifetime, failure behavior, untrusted Paste, inert
browser presentation, no hidden clipboard archive, and MCP parity.

Current frontend evidence has one `writeText` path and no clipboard-read API,
command parser, HTML sink, domain request, or browser storage path. A lifecycle
fixture produced zero writes before Copy, exactly one blocked write after Copy,
kept hostile markup and command-like prose inert, and on `pagehide` scrubbed the
prompt/response without issuing a second clipboard write to simulate revocation.
The static Copy description also states that system clipboard data can outlive
the Atrament session and remains available with JavaScript disabled.

Browser/backend wiring for real command responses remains open.

### TODO - Expose CLI parity for every application command

Create, inspect, validate, transform, render, export, and plan hardware without
requiring a browser or changing domain behavior.

### TODO - Expose MCP from the same command schemas

Project bounded capability discovery, inspect, validate, apply, render, export,
and plan capabilities for agents and prove normalized receipts match CLI and
interactive application. Keep physical `arm` and `start` behind their separate
device-safety boundary.

Current design evidence freezes MCP effect classes, capability discovery,
revision-bound Inspect semantics, explicit completeness and continuation,
backend-owned command-context derivation, bounded read/write scope, receipt
chaining, lost-receipt recovery, command and output result classes, optional
operation lifecycle semantics, autonomous-loop progress/stop conditions, local
per-session adapter admission, effect-class authorization, session-scoped
recovery, no internal-file authority, and browser/CLI/MCP parity targets. Tool
schemas, concrete stdio/loopback mechanism, and backend implementation remain
open.


The dependency-free `mcp-capability-effect` domain now freezes the eight generic
application capability classes and their declared effect classes without naming
MCP tools or implementing adapter admission, transport, schemas, receipts, or
execution. Undo and Redo remain grouped as History traversal at this vocabulary
layer.
The same vocabulary also reports that none of those generic application
capabilities grants physical-device authority; connect, home, arm, start, pause,
resume, cancel, and safe-stop remain outside this MCP application layer.


A separate dependency-light MCP effect-admission domain now applies exact
membership over the seven frozen application effect classes. It authorizes only
that axis; session identity, transport, packaged/live capability presence, and
all capability-specific revision/scope/path/retry/output checks remain open.

For MCP Command context, Validate, and Apply, a composed read-only projection
retains that effect authorization beside exact semantic-command snapshot
membership without collapsing either axis. It still does not authenticate a
session, choose rejection precedence, normalize commands, or execute work.


The shared application-operation-lifecycle domain now freezes the six
first-release operation classes and maps Apply, history traversal, Export, and
the three read-only operations to their authoritative effect boundaries. It does
not implement cancellation, progress, operation identity, scheduling, or retry.

Current design evidence freezes autonomous-agent loop stop conditions, typed
progress versus non-progress, same-retry recovery, bounded automation budgets,
and the separation between semantic completion, explicit output, and physical
authority. Concrete MCP tools and backend execution remain open.

Autonomous budget admission now preserves all five frozen budget axes and stops
further mutation on this axis when any caller/backend-supplied exhaustion
fact is present. Numeric limits, budget accounting, scheduling, and overall
mutation authority remain outside this domain.

Budget-stop projection now maps exhausted-budget admission to the frozen
terminal stop class and returns no terminal outcome when the budget axis does
not require a stop. Merging it with other stop/completion conditions remains
coordinator work.

Autonomous output intent now distinguishes caller-explicit goals, independently
admitted host policy, and untrusted semantic content. Caller goals can establish
Render, Plan, or Export intent; host policy is an alternate source only for
persistent Export, and edited/model-provided content grants none.

Autonomous progress evidence now distinguishes five frozen authoritative
progress conditions from four explicit non-progress conditions without
inspecting raw state or model prose. Stateful revision/diagnostic comparison
and loop orchestration remain open application work.

Application-capability progress projection now treats a transition from
Unsupported to Admitted as new authority only for the exact same capability. It
does not infer goal requirement, blocker resolution, or terminal unavailability.

Repeated-input progress projection now recognizes unchanged qualified revision,
blocking evidence, and bounded intent with no new admission as frozen
non-progress. Fingerprint construction and owning loop policy remain open.

Repeated-No-op control now maps only qualified repeated No-op for the same
bounded intent to a local requirement to stop equivalent mutation attempts. It
does not choose a terminal goal class or grant continuation on other evidence.

Semantic-result progress projection now maps Applied directly to accepted
revision progress and Idempotent replay to replay-recovery non-progress. No-op
repetition, stale-state comparison, and diagnostic progress still require
stateful evidence.

History-result progress projection now maps Traversed directly to accepted
revision progress and Idempotent replay to replay-recovery non-progress.
Boundary and stale handling remain separate from progress evidence.

History-boundary control now preserves the exact Undo or Redo direction from a
typed Boundary outcome and stops blind retry only in that direction. Stateful
scheduling and terminal-goal decisions remain open coordinator work.
Read-only history availability now projects each requested direction as
admitted, boundary, or no accepted revision before mutation. This does not
implement scheduling, traversal, retry, or terminal-goal policy.

Stable-blocking stop projection now maps only qualified repeated blocking
diagnostic evidence to the frozen stable-blocking terminal outcome. Stateful
diagnostic comparison and all other stop/completion composition remain open.

Unresolved-stop projection now requires an explicit owning-caller choice before
Unrepresentable or unresolved can become the frozen unresolved terminal class.
Requesting broader workflow remains nonterminal and grants no authority itself.

Autonomous terminal outcomes now distinguish successful goal completion from
stopping for unresolved evidence, exhausted budget, stable blocking failure, or
unavailable capability. Detecting those terminal conditions and constructing
completion receipts remain open coordinator work.

Autonomous completion admission now requires qualified semantic satisfaction
and complete explicitly requested output work before producing the successful
terminal class. It also aggregates already-qualified requested-output items so
empty means not requested, any incomplete item blocks completion, and only
all-complete items satisfy the aggregate. Revision comparison, identities, and
completion receipt construction remain open coordinator work.

Satisfied-No-op projection now maps No-op to successful completion only when the
separate completion-admission gate already proves semantic and requested-output
satisfaction. Stateful equivalence and satisfaction detection remain open.

Refreshed-state completion projection now allows Stale base or Command-context
mismatch to complete only after independently qualified fresh semantic/output
evidence already satisfies the goal. Refresh and satisfaction detection remain
open coordinator work.

Derived-output completion projection now classifies all 33 operation/result
pairs so only successful projection, Export commit, or recovered Export can
satisfy one requested output. The completion boundary aggregates those qualified
items; receipt and output-identity construction remain open.

Requested-output progress projection now crosses all 99 operation/result/intent
combinations. Only caller-requested successful projection or fresh Export counts
as new progress; recovered Export is replay-recovery non-progress, and host
policy does not become caller-goal progress.

Autonomous session-boundary evidence now invalidates prior command context,
retry/recovery state, and session admission while keeping explicit external
caller workflow state outside Atrament ownership. Fresh-session bootstrap starts
from capability discovery and Inspect; reconnection/reattachment remains open.

Generic autonomous physical-boundary evidence now proves that no sequence of
successful Apply, Render, Export, or Plan steps accumulates authority for the
eight frozen hardware actions. Physical admission and execution remain separate.

A partial autonomous semantic-result classifier now encodes the twelve
next-step branches that the frozen contracts define exactly. Successful
validation, pre-commit cancellation, and internal known-no-commit failure remain
unclassified rather than receiving invented retry or escalation policy.
No-op guidance now requires inspecting accepted state and deciding whether the
bounded intent is already satisfied; it no longer shares Applied continuation.

History automation now separately classifies Traversed, Idempotent replay,
History boundary, and Stale current revision into their frozen continuation,
recovery, stop, or reinspection guidance. Cancellation and known-no-commit
failure remain intentionally unclassified.

Derived/output automation now separately classifies five explicit remediation
branches for stale revision, overwrite conflict, Export retry conflict, external
target drift, and idempotent Export recovery. Output successes and outcomes
whose next action depends on diagnostics, cancellation intent, or caller
authority remain unclassified.

A shared MCP/lifecycle projection now maps the six operational MCP capability
classes to the frozen application operation lifecycle and leaves Inspect and
Command context outside that lifecycle. It does not imply those capabilities
are packaged, admitted, running, cancellable, or exposed as tools.


A separate MCP/output-result projection now admits derived/output result meaning
only for Render, Plan, and Export and preserves the result-class applicability
check for all 11 frozen outcomes. The other five capability classes receive no
output-result disposition, and no output execution is implied.

MCP Command context, Validate, and Apply now have a separate read-only
projection
onto semantic-command application admission. Exact backend snapshot membership
remains authoritative across all four command application bits; MCP vocabulary
alone never turns an omitted operation into an admitted one.

MCP History traversal now has a separate read-only projection for all six frozen
history result dispositions and backend-owned Undo/Redo availability. The other
seven MCP capability classes receive no history projection, and retry,
cancellation, traversal execution, and concrete MCP tooling remain open.

### TODO - Package self-contained agent instructions

Allow a user to provide the repository or release bundle to an agent and have
it discover the exact CLI, schemas, examples, and validation workflow locally.
Command-mode instructions must distinguish readable context from writable scope,
forbid agent-allocated accepted IDs and embedded adapter effects, and use an
admitted unresolved response when the requested edit cannot be represented.

Current design evidence freezes offline release discovery, truthful implemented
versus design-only capability status, live capability-snapshot negotiation,
local schema/contract/example discovery, typed result and diagnostic handling,
clipboard/native automation separation, credential exclusion, version mismatch,
and physical-safety boundaries. The concrete discovery filename, generated
schemas, CLI/MCP executables, and packaging integration remain open.

### TODO - Implement optional audio and video transcription

Normalize supported media to bounded temporary audio, invoke the admitted
WhisperX adapter, preserve word timing and confidence, and always clean up.

Current executable evidence now freezes a transport-neutral derived transcript
separate from original media and any particular transcription engine. It retains
original media kind/identity, unique job identity, and replaceable engine
identity inside one typed origin bundle. It also retains resolved word order,
optional word timing/confidence/speaker evidence, and
explicit unresolved fragments. Media decoding, bounded waveform conversion,
WhisperX invocation, job progress/cancellation, temporary storage, and cleanup
remain open adapter/lifecycle work.

Terminal media-job cleanup state is now executable independently of filesystem
implementation. Success, cancellation, and handled failure all remain unsettled
while an owned waveform intermediate is pending cleanup; cleanup failure becomes
an explicit retry-required state, and an intermediate owned by another job
rejects. Temporary path selection, actual deletion, retry scheduling/limits,
process interruption recovery, decoder/engine invocation, and persistence remain
open application/adapter work.

A process-local media-job application service now owns opaque job and waveform
identities around that terminal rule. One active job can register at most one
waveform intermediate, terminal outcome cannot be rewritten, cleanup failure
remains retry-required, and recorded cleanup success is the only transition to
Settled after an intermediate existed; late cleanup success or failure for that
retired intermediate rejects without recreating cleanup work. Dropping the
service leaves a fresh owner with no prior job authority. The service still
chooses no path, creates or
deletes no file, invokes no decoder or engine, and schedules no retry.

### TODO - Structure transcripts without hiding uncertainty

Turn reviewed transcript spans into sections, definitions, examples, and
formulas while retaining time ranges, confidence, speakers, and unresolved text.

Current executable evidence now freezes an evidence-preserving reviewed-span
boundary without choosing a structuring algorithm. Section, Definition, Example,
Formula, and Unresolved roles remain caller-reviewed classifications linked to
the complete source transcript. Review now rejects resolved-word slices or
unresolved-fragment references borrowed from a different transcript instead of
relabeling them under the supplied transcript provenance. Empty resolved-word
slices also fail as unverifiable because they contain no addressable word
evidence that can prove source membership.

Resolved spans retain original word timing, confidence, speaker, and text
evidence; explicit unresolved fragments retain their text, timing, and
confidence and cannot be silently promoted to a confident role. Span
segmentation, role selection, transcript correction, confidence policy, semantic
notebook construction, and layout remain open.

## P7 — Honest single-pen live output

### TODO - Implement the live capability compiler

Reject or explicitly convert every color, highlight, photograph, shadow, paper
note, raster-only effect, or multi-tool action before motion planning.

Current executable evidence now implements all frozen first-release Digital/Live
capability-matrix rows across semantic objects, handwriting/decoration styles,
color, image treatments, page/paper objects, and hardware actions. Every one of
the 118 rows has an exact `Accept`, `Convert`, `Reject`, or `Future` disposition
for both modes; conversion is never treated as acceptance, and Future is never a
best-effort fallback. A source-linked projection review now also enumerates
every
capability use in caller order, requires explicit conversion choice/provenance
for `Convert`, keeps `Reject` and `Future` blocked, and rejects surprise
conversions on directly accepted capabilities.

A Live-specific typed review now also validates the accepted conversion kind for
all 27 Live `Convert` rows: highlight underline/box/spacing/stroke-weight, sober
one-pen title, calibrated ink, accepted line-art projection, one-pen geometry,
or same-pen paper marks. Conversion details and provenance remain caller-owned.

An exhaustive 27-by-9 regression checks every Live `Convert` row against every
frozen conversion kind so over-admission cannot silently make an unsupported
projection ready.
The mode-neutral review retains supplied conversion evidence but cannot mark
a Live `Convert` row ready; only the typed Live review promotes an admitted kind
to `Converted`. Blocking diagnostics, conversion execution, semantic projection
construction, and Plan compilation remain open.

### TODO - Implement the device-neutral motion plan

Emit ordered pen-up and pen-down paths, speed, acceleration, optional pressure,
safe bounds, pauses, checkpoints, semantic origin, and estimated duration.

Current executable evidence now freezes the transport-neutral motion-plan
content
boundary without hardware authority. Ordered operations retain pen-up/pen-down
physical geometry, semantic origin, speed/acceleration intent, optional
pressure,
pauses/checkpoints, physical/writable-bound evidence, calibrated pen and live
capability assumptions, accepted revision identity, backend plan identity, and
optional estimated duration. Path optimization, coordinate calibration, plan
identity computation, capability compilation, simulation, vendor command
projection, device I/O, and physical authorization remain open.

Current design evidence freezes Plan as a read-only derived application
capability bound to one accepted revision and live capability profile, with
deterministic inputs, blocking diagnostics, plan identity, read-only
cancellation/progress semantics, typed projection results, no file or device
side
effect, safe retry, derived invalidation, and browser/CLI/MCP parity. Backend
plan compilation remains open.

### TODO - Optimize path order without changing handwriting

Reduce pen-up travel and drying conflicts while preserving stroke order where
joins, ink behavior, semantics, or the handwriting profile require it.

Current executable evidence now freezes the order-preservation boundary without
choosing an optimizer. One constraint set is bound to an exact plan identity and
operation count, with explicit Join, InkBehavior, Semantic, or
HandwritingProfile reasons. A proposed optimized order must be a complete
permutation of source operation indexes and retain every declared earlier/later
relation; unconstrained operations may move. Route selection, distance/cost
metrics, drying-conflict detection, and optimization policy remain open.

### TODO - Build the hardware simulator and dry run

Visualize pen-up travel, pen-down paths, limits, time, pauses, and checkpoints
without connecting to a physical machine.

Current executable dry-run evidence now binds one exact device-neutral plan to
one concrete coordinate-calibration record and one ordered limit-evidence entry
per plan operation. Real plan operations classify as pen-up, pen-down, pause, or
checkpoint; motion cannot mark limit checking inapplicable, and unknown or
violated limits reject at the exact operation index. A 16-case matrix covers
all four limit states at pen-up, pause, checkpoint, and pen-down operations.
Coordinate transforms,
limit geometry, visual rendering, timing simulation beyond plan evidence, and
hardware contact remain open.

Offline simulation now also produces a borrowed inspection trace only after the
complete dry run validates. The trace retains the exact plan and calibration,
operation order, pen-up/down geometry, pause/checkpoint payloads, per-operation
limit evidence, plan/revision identities, physical bounds, and admitted total
estimated duration without copying or rewriting plan authority. Pixel/vector
visualization, coordinate transforms, limit geometry, and derived per-operation
timing remain open.

### TODO - Calibrate pen and blank-sheet coordinates

Measure usable area, origin, axis orientation, scale, skew, pen-up height,
contact height, speed, acceleration, page clamping, and boundary clearance.

Current executable evidence now freezes those calibration measurements as one
transport-neutral record bound to an identified device and explicit measurement
evidence. Coordinate transform, pen-motion envelope, and blank-sheet
registration remain separate grouped values while all units, tolerances, axis
conventions, calibration algorithms, coordinate transforms, limit validation,
dry-run admission, and physical arming remain open hardware-boundary work.

### TODO - Implement the NextDraw and AxiDraw CLI adapter

Use documented SVG or CLI control behind a managed process boundary and prove
preview, plot, pause, resume, cancellation, and failure diagnostics.

### TODO - Implement documented HP-GL and GP-GL export

Map admitted motion capabilities to each command language without claiming
direct-device support for untested transports, firmware, or model behavior.

### TODO - Build the physical compatibility ledger

Record exact model, firmware, adapter tier, transport, paper, pen, usable area,
settings, evidence, known limitations, and last acceptance date.

Current executable evidence now freezes that compatibility record without
claiming support for any untested hardware. Exact model, firmware, transport,
adapter tier, paper, pen, usable area, settings, acceptance evidence, known
limitations, and last acceptance date remain caller-owned values tied to one
combination. Hardware probing, compatibility admission, persistence, expiry or
recertification policy, and physical support claims remain open.

### TODO - Prove safe interruption and uncertain-state recovery

Test disconnect, power loss, user pause, emergency stop, process crash, partial
stroke, unknown carriage position, restart, and refusal to resume unsafely.

Current executable evidence now freezes fail-closed recovery admission after
disconnect, power loss, pause, emergency stop, process crash, or process
restart.
Resume remains admissible only when required feedback is available, carriage
position is known, bounds remain valid, and no partial stroke is unresolved;
every other snapshot requires operator recovery. A 96-state Cartesian fixture
covers all six interruption provenances across feedback, position, boundary,
and stroke certainty; only the six fully known states remain resumable.
Restart is provenance only:
adapter commands, homing, position acquisition, safe-stop execution, restart
persistence, and operator procedures remain open.

### TODO - Prove complete single-pen notebook output

Write a multi-page English and Spanish fixture with titles, prose, equations,
tables, and line art on calibrated blank sheets with no manual content repair.

## P8 — First-release completion

### TODO - Complete data-format and migration compatibility

Test current, prior, future, corrupted, truncated, and partially unsupported
notebook and profile data with explicit, non-destructive outcomes.

Current profile-container regressions now exercise a compatibility matrix over
current, prior-version, future-version, partially unsupported, corrupted,
truncated, and missing-entry evidence. Current data admits without mutation,
unknown optional metadata remains intact, unsupported required features and
non-current container versions fail closed, and corrupt/truncated/missing entry
evidence leaves declarations unchanged. No prior migration is claimed because
the accepted profile contract does not yet define one; notebook migrations and
all real supported profile migrations remain open.

### TODO - Fuzz every untrusted input boundary

Cover notebook bundles, `.atrament` profiles, clipboard HTML, images, TeX,
model responses, media metadata, CLI, MCP, and hardware status messages.

The `.atrament` profile path boundary now also has a deterministic generated
162-case Cartesian corpus across both admitted roots and safe, empty, traversal,
backslash, Unicode, space-containing, and percent-like path segments. It pins
validation precedence and exact admission without treating percent-like text as
URI decoding. This is targeted boundary hardening only; full coverage-guided
fuzzing of profiles and the other listed untrusted inputs remains open.
The Cartesian corpus explicitly requires both admitted entry kinds plus
backslash, empty-segment, and traversal failures; unsupported-root and reserved
manifest paths remain pinned by dedicated cases.


Archive inventory admission now adds a deterministic 4,096-case mutation corpus
against an independent reference oracle. Replacement, insertion, deletion, and
duplication of observed member names exercise duplicate detection, missing
manifest and declared members, unsafe separators and traversal, undeclared
members, Unicode, spaces, and percent-like names with exact first-error
precedence.
The observed-member corpus now also requires valid admission and every inventory
outcome it owns: duplicate, invalid observed path, missing manifest, missing
declared member, and undeclared observed member.


Typed profile-manifest admission now adds another deterministic 4,096-case
reference corpus. Generated values combine current/prior/future version strings,
supported and unsupported required-feature sets, safe and unsafe entry paths,
duplicate declarations, empty media types, Unicode, spaces, and percent-like
names while preserving exact validation precedence.
The corpus also requires valid admission and every typed manifest failure class
to occur, so selector coverage cannot hide a precedence branch that is never
observed.


Canonical archive-encoding evidence now exhausts all 16 stored/compressed,
platform-extra, ZIP64-required, and ZIP64-present combinations against an
independent precedence model. Entry verification adds 4,096 full-range `u64`
length and digest combinations, retaining byte-length mismatch precedence before
digest comparison.

The TeX-compatible mathematics boundary now also runs a deterministic generated
4,096-source corpus in inline, display, and aligned modes. Generated mixtures
cover supported and unsupported commands, malformed groups/environments,
control symbols, whitespace, NUL, Unicode, alignment, scripts, and root syntax.
Repeated analysis must agree exactly; successful parses retain complete source
and span invariants, while typed syntax failures must point to a UTF-8 boundary
within the original source. This remains generated regression coverage rather
than coverage-guided fuzzing.

The localhost HTTP router now also runs 4,096 deterministic raw-byte mutation
cases derived from a canonical public request. Replacement, insertion, deletion,
and duplication mutations span arbitrary byte values and request positions.
Repeated routing must return identical HTTP bytes, cannot reflect the session
secret or seeded private draft markers, and cannot change task, source, or
candidate draft state. Socket-level timing/resource fuzzing and coverage-guided
request generation remain open.

The deterministic HTTP and TeX generators now also assert that every declared
mutation operation, mutation/fragment-count bucket, and TeX fragment token is
actually exercised, preventing a passing corpus from silently starving one of
its advertised selector classes.

TeX fragment selection uses upper LCG bits and requires every ordered adjacent
fragment pair, including the current fixed AMS aliases and LaTeX math sterling
symbol, to occur in the generated corpus. The TeX corpus also requires success
and syntax-error outcomes in Inline,
Display, and Aligned modes, with both fully supported and unsupported-preserving
successful analyses.
Its malformed-source coverage also pins group open/close, matrix/cases
open/close,
missing required groups, incomplete root indexes, and out-of-structure alignment
as observed syntax classes.
HTTP corpus guards additionally require both public health success/rejection and
authenticated draft apply/reject outcomes, so the mutation checks cannot pass by
exercising only one routing disposition.

The profile manifest/evidence and browser fragment/draft generators likewise
assert their declared selector and result classes, so deterministic P8 coverage
fails when a generator loses a promised branch even if its sampled assertions
would otherwise remain green.

A second 4,096-case corpus mutates only post-request-line bytes of one
canonical
authenticated Task replacement, so generated inputs cannot turn into private GET
reads. Host, authorization, origin, framing, and body mutations must route
deterministically. Any non-204 outcome leaves all seeded drafts unchanged; a
successful replacement may change only Task and must agree across repeated runs.
Responses never reflect the session secret.


The browser session-fragment parser now also runs a deterministic 4,096-case
mutation corpus against an independent ASCII oracle. Replacement, insertion,
deletion, and duplication mutations cover credential-prefix bytes, lowercase and
uppercase hexadecimal, delimiters, percent-like text, NUL, Unicode, and emoji.
Upper LCG bits drive mutation selection, and the corpus requires every
operation/token and operation/position pair. Repeated parsing must agree
exactly,
and only 64 lowercase hexadecimal characters after the exact `#session=` prefix
are admitted.


Browser handshake response admission now adds a deterministic 4,096-case corpus
against an independent fail-closed oracle. Generated compatible, incompatible,
and invalid payloads vary all six version dimensions, diagnostic namespace and
completeness, item count, code, dimension, and expected version without enabling
compatibility on malformed values.
The generator explicitly fails if any declared dimension, metadata selector,
diagnostic item-count shape, or compatible/incompatible/invalid result class is
starved.


Browser draft resource-limit classification now adds a deterministic 4,096-case
corpus against an independent fail-closed oracle. Generated error kinds,
diagnostic versions and completeness, item counts, and stable codes must
preserve the known resource-limit condition without accepting malformed
metadata.


Shared browser diagnostic-set admission now adds a deterministic 4,096-case
reference corpus across namespace versions, completeness states, zero through
three items, valid and invalid item/code shapes, and ignored opaque fields. The
generator asserts every declared selector class is exercised while malformed
metadata remains unadmitted.

### TODO - Complete visual regression coverage

Compare semantic layout, vector topology, layer composition, final pixels,
digital themes, live themes, and overflow overlays independently.

Current executable evidence now requires one independently computed result for
semantic layout, vector topology, layer composition, render seeds, physical
bounds, final pixels, digital theme, live theme, and overflow overlays before a
render-regression report is structurally complete. Duplicate or missing axes
reject, while mismatches remain explicit per axis instead of being hidden by a
passing final-pixel result. Leave-one-out and
one-duplicate-at-a-time matrices prove every axis independently participates in
both completeness checks. Axis-specific comparators, pixel tolerances, golden
artifact storage, renderer output, and visual acceptance policy remain open.

Reversed all-mismatch evidence also proves every axis is surfaced in canonical
mismatch order.

### TODO - Complete accessibility and keyboard operation

Make both editors, page navigation, diagnostics, drag alternatives, prompt copy,
import, export, and hardware arming usable without a pointer.

Current frontend evidence includes skip links, labeled text areas and counters,
a keyboard-operable splitter, button-based zoom, and focusable source, preview,
and page-stage scroll regions with explicit focus indicators measuring at least
7.08:1 contrast. The two skip links also focus and reveal their panel headings
using native anchor behavior when `main.ts` is absent. Session text follows its
presented LTR or RTL direction without changing clipboard content; status
announcements remain available for session, clipboard, and zoom. Diagnostics
remain ordinary unavailable text until a backend feed can update them.

A visible-text sweep, explicit textarea placeholders, and ruled preview copy all
measure at least 4.5:1 contrast. If a focused splitter becomes inert at the
compact breakpoint, focus moves to a source heading whose full contextual header
is scrolled into view.

Trusted Firefox keyboard actions Tab to each skip link, activate it with Enter,
PageDown the focused heading's scrollport, and continue with Tab to the next
local control. BiDi viewport emulation confirms the compact 320-pixel tab order
omits the inert divider while 481 pixels restores it between source and preview.

At a real 320-pixel viewport, both skip links still focus the expected heading
and return that panel to its origin. Sharing their viewport anchor keeps the
preview skip link 60 pixels tall at 200% text instead of the prior 104-pixel
wrapped block, without horizontal overflow through 480 pixels.

A real-compact text-spacing override at 320 and 480 pixels, plus the 481-pixel
wide boundary, keeps document overflow at zero. Task, Copy prompt, page stage,
and Diagnostics all remain reachable through their owning scrollports at short
225-pixel height.

Modified divider navigation keys remain unconsumed while plain Arrow, Home, and
End keys retain separator behavior. Zoom controls hand focus to an enabled
sibling instead of the document body when a boundary or reset action disables
the control that was activated.

Divider pointer gestures preserve grab offsets and tenth-point ratios that match
measured panel geometry, and are serialized and released on cancellation,
navigation, compacting, viewport resize, browser-window blur, or document
hiding. If captured
dragging is absent,
throws, or silently fails to capture, native touch defaults remain available and
only a completed click changes the split; an uncaptured pointerdown also
preserves the prior keyboard focus. Compact state is derived from the same
480-pixel viewport-width boundary as CSS and updates through one resize
listener.
Repeated same-state resizes do not rewrite separator ARIA, while real breakpoint
changes retain the compact splitter contract.

Firefox BiDi viewport emulation at 320, 479, 480, 481, 482, 640, and 1024 pixels
matched `innerWidth`, CSS media queries, and the rendered compact state exactly.
A trusted pointer drag interrupted by a wide-to-wide viewport resize released
capture before geometry changed; its late move did not alter the split, and the
next drag remained usable.

The inert compact divider restores native touch behavior without an overlapping
hit target. Backend-owned editing, import, export, diagnostic actions, and
hardware actions still need complete no-pointer paths before this task closes.

### TODO - Prove localhost security and privacy

Test loopback binding, session token, hostile local pages, file-path
confinement, temporary cleanup, no telemetry, no autosave, and no undeclared
network access.

Current loopback-runtime evidence requires the exact canonical Host, browser
Origin, and Bearer credential before protected draft mutation. The request line
requires exactly one ASCII space between method, target, and HTTP/1.1; tabs,
repeated spaces, trailing whitespace, and other versions reject before routing.

Browser privacy regression evidence also pins the generated workspace to exactly
three admitted same-origin `fetch` call sites for handshake and draft transport.
All three omit ambient credentials, reject redirects, use no-referrer and
no-store behavior, and contain no absolute HTTP(S), beacon, XHR, WebSocket, or
EventSource transport path.

Malformed bare-LF or bare-CR header line endings reject while the peer remains
connected instead of occupying the listener until the request timeout. EOF also
cannot replace the required blank CRLF header terminator. Exhaustive prefix and
two-write split fixtures cover every byte boundary of valid GET and body-bearing
POST requests.

Configured request and response timeouts are total transport deadlines rather
than per-read or per-write budgets. Trickle bytes cannot extend the production
two-second request budget, and a slow response consumer cannot hold the
single-thread listener open by making intermittent write progress. The reader
and writer both refuse to run without their configured deadline; accepted
connections receive both deadlines before request handling begins. Five
production trials with a one-mebibyte authenticated draft and tiny-window
trickle readers released queued health in 1.932-1.980 seconds instead of
remaining blocked beyond eight seconds.

The transport admits at most 16 KiB of request headers and 2 MiB of request
body. Exact-limit fixtures pass while one byte over either ceiling rejects. The
first-release authenticated handshake additionally admits no request body;
absent or zero `Content-Length` is accepted only when no trailing bytes exist.
Each of its six required version headers must appear exactly once: a 12-case
missing/duplicate matrix rejects malformed multiplicity with `400` before the
application compatibility service is invoked.

Automatic browser launch now fails startup closed before `ready`: the bound
loopback listener and fresh credential are released instead of leaving an
unreachable authenticated session running. The opener itself has one five-second
completion budget, so a hung platform helper is killed rather than holding the
process forever in `listening`. Recovery text is secret-free and directs a
restart after browser launch is fixed; it does not present the bare origin as an
authenticated continuation URL.

A real process fixture now records startup publication rather than inferring it
from source order: successful launch emits exactly `starting`, `listening`, then
`ready`, with no origin until the listener exists and no credential field in any
record. A deterministic failing opener emits only `starting` and `listening`,
exits nonzero, and keeps both the fragment and bound origin out of recovery
stderr. On Linux, absent or empty `DISPLAY` and `WAYLAND_DISPLAY` markers are
unavailable; an empty-marker process fixture still fails before `ready` even
when its fake opener would succeed.

The opener process now receives null stdin, stdout, and stderr. Before that
change, a synthetic opener that echoed its URL copied the credential-bearing
fragment into both Atrament output streams; the same probe is now silent. A live
Linux `/proc` check also observed file descriptors 0, 1, and 2 all bound to
`/dev/null`.

Three hung-opener production trials failed closed in 5.003-5.007 seconds and
left no matching helper process. This closes child-output reflection and
unbounded launch waiting without claiming to solve the separate argv exposure
below.

The current automatic-launch handoff still passes the fragment-bearing URL as a
platform-opener command-line argument. A synthetic Linux probe observed that
argument, including its marker credential, through both `/proc/<pid>/cmdline`
and `ps` while the opener was alive. This violates the intended protection from
unrelated same-user local processes even though the fragment never enters an
HTTP request, log, query string, or persistent product file. P1 remains open
until the platform launcher uses a credential handoff that is not exposed via
child argv, environment, temporary files, or shell history; moving the same
secret between those surfaces is not an acceptable fix.

Request targets must use RFC 3986 origin-form path/query characters beginning
with `/`; absolute proxy form, `*`, fragments, controls, raw Unicode, and URI-
invalid visible punctuation reject before routing. Any `%` escape must carry
exactly two ASCII hexadecimal digits before routing.

An exhaustive raw graphic-ASCII oracle pins the admitted URI character classes
and representative upper/lowercase percent escapes against public routing.

Every header field name must also use the HTTP token grammar; whitespace,
Unicode, control characters, and other non-token names reject even when the
header is unrelated to Atrament. Header values reject control bytes other than
HTTP tab whitespace, including NUL and DEL, before routing.

Request framing rejects transfer encoding,
duplicate content lengths,
body-length mismatch, and non-digit Content-Length syntax such as a leading plus
sign before draft state can change.

Security-sensitive header values trim only HTTP space/tab OWS;
Unicode whitespace remains part of the value and therefore cannot normalize a
malformed Host, Origin, or Bearer credential into an admitted one.
Session-secret comparison executes all 64 admitted byte positions for every
candidate length;
0 through 128 byte candidates pin exact-length admission without claiming
platform-level constant-time execution.

Public, missing-route, unauthenticated, and authenticated empty responses are
also checked against exact session-credential reflection.

Current frontend evidence has no runtime dependencies, network client,
persistent browser storage, domain parser, external assets, referrer, autofill,
or browser spellcheck on session text. Its CSP blocks cross-origin script,
style, image, and fetch attempts before a second loopback origin receives a
request and denies unused frame, font, manifest, media, object, and worker
classes. Text, selection, and dynamic enabled-state restoration are disabled.
With the browser adapter active, every page exit clears all four session text
surfaces and their local enabled/count/copy state before navigation continues.

Fresh loads discard only URL fragments that resolve to local document IDs and
reset all three nested scrollports to their static origin. A 70-reload mixed
stress covered plain and percent-encoded local IDs plus unknown and malformed
fragments without a viewport-restoration failure.

Unknown fragments remain untouched by the presentation adapter. This avoids
preempting the runtime contract's in-memory session-secret handoff before the
authenticated startup adapter exists. It is an interim shell behavior, not the
final secret-handling guarantee: authenticated startup must consume an admitted
secret in memory and remove it from the visible URL.

Malformed or rejecting clipboard promises fail closed, and stale duplicate
completions cannot unlock a newer write. One thousand same-value prompt events
plus one thousand concurrent repeat clicks retained one active clipboard write
with no redundant status or disabled-state mutations before completion.

Prompt changes and page exit discard pending work and scrub the frontend's
prompt copy from an already-started request without claiming that the external
clipboard operation itself can be cancelled.

Before a bfcache snapshot, all four session text live values, default values,
and textarea text nodes are cleared. The disposable workspace then scrubs
retained descendant text and comment payloads plus detached attribute-node
values before removing its element tree.

A fixture retaining source, preview, page-stage, paper, diagnostic, textarea,
Text, Comment, and Attr references observed empty text and attribute payloads
after `pagehide(persisted=true)`. The textarea references also remained empty in
both live and default value state.

A real workspace → second-page → Back flow produced no intermediate workspace
request, then forced one fresh reload where those controls, counters, copy
status, and the three nested scrollports returned to their static pre-session
state. A 30-cycle bfcache stress repeated retained Text/Attr scrubbing and the
forced fresh reload without a scrub or restoration failure.

With bfcache disabled, the equivalent Back navigation performed a fresh
`back_forward` document load and again restored all four text surfaces empty and
all three nested scrollports to their static origin.

A Firefox crash-recovery fixture created live sessionstore recovery files, used
trusted keyboard input for task, source, and candidate text, presented a prompt,
then terminated Firefox with `SIGKILL`. Restarting the same profile restored the
workspace with all four surfaces empty and disabled and no plaintext secret hit
in the profile files.

Disposable viewport state follows the same rule. Reload, bfcache Back,
non-bfcache `back_forward`, and same-profile crash recovery all reset a 65/35
split, 160% preview zoom, and available nested scroll offsets to the static
46/54, 100%, and zero-origin state.

A control copy with `autocomplete="off"` removed only from Task restored its
secret after the same crash cycle, confirming that the fixture detects Firefox
form-state persistence rather than merely reopening a blank page.

The localhost runtime now emits response-level
`Content-Security-Policy: frame-ancestors 'none'` and
`Referrer-Policy: no-referrer` on every response. The former cannot be enforced
from the document-level meta CSP, while the latter reinforces the shell's own
no-referrer declaration. Backend fixtures pin both headers on every served
workspace resource and across representative success, empty, authentication,
conflict, routing, and malformed-request response classes.

A real Firefox BiDi fixture now launches Atrament plus a hostile second loopback
origin. Direct top-level loading commits the workspace document, while framing
the same Atrament URL from that hostile origin commits Firefox's blocked
document instead. A permissive third loopback origin frames successfully from
the same attacker page, isolating Atrament's response policy rather than generic
cross-origin framing behavior. This proves browser enforcement of
`frame-ancestors 'none'`, not only header emission.

The same fixture performs document-initiated navigation from Atrament to a
hostile loopback recorder and observes no `Referer`, while a permissive control
document sends its ordinary origin referrer to that recorder. This proves
browser enforcement of `no-referrer`, not only header emission.

These browser constraints do not replace backend socket, token, host, path,
cleanup, framing, or hostile-origin acceptance tests.

### TODO - Package reproducible desktop releases

Ship the Rust backend and TypeScript frontend as one verifiable installation
with checksums, licenses, offline startup, diagnostics, and clean uninstall.

### TODO - Complete operator and developer documentation

Document calibration, composition, one-shot LLM use, PDF, live safety, supported
hardware, profile internals, schemas, adapters, validation, and troubleshooting.

Current developer documentation now includes a verified portable-profile
container guide mapping the accepted `.atrament` design to executable manifest,
path, inventory, entry-evidence, ZIP-canonicality, rewrite, and compatibility
behavior. The guide also marks the Rust values as non-wire-schema authority and
lists the still-missing ZIP/JSON adapters, migrations, real digest computation,
resource limits, typed section schemas, golden files, and inspection command.

A verified PDF/print composition guide now maps the accepted output ADR to
executable vector authority, pre-serialization PDF page/resource intent, render
manifest inputs, preview/final invariants, print/scan evidence, and independent
visual-regression axes. It explicitly keeps PDF serialization, image/resource
adapters, manifest identity, visual comparators, and physical tolerances open.

A verified calibration-evidence guide now documents observed/inferred parameter
evidence, source/unit/confidence/correction retention, held-out separation,
resumable prompt plans, underdetermination, and downstream bounded-variation
invariants. It keeps capture geometry, extraction, minimum sample sets, quality
metrics, confidence policy, and persistence explicitly open.

A verified live physical-safety guide now documents capability disposition,
device-neutral plan intent, coordinate-calibration evidence, fail-closed dry
runs, offline traces, interruption recovery, and exact compatibility records.
It explicitly states that none of those types arm hardware or establish generic
device support. Vendor adapters, operator recovery, physical certification,
and compatibility admission/recertification policy remain open.

A verified one-shot formatting-prompt guide now documents complete initial
request inputs, prompt identity/version, explicit clipboard egress, untrusted
raw
response handling, and the separation from later command-mode refinement. It
keeps prompt serialization/identity computation, response parsing, candidate
validation, provider guidance, and end-to-end execution explicitly open.

A verified adapter-boundary guide now maps browser, localhost, clipboard, MCP,
Render, Export, and Plan contracts to their separate admission, effect, retry,
identity, persistence, and physical-safety boundaries. It also distinguishes
implemented browser/runtime behavior from contract-only CLI, MCP, file, model,
media, and hardware adapters.

A verified first-release troubleshooting guide now maps current startup,
handshake, draft, HTTP, clipboard, profile-integrity, capability, dry-run, and
recovery outcomes to fail-closed operator/developer actions. It explicitly
marks Export, CLI, MCP transport, model/media, encoders, and hardware adapters
as unavailable rather than inventing recovery procedures.

A verified documentation-coverage record now maps every category named by this
task to its current guide and marks design-only adapters, schemas, packaging,
and hardware controls unavailable until their owning implementation tasks add
real operator surfaces. This closes the documentation task for the executable
first-release surface; future implementation tasks own the documentation they
introduce.

### TODO - Close every known reproducible defect

Declare the first release complete only when P0 through P8 are closed, Jig is
clean, physical evidence is current, and no known product work remains.

## P9 — Optional compact multi-color writer

### TODO - Research a quiet compact multi-tool mechanism

Evaluate pen carousels, independent tool heads, registration, vibration, noise,
desk footprint, serviceability, and safe color-change recovery.

### TODO - Define a multi-tool capability profile

Add colors, highlighters, automatic tool changes, cleaning, verification, and
resume rules without changing the accepted single-pen plan contract.

### TODO - Build and physically certify the custom hardware

Treat the device as a separate product with its own electronics, firmware,
enclosure, safety, calibration, endurance, and manufacturing evidence.
