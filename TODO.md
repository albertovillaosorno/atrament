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

A process fixture now retains bytes for two semantic assets, commits text and
asset-reference history,
and proves both orderly and forced termination leave a fresh process with no
accepted revision, history, or bytes. Browser edits send complete authenticated
replacements and no browser persistence API is used.

This evidence establishes raw-byte ownership after a separate ingestion
boundary; it does not implement media decoding, format validation, clipboard or
file intake, or temporary conversion files. The task remains open until
previews, diagnostics, renders, plans, and the remaining media operations join
the same lifecycle invariant.

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

### TODO - Prove session destruction and temporary cleanup

Close, refresh, cancel, crash, and restart fixtures must show that ephemeral
state and media intermediates disappear while explicit exports remain intact.

Current runtime evidence proves orderly process termination releases the old
loopback listener, restart generates a fresh session credential, and a stale
credential cannot authenticate to the new process. Process-level draft fixtures
also write and read task, source, and raw-response text, then verify both
orderly restart and forced process death yield empty fields in the fresh
session. A checked-in application-process fixture now also populates an accepted
semantic revision containing two semantic asset records and a figure reference,
retains distinct raw byte sequences for both assets, commits text and
asset-reference history, then proves orderly exit and forced process death both
leave a fresh owner with no accepted revision, history position, or retained raw
bytes. Media decoding and temporary conversion-file cleanup remain open.

While private draft text is live, the runtime holds no writable regular-file
descriptor and changes no declared repository runtime-root file.

Checked-in browser policy tests guard one-time launch-credential fragment
consumption, credential invalidation on `pagehide`, in-flight handshake and
draft request cancellation, pending clipboard/draft invalidation, session-text
clearing, and bfcache subtree scrubbing. Aborted draft requests cannot
repopulate status text after page-exit invalidation. Refresh therefore cannot
recover the launch credential
from the rewritten browser URL or persistence API.

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
completeness metadata, and the localhost adapter refuses to invent a code for
an empty application diagnostic set.

Layout now provides the first accepted-revision producer beyond handshake and
session draft: fixed-region overflow emits stable blocking diagnostics with
semantic object/page locations, typed boundary and physical-length evidence,
operation binding, and complete-set semantics. A layout-only Export preflight
preserves those diagnostics rather than interpreting prose.

A transport-neutral semantic-command Apply producer now exists, but its typed
results do not yet carry `DiagnosticSet`. The shared envelope now reserves the
stable `atrament.semantic-command.precondition-rejected` code and typed local
precondition condition/failure evidence for family, kind, owner, target,
editability, and exact-value checks. Domain fixtures prove the code, Semantic
Validate binding, command/object/field locations, and typed evidence shape
without inventing adapter prose.

Complete Apply diagnostic projection remains open because accepted semantic
identities are intentionally opaque and command identities are still generic in
the pre-normalization batch. No stable diagnostic identity text or final command
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
extension-data preservation. Definitions are now a distinct semantic block kind
whose editable inline spans retain their own identities, style references, and
provenance references rather than being flattened into paragraph text. Explicit
candidate acceptance validates duplicate, dangling, and wrong-kind references
before mutation, promotes candidate-local identities through one backend-owned
mapping, and commits one new accepted revision atomically while preserving
nested semantic references.

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

Aggregate command behavior and typed-result behavior are version 54 after
admitting Definition blocks through the existing inline-span edit semantics.
Asset reference, Ordering and grouping, and Text content retain family behavior
version 1; Provenance and Style role are version 2, Document constraint is
version 3, and Structured content is version 46. The immediately previous
aggregate version 53 rejects instead of being reinterpreted.

A version-bound single-target proposal combines capability, exact local
preconditions, and direct-edit simulation read-only. All five dedicated direct
mutation paths consume that simulator before commit.

A separate generic command-graph domain validates duplicate command identities,
self-dependencies, missing dependencies, cycles, and dependency direction
without choosing command-ID syntax or changing command order. Interactive
command-ID selections can report their complete transitive omitted dependency
requirements without silently changing the caller's selection.

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
changing only `Page.page_profile`. It seeds that exact page with `AllDerived`;
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

