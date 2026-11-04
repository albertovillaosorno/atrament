# Explicit local PDF printing (Linux CUPS)

## Status

Partially implemented and fake-spool tested on Linux, November 4, 2026.
No physical printer has been verified, and ATR-P5-012 remains open.

## Purpose

Atrament has a separate opt-in Rust command for submitting an already
exported PDF to a named local CUPS printer. Notebook, handwriting, paper,
PDF, and PNG **exports do not print as a side effect**.

## Scope

This covers a selected local CUPS queue and an already saved, bounded PDF.
It does not cover printer discovery, image composition, automatic printing,
Windows/macOS spoolers, or verified physical output.

The notebook renderer already supports `printed_sheet: true`. It suppresses
existing ruling while keeping optional numeric date, page numbers, borders,
and explicitly authored ink in the resulting PDF/PNG.

## Contract

### Operator workflow

1. Save and review the notebook, then export a PDF with its correct page
   count. Use `--printed-sheet true` only if the physical sheet already has
   notebook ruling.
2. Inspect the PDF in a viewer and select a local queue with `lpstat -p`.
3. Validate without printing first:

```sh
cargo run --offline -q -p atrament_notebook_paper_cli \
  --bin atrament-print -- --pdf notebook.pdf --printer Local_Queue \
  --dry-run
```

4. Only when physical printing is intended, invoke:

```sh
cargo run --offline -q -p atrament_notebook_paper_cli \
  --bin atrament-print -- --pdf notebook.pdf --printer Local_Queue
```

The queue is mandatory; no default printer is selected. `--dry-run`
checks the selected PDF without launching a CUPS process and prints its
SHA-256 digest in lower-case hexadecimal. After reviewing the exact file,
copy that 64-character digest to require a byte-for-byte match on a later
physical submission:

```sh
cargo run --offline -q -p atrament_notebook_paper_cli \
  --bin atrament-print -- --pdf notebook.pdf --printer Local_Queue \
  --expect-sha256 64_HEX_DIGITS_FROM_DRY_RUN
```

`--expect-sha256` is optional; it also works with `--dry-run` and accepts
upper- or lower-case ASCII hex. Any malformed or stale digest refuses before
contacting `lp`. SHA-256 verifies the exact snapshot that is subsequently
piped to CUPS (not a second read of the filename), including the scenario
where a file changed after the preview. It does not verify PDF semantics,
printable margins or that a job will actually print.

The binary invokes CUPS with fixed arguments equivalent to
`lp -d Local_Queue -`, piping the already-read PDF bytes into standard input.
It does not invoke a shell, re-render the PDF, or select a copy count.

A zero CUPS exit status means that the spooler **accepted the job**, not that
sheets emerged from the printer. Paper feed, alignment, printer readiness
and physical margins remain device-specific and unverified.

## Failure Modes

One `--pdf` and one `--printer` are required. An optional reviewed
`--expect-sha256` must contain exactly 64 ASCII hexadecimal digits and may
appear only once. Queue names must be 1–80 ASCII
bytes, start with a letter or number, and otherwise contain only letters,
numbers, `_`, `-` or `.`. Unknown flags, invalid queues and duplicate options
refuse before any print submission.

The PDF is read through Atrament's symlink-refusing Linux selected-file
opener and exact bounded reader. Missing, nonregular, symlinked, changing
or larger-than-24,000,000-byte inputs refuse.

A selected PDF must begin
with a complete `%PDF-1.0` through `%PDF-1.7` or `%PDF-2.0` version line,
terminated by LF or CR, and end with `%%EOF` plus optional whitespace.
It must also have a terminal decimal `startxref` locator pointing into the
source to an `xref` table or a plausible PDF 1.5+ indirect `/Type /XRef`
stream header. The stream form is admitted only for PDF 1.5–1.7 or 2.0;
older PDF 1.0–1.4 headers require the traditional `xref` table form.

The stream type name must be an exact top-level `/Type /XRef` dictionary
entry; longer names such as `/XRefShadow` and apparent type entries inside
comments, literal strings, arrays or nested dictionaries are not sufficient.
The dictionary must be followed by an actual `stream` keyword and line break,
not an incidental word substring. These are bounded lexical framing checks,
not PDF stream decoding or a complete syntax parser.

Missing, malformed, out-of-bounds or repointed locators,
non-xref indirect objects, unsupported version numbers and truncated headers
refuse without contacting CUPS. The last cross-reference section is checked
for framing; earlier incremental updates are not decoded.

The framing check is intentionally not a full PDF grammar, cross-reference
parser or security validator. Some unusual or nonconforming external PDF
writers may need a reviewed re-export before printing. Operators should print
PDFs produced by the native Rust exporter and review their content first.
Missing `/usr/bin/lp` or a failing CUPS process returns an error instead of
claiming a successful submission.

## Verification

Run the isolated test without contacting a printer:

```sh
cargo test --offline -p atrament_notebook_paper_cli --bin atrament-print
cargo clippy --offline -p atrament_notebook_paper_cli --lib --bins -- \
  -D warnings
```

Five focused Rust tests use a real two-page, preprinted native PDF,
including SHA-256 known-answer, malformed digest, changed-valid-PDF,
unsupported PDF versions and forged cross-reference trailer refusals before
the fake CUPS transport starts. A sixth regression rejects false stream types
in comments, strings, arrays and nested maps while accepting a real-looking
nested stream dictionary with CRLF boundaries.
A fake `lp` verifies exact PDF bytes and arguments: `-d`, the chosen queue,
then `-`. The tests also reject invalid flags, missing or malformed PDFs,
files above the size cap and symlinks; none sends a real job.

This is partial ATR-P5-012 evidence only. Actual local queue acceptance,
physical page positioning, and saved notebook image overlays remain open,
as does the complete ATR-P8-009 end-to-end journey. No Python runtime was
added or invoked by the production print command.
