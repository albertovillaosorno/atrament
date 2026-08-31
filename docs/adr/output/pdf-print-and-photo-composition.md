# PDF, print, and photo composition

## Status

Accepted.

## Decision ID

`atrament.output.pdf-print-and-photo-composition`

## Context

Atrament needs complete notebooks that combine simulated handwriting,
mathematics, highlighters, ruled geometry, and photographs. Screen preview
alone cannot guarantee physical size, pagination, color, embedded assets, or
print and scan behavior.

## Decision

PDF is the primary portable page output and is compiled from the accepted page
and stroke plans. Export preserves real dimensions, color intent, page order,
embedded or safely referenced assets, searchable semantic text where compatible
with the handwriting contract, and a manifest linking output to source inputs.

Photographs retain source identity, crop, transform, resolution policy, color
handling, and placement constraints. Preview and final export share the same
layout plan; quality differences are declared renderer choices rather than a
second layout engine.

Clipboard and file intake initially accept PNG, JPEG, and WebP. A placed image
may sit below text, inline with flow, above text, or inside a clipped region.
The editor exposes position, size, crop, opacity, z-order, and a derived
single-color line-art view without overwriting the original session asset.

## Consequences

- Printed geometry can be calibrated against the same physical coordinate model.
- Missing or insufficient image assets fail explicitly.
- Export manifests support reproducibility without appearing on the clean page.
- PDF conformance and font or asset licensing become release concerns.

## Rejected Alternatives

- Screenshots of preview pages were rejected because scale, pagination, and
  asset quality would depend on a display session.
- A separate print layout path was rejected because preview and export would
  drift.
- Silent image downsampling was rejected because it hides irreversible quality
  loss.
- Flattening every page to one raster was rejected because tables, equations,
  writing geometry, and hardware centerlines remain vector authorities.

## Verification

Export fixtures must inspect page boxes, dimensions, asset embedding, color,
metadata, pagination, and reproducible manifests. Physical print and scan tests
must measure scale, clipping, grid registration, and photo placement.
