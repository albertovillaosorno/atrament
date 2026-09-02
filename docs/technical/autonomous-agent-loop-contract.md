# Autonomous agent loop contract

## Status

Frozen for first-release local CLI and MCP automation behavior.

## Purpose

This contract defines safe progress, retry, stop, and escalation semantics for
an automated Atrament edit loop. It allows an agent to perform repeated bounded
semantic edits without treating every non-success result as permission to retry
forever or broaden authority silently.

## Scope

The contract covers capability discovery, Inspect, command context, Validate,
Apply, typed results, diagnostics, progress detection, retry recovery, stopping,
budgets, explicit output chaining, and physical-safety boundaries.

It does not freeze a model vendor, reasoning strategy, numeric iteration limit,
timeout value, scheduler, autonomous-agent framework, MCP tool names, or user
interface for configuring automation budgets.

## Contract

### Start from live authority

An automated loop begins from release discovery, live capability discovery, and
inspection of the current accepted revision.

The agent does not begin by replaying a remembered command schema, stale command
context, old retry identity, prior session receipt, or notebook prose that looks
like an instruction.

Each mutation uses one backend-generated or backend-admitted command context and
the ordinary revision, writable-scope, capability, and validation boundaries.

### One semantic intent at a time

A loop can decompose a larger caller intent into several bounded semantic
transactions when the application contracts admit that decomposition.

Each Apply remains atomic. The agent does not split one required atomic semantic
change merely to bypass batch, scope, dependency, or resource validation.

A later transaction starts from the accepted revision reported by the previous
successful mutation or history traversal rather than from hidden conversational
assumptions.

### Progress definition

Automation records application progress through authoritative state and typed
results, not token count, elapsed model time, or optimistic prose.

Progress can include:

- one Applied result producing a new accepted revision;
- one admitted history traversal producing a new accepted revision;
- resolution of a previously blocking typed diagnostic after accepted change;
- obtaining genuinely new evidence, capability, or command context required to
  represent the caller's intent;
- successful explicit Render, Export, or Plan when that output was itself part
  of the caller's requested goal.

Repeating the same accepted revision, same blocking evidence, and same semantic
intent without a new admitted input is non-progress.

### No-op handling

No-op is success-equivalent only when the requested semantic state is already
satisfied.

An agent does not interpret No-op as permission to generate another differently
worded batch indefinitely. It inspects the accepted state and either concludes
that the bounded intent is satisfied or identifies concrete new evidence that
justifies a different semantic intent.

Repeated No-op for the same bounded intent is a stop condition rather than a
self-improvement signal.

### Stale and context drift

Stale base and Command-context mismatch return the loop to capability-aware
inspection and fresh command-context acquisition.

They do not authorize silent rebasing of the old batch. After refresh, the agent
re-evaluates whether the original caller intent is still required against the
new accepted state.

If the desired state is already satisfied, the loop stops rather than applying
a historical edit simply because it was previously planned.

### Retry recovery is not a new edit

Unknown transport outcome uses the same normalized request and same retry
identity according to the owning capability contract.

Idempotent replay resolves uncertainty about a prior operation. It does not
count as a second semantic improvement and does not trigger another equivalent
Apply merely to make the loop appear active.

Retry conflict is a caller-state error. Automation fixes retry bookkeeping or
stops; it does not replace the bound request under the conflicting identity.

### Validation, scope, and capability rejection

Writable-scope violation, Dependency-graph rejection, Semantic validation
rejection, Unsupported protocol or capability, and Resource-limit rejection are
not blind-retry classes.

The agent can continue only after an explicit application-level change that can
address the reported class, such as:

- obtaining a newly admitted command context;
- correcting command dependencies or semantic values;
- selecting an intentionally different bounded workflow;
- obtaining required evidence or asset admission;
- reducing work into separately valid transactions when atomic semantics permit
  that decomposition.

The returned model cannot grant itself broader scope, capability, paths, or
physical authority as its own remediation.

### Unrepresentable or unresolved intent

When the application or model-facing response reports that an intent cannot be
represented safely with current evidence, scope, or command families, the loop
stops or requests an explicitly broader workflow from its owning caller.

