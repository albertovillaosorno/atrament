# PDF, print, and composition validation

## Status

Verified against the current output domains and an isolated vector-only PDF
byte serializer. Full notebook-to-PDF Export remains unimplemented.

## Purpose

This guide maps the accepted PDF, print, and photo composition ADR to the
first-release executable boundaries that exist today. It describes what Atrament
can already preserve and inspect before PDF serialization, which reproducibility
inputs are retained, which physical round-trip measurements are typed, and
which output responsibilities remain deliberately unimplemented.

The authoritative product decision remains
`docs/adr/output/pdf-print-and-photo-composition.md`. This guide records current
implementation evidence rather than creating a new wire format or PDF policy.

## Scope

The current executable composition boundary spans these pure backend domains:

- `src/backend/vector-geometry/domain/lib.rs` for ordered vector primitives,
  physical page bounds, and semantic provenance;
- `src/backend/pdf-composition-plan/domain/lib.rs` for ordered PDF page intent,
  page boxes, color intent, searchable-text disposition, asset disposition, and
  render-manifest linkage;
- `src/backend/render-manifest/domain/lib.rs` for reproducibility inputs tied to
  one render result;
- `src/backend/render-quality-profile/domain/lib.rs` for the invariant that
  preview and final quality profiles share geometry, seed, blend order, and
  physical bounds;
- `src/backend/print-scan-measurement/domain/lib.rs` for scale, clipping, grid
  registration, and photo-placement observations; and
- `src/backend/render-regression-evidence/domain/lib.rs` for independent
  regression-axis completeness before a visual report is considered complete.

These transport-neutral values do not emit PDF bytes, choose a PDF library,
write files, rasterize pages, embed fonts, compute hashes, or decide physical
pass/fail tolerances. A separate vector-only PDF application adapter now emits
bytes for fully resolved, supported paths, without changing these domains.

## Contract

### Vector authority remains primary

`VectorGeometryPage` retains caller-owned physical page bounds and ordered
`VectorPrimitive` values. Every primitive carries both a typed primitive family
and semantic provenance. The accepted primitive families include layout boxes,
handwriting centerlines, expanded ink contours, equations, rules, tables, and
diagram paths.

The vector domain does not generate or tessellate geometry. Its role is to keep
topology and semantic ownership inspectable so preview, PDF, and physical
planning do not need to infer authority from rendered pixels.

### PDF composition is pre-serialization intent

`PdfCompositionPlan` retains pages in final document order, accepted asset
resources in caller-owned deterministic order, and the identity of the render
manifest associated with the composition.

Each `PdfPagePlan` retains:

- exact caller-owned physical page-box authority;
- caller-owned color intent;
- authoritative vector page projection; and
- an explicit searchable-semantic-text disposition.

Searchable text is either `Preserved(...)` or `OmittedAsIncompatible`. Omission
is therefore an explicit compatibility result rather than an implicit side
effect of PDF generation.

Each accepted asset resource similarly records either `Embedded` or
`SafelyReferenced`. An 18,432-state exhaustive oracle crosses every order of
four pages and three resources with every searchable-text and asset-disposition
mask, requiring the exact caller-owned page, resource, and manifest authority to
remain
unchanged. The current domain does not define how a safe external reference is
represented or whether one is admitted for a particular export; that belongs to
future resource and PDF adapters.

### Isolated vector-only PDF byte serialization

`src/backend/pdf-vector-serializer/application/lib.rs` is a narrow, executable
PDF 1.4 serializer for caller-resolved geometry (not a notebook converter).
It consumes concrete `PdfCompositionPlan` page authority with positive
physical sizes in micrometers, one explicit `DeviceRGB` color intent, and
ordered stroke/fill vector paths with an explicit nonzero or even-odd fill
rule for compound contours. Its output is deterministic in-memory bytes
with real page boxes, cubic paths, PDF object offsets, and a checked ASCII
render-manifest identity in the Info dictionary.

Top-left physical coordinates
map to PDF points by an explicit page transform. Bounds include the round
stroke's half-width, so an otherwise in-page centerline cannot clip silently.

