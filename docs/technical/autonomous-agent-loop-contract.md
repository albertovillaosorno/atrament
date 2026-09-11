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

A transport-neutral progress-evidence vocabulary now classifies the five frozen
positive evidence conditions above and four explicit non-progress conditions:
idempotent replay recovery, repeated No-op for the same intent, unchanged
revision/evidence/intent without new admitted input, and a stable repeated
blocking diagnostic. It classifies only already-qualified evidence conditions.

An application-level semantic-result projection now derives accepted-revision
progress directly from Applied and replay-recovery evidence directly from
Idempotent replay. Other result classes remain unclassified until their owning
state comparison qualifies additional progress or non-progress evidence.

### No-op handling

No-op is success-equivalent only when the requested semantic state is already
satisfied.

An agent does not interpret No-op as permission to generate another differently
worded batch indefinitely. It inspects the accepted state and either concludes
that the bounded intent is satisfied or identifies concrete new evidence that
justifies a different semantic intent.

Repeated No-op for the same bounded intent is a stop condition rather than a
self-improvement signal.

History-result guidance is also transport-neutral: a committed traversal
continues from its reported revision, idempotent replay recovers prior
completion without another traversal, a history boundary stops blind retry in
that direction, and stale history returns to inspection. Cancellation and known
no-commit failure remain workflow-policy decisions.

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

A transport-neutral partial guidance classifier now materializes the result
branches whose next-step class is frozen by these rules. It maps twelve semantic
command result classes and deliberately leaves Successful validation, Cancelled
before commit, and Internal failure with known no-commit unclassified because
their next action still depends on owning workflow policy.

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


An application-level stop projection now maps only the already-qualified
repeated stable-blocking evidence class to the frozen stable-blocking terminal
stop. Replay, repeated No-op, and generic unchanged-input evidence do not become
terminal on this projection alone.

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

A transport-neutral budget admission boundary now retains independent exhaustion
facts for attempt, elapsed-time, model-call, output, and resource budgets. It
requires a budget-only mutation stop whenever any supplied axis is exhausted,
without freezing numeric thresholds or granting mutation when no budget stops
it.

The application-level budget-stop projection maps that stop disposition to the
frozen Stopped exhausted budget terminal class. A non-exhausted budget axis does
not manufacture completion or any other terminal outcome.

### Persistent output intent

An autonomous edit goal does not acquire Export authority from model-generated
or notebook-provided prose. Persistent output must be part of the caller's
explicit goal or an admitted host policy independent from the untrusted semantic
content being edited.

When automatic Export is admitted, the agent forms a request within that path
and overwrite policy and still uses the frozen Export validation boundary. It
does not copy a path from notebook prose into file authority merely because it
looks operational.

A transport-neutral output-intent boundary now records that caller-explicit
goals can establish Render, Plan, or Export intent, while an independently
admitted host policy can establish persistent Export intent only.
Model-generated or notebook-provided semantic content establishes no output
intent on this axis.

### Output chaining

After the desired accepted semantic state is reached, Render, Export, or Plan
are invoked only when they belong to the caller's explicit requested goal.

Apply does not imply any output capability. Export requires its explicit target
and overwrite intent, and Plan remains device-neutral.

A failed or cancelled output operation follows its owning typed result and
lifecycle semantics; the edit loop does not mutate notebook content merely to
make an unrelated output failure disappear.

An application-level completion projection now marks only successful Render/Plan
projection, committed Export, or recovered prior Export as complete requested
output. Applicable rejection, conflict, cancellation, failure, and stale results
remain incomplete; invalid operation/result pairs remain unclassified.

A partial output-remediation classifier now materializes only five frozen
branches: stale revision reinspection, explicit overwrite choice, Export retry
correction, a new explicit Export after external target drift, and idempotent
Export recovery. Success, cancellation, path/validation rejection, and internal
known-no-effect outcomes retain caller- or diagnostic-dependent next steps.

### Physical-device boundary

Generic autonomous semantic editing stops at device-neutral Plan for physical
workflows unless a separate admitted physical-device contract and operator
boundary explicitly authorizes more.

No number of successful Apply, Render, Export, or Plan iterations accumulates
implicit permission to connect, home, arm, start, pause, resume, cancel, or
safe-stop hardware.

A transport-neutral physical-boundary domain now preserves that no-accumulation
rule across generic workflow history. Empty, repeated, or mixed successful
semantic/output steps still grant no connect, home, arm, start, pause, resume,
cancel, or safe-stop authority.

### Completion

A loop reports completion when the caller's admitted goal is satisfied by the
current accepted revision and any explicitly requested output results.

Completion identifies the final accepted revision and relevant receipts or
output identities. It does not require preserving the full hidden reasoning or
external model conversation as application state.

Stopping because of unresolved evidence, exhausted budget, stable blocking
failure, or unavailable capability is distinct from successful completion.

A transport-neutral terminal-outcome vocabulary now preserves one successful
completion class separately from the four frozen stop classes above. It does not
decide when the goal is satisfied or when a stop condition has been reached.

A transport-neutral completion-admission boundary now emits that successful
terminal class only when already-qualified semantic satisfaction is present and
any explicitly requested output is complete. Goals with no requested output do
not acquire an artificial output requirement.

### Session boundary

Autonomous loop state is disposable with the active application/agent session
unless an explicit caller outside Atrament maintains its own allowed workflow
state.

Atrament does not create a hidden persistent loop journal, command queue, or
credential cache merely to resume autonomous edits after process restart.

A fresh Atrament session begins again from release/capability discovery and
Inspect rather than assuming prior retry or command contexts remain valid.

A transport-neutral session-boundary vocabulary now marks command context,
retry/recovery state, and session admission as invalidated with the session.
Explicit external caller workflow state remains outside Atrament ownership, and
a fresh session starts from capability discovery followed by Inspect.

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

The application-level satisfied-No-op projection now materializes that final
step only when independently qualified semantic/output completion evidence also
admits completion. No-op by itself remains insufficient.

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
