# MCP application capability projection

## Status

Frozen for first semantic-command MCP integration.

## Purpose

This contract defines the minimum application capabilities that MCP must project
from the Atrament core. It enables end-to-end agent automation without creating
an MCP-only mutation model or bypassing revision, provenance, file, and physical
safety boundaries.

## Scope

The contract defines capability effects, ordering, receipts, scope, and parity.
It does not freeze MCP tool names, transport configuration, authentication
mechanism, JSON field names, or a Rust MCP framework.

MCP is one inbound adapter to the same application services used by direct, CLI,
and browser-assisted workflows. Its admission boundary is separate from browser
session credentials as required by the localhost runtime contract.

## Contract

### Adapter admission

MCP does not inherit the browser session secret merely because both adapters run
locally. The final backend integration must admit MCP through an explicit
inbound
adapter boundary appropriate to its launch and transport model.

Admission determines whether an MCP caller may reach application capabilities.
It does not weaken per-capability revision, scope, path, provenance, capability,
or device checks.

The browser secret, copied model prompt, and command retry identity are
different
concepts. None is reused as a generic MCP credential.

### Capability classes

#### Inspect

Effect class: read-only.

Inspect returns accepted revision identity plus bounded semantic context,
diagnostics, capability metadata, or receipts required for the next operation.
It never mutates the notebook, history, files, adapters, or hardware.

Capability discovery projects the backend-owned capability snapshot used by the
semantic command contract. An MCP caller can learn admitted protocol versions,
command families, relevant limits, and behavior versions before constructing a
batch instead of probing unsupported mutations.

The capability snapshot is not MCP authentication and does not widen writable
scope merely because an agent can read it.

An agent may request broader context when the admitted application contract
allows it. The core determines the returned semantic representation rather than
exposing internal storage objects or DOM state.

#### Command context

Effect class: read-only.

A command-context capability can project the same backend-owned command request
semantics used by clipboard-assisted chat. It identifies base revision, readable
context, writable scope, admitted command families, constraints, and prompt or
context identity.

An MCP-native agent does not need to round-trip that text through clipboard. It
may use the structured application context directly when the final MCP schema
admits that representation.

#### Validate

Effect class: read-only candidate simulation.

Validate resolves one complete semantic batch against its named base revision
and command context. It returns semantic diff, predicted impact, per-command
outcomes, and diagnostics without changing accepted state.

A successful validation is not a reservation. Apply still rechecks the accepted
base revision before commit.

#### Apply

Effect class: accepted-revision mutation.

Apply validates and atomically commits one semantic command batch. It returns
the
normalized application receipt and creates at most one accepted revision.

Apply does not implicitly export a file, compile a device plan, arm hardware, or
start motion. Those are separate capabilities.

#### Undo and redo

Effect class: accepted-history mutation.

Undo and redo traverse accepted application history through the core. They
replay or reverse accepted transactions according to the frozen semantic
application-history contract and return the resulting accepted revision and
diagnostics.

Read-only inspection may expose whether each history direction is currently
admitted. Traversal returns machine-readable history outcomes such as Traversed,
History boundary, Idempotent replay, or Stale current revision rather than
requiring agents to match prose.

They are not semantic command families and cannot be embedded inside an edit
batch to obscure history traversal.

#### Render

Effect class: derived computation.

Render consumes an accepted revision and admitted render inputs through the
frozen Render application contract. It does not mutate semantic source merely to
make rendering succeed.

A render receipt identifies the accepted revision and renderer inputs that
produced the result. MCP does not define a separate preview or layout engine.

#### Export

Effect class: explicit persistent side effect.

Export writes an admitted output to an explicit caller-supplied or otherwise
explicitly selected path through the owning file adapter. It consumes accepted
source and derived authorities and does not become autosave.

The path boundary, overwrite behavior, format validation, retry recovery, and
output receipt are owned by the frozen explicit Export application contract.
MCP does not gain permission to write arbitrary internal repository files.

#### Plan

Effect class: derived device-neutral computation.

Plan compiles an accepted revision against one admitted live capability profile
through the frozen device-neutral Plan application contract. It returns a
motion plan plus diagnostics and provenance.

Planning does not connect, home, arm, or start physical hardware, and it does
not write a persistent plan file implicitly.

### Physical device capabilities

Device connect, identify, home, arm, start, pause, resume, cancel, and safe-stop
belong to the physical adapter boundary. They are not implied by semantic
command
mode or by the generic MCP edit workflow.

