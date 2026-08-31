# First complete user journey

## Status

Frozen for the first-release design contract.

## Purpose

This journey defines the first end-to-end behavior Atrament must prove before
individual editor or renderer features can be called complete. It connects the
one-shot external-chat workflow to transactional candidate acceptance, direct
human correction, blocking layout diagnostics, PDF export, and a validated live
motion plan.

## Scope

The journey covers one localhost session and one notebook. It does not require
an embedded model account, network access from Atrament, physical device motion,
or a final cross-boundary field schema. Later schema names may change without
changing the observable states and invariants defined here.

The journey uses the exact authored content of the `sober-single-pen` fixture.
Its intended live output is one calibrated pen on one blank A4 sheet.

## Contract

### Initial state

Atrament starts with an empty ephemeral session. The interface visibly states
that closing the session discards unexported work. Both the LLM editor and human
page editor are present in the primary split workspace, even before a notebook
has been accepted.

The user pastes this task into the task surface:

```text
Turn these chain-rule notes into one clear A4 study page. Keep the Spanish text
and formulas exactly as provided. Use a sober hierarchy that can also be written
with one physical pen. Do not add facts that are not in my notes.
```

The source material is the exact content block from fixture
`sober-single-pen`. The user selects an admitted handwriting profile, blank A4
paper, the sober theme, and both PDF and live output targets.

State: `source-prepared`.

### Copy the one-shot prompt

The permanent Copy prompt control generates one self-contained request. The
copied request includes the task, complete source material, current paper and
style constraints, output targets, complete admitted semantic schema, source
and provenance rules, diagnostic expectations, and required return envelope.

The copy result exposes a prompt identity and prompt version. Repeating Copy
without an intervening source or constraint change produces the same prompt
identity. No hidden previous chat message is required to interpret the copied
request.

State: `prompt-copied`.

### External model round trip

The user pastes the prompt once into an external chat and pastes the complete
structured response once back into Atrament. The raw pasted response is retained
for inspection as session state but is not yet the accepted notebook.

Parsing creates a candidate document or a diagnostic set. A malformed response,
unknown required schema feature, fabricated source fact, or invalid identity
cannot partially mutate the accepted notebook.

For this journey, the candidate preserves every fixture sentence and formula,
uses semantic headings and aligned mathematics, and places the final
`Error común` callout partly beyond the writable bottom edge by 6 mm.

State: `candidate-ready`.

### Review and accept the candidate

The interface shows the candidate source, the candidate page preview, the
out-of-bounds diagnostic, and a semantic difference view against the currently
accepted notebook. Because this is the first candidate, the accepted side is
empty rather than implicitly equal to the candidate.

The user confirms that the candidate did not invent or remove source content and
accepts it. Acceptance is one transaction: semantic source, stable identities,
and declared constraints become current together. Derived layout and render
state may then recompute from that accepted authority.

State: `candidate-accepted`.

Export remains blocked because the accepted fixed callout crosses the writable
region.

### Make a human text correction

In the human editor, the user changes the `Idea` paragraph from:

```text
Si y = f(g(x)), entonces la derivada exterior se evalúa en g(x) y se multiplica
por la derivada interior.
```

to:

```text
Si y = f(g(x)), la derivada exterior se evalúa en g(x) y se multiplica por la
derivada interior.
```

The edit is a typed semantic text command against the paragraph identity, not a
canvas-pixel mutation. The LLM editor immediately reflects the same text and
keeps selection on that semantic paragraph. Undo and redo replay the command
without regenerating unrelated notebook content.

State: `human-corrected`.

### Resolve the overflow

The page preview highlights the `Error común` callout and the violated bottom
edge. The diagnostic identifies the owning object, the 6 mm overflow amount,
and supported correction classes.

The user drags the callout upward until it is fully inside the writable region.
That direct manipulation serializes a placement constraint on the callout and
recompiles layout. The diagnostic disappears only after geometry proves the
object fits; no content is clipped or silently moved to an invisible area.

State: `layout-resolved`.

At this point there are no blocking source, glyph, collision, overflow, or
capability diagnostics for the selected PDF and live targets.

### Export PDF

The user requests PDF export. Atrament compiles from the accepted semantic
source and accepted layout, preserving physical A4 page dimensions, vector
writing and mathematics where admitted, the single-page order, and a render
manifest that identifies the notebook, profile, paper, renderer choices, and
seed.

The PDF export does not mutate session source or live-plan state. Its success
returns an explicit user-selected path and output identity.

State: `pdf-exported`.

### Compile the live plan

The user requests live output from the same accepted notebook. The live
capability compiler enumerates the source and confirms that every object is
accepted without conversion under the sober single-pen profile.

The device-neutral plan contains one pen identity, ordered pen-up and pen-down
paths, physical bounds, speed, acceleration, admitted pressure data, pauses or
checkpoints where required, semantic origin for strokes, and an estimated
execution duration. It contains no raster action, physical color change, loose
paper object, shadow, or tool change.

A simulator can inspect the complete plan without connecting to hardware. The
journey is complete when the plan validates and dry-run bounds are clean; actual
machine arming and motion belong to the hardware acceptance journey.

State: `live-plan-ready`.

### Required state sequence

The successful path is exactly:

```text
empty-session
→ source-prepared
→ prompt-copied
→ candidate-ready
→ candidate-accepted
→ human-corrected
→ layout-resolved
→ pdf-exported
→ live-plan-ready
```

A failure may return to an earlier review state, but it cannot skip candidate
acceptance, layout resolution, or capability validation merely because a later
output was requested.

### Cross-step invariants

- The accepted notebook changes only through explicit candidate acceptance or a
  typed human command.
- The pasted model response is never document authority by itself.
- Every exact source sentence and formula remains traceable to supplied content.
- Prompt identity changes when task, source, schema, paper, style, or output
  constraints change.
- Human direct manipulation serializes intent and recompiles authoritative
  geometry.
- Blocking overflow prevents both PDF and live-plan success.
- PDF and live compilation consume the same accepted semantic and layout
  authorities.
- Output-specific compilation may create projections but never silently rewrite
  the source notebook.
- Closing the session after export discards session state while explicit output
  files remain.

## Failure Modes

The journey fails if Copy prompt depends on hidden chat context, a pasted model
response mutates accepted content before acceptance, source facts change without
review, human correction edits only preview pixels, overflow is clipped instead
of diagnosed, PDF and live paths use different layout authorities, live
compilation drops an unsupported object, or export creates undeclared autosave
state.

It also fails if an invalid candidate leaves a partially updated notebook or if
the user cannot identify why export is blocked and which semantic object owns
the diagnostic.

## Verification

An end-to-end test must drive every named state and assert the invariants after
each transition. Negative cases must cover malformed model output, source drift,
rejected candidate acceptance, undo and redo, unresolved overflow, unsupported
live content, and session closure.

The successful fixture must compare normalized authored content before and after
the model round trip, after the human correction, after overflow resolution,
and after both outputs. The final PDF manifest and live plan must reference the
same accepted notebook, profile, paper, and seed identities. The live plan must
contain exactly one pen identity and zero tool-change or raster actions.
