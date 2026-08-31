# Notebook paper and decoration

## Status

Accepted.

## Decision ID

`atrament.style.notebook-paper-and-decoration`

## Context

The handwriting must sit naturally on squared, ruled, dotted, blank, or custom
paper and support dates, titles, highlighter marks, boxes, dividers, and margin
systems. Treating these as arbitrary decoration would make page calibration and
semantic restyling inconsistent.

## Decision

Paper is a physical page profile with size, orientation, margins, binding edge,
printable area, background, and optional rule or grid geometry in real units.
Decoration is expressed through semantic style roles such as title, date,
highlight, callout, rule, margin note, and table rule.

Paper controls include sheet dimensions, grid or rule spacing, outer margin,
inner writing inset, top clearance, border shape, corner roundness, and whether
page marks appear below or above simulated ink. A nominally square grid may use
slightly rounded joins and bounded ruler error without changing cell dimensions.

Styles can choose handwriting, ruler-like geometry, marker behavior, color,
spacing, and relationship to the paper grid. A notebook theme binds roles to
presets without replacing semantic content or calibrated coordinates.

Digital composition may simulate loose notes with a paper fill, folded edge,
and soft shadow. That visual object is never interpreted as a physical sticky
note and is excluded from live single-pen output.

## Consequences

- One notebook can be retargeted across paper formats with explicit reflow.
- Titles and dates remain editable concepts rather than flattened marks.
- Grid-aware placement can align writing without forcing every baseline onto a
  perfect rule.
- Ruler-like marks remain slightly human without losing measured alignment.
- Themes need accessibility and print-contrast validation.

## Rejected Alternatives

- Baking paper lines into every page image was rejected because dimensions and
  printing behavior would be ambiguous.
- Styling by arbitrary per-object colors and offsets alone was rejected because
  consistency and restyling would be difficult.
- Perfect baseline locking was rejected because calibrated handwriting needs
  controlled drift around the paper geometry.

## Verification

Golden pages must cover supported paper families, orientations, margins, and
style roles at physical scale. Print and scan measurements must verify grid
spacing, inset, border geometry, baseline relationships, color, clipping, and
contrast. Live-mode projection must reject unsupported decorative objects.
