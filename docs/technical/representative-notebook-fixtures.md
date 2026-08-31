# Representative notebook fixtures

## Status

Frozen for the first-release design contract.

## Purpose

These five fixtures are the stable visual and semantic targets referenced by
Atrament architecture decisions. They prevent implementation work from quietly
optimizing for one easy page shape while leaving dense flow, ruled structure,
diagrams, digital decoration, or single-pen output undefined.

## Scope

This contract fixes each fixture's page intent, exact authored content, semantic
hierarchy, and required layout relationships. It does not freeze serialized
field names, renderer pixels, handwriting samples, or final coordinates; those
belong to later model, layout, profile, and visual-regression authorities.

Whitespace inside the content blocks below is not semantic except inside the
mathematical displays. Punctuation, capitalization, formulas, table cells, and
listed ordering are semantic and must be preserved.

## Contract

### Fixture `dense-two-column`

The page is A4 portrait on a 5 mm squared-paper profile. It is a digital page
with one full-width title region followed by two balanced reading columns. A
column may grow taller than the other, but prose must not alternate between
columns line by line.

Exact content:

```text
Conservation of Mechanical Energy

Core idea
For a closed system with only conservative forces, mechanical energy stays
constant.

E = K + U
K = 1/2 mv²
U_g = mgh

Conditions
• Choose one reference height for gravitational potential energy.
• Keep units consistent through the complete calculation.
• If friction or drag does work, include that energy transfer explicitly.

Worked example
A 2.00 kg ball is released from rest 5.00 m above the floor. Ignore air
resistance and use g = 9.81 m/s².

Initial: K_i = 0 and U_i = mgh.
Final: U_f = 0 and K_f = 1/2 mv².

mgh = 1/2 mv²
v = √(2gh)
v = √(2 × 9.81 m/s² × 5.00 m)
v = 9.90 m/s

Check
Mass cancels, the result has units of speed, and the final kinetic energy is
98.1 J.
```

Expected hierarchy:

```text
page
├── heading "Conservation of Mechanical Energy"
├── column-flow left
│   ├── heading "Core idea"
│   ├── paragraph
│   ├── aligned-math [E, K, U_g]
│   ├── heading "Conditions"
│   └── list [3 items]
└── column-flow right
    ├── heading "Worked example"
    ├── paragraph
    ├── aligned-math [initial, final, derivation, substitution, result]
    ├── heading "Check"
    └── paragraph
```

The title spans both columns. The `Conditions` list stays together when it fits.
The worked derivation preserves equation order and aligns as one semantic block.

### Fixture `table-led`

The page is US Letter portrait on ruled paper. A table is the dominant object;
the title, short orientation sentence, and one boxed note support it without
becoming a competing layout system.

Exact content:

```text
Periodic Trends

Use position in the periodic table to predict how strongly an atom holds and
attracts electrons.

Property | Across a period → | Down a group ↓ | Main reason
Atomic radius | decreases | increases | stronger nuclear pull; more shells down
First ionization energy | increases | decreases | harder to remove an electron
Electronegativity | increases | decreases | stronger attraction in a bond
Metallic character | decreases | increases | electrons are lost more easily down

Shielding note
Inner electrons reduce the effective nuclear attraction felt by outer
electrons. Shielding changes much more down a group than across one period.
```

Expected hierarchy:

```text
page
├── heading "Periodic Trends"
├── paragraph
├── table
│   ├── header-row [4 cells]
│   └── body-rows [4 × 4 cells]
└── callout
    ├── heading "Shielding note"
    └── paragraph
```

The table header is distinct from data rows. Cell text wraps inside its own
cell, row boundaries remain aligned, and the callout cannot overlap the table.

### Fixture `diagram-led`

The page is A4 portrait on dotted paper. A central labeled neuron diagram is the
primary object, with short explanatory blocks arranged around it. Diagram labels
are semantic text, not baked into raster pixels.

Exact content:

```text
Neuron Signal Path

Dendrites → soma → axon hillock → axon → terminals

Signal sequence
1. Dendrites receive input from neighboring cells.
2. The soma integrates those inputs.
3. The axon hillock initiates an action potential when threshold is reached.
4. The action potential travels along the axon.
5. Terminals release neurotransmitter at a synapse.

Myelin speeds conduction along many axons. Gaps between myelin segments are
nodes of Ranvier, where the electrical signal is regenerated.

Key distinction
Along the axon: electrical signal.
Across most synapses: chemical signal.
```

