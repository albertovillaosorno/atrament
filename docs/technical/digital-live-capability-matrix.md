# Digital and live capability matrix

## Status

Frozen for the first-release design contract.

## Purpose

This matrix turns the digital/live output ADR into an exhaustive first-release
compiler contract. Every admitted semantic object and output-affecting treatment
has an explicit result in each mode so unsupported content cannot disappear or
change meaning merely to make compilation succeed.

## Scope

The matrix covers semantic blocks, handwriting and decoration styles, image
treatments, color, page and paper objects, and device-neutral hardware actions.
It describes first-release capability, not whether a particular theme or device
chooses to use every accepted feature.

The result words have fixed meanings:

- `accept`: preserve the requested semantics directly in that output mode.
- `convert`: require a visible, explicit conversion before mode acceptance.
- `reject`: block that output mode for the object or action as requested.
- `future`: reserved for a later capability profile, not the first release.

A conversion is never implicit. The accepted document or output projection must
record the chosen conversion and its provenance.

## Contract

### Semantic blocks

| Capability | Digital | Live |
| --- | --- | --- |
| heading | accept | accept |
| date | accept | accept |
| paragraph | accept | accept |
| quotation | accept | accept |
| ordered list | accept | accept |
| unordered list | accept | accept |
| definition | accept | accept |
| citation | accept | accept |
| source note | accept | accept |
| footnote | accept | accept |
| unresolved claim | accept | accept |
| inline mathematics | accept | accept |
| displayed mathematics | accept | accept |
| aligned mathematics | accept | accept |
| matrix mathematics | accept | accept |
| units in mathematics | accept | accept |
| table | accept | accept |
| merged table cells | accept | accept |
| box | accept | accept |
| divider | accept | accept |
| arrow | accept | accept |
| text label | accept | accept |
| page reference | accept | accept |
| semantic diagram | accept | accept |
| callout | accept | accept |
| margin note | accept | accept |
| freeform region | accept | accept |
| photograph | accept | convert |
| raster illustration | accept | convert |
| vector line art | accept | accept |
| loose paper note | accept | reject |
| unresolved unsupported block | reject | reject |

Unresolved claims remain visible review objects but cannot be exported as if
verified. An unresolved unsupported block blocks export until the user resolves,
replaces, or deliberately removes it from the accepted document.

### Handwriting and decoration styles

| Capability | Digital | Live |
| --- | --- | --- |
| body handwriting role | accept | accept |
| title handwriting role | accept | accept |
| subtitle handwriting role | accept | accept |
| label handwriting role | accept | accept |
| caption handwriting role | accept | accept |
| formula handwriting role | accept | accept |
| margin handwriting role | accept | accept |
| annotation handwriting role | accept | accept |
| ruler-like line | accept | accept |
| hand-drawn line | accept | accept |
| bounded ruler error | accept | accept |
| bounded baseline drift | accept | accept |
| marker highlight | accept | convert |
| filled highlight | accept | convert |
| decorative title layering | accept | convert |
| title outline | accept | convert |
| simulated shadow | accept | reject |
| loose-note fold | accept | reject |
| textured paper fill | accept | reject |
| digital paper shadow | accept | reject |

Live conversions preserve hierarchy with one pen. Marker and filled highlights
convert only to an explicit underline, box, spacing, or admitted stroke-weight
change. Decorative title layers and outlines convert to a sober one-pen title
role; the original digital treatment remains part of the source document.

### Color

| Capability | Digital | Live |
| --- | --- | --- |
| one ink color | accept | accept |
| multiple simulated ink colors | accept | convert |
| marker color | accept | convert |
| colored title layers | accept | convert |
| colored diagram strokes | accept | convert |
| full-color photograph | accept | convert |
| grayscale photograph | accept | convert |
| transparent alpha | accept | convert |
| automatic physical pen change | reject | future |
| multiple physical pen colors | reject | future |

A live color conversion maps meaning to one calibrated physical ink identity.
It must not rely on grayscale shade differences that the single pen cannot
reliably reproduce.

### Image treatments

