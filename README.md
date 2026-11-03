# Atrament

> Project started: August 31, 2026

**An artificial penmanship atelier that alchemizes text, lectures, formulas,
and photographs into uncannily human notebooks—PDF or real ink.**

Atrament is a disposable localhost workspace for composing polished handwritten
notes. The first-release design admits either a complete structured notebook or
a targeted semantic command batch from an LLM, then validates accepted state
before PDF, preview, live-plan, or physical-output projections.

It is not a word processor, a handwriting font, or a cloud notebook. Content is
semantic, layout is compiled against a physical page, handwriting comes from a
calibrated personal profile, and every visual adjustment remains an explicit
constraint.

## Personal priority: automate handwritten notebooks

Atrament exists because repetitive compulsory handwriting wastes time I would
rather spend learning, thinking, and living. The practical priority is a
**local Rust CLI and localhost web editor** for configuring notebook paper and
statistical handwriting, including per-glyph adjustments and symbols used in
Mexico, Spain, and English. The editor must load and save separate notebook
and handwriting profiles, expose all admitted numeric controls with ranges,
show a real preview, and export printable PDF or transparent PNG pages.

**Rust is the application authority.** The existing TypeScript UI is a thin
localhost client, not a separate renderer or profile engine. Do not add a
Python notebook application, a second server, or parallel profile semantics.
Complex layout, mathematics, LLM tooling, animation, cross-platform packaging,
and physical plotters are preserved as future work; they must not block the
first real notebook-to-PDF journey.

For the staged, unfinished work see [TODO.md](TODO.md) and the
[Rust-first local roadmap](docs/technical/rust-cli-localhost-priority.md).

## Current implementation and release status (November 3, 2026)

Atrament already has a Rust localhost server and browser workspace,
authenticated draft transport, physical-page and handwriting domain contracts,
and a local `.atrament` profile verifier, inspector, and canonical repacker.
Run the Rust binary without arguments for the existing localhost workspace;
its notebook metadata, paper, handwriting statistics controls and native
PDF/PNG paper preview are connected. Local profile file read/write now has an
explicit opt-in directory; full rendered handwriting is still unfinished.
The available CLI commands are documented below.

The native bounded statistical controls and the localhost profile editing
workflow now have a real Firefox-to-Rust acceptance test: separate notebook
and handwriting profiles are edited with sliders/numeric inputs, saved to
the opted-in local directory, reopened, and previewed. An exact `ñ` override
and maximum unsigned 64-bit seed survive the round-trip, while invalid
statistics cannot overwrite the accepted file. PNG preview is the notebook
**paper layer**; handwriting remains numeric-only, not generated ink.

The same browser-saved notebook profile also opens in the independent Rust
`atrament-profiles` CLI and exports a real two-page paper PDF and 150-DPI
transparent PNG. The selected PNG page has an identical SHA-256 digest to
the browser preview. These remain paper-layer exports, not drawn handwriting
or full notebook content.

The earlier 77 tasks were accepted as bounded academic/design artifacts, not
as proof of a finished product. **Four of nine executable TODOs remain**
for actual Spanish/English glyph coverage, rendered notebook content,
explicit local printing and the complete personal end-to-end journey.
The previous NO-GO decision still applies to production claims, and
physical hardware is not required for local use.

### Localhost Open/Save for notebook and handwriting profiles (opt-in)

The existing **Rust** loopback workspace accepts an explicitly selected,
pre-existing directory for two distinct native profile types. No server-side
file writes are enabled in the default no-argument mode:

```sh
mkdir -p ./my-atrament-profiles
cargo run -q -p atrament_runtime_bootstrap -- --profiles-dir \
  ./my-atrament-profiles
```

After the normal authenticated session opens in the browser, use **Notebook
page profile → Local notebook filename → Save local notebook** or
**Handwriting statistics → Local handwriting filename → Save local
handwriting**. **Open local** loads an existing file and its exact revision;
after editing, **Save local** checks that revision before allowing a
replacement. The original **Load JSON** file picker and **Save JSON**
download still work without this directory opt-in.

Names are deliberately limited to one ASCII `*.json` basename (maximum
64 characters); paths and symlinks cannot be selected by the browser.
The Rust host stores each type separately as `notebook-NAME` and
`handwriting-NAME` in the selected directory. For example, entering
`pages.json` and `writer.json` produces
`my-atrament-profiles/notebook-pages.json` and
`my-atrament-profiles/handwriting-writer.json`.
The directory must exist when the process starts, and the current atomic
file commit transport supports Linux; neither an upload nor a download
silently changes this directory.

