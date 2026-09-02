# Device-neutral plan application contract

## Status

Frozen for first-release live-plan compilation.

## Purpose

This contract defines the read-only application capability that compiles an
accepted Atrament revision into a device-neutral writing plan. It enables
browser, CLI, and MCP automation to inspect exact intended motion without
connecting to or controlling physical hardware.

## Scope

The contract covers Plan inputs, accepted-revision binding, live capability
validation, deterministic compilation, plan identity, diagnostics, receipts,
retries, derived invalidation, and adapter parity.

It does not define vendor motion languages, device connection, calibration,
homing, arming, physical start, transport protocols, or persistent file export.

## Contract

### Explicit read-only capability

Plan runs only when an application caller explicitly requests device-neutral
plan compilation. Semantic Apply does not compile a plan implicitly.

Plan is read-only with respect to accepted notebook source and application
history. It may create bounded in-memory derived state but does not create a new
accepted notebook revision.

Planning also does not write a persistent file. Persisting an admitted plan uses
the separate explicit Export capability.

### Accepted revision binding

Every Plan request identifies the accepted revision whose live projection is
being compiled. The backend does not silently switch to a newer revision because
editing continued after the request was formed.

If the requested revision is no longer admitted for planning, the operation
returns a typed stale or unavailable result rather than compiling another
revision without caller knowledge.

The receipt identifies the revision actually consumed.

### Live capability profile

Plan names one admitted live capability profile. The profile defines the
semantic and output features the device-neutral compiler is allowed to
represent.

The first live profile remains the accepted single-pen boundary: one calibrated
pen identity, one physical ink color, flat blank media, handwriting, sober
headings, ruler-like vectors, tables, equations, and admitted single-color
vector
art.

Digital-only effects do not disappear merely to make planning succeed.
Unsupported
photographs, shadows, sticky-note effects, colors, raster-only content, or
undeclared tool changes produce blocking capability diagnostics unless an
explicit accepted conversion occurred earlier.

### Deterministic plan inputs

Plan compilation is deterministic for its authoritative inputs. Those inputs
include at least:

- accepted revision identity and semantic state;
- admitted handwriting/profile identities and measured geometry;
- live capability profile and behavior version;
- physical page and writable-region constraints owned by the revision;
- accepted assets or line-art projections used by live output;
- declared document or output seed inputs required by handwriting variation;
- plan-engine behavior version and relevant model choices.

Wall-clock time, adapter identity, browser viewport, clipboard state, device
connection state, and ambient randomness do not alter the plan.

### Plan contents

A successful device-neutral plan identifies ordered physical-unit operations
needed by the later hardware boundary. It includes the admitted information
required by the physical-writing ADR, such as:

- semantic origin for strokes or paths;
- pen-up and pen-down geometry;
- physical bounds and writable-region checks;
- speed and acceleration intent;
- optional admitted pressure data;
- pauses or checkpoints required by the plan contract;
- pen identity and capability assumptions;
- estimated execution duration or other derived execution metrics when admitted.

The plan contains no vendor-specific command stream, device file descriptor,
USB endpoint, shell command, or browser presentation coordinate as authority.

### Result semantics

Plan returns result classes from the frozen derived/output result taxonomy. A
successful plan is distinguishable from stale revision, capability or validation
rejection, cancellation, and known-no-result failure without parsing prose.

### Diagnostics and refusal

Planning validates every semantic object against the selected live capability
profile. Blocking incompatibilities use the frozen typed diagnostic envelope
contract.

The compiler does not silently omit unsupported objects, convert content without
an accepted conversion, clip out-of-bounds motion, or invent tool changes to
force a successful plan.

A failed Plan leaves accepted notebook source and history unchanged.

### Plan identity

A successful Plan returns a backend-owned plan identity tied to the accepted
revision and every output-affecting deterministic input.

Equivalent authoritative inputs and behavior versions produce equivalent
plan semantics and plan identity according to the owning identity contract.
Intentional seed changes or admitted profile changes produce a different plan
identity when they can change motion.