A future MCP projection of physical capabilities must preserve explicit
calibration, limits, dry-run, arming, and operator requirements from the
physical
writing device ADR. This contract does not admit automatic motion as a side
effect of Apply or Plan.

### Automation sequence

A normal autonomous editing loop is compositional:

```text
inspect accepted revision
→ obtain bounded command context
→ validate or apply semantic batch
→ inspect receipt and diagnostics
→ repeat if another edit is required
→ render, export, or plan explicitly
```

The agent can perform the complete loop without opening a browser. Each step
still has one declared effect class and one normalized receipt.

A stale-base result returns to inspection. The agent does not receive an
implicit
rebase or silent retry against a newer revision.

### Bounded context and writable scope

Read context and write scope are distinct for MCP just as they are for copied
command prompts. An inspect result may show neighboring semantic identities
while
one Apply remains bounded to explicitly admitted targets or insertion anchors.

An agent cannot widen Apply scope by echoing additional IDs from readable
context. A genuinely global edit requires an intentionally global command
context or application capability.

### Receipt chaining

Every mutating or derived capability identifies the accepted revision it
consumed or produced. Receipts are suitable for chaining without relying on
hidden conversational state.

Apply returns its result revision before a later Render, Export, or Plan is
invoked. Export and Plan therefore cannot accidentally consume the stale base
revision merely because the agent issued them in the same conversation.

Retry identities are scoped to their owning mutating capability. They are not a
global credential and are not reused to authorize unrelated operations.

If an MCP transport timeout or disconnect makes an Apply outcome unknown, the
agent retries the same normalized batch with the same retry identity before
issuing another edit. It does not assume timeout means rollback.

That recovery is bounded to the active ephemeral session. A later independent
session cannot use a retry identity as a durable transaction lookup or session
credential.

### Result-class projection

MCP exposes the semantic application result class frozen by the command result
taxonomy in a machine-readable form. Human-readable diagnostics may accompany it
but are not the automation branch condition.

Stale base and Command-context mismatch lead back to inspection. Idempotent
replay is success-equivalent for outcome recovery. Retry conflict, scope,
dependency, resource, and semantic validation failures remain distinguishable so
an agent can correct the appropriate part of its request.

A transport error that arrives without a valid Apply receipt remains an Unknown
transport outcome at the caller. The MCP adapter does not fabricate a rejected
or
cancelled application result merely to fit transport error framing.

Adapter-specific MCP error metadata may exist alongside the application result.
It cannot erase or reinterpret the normalized core outcome when the application
actually completed.

### Adapter parity

MCP may improve ergonomics by projecting bounded tools, but it cannot create new
domain behavior unavailable through the application core.

Equivalent direct, CLI, and MCP operations normalize to the same accepted
semantic state, diagnostics, impact decisions, and output authorities. Browser
clipboard transport may add human review but does not create a second command
language.

### No internal-file authority

MCP tools do not mutate Atrament by editing repository files, serialized session
memory, cache files, renderer internals, or browser state. Application mutations
enter through typed core capabilities.

Explicit import and export are the admitted persistent file boundaries. Internal
implementation files remain implementation details rather than an agent API.

## Failure Modes

The projection fails if an MCP-only tool can bypass command validation, revision
checks, provenance, capability diagnostics, or the accepted file boundary. It
also fails if one generic tool can hide unrelated mutation, export, and physical
motion effects behind one opaque call.

It fails if MCP inherits browser credentials implicitly, if retry identities are
used as authentication, or if hidden conversational context is required to know
which accepted revision a mutating operation consumes.

Automation parity fails when an equivalent MCP edit produces accepted notebook
semantics that direct or CLI application cannot reproduce. Physical safety fails
if Apply or Plan can arm or start a machine as an undeclared side effect.

## Verification

The semantic command acceptance fixtures run first against the direct
application boundary. Equivalent MCP calls then normalize and compare accepted
revisions, semantic changes, impact sets, diagnostics, and provenance.

A complete MCP acceptance flow starts from `sober-single-pen`, inspects its
accepted revision, applies the `Idea` correction, inspects the receipt, and
renders or exports the resulting revision without opening a browser.

A stale-base fixture changes the accepted revision between Inspect and Apply.
Apply must reject the stale batch and force another Inspect before a new edit
can
commit.

An export fixture proves that Apply creates no file, then invokes Export
explicitly and verifies only the selected output path is written. A Plan fixture
proves no device connection, arm, or start occurs.

Admission tests prove MCP does not reuse the browser session secret implicitly.
Repository and temporary-file checks prove tools cannot mutate product state by
writing implementation files outside explicit import or export capabilities.