A pinned release probe with the target first measured about 20 microseconds with
100,000 unrelated top-level blocks versus 1,099 microseconds before traversal
hardening. A nested first-child probe measured about 19 microseconds with
100,000
siblings versus 186 microseconds before slice frames. These are implementation
measurements, not product latency or resource guarantees.

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
mark. Squared grids keep identical nominal spacing on both axes. Accepted page
profiles own the maximum ruler-error envelope, rounded or sharp join treatment,
and mark layer.

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
Measurements bind to one accepted revision and one accepted flow; stale,
incomplete, reordered, unknown-flow, and out-of-flow measurements reject before
layout. Repeated measured fragments may retain one block owner, and no fragment
is split by pagination.

Keep-together groups move intact when a current or later page can contain them.
Groups too tall for any one remaining page fall back only to measured-fragment
boundaries, while exact-bottom fits do not manufacture page breaks. A page
profile edit invalidates old measurements, and fresh measurements reflow
deterministically against the new accepted writable geometry without backfilling
pages before the selected flow's owning page.

Measurement admission now streams contiguous block-owner runs instead of
materializing a second owner vector. Independent pagination advances through the
remaining page sequence once per fragment, and keep-together search computes one
checked total height and maximum width per group. Three reference oracles cover
7,504 small independent and keep-together placement cases, including nonzero
current-page remainder, while 341 owner-sequence cases cover complete,
incomplete, repeated, reordered, and out-of-flow semantic measurements.

Empty measured flows now avoid unused page-profile authority entirely. Nonempty
semantic pagination indexes page profiles once per call while preserving
first-match defensive behavior, and derives each writable profile region once
for all pages sharing it. Fixtures cover 10,000 ordered measured blocks, 10,000
additional distinct page/profile pairs, and 10,000 pages sharing one profile
without changing semantic order.

Runtime composition of this read-only pagination service remains open. The
current architecture declaration does not admit a dependency from the live
`SessionApplication` owner to `atrament_semantic_flow_pagination`, so adapters
must not bypass that boundary by wiring it ad hoc.

The task remains open for real handwriting and formula measurement, grapheme-
aware line breaking and paragraph wrapping, quotation and citation measurement,
column-flow policy, measurement diagnostics, and render or live-output
consumption.

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
first-journey 6 mm bottom overflow is exercised directly.

Stale placements, wrong page ownership, missing or invalid profiles, and
unrepresentable coordinates fail before a diagnostic can be presented as
current. Page-profile edits invalidate prior derived geometry, while nested
semantic blocks retain their accepted page ownership. Overflow amounts retain
the full physical `u64` range without diagnostic truncation.

A read-only layout-only Export preflight now consumes revision-bound layout
diagnostics and refuses layout readiness when evidence is blocking or explicitly
incomplete. Stale revisions, evidence from another revision, and non-layout
diagnostics cannot be smuggled through that gate. It accepts no path, overwrite,
retry, format, or file-commit input and therefore cannot report `Exported`.

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
alphabets, common one-group and wide accents, upright unit or label groups,
explicit `\text{...}` fragments, grouped custom operator names, stacked
annotations and relations, vector, directional over-arrows, overline, and
underline
decorations, escaped TeX special characters,
standard and variant Greek control-sequence notation, common named and delimiter
symbols, binary-operator symbols, arrow and ellipsis symbols, relation and logic
symbols, operators, calculus, set notation, aligned row and column separators,
and ordered aligned, brace-delimited-matrix, bracketed-matrix, cases,
double-vertical-bar-matrix, gathered, matrix, parenthesized-matrix,
small-matrix, split, and vertical-bar-matrix environments. Paired
delimiter-sizing controls remain explicit unsupported input rather than being
inferred from admitted delimiter glyph names.

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
allocation. Accepted promotion preserves valid spans exactly, and validation
keeps logical coverage compact instead of allocating per spanned column.

An exact-base direct cell-span edit reuses that complete table-grid invariant. A
valid edit preserves cell identity and child blocks, creates one accepted
revision, and enters Undo history; a structurally invalid replacement is a typed
no-effect. Table-cell spans are now Structured-content editable values in the
generic simulator. Ordered span commands validate cloned owning-table candidate
grids, Apply revalidates affected final tables atomically, and one Undo restores
the complete batch.

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
than inventing a normalization form. Normalization policy, grapheme-indexed
mutation, punctuation generation, and language-aware wrapping remain open.

### TODO - Make missing glyph coverage impossible to miss

Report every unsupported grapheme by profile and offer only a visible declared
fallback, profile repair, content replacement, or export refusal.

