# Atrament TODO — Rust CLI + localhost notebook editor

**Current implementation priority, November 2, 2026:** work locally, with
Rust as the authoritative backend, a usable command-line application, and an
optional localhost web UI for editing notebook and handwriting profiles.
The first deliverable is personal: load/edit/save profiles and generate real
printable PDF and transparent PNG notebook pages.

**Do not delete the difficult work.** All 77 earlier P1–P9 tasks remain in
[`docs/todo/completed/`](docs/todo/completed/) as bounded theoretical or
partially implemented academic deliverables. Completion of those artifacts
never proves the corresponding feature runs. Hardware, multi-platform
installers, cloud, MCP, advanced semantic/LLM editing, and physical plotters
are retained for later, not needed to complete the first local workflow.

Eight of nine **new executable integration tasks** remain open. The
Rust notebook and handwriting CLI file round-trip is complete; other tasks
retain their original identities, finite scope and dependencies. See the
[Rust-first roadmap](docs/technical/rust-cli-localhost-priority.md) and
[task policy](docs/todo/README.md). Keep the index unfinished-only.

## P1 — Rust profiles and local runtime

### TODO - Authenticated localhost profile file operations

Use the existing loopback Rust server for authorized profile load and save.

[docs/todo/open/runtime/006-web.mdc](docs/todo/open/runtime/006-web.mdc)

## P2 — Notebook geometry

### TODO - Editable physical notebook page profiles

Edit grid, rules, sheet size, margins, date, border, and printed-page flags.

[docs/todo/open/semantic/010-page.mdc](docs/todo/open/semantic/010-page.mdc)

## P3 — Localhost profile UI

### TODO - Localhost visual notebook and handwriting editors

Open, preview, change every admitted scalar, and save separate profile files.

[docs/todo/open/editor/011-ui.mdc](docs/todo/open/editor/011-ui.mdc)

## P4 — Personal statistical handwriting

### TODO - Rust statistical handwriting and per-glyph controls

Own bounded handwriting distributions and every existing editable variable.

[docs/todo/open/handwriting/011-ink.mdc](docs/todo/open/handwriting/011-ink.mdc)

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
