# Atrament TODO — Rust CLI + localhost notebook editor

**Current implementation priority, November 2, 2026:** work locally, with
Rust as the authoritative backend, a usable command-line application, and an
optional localhost web UI for editing notebook and handwriting profiles.
The first deliverable is personal: load/edit/save profiles and generate real
printable PDF and transparent PNG notebook pages.

**Theoretical acceptance is sufficient for a completed design.** All 77
earlier P1–P9 task records stay in
[`docs/todo/completed/`](docs/todo/completed/) with their complex content,
worked examples and explicit limits intact. Completion of an academic design
does not claim the corresponding feature already runs. Hardware,
multi-platform installers, cloud, MCP, advanced semantic/LLM editing, and
physical plotters remain available for later without blocking local output.
See [audit](docs/technical/completed-todo-acceptance-audit-2026-11-03.md).

Four of nine **new executable integration tasks** remain open. The Rust
notebook and handwriting CLI file round-trip, authenticated localhost file
operations, notebook geometry, bounded statistics controls and the local
visual profile round-trip are complete. Actual handwriting, complete glyph
shapes, image layers, and printing remain open executable work.

See the [Rust-first roadmap](docs/technical/rust-cli-localhost-priority.md) and
[task policy](docs/todo/README.md). Keep the index unfinished-only.

## P4 — Personal statistical handwriting

### TODO - Spanish and English glyph coverage and fallback

Render the documented Spanish/English baseline, accents, punctuation,
guillemets, math symbols, and customizable per-character replacements.

[docs/todo/open/handwriting/012-abc.mdc](docs/todo/open/handwriting/012-abc.mdc)

## P5 — Rust digital rendering

### TODO - Rust notebook to PDF and transparent PNG

Render real physical-size notebook pages with ink, images, and printable output.

[docs/todo/open/render/011-pdf.mdc](docs/todo/open/render/011-pdf.mdc)

### TODO - Printed-page overlays and local printing

Avoid reprinting existing rules and send explicitly selected output to paper.

[docs/todo/open/render/012-print.mdc](docs/todo/open/render/012-print.mdc)

## P8 — Personal end-to-end acceptance

### TODO - Personal Rust CLI and localhost end-to-end journey

Demonstrate one CLI/localhost load, edit, save, render, and print-ready journey.

[docs/todo/open/release/009-local.mdc](docs/todo/open/release/009-local.mdc)
