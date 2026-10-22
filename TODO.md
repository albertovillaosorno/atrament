# Atrament TODO

Only unfinished work appears here. P1 through P8 are the first-release delivery
horizons, and P9 is optional hardware research. Within each horizon, open work
retains its original order and stable task identity. Typed records are the
execution authority for lanes, dependencies, acceptance, and evidence.

Full scope, acceptance criteria, evidence, dependencies, and planning notes
remain in typed records under `docs/todo/open/`. Completed work belongs under
`docs/todo/completed/`.

**Canonical TODO format:** one `### TODO - ...` title, one synthesis paragraph,
then one direct Markdown link to the complete typed record. No per-item field
labels belong here.

## P1 — Localhost runtime and ephemeral session

### TODO - Keep all notebook state in memory

Hold documents, assets, undo history, previews, and derived plans only for
the active session with no database, autosave, hidden recovery file, or cloud
copy.

[docs/todo/open/runtime/001-keep-all-notebook-state-in-memory.mdc](docs/todo/open/runtime/001-keep-all-notebook-state-in-memory.mdc)

### TODO - Implement explicit import and export

Allow deliberate notebook bundle and `.atrament` profile reads or writes at
caller-selected or explicitly host-policy-admitted paths without converting
them into background persistence. Keep paths mentioned only in notebook,
source, diagnostic, clipboard, or model-response text as data rather than
Export authority.

[docs/todo/open/runtime/002-implement-explicit-import-and-export.mdc](docs/todo/open/runtime/002-implement-explicit-import-and-export.mdc)

### TODO - Prove session destruction and temporary cleanup

Close, refresh, cancel, crash, and restart fixtures must show that ephemeral
state and media intermediates disappear while explicit exports remain intact.

[docs/todo/open/runtime/003-session-destruction-cleanup.mdc](docs/todo/open/runtime/003-session-destruction-cleanup.mdc)

### TODO - Define one typed diagnostic envelope

Represent field, object, page, source, glyph, collision, capability,
renderer, and hardware errors with stable codes and actionable locations.

[docs/todo/open/runtime/004-define-one-typed-diagnostic-envelope.mdc](docs/todo/open/runtime/004-define-one-typed-diagnostic-envelope.mdc)

## P2 — Semantic notebook and physical layout

### TODO - Implement the semantic notebook model

Represent notebooks, pages, flows, blocks, spans, formulas, tables, figures,
styles, assets, constraints, output profiles, and provenance with stable IDs.

[docs/todo/open/semantic/001-implement-the-semantic-notebook-model.mdc](docs/todo/open/semantic/001-implement-the-semantic-notebook-model.mdc)

### TODO - Implement calibrated grid and rule geometry

Preserve nominal spacing and square cells while allowing bounded ruler error,
rounded joins, layer order, and profile-specific appearance.

[docs/todo/open/semantic/002-calibrated-grid-rule-geometry.mdc](docs/todo/open/semantic/002-calibrated-grid-rule-geometry.mdc)

### TODO - Implement measured text flow and pagination

Wrap paragraphs, lists, quotations, and citations from actual handwriting
metrics and move complete semantic fragments to following pages predictably.

[docs/todo/open/semantic/003-text-flow-pagination.mdc](docs/todo/open/semantic/003-text-flow-pagination.mdc)

### TODO - Implement fixed-region constraint solving

Place titles, figures, callouts, and freeform regions with anchors,
alignment, minimum size, collision policy, and explicit infeasibility
diagnostics.

[docs/todo/open/semantic/004-fixed-region-constraints.mdc](docs/todo/open/semantic/004-fixed-region-constraints.mdc)

### TODO - Prevent invisible page overflow

Block export while fixed content crosses the writable region and identify the
object, violated edge, amount, and valid move, crop, resize, or reflow
choices.

[docs/todo/open/semantic/005-prevent-invisible-page-overflow.mdc](docs/todo/open/semantic/005-prevent-invisible-page-overflow.mdc)

