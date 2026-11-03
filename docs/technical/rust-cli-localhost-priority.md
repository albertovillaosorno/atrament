# Rust-first notebook implementation roadmap

**Owner decision, November 2, 2026:** re-prioritize, do not delete the
complex designs. Atrament is a personal, local notebook and handwriting
application. The immediate executable target is a **Rust CLI plus a localhost
web editor** that loads, changes, previews, and saves notebook and handwriting
profiles, then exports printable PDF and transparent PNG pages.

**Glyph-first vision:** Start with exact, inspectable geometric letterforms,
not random strokes or an opaque handwriting font. A simple lowercase **o** is
an analytic circle; a single-storey **a** reuses that loop with a stem, while
the two-storey printed **a** is a separate selectable form.

Italic/cursive forms and joined writing must also be supported. Accents
(especially vowels), Spanish ñ/ü and other marks use measured placement,
not guessed pixels.

Each glyph exposes individual strokes, their order and direction, including
explicit reversals. A user can redraw, measure and preview every stroke until
it matches their own handwriting. Width, height, slant, thickness, baseline,
spacing, regularity, texture and context variation act **on top of** the
canonical geometry; repeatable settings must visibly change the final ink.

**Notebook workflow:** Paste plain text or Markdown, lay headings and body
text onto the selected physical pages, paginate without silent clipping, and
export PDF/transparent PNG. Headings may use decorative lettering, marker
strokes and highlighter treatments with independently editable colors and
thicknesses. The local editor and Rust CLI share the same saved profiles.

## Architecture and ownership

1. **Rust is the source of truth.** Reuse the existing Rust notebook, physical
   page, handwriting variation, grapheme, profile-container, and PDF modules.
   Add only the missing application adapters. All persisted data must pass
   Rust admission before a successful application save or export is claimed.
2. **The CLI and localhost UI use the same core.** The browser contains input
   controls, accessible sliders, numerical entry, and preview presentation;
   it must not independently define layout, measurement, statistical sampling,
   glyph coverage, or serialization semantics. Use the existing loopback-only
   Rust transport rather than building another server.
3. **Profiles are separate, versioned resources:** notebook page/profile and
   handwriting/glyph profile. A notebook can choose a handwriting profile, and
   local files can be loaded or explicitly saved without cloud accounts.
   Revisions, immutable source on refusal, and explicit overwrite are required.
4. **No parallel Python notebook implementation.** The accidental notebook
   Python launcher, renderer, sample data, and Python-only tests were removed.
   Build the functionality within existing Rust domains and the native CLI.
   Do not recreate a separate renderer as an expedient shortcut.
5. **No GUI toolkit or desktop packaging dependency.** The browser is opened
   against localhost; the CLI remains fully usable without a browser.
   Windows/macOS portability is not a release gate for personal local use.

## Immediate milestones, in dependency order

### M1. Shared Rust profiles and native CLI

- One canonical schema for notebook and handwriting profiles, with explicit
  version, physical units, and defaults. Validate existing paper styles,
  sheet dimensions, margins, date boxes, page numbering, existing printed
  sheets, image overlays, and every current handwriting distribution variable.
- CLI can load, inspect, edit, save, and refuse corrupt notebook or handwriting
  profiles using Rust. A real filename is chosen by the caller, not page
  content. Existing `.atrament` archive verify/inspect/repack commands are
  operational but do not yet provide these typed edits.
- One accepted and one rejected file fixture prove the CLI and domain return
  the same validation outcome. No exhaustive third-party proof is required.

### M2. Localhost profile editor

- Open the existing Rust local server and see a separate **Notebook** and
  **Handwriting** profile editor with Open, Save, and Save As actions.
- All persisted scalar controls have visible numeric input and an adjustable
  slider within domain bounds; enums use selects, colors use color inputs,
  booleans use checkboxes, and complex glyphs/images use a file chooser.
- Editing updates the Rust model and a correctly labeled preview, or returns
  a typed refusal. An invalid change never silently alters the saved profile.