The plan identity is not a hardware authorization token and does not imply that
a device is connected, calibrated, armed, or safe to start.

### Receipt

The normalized Plan receipt identifies the consumed revision, live capability
profile, plan identity, deterministic plan inputs or their admitted identities,
plan bounds, relevant diagnostics, and provenance needed for later inspection.

Receipt prose or timing metadata may differ by adapter without changing plan
semantics. The receipt never claims physical execution occurred.

### Operation lifecycle

Plan follows the frozen application operation lifecycle contract for optional
progress, cancellation, transport loss, and session shutdown. Cancellation does
not expose a partial plan as complete or authorize physical-device behavior.

### Retry behavior

Plan has no persistent or physical side effect, so a caller may repeat the same
normalized request after a lost response without risking duplicate notebook,
file, or hardware mutation.

An implementation may cache an equivalent in-memory plan for efficiency, but
cache identity is not application authority. Recomputing from the same
authoritative inputs remains semantically equivalent.

### Derived invalidation

A plan is a derived projection. When an accepted semantic change invalidates any
plan dependency, that prior plan becomes stale for the new revision.

Impact-scoped recomputation may preserve unaffected derived plan regions when
the
backend dependency model proves they remain valid, but the complete exposed plan
must remain internally consistent for one accepted revision and capability
profile.

A caller cannot patch an old motion plan directly to bypass semantic command and
capability validation.

### Separation from physical device state

Plan does not connect, identify, home, calibrate, arm, start, pause, resume,
cancel, or safe-stop a machine.

Physical coordinate calibration and device limits are validated at the owning
hardware adapter boundary before real motion. A device-neutral plan can be fully
simulated and inspected while no physical device is present.

An eventual MCP projection of physical capabilities cannot treat a successful
Plan receipt as implicit operator arming.

### Separation from file export

Plan returns in-memory derived application data. If a caller wants SVG, HP-GL,
GP-GL, or another admitted persistent representation, it invokes the explicit
Export capability or a documented outbound adapter separately.

A Plan request therefore cannot create undeclared files merely because an MCP
agent intends to use the result later.

### Adapter parity

Direct, browser, CLI, and MCP Plan entry paths dispatch through the same
application capability. Equivalent authoritative inputs produce equivalent plan
semantics, diagnostics, identity inputs, and bounds.

MCP may consume structured plan data without opening a browser. That convenience
does not create a second plan compiler or hardware authority.

## Failure Modes

The contract fails if Plan mutates accepted notebook source, adds undo history,
writes a persistent file implicitly, or controls physical hardware as a side
effect.

It fails if unsupported live content disappears silently, if browser pixels or
vendor commands become plan authority, or if ambient randomness makes identical
accepted inputs produce different intended motion.

Safety fails if a Plan identity is treated as device arming, if stale revision
content is silently substituted, or if an MCP caller can bypass capability
validation by supplying raw motion.

## Verification

The first complete user journey compiles its `layout-resolved` accepted revision
under the sober single-pen live profile. The resulting plan contains exactly the
admitted one-pen vectors, safe bounds, speeds, checkpoints, provenance, and
estimated duration described by that journey.

A digital-only fixture adds one unsupported photograph, shadow, or color effect.
Plan rejects or reports the required explicit conversion and does not omit the
object silently.

A determinism fixture compiles the same revision, profile, seed, and engine
version repeatedly through direct, CLI, and MCP paths. Normalized plan semantics
and plan identity agree.

A seed-change fixture intentionally changes one admitted variation seed. The new
plan remains valid but receives the different identity required by changed
motion inputs.

A stale-revision fixture forms a Plan request for one revision, advances
accepted
state, and proves planning never substitutes the newer revision silently.

A no-device fixture runs complete Plan compilation and simulation with no
physical adapter connected. Process inspection proves no connect, home, arm, or
start operation occurred.

A persistence fixture proves Plan itself creates no file. An explicit later
Export operation is required before any plan representation survives session
shutdown.