### TODO - Implement mathematics as editable structure

Parse and preserve TeX-compatible inline, displayed, aligned, matrix, unit,
and derivation content while exposing unsupported constructs without
rewriting.

[docs/todo/open/semantic/006-editable-mathematics.mdc](docs/todo/open/semantic/006-editable-mathematics.mdc)

### TODO - Implement tables and ruled educational blocks

Support merged cells, headers, alignment, wrapping, ruler-like borders,
boxes, dividers, arrows, labels, and page references.

[docs/todo/open/semantic/007-tables-educational-blocks.mdc](docs/todo/open/semantic/007-tables-educational-blocks.mdc)

### TODO - Implement English and Spanish text behavior

Normalize Unicode, edit by grapheme, apply language-aware punctuation and
wrapping, and preserve curly quotes, guillemets, accents, and en or em
dashes.

[docs/todo/open/semantic/008-english-spanish-text.mdc](docs/todo/open/semantic/008-english-spanish-text.mdc)

### TODO - Make missing glyph coverage impossible to miss

Report every unsupported grapheme by profile and offer only a visible
declared fallback, profile repair, content replacement, or export refusal.

[docs/todo/open/semantic/009-missing-glyph-coverage.mdc](docs/todo/open/semantic/009-missing-glyph-coverage.mdc)

## P3 — The dual human and LLM editor

### TODO - Build the responsive 16:9 split shell

Show both editors concurrently with adjustable division, stable selection,
keyboard navigation, zoom, page list, diagnostics, and no hidden main
surface.

[docs/todo/open/editor/001-build-the-responsive-16-9-split-shell.mdc](docs/todo/open/editor/001-build-the-responsive-16-9-split-shell.mdc)

### TODO - Build the structured LLM editor

Edit task instructions and notebook source with structured completion,
formatting, object navigation, diagnostics, and a safe raw-response
inspection boundary.

[docs/todo/open/editor/002-build-the-structured-llm-editor.mdc](docs/todo/open/editor/002-build-the-structured-llm-editor.mdc)

### TODO - Build the human page editor

Support selection, text correction, drag, resize, crop, align, group, layer,
duplicate, delete, and style changes through typed semantic commands.

[docs/todo/open/editor/003-build-the-human-page-editor.mdc](docs/todo/open/editor/003-build-the-human-page-editor.mdc)

### TODO - Synchronize selection between both editors

Selecting source, a diagnostic, a page object, or a table cell must focus the
same stable semantic identity in the other surface.

[docs/todo/open/editor/004-synchronize-editor-selection.mdc](docs/todo/open/editor/004-synchronize-editor-selection.mdc)

### TODO - Implement semantic undo and redo

Record accepted command batches rather than DOM or canvas snapshots and
preserve stable semantic identities plus deterministic recomputation across
history. Expose undo and redo as application-history commands, not semantic
operations embedded in a new batch.

[docs/todo/open/editor/005-implement-semantic-undo-and-redo.mdc](docs/todo/open/editor/005-implement-semantic-undo-and-redo.mdc)

### TODO - Implement rich clipboard intake

Accept text, structured table fragments, formulas, PNG, JPEG, and WebP while
reporting exactly which source structure or metadata cannot be retained.

[docs/todo/open/editor/006-implement-rich-clipboard-intake.mdc](docs/todo/open/editor/006-implement-rich-clipboard-intake.mdc)

### TODO - Implement image placement and layering

Expose source identity, crop, scale, opacity, z-order, below-text, inline,
above-text, and clipped-region placement without overwriting the original.

[docs/todo/open/editor/007-implement-image-placement-and-layering.mdc](docs/todo/open/editor/007-implement-image-placement-and-layering.mdc)

### TODO - Keep the Copy prompt control permanently available

Generate one complete, versioned request from current task and constraints,
copy it with one action, and confirm the exact prompt identity copied.

[docs/todo/open/editor/008-copy-prompt-control.mdc](docs/todo/open/editor/008-copy-prompt-control.mdc)

