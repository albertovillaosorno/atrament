# Typed diagnostic envelope contract

## Status

Frozen for first-release cross-capability diagnostics.

## Purpose

This contract defines one structured diagnostic model shared by semantic
validation, layout, inspection, history, Render, Export, Plan, and later
physical
adapter capabilities. It lets humans and automation locate and reason about a
problem without parsing prose or turning adapter-specific errors into domain
authority.

## Scope

The contract covers diagnostic codes, severity, blocking disposition, semantic
locations, evidence, measurements, remediation classes, operation binding,
normalization, completeness, privacy, and adapter parity.

It does not freeze final JSON field names, numeric code values, localization
strings, UI presentation, HTTP statuses, terminal formatting, vendor error
formats, or a complete catalog of future diagnostic codes.

## Contract

### Diagnostic versus application result

An application result class answers what happened to an operation, such as
Applied, Stale base, No-op, Traversed, or an Export conflict. Diagnostics
explain
why, where, and with what evidence.

A diagnostic never replaces the owning operation's typed result class. Severity
alone does not determine whether an operation committed, and an adapter does not
infer result semantics from message text.

One operation can return several diagnostics. An atomically rejected semantic
batch may report several useful candidate findings while still committing none
of them.

### Stable diagnostic code

Every diagnostic has one backend-owned stable semantic code within a versioned
diagnostic namespace. The code identifies the condition class rather than a
localized sentence.

Equivalent conditions exposed through browser, CLI, or MCP preserve the same
semantic code even when wording or presentation differs.

Changing the meaning of an existing code requires an explicit compatibility or
versioning decision. Adapters cannot reuse one code for unrelated failures.

### Severity

Each diagnostic carries a typed severity appropriate to human attention, such as
informational, warning, or error semantics admitted by the final vocabulary.

Severity is distinct from operation blocking. A warning can block one physical
capability while remaining advisory for a digital Render, and a severe-looking
message does not imply that a prior operation committed or rolled back.

### Capability blocking disposition

A diagnostic states whether it is advisory or blocking for the operation and
capability context that produced it. That disposition comes from backend
capability policy, not browser styling or agent preference.

A blocking Plan diagnostic therefore prevents that admitted Plan result even if
the same semantic object can render digitally. Conversely, a digital-only
warning does not silently become a live-output conversion.

### Operation binding

Diagnostics identify the operation context needed to interpret them. Depending
on the capability, this includes the consumed or candidate revision, command
context, command identity, render inputs, export intent, plan capability
profile,
history traversal, or physical adapter state that the diagnostic describes.

Operational request IDs may aid correlation but are not notebook authority.
A diagnostic from one operation cannot be copied onto another revision and
presented as current evidence without backend re-evaluation.

### Semantic locations

Diagnostics locate problems through admitted semantic coordinates rather than
storage representation. A location can name, as applicable:

- accepted semantic object identity;
- semantic field or property key;
- page, flow, table cell, formula, figure, or source identity;
- command identity or admitted batch-local handle during candidate validation;
- citation or provenance owner;
- glyph, line, collision, or geometry owner derived from a semantic identity;
- output, render, plan, or device capability region admitted by its contract.

Locations do not expose DOM nodes, CSS selectors, serialized array offsets,
repository paths, database keys, memory addresses, or browser pixel coordinates
as document authority.

### Multiple locations and relationships

A diagnostic can identify more than one semantic location when the condition is
relational, such as a collision, overlap, dependency conflict, or incompatible
constraint pair.

The diagnostic distinguishes primary owner, related owners, and relationship
semantics sufficiently for an adapter to highlight the right objects without
inventing ownership from list position.

### Typed evidence

When a condition depends on measurable or structured evidence, the diagnostic
carries typed evidence rather than hiding the decision only in prose.

Evidence can include expected and observed values, physical quantities with
units, capability requirements, unsupported feature classes, source/provenance
state, constraint relationships, or another versioned diagnostic payload owned
by the backend.

Measurements use the same authoritative unit and geometry contracts as the
owning domain. A message saying "6 mm overflow" cannot disagree with a typed
measurement that says something else.

### Human-readable explanation

A diagnostic can include localized or otherwise human-readable explanation.
That text is presentation and assistance, not the branch condition for MCP, CLI,
or application logic.

User-controlled notebook or source text quoted in an explanation remains data.
It cannot grant application capabilities or inject instructions into another
adapter merely because it appears inside diagnostic prose.

### Remediation classes