The authenticated, same-origin `GET /api/notebook/file` and
`GET /api/handwriting/file` require a unique
`X-Atrament-Profile-Name: NAME` header. Each returns canonical native
profile JSON and a `sha256:` revision for the exact source bytes.
Corresponding `POST` requests also require Origin, JSON body, and
`X-Atrament-Profile-Revision: new` to create a new target, or the
previously read SHA-256 revision to replace an observed regular file.
Existing/new-name collisions and stale revisions return HTTP 409;
invalid JSON, names, duplicates or unexpected paths refuse before commit.

Without `--profiles-dir`, these file routes respond HTTP 503, while
read-only validation and PDF/PNG rendering continue to work.

The selected-directory save uses the existing Rust bounded-reader and
Linux atomic file commit adapter. It is not a general remote file manager,
does not persist anything in browser storage, and does not provide a
cross-process lock against simultaneous external writers. A failed
or uncertain commit must not be treated as permission to blindly retry an
overwrite; reopen the profile and inspect the result first.

### Native Rust printable-paper PDF and PNG (limited first slice)

The `atrament-paper` Rust CLI can already generate a physically sized **blank,
ruled, squared-grid, or dotted sheet** using the existing Rust page-profile,
nominal paper-mark, and vector PDF components. For example:

```sh
cargo run -q -p atrament_notebook_paper_cli -- \
  --style grid --width-mm 210 --height-mm 297 --spacing-mm 5 \
  --margin-top-mm 22 --margin-bottom-mm 16 \
  --margin-left-mm 20 --margin-right-mm 14 \
  --line-color '#c5d4e7' > paper.pdf
```

The default is an A4 5-mm square grid; `--style ruled`, `--style dot`, and
`--style blank` select other paper. `--paper-size a3|a4|a5|b5|letter|legal`
selects standard physical dimensions. `--landscape` or `--portrait`
explicitly chooses orientation, including for a loaded paper profile.

Set a paper-size preset before any manual `--width-mm` or `--height-mm`
overrides; the saved JSON retains the resulting exact dimensions. All
measurements are millimeters and the PDF uses physical point dimensions,
not screenshot pixels. Invalid options fail before the CLI writes PDF bytes.

This is a **paper-only** CLI: it does not load/save notebook or handwriting
profiles, draw handwriting, embed images, or provide handwriting controls.
The localhost page includes a paper-only editor with numeric inputs and
sliders, plus a separate Rust-validated handwriting statistics editor.
Neither editor is a complete notebook profile or handwriting renderer.

A printable collection of identical paper sheets is possible with `--pages`
(1–32). This is an output-only choice, not a field in the paper profile:

```sh
cargo run -q -p atrament_notebook_paper_cli -- \
  --paper-size a5 --style ruled --spacing-mm 7 --pages 12 > sheets.pdf
```

Pages contain the selected paper marks only; they do not yet include
handwriting, dates, page numbers, or custom per-page content.

For a **transparent PNG** of one physical paper sheet, select Rust PNG export
and DPI instead of the default vector PDF:

```sh
cargo run -q -p atrament_notebook_paper_cli -- \
  --paper-size a5 --style grid --spacing-mm 5 \
  --format png --dpi 150 --output sheet.png
```

Rust renders paper marks as RGBA pixels over transparency and writes the
physical DPI into PNG metadata. The supported DPI range is 72–300; output
is limited to five million pixels and 24 MB. A4 at 300 DPI is too large,
so use a lower DPI for larger sheets. The PNG exporter supports one sheet,
not handwriting, embedded image assets, or PDF's `--pages` option.

For explicit file saving instead of shell redirection, pass `--output FILE`.
It uses the existing Rust atomic file-commit adapter and refuses to overwrite
an existing file or symlink. `--replace` permits replacement of an existing
regular file, never a symlink; invalid profiles and geometry are rejected
before the target is changed. An uncertain post-commit durability failure is
reported explicitly rather than treated as a safe retry:

```sh
cargo run -q -p atrament_notebook_paper_cli -- \
  --paper-size letter --style ruled --pages 2 --output ruled.pdf
cargo run -q -p atrament_notebook_paper_cli -- \
  --style grid --emit-profile --output paper-profile.json
cargo run -q -p atrament_notebook_paper_cli -- \
  --profile paper-profile.json --spacing-mm 4.5 \
  --emit-profile --output paper-profile.json --replace
```