### TODO - Validate pasted model responses transactionally

Parse the complete response into a candidate notebook, show all differences
and errors, and change the accepted session only after explicit acceptance.

[docs/todo/open/editor/009-transactional-response-validation.mdc](docs/todo/open/editor/009-transactional-response-validation.mdc)

### TODO - Make overflow and collision correction visual

Draw page-edge, collision, and unsupported-mode diagnostics on preview and
link each overlay to the owning source object and correction actions.

[docs/todo/open/editor/010-visual-overflow-collision-correction.mdc](docs/todo/open/editor/010-visual-overflow-collision-correction.mdc)

## P4 — Personal handwriting profiles and synthesis

### TODO - Freeze the first `.atrament` container

Choose canonical encoding, manifest, checksums, optional assets, version
rules, unknown-field behavior, migration, and a human-readable inspection
command.

[docs/todo/open/handwriting/001-freeze-the-first-atrament-container.mdc](docs/todo/open/handwriting/001-freeze-the-first-atrament-container.mdc)

### TODO - Build the guided calibration session

Collect isolated characters, joins, words, numerals, punctuation,
mathematics, titles, labels, sizes, speeds, pressure proxies, and free
writing on known paper.

[docs/todo/open/handwriting/002-guided-calibration-session.mdc](docs/todo/open/handwriting/002-guided-calibration-session.mdc)

### TODO - Correct photographed calibration geometry

Detect reference marks, perspective, lens distortion, scale, grid, baseline,
and capture quality before treating any observed stroke as evidence.

[docs/todo/open/handwriting/003-photographed-calibration-geometry.mdc](docs/todo/open/handwriting/003-photographed-calibration-geometry.mdc)

### TODO - Extract the personal stroke vocabulary

Derive centerlines, contours, entry and exit conditions, pen lifts,
ligatures, diacritics, contextual forms, and confidence with sample
provenance.

[docs/todo/open/handwriting/004-personal-stroke-vocabulary.mdc](docs/todo/open/handwriting/004-personal-stroke-vocabulary.mdc)

### TODO - Implement compositional diacritics safely

Reuse accents only when profile evidence admits the composition and preserve
placement, scale, collision, and language-specific forms.

[docs/todo/open/handwriting/005-compositional-diacritics.mdc](docs/todo/open/handwriting/005-compositional-diacritics.mdc)

### TODO - Implement continuous contextual stroke planning

Select, connect, deform, space, and lift strokes from neighboring graphemes,
word position, line geometry, semantic role, and calibrated writing style.

[docs/todo/open/handwriting/006-contextual-stroke-planning.mdc](docs/todo/open/handwriting/006-contextual-stroke-planning.mdc)

### TODO - Implement bounded correlated variation

Support observed or authorized minimum, maximum, distribution, and
correlation for size, slant, roundness, flattening, spacing, drift, speed,
and pressure.

[docs/todo/open/handwriting/007-bounded-correlated-variation.mdc](docs/todo/open/handwriting/007-bounded-correlated-variation.mdc)

### TODO - Implement handwriting roles and sizes

Allow one profile to expose body, title, subtitle, label, caption, formula,
margin, and annotation roles without pretending they are unrelated writers.

[docs/todo/open/handwriting/008-handwriting-roles-sizes.mdc](docs/todo/open/handwriting/008-handwriting-roles-sizes.mdc)

### TODO - Prevent repeated-glyph and repeated-line artifacts

Detect frozen contours, identical word rhythms, mechanical baselines, local
white noise, and configurations that leave the calibrated writer's envelope.

[docs/todo/open/handwriting/009-repeated-glyph-line-artifacts.mdc](docs/todo/open/handwriting/009-repeated-glyph-line-artifacts.mdc)

### TODO - Validate against held-out writing

Measure geometry, rhythm, joins, spacing, punctuation, and perceptual
fidelity against samples excluded from extraction and publish known failure
modes.

