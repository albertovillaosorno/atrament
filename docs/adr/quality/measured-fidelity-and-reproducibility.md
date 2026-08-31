# Measured fidelity and reproducibility

## Status

Accepted.

## Decision ID

`atrament.quality.measured-fidelity-and-reproducibility`

## Context

An output can look attractive while repeating glyphs unnaturally, drifting from
the writer, misaligning with paper, or changing between preview and export.
Subjective inspection is necessary but insufficient for a system built around
personal and physical fidelity.

## Decision

Every render records the semantic document identity, profile identity, paper
profile, asset identities, engine version, model choices, and variation seed.
Given those accepted inputs, layout and stroke planning are reproducible.

Quality evidence combines geometric measures, distribution comparisons,
held-out sample tests, perceptual review, print or scan measurements, and real
device trials where applicable. No single similarity score is treated as proof
that writing is natural, identical, or physically faithful.

## Consequences

- Defects can be reproduced from a compact render manifest.
- Intentional rerolling creates a new seed and traceable output identity.
- Benchmarks require representative writers, media, pages, and edge cases.
- Product claims remain bounded by the evidence actually collected.

## Rejected Alternatives

- Visual approval alone was rejected because reviewers miss systematic and
  physical errors.
- One opaque machine-learning score was rejected because it cannot explain
  failure or cover every fidelity dimension.
- Bit-identical raster output as the only requirement was rejected because
  physical and perceptual correctness matter beyond bytes.

## Verification

The test corpus must include held-out handwriting, repeated text, formulas,
tables, images, paper variants, and physical outputs. Validation must detect
input drift, non-reproducible plans, frozen variation, out-of-range parameters,
and unsupported fidelity claims.