The serializer checks both the physical half-width and the *serialized*
three-decimal-point coordinates and stroke width. Independent rounding of
these PDF numbers could otherwise leave a nominally admissible 8 µm stroke
half a thousandth of a point outside the sheet edge. The exact fixed-point
boundary check refuses such paths as `InvalidPath`, preserving nearby strokes
with another micrometer of clearance and leaving admitted PDF bytes unchanged.

The adapter refuses *every* accepted asset and preserved searchable text:
image/color-profile embedding and font/semantic-text rendering do not exist
here. It also refuses malformed paths, invalid page boxes, unsafe manifest
identities, and caller-bounded page/path/command/byte overruns. The output
contains no inferred notebook layout, complete manifest, file path,
filesystem effect, or user-authorized Export operation. It is not evidence
that a pasted assignment can be turned into a PDF by the current UI/CLI.

Each closed filled contour must have a non-collinear geometric witness among
its endpoints and cubic control points. A closed line segment, an all-collinear
polygon, or an all-collinear cubic refuses as `InvalidPath` rather than
silently producing invisible fill ink. One degenerate member refuses an
entire compound fill, even when other contours are valid.

This check is necessary, not a full visible-area proof. Legitimate curved
loops and self-crossing contours remain admitted without using a signed-area
heuristic, which would incorrectly discard some filled self-intersections.
The same collinearity check does not apply to stroked lines.

Each stroked subpath instead needs nonzero physical extent: a lone line whose
endpoints coincide, a cubic whose start, controls and end all coincide, or a
second collapsed subpath refuses as `InvalidPath`. Every individually collapsed
line or cubic also refuses, even inside an otherwise nonempty stroke. PDF round
caps must not silently create a dot from a zero-length authored pen stroke
that the native transparent PNG renderer already refuses.

A genuinely curved cubic loop may start and end at the same point, and
multiple nonempty subpaths remain valid. A harmless repeated fill vertex also
remains valid. The regression checks both refusal and those admitted
counterexamples.

For multi-page output, the PDF assembler now renders and appends one page
at a time, checking the exact current output byte count before rendering the
next page. It no longer retains every prior page content stream while also
building the final document buffer. The serializer still holds a single
rendered page stream until its length is written; it is not a streaming
text/font/image renderer or a general memory cap on caller-owned geometry.

The final byte budget applies to the header, page tree, page objects, content
streams, metadata, cross-reference entries and trailer. A red/green regression
confirmed that the old all-pages-first approach could inspect a later invalid
page even after the final PDF overhead had already exhausted the byte budget.
It now refuses as `ResourceLimit` before that later page is rendered.

The remaining output allowance now bounds each page's rendering, not only
the final append. Once preceding PDF objects consume the exact byte budget,
the assembler refuses before inspecting a later invalid page. A dedicated
red/green test fixes this refusal order without changing valid PDF bytes.
The page transform and closing graphics-state operator are also checked
against that remaining allowance, before an oversized header can defer a
resource refusal behind malformed path commands.

The fixed `Q` graphics-state close is now reserved from the page stream
budget *before* inspecting any path. A malformed path cannot mask an already
unclosable page stream as `InvalidPath` rather than `ResourceLimit`.
Likewise each mandatory path paint operator (`S`, `f`, `f*`) is reserved
before processing its geometry. A path that cannot finish painting now refuses
before a later malformed command can mask the exhausted byte allowance.

The existing oversized-stream regression, large-content fixture and exact-limit
boundary regressions still apply. Output bytes and PDF object order are
unchanged.

A local smoke fixture fed real output bytes to Poppler `pdfinfo` and
`pdftoppm`; both accepted and rendered an A4 page (595.276 by 841.89 points).
The fixture is a synthetic vector path, not a source notebook or visual
fidelity measurement. PDF conformance beyond the supported subset and other
PDF readers remain unverified.

A second Poppler smoke generated two identical-orientation nested rectangles
with the explicit even-odd and nonzero fill rules. At 72 DPI, the hole center
was white for even-odd and colored for nonzero, while the surrounding ink and
page background remained consistent. This verifies the vector-rule difference
with an independent renderer, not handwriting or print fidelity.

A test-only, reproducible two-page smoke can also be generated without
loading any notebook. The Cargo example lives under `tests/backend/`, so the
production application has no fixture module or file-output authority. It
writes PDF bytes to stdout, and redirection is an explicit tester choice:

