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

The table form also requires at least one numbered subsection with
fixed-width decimal offset/generation and `n`/`f` state records, followed by
a `trailer` dictionary. An `xref` keyword alone or an empty or malformed
table no longer qualifies for print admission. Its trailer also needs a
lexically recognizable `/Size` entry with a positive direct ASCII decimal;
missing, zero, signed, non-decimal or duplicate size tokens refuse.

Numeric values immediately adjacent to the closing `>>` remain eligible.
The declared
`/Size` must be at least each subsection's first object number plus its
record count; otherwise a table claims objects outside its own extent.

Each in-use `n` record also needs an absolute decimal byte offset within
the selected PDF, after its version header. An offset of zero or past the
file refuses before CUPS. At an in-use offset the gate also requires a
matching `number generation obj` header; an offset into a body, the xref
section, or a different numbered/generation object refuses.

Free `f` entries use object indices, not document byte offsets, and are
exempt from byte position checks. Instead their next-free object index
must be less than the trailer `/Size`; the five-digit generation counter
on both `f` and `n` records must fit an unsigned 16-bit PDF generation
number. This does not parse the referenced object body or prove
cross-reference completeness.

The trailer scanner now balances nested dictionaries and arrays, skipping
literal/hex strings and comments when locating the one top-level `/Size`.
Only whitespace and PDF comments may follow the complete outer `>>` before
`startxref`; unclosed maps, nested-only size entries or trailing extra
operators refuse before any printer launch. It does not validate all PDF
objects or the byte offsets of cross-reference entries.

The stream type name must be an exact top-level `/Type /XRef` dictionary
entry; longer names such as `/XRefShadow` and apparent type entries inside
comments, literal strings, arrays or nested dictionaries are not sufficient.
The dictionary must be followed by an actual `stream` keyword and line break,
not an incidental word substring. A solitary `>` delimiter outside a hex
string cannot impersonate a nested or terminal dictionary closure in either
xref stream or traditional-table trailer preflight.

Cross-reference streams now also need a single direct, top-level decimal
`/Size` in the same range as traditional-table trailers: 1 through
1,333,334 objects. The stream scanner skips nested dictionaries, strings
and comments when finding that value and refuses missing, duplicate,
nondecimal, signed or oversized entries without launching the spooler.

The stream's own indirect object header must use a nonzero object number
below its declared `/Size` and a generation from 0 through 65535.
An object outside that range cannot identify its own cross-reference data;
these are framing constraints, not a complete object graph check.

A typed cross-reference stream must also
end with standalone `endstream` and `endobj` keywords, allowing whitespace
and comments before its final `startxref` locator. Missing, fused or
junk-trailed terminators refuse before invoking CUPS. These are bounded
lexical framing checks, not PDF stream decoding or a complete syntax parser.

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

A seventh regression refuses empty, truncated and invalid traditional
`xref` tables before fake-CUPS invocation, while admitting numbered
multi-subsection framing and the unchanged native two-page output.
The xref-stream regression also rejects six header-only or malformed
terminal delimiter cases without spooling, while admitting terminal comments.

Five additional fake-table cases refuse trailing tokens, missing outer
dictionary closure, and `/Size` entries seen only in nested maps or comments.
Valid compact, nested/commented and native two-page trailers remain eligible.
A further four fake-CUPS cases reject unmatched single `>` and `]` tokens
inside the stream header or table trailer; ordinary nested maps still pass.
Two more refuse impossible `/Size` totals for complete xref subsections,
while a high-numbered valid subsection remains admissible.

Two fake-spool refusals cover zero and out-of-file in-use object offsets.
Four more refuse an in-range offset into a table or object body and a
mismatched object number or generation. A matching synthetic indirect
object and native two-page Rust PDF still pass; no real printer was used.
Another two fake-spool refusals cover an out-of-range free-list object
index and a generation value greater than 65535, while a valid two-entry
free-list chain remains eligible.

Two more fake-CUPS refusals cover duplicate or overlapping object-index
subsections in one terminal xref table. Subsection ranges must advance in
object-number order and contain at least one record. Sparse gaps remain
accepted, as verified by a positive disjoint-section fixture. This does
not traverse incremental-update `/Prev` chains or parse all PDF objects.

A tiny selected PDF may otherwise advertise a multi-billion-object `/Size`
without supplying those xref records. The traditional-table gate now caps
`/Size` at the 24 MB input budget divided by an 18-byte minimum record,
rounded up (1,333,334 objects). This is a conservative local print resource
policy, not a PDF-specification size limit: extremely sparse external PDFs
above it require reviewed re-export. The inclusive ceiling and a forged
9,999,999,999-object trailer have positive and fake-spool refusal tests.

Ten more fake-spool refusals exercise malformed or unbounded cross-reference
stream `/Size` declarations, including missing and nested-only values.
The tests began red on the prior stream framing gate, then passed after
sharing the numeric bound with tables. Plausible bounded stream framing
and the exact inclusive size ceiling remain accepted.

Six fake-spool refusal cases cover stream object zero, number at or above
`/Size`, and out-of-range generation. Three positive headers cover distinct
bounded object IDs and the inclusive 65535 generation limit. All refused
files remain unchanged and no real CUPS queue is contacted.

A fake `lp` verifies exact PDF bytes and arguments: `-d`, the chosen queue,
then `-`. The tests also reject invalid flags, missing or malformed PDFs,
files above the size cap and symlinks; none sends a real job.

This is partial ATR-P5-012 evidence only. Actual local queue acceptance,
physical page positioning, and saved notebook image overlays remain open,
as does the complete ATR-P8-009 end-to-end journey. No Python runtime was
added or invoked by the production print command.
