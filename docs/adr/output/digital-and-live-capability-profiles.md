# Digital and live capability profiles

## Status

Accepted.

## Decision ID

`atrament.output.digital-and-live-capability-profiles`

## Context

A PDF can simulate colors, highlighters, photographs, shadows, and stacked
paper. A physical single-pen machine cannot honestly reproduce those effects.
Allowing preview-only decoration to leak into a motion plan would create missing
content or require manual intervention that the document never declared.

## Decision

Every notebook compiles against an explicit output capability profile. Digital
mode supports layered color, marker styles, photographs, simulated paper notes,
shadows, textured fills, decorative titles, and print or PDF export.

Live mode initially supports one calibrated pen, one physical ink color, blank
flat sheets, handwriting, sober titles, ruler-like lines, tables, equations,
and single-color vector drawings. Photographs or drawings are converted to
transparent black line art with configurable levels before acceptance.

Unsupported objects never disappear automatically. The compiler reports each
incompatible block and offers an explicit conversion, replacement, omission, or
mode change. A future quiet multi-tool machine receives a new capability profile
rather than weakening the single-pen contract.

## Consequences

- The same semantic notebook can have different accepted projections.
- Users see live incompatibilities before approaching hardware.
- Sticky-note simulation remains a digital composition effect only.
- Future color hardware can be added without pretending current machines have
  unsupported capabilities.

## Rejected Alternatives

- One universal feature set was rejected because digital and physical media
  have materially different capabilities.
- Silent grayscale or object removal was rejected because the output could lose
  meaning.
- Requiring manual pen changes in the first live release was rejected because
  it would complicate calibration, resumption, and unattended safety.

## Verification

Capability tests must enumerate every semantic and style block in both modes.
Live acceptance must prove that its complete plan uses one pen and contains no
raster-only, color-only, shadow, sticky-note, or undeclared tool-change action.