The explicit file-saving transport is Linux-only for now. Without `--output`,
the original stdout PDF/JSON behavior remains unchanged. PNG may also be
written to stdout explicitly with `--format png`.

The authenticated localhost runtime also exposes this **same Rust authority**.
After a compatible session handshake, `GET /api/paper/profile` returns the
canonical default JSON; `POST /api/paper/profile` checks and canonicalizes a
paper-only profile; `POST /api/paper/pdf` produces its physical vector
PDF; and `POST /api/paper/png` produces transparent RGBA paper bytes.

PNG POST also requires exactly one `X-Atrament-Png-Dpi: 150` header
(value 72–300) and returns `Content-Type: image/png`. Every POST
requires the exact session Origin, Bearer credential and bounded JSON
input. These routes do not save files server-side or render handwriting.

The same Rust session now admits `GET /api/handwriting/profile` for the
uncalibrated default, `POST /api/handwriting/profile` for canonical, bounded
statistics validation, and `GET /api/handwriting/dimensions` for the ten
variable names, physical units, and domain bounds. All three require the
current session credential; POST additionally requires the exact Origin.
These default validation endpoints do not themselves save files or render
written glyphs. The opt-in file endpoints above are separate.

The localhost workspace presents a **paper profile** editor after a
successful session handshake. It offers A3/A4/A5/B5/Letter/Legal presets,
blank/ruled/grid/dot patterns, physical dimensions, editable spacing and
four margins (numeric fields and synchronized sliders), and line color.

Choose **Load JSON** to validate a selected paper-only profile through Rust,
**Save JSON** to download the Rust-canonicalized profile, **Export PDF** for
vector paper, or **Export transparent PNG** for a pixel image. PNG DPI has
linked numeric and slider inputs. The browser renders neither PDF nor PNG
and persists no profile in browser storage; downloads are explicit actions.

A separate **handwriting statistics** section loads the Rust dimension
catalog and versioned JSON profile only after session admission. It allows
editing all ten native dimensions with numeric fields and sliders for minimum,
maximum, mean, median, and deviation, plus seed, and exact per-grapheme
overrides such as `ñ` and decomposed accents.

**Load JSON** validates with
Rust, **Save JSON** downloads canonical Rust-validated statistics, and
**Default** restores the uncalibrated native seed-zero profile. Bounds and
units come from Rust; this is a numeric statistics preview, not actual ink
rendering. **Preview seeded number in Rust** uses the current (including
unsaved) controls, an exact grapheme, the selected native dimension, and a
zero-based occurrence. Authenticated `POST /api/handwriting/preview` checks
the whole profile and returns a repeatable integer for identical inputs.
This is not calibrated ink or glyph shape data, and the preview saves no file.

The localhost editor preserves all unsigned 64-bit seeds and occurrence
indices, including values above JavaScript safe integer precision, as exact
decimal JSON tokens. It requires a modern browser with JSON parse source
context; browsers without it refuse rather than round a saved seed.

Server-side file persistence is available only by explicitly launching the
Rust runtime with a selected directory as described below; full notebook
content and handwriting rendering are not implemented yet.

For a reusable paper-only profile, the same CLI can emit versioned JSON,
read it again, and apply further explicit edits:

```sh
cargo run -q -p atrament_notebook_paper_cli -- \
  --style ruled --spacing-mm 7 --emit-profile > paper-profile.json
cargo run -q -p atrament_notebook_paper_cli -- \
  --profile paper-profile.json --margin-top-mm 18 > ruled.pdf
```

`--profile` must precede edit flags. The `atrament.paper.v1` profile uses
integer micrometers, has a 4-KiB input ceiling, rejects duplicate or unknown
fields, and is validated against the actual PDF path before acceptance.
The shell chooses the output file and handles any overwrite; this
paper-only CLI is separate from the opt-in localhost notebook/handwriting
profile Open/Save feature above.

### Shared native paper and handwriting profile CLI

`atrament-profiles` is the first combined **Rust-only profile CLI**. It
reuses `atrament-paper` for physical paper/PDF options, and the bounded Rust
handwriting-statistics codec for separate `atrament.handwriting.statistics.v1`
files. The paper schema is still paper-only, not a complete notebook schema.