This task is currently blocked on an executable handwriting-profile coverage
authority. The accepted language ADR requires profiles to declare coverage per
grapheme or compositional rule, but the backend does not yet implement a
handwriting-profile domain or admitted fallback identity. Glyph validation must
not infer support from an installed font, renderer behavior, or successful text
acceptance.

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
remains untrusted until backend validation. Structured completion, object
navigation, candidate diagnostics, acceptance, and backend transport remain
unfinished.

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

### TODO - Implement image placement and layering

Expose source identity, crop, scale, opacity, z-order, below-text, inline,
above-text, and clipped-region placement without overwriting the original.

Current executable command evidence covers only source identity on an existing
semantic figure. `AssetReference` can attach, replace, or remove a reference to
an asset already admitted in the same accepted revision, including nested
figures and atomic Undo. The value cannot carry raw bytes, base64, filesystem
paths, or remote URLs, so it does not implement media ingestion.

Crop, scale, opacity, z-order, below-text, inline, above-text, clipped-region
placement, original-media ownership, and rendering remain open until their typed
semantic authorities are executable.

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

Backend prompt generation, identity, mode selection, and transport still keep
this task open.

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

### TODO - Build the guided calibration session

Collect isolated characters, joins, words, numerals, punctuation, mathematics,
titles, labels, sizes, speeds, pressure proxies, and free writing on known
paper.

### TODO - Correct photographed calibration geometry

Detect reference marks, perspective, lens distortion, scale, grid, baseline,
and capture quality before treating any observed stroke as evidence.

### TODO - Extract the personal stroke vocabulary

Derive centerlines, contours, entry and exit conditions, pen lifts, ligatures,
diacritics, contextual forms, and confidence with sample provenance.

### TODO - Implement compositional diacritics safely

Reuse accents only when profile evidence admits the composition and preserve
placement, scale, collision, and language-specific forms.

### TODO - Implement continuous contextual stroke planning

Select, connect, deform, space, and lift strokes from neighboring graphemes,
word position, line geometry, semantic role, and calibrated writing style.

### TODO - Implement bounded correlated variation

Support observed or authorized minimum, maximum, distribution, and correlation
for size, slant, roundness, flattening, spacing, drift, speed, and pressure.

### TODO - Implement handwriting roles and sizes

Allow one profile to expose body, title, subtitle, label, caption, formula,
margin, and annotation roles without pretending they are unrelated writers.

### TODO - Prevent repeated-glyph and repeated-line artifacts

Detect frozen contours, identical word rhythms, mechanical baselines, local
white noise, and configurations that leave the calibrated writer's envelope.

### TODO - Validate against held-out writing

Measure geometry, rhythm, joins, spacing, punctuation, and perceptual fidelity
against samples excluded from extraction and publish known failure modes.

## P5 — Deterministic CPU rendering and digital output

### TODO - Implement authoritative vector geometry

Generate layout boxes, stroke centerlines, expanded ink contours, equations,
rules, tables, and diagram paths with physical units and semantic provenance.

### TODO - Implement layered ink materials

Compose base deposition, edges, starvation, pooling, paper interaction, color,
and highlight layers without altering vector authority.

### TODO - Implement bounded page texture and soft noise

Apply seeded low-frequency texture at declared physical scale and prove it does
not move geometry, obscure small writing, or create repeated tiles.

### TODO - Implement fast and final CPU quality profiles

Use the same vectors, seeds, blend order, and page geometry while varying only
declared texture resolution and sampling cost.

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

### TODO - Implement digital paper notes and shadows

Render loose note fills, folds, stacking, and soft shadows as digital-only
objects with readable contrast and explicit live incompatibility.

### TODO - Implement theme-safe decorative titles

Support layered lettering, outlines, highlights, and motifs in digital mode and
derive a sober, single-pen alternative without losing title hierarchy.

### TODO - Produce vector-preserving PDF

Embed bounded texture and image resources, preserve physical page boxes and
color, retain searchable semantics when compatible, and emit a render manifest.

### TODO - Calibrate print and scan round trips

Measure physical scale, clipping, margins, grid registration, color shifts,
photo placement, line weight, and scanner distortion on representative devices.

### TODO - Meet CPU preview latency and memory budgets

Benchmark long pages, many images, dense equations, zoom, rapid edits, and final
export on ordinary integrated-graphics computers with no compute GPU.

## P6 — LLM, CLI, MCP, and media intake

### TODO - Freeze the one-shot formatting prompt

