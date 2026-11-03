# Atrament TODO — Rust CLI first, advanced work retained

The product priority is a **local Rust CLI and localhost browser editor** for
loading/saving notebook and handwriting profiles, tuning real ink/glyphs,
configuring paper and printing/exporting physical-size PDF and transparent PNG.
No Python renderer, remote account or physical plotter is required.

**Theoretical acceptance is sufficient for an academic design.** The 77
original designs remain completed and available in
[docs/todo/completed/](docs/todo/completed/). But every original design now
has a corresponding **open future implementation TODO** below: 25 bounded
follow-up tracks map all 77 original design IDs exactly once. None of the
complex features was removed simply for being difficult.

## Priorities

1. **NOW — usable local notebook:** finish bilingual glyph shapes, native
   handwritten PDF/PNG, preprinted-sheet printing and the end-to-end CLI/UI
   journey (the four original open implementation tasks).
2. **AFTER MVP — improve the result:** photo and image placement, measured
   flow, mathematical and educational blocks, realistic pen/ink, handwriting
   roles, advanced editor controls, robust validation and automation.
3. **OPTIONAL — no release blocker:** audio/video, packaging and other OSes,
   vendor plotter adapters, custom hardware and its physical certification.

The initial four tasks remain open until their practical deliverables exist;
accepted theoretical work is not falsely described as running code. Later
features stay in this TODO index but must not block priority 1.
Follow the [task policy](docs/todo/README.md),
[Rust-first roadmap](docs/technical/rust-cli-localhost-priority.md) and
[audit](docs/technical/completed-todo-acceptance-audit-2026-11-03.md).

## First delivery — local Rust CLI and notebook output

## P4 — Handwriting and bilingual glyphs

### TODO - Spanish and English glyph coverage and fallback

**NOW.** Render the documented Spanish/English baseline, accents, punctuation,
guillemets, math symbols, and customizable per-character replacements.

[docs/todo/open/handwriting/012-abc.mdc](docs/todo/open/handwriting/012-abc.mdc)

## P5 — PDF/PNG and printing

### TODO - Rust notebook to PDF and transparent PNG

**NOW.** Render real physical-size notebook pages with ink, images, and
printable output.

[docs/todo/open/render/011-pdf.mdc](docs/todo/open/render/011-pdf.mdc)

### TODO - Printed-page overlays and local printing

**NOW.** Avoid reprinting existing rules and send explicitly selected output to
paper.

[docs/todo/open/render/012-print.mdc](docs/todo/open/render/012-print.mdc)

## P8 — Working end-to-end local journey

### TODO - Personal Rust CLI and localhost end-to-end journey

**NOW.** Demonstrate one CLI/localhost load, edit, save, render, and
print-ready journey.

[docs/todo/open/release/009-local.mdc](docs/todo/open/release/009-local.mdc)

## Second delivery — richer notebook and writing

## P3 — Advanced local editing and image placement

### TODO - Add advanced direct page editing selection and undo/redo

**AFTER MVP.** Extend the localhost workspace with keyboard and pointer page
edits, cross-panel semantic selection, bounded undo/redo and visible collision
correction.

Accepted design sources: ATR-P3-001, ATR-P3-003, ATR-P3-004, ATR-P3-005,
ATR-P3-010.

[docs/todo/open/editor/012-d.mdc](docs/todo/open/editor/012-d.mdc)

### TODO - Complete structured source clipboard and candidate review

**AFTER MVP.** Wire the structured LLM source editor, rich clipboard intake,
Copy prompt identity and transactional pasted-response review to Rust
admission.

Accepted design sources: ATR-P3-002, ATR-P3-006, ATR-P3-008, ATR-P3-009.

[docs/todo/open/editor/013-c.mdc](docs/todo/open/editor/013-c.mdc)

### TODO - Edit photo and illustration transforms visually

**AFTER MVP.** Connect authorized image intake, crop, transform, opacity and
front/behind layering to the Rust notebook/page model.

Accepted design sources: ATR-P3-007.

[docs/todo/open/editor/014-i.mdc](docs/todo/open/editor/014-i.mdc)

## P4 — Handwriting calibration, joins, roles and realism

### TODO - Add optional photographed handwriting calibration workflow

**AFTER MVP.** Implement the versioned handwriting-container sections,
printable calibration guide, optional photo geometry correction and curated
stroke vocabulary.

Accepted design sources: ATR-P4-001, ATR-P4-002, ATR-P4-003, ATR-P4-004.