```sh
cargo run -q -p atrament_profile_editor_cli -- \
  handwriting --seed 7 --output handwriting.json
cargo run -q -p atrament_profile_editor_cli -- \
  handwriting --profile handwriting.json \
  --set-global height 2500 3500 3000 3000 200 \
  --set-glyph 'ñ' width 1800 2600 2200 2200 200 \
  --output handwriting.json --replace
cargo run -q -p atrament_profile_editor_cli -- \
  handwriting --profile handwriting.json \
  --remove-glyph-dimension 'ñ' width \
  --output handwriting.json --replace
cargo run -q -p atrament_profile_editor_cli -- \
  handwriting --profile handwriting.json --inspect
cargo run -q -p atrament_profile_editor_cli -- \
  handwriting --profile handwriting.json \
  --inventory-text '¡Hola, señor! ¿Qué tal? «Sí» ñ ∞'
cargo run -q -p atrament_profile_editor_cli -- \
  handwriting --profile handwriting.json --preview-glyph 'ñ' 7
cargo run -q -p atrament_profile_editor_cli -- \
  paper --style ruled --emit-profile --output paper.json
cargo run -q -p atrament_profile_editor_cli -- \
  paper --profile paper.json --spacing-mm 6 --output sheet.pdf
```

Each statistical edit requires the dimension name followed by **minimum,
maximum, mean, median, and standard deviation** in that dimension's native
units: micrometres, millidegrees, or parts per thousand. Ten named global
variables and exact per-grapheme overrides, including decomposed accents,
are supported.

`--remove-glyph GRAPHEME` removes an entire exact glyph,
`--remove-glyph-dimension GRAPHEME DIM` restores that dimension to its
global value, and removing the last override deletes the empty glyph.
Missing glyphs or dimensions refuse before saving.

The Rust CLI `--inventory-text` audit checks each distinct Unicode extended
grapheme against the existing 114-entry English/Spanish **text** inventory,
reports whether a matching exact statistical override exists, and lists
unlisted graphemes explicitly. NFC `ñ` and NFD `ñ` remain separate;
mathematical symbols such as `∞` are reported outside this *text*
inventory because mathematical coverage is a separate authority. The audit
allows 1–4096 UTF-8 bytes, refuses control characters and output-file flags,
and writes only to stdout. **Inventory membership does not provide glyph
outlines, fallback consent, or rendered handwriting.**

The read-only `--preview-glyph GRAPHEME OCCURRENCE` CLI command produces
a reproducible integer preview for each of ten admitted dimensions using
the stored seed, exact grapheme identity, and occurrence index. It applies
global controls and per-grapheme overrides without saving or rendering.
This is intentionally an **uncalibrated numeric estimate**: the configured
mean, deviation, and limits affect it, but the median, exact distribution
moments, and ink shapes are **not** reproduced. Use it to compare edits,
not to claim a finished handwriting style.

The default profile is **not calibrated handwriting**; its seed is recorded,
but no stochastic stroke generator is connected yet.

`--profile` must come before statistical edits. Input is limited to 1 MiB,
unknown/duplicate fields or impossible statistics refuse, and output is
canonical JSON. `--output` refuses to overwrite an existing file unless
`--replace` is explicit; existing symlinks cannot be overwritten.

For example, `--set-global height 2500 3500 4500 3000 200` is invalid and
refuses before any output file change. The initial localhost statistics
editor now accepts bounded global and per-grapheme changes. Notebook
content profiles, actual handwriting rendering, and server-side Open/Save
web controls remain open implementation tasks.

### Native notebook page metadata profile (first Rust slice)

The `atrament-profiles notebook` subcommand creates a **versioned
`atrament.notebook.v1` profile** containing the existing Rust-validated
`atrament.paper.v1` sheet plus date placeholder style and location, border
radius in micrometres, optional printed numeric date, page numbers, and
a printed-sheet flag. This is page metadata, **not** notebook body content,
searchable date text, or generated handwriting. Existing seven-field v1
files reopen with an empty printed-date field.

```sh
cargo run -q -p atrament_profile_editor_cli -- \
  paper --style ruled --spacing-mm 7 --margin-top-mm 25 \
  --emit-profile --output my-paper.json
cargo run -q -p atrament_profile_editor_cli -- \
  notebook --paper-profile my-paper.json --date-style box \
  --date-placement top-right --date-text 2026-11-03 \
  --border-radius-um 3000 \
  --page-numbers true --printed-sheet false --output notebook.json
cargo run -q -p atrament_profile_editor_cli -- \
  notebook --profile notebook.json --page-numbers false \
  --output notebook.json --replace
cargo run -q -p atrament_profile_editor_cli -- \
  notebook --profile notebook.json --inspect
```

