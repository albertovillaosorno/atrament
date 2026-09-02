# MCP local session admission contract

## Status

Frozen for first-release local MCP admission semantics.

## Purpose

This contract defines how an MCP inbound adapter can reach one active Atrament
application session without inheriting browser credentials, exposing a remote
service by default, or turning tool discovery into authorization.

It enables unattended local automation while preserving revision, file,
provenance, retry, and physical-safety boundaries owned by the application core.

## Scope

The contract covers local transport posture, session binding, admission
identity, effect-class authorization, credential lifetime, capability discovery,
reconnection, retry recovery, concurrency, provenance, logging, and physical
safety.

It does not freeze stdio versus loopback transport, final authentication fields,
MCP framework choice, tool names, launcher syntax, operating-system IPC, or a
remote multi-user deployment model.

## Contract

### Local-first transport posture

First-release MCP admission is local to the machine and user context running the
Atrament session. A supported transport does not listen on wildcard, LAN,
public, tunnel, or other remotely reachable interfaces merely to make agent
connection convenient.

A process-owned channel such as stdio can satisfy this posture without opening a
network listener. A loopback transport, if chosen, follows the same exact-host
and non-remote principles as the localhost runtime boundary while keeping MCP
admission distinct from browser authentication.

Remote MCP access requires a future explicit security and deployment contract;
it is not implied by this local admission design.

### Separate inbound admission

MCP does not reuse the browser session secret, browser origin proof, copied
model
prompt, command-context identity, command retry identity, or export retry
identity
as generic adapter admission.

The final transport establishes its own non-ambient admission boundary. If that
boundary uses secret material, the secret is unpredictable, ephemeral, scoped to
its owning session/admission, and handled according to the same no-log,
no-persistence expectations applied to other active credentials.

A process channel can derive admission from explicit launcher ownership or
another operating-system boundary when the implementation proves equivalent
isolation without inventing a reusable token.

### One active application session

An admitted MCP connection is bound to one active Atrament application session.
It cannot inspect or mutate another process/session merely because stable
semantic IDs, retry IDs, or notebook-like content happen to match.

Session shutdown invalidates MCP admission and all ephemeral mutation recovery
state owned by that session. A restarted Atrament process is a new application
session even when launched by the same agent.

### Admission identity is not document authority

An MCP admission identity, connection ID, process handle, or token identifies
the
inbound adapter relationship. It is not a notebook revision, semantic object ID,
command-context ID, retry identity, output identity, or accepted document
provenance value.

Application requests still name and validate the authoritative revision and
capability-specific inputs required by their owning contracts.

### Effect-class authorization

Admission can restrict which application effect classes the MCP caller may
invoke. The supported vocabulary can include read-only discovery/Inspect,
command-context generation, Validate, semantic Apply, history traversal, Render,
Export, and device-neutral Plan according to the packaged release.

The live capability snapshot reflects the application capabilities admitted for
that MCP session. An agent does not gain mutation permission by discovering a
schema for a capability excluded by its admission.

Read-only admission is therefore distinguishable from revision mutation or
persistent Export authority. A future implementation can narrow effect classes
without defining a second semantic command model.

### Capability-specific checks remain mandatory

MCP admission only allows a caller to reach an application capability. It does
not waive base revision, command context, writable scope, retry identity,
preconditions, file-path policy, overwrite intent, provenance, output profile,
or plan capability checks.

An admitted Export caller still cannot write arbitrary repository internals. An
admitted Apply caller still cannot mutate outside one backend-owned semantic
command context.

### Tool and schema discovery

Tool names, schemas, result taxonomy, diagnostic codes, and capability metadata
are interface descriptions rather than credentials.

The adapter exposes only the tools or explicit unavailable-state information
admitted by the packaged/live projection. A client does not infer hidden tools
by guessing names or sending arbitrary application method identifiers.

Self-contained release instructions point the agent to live discovery instead of
hard-coding credentials or assuming every designed capability exists.

### Credential handling

When MCP admission uses secret material, it is never written to notebook state,
model prompts, clipboard content, exported output, render manifests, shell
history, public logs, URL query strings, or repository files.

Human-facing errors do not dump raw authorization headers, process-channel
secrets, or environment values used solely for admission.

Documentation and fixtures use synthetic placeholders and cannot serve as live
credentials.

### Connection loss and application outcome

Transport disconnect does not mean a mutation rolled back. If Apply, Undo/Redo,
or Export crossed its owning commit boundary before the connection failed, that
application result remains authoritative.

Unknown-outcome recovery uses the same normalized request and retry identity
only after the caller reconnects to the same still-active application session
through an admitted MCP boundary.

If the Atrament session ended, its retry-result state ended with it. A new
session cannot use an old retry identity as a durable transaction lookup or
credential.