A diagnostic may expose backend-owned remediation classes or admissible
next-step
categories, such as edit content, change a constraint, obtain provenance,
request a conversion, choose another capability profile, or inspect a related
identity.

A remediation class is not an automatically authorized semantic command. The
caller still invokes the appropriate application capability with its ordinary
scope, revision, validation, and safety checks.

Diagnostics do not embed raw device motion, shell commands, hidden file writes,
or browser-side patches as remediation.

### Deterministic diagnostic semantics

For identical authoritative inputs and behavior versions, deterministic
validation produces equivalent diagnostic codes, semantic locations, blocking
disposition, and typed evidence.

Wall-clock timestamps, adapter request IDs, localized message strings, terminal
color, UI ordering used only for presentation, or transport framing do not alter
semantic diagnostic identity.

When diagnostic ordering has semantic meaning, such as an ordered command
sequence, the backend preserves that order explicitly. Otherwise adapters
compare
sets using typed semantic keys rather than incidental map iteration order.

### Diagnostic identity and lifecycle

The final protocol may expose a diagnostic instance identity for correlation.
Such an identity is tied to the owning revision or operation evidence and is not
a stable semantic object identity, credential, retry token, or permission to
mutate the referenced object.

After accepted state or capability inputs change, a caller re-evaluates affected
diagnostics rather than assuming an old instance is still current.

### Completeness and limits

Backend-owned limits can bound diagnostic detail, but an operation cannot report
success while silently omitting a blocking diagnostic required to determine that
success.

If the complete required diagnostic set cannot be represented under an admitted
response limit, the backend exposes explicit incompleteness or another typed
non-success condition appropriate to the capability. It does not silently drop
blocking evidence and invite the caller to proceed.

Non-blocking detail may use an admitted bounded or continuation representation
when the owning capability contract supports it. Completeness remains explicit.

### Privacy and paths

Diagnostics exclude browser session secrets, MCP credentials, unrelated retry
identities, hidden temporary paths, memory locations, and internal repository
paths.

An explicit caller-selected Export target path can appear in an Export
diagnostic when that path is necessary to explain the requested file operation.
That exception does not authorize disclosure of unrelated internal paths.

### Adapter parity

Direct, browser-assisted, CLI, and MCP projections preserve diagnostic code,
severity semantics, blocking disposition, semantic locations, typed evidence,
and completeness for equivalent application outcomes.

Adapters may localize or format prose differently. They do not create new domain
diagnostics, discard blocking evidence, or reinterpret severity and location to
change application behavior.

## Failure Modes

The contract fails if callers must parse English text to distinguish condition
classes, if severity is used as a substitute for operation result, or if browser
styling decides whether a condition blocks backend capability execution.

Location safety fails if diagnostics use DOM nodes, storage paths, array
offsets,
or browser pixels as semantic authority, or if a relational diagnostic loses the
owners required to identify the conflict.

Automation fails if typed evidence and prose disagree, if an old diagnostic is
silently treated as current after its owning inputs change, or if adapters map
the same backend condition to different semantic codes.

Completeness fails if a successful operation can hide a blocking diagnostic due
to response limits. Privacy fails if diagnostics expose credentials or unrelated
internal runtime paths.

## Verification

A semantic-command fixture rejects an invalid command. The batch result remains
Semantic validation rejection while diagnostics identify the failing command,
target identity, stable code, semantic field, and typed precondition evidence.

The first user-journey overflow fixture reports the owning semantic object, page
or flow location, blocking layout condition, and typed 6 mm measurement. Browser
and CLI prose can differ while the code and measurement agree.

A digital-versus-live fixture evaluates one admitted digital-only object. Render
can return an advisory or successful digital diagnostic while Plan reports the
corresponding live capability condition as blocking without changing the shared
semantic owner.

An Export fixture targets an explicitly selected path that cannot satisfy the
requested overwrite or path policy. The diagnostic can name that explicit target
without exposing hidden temporary or repository paths.

A relational collision fixture returns both semantic owners and typed geometry
evidence. Reversing adapter presentation order does not lose which objects are
in conflict.

A diagnostic-limit fixture forces more diagnostic detail than one response can
carry. The application never reports success by silently dropping a required
blocking diagnostic; incompleteness is explicit.

Parity fixtures trigger equivalent validation, Render, Export, and Plan
conditions through direct, browser, CLI, and MCP paths and compare diagnostic
code, severity meaning, blocking disposition, semantic locations, evidence, and
completeness.
