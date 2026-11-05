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

The local submission path has a fixed 60-second deadline, covering both
the transfer of selected PDF bytes to `lp` stdin and the process exit.
A stalled spooler is terminated and its immediate process reaped; the CLI
returns an **uncertain submission outcome**, not proof of rejection.
Inspect the named CUPS queue before deciding whether to submit again.
Atrament never retries a job automatically, and does not manage processes
that `lp` might spawn independently.

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
Trailing `%%EOF` trivia and xref dictionary token separators use PDF's
six whitespace values (NUL, horizontal tab, LF, form feed, CR, space), not
Rust's ASCII-whitespace predicate. A NUL separator in valid metadata or
after EOF is preserved and accepted; a vertical tab is not a legal PDF
numeric delimiter and refuses before CUPS. This is a lexical preflight,
not a full PDF syntax or content validator.

It must also have a terminal decimal `startxref` locator pointing into the
source to an `xref` table or a plausible PDF 1.5+ indirect `/Type /XRef`
stream header. The stream form is admitted only for PDF 1.5–1.7 or 2.0;
older PDF 1.0–1.4 headers require the traditional `xref` table form.

The terminal decimal byte offset allows the same PDF whitespace on either
side of its digits, while refusing vertical tab. The offset must still be
an unsigned decimal and point to a framed cross-reference structure.

A traditional `trailer` keyword may directly touch its `<<` dictionary
opener: PDF delimiters separate tokens without intervening whitespace.
The parser still rejects extended keywords such as `trailerX` and checks
the same bounded trailer dictionary after the token boundary.

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

Traditional xref subsection counts now use ASCII decimal digits only,
as their first object indices already do. Signed tokens such as `+1` and
`+0001` refuse before CUPS, even though the Rust integer parser could
otherwise convert them to valid record counts.

The index and count must
occupy one physical line separated by horizontal whitespace; LF/CRLF or
PDF comments cannot splice them across separate lines. The `xref` keyword
itself needs a line break before the first subsection. These framing checks
also apply to the first section of an earlier `/Prev` table target.

Traditional table entries now require the offset, generation and `f`/`n`
status to appear on one complete CR- or LF-terminated physical line, after
the subsection header's line break, with horizontal whitespace between
them. The preflight no longer splices a record
from successive lines or ignores stray trailing words. This does not require
one exact 20-byte PDF entry layout or parse earlier incremental sections.

The reserved object-zero table record is always a free-list head with
generation 65535, even when another table record has that same generation.
An in-use record zero or a free record zero with any other generation
refuses before CUPS. The gate does not require a sparse terminal table to
repeat object zero if that record is not present in the selected subsection.

All PDF dictionary names in terminal table trailers and cross-reference
stream headers must use valid `#` plus two hexadecimal digits for escapes,
including unknown extension keys. Malformed or truncated escapes refuse
before contacting CUPS; valid escaped names and aliases remain supported.

Hexadecimal string values (`<...>`) in traditional trailers and typed
cross-reference stream dictionaries must contain only ASCII hexadecimal
digits and the six PDF whitespace bytes. Mixed-case digits, whitespace and
odd final nibbles remain valid; punctuation, vertical tabs and nested `<`
delimiters refuse before any queue contact. This is a lexical metadata check,
not validation of every PDF object or encoded stream row.

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

A terminal cross-reference stream now needs exactly one top-level direct
`/Length` between 1 and 24 MB, bounded by the selected PDF input cap.
The length counts encoded data bytes after the `stream` line ending; the
terminating `endstream` must appear after those exact bytes and a line break.
The scanner does not look for an arbitrary last `endstream` inside content.
Indirect length references may refuse under this conservative print policy.

A cross-reference stream must advertise one top-level direct `/W` array
of exactly three unsigned decimal byte widths, each from 0 to 8 inclusive.
At least one field must occupy bytes. Missing, nested-only, duplicated,
negative, noninteger, oversized and extra array fields refuse before CUPS.
This local preflight does not decode compressed or uncompressed stream rows.

An optional top-level `/Index` array must comprise nonempty pairs of
unsigned decimal object numbers and positive counts. The pairs must be
ordered and disjoint, with each exclusive endpoint inside `/Size` and the
same resource ceiling. Invalid, repeated, signed or overflowing entries
refuse before CUPS; absent `/Index` retains PDF's implicit range policy.
This does not verify encoded rows or incremental cross-reference chains.