Notebook input is bounded at 8 KiB. The nested paper and notebook fields
are validated by Rust. Unknown or duplicate fields, invalid dates,
impossible paper geometry or oversized borders refuse. `--profile`
must precede edits.

`--output` refuses existing files unless `--replace` is explicit, and never
overwrites symlinks. Invalid inputs cannot change the output.

A saved notebook profile can also export its **paper layer** through the
shared Rust PDF or PNG renderer, without a separate paper JSON file:

```sh
cargo run -q -p atrament_profile_editor_cli -- \
  notebook --profile notebook.json --date-style box \
  --date-placement top-right --export-paper pdf --pages 3 \
  --output notebook-paper.pdf
cargo run -q -p atrament_profile_editor_cli -- \
  notebook --profile notebook.json --page-numbers true \
  --export-paper png --dpi 150 --png-page 10 \
  --output notebook-page-10.png
```


The same Rust-only CLI can draw **explicit caller-owned pen segments** over
paper, printed dates, and page numbers. A separate `--ink-line` is repeated
for each straight segment, with `PAGE X1_UM Y1_UM X2_UM Y2_UM WIDTH_UM
'#RRGGBB'` (one-based page number, physical micrometres, ink color). For
example, the following draws red diagonal ink *only on page 2* of a two-page
A4 PDF, then emits its transparent RGBA page-2 PNG:

```sh
cargo run -q -p atrament_profile_editor_cli -- \
  notebook --printed-sheet true --export-paper pdf --pages 2 \
  --ink-line 2 25400 25400 50800 50800 700 '#DE1144' \
  --output notebook-ink.pdf
cargo run -q -p atrament_profile_editor_cli -- \
  notebook --printed-sheet true --export-paper png --png-page 2 --dpi 72 \
  --ink-line 2 25400 25400 50800 50800 700 '#DE1144' \
  --output notebook-ink-page-2.png
```

The compositor refuses zero-length strokes, nonprintable endpoints, line
width outside 80–1500 µm, unrequested PDF page numbers, and more than 512
segments. Selected PNG pages render only their page's ink; invalid off-page
geometry still refuses.

These are **manually specified segments**, not
Unicode glyphs, calibrated handwriting, fonts, or image overlays.
`atrament.notebook.v1` still saves page metadata alone; `--ink-line` marks
are not stored in that profile or editable in the localhost web UI yet.
The full notebook-content rendering task therefore remains open.

PDF allows 1–32 physical pages; transparent RGBA PNG renders one page at
72–300 DPI with physical resolution metadata. For `printed_sheet: true`,
existing ruling is suppressed to avoid duplicating marks on stationery.
When no date or page numbers are selected, preprinted PDF/PNG overlays are
unmarked.

No profile file is changed by rendering. Output is explicit and existing paths
require `--replace`; invalid options refuse before changing output bytes.

When a notebook selects a date placeholder (`line` or `box` at the top left
or top right), the same Rust PDF/PNG renderer draws a 36 × 8 mm reserved
area in the top margin. The optional `--date-text` (or `date_text` field in
the localhost notebook editor) prints fixed-width numerals and separators
inside that area, without synthesizing handwriting. Accepted examples are
`2026-11-03`, `03/11/2026`, `11/03/2026`, and `03.11.2026`.

An ambiguous day/month order is preserved exactly as typed, never guessed
from locale. Impossible Gregorian dates, malformed separators, or dates
without a selected placeholder refuse. Existing empty placeholders stay
empty. The reserved area requires at least 10 mm of top clearance and
36 mm of usable width; invalid placements refuse profile validation.

With `page_numbers: true`, Rust prints page numbers 1–32 in the bottom
margin using compact block numerals; each PDF page receives its own number.
A PNG export can select page 1–32 with `--png-page N` (defaults to page 1).
The PNG exporter emits one physical sheet per invocation, not an image bundle.

Numbering requires at least 7 mm of bottom margin; invalid profiles refuse.
Numbered sheets retain the number and date placeholder when `printed_sheet`
suppresses the existing paper marks.

A nonzero `border_radius_um` draws a 0.12-mm rounded outline along the
printable region in both PDF and transparent PNG; zero leaves the border off.
The radius must fit half the printable area, and each sheet-edge margin must
be at least 0.06 mm to contain the outline's centered stroke.

This is **not** a complete notebook export: only printed numeric date
annotations, paper marks and page numbers render. Handwriting, arbitrary text,
searchable text and image overlays remain unavailable. Date and page-number
glyphs are printed markers, not sampled pen strokes.