```sh
cargo +stable run --offline -p atrament_pdf_vector_serializer \
  --example synthetic_vector_pdf --quiet > .temp/pdf-vector-demo.pdf
pdfinfo -f 1 -l 2 -box .temp/pdf-vector-demo.pdf
pdftoppm -f 1 -l 2 -r 72 -png \
  .temp/pdf-vector-demo.pdf .temp/pdf-vector-demo
```

The two pages are A4 with a colored stroke and US Letter with a compound
rectangle containing an even-odd hole. On November 2, 2026, Poppler
`pdfinfo` recognized both MediaBoxes, and `pdftoppm` at 72 DPI showed the
Letter contour pixel `(190, 50, 80)` with a white hole `(255, 255, 255)`.
Two independent runs produced identical 1,086-byte PDF output. This example
does not accept semantic notebooks, images, fonts, or output paths and does
not demonstrate Export.

### Render manifests retain reproducibility inputs

`RenderManifest` groups source, behavior, and appearance inputs without defining
serialization or manifest identity computation.

Source inputs retain accepted semantic-document and revision identities, the
handwriting and paper profile identities, and ordered asset identities. Behavior
inputs retain engine behavior version, ordered model choices, and the accepted
variation seed. Appearance inputs retain material authority, quality profile,
output-affecting render options, and renderer behavior version.

The domain preserves caller-owned order and values exactly. A 2,880-state
exhaustive oracle crosses every order of five asset identities with every order
of four model choices while retaining all scalar source, behavior, and
appearance inputs. It does not normalize models or assets, choose a seed, or
claim that the current Rust struct layout is a persistent manifest schema.

### Preview and final share output authority

`validate_preview_final_pair` rejects preview/final pairs whose shared authority
differs. The shared authority contains vector geometry, deterministic seed,
material blend order, and physical bounds. Only caller-owned quality-cost inputs
such as sampling cost or texture resolution may differ between Preview and
Final.

This enforces the accepted rule that preview and final are quality variants of
one layout/render authority rather than separate layout engines. A
constructor-sealed view retains the exact admitted pair, and an exhaustive
256-mask oracle crosses both role errors with all six shared-authority drift
bits while quality-only costs differ.

### Physical round-trip evidence is typed but uninterpreted

`PrintScanMeasurementKind` explicitly names all eight first-release physical
round-trip measurement families. The accepted output ADR requires four minimum
verification families:

- physical scale;
- clipping;
- grid registration; and
- photo placement.

The P5 product task additionally requires margins, color shift, line weight, and
scanner distortion. Each observation retains its caller-owned source/provenance,
unit, and measured value. `PrintScanEvidence` binds an ordered observation set
to the exact rendered or exported output identity being measured.

Structural admission requires at least one observation from every family in
canonical requirement order and then seals the exact evidence set for later
consumers. Repeated measurements remain valid, so this does not choose an
aggregation or sample-sufficiency policy.

No tolerance or calibration conclusion is inferred from a measurement. Device
selection, correction transforms, statistical treatment, and pass/fail policy
remain separate authorities.

### Visual regression evidence stays independent

A complete `RenderRegressionEvidence` report requires one result for semantic
layout, vector topology, layer composition, render seeds, physical bounds, final
pixels, digital theme, live theme, and overflow overlays. Duplicate or missing
axes reject before completeness can be claimed.

A mismatch remains valid evidence. Complete reports now produce a
constructor-sealed view of the exact caller sequence. A checked-in 512-state
oracle exhausts every match/mismatch combination across the nine axes and
requires direct/sealed canonical mismatch order even when caller evidence is
reversed. The domain does not choose pixel tolerances, topology metrics, release
policy, or golden-artifact storage.

## Failure Modes

The present output domains fail structurally before adapter work when:

- preview/final roles are supplied incorrectly;
- preview and final disagree on shared geometry, seed, blend order, or physical
  bounds;
- render-regression evidence duplicates an axis;
- render-regression evidence omits a required independent axis; or
- print/scan evidence omits a required physical measurement family.