[docs/todo/open/handwriting/013-c.mdc](docs/todo/open/handwriting/013-c.mdc)

### TODO - Implement joined strokes diacritics and correlated variation

**AFTER MVP.** Connect accent placement, contextual joining and seeded
cross-line glyph/pressure variation to actual Rust stroke geometry.

Accepted design sources: ATR-P4-005, ATR-P4-006, ATR-P4-007.

[docs/todo/open/handwriting/014-s.mdc](docs/todo/open/handwriting/014-s.mdc)

### TODO - Tune handwriting roles and measure repeated-glyph quality

**AFTER MVP.** Add body/title size roles, repeat-artifact detection, and an
optional human-reviewed held-out writing comparison.

Accepted design sources: ATR-P4-008, ATR-P4-009, ATR-P4-010.

[docs/todo/open/handwriting/015-q.mdc](docs/todo/open/handwriting/015-q.mdc)

## P5 — Vector resources, textures and CPU performance

### TODO - Preserve vector geometry and text resources in native PDF

**AFTER MVP.** Connect Rust vector-authoritative geometry and a bounded
font/line/path resource writer to complete notebook PDF output.

Accepted design sources: ATR-P5-001, ATR-P5-008.

[docs/todo/open/render/013-v.mdc](docs/todo/open/render/013-v.mdc)

### TODO - Add pen material texture paper effects and decorative themes

**AFTER MVP.** Implement bounded ink deposition, seeded paper texture, digital
note/shadow layers and contrast-aware decorative heading treatments in Rust.

Accepted design sources: ATR-P5-002, ATR-P5-003, ATR-P5-006, ATR-P5-007.

[docs/todo/open/render/014-m.mdc](docs/todo/open/render/014-m.mdc)

### TODO - Optimize CPU preview line art and print scan calibration

**AFTER MVP.** Add CPU Preview/Final profiles, bounded transparent line-art
extraction, measurable render costs and optional one-sheet print/scan
correction.

Accepted design sources: ATR-P5-004, ATR-P5-005, ATR-P5-009, ATR-P5-010.

[docs/todo/open/render/015-r.mdc](docs/todo/open/render/015-r.mdc)

## P2 — Page flow, educational math and bilingual layout

### TODO - Compile semantic notebooks with measured flow and fixed regions

**AFTER MVP.** Connect the existing semantic notebook model, measured
pagination, fixed placement and four-edge overflow diagnostics in Rust.

Accepted design sources: ATR-P2-001, ATR-P2-003, ATR-P2-004, ATR-P2-005.

[docs/todo/open/semantic/011-l.mdc](docs/todo/open/semantic/011-l.mdc)

### TODO - Render editable mathematics tables and educational structures

**AFTER MVP.** Connect typed equations, worked calculations, tables and ruled
school blocks to Rust layout and selected-region editing.

Accepted design sources: ATR-P2-006, ATR-P2-007.

[docs/todo/open/semantic/012-s.mdc](docs/todo/open/semantic/012-s.mdc)

### TODO - Add calibrated page marks and bilingual text layout diagnostics

**AFTER MVP.** Implement seeded measured ruler marks, English/Spanish line
breaking and explicit missing-glyph warnings above the minimal bilingual
inventory.

Accepted design sources: ATR-P2-002, ATR-P2-008, ATR-P2-009.

[docs/todo/open/semantic/013-b.mdc](docs/todo/open/semantic/013-b.mdc)

## P1 — Session state and diagnostics

### TODO - Extend session state and explicit notebook import/export

**AFTER MVP.** Connect verified native profile admission to session-owned
notebook state and caller-selected Import/Export, without implicit writes.

Accepted design sources: ATR-P1-001, ATR-P1-002.

[docs/todo/open/runtime/007-s.mdc](docs/todo/open/runtime/007-s.mdc)

### TODO - Complete session cleanup and shared runtime diagnostics

**AFTER MVP.** Wire bounded session-end cleanup and stable typed diagnostics
through notebook, renderer and optional asset adapters.

Accepted design sources: ATR-P1-003, ATR-P1-004.

[docs/todo/open/runtime/008-l.mdc](docs/todo/open/runtime/008-l.mdc)

## P8 — Migration, privacy and regression assurance

### TODO - Harden data migration untrusted inputs and localhost privacy

**AFTER MVP.** Implement concrete legacy-format refusal/migration policy,
bounded adverse-input coverage and the documented localhost credential/privacy
fixes.