The localhost Rust runtime also exposes authenticated
`GET /api/notebook/profile` (default profile) and
`POST /api/notebook/profile` (bounded JSON validation and canonicalization).
These endpoints are in-memory only; POST does not persist a file or set
session-wide defaults. They require the existing session bearer credential
and same-origin admission, like the paper profile endpoint.

The authenticated localhost runtime also accepts notebook JSON at
`POST /api/notebook/paper/pdf` (with one `X-Atrament-Pdf-Pages: 1..32`
header) and `POST /api/notebook/paper/png` (with one
`X-Atrament-Png-Dpi: 72..300` header). PNG optionally accepts a single
`X-Atrament-Png-Page: 1..32` header and defaults to page 1 if omitted;
malformed or repeated page headers refuse before rendering.

Both formats use the same Rust notebook validator and paper serializer as the
CLI. The web editor supplies page/DPI headers from its PDF and PNG controls,
then downloads the selected physical paper layer. One PNG contains one sheet.

Preprinted-sheet suppression applies to ruling in both formats; explicitly
selected date placeholders, optional printed numeric dates, rounded outlines
and page numbers remain visible. Handwriting, arbitrary text and image
content are still not drawn in these outputs.

The localhost web editor now also offers **Notebook page profile** controls
for date placeholder style and placement, optional printed date text,
border radius, page numbers and preprinted-sheet intent. PDF/PNG exports draw
these annotations only when physically admissible.

**Use current paper** includes unsaved sheet settings before Rust validation.
**Preview selected page**
validates the current notebook proposal with Rust and shows the requested
PNG paper layer inline, including rounded borders, without downloading it.
Changing page or notebook settings hides stale previews; ending the session
revokes its image URL.

**Edit notebook paper** sends the notebook's embedded sheet into the visual
paper editor through Rust validation; **Use current paper** copies edited
settings back only when requested. Load and Save JSON use explicitly
selected files; no browser state is silently persisted or unsupported
metadata rendered beyond these annotations.

## The workspace

The following is the intended notebook workflow, not a claim that the
existing localhost shell already implements the complete editor. The design
uses a split workspace with two editors:

- **LLM editor:** task, structured notebook source, validation diagnostics, and
  the one-shot chat exchange.
- **Human editor:** live page preview, text correction, selection, movement,
  resize, crop, layering, and physical-page controls.

The first LLM workflow requires no embedded model account:

1. Paste the task, notes, formulas, sources, and optional images.
1. Select a handwriting profile, paper, style, and output mode.
1. Press the permanent **Copy prompt** button.
1. Paste that self-contained prompt once into the chosen chat.
1. Paste the structured response back into the LLM editor.
1. Resolve validation, source, glyph, collision, and overflow diagnostics.
1. Adjust the accepted page in the human editor.
1. Export PDF or compile a live writing plan.

The copied prompt includes the complete backend-owned return format, page
constraints, source rules, style vocabulary, and return envelope. It never
depends on hidden prior chat context.

After a notebook is accepted, the first-release design specifies a semantic
command mode. Instead of regenerating the notebook, an LLM returns a versioned
batch of typed operations against stable notebook identities and a specific base
revision. The backend contract validates the complete batch atomically, applies
only accepted semantic effects, and recomputes derived regions invalidated by
those effects and their dependencies.

The same command model is designed for clipboard-assisted chat, CLI, and MCP.
Clipboard responses remain untrusted until backend validation and review; the
frozen MCP contract projects the same validated application transaction without
requiring a browser click.

That MCP projection is designed to automate notebook inspection, bounded edits,
diagnostics, rendering, caller-authorized export, and device-neutral plan
compilation end to end. Typed results and stable blocking diagnostics provide
stop conditions instead of an unbounded "retry until success" loop.

Export still requires explicit caller or admitted host-policy path and overwrite
intent; paths found only in notebook or model text remain data. Physical machine
arming and start remain separate explicit safety operations rather than side
effects of a generic LLM edit.

### Targeted command mode

For a bounded edit to an accepted notebook, the backend can generate a command
prompt instead of a full-notebook formatting prompt. That request identifies the
base revision, readable context, writable semantic targets or insertion anchors,
and admitted command families.

The user copies it with the same **Copy prompt** control, pastes it into an
external chat, and pastes the returned command envelope into the same **Model
response** surface. The backend validates the complete batch, shows the semantic
diff and dependency-expanded impact, then applies one accepted transaction.