- A user can close and reopen the UI, load the saved files, and recover the
  same values. Browser storage must not be the only copy of private profiles.

### M3. Handwriting fidelity and Spanish/English glyph coverage

- Every existing variation variable is represented: height, width, baseline,
  slant/rotation, spacing, roundness, flattening, pressure/ink thickness,
  texture, and available role/context/correlation controls. When a variable
  exists only as theory, label it unavailable until Rust actually samples it.
- The first required glyph inventory is the union of the existing
  `english-spanish-grapheme-inventory` and the admitted mathematical symbol
  vocabulary. Cover uppercase/lowercase Latin, numerals, accents and
  decomposed equivalents, ñ/Ñ, ü/Ü, ¿/¡, Spanish guillemets « », English
  quotation marks “ ” ‘ ’, basic punctuation, dashes, ellipses, and school
  mathematics/currency symbols used in the documented fixture.
- Mexico, Spain, and English share this initial orthographic baseline; locale
  differences in dates, quotation preference, and notation are profile
  choices, not separate hard-coded alphabets. This is **not** a claim that
  every Unicode symbol is implemented. Unsupported grapheme clusters must be
  identified explicitly and permit a manually supplied glyph/shape fallback.
- Each glyph starts from an exact canonical geometric skeleton. A lowercase
  **o** uses a circle primitive with an exact center and radius; a simple
  **a** composes that loop and a stem. A separate two-storey printed **a**,
  italic and cursive/connected forms are explicit selectable variants.
- Each glyph has authored pen strokes with editable start/end geometry,
  stroke order, direction, and safe reversal. Diacritic strokes may be
  separate, reusable marks with adjustable anchors and clearance; cover
  á/é/í/ó/ú as well as ñ and ü, without restricting all marks to vowels.
- Each variation control must change an identifiable rendered property:
  width, height, slant, thickness, baseline, spacing, regularity, texture,
  and seeded contextual differences. Keep the ideal skeleton intact for
  comparison and expose a reproducible before/after preview; do not claim
  a distribution is rendered if it is only stored in a profile.
- Each glyph can be tuned independently, including custom stroke/image data,
  distribution bounds, and a reproducible sample preview. A reference photo
  can be used as a **manual** measuring aid, not mandatory OCR.

### M4. Export and physical notebook workflow

- Paste or load text/Markdown with paragraphs, headings and lists. Apply
  measured wrapping and automatic page breaks to the configured paper profile
  before export, with explicit overflow/refusal instead of hidden clipping.
- Titles can use independently adjustable decorative strokes, marker-style
  ink and highlighter layers; printing on existing ruled paper must not
  accidentally duplicate the background.
- Rust exports a multi-page document with true page dimensions, the selected
  paper profile, handwriting profile, dates, image layers, and page numbers.
- PDF prints at actual size; PNG can omit the paper image and carry physical
  DPI metadata for transparent overlays. Mark already-printed physical sheets
  so the compositor does not duplicate background/rules.
- An end-to-end example starts from saved profile files, edits one glyph,
  produces one PDF and one transparent PNG, and reopens both outputs. No
  plotter, robot, Windows/macOS installer, cloud service, or AI account is
  required to finish this milestone.

## Preserved advanced work (not discarded)

The historical P1–P9 `.mdc` records are **completed theoretical or partial
artifacts**, not proof of a working product. They remain in
`docs/todo/completed/` with their original identities and evidence. Their
advanced subjects—calibration, extended mathematics, complex page solvers,
semantic editing, LLM/MCP, further theme effects, animation,
accessibility, paper realism, plotters, and device safety—remain available as
future extensions, but none may block M1–M4 unless it is actually required by
an identified dependency. Physical hardware remains theory-only.

## Decision and stop rules

Each **new implementation TODO** must have a finite working outcome, an
observable acceptance example, and an honest failure result. The owner may
accept a complete theoretical design when execution is unavailable, but that
only closes its *design artifact*: the dependent executable feature stays open
until usable. Never equate a completed historical record with implemented
functionality. Keep this roadmap updated when a new executable feature is
admitted; do not silently grow the critical path.