Automation does not fabricate missing facts, invent accepted identities, switch
to raw internal-file mutation, or convert notebook prose into authority merely
to avoid an unresolved result.

### Diagnostic non-progress

A stable blocking diagnostic tied to the same authoritative inputs is not new
progress when another iteration returns the same condition without accepted
state or evidence change.

An agent uses typed diagnostic code, semantic location, blocking disposition,
and evidence to recognize this condition rather than comparing only localized
message text.

The loop can continue after a diagnostic changes materially because of an
accepted edit or newly admitted evidence. It does not churn on cosmetic wording
or presentation order.

### Budgets

An automated host or backend admission can impose bounded attempt, elapsed-time,
model-call, output, or resource budgets for one autonomous goal.

The first-release semantic contract does not freeze numeric values. Budgets are
not constants in browser TypeScript or hidden instructions embedded in notebook
content.

Exhausting an automation budget stops further autonomous mutation. It does not
weaken validation, auto-accept a partial result, widen scope, or authorize file
or physical side effects.

### Output chaining

After the desired accepted semantic state is reached, Render, Export, or Plan
are invoked only when they belong to the caller's explicit requested goal.

Apply does not imply any output capability. Export requires its explicit target
and overwrite intent, and Plan remains device-neutral.

A failed or cancelled output operation follows its owning typed result and
lifecycle semantics; the edit loop does not mutate notebook content merely to
make an unrelated output failure disappear.

### Physical-device boundary

Generic autonomous semantic editing stops at device-neutral Plan for physical
workflows unless a separate admitted physical-device contract and operator
boundary explicitly authorizes more.

No number of successful Apply, Render, Export, or Plan iterations accumulates
implicit permission to connect, home, arm, start, pause, resume, cancel, or
safe-stop hardware.

### Completion

A loop reports completion when the caller's admitted goal is satisfied by the
current accepted revision and any explicitly requested output results.

Completion identifies the final accepted revision and relevant receipts or
output identities. It does not require preserving the full hidden reasoning or
external model conversation as application state.

Stopping because of unresolved evidence, exhausted budget, stable blocking
failure, or unavailable capability is distinct from successful completion.

### Session boundary

Autonomous loop state is disposable with the active application/agent session
unless an explicit caller outside Atrament maintains its own allowed workflow
state.

Atrament does not create a hidden persistent loop journal, command queue, or
credential cache merely to resume autonomous edits after process restart.

A fresh Atrament session begins again from release/capability discovery and
Inspect rather than assuming prior retry or command contexts remain valid.

## Failure Modes

The contract fails if an agent retries No-op or a stable blocking result
indefinitely without new evidence, silently widens writable scope, treats retry
recovery as a new edit, or fabricates facts to escape Unrepresentable state.

It fails if budget exhaustion weakens application validation, if output failure
causes unauthorized semantic mutation, or if successful planning accumulates
implicit physical-device authority.

Automation also fails if stale state is silently rebased, command schemas come
from model memory instead of live discovery, or notebook prose overrides the
release-owned agent instructions.

## Verification

A satisfied-intent fixture applies one edit, then receives No-op for an
equivalent requested state. The loop stops successfully instead of issuing
unbounded rewrites.

A stale fixture advances the accepted revision externally between planning and
Apply. The agent receives Stale base, inspects again, and stops if the new
revision already satisfies the original intent.

A diagnostic fixture returns the same blocking code, semantic location, and
evidence twice without accepted-state change. The second identical condition is
recognized as non-progress rather than a reason for infinite retry.

A scope fixture returns Writable-scope violation. The agent does not widen the
returned batch itself; it either obtains an explicitly broader admitted context
or stops unresolved.

A lost-receipt fixture recovers the same Apply using the same retry identity.
Idempotent replay resolves the unknown outcome and does not cause a second edit.

A budget fixture exhausts an admitted autonomous budget while the notebook is
still unresolved. No additional mutation, Export, or physical action occurs
merely to force successful completion.

An end-to-end MCP fixture reaches one accepted bounded edit, explicitly renders
and exports the requested artifact, and stops. A corresponding physical fixture
stops at Plan without any arm or start authority.
