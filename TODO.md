# Atrament TODO

Only unfinished work appears here. P1 is highest priority; P4 is lowest.
Detailed tasks are in `docs/todo/open/`; completed work is in
`docs/todo/completed/`.

## P1 — Notebook and handwriting

### TODO - Spanish and English glyph coverage and fallback

Render the documented Spanish/English baseline, accents, punctuation,
guillemets, math symbols, and customizable per-character replacements.

[docs/todo/open/handwriting/012-abc.mdc](docs/todo/open/handwriting/012-abc.mdc)

### TODO - Implement joined strokes diacritics and correlated variation

Connect accent placement, contextual joining and seeded cross-line
glyph/pressure variation to actual Rust stroke geometry.

[docs/todo/open/handwriting/014-s.mdc](docs/todo/open/handwriting/014-s.mdc)

### TODO - Rust notebook to PDF and transparent PNG

Render real physical-size notebook pages with ink, images, and printable
output.

[docs/todo/open/render/011-pdf.mdc](docs/todo/open/render/011-pdf.mdc)

### TODO - Edit photo and illustration transforms visually

Connect authorized image intake, crop, transform, opacity and front/behind
layering to the Rust notebook/page model.

[docs/todo/open/editor/014-i.mdc](docs/todo/open/editor/014-i.mdc)

### TODO - Printed-page overlays and local printing

Avoid reprinting existing rules and send explicitly selected output to paper.

[docs/todo/open/render/012-print.mdc](docs/todo/open/render/012-print.mdc)

### TODO - Personal Rust CLI and localhost end-to-end journey

Demonstrate one CLI/localhost load, edit, save, render, and print-ready
journey.

[docs/todo/open/release/009-local.mdc](docs/todo/open/release/009-local.mdc)

## P2 — Editing, realism and reliability

### TODO - Extend session state and explicit notebook import/export

Connect verified native profile admission to session-owned notebook state and
caller-selected Import/Export, without implicit writes.

[docs/todo/open/runtime/007-s.mdc](docs/todo/open/runtime/007-s.mdc)

### TODO - Complete session cleanup and shared runtime diagnostics

Wire bounded session-end cleanup and stable typed diagnostics through notebook,
renderer and optional asset adapters.

[docs/todo/open/runtime/008-l.mdc](docs/todo/open/runtime/008-l.mdc)

### TODO - Compile semantic notebooks with measured flow and fixed regions

Connect the existing semantic notebook model, measured pagination, fixed
placement and four-edge overflow diagnostics in Rust.

[docs/todo/open/semantic/011-l.mdc](docs/todo/open/semantic/011-l.mdc)

### TODO - Render editable mathematics tables and educational structures

Connect typed equations, worked calculations, tables and ruled school blocks to
Rust layout and selected-region editing.

[docs/todo/open/semantic/012-s.mdc](docs/todo/open/semantic/012-s.mdc)

### TODO - Add calibrated page marks and bilingual text layout diagnostics

Implement seeded measured ruler marks, English/Spanish line breaking and
explicit missing-glyph warnings above the minimal bilingual inventory.

[docs/todo/open/semantic/013-b.mdc](docs/todo/open/semantic/013-b.mdc)

### TODO - Add advanced direct page editing selection and undo/redo

Extend the localhost workspace with keyboard and pointer page edits,
cross-panel semantic selection, bounded undo/redo and visible collision
correction.

[docs/todo/open/editor/012-d.mdc](docs/todo/open/editor/012-d.mdc)

### TODO - Add optional photographed handwriting calibration workflow

Implement the versioned handwriting-container sections, printable calibration
guide, optional photo geometry correction and curated stroke vocabulary.

[docs/todo/open/handwriting/013-c.mdc](docs/todo/open/handwriting/013-c.mdc)

### TODO - Tune handwriting roles and measure repeated-glyph quality

Add body/title size roles, repeat-artifact detection, and an optional
human-reviewed held-out writing comparison.

[docs/todo/open/handwriting/015-q.mdc](docs/todo/open/handwriting/015-q.mdc)

### TODO - Preserve vector geometry and text resources in native PDF

Connect Rust vector-authoritative geometry and a bounded font/line/path
resource writer to complete notebook PDF output.