Other important failures are not executable yet because their owning adapters or
policies do not exist. These include malformed PDF objects, font or asset
embedding failure, missing image bytes, image decode failure, unsafe external
resource references, PDF conformance failure, filesystem write failure, color
profile conversion failure, insufficient image resolution, and physical
measurement outside an accepted tolerance.

The current pre-serialization domains must not convert those future failures
into guessed defaults. In particular, they must not silently rasterize vector
authority, downsample an asset, omit semantic text without an explicit
compatibility disposition, or invent a physical acceptance threshold.

## Verification

Current checked-in regression evidence includes:

- `tests/backend/vector-geometry/domain/lib.rs`, which pins primitive taxonomy,
  provenance, page bounds, ordering, exact caller geometry, and all 4,096
  semantic-origin masks over a twelve-primitive ordered page;
- `tests/backend/pdf-composition-plan/domain/lib.rs`, which pins page order,
  physical boxes, vector authority, color intent, searchable-text disposition,
  asset disposition, manifest linkage, and all 18,432 compact order/disposition
  states;
- `tests/backend/pdf-vector-serializer/application/lib.rs`, which pins
  reproducible two-page vector PDF bytes, object/xref offsets, physical A4
  dimensions, material paints and fill-rule selection, explicit resource
  refusal, malformed geometry, clipping and byte-budget boundaries, plus
  the 144-state resource/text/physical-box/path/identity admission matrix;
- `tests/backend/render-manifest/domain/lib.rs`, which pins source, asset,
  model, seed, quality, and version input retention across all 2,880 compact
  asset/model ordering states;
- `tests/backend/pen-contact-model/domain/lib.rs`, which pins calibrated-input
  and observable-output range classification with two exhaustive 343-case
  compact integer oracles;
- `tests/backend/render-quality-profile/domain/lib.rs`, which pins preview/final
  role and shared-authority invariants;
- `tests/backend/render-performance-evidence/domain/lib.rs`, which pins all six
  CPU workload scenarios, all 64 complete-scenario Preview/Final assignments,
  and 256 scenario/quality presence states through direct and sealed admission
  without introducing performance budgets;
- `tests/backend/line-art-extraction/domain/lib.rs`, which pins source identity,
  all six caller-owned extraction controls, transparent-black appearance,
  ordered vector paths, and the complete 128-case control/source drift oracle;
- `tests/backend/digital-paper-note-plan/domain/lib.rs`, which pins fill, fold,
  stacking, soft-shadow intent, caller-provided readable-contrast evidence, and
  sealed/direct parity across the 64-state style/readability sweep;
- `tests/backend/theme-safe-title-plan/domain/lib.rs`, which pins shared title
  identity/hierarchy precedence and sealed/direct parity across all 256 compact
  drift states while allowing media-specific treatments to differ;
- `tests/backend/print-scan-measurement/domain/lib.rs`, which pins all eight
  first-release physical observation families, their provenance, and all 256
  completeness masks through both direct and sealed admission; and
- `tests/backend/render-regression-evidence/domain/lib.rs`, which pins complete,
  independent visual-regression evidence and explicit mismatches.

Before extending this output path, preserve these boundaries:

1. do not serialize a Rust domain value merely because its fields resemble a
   future PDF or manifest schema;
2. keep physical geometry and vector topology authoritative before raster
   projection;
3. make any omission, conversion, downsampling, or external resource decision
   explicit and typed;
4. bind reproducibility evidence to accepted source/version/seed inputs rather
   than adapter-local state; and
5. add real PDF, image, color, filesystem, and physical-measurement failures
   only
   in the adapters or policies that can observe them.

The line-art extraction boundary now also preserves caller-owned levels,
threshold, cleanup, detail, minimum-feature, and preview controls as one exact
request configuration. A result must match that entire configuration and the
source image identity before its ordered transparent-black vector paths can be
accepted as belonging to the request. Successful comparison returns sealed
request/result evidence; the 128-state drift oracle requires that sealed path to
match the direct validator exactly. Control units, ranges, algorithms, and
defaults remain intentionally unspecified.

Still-open implementation work includes complete notebook-to-PDF Export and
broader PDF conformance validation, resource embedding, font/searchable-text
compatibility, image intake and
placement, render-manifest serialization/identity, output file adapters, actual
visual comparators and goldens, line-art algorithms, and measured print/scan
acceptance thresholds.