Include the complete backend-owned return format, allowed styles, output mode,
source rules, ambiguity behavior, and exact return envelope in one copy action.

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

The task remains open for richer source/citation linkage semantics, provenance
diagnostics, citation UI, and any source-authority model beyond existing
revision-owned provenance records.

### TODO - Prove complex educational coverage

Exercise bilingual mathematics, physics, chemistry, biology, history, and
language assignments with dense but readable page organization.

### TODO - Freeze the semantic command-batch envelope

Version the batch, base notebook revision, command-context identity, readable
context, writable targets or insertion anchors, preconditions, retry identity,
and normalized application receipt without exposing storage paths as document
semantics.

Current design evidence freezes those semantics, revision-owned command
families, acceptance fixtures, semantic normalization, command ordering,
dependency validity, retry-safe batch-local insertion handles, and backend-owned
resource limits without choosing final wire field names or JSON Schema. The task
remains open until the backend-owned envelope and compatibility rules are
implemented and versioned.

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

Current design evidence freezes autonomous-agent loop stop conditions, typed
progress versus non-progress, same-retry recovery, bounded automation budgets,
and the separation between semantic completion, explicit output, and physical
authority. Concrete MCP tools and backend execution remain open.

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

### TODO - Structure transcripts without hiding uncertainty

Turn reviewed transcript spans into sections, definitions, examples, and
formulas while retaining time ranges, confidence, speakers, and unresolved text.

## P7 — Honest single-pen live output

### TODO - Implement the live capability compiler

Reject or explicitly convert every color, highlight, photograph, shadow, paper
note, raster-only effect, or multi-tool action before motion planning.

### TODO - Implement the device-neutral motion plan

Emit ordered pen-up and pen-down paths, speed, acceleration, optional pressure,
safe bounds, pauses, checkpoints, semantic origin, and estimated duration.

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

### TODO - Build the hardware simulator and dry run

Visualize pen-up travel, pen-down paths, limits, time, pauses, and checkpoints
without connecting to a physical machine.

### TODO - Calibrate pen and blank-sheet coordinates

Measure usable area, origin, axis orientation, scale, skew, pen-up height,
contact height, speed, acceleration, page clamping, and boundary clearance.

### TODO - Implement the NextDraw and AxiDraw CLI adapter

Use documented SVG or CLI control behind a managed process boundary and prove
preview, plot, pause, resume, cancellation, and failure diagnostics.

### TODO - Implement documented HP-GL and GP-GL export

Map admitted motion capabilities to each command language without claiming
direct-device support for untested transports, firmware, or model behavior.

### TODO - Build the physical compatibility ledger

Record exact model, firmware, adapter tier, transport, paper, pen, usable area,
settings, evidence, known limitations, and last acceptance date.

### TODO - Prove safe interruption and uncertain-state recovery

Test disconnect, power loss, user pause, emergency stop, process crash, partial
stroke, unknown carriage position, restart, and refusal to resume unsafely.

### TODO - Prove complete single-pen notebook output

Write a multi-page English and Spanish fixture with titles, prose, equations,
tables, and line art on calibrated blank sheets with no manual content repair.

## P8 — First-release completion

### TODO - Complete data-format and migration compatibility

Test current, prior, future, corrupted, truncated, and partially unsupported
notebook and profile data with explicit, non-destructive outcomes.

### TODO - Fuzz every untrusted input boundary

Cover notebook bundles, `.atrament` profiles, clipboard HTML, images, TeX,
model responses, media metadata, CLI, MCP, and hardware status messages.

### TODO - Complete visual regression coverage

Compare semantic layout, vector topology, layer composition, final pixels,
digital themes, live themes, and overflow overlays independently.

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

A hostile page on a second loopback port can currently frame the static shell
and cause its three assets to load. The document-level CSP cannot provide an
effective `frame-ancestors` policy from a meta element, so the future localhost
server must emit a framing-denial response header and prove it against hostile
local origins.

These browser constraints do not replace backend socket, token, host, path,
cleanup, framing, or hostile-origin acceptance tests.

### TODO - Package reproducible desktop releases

Ship the Rust backend and TypeScript frontend as one verifiable installation
with checksums, licenses, offline startup, diagnostics, and clean uninstall.

### TODO - Complete operator and developer documentation

Document calibration, composition, one-shot LLM use, PDF, live safety, supported
hardware, profile internals, schemas, adapters, validation, and troubleshooting.

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