A typed cross-reference stream must also
end with standalone `endstream` and `endobj` keywords, allowing whitespace
and comments before its final `startxref` locator. Missing, fused or
junk-trailed terminators refuse before invoking CUPS. These are bounded
lexical framing checks, not PDF stream decoding or a complete syntax parser.

Missing, malformed, out-of-bounds or repointed locators,
non-xref indirect objects, unsupported version numbers and truncated headers
refuse without contacting CUPS. An optional terminal `/Prev` must point
back to a prior xref section.

An optional `/XRefStm` key in a traditional-table trailer now receives a
separate hybrid-reference check. Only PDF 1.5+ may use it; the direct decimal
byte offset must lead back to a complete `/Type /XRef` indirect stream, not
an arbitrary object, an xref table, a truncated stream or a future offset.

Its bounded dictionary and exact `/Length` data and closing keywords are
checked without decoding stream records or following older chains. Duplicate
keys, including escaped-name aliases, refuse before CUPS. A real one-page
PDF 1.5 hybrid-reference example remains readable by `pdfinfo` and reaches
the fake spool with exact source bytes.

When an earlier xref stream itself declares `/Prev`, that operand must still
be a direct positive decimal byte offset before that stream. The older
target is not followed: terminal `/Prev` and hybrid `/XRefStm` receive
immediate predecessor framing, not recursive revision-chain validation.

A PDF 1.5 incremental hybrid regression includes two framed xref streams
and a terminal traditional table with valid backward `/Prev` and `/XRefStm`
links. The local fake CUPS receives the selected source byte-for-byte; an
independently generated equivalent PDF is readable by `pdfinfo` as one page.

Prior traditional tables need a bounded, complete first subsection with
valid record fields; any in-use entries in it must point to matching object
headers before that earlier table. A forged earlier count that omits an
advertised row refuses. Earlier free-list next-object pointers must remain
below the same local object-count ceiling as terminal tables.

Prior xref streams need an admitted dictionary,
their exact encoded data length, and standalone `endstream`/`endobj` markers.
Further sections, trailers and older revision chains are not recursively
validated or decoded.

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

Nine fake-spool refusal cases test missing, zero, nondecimal, duplicate,
nested-only, oversized and mismatched stream lengths. Four positive framing
fixtures admit bounded exact lengths including an `endstream` byte sequence
inside encoded data; no stream payload is decoded or printed on a real queue.

Two fake-spool cases refuse an in-use xref-table entry for object zero
and a free object-zero record with generation other than 65535.
The valid head-to-free-object chain and native pages remain admitted.

Two additional fake-CUPS cases refuse signed traditional xref subsection
counts and keep the source unchanged; an ordinary numeric count still passes.

Six fake-spool refusals cover split-line offset/generation/status, a count
and record sharing one line, and trailing extra tokens. Valid LF and CRLF
record lines remain admissible;
source bytes are never changed during refused print admission.

Thirteen additional fake-CUPS refusal cases prove malformed terminal
xref-stream `/W` arrays cannot be submitted or modify source bytes.
Three valid direct arrays preserve conservative admission, including a
nested decoy and a PDF comment between widths.

Sixteen additional fake-spool refusals cover malformed `/Index` ranges,
including overlap, overflow and pairs outside declared `/Size`. Five
framing positives preserve optional, bounded and commented forms.

A genuine one-page PDF 1.5 cross-reference-stream positive fixture
contains a real catalog/page tree and five binary xref records. `pdfinfo`
reads the generated 353-byte document as one Letter page without repair.
A fake-CUPS test confirms exact SHA-256 review and spool byte identity
for explicit, implicit and partitioned `/Index` forms; no printer runs.

The same real stream test also exercises PDF 2.0 and requires its
reviewed bytes to reach fake CUPS unchanged. Identical binary structure
under a legacy PDF 1.4 header refuses admission before any submission,
regardless of whether an external viewer can repair the invalid version.

A fake `lp` verifies exact PDF bytes and arguments: `-d`, the chosen queue,
then `-`. The tests also reject invalid flags, missing or malformed PDFs,
files above the size cap and symlinks; none sends a real job.

This is partial ATR-P5-012 evidence only. Actual local queue acceptance,
physical page positioning, and saved notebook image overlays remain open,
as does the complete ATR-P8-009 end-to-end journey. No Python runtime was
added or invoked by the production print command.