Unrelated semantic identities remain unchanged. Derived page regions may still
recompute when the changed target affects their dependencies, such as text
reflow moving later content.

With MCP, the clipboard round trip disappears: an agent can inspect the current
revision, validate or apply the same command batch, inspect its receipt and
diagnostics, then explicitly request rendering, export, or planning through the
same application core when those outputs belong to the caller's goal.

## What a notebook can contain

The initial semantic vocabulary covers the material expected in serious school
notes and assignments:

- headings, dates, paragraphs, quotations, lists, and definitions;
- citations, source notes, footnotes, and unresolved claims;
- inline and displayed mathematics, aligned work, matrices, and units;
- tables, boxes, dividers, arrows, labels, and page references;
- photographs, transparent line art, diagrams, and clipped regions;
- highlights, callouts, margin notes, and digital paper notes;
- multiple handwriting roles for body text, titles, labels, and annotations.

An LLM may organize and format content, but a plausible citation is not proof.
Atrament distinguishes material supplied by the user, derived material, cited
claims, and unresolved claims so the human can review them before export.

## Page geometry

Every page uses physical units. The user controls:

- sheet size, orientation, and printable region;
- blank, ruled, dotted, squared, or custom paper;
- grid or rule spacing and color;
- binding edge, outer margin, and top clearance;
- inner writing inset between the margin and content;
- border shape, corner roundness, and layer order;
- measured ruler error and controlled grid imperfection;
- default body, title, caption, and annotation sizes.

Flowing content moves to another page. Fixed content that crosses the writable
region produces a blocking diagnostic; nothing silently vanishes outside the
sheet.

## Images and drawings

Clipboard and file intake initially support PNG, JPEG, and WebP. An image can be
placed behind text, inline, above text, or inside a clipped region and exposes
position, size, crop, opacity, and z-order.

For simple educational drawings, Atrament can derive transparent single-color
line art with configurable levels. The intended result is readable black ink
that can be colored by hand later, not a muddy grayscale photograph.

## Two honest output modes

| Capability | Digital / PDF | Live / pen machine |
| --- | --- | --- |
| Ink | Multiple simulated colors | One calibrated physical pen |
| Titles | Decorative and layered | Sober, single-pen geometry |
| Highlights | Marker and color layers | Underline, box, spacing, or weight |
| Photographs | Color, crop, texture | Converted transparent line art |
| Paper notes | Fill, fold, and shadow | Unsupported |
| Page | Any configured printable page | One blank, flat, calibrated sheet |
| Drawing | Raster, vector, or line art | Single-color vector paths |
| Output | PDF and print | Validated motion plan |

Unsupported live objects are reported individually. Atrament never drops a
color, photograph, shadow, sticky note, or tool change merely to make a motion
plan compile.

A future compact, quiet, multi-tool writer may add physical colors and pen
changes through a new capability profile. It is not part of the initial live
contract.

## Visual language

The target is organized handwriting, not random decoration. Reference notebooks
contribute a shared grammar:

- one dominant title and unmistakable section hierarchy;
- short columns and bounded modules for dense material;
- tables and rules that are measured but not machine-perfect;
- highlights used for hierarchy rather than indiscriminate color;
- compact line diagrams, labels, arrows, and deliberate whitespace;
- handwriting and ruler variation that remains controlled and legible.

Digital themes may be colorful, layered, and playful. Live themes preserve the
same hierarchy with one pen through scale, spacing, boxes, underlines, and
line-art diagrams.

## Handwriting model

A portable `.atrament` profile records the writer's calibration evidence,
stroke vocabulary, contextual forms, joins, pen lifts, dimensions, variation
bounds, paper relationship, and pen or ink presets.

The renderer does not stamp a font repeatedly. It plans continuous strokes and
uses correlated, seeded variation across page, line, word, character, and
stroke scales. Controls include height, width, slant, roundness, flattening,
baseline drift, spacing, speed, pressure proxy, and alternate forms.

English and Spanish are the first complete language baseline. Profiles cover
their letters and diacritics, opening punctuation, curly quotations,
apostrophes, ellipsis, en and em dashes, and the admitted mathematical symbols.
Missing coverage is visible and blocking unless the user accepts a declared
fallback.

### Current local profile utilities (Linux)

The present `atrament` binary offers four explicit local profile operations:

