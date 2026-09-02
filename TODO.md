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

Current executable evidence keeps source-preparation task text, source material,
and raw model-response text in one backend application object owned by the
active process. Accepted semantic notebook revisions now also live only inside
one process-local application service: a fresh service starts without accepted
state after the prior owner is dropped, and its Debug projection does not expose
private notebook text. Browser edits send complete authenticated replacements
and no browser persistence API is used.

The task remains open until the accepted-revision service is composed into the
live session and assets, history, previews, diagnostics, renders, and plans all
exist with the same executable lifecycle invariant.

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
format-specific execution remain open.

### TODO - Prove session destruction and temporary cleanup

Close, refresh, cancel, crash, and restart fixtures must show that ephemeral
state and media intermediates disappear while explicit exports remain intact.

Current runtime evidence proves orderly process termination releases the old
loopback listener, restart generates a fresh session credential, and a stale
credential cannot authenticate to the new process. Process-level draft fixtures
also write and read task, source, and raw-response text, then verify both
orderly restart and forced process death yield empty fields in the fresh
session. While private draft text is live, the runtime holds no writable
regular-file descriptor and changes no declared repository runtime-root file.

The task remains open until accepted notebook state and temporary media exist
and fixtures also cover browser close, refresh, cancellation, media cleanup,
and explicit-export survival.

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

The task remains open because semantic-command, layout, Render, Export, and Plan
producers do not exist yet, and CLI/MCP parity fixtures therefore cannot prove
that those capabilities preserve the same diagnostic semantics.

## P2 — Semantic notebook and physical layout

### TODO - Implement the semantic notebook model

Represent notebooks, pages, flows, blocks, spans, formulas, tables, figures,
styles, assets, constraints, output profiles, and provenance with stable IDs.

Current executable evidence defines transport-independent typed values for those
semantic families, separate opaque candidate, accepted, and revision identities,
non-recycling active-session allocation, unresolved semantic blocks, and exact
extension-data preservation. Explicit candidate acceptance validates duplicate,
dangling, and wrong-kind references before mutation, promotes candidate-local
identities through one backend-owned mapping, and commits one new accepted
revision atomically while preserving nested semantic references. A direct
accepted-text edit now preconditions the exact current revision, preserves all
semantic identities while replacing one admitted inline text identity, creates
one new revision only for a real change, and rejects no-op, stale, unavailable,
or non-text targets without mutating accepted state.

The task remains open for the complete first-release semantic vocabulary,
format parsing and canonical serialization, migrations and round-trip fixtures,
full semantic command/history integration, measured layout/render consumers,
and format-compatible import/export.

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

The task remains open for seeded calibrated ruler-path synthesis, additional
profile-specific visual appearance, and renderer or live-output consumption.

### TODO - Implement measured text flow and pagination

Wrap paragraphs, lists, quotations, and citations from actual handwriting
metrics and move complete semantic fragments to following pages predictably.

### TODO - Implement fixed-region constraint solving

Place titles, figures, callouts, and freeform regions with anchors, alignment,
minimum size, collision policy, and explicit infeasibility diagnostics.

### TODO - Prevent invisible page overflow

Block export while fixed content crosses the writable region and identify the
object, violated edge, amount, and valid move, crop, resize, or reflow choices.

### TODO - Implement mathematics as editable structure

Parse and preserve TeX-compatible inline, displayed, aligned, matrix, unit, and
derivation content while exposing unsupported constructs without rewriting.

### TODO - Implement tables and ruled educational blocks

Support merged cells, headers, alignment, wrapping, ruler-like borders, boxes,
dividers, arrows, labels, definitions, and page references.

### TODO - Implement English and Spanish text behavior

Normalize Unicode, edit by grapheme, apply language-aware punctuation and
wrapping, and preserve curly quotes, guillemets, accents, and en or em dashes.

### TODO - Make missing glyph coverage impossible to miss

Report every unsupported grapheme by profile and offer only a visible declared
fallback, profile repair, content replacement, or export refusal.

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

Current design evidence freezes new revision identities for every accepted
history traversal, stable semantic identity restoration, branch invalidation,
transaction provenance, typed traversal outcomes, read-only traversal
availability, same-retry lost-receipt recovery, concurrency, and session-only
history lifetime. Backend history storage and execution remain open.

### TODO - Implement rich clipboard intake

Accept text, structured table fragments, formulas, PNG, JPEG, and WebP while
reporting exactly which source structure or metadata cannot be retained.

### TODO - Implement image placement and layering

Expose source identity, crop, scale, opacity, z-order, below-text, inline,
above-text, and clipped-region placement without overwriting the original.

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

Current command-mode design evidence includes a provenance-only acceptance case
where source/citation metadata changes while claim text and identity stay
stable.
Backend provenance storage, validation, and citation UI remain open.

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

Current design evidence covers stale bases, retry identity, no-op behavior,
concurrent commits, scope escape, unsupported requests, forged identities,
semantic local preconditions, middle-command failure, lost Apply receipts,
normalized retry equality, invalid command dependencies, and typed application
result classes. No tracked Rust application core exists yet, so executable apply
remains open.

### TODO - Implement impact-scoped recomputation

Compute changed semantic identities and dependency-expanded invalidation so a
small command does not regenerate unrelated notebook content, while every
derived layout, handwriting, diagnostic, preview, export, or motion result that
can change is recomputed.

Current design evidence separates semantic change from derived impact and covers
one-paragraph, table-cell, spatial, asset-reference, and document-wide
constraint
fixtures. Dependency tracking and incremental backend recomputation remain open.

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