| Capability | Digital | Live |
| --- | --- | --- |
| PNG source | accept | convert |
| JPEG source | accept | convert |
| WebP source | accept | convert |
| source identity | accept | accept |
| crop | accept | accept |
| scale | accept | accept |
| position | accept | accept |
| z-order | accept | accept |
| below-text placement | accept | convert |
| inline placement | accept | convert |
| above-text placement | accept | convert |
| clipped-region placement | accept | convert |
| opacity | accept | convert |
| original raster pixels | accept | reject |
| single-color line art | accept | accept |
| configurable line-art levels | accept | accept |

For live output, image placement is accepted only after the raster source has an
explicit accepted line-art projection. Crop, scale, position, z-order, and clip
then apply to that vector projection. Opacity and layer blending must resolve to
one-pen geometry rather than simulated tone.

### Page and paper objects

| Capability | Digital | Live |
| --- | --- | --- |
| blank sheet | accept | accept |
| ruled paper | accept | convert |
| dotted paper | accept | convert |
| squared paper | accept | convert |
| custom digital paper | accept | convert |
| sheet size | accept | accept |
| orientation | accept | accept |
| printable region | accept | accept |
| binding edge | accept | accept |
| outer margin | accept | accept |
| writing inset | accept | accept |
| top clearance | accept | accept |
| border geometry | accept | convert |
| grid or rule geometry | accept | convert |
| page background color | accept | reject |
| paper texture | accept | reject |
| simulated loose sheet | accept | reject |

The initial live contract assumes one blank, flat, calibrated physical sheet.
Rules, dots, grids, borders, or custom paper marks convert only when the user
chooses to draw them with the same pen. A preprinted or pre-ruled physical sheet
requires a future registration capability and is not implied by digital paper.

### Device-neutral hardware actions

| Capability | Digital | Live |
| --- | --- | --- |
| pen-up travel | reject | accept |
| pen-down stroke | reject | accept |
| speed value | reject | accept |
| acceleration value | reject | accept |
| calibrated pressure | reject | accept |
| pressure proxy | reject | accept |
| safe bounds | reject | accept |
| pause | reject | accept |
| checkpoint | reject | accept |
| estimated duration | reject | accept |
| connect device | reject | accept |
| identify device | reject | accept |
| home device | reject | accept |
| arm device | reject | accept |
| start plan | reject | accept |
| pause plan | reject | accept |
| resume known state | reject | accept |
| cancel plan | reject | accept |
| safe stop | reject | accept |
| resume uncertain state | reject | reject |
| automatic tool change | reject | future |
| multiple active pens | reject | future |
| raster printing action | reject | reject |

Digital export never embeds device control as a side effect. Live execution
requires a device adapter to admit each accepted action and may still reject a
plan when the exact device, calibration, limits, feedback, or recovery state is
insufficient.

### Compiler invariants

Every mode compiler must enumerate incompatible objects before output. A mode
change cannot mutate the semantic source merely to remove diagnostics. Accepted
conversions create an inspectable projection linked to the source identity and
conversion choice.

If one object requires `convert`, the output remains blocked until the user has
selected and accepted a supported conversion. `Reject` means there is no
first-release conversion for that request. `Future` is never interpreted as a
best-effort fallback.

## Failure Modes

The contract is violated if a compiler omits an unsupported object, collapses
multiple physical colors into one without an accepted conversion, rasterizes
live content, turns a photograph into line art without approval, claims a
preprinted sheet is calibrated from a digital paper profile, or resumes hardware
from an uncertain position.

It is also a failure to expose a capability in the editor that cannot be mapped
to one of these mode outcomes. New semantic or style capabilities require a
matrix row before they can be accepted into the first-release model.

## Verification

Model coverage tests must enumerate every admitted block, style, image
treatment, color,
paper object, and hardware action and prove that each has exactly one matrix
result per output mode. A coverage test must fail when a new capability lacks a
row.

Golden compilation tests must exercise every `convert` and `reject` path. The
colorful digital fixture must compile without lossy conversion. The sober live
fixture must compile with one pen, no raster action, no tool change, and no
unsupported object. Hardware simulation must refuse uncertain-state resume even
when every semantic object is otherwise live-compatible.
