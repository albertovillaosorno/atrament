# Atrament TODO

Only unfinished work appears here. P1 through P8 are the first-release delivery
horizons, and P9 is optional hardware research. Within each horizon, open work
retains its original order and stable task identity. Typed records are the
execution authority for lanes, dependencies, acceptance, and evidence.

Each record now has a finite first-release completion criterion. Where a
practical demonstration is unavailable, theoretical acceptance can close its
design scope without claiming the feature works or is safe to operate.
See [the acceptance policy](docs/todo/README.md).

Full scope, acceptance criteria, evidence, dependencies, and planning notes
remain in typed records under `docs/todo/open/`. Completed work belongs under
`docs/todo/completed/`.

**Canonical TODO format:** one `### TODO - ...` title, one synthesis paragraph,
then one direct Markdown link to the complete typed record. No per-item field
labels belong here.

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
