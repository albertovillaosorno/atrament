# Explicit local PDF printing (Linux CUPS)

## Status

Partially implemented and fake-spool tested on Linux, November 3, 2026.
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
3. Only when physical printing is intended, invoke:

```sh
cargo run --offline -q -p atrament_notebook_paper_cli \
  --bin atrament-print -- --pdf notebook.pdf --printer Local_Queue
```

The queue is mandatory; no default printer is selected. The binary invokes
CUPS with fixed arguments equivalent to `lp -d Local_Queue -`, piping the
already-read PDF bytes into standard input. It does not invoke a shell,
re-render the PDF, or select a copy count.

A zero CUPS exit status means that the spooler **accepted the job**, not that
sheets emerged from the printer. Paper feed, alignment, printer readiness
and physical margins remain device-specific and unverified.

## Failure Modes

One `--pdf` and one `--printer` are required. Queue names must be 1–80 ASCII
bytes, start with a letter or number, and otherwise contain only letters,
numbers, `_`, `-` or `.`. Unknown flags, invalid queues and duplicate options
refuse before any print submission.

The PDF is read through Atrament's symlink-refusing Linux selected-file
opener and exact bounded reader. Missing, nonregular, symlinked, changing
or larger-than-24,000,000-byte inputs refuse. It must have a PDF version
prefix and a terminal `%%EOF` marker.

The framing check is intentionally not a full PDF grammar or security
validator. Operators should print PDFs produced by the native Rust exporter
and review their content first. Missing `/usr/bin/lp` or a failing CUPS
process returns an error instead of claiming a successful submission.

## Verification

Run the isolated test without contacting a printer:

```sh
cargo test --offline -p atrament_notebook_paper_cli --bin atrament-print
cargo clippy --offline -p atrament_notebook_paper_cli --lib --bins -- \
  -D warnings
```

Three focused Rust tests use a real two-page, preprinted native PDF.
A fake `lp` verifies exact PDF bytes and arguments: `-d`, the chosen queue,
then `-`. The tests also reject invalid flags, missing or malformed PDFs,
files above the size cap and symlinks; none sends a real job.

This is partial ATR-P5-012 evidence only. Actual local queue acceptance,
physical page positioning, and saved notebook image overlays remain open,
as does the complete ATR-P8-009 end-to-end journey. No Python runtime was
added or invoked by the production print command.