```sh
atrament profile verify SOURCE.atrament
atrament profile check-canonical SOURCE.atrament
atrament profile inspect SOURCE.atrament
atrament profile repack SOURCE.atrament NEW-TARGET.atrament
atrament profile repack --replace SOURCE.atrament EXISTING-TARGET.atrament
```

Verification checks a bounded archive, manifest, member digests, and
JSON-section syntax, then reports only a fixed success message, without
revealing profile content. The read-only `check-canonical` operation performs
those checks and compares all original ZIP bytes with the deterministic
first-release encoding. Valid but noncanonical archives refuse without stdout,
with fixed messages distinguishing manifest JSON from ZIP framing; it does not
rewrite the source or display any private profile data. Inspection
performs the same integrity checks before displaying the manifest and readable
sections.

Repack performs those same checks, then writes canonical ZIP bytes to a
caller-selected destination using atomic, synchronized file creation. By
default, repack never overwrites an existing target. The explicit `--replace`
flag authorizes atomic replacement of an existing regular target (including
the selected source itself), or creation of an absent target, but never follows
symlinks.

Bare dash-prefixed tokens are treated as options in every profile command,
not source or output filenames. Prefix `./` to select a file such as
`./--replace`.

Canonical repack preserves verified opaque assets and unknown
optional features. Unsupported required features and malformed sections refuse
without changing an existing target.

These are independent command-line utilities; none accepts typed handwriting
sections into the active workspace, creates a session Export receipt, or
provides a notebook/PDF Export path.

## Rendering

Rust produces authoritative geometry on the CPU:

1. semantic blocks compile into physical layout boxes;
1. handwriting compiles into stroke centerlines and ink contours;
1. equations, tables, rules, and diagrams remain vector geometry;
1. ink, edge, deposition, paper, and highlight textures form separate layers;
1. bounded soft noise unifies the page without moving geometry;
1. PDF, preview, and live motion consume the appropriate shared authorities.

The same accepted document, profile, paper, assets, material presets, and seed
produce the same layout and stroke plans. No CUDA, WebGPU, or discrete compute
GPU is required.

## Architecture

```mermaid
flowchart LR
    UI[TypeScript browser] -->|loopback| CORE[Rust backend]
    CLI[CLI and MCP] --> CORE
    CORE --> PDF[PDF and images]
    CORE --> MEDIA[Transcription adapter]
    CORE --> HW[Pen-machine adapters]
```

- **Rust backend:** document model, physical units, layout, handwriting,
  deterministic CPU rendering, diagnostics, PDF, CLI, MCP, and motion plans.
- **TypeScript frontend:** localhost interaction, clipboard, split editors,
  direct manipulation, preview presentation, and accessibility. It has no
  runtime package dependency or independent domain-validation layer.
- **Hexagonal boundaries:** the browser and agents are inbound adapters;
  codecs, transcription, files, and physical writers are outbound adapters.
- **Ephemeral session:** no account, cloud sync, database, or autosave. Only an
  explicit import or export persists beyond the current localhost session.

The frontend never becomes a second layout engine, and hardware adapters never
become notebook authorities.

## Hardware compatibility

Compatibility is capability- and model-specific. Atrament distinguishes:

1. vector file export for vendor software;
1. a managed vendor CLI adapter;
1. a documented command-protocol adapter;
1. a direct device adapter with status and recovery.

Initial research baselines include:

- [NextDraw CLI](https://bantam.tools/nd_cli/) for SVG plotting, preview,
  configuration, pause, and resume control;
- [AxiDraw's SVG and interactive API](https://axidraw.com/doc/py_api/),
  including pen motion, preview, pause, and resume behavior;
- documented Graphtec devices with a plotting pen and HP-GL or GP-GL command
  modes, such as the [FC9000](https://graphtecamerica.com/fc9000/).

None is called supported until its exact model, firmware, transport, paper,
pen, usable area, homing, interruption, and safe-stop behavior pass physical
acceptance.

## Repository map

- [`TODO.md`](TODO.md) is the unfinished delivery sequence.
- [`docs/adr/index.yml`](docs/adr/index.yml) catalogs durable decisions.
- [`.jig/jig.toml`](.jig/jig.toml) anchors repository governance.
- [`docs/technical/index.yml`](docs/technical/index.yml) reserves measured
  implementation evidence and contracts.

## Responsible use

Use handwriting profiles created by, or with the explicit authorization of,
the represented writer. Atrament supports personal notes, accessibility,
authorized document production, and physical pen workflows; it does not make a
claim about who physically wrote an exported page.

## License

Atrament is licensed under the [MIT License](LICENSE-MIT).
