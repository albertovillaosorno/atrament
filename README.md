# Atrament — less handwriting, more living

Atrament is **my personal CLI notebook generator**. I dislike compulsory
handwriting: for me it is a repetitive, bureaucratic use of time, not a better
way to learn. I would rather spend that time thinking, imagining, learning,
and living. This project automates the preparation of handwritten-looking
pages, so the useful work can stay useful and the mechanical copying can stop.

**Actual priority:** configure a notebook, describe handwriting statistically,
compose content and pictures, export pages as **printable PDF or PNG**, and
optionally print them. I can also generate transparent pages to combine with a
photo of a physical notebook in an image editor. This is a personal tool, not
an attempt to ship an enterprise app or build a physical writing robot.

## Scope: the smallest useful thing

- **Only a Linux command-line interface.** No browser workflow, Windows/macOS
  distributions, servers, accounts, hosted AI, MCP, or GUI are needed here.
- **Paper profiles:** arbitrary physical sheet width/height in millimeters;
  blank, ruled, grid, or dot backgrounds; line/grid spacing and colors;
  separate top/bottom/left/right margins; rounded borders and a binding line;
  configurable date location/style and page labels.
- **Handwriting:** ink color, simulated pen width and grain, base glyph font,
  reproducible random seed, line height and letter spacing. Set bounded mean,
  median, minimum, maximum, and standard deviation for size, width, baseline,
  and rotation. Override every character individually, including Spanish
  accents, punctuation, symbols, and custom transparent glyph-image shapes.
  The photo-measuring helper is **manual**, not an unreliable automatic
  handwriting-recognition pipeline.
- **Assets:** local PNG, JPG, WebP, or restricted SVG, in front of or behind
  writing; change size/position, saturation, color tint, and opacity.
- **Pages and exports:** explicit page ordering, dates, printed-sheet flags,
  per-page transparent PNGs, and physical-size print PDFs. Already printed
  sheets can omit grid/background so only remaining content is printed.
  Printing to a CUPS printer requires an explicit command.
- **No plotter/hardware requirement.** The former single-pen and multi-tool
  designs are theoretical history, not a mandatory product or release gate.

## Run it now

Needs Python 3.14+, **Pillow**, and optionally ImageMagick (`magick`) for local
SVG images. Fedora's system packages work; use an isolated environment if
Pillow is not installed. There is no app server or network account.

```sh
python3 cli/notebook.py init --out my-notebook.json
# Edit the JSON: paper, handwriting, pages, images, and dates.
python3 cli/notebook.py render --config my-notebook.json \
  --format pdf --dpi 300 --out homework.pdf
python3 cli/notebook.py render --config my-notebook.json \
  --format png --dpi 300 --transparent --out transparent-pages/
# Optional: explicitly send the generated PDF to the local printer.
python3 cli/notebook.py print --pdf homework.pdf
```

An example with dates, an SVG drawing, and per-letter edits is in
[`examples/notebook.json`](examples/notebook.json):

```sh
python3 cli/notebook.py render --config examples/notebook.json \
  --format pdf --dpi 200 --out example.pdf
```

File writes refuse overwrite unless `--replace` is supplied. Image paths and
custom glyph-image paths are resolved relative to the notebook JSON file.
Raster PNG pages are full-sheet RGBA, so they align with a photographed or
preprinted physical page.

The PDF is a **raster print PDF at the selected DPI**, not searchable/vector
handwriting. Print it at **100% / actual size**, never with automatic
fit-to-page scaling.

## Manual statistical handwriting (the central feature)

The `handwriting` JSON object contains per-glyph statistics. For example:

```json
{
  "height_mm": {
    "minimum": 2.8, "mean": 3.5, "median": 3.45,
    "maximum": 4.3, "stddev": 0.27, "distribution": "gaussian"
  },
  "glyphs": {
    "a": {"height_scale": 0.92, "width_scale": 1.1,
          "rotation_deg": -1, "baseline_shift_mm": 0.1},
    "ñ": {"height_scale": 1.12},
    "?": {"ink": "#254e80"},
    "e": {"image": "my-letters/e.png"}
  }
}
```

- In `gaussian` mode the mean and standard deviation drive the sampled value;
  minimum/maximum clamp it. The median is preserved as measured metadata.
  In `triangular` mode the median is the distribution's mode. Neither method
  guarantees exact empirical distributions in a small text sample.
- Use a photograph plus *adjustable pixel guides* as a visual ruler, then
  enter your own physical measurements in millimeters (no OCR or AI inference):

```sh
python3 cli/notebook.py guide --image photo.jpg \
  --upper 110 --lower 147 --out guides.png
python3 cli/notebook.py calibrate --config my-notebook.json --glyph a \
  --heights 3.1,3.3,3.6,3.4 --widths 1.6,1.7,1.5,1.8 \
  --out calibrated.json
```

For precise letter shape, draw a transparent PNG for the character and set
`glyphs["a"].image` to its path. You can adjust every glyph and every sign in
JSON, without a camera or a trainable algorithm.

## Sheet layouts, photos, and printed-page flags

Edit `paper.style` (`blank`, `ruled`, `grid`, `dot`), `spacing_mm`, physical
margins, `corner_radius_mm`, and date style (`none`, `plain`, `box`,
`underline`). Each `pages[]` entry owns its `text` lines, `images`, `date`,
optional `number`, and `printed` flag. `printed: true` omits the paper
background and ruling; it still renders handwriting and overlays, so a
preprinted page can receive the ink layer only.

```sh
python3 cli/notebook.py mark-printed --config my-notebook.json \
  --pages 1,3-4 --out already-printed.json
python3 cli/notebook.py render --config already-printed.json \
  --format png --transparent --out overlays/
```

To draw the grid in a transparent PNG as well, add `--keep-rules`. A page's
`images` entries support `path`, `x_mm`, `y_mm`, `width_mm`, `height_mm`,
`layer` (`under`/`over`), `saturation` (0–5), `tint` (`#RRGGBB`),
`tint_strength` (0–1), and `opacity` (0–1). Add a photograph of the paper as
an underlay only when you actually want it included in the exported image;
for compositing in Photoshop or similar, export transparently instead.

## Honest current limits

The initial CLI **does produce** multi-page PDF/PNG files, but it is
intentionally minimal. Text must be supplied as explicit lines/pages; lines
extending past margins are refused rather than silently cropped. It does not
automatically paginate essays or reconstruct handwriting from photographs.

Character drawing uses a chosen font or manually supplied glyph PNGs, so
appearance is an *adjustable approximation*, not a guarantee of personal
handwriting fidelity. PDF handwriting is rasterized and cannot be searched or
edited as vector text. Restricted local SVGs require ImageMagick. Maximum
output: 40 pages, 400 DPI and 32 million pixels/page.

The older browser/editor, autonomous-agent, portable-profile, hardware, and
cross-platform distribution studies are preserved in
[`docs/technical/`](docs/technical/) and
[`docs/todo/completed/`](docs/todo/completed/). They were accepted as bounded
academic designs, **not** end-to-end product capabilities. The historical
release ledger remains NO-GO for that old browser-centered release; this local
CLI is an independent, narrower usable surface. Do not use theoretical
acceptance as evidence that external integrations or physical hardware work.

## Personal goal

Avoid handwriting solely to comply with a mechanical assignment format. Make
my own notebook layout, ink and letter profiles as editable as possible, then
print or composite the finished pages. **The time saved is the point.**
