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

No open TODO records remain as of November 2, 2026. The finite task
records were accepted as practical or theoretical deliverables under the
[acceptance policy](docs/todo/README.md); that does **not** establish a
production-ready application or certified machine.

The dated `ATR-P8-008` release decision ledger remains **NO-GO** because
identified security, PDF, packaging, and verification work is incomplete.
Optional P9 hardware also has design acceptance only.

## Scope correction — personal CLI, November 2, 2026

The owner's latest requirement replaces the old multiplatform/browser/plotter
product ambition with a **single Linux CLI notebook-to-PDF/PNG generator**.
The critical experience is statistical handwriting editing, physical-page
profiles, manual per-character refinement, custom images, and printed-page
handling. This is a personal academic aid, not a hardware project.

The 77 prior items were **accepted as finite practical or theoretical design
deliverables**, not certified as running product features; preserve their
completed records and historical evidence. None of their speculative
multiplatform, web, MCP, plotter, or robot requirements may delay the scoped
CLI. Physical hardware is research-only; no physical demonstration is needed.
Theoretical acceptance still must never be described as executed functionality.

The standalone CLI at `cli/notebook.py` now implements a bounded user path.
For scope and what is still unavailable, follow [README.md](README.md), not
the superseded full-platform product narrative in old design records.