[docs/todo/open/handwriting/010-validate-against-held-out-writing.mdc](docs/todo/open/handwriting/010-validate-against-held-out-writing.mdc)

## P5 — Deterministic CPU rendering and digital output

### TODO - Implement authoritative vector geometry

Generate layout boxes, stroke centerlines, expanded ink contours, equations,
rules, tables, and diagram paths with physical units and semantic provenance.

[docs/todo/open/render/001-implement-authoritative-vector-geometry.mdc](docs/todo/open/render/001-implement-authoritative-vector-geometry.mdc)

### TODO - Implement layered ink materials

Compose base deposition, edges, starvation, pooling, paper interaction,
color, and highlight layers without altering vector authority.

[docs/todo/open/render/002-implement-layered-ink-materials.mdc](docs/todo/open/render/002-implement-layered-ink-materials.mdc)

### TODO - Implement bounded page texture and soft noise

Apply seeded low-frequency texture at declared physical scale and prove it
does not move geometry, obscure small writing, or create repeated tiles.

[docs/todo/open/render/003-page-texture-soft-noise.mdc](docs/todo/open/render/003-page-texture-soft-noise.mdc)

### TODO - Implement fast and final CPU quality profiles

Use the same vectors, seeds, blend order, and page geometry while varying
only declared texture resolution and sampling cost.

[docs/todo/open/render/004-cpu-quality-profiles.mdc](docs/todo/open/render/004-cpu-quality-profiles.mdc)

### TODO - Implement configurable line-art extraction

Convert images to transparent single-color paths with levels, threshold,
cleanup, detail, minimum feature, and preview controls suitable for hand
color.

[docs/todo/open/render/005-line-art-extraction.mdc](docs/todo/open/render/005-line-art-extraction.mdc)

### TODO - Implement digital paper notes and shadows

Render loose note fills, folds, stacking, and soft shadows as digital-only
objects with readable contrast and explicit live incompatibility.

[docs/todo/open/render/006-digital-paper-notes-shadows.mdc](docs/todo/open/render/006-digital-paper-notes-shadows.mdc)

### TODO - Implement theme-safe decorative titles

Support layered lettering, outlines, highlights, and motifs in digital mode
and derive a sober, single-pen alternative without losing title hierarchy.

[docs/todo/open/render/007-implement-theme-safe-decorative-titles.mdc](docs/todo/open/render/007-implement-theme-safe-decorative-titles.mdc)

### TODO - Produce vector-preserving PDF

Embed bounded texture and image resources, preserve physical page boxes and
color, retain searchable semantics when compatible, and emit a render
manifest.

[docs/todo/open/render/008-produce-vector-preserving-pdf.mdc](docs/todo/open/render/008-produce-vector-preserving-pdf.mdc)

### TODO - Calibrate print and scan round trips

Measure physical scale, clipping, margins, grid registration, color shifts,
photo placement, line weight, and scanner distortion on representative
devices.

[docs/todo/open/render/009-calibrate-print-and-scan-round-trips.mdc](docs/todo/open/render/009-calibrate-print-and-scan-round-trips.mdc)

### TODO - Meet CPU preview latency and memory budgets

Benchmark long pages, many images, dense equations, zoom, rapid edits, and
final export on ordinary integrated-graphics computers with no compute GPU.

[docs/todo/open/render/010-preview-latency-memory-budgets.mdc](docs/todo/open/render/010-preview-latency-memory-budgets.mdc)

## P6 — LLM, CLI, MCP, and media intake

### TODO - Freeze the one-shot formatting prompt

Include the complete backend-owned return format, allowed styles, output
mode, source rules, ambiguity behavior, and exact return envelope in one copy
action.

[docs/todo/open/automation/001-one-shot-formatting-prompt.mdc](docs/todo/open/automation/001-one-shot-formatting-prompt.mdc)

### TODO - Implement assignment-to-notebook structuring

