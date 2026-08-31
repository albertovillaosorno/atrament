# Calibration from written samples

## Status

Accepted.

## Decision ID

`atrament.handwriting.calibration-from-written-samples`

## Context

A single alphabet sheet cannot reveal natural variation, joins, spacing,
baseline rhythm, numerals, formulas, headings, pressure, or behavior across
different paper sizes. Calibration must gather enough evidence without making
the user perform an unbounded handwriting study.

## Decision

Calibration is a guided, resumable session with known physical reference marks.
It collects isolated characters, common joins, words, sentences, numerals,
punctuation, mathematical symbols, headings, and free writing at selected
speeds and sizes.

Every extracted parameter retains its source region, measurement units,
confidence, and accepted correction history. The system separates observed
evidence from inferred extremes and asks for additional samples when a required
behavior is underdetermined.

## Consequences

- Paper or camera distortion can be corrected against reference geometry.
- Users can inspect and replace weak samples without recalibrating everything.
- Calibration takes longer than importing a font but produces richer behavior.
- The product needs a clear minimum viable sample set and quality report.

## Rejected Alternatives

- Generating a profile from one photograph without confidence reporting was
  rejected because missing behaviors would be invented invisibly.
- Requiring exhaustive samples of every possible pair was rejected because the
  burden would make calibration impractical.
- Treating inferred parameters as observations was rejected because later model
  validation needs their different evidentiary status.

## Verification

Calibration fixtures must recover physical scale and representative parameters
from photographed, scanned, and digitally captured sheets. Held-out writing
must remain separate from training samples and drive the final quality report.