Expected hierarchy:

```text
page
├── heading "Neuron Signal Path"
├── diagram neuron
│   ├── nodes [dendrites, soma, axon-hillock, axon, terminals]
│   ├── directed-edges [4]
│   └── label "Dendrites → soma → axon hillock → axon → terminals"
├── section "Signal sequence"
│   └── ordered-list [5 items]
├── paragraph about myelin
└── callout "Key distinction"
    ├── paragraph "Along the axon: electrical signal."
    └── paragraph "Across most synapses: chemical signal."
```

The diagram owns the page center. Leader lines and labels must stay attached to
their semantic targets after reflow or direct manipulation.

### Fixture `colorful-digital`

The page is A4 portrait on light squared paper and targets only the digital
capability profile. It intentionally uses layered title lettering, multiple ink
roles, a marker highlight, and one loose paper note with a soft shadow.

Exact content:

```text
The Water Cycle · El ciclo del agua

1 · Evaporation / Evaporación
Solar energy warms surface water and changes liquid water into water vapor.

2 · Condensation / Condensación
Cooling water vapor forms tiny liquid droplets and clouds.

3 · Precipitation / Precipitación
Water returns to the surface as rain, snow, sleet, or hail.

4 · Collection / Acumulación
Water gathers in oceans, lakes, rivers, soil, ice, and groundwater.

Energy source: the Sun

Transpiration
Plants also release water vapor through their leaves, adding moisture to the
atmosphere.
```

Expected hierarchy:

```text
page digital
├── decorative-heading "The Water Cycle · El ciclo del agua"
├── cycle-diagram
│   └── stages [evaporation, condensation, precipitation, collection]
├── stage-card × 4
│   ├── bilingual heading
│   └── paragraph
├── highlighted-callout "Energy source: the Sun"
└── loose-paper-note "Transpiration"
    └── paragraph
```

The four stage cards form one visual cycle and retain their numeric order.
Color and marker treatments communicate hierarchy but never replace the stage
names. The loose note and shadow are rejected, not silently dropped, by a live
single-pen compiler.

### Fixture `sober-single-pen`

The page is A4 portrait on a blank live-paper profile. It must compile to one
calibrated pen, one ink identity, handwriting, equations, boxes, and ruler-like
lines only. No color-only or raster-only object is part of the accepted source.

Exact content:

```text
Derivadas — regla de la cadena

Idea
Si y = f(g(x)), entonces la derivada exterior se evalúa en g(x) y se multiplica
por la derivada interior.

(d/dx) f(g(x)) = f′(g(x)) · g′(x)

Ejemplo 1
 y = (3x² + 1)⁵
 y′ = 5(3x² + 1)⁴ · 6x
 y′ = 30x(3x² + 1)⁴

Ejemplo 2
 y = sin(2x³)
 y′ = cos(2x³) · 6x²
 y′ = 6x² cos(2x³)

Comprobación
Deriva primero la función exterior, conserva la interior y después multiplica
por la derivada de la interior.

Error común
No olvides el factor g′(x).
```

Expected hierarchy:

```text
page live
├── heading "Derivadas — regla de la cadena"
├── section "Idea"
│   ├── paragraph
│   └── displayed-math
├── section "Ejemplo 1"
│   └── aligned-math [source, derivative, simplified]
├── section "Ejemplo 2"
│   └── aligned-math [source, derivative, simplified]
├── section "Comprobación"
│   └── paragraph
└── boxed-callout "Error común"
    └── paragraph "No olvides el factor g′(x)."
```

The complete page must remain valid after conversion to a device-neutral live
plan without substitutions, omitted objects, or manual pen changes.

## Failure Modes

A future implementation does not satisfy this contract if it changes authored
facts or formulas, flattens semantic labels into page pixels, silently removes
unsupported live objects, splits ordered derivations incorrectly, allows table
cells or diagram labels to overlap, or claims success after content crosses the
writable region.

Changing exact fixture content requires an explicit contract revision with a
reason. Renderer evolution may change pixels while these semantic authorities
remain stable.

## Verification

Before model freeze, validation consists of review plus exact-content and
hierarchy checks against this document. After the semantic model exists, each
fixture must have a machine-readable source whose normalized authored content
matches these blocks exactly and whose semantic tree matches the stated
hierarchy.

Layout acceptance must render all five pages without blocking diagnostics in
their intended output mode. The colorful fixture must demonstrate digital-only
features, while the sober fixture must compile with exactly one live pen and no
implicit conversions.
