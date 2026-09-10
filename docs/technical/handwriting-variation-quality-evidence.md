# Handwriting variation quality evidence

## Status

Verified against the current transport-neutral evidence boundary.

## Purpose

This guide maps the artifact-validation requirements in the accepted bounded
natural variation ADR to executable structural evidence. It does not define the
statistics, thresholds, perceptual metrics, or sampling behavior that produce a
finding.

## Scope

`src/backend/handwriting-variation-quality-evidence/domain/lib.rs` owns only
report completeness and canonical detected-axis projection. The existing
`handwriting-variation` domain continues to own parameter envelopes and replay
inputs; the new evidence boundary deliberately does not depend on it because a
future statistical harness may compare richer rendered or sampled evidence.

## Contract

A complete report contains exactly one caller-computed finding for each of seven
independent axes:

- calibrated-envelope compliance;
- correlation retention;
- extreme legibility;
- frozen contours;
- local white-noise behavior;
- mechanical baselines; and
- repeated word rhythm.

Calibrated-envelope compliance, frozen contours, local white-noise behavior,
mechanical baselines, and repeated word rhythm directly cover the current TODO
artifact list. Correlation retention and extreme legibility preserve the
additional statistical fixtures required by
`atrament.handwriting.bounded-natural-variation`.

`validate_handwriting_variation_quality_evidence` rejects the first duplicate
axis in caller report order, then the first missing axis in canonical order. An
`ArtifactDetected` finding is valid evidence rather than a structural error.
`detected_handwriting_variation_artifacts` first requires a structurally
complete
report and then returns every detected axis in canonical order, independent of
input ordering.

The checked-in regression fixture exhausts all 128 not-detected/detected
masks and
also removes or duplicates every required axis independently.

## Failure Modes

This boundary can reject incomplete or ambiguous evidence. It cannot decide that
handwriting is statistically natural, visually legible, acceptably correlated,
or within a writer's perceptual identity because the owning metrics and
thresholds do not exist yet.

A future detector must supply independently computed findings without weakening
these completeness rules. A later product/release authority may decide whether
one or more detected artifacts block rendering or profile acceptance; this
domain does not.

## Verification

Run the checked-in exact unit at
`tests/backend/handwriting-variation-quality-evidence/domain/lib.rs`. Keep the
seven axes independent when statistical or perceptual comparators are added so
one passing aggregate score cannot hide an omitted artifact class.