Accepted design sources: ATR-P8-001, ATR-P8-002, ATR-P8-005.

[docs/todo/open/release/010-s.mdc](docs/todo/open/release/010-s.mdc)

### TODO - Add visual goldens and complete keyboard-only editor journeys

**AFTER MVP.** Introduce reproducible notebook PNG/PDF goldens and the keyboard
navigation/refusal flow for every currently implemented page-editor control.

Accepted design sources: ATR-P8-003, ATR-P8-004.

[docs/todo/open/release/011-r.mdc](docs/todo/open/release/011-r.mdc)

## P6 — Optional semantic commands and agent integration

### TODO - Integrate one-shot prompts and accountable school note structuring

**AFTER MVP.** Wire versioned formatting prompts, structured assignment intake,
claim/source provenance and bilingual subject examples into admitted notebook
candidates.

Accepted design sources: ATR-P6-001, ATR-P6-002, ATR-P6-003, ATR-P6-004.

[docs/todo/open/automation/014-p.mdc](docs/todo/open/automation/014-p.mdc)

### TODO - Execute atomic semantic command batches with scoped recomputation

**AFTER MVP.** Implement schema-backed command envelope, exact-base two-step
Apply, history receipts and dependency-limited derived layout updates.

Accepted design sources: ATR-P6-005, ATR-P6-006, ATR-P6-007.

[docs/todo/open/automation/015-c.mdc](docs/todo/open/automation/015-c.mdc)

### TODO - Provide clipboard CLI and optional MCP semantic command parity

**AFTER MVP.** Project the same admitted application command/result schema to
clipboard, Rust CLI, an optional local MCP adapter and an offline agent guide.

Accepted design sources: ATR-P6-008, ATR-P6-009, ATR-P6-010, ATR-P6-011.

[docs/todo/open/automation/016-a.mdc](docs/todo/open/automation/016-a.mdc)

## Optional future research — not required for local PDF

## P6 — Audio/video and uncertain transcripts

### TODO - Optionally transcribe local audio/video with explicit uncertainty

**OPTIONAL.** Add an opt-in local media decoder, bounded temporary-file
lifetime, timed transcript roles and unresolved-word review before semantic
insertion.

Accepted design sources: ATR-P6-012, ATR-P6-013.

[docs/todo/open/automation/017-m.mdc](docs/todo/open/automation/017-m.mdc)

## P8 — Platform distribution beyond personal use

### TODO - Prepare optional releases documentation and true release-gate ledger

**OPTIONAL.** Track actual reproducible desktop packages, current
operator/developer guidance and a defect ledger that never implies unfinished
features are complete.

Accepted design sources: ATR-P8-006, ATR-P8-007, ATR-P8-008.

[docs/todo/open/release/012-d.mdc](docs/todo/open/release/012-d.mdc)

## P7 — Pen-machine plans and physical output

### TODO - Implement optional device-neutral pen path and ordering

**OPTIONAL.** Convert admitted single-pen geometry to a typed motion plan,
retaining pen lifts and identity while optionally reducing safe travel.

Accepted design sources: ATR-P7-001, ATR-P7-002, ATR-P7-003.

[docs/todo/open/live/011-m.mdc](docs/todo/open/live/011-m.mdc)

### TODO - Implement offline plot preview calibration and safe-stop contracts

**OPTIONAL.** Add device-free dry-run visualization, opt-in paper coordinate
calibration, compatibility ledger and uncertain-interruption refusal.

Accepted design sources: ATR-P7-004, ATR-P7-005, ATR-P7-008, ATR-P7-009.

[docs/todo/open/live/012-s.mdc](docs/todo/open/live/012-s.mdc)

### TODO - Add optional documented plotter adapters and physical acceptance

**OPTIONAL.** Implement explicit AxiDraw/NextDraw or HP-GL/GP-GL export
contracts and a conditional two-page single-pen physical trial.

Accepted design sources: ATR-P7-006, ATR-P7-007, ATR-P7-010.

[docs/todo/open/live/013-v.mdc](docs/todo/open/live/013-v.mdc)

## P9 — Custom multi-tool hardware

### TODO - Optionally prototype and certify a compact multi-tool writer

**OPTIONAL.** If ever commissioned, implement the accepted four-tool capability
extension and hardware mechanism comparison with evidence gates separate from
digital output.

Accepted design sources: ATR-P9-001, ATR-P9-002, ATR-P9-003.

[docs/todo/open/hardware/004-m.mdc](docs/todo/open/hardware/004-m.mdc)