Support titles, explanations, derivations, tables, equations, diagrams,
citations, examples, and conclusions without inventing missing task facts.

[docs/todo/open/automation/002-assignment-notebook-structuring.mdc](docs/todo/open/automation/002-assignment-notebook-structuring.mdc)

### TODO - Implement source and claim provenance

Distinguish provided, derived, cited, and unverified content and link every
citation to the exact claim and source metadata the user must review. Keep
that semantic provenance separate from application transaction provenance
recording whether an accepted mutation entered through direct human editing,
clipboard-assisted model response, CLI, or MCP.

[docs/todo/open/automation/003-source-claim-provenance.mdc](docs/todo/open/automation/003-source-claim-provenance.mdc)

### TODO - Prove complex educational coverage

Exercise bilingual mathematics, physics, chemistry, biology, history, and
language assignments with dense but readable page organization.

[docs/todo/open/automation/004-prove-complex-educational-coverage.mdc](docs/todo/open/automation/004-prove-complex-educational-coverage.mdc)

### TODO - Freeze the semantic command-batch envelope

Version the batch, base notebook revision, command-context identity, readable
context, writable targets or insertion anchors, preconditions, retry
identity, and normalized application receipt without exposing storage paths
as document semantics.

[docs/todo/open/automation/005-semantic-command-batch-envelope.mdc](docs/todo/open/automation/005-semantic-command-batch-envelope.mdc)

### TODO - Implement atomic command validation and apply

Validate the whole batch against one accepted snapshot, reject stale or
invalid commands without partial mutation, and make retry behavior idempotent
enough for CLI and MCP automation.

[docs/todo/open/automation/006-atomic-command-validation-apply.mdc](docs/todo/open/automation/006-atomic-command-validation-apply.mdc)

### TODO - Implement impact-scoped recomputation

Compute changed semantic identities and dependency-expanded invalidation so a
small command does not regenerate unrelated notebook content, while every
derived layout, handwriting, diagnostic, preview, export, or motion result
that can change is recomputed.

[docs/todo/open/automation/007-impact-scoped-recomputation.mdc](docs/todo/open/automation/007-impact-scoped-recomputation.mdc)

### TODO - Add clipboard command-mode round trips

Let the backend present a self-contained command-mode prompt for an accepted
revision and parse the pasted command envelope through the same untrusted raw
response boundary. Delimit notebook/task/source material as data, exclude
session credentials and internal paths, and treat prompt-injection-like
source prose as untrusted content rather than application authority. Show the
semantic diff and diagnostics before interactive acceptance rather than
parsing commands in TypeScript.

[docs/todo/open/automation/008-clipboard-command-round-trips.mdc](docs/todo/open/automation/008-clipboard-command-round-trips.mdc)

### TODO - Expose CLI parity for every application command

Create, inspect, validate, transform, render, export, and plan hardware
without requiring a browser or changing domain behavior.

[docs/todo/open/automation/009-cli-application-command-parity.mdc](docs/todo/open/automation/009-cli-application-command-parity.mdc)

### TODO - Expose MCP from the same command schemas

Project bounded capability discovery, inspect, validate, apply, render,
export, and plan capabilities for agents and prove normalized receipts match
CLI and interactive application. Keep physical `arm` and `start` behind their
separate device-safety boundary.

[docs/todo/open/automation/010-mcp-command-schema-parity.mdc](docs/todo/open/automation/010-mcp-command-schema-parity.mdc)

### TODO - Package self-contained agent instructions

Allow a user to provide the repository or release bundle to an agent and have
it discover the exact CLI, schemas, examples, and validation workflow
locally. Command-mode instructions must distinguish readable context from
writable scope, forbid agent-allocated accepted IDs and embedded adapter
effects, and use an admitted unresolved response when the requested edit
cannot be represented.

[docs/todo/open/automation/011-self-contained-agent-instructions.mdc](docs/todo/open/automation/011-self-contained-agent-instructions.mdc)

### TODO - Implement optional audio and video transcription

