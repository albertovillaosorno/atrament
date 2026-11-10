# Executable handwriting-stroke projection (Rust)

## Status

Implemented and used by native notebook exports, not a prospective API. The
geometry engine requires caller-authored glyph shapes and does not synthesize
a learned writer's letterforms. Full notebook generation is unfinished.

## Purpose

Atrament's executable handwriting function is the projection of **physical
pen strokes**. PDF and PNG consume the same admitted centerline geometry, not
two separate scripts for drawing letters. The localhost editor is a client;
Rust owns accepted geometry and export semantics.

## Scope

The independent crate `atrament_handwriting_stroke_projection` owns exact
local-to-page projection and optional directed joins. It is side-effect-free,
uses integer micrometres and does not import PDF/PNG codecs or filesystem I/O.
The CLI continues to own Unicode segmentation, exact source glyph admission,
seeded creative variation, selected mean statistics and the physical stroke
count budget. Actual PDF and PNG writers are downstream adapters.

## Contract

```text
Authored Unicode grapheme library (exact v1 TSV)
    |
    v
profile-editor-cli/composition/glyph.rs
  - exact grapheme selection; refuses missing shapes
  - optional seeded local variation and statistical means
    |
    v
handwriting-stroke-projection/domain/lib.rs
  project_ink_segment(...)
    - whole-micrometre scaling and lower-baseline shear
    - width, RGB and representable-coordinate admission
  connect_ink_segments(...)
    - actual directed endpoints after all transforms
    - same page / RGB / width and checked squared distance
    |
    v
profile-editor-cli/composition/glyph.rs
  - preserves original library and exact order
  - bounds total pen segments and optional bridges
    |
    v
notebook-paper-cli (shared physical ink)
    +-- native vector PDF
    +-- native raster PNG
```

The compiled Cargo dependency is
`atrament_profile_editor_cli -> atrament_handwriting_stroke_projection`.
The production `GlyphOverlay::expand` executes the CLI `placed_ink` adapter
for **every** authored stroke; `append_text_ink` delegates explicit adjacent
joins to the same core. The requested export format does not change either
function's physical output.

## Failure Modes

The core refuses a zero page index, zero-width/height mean cells, negative
or unrepresentable coordinates, nonfinite shear, pen widths outside
80–1500 µm, source or scaled zero-length segments,
slant angles outside the open (-90°, 90°) interval (including horizontal
baseline-only contours where an invalid tangent would otherwise be hidden),
cross-page connections, mismatched pen color or width, and gaps beyond the
supplied 1200 µm production limit. Join distance uses checked `u128`
arithmetic, including extreme `u32` coordinates.
It does not clip or mutate original shapes. The physical export adapter has
its own additional page-boundary and stroke-margin checks.

Neither the engine nor the current exporter claims OCR, handwriting learning,
automatic Unicode-wide shape generation, a physical plotter controller or
fitted pen-contact response. Those remain separate research/future contracts.

## Verification

- `tests/backend/handwriting-stroke-projection/domain/lib.rs`: exact mean,
  shear, width, color, rounding-collapse refusal, overflow, join and replay.
- `tests/backend/profile-editor-cli/composition/lib.rs`: actual Rust CLI
  regression tests for profile means, glyph placement and PDF/PNG output.
- `tests/backend/profile-editor-cli/composition/joined-varied-handwriting.mjs`:
  subprocess acceptance with real accented ink, variation and joined paths.
- `tests/backend/profile-editor-cli/composition/main.mjs`: exact native file
  read/write and PDF/PNG outputs, including no-clobber refusal.

```sh
cargo test --locked --offline -p atrament_handwriting_stroke_projection
cargo test --locked --offline -p atrament_profile_editor_cli
node --test \
  tests/backend/profile-editor-cli/composition/joined-varied-handwriting.mjs
```
