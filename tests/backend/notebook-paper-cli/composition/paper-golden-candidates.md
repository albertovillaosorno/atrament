# Native paper visual reference candidates

These are **unapproved visual candidates**, not owner-reviewed goldens.
They preserve the actual Rust CLI's transparent A4 page PNG and physical
vector PDF bytes for the default 5-mm grid and the ruled variant.

| Style | Native PDF | Native 72-DPI PNG |
| --- | --- | --- |
| Ruled | `a4-ruled-72dpi.pdf` | `a4-ruled-72dpi.png` |
| Grid | `a4-grid-72dpi.pdf` | `a4-grid-72dpi.png` |

Both PNGs are 596 × 842 pixels with RGBA transparency. The native PNG
writer intentionally uses uncompressed deflate blocks, so these image
fixtures are larger than typical compressed screenshots. Do not transcode
them without an explicit byte-golden review.

Reproduce from the repository root with the native Rust CLI, changing the
style and destination filename for the other pair:

```sh
cargo run --offline -q -p atrament_notebook_paper_cli \
  --bin atrament-paper -- --style ruled \
  --output tests/backend/notebook-paper-cli/composition/a4-ruled-72dpi.pdf
cargo run --offline -q -p atrament_notebook_paper_cli \
  --bin atrament-paper -- --style ruled --format png --dpi 72 \
  --output tests/backend/notebook-paper-cli/composition/a4-ruled-72dpi.png
```

Use a fresh output path, or opt in to `--replace` only after reviewing the
new visual output. Test the candidate fixtures against the independent
pinned hashes, native PDF/PNG bytes and decoded pixels with:

```sh
cargo test --offline -p atrament_notebook_paper_cli --lib \
  ruled_and_grid_page_candidate_golden_bytes_and_pixels_are_stable
```

Do not infer color fidelity on a printer, human visual approval, or
handwriting/image-overlay coverage from these paper-only candidates.
