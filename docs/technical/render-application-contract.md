# Render application contract

## Status

Frozen for first-release preview and final rendering.

## Purpose

This contract defines the read-only application capability that renders an
accepted Atrament revision through the authoritative CPU vector and material
pipeline. It keeps preview, CLI, MCP, and later Export projections aligned
without making browser pixels or files into document authority.

## Scope

The contract covers Render inputs, accepted-revision binding, deterministic
vector and material inputs, quality profiles, result identity, diagnostics,
receipts, retry behavior, derived invalidation, and adapter parity.

It does not freeze raster codecs, canvas APIs, PDF file writing, final pixel
formats, UI zoom, filesystem paths, or GPU implementation details.

## Contract

### Explicit read-only capability

Render runs only when an application caller requests an admitted preview or
other render projection. Semantic Apply does not render as an implicit mutation
side effect.

Render does not change accepted semantic source or application history. It may
create bounded in-memory derived render state for the active session.

Render itself does not write persistent output. A caller that wants a file uses
the explicit Export capability.

### Accepted revision binding

Every Render request identifies the accepted revision whose visual projection is
being requested. The backend does not silently switch to a newer revision merely
because editing continues.

If the requested revision is no longer admitted for rendering, Render returns a
typed stale or unavailable result rather than substituting another revision.

The receipt identifies the revision actually consumed.

### Authoritative geometry

Render consumes backend-owned semantic layout, handwriting stroke geometry,
mathematics, tables, diagrams, page rules, image placement, and other accepted
vector authorities.

The browser does not become layout authority by displaying the result. Canvas
pixels, CSS geometry, viewport dimensions, and screenshot coordinates do not
replace physical-unit vectors as accepted source.

Preview and final digital output therefore start from the same accepted geometry
and physical page dimensions.

### Deterministic render inputs

Render behavior is deterministic for its authoritative inputs. Those inputs
include at least:

- accepted revision identity and semantic state;
- handwriting profile identity and admitted model choices;
- paper profile and physical page geometry;
- accepted asset identities and placement constraints;
- document variation seed and semantic object identities;
- material presets, blend order, and other appearance behavior versions;
- renderer behavior version;
- declared quality profile and output-affecting render options.

Wall-clock time, adapter identity, browser focus, clipboard state, UI split
ratio,
and ambient randomness do not alter authoritative render meaning.

### Preview and final quality profiles

Preview and final modes share vector geometry, physical dimensions, seeds,
semantic object identities, and material ordering.

A lower-cost preview profile may reduce declared texture resolution, sampling
density, or other quality-only work. It does not silently change line wrapping,
page breaks, object placement, stroke centerlines, semantic order, or physical
bounds.

A quality option that can legitimately change authoritative geometry is not a
mere preview-quality setting and must enter through the owning semantic or
layout contract instead.

### Material rendering

Material appearance is composed deterministically from admitted layers such as
base ink, edge variation, deposition texture, paper interaction, highlights,
color, and bounded low-frequency noise.

Stochastic material behavior derives from declared seeds plus semantic identity,
profile, and material inputs. Unseeded randomness is not an admitted render
input.

Material layers may change appearance but cannot displace accepted vector
geometry merely to look more natural.

### Digital-only features

Digital render profiles may represent admitted colors, highlighters,
photographs, shadows, textured fills, paper-note effects, and decorative titles
according to the digital capability contract.

Their successful Render does not imply that a live single-pen Plan can represent
them. Render and Plan validate against different explicit output capability
profiles while sharing one semantic notebook authority.

### Diagnostics

Render returns diagnostics through the frozen typed diagnostic envelope
contract for missing assets, unsupported render inputs, invalid geometry,
material failures, or other conditions that prevent an admitted result.

The renderer does not silently omit a semantic object, replace an asset, change
layout, or flatten unsupported content simply to manufacture a successful
preview.

A failed Render leaves accepted source and history unchanged.

### Render identity and manifest inputs

A successful Render returns a backend-owned render identity or equivalent
manifest identity tied to the accepted revision and every output-affecting
render input.

The manifest records enough accepted identities and behavior versions to
reproduce the projection according to the measured-fidelity contract.

Adapter request IDs, UI timestamps, browser dimensions, or terminal formatting
do not become render identity inputs unless an explicit output contract declares
them semantically relevant.

### Result representation

Render may return vector data, bounded raster resources, page surfaces, or
another backend-owned representation appropriate to the admitted adapter. The
representation remains derived output rather than notebook source.

A browser may compose or display that result locally. It does not reinterpret
semantic blocks or recalculate authoritative layout to fill gaps in the returned
projection.

### Operation lifecycle

Render follows the frozen application operation lifecycle contract for optional
progress, cancellation, transport loss, and session shutdown. Cancellation does
not expose a partial render as complete or mutate accepted source.

### Retry behavior

Render has no persistent, semantic, or physical side effect. Repeating the same
normalized Render request after a lost response is safe and produces equivalent
render semantics.

The backend may reuse an in-memory cached result when its full authoritative
input identity matches. Cache presence or eviction does not change application
meaning.

### Derived invalidation

A render is stale whenever an accepted semantic or derived dependency that can
change its output is invalidated.

Impact-scoped recomputation may preserve unaffected vectors, material regions,
or pages when dependency evidence proves them reusable. It may not preserve a
cached region whose result can change merely to make incremental rendering
appear smaller.

A complete exposed Render result remains internally consistent with one accepted
revision and one admitted set of render inputs.

### Separation from Export

Render produces in-memory derived application output. Persistent PDF, image, or
other file creation uses the explicit Export application contract.

Export may consume authoritative vectors and render inputs directly rather than
capturing browser pixels. A screenshot is not an equivalent file-export path.

### Adapter parity

Direct, browser, CLI, and MCP Render entry paths dispatch through the same
backend render capability. Equivalent authoritative inputs produce equivalent
geometry, render identity inputs, diagnostics, and material semantics.

Adapters may choose different presentation containers for the same admitted
result. They cannot introduce a second layout engine or alter semantic content.

## Failure Modes

The contract fails if Render mutates accepted notebook source, adds undo
history,
writes persistent files implicitly, or treats browser layout or screenshot
pixels as authoritative geometry.

It fails if preview and final modes use different page layout, seeds, stroke
geometry, or physical dimensions merely because quality differs.

Reproducibility fails if ambient randomness, wall-clock time, map iteration
order, or adapter identity changes render meaning for identical authoritative
inputs.

It also fails if cached derived regions survive a dependency invalidation that
can change their output or if unsupported semantic content disappears silently.

## Verification

A representative notebook renders repeatedly from the same accepted revision,
profile, paper, assets, seed, renderer version, and quality profile. Vector
geometry, physical bounds, render identity inputs, and deterministic material
behavior remain equivalent.

A preview-versus-final fixture compares vector topology, line wrapping, page
breaks, stroke centerlines, physical dimensions, seeds, and blend order. Only
admitted quality-cost dimensions such as sampling or texture resolution differ.

A seed-change fixture alters the accepted variation seed deliberately. The new
render obtains the distinct identity required by changed appearance inputs while
remaining geometrically valid.

An invalidation fixture changes one bounded paragraph and proves only dependent
render regions are reused or recomputed. A global paper or style change expands
the affected render work according to real dependencies.

A no-file fixture invokes Render through direct, browser, CLI, and MCP paths and
proves no persistent output appears. A later explicit Export is required for a
file to survive session shutdown.

A browser-parity fixture proves the frontend displays backend-owned projection
data without issuing domain network calls, parsing semantic commands, or
recomputing authoritative page layout in TypeScript.