Normalize supported media to bounded temporary audio, invoke the admitted
WhisperX adapter, preserve word timing and confidence, and always clean up.

[docs/todo/open/automation/012-audio-video-transcription.mdc](docs/todo/open/automation/012-audio-video-transcription.mdc)

### TODO - Structure transcripts without hiding uncertainty

Turn reviewed transcript spans into sections, definitions, examples, and
formulas while retaining time ranges, confidence, speakers, and unresolved
text.

[docs/todo/open/automation/013-transcript-uncertainty.mdc](docs/todo/open/automation/013-transcript-uncertainty.mdc)

## P7 — Honest single-pen live output

### TODO - Implement the live capability compiler

Reject or explicitly convert every color, highlight, photograph, shadow,
paper note, raster-only effect, or multi-tool action before motion planning.

[docs/todo/open/live/001-implement-the-live-capability-compiler.mdc](docs/todo/open/live/001-implement-the-live-capability-compiler.mdc)

### TODO - Implement the device-neutral motion plan

Emit ordered pen-up and pen-down paths, speed, acceleration, optional
pressure, safe bounds, pauses, checkpoints, semantic origin, and estimated
duration.

[docs/todo/open/live/002-implement-the-device-neutral-motion-plan.mdc](docs/todo/open/live/002-implement-the-device-neutral-motion-plan.mdc)

### TODO - Optimize path order without changing handwriting

Reduce pen-up travel and drying conflicts while preserving stroke order where
joins, ink behavior, semantics, or the handwriting profile require it.

[docs/todo/open/live/003-path-order-preserves-handwriting.mdc](docs/todo/open/live/003-path-order-preserves-handwriting.mdc)

### TODO - Build the hardware simulator and dry run

Visualize pen-up travel, pen-down paths, limits, time, pauses, and
checkpoints without connecting to a physical machine.

[docs/todo/open/live/004-build-the-hardware-simulator-and-dry-run.mdc](docs/todo/open/live/004-build-the-hardware-simulator-and-dry-run.mdc)

### TODO - Calibrate pen and blank-sheet coordinates

Measure usable area, origin, axis orientation, scale, skew, pen-up height,
contact height, speed, acceleration, page clamping, and boundary clearance.

[docs/todo/open/live/005-calibrate-pen-and-blank-sheet-coordinates.mdc](docs/todo/open/live/005-calibrate-pen-and-blank-sheet-coordinates.mdc)

### TODO - Implement the NextDraw and AxiDraw CLI adapter

Use documented SVG or CLI control behind a managed process boundary and prove
preview, plot, pause, resume, cancellation, and failure diagnostics.

[docs/todo/open/live/006-nextdraw-axidraw-cli-adapter.mdc](docs/todo/open/live/006-nextdraw-axidraw-cli-adapter.mdc)

### TODO - Implement documented HP-GL and GP-GL export

Map admitted motion capabilities to each command language without claiming
direct-device support for untested transports, firmware, or model behavior.

[docs/todo/open/live/007-hp-gl-gp-gl-export.mdc](docs/todo/open/live/007-hp-gl-gp-gl-export.mdc)

### TODO - Build the physical compatibility ledger

Record exact model, firmware, adapter tier, transport, paper, pen, usable
area, settings, evidence, known limitations, and last acceptance date.

[docs/todo/open/live/008-build-the-physical-compatibility-ledger.mdc](docs/todo/open/live/008-build-the-physical-compatibility-ledger.mdc)

### TODO - Prove safe interruption and uncertain-state recovery

Test disconnect, power loss, user pause, emergency stop, process crash,
partial stroke, unknown carriage position, restart, and refusal to resume
unsafely.

[docs/todo/open/live/009-safe-interruption-recovery.mdc](docs/todo/open/live/009-safe-interruption-recovery.mdc)

### TODO - Prove complete single-pen notebook output

Write a multi-page English and Spanish fixture with titles, prose, equations,
tables, and line art on calibrated blank sheets with no manual content
repair.

