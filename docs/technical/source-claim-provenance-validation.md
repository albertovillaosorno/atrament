# Source and claim provenance validation

## Status

Verified against the current transport-neutral citation linkage boundary.

## Purpose

This guide maps the first-release source/claim provenance requirement to
executable structural evidence. It complements semantic-notebook provenance
records by proving that a claim marked `Cited` resolves through its exact
assigned provenance identity to reviewable source metadata.

It does not fetch sources, parse citation syntax, judge source quality, select a
citation style, render citation UI, or merge semantic source provenance with
application transaction provenance.

## Scope

The executable linkage boundary lives in
`src/backend/citation-review-linkage/domain/lib.rs` and reuses
`atrament_semantic_notebook::ProvenanceKind` rather than defining a competing
source-status vocabulary.

The boundary retains three caller-owned inventories:

- claim provenance assignments identifying one claim and its exact assigned
  semantic provenance identity;
- revision-owned semantic `Provenance` records whose existing kind is the only
  authority for whether an assigned claim is `Cited`;
- source records identifying exact caller-owned metadata for human or agent
  review; and
- citation links identifying the exact claim, assigned provenance record, and
  exact source metadata identity participating in one citation relationship.

Source metadata remains opaque to this domain. A DOI, URL, bibliographic record,
local source label, or later typed metadata representation may be carried by the
caller without making this validator the authority for its syntax or quality.

## Contract

`validate_citation_review_linkage` rejects deterministic structural ambiguity.
Claim, provenance, and source identities must be unique in their inventories.
Every claim assignment must resolve to a revision-owned semantic `Provenance`
record. Every citation link must resolve to a known claim, must name that
claim's exact assigned provenance identity, must resolve to known source
metadata, and may only target a claim whose resolved provenance kind is `Cited`.

An exact claim/source relationship may appear only once. One cited claim may
still link to multiple distinct sources. After link validation, every `Cited`
claim must own at least one valid citation link.

The failure order is part of executable evidence: duplicate claim,
provenance, and source inventories; claim-order provenance resolution;
link-order
identity/kind/duplicate checks; then claim-order missing-link checks. No invalid
set is reordered or repaired by the validator.

After that complete structural validation, callers can project the exact
revision-owned `Provenance` record assigned to one claim, exact linked source
identities, or the exact `CitationSource` records. The provenance projection
borrows the authoritative record without copying its kind or source reference.
Source projections preserve caller citation-link order, and source-record
projection exposes the same opaque metadata already admitted by the review set;
it does not fetch, parse, normalize, rank, or judge that metadata. A valid
non-cited claim projects an empty source set, while an unknown requested claim
remains a typed failure after structural validation.

### Relationship to semantic notebook provenance

The semantic notebook already retains `Supplied`, `Derived`, `Cited`, and
`Unresolved` provenance kinds plus revision-owned provenance identities. Block
and inline-span targets can retain a reference to one of those provenance
records, and command-mode editing can change that linkage independently from
claim text.

Citation review linkage adds source-metadata resolution outside that authored
content. It does not rewrite `Provenance.reference`, replace the semantic
notebook's ownership model, or define a new transaction provenance channel.

Application transaction provenance remains separate: whether an accepted
mutation entered through direct human editing, clipboard-assisted model use,
CLI, or MCP is not a citation or source relationship.

### Still open

First-release work still needs source retrieval/admission, a concrete source
metadata schema, citation parsing and formatting, source-quality/sufficiency
policy, provenance diagnostics, candidate-response integration, review UI, and
cross-adapter projection where those capabilities become executable.

## Failure Modes

The present boundary fails structurally when:

- a claim identity appears more than once;
- a provenance identity appears more than once;
- a source metadata identity appears more than once;
- a claim assignment names an unknown semantic provenance record;
- a link names an unknown claim;
- a link names a provenance identity other than the one assigned to its claim;
- a link names unknown source metadata;
- a non-`Cited` claim owns a citation link;
- an exact claim/source relationship is duplicated; or
- a `Cited` claim has no valid source-metadata link.

These failures establish linkage integrity only. They do not establish that a
source is correct, trustworthy, reachable, sufficient for the claim, or
formatted
according to any citation style.

## Verification

`tests/backend/citation-review-linkage/domain/lib.rs` proves exact claim,
provenance, source, and metadata retention; unknown and mismatched identities;
resolved provenance-kind authority; non-cited and missing-link rejection;
duplicate inventory/link rejection; multiple distinct sources for one cited
claim; exact borrowed provenance-record projection; and exact source-record
projection in caller link order with structural failure precedence preserved.

A 32-case compact oracle crosses all four semantic provenance kinds with link
presence, source presence, and provenance-identity match state. It independently
pins missing-link, provenance-mismatch, unknown-source, non-cited-link, and
success behavior without assigning meaning to source metadata.

The test suite uses caller-owned scalar identities and opaque metadata so the
result does not depend on a particular source schema or citation renderer.
