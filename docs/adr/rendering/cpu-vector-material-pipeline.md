# CPU vector and material rendering pipeline

## Status

Accepted.

## Decision ID

`atrament.rendering.cpu-vector-material-pipeline`

## Context

Atrament needs crisp geometry for text, equations, tables, PDF, and physical
motion while still looking like ink on paper. A flat font render looks sterile,
but a fully raster authority would lose scale, editability, and machine paths.

## Decision

Authoritative output begins as CPU-generated vector geometry. Semantic blocks
produce layout boxes, handwriting produces stroke centerlines and expanded ink
contours, and ruled or diagram elements produce measured paths.

Appearance is composed from deterministic material layers clipped to that
geometry: base ink, edge variation, deposition texture, paper interaction, and
optional highlight or color layers. A final low-frequency, soft noise layer may
unify the page without displacing geometry or hiding legibility.

Every stochastic layer derives from the accepted document seed, semantic object
identity, profile, and material preset. Preview may lower texture resolution,
but it uses the same vectors, seeds, blend order, and physical dimensions as
final output.

## Consequences

- PDF can preserve vectors while embedding bounded texture resources.
- Live hardware consumes centerlines rather than simulated surface pixels.
- CPU cost is measurable and independent of a compute GPU.
- Texture resolution, blend order, and noise scale become versioned inputs.

## Rejected Alternatives

- A single texture over the complete page was rejected because ink, marker,
  paper, and photos require different material behavior.
- GPU-only raster simulation was rejected because ordinary CPU machines are the
  required baseline.
- Converting the final page into one bitmap was rejected because it would erase
  vector and motion authority.

## Verification

Golden tests must compare vector topology, layer order, seeds, physical bounds,
and final pixels independently. CPU benchmarks must cover preview and final
quality on machines without a discrete GPU.