[docs/todo/open/live/010-prove-complete-single-pen-notebook-output.mdc](docs/todo/open/live/010-prove-complete-single-pen-notebook-output.mdc)

## P8 — First-release completion

### TODO - Complete data-format and migration compatibility

Test current, prior, future, corrupted, truncated, and partially unsupported
notebook and profile data with explicit, non-destructive outcomes.

[docs/todo/open/release/001-data-format-migration-compatibility.mdc](docs/todo/open/release/001-data-format-migration-compatibility.mdc)

### TODO - Fuzz every untrusted input boundary

Cover notebook bundles, `.atrament` profiles, clipboard HTML, images, TeX,
model responses, media metadata, CLI, MCP, and hardware status messages.

[docs/todo/open/release/002-fuzz-every-untrusted-input-boundary.mdc](docs/todo/open/release/002-fuzz-every-untrusted-input-boundary.mdc)

### TODO - Complete visual regression coverage

Compare semantic layout, vector topology, layer composition, final pixels,
digital themes, live themes, and overflow overlays independently.

[docs/todo/open/release/003-complete-visual-regression-coverage.mdc](docs/todo/open/release/003-complete-visual-regression-coverage.mdc)

### TODO - Complete accessibility and keyboard operation

Make both editors, page navigation, diagnostics, drag alternatives, prompt
copy, import, export, and hardware arming usable without a pointer.

[docs/todo/open/release/004-accessibility-keyboard-operation.mdc](docs/todo/open/release/004-accessibility-keyboard-operation.mdc)

### TODO - Prove localhost security and privacy

Test loopback binding, session token, hostile local pages, file-path
confinement, temporary cleanup, no telemetry, no autosave, and no undeclared
network access.

[docs/todo/open/release/005-prove-localhost-security-and-privacy.mdc](docs/todo/open/release/005-prove-localhost-security-and-privacy.mdc)

### TODO - Package reproducible desktop releases

Ship the Rust backend and TypeScript frontend as one verifiable installation
with checksums, licenses, offline startup, diagnostics, and clean uninstall.

[docs/todo/open/release/006-package-reproducible-desktop-releases.mdc](docs/todo/open/release/006-package-reproducible-desktop-releases.mdc)

### TODO - Complete operator and developer documentation

Document calibration, composition, one-shot LLM use, PDF, live safety,
supported hardware, profile internals, schemas, adapters, validation, and
troubleshooting.

[docs/todo/open/release/007-operator-developer-documentation.mdc](docs/todo/open/release/007-operator-developer-documentation.mdc)

### TODO - Close every known reproducible defect

Declare the first release complete only when P1 through P8 are closed, Jig is
clean, physical evidence is current, and no known product work remains.

[docs/todo/open/release/008-close-every-known-reproducible-defect.mdc](docs/todo/open/release/008-close-every-known-reproducible-defect.mdc)

## P9 — Optional compact multi-color writer

### TODO - Research a quiet compact multi-tool mechanism

Evaluate pen carousels, independent tool heads, registration, vibration,
noise, desk footprint, serviceability, and safe color-change recovery.

[docs/todo/open/hardware/001-research-a-quiet-compact-multi-tool.mdc](docs/todo/open/hardware/001-research-a-quiet-compact-multi-tool.mdc)

### TODO - Define a multi-tool capability profile

Add colors, highlighters, automatic tool changes, cleaning, verification, and
resume rules without changing the accepted single-pen plan contract.

[docs/todo/open/hardware/002-define-a-multi-tool-capability-profile.mdc](docs/todo/open/hardware/002-define-a-multi-tool-capability-profile.mdc)

### TODO - Build and physically certify the custom hardware

Treat the device as a separate product with its own electronics, firmware,
enclosure, safety, calibration, endurance, and manufacturing evidence.

[docs/todo/open/hardware/003-custom-hardware-certification.mdc](docs/todo/open/hardware/003-custom-hardware-certification.mdc)