[docs/todo/open/render/013-v.mdc](docs/todo/open/render/013-v.mdc)

### TODO - Add pen material texture paper effects and decorative themes

Implement bounded ink deposition, seeded paper texture, digital note/shadow
layers and contrast-aware decorative heading treatments in Rust.

[docs/todo/open/render/014-m.mdc](docs/todo/open/render/014-m.mdc)

### TODO - Optimize CPU preview line art and print scan calibration

Add CPU Preview/Final profiles, bounded transparent line-art extraction,
measurable render costs and optional one-sheet print/scan correction.

[docs/todo/open/render/015-r.mdc](docs/todo/open/render/015-r.mdc)

### TODO - Harden data migration untrusted inputs and localhost privacy

Implement concrete legacy-format refusal/migration policy, bounded
adverse-input coverage and the documented localhost credential/privacy fixes.

[docs/todo/open/release/010-s.mdc](docs/todo/open/release/010-s.mdc)

### TODO - Add visual goldens and complete keyboard-only editor journeys

Introduce reproducible notebook PNG/PDF goldens and the keyboard
navigation/refusal flow for every currently implemented page-editor control.

[docs/todo/open/release/011-r.mdc](docs/todo/open/release/011-r.mdc)

## P3 — Automation and advanced workflows

### TODO - Complete structured source clipboard and candidate review

Wire the structured LLM source editor, rich clipboard intake, Copy prompt
identity and transactional pasted-response review to Rust admission.

[docs/todo/open/editor/013-c.mdc](docs/todo/open/editor/013-c.mdc)

### TODO - Integrate one-shot prompts and accountable school note structuring

Wire versioned formatting prompts, structured assignment intake, claim/source
provenance and bilingual subject examples into admitted notebook candidates.

[docs/todo/open/automation/014-p.mdc](docs/todo/open/automation/014-p.mdc)

### TODO - Execute atomic semantic command batches with scoped recomputation

Implement schema-backed command envelope, exact-base two-step Apply, history
receipts and dependency-limited derived layout updates.

[docs/todo/open/automation/015-c.mdc](docs/todo/open/automation/015-c.mdc)

### TODO - Provide clipboard CLI and optional MCP semantic command parity

Project the same admitted application command/result schema to clipboard, Rust
CLI, an optional local MCP adapter and an offline agent guide.

[docs/todo/open/automation/016-a.mdc](docs/todo/open/automation/016-a.mdc)

## P4 — Optional integrations and hardware

### TODO - Optionally transcribe local audio/video with explicit uncertainty

Add an opt-in local media decoder, bounded temporary-file lifetime, timed
transcript roles and unresolved-word review before semantic insertion.

[docs/todo/open/automation/017-m.mdc](docs/todo/open/automation/017-m.mdc)

### TODO - Prepare optional releases documentation and true release-gate ledger

Track actual reproducible desktop packages, current operator/developer guidance
and a defect ledger that never implies unfinished features are complete.

[docs/todo/open/release/012-d.mdc](docs/todo/open/release/012-d.mdc)

### TODO - Implement optional device-neutral pen path and ordering

Convert admitted single-pen geometry to a typed motion plan, retaining pen
lifts and identity while optionally reducing safe travel.

[docs/todo/open/live/011-m.mdc](docs/todo/open/live/011-m.mdc)

### TODO - Implement offline plot preview calibration and safe-stop contracts

Add device-free dry-run visualization, opt-in paper coordinate calibration,
compatibility ledger and uncertain-interruption refusal.

[docs/todo/open/live/012-s.mdc](docs/todo/open/live/012-s.mdc)

### TODO - Add optional documented plotter adapters and physical acceptance

Implement explicit AxiDraw/NextDraw or HP-GL/GP-GL export contracts and a
conditional two-page single-pen physical trial.

[docs/todo/open/live/013-v.mdc](docs/todo/open/live/013-v.mdc)

### TODO - Optionally prototype and certify a compact multi-tool writer

If ever commissioned, implement the accepted four-tool capability extension and
hardware mechanism comparison with evidence gates separate from digital output.

[docs/todo/open/hardware/004-m.mdc](docs/todo/open/hardware/004-m.mdc)
