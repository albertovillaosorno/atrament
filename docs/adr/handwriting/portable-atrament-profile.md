# Portable Atrament handwriting profile

## Status

Accepted.

## Decision ID

`atrament.handwriting.portable-atrament-profile`

## Context

A handwriting identity includes more than glyph outlines. It needs calibration
units, stroke samples, joins, variation ranges, pen and ink preferences, paper
relationships, provenance, and compatibility metadata that can move between
the editor, CLI, MCP, renderer, and writing machine.

## Decision

One file with the `.atrament` extension is the portable profile authority. The
canonical file is a deterministic ZIP archive with `manifest.json` at its root,
typed JSON sections below `sections/`, and optional opaque files below
`assets/`. JSON is UTF-8, uses one canonical key and number encoding, and must
not depend on whitespace or object insertion order for identity.

The manifest declares the container version, profile identity, required and
optional feature identifiers, section paths, media types, byte lengths, and a
SHA-256 digest for every non-manifest entry. Readers validate names, declared
sizes, digests, duplicate paths, and path traversal before decoding a section.
Unknown required features reject the profile; unknown optional features and
assets are retained without being interpreted.

Canonical writers sort entries by path, use fixed archive metadata, omit
platform-specific extras, and store entries without compression. ZIP64 records
are used only when the ordinary ZIP limits require them. Those rules make a
canonical rewrite byte-stable without relying on a particular compressor.
Applications may preserve the original bytes for an unchanged profile, while a
changed profile is rewritten canonically.

Identity, authorization, calibration, stroke vocabulary, contextual variants,
bounded parameters, physical-media presets, and provenance remain typed
sections with explicit schema versions. Embedded source samples are optional;
the manifest records whether reproducibility depends on them. An inspection
command exposes the manifest and typed sections as readable JSON without
extracting opaque assets unless explicitly requested.

## Consequences

- Profiles are single-file, portable, inspectable, and independently verifiable.
- Byte identity is deterministic without standardizing compressor output.
- Large source samples cost their stored size when embedded in the profile.
- Readers can reject corrupt or unsupported required data before rendering.
- Migrations preserve source evidence and report every lossy transformation.
- Future sections can be added without teaching old readers to parse them.

## Rejected Alternatives

- A font file as the complete profile was rejected because it cannot represent
  calibration, joins, dynamics, physical media, or provenance.
- A renamed JSON or YAML document was rejected because large opaque samples,
  partial inspection, and independent entry verification need a container.
- Compressed canonical entries were rejected because compressor versions can
  change bytes even when decoded content is identical.
- A profile stored only in an application database was rejected because it
  would not be portable across interfaces and machines.

## Verification

Golden profiles must rewrite byte-for-byte from the same semantic data and
round-trip semantically across supported migrations. Tests must cover reordered
JSON input, archive metadata, ZIP64 boundaries, duplicate and traversing paths,
size limits, digest mismatch, truncation, unknown required features, preserved
optional data, missing assets, and future container or section versions.