### Reconnection

A transport can support reconnection when its final admission mechanism proves
that the caller has rejoined the same active MCP/application session safely.
Reconnection does not widen effect classes or refresh stale notebook revisions
implicitly.

If safe reattachment cannot be established, the caller creates or joins a new
admitted session and begins from capability discovery and inspection instead of
assuming prior mutation recovery state exists.

### Multiple MCP clients

More than one admitted local MCP connection can exist only if the implementation
preserves application concurrency rules and clear transaction provenance.

Concurrent callers do not receive isolated imaginary copies of the current
revision. Apply and history traversal still serialize through commit-time
revision checks; losing callers receive typed stale/conflict outcomes and
inspect again.

Admission identities can distinguish transaction origins for audit without
becoming semantic notebook authority.

### Transaction provenance

Accepted MCP mutations record MCP transaction provenance through the shared
application history model. Provenance can distinguish separately admitted agent
sessions when useful for local audit without storing their credentials or full
conversation transcripts.

An MCP caller cannot self-label its accepted mutation as an unaided human edit.

### Clipboard independence

Native MCP automation does not require the browser session, system clipboard, or
clipboard permissions. Command context and returned batches use the structured
MCP projection owned by the same application core.

Conversely, browser Copy does not create MCP admission. Possessing command
prompt
text does not authorize an MCP connection.

### Export intent authority

MCP admission for Export defines whether the caller can reach the Export
capability; it does not make notebook or model text an authorized path source.
The explicit target and overwrite intent still come from the admitted caller or
host policy and pass the shared Export boundary.

Read-only or semantic-edit admission cannot manufacture Export authority by
asking a model to return a path inside a command response.

### File and process authority

MCP admission does not grant arbitrary shell execution, repository mutation,
process inspection, environment access, or filesystem authority outside an
explicit application capability.

If a host agent separately has operating-system tools, those tools are outside
the Atrament MCP admission contract. Their existence does not make direct edits
to internal Atrament files equivalent to application operations.

### Physical-device separation

Generic MCP admission for semantic editing, Render, Export, or Plan does not
include physical connect, home, arm, start, pause, resume, cancel, or safe-stop
authority.

A future physical MCP projection requires an explicit separate capability and
operator-safety contract. A device-neutral Plan receipt is not a device token.

### Session shutdown

Orderly Atrament shutdown stops accepting new MCP application operations and
invalidates the session's admission and retry recovery state. In-flight
operations finish or cancel according to their owning application contracts.

The adapter does not write a credential cache or hidden reconnect file to make a
later process appear to be the same session.

## Failure Modes

The contract fails if MCP silently reuses the browser secret, listens remotely
by
default, authenticates through source IP or knowledge of a port, or treats
schema
discovery as mutation permission.

Session isolation fails if a credential from one process can inspect or mutate a
new Atrament session, or if retry state survives shutdown as a durable hidden
transaction database.

Least-authority behavior fails if read-only admission can invoke Apply/Export,
if generic edit admission implies physical motion, or if application-specific
revision/path/scope checks are skipped after adapter admission.

Recovery fails if disconnect is treated as rollback, if same-retry recovery is
attempted against a different session, or if a new retry identity is generated
merely because the prior receipt was lost.

Privacy fails if admission secrets enter prompts, clipboard, exported artifacts,
logs, URL query strings, repository files, or agent instruction bundles.

## Verification

A browser-separation fixture starts one browser session and one MCP admission.
Their credentials or admission mechanisms are different; using the browser
secret as MCP admission and the MCP secret as browser authentication both fail.

A local-transport fixture inspects process sockets or channels and proves the
first-release MCP adapter creates no wildcard, LAN, public, or tunnel listener.

A read-only admission fixture can discover capabilities and Inspect but cannot
Apply, traverse history, or Export. Capability discovery reports the admitted
effect classes without converting tool visibility into permission.

A session-restart fixture records one MCP admission and retry identity, shuts
down Atrament, starts a fresh session, and proves neither value recovers old
state or authorizes the new session.

A lost-receipt fixture commits Apply, drops the MCP transport response, safely
reconnects to the same active session, and recovers exactly one result with the
same retry identity. Repeating that recovery against a restarted session is
refused as unavailable prior-session state.

A concurrency fixture admits two local MCP clients against one session and races
two mutations from the same accepted revision. At most one commits from that
base; the other receives the shared stale/conflict semantics.

A physical-boundary fixture admits semantic Apply, Render, Export, and Plan but
no physical capability. Tool discovery and attempted invocation provide no arm
or start authority.

A credential-scan fixture inspects logs, prompts, clipboard fixtures, exports,
repository files, and packaged agent instructions. No live MCP admission secret
appears in those surfaces.
