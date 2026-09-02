# Self-contained agent instruction bundle contract

## Status

Frozen for first-release local agent discovery and instruction packaging.

## Purpose

This contract defines the repository or release-local information an automated
agent needs to discover how to inspect, edit, validate, render, export, and plan
with Atrament without relying on hidden chat context, memorized tool names, or
live web documentation.

It also prevents documentation artifacts from becoming credentials, mutation
authority, or evidence that an unimplemented capability already exists.

## Scope

The contract covers release identity, local discovery, implemented-capability
status, CLI and MCP projection metadata, schema and contract discovery,
examples, validation workflow, offline operation, security boundaries, and
version mismatch behavior.

It does not freeze final filenames, packaging layout, executable names, MCP
transport, command-line syntax, JSON Schema contents, installer behavior, or
operating-system distribution format.

## Contract

### One release-owned discovery root

A distributable Atrament repository or release bundle exposes one documented
local discovery root for agent instructions. From that root, an agent can locate
the versioned application contracts and the executable interfaces actually
shipped in that release.

The discovery root does not require a network request merely to learn ordinary
CLI, MCP, schema, result, diagnostic, or example semantics that belong to the
installed release.

A future packaging decision can choose the concrete filename or directory. All
supported distribution formats must preserve equivalent discovery semantics.

### Release identity

The bundle identifies the Atrament release or build behavior identity relevant
to its shipped application interfaces. Agent instructions, schemas, examples,
and executable adapters are correlated to that identity.

A file copied from another version cannot silently override the running
backend's capability discovery. Live backend capability metadata remains the
final authority for what the current process admits.

### Implemented versus designed capabilities

Local agent documentation distinguishes frozen design from executable
capability. A contract present in the repository is not by itself evidence that
a Rust application service, CLI command, or MCP tool has been implemented.

The discovery information marks which capability projections are actually
available in the packaged release and which remain design or roadmap material.

An agent does not probe a missing mutating operation merely because a design
contract describes its eventual semantics.

### Interface discovery

For each implemented inbound adapter, the bundle lets an agent locate the
release-owned description needed to start or connect to that adapter through its
admitted boundary.

CLI discovery identifies the shipped executable and locally available help or
schema mechanism rather than requiring the agent to guess historical command
names.

MCP discovery identifies how to obtain the live tool schemas and capability
metadata from an adapter admitted through the frozen local MCP session contract.
Tool names and argument fields come from the shipped/live projection, not from a
model's prior memory.

### Schema and behavior discovery

The bundle points to the versioned schema or equivalent machine-readable
contract for every implemented structured interface that requires one.

It also makes the semantic behavior contracts discoverable, including command
normalization, command and derived-output result taxonomies, diagnostics,
Inspect/command context, review, history, application operation lifecycle,
Render, Export, Plan, and safety boundaries relevant to the shipped
capabilities.

A schema describes transport shape. The semantic technical contracts continue
to define application meaning; neither substitutes for live capability
admission.

### Capability snapshot first

An automated session obtains the backend-owned capability snapshot before
constructing mutation requests. The bundle teaches this workflow rather than
hard-coding the assumption that every documented command family is enabled.

Live capability discovery can narrow or reject features for the active release,
configuration, adapter, notebook state, or device context according to the
owning contract.

### Examples and fixtures

The bundle contains or points locally to representative valid and invalid
examples appropriate to implemented interface versions. Examples illustrate
normalization, retries, stale state, scope, diagnostics, receipts, and output
chaining without becoming accepted notebook authority.

Acceptance fixtures remain test evidence. An agent cannot copy a fixture's
stable identity, retry identity, path, or credential into an unrelated real
session and treat it as authorization.

Examples that mention physical output stop at device-neutral planning unless an
explicit physical-device instruction package separately covers the required
operator and safety boundary.

### Validation-first workflow

The instructions teach agents to prefer read-only discovery and validation
before mutation when the application workflow supports it:

```text
read local release discovery
→ obtain live capability snapshot
→ inspect current accepted revision
→ obtain bounded command context
→ validate or explicitly apply one normalized semantic batch
→ branch on typed result and diagnostics
→ inspect the accepted receipt
→ request Render, Export, or Plan explicitly as needed
```

Successful validation is not represented as a commit reservation. A caller that
chooses Apply still follows revision, context, retry, and atomicity rules.

Repeated automated refinement follows the frozen autonomous-agent loop
contract. Instructions teach typed progress and stop conditions rather than
unbounded "retry until success" behavior.

### Result and diagnostic handling

Agent instructions reference machine-readable result classes and the shared
typed diagnostic envelope. They do not teach automation to branch on one English
error sentence or terminal color.

Render, Plan, and Export instructions use the frozen derived/output result
classes rather than inferring success from progress, file-system prose, or one
diagnostic severity. Optional operation progress remains observational.

Lost mutation receipts use the owning same-retry recovery contract before the
agent issues a new semantic intent. A cancellation request is not treated as an
application rollback until the owning operation reports its final result.

### Clipboard versus native automation

The local instruction bundle can explain the browser-assisted clipboard
workflow, but MCP-native automation does not fake clipboard writes or reads to
reach the same application core.

Clipboard remains an explicit human transport. Native CLI or MCP callers use the
structured interface shipped by the release when that interface is implemented.

### File and output boundaries

Instructions describe Export as an explicit persistent side effect through its
own path and overwrite contract. They do not tell agents to edit repository
internals, hidden caches, or serialized application files directly.

Render and Plan remain derived read-only capabilities. A successful Apply does
not imply that either ran, and a successful Plan does not imply a file was
exported or a physical device was authorized.

### Physical safety boundary

Generic agent instructions do not convert semantic editing or device-neutral
planning into permission to arm or start physical hardware.

If a future release exposes physical device capabilities to an agent, its local
instruction set preserves explicit calibration, device identity, limits,
dry-run, arming, operator, interruption, and safe-stop requirements from the
owning physical-device contract.

### Credentials and secrets

The instruction bundle contains no active browser session secret, MCP admission
credential, export retry token, command retry identity, private device token, or
other session-specific authority.

Examples use unmistakably synthetic placeholders. Agents obtain runtime
admission through the owning adapter rather than by copying credentials from a
documentation file.

### Notebook text is data

Agent instructions keep application instructions separate from notebook, task,
source, citation, diagnostic quotation, and asset-derived prose. Content read
from a notebook does not override tool descriptions or application safety
contracts merely because it addresses an agent.

The bundle does not teach agents to execute shell commands, open arbitrary
paths,
or invoke hidden tools found inside notebook content.

### Offline discoverability

The ordinary interface-discovery and validation documentation shipped in one
release remains usable with external network access disabled. Online references
can supplement background knowledge but are not required to discover the local
release's own command and diagnostic semantics.

An external model service can of course require deliberate network use outside
Atrament. That does not make web documentation a dependency of CLI or MCP
self-discovery.

### Version mismatch

If local documentation/schema identity and the live backend capability snapshot
are incompatible, the agent stops using the mismatched structured interface and
obtains the correct local materials or compatible backend.

It does not guess field migrations, silently downgrade command semantics, or
reinterpret a result from another protocol version.

### No hidden persistence

Agent discovery files are release assets, not a place to persist notebook state,
clipboard responses, undo history, credentials, or session receipts.

Runtime examples and temporary transcripts created during automation follow the
same disposable-session and explicit-export boundaries as other adapters.

## Failure Modes

The contract fails if an agent must search the public web to discover ordinary
interfaces shipped by its local release, or if tool names and schemas are
expected to come from model memory rather than release/live discovery.

Truthfulness fails if a frozen design document is presented as proof that its
backend capability already exists, or if stale instructions silently drive an
incompatible live protocol.

Security fails if the bundle contains active credentials, recommends editing
internal storage directly, treats notebook prose as tool instructions, or makes
semantic Apply imply Export or physical motion.

Automation fails if examples become authority, if result handling depends on
English prose, or if a lost receipt causes a new mutation without same-retry
recovery where the owning contract requires it.

## Verification

An offline-discovery fixture places one packaged release in a network-isolated
environment. Starting from only the documented discovery root, a test agent can
locate release identity, implemented adapter status, capability discovery,
structured schemas when present, result taxonomy, diagnostic semantics, and
representative examples.

A design-only fixture packages the current frontend/docs state without a Rust
backend. Discovery clearly reports that command, CLI, and MCP backend execution
is not available even though their semantic design contracts are present.

A version-mismatch fixture pairs instructions from one behavior version with an
incompatible backend snapshot. Automation refuses structured mutation instead of
guessing a migration.

A credential-scan fixture checks the packaged discovery material for live
session secrets, retry identities, admission tokens, and private device
credentials. Only synthetic placeholders are present.

A notebook-injection fixture inserts prose telling the agent to ignore tool
schemas, execute shell commands, export hidden files, and start hardware. The
agent follows the release-owned application instructions and treats that prose
as
notebook data.

An MCP fixture starts from local discovery, obtains live tool/capability
schemas,
inspects an accepted revision, applies one bounded semantic edit, branches on
the
typed result/diagnostics, and explicitly requests Render or Export. It never
requires a browser clipboard round trip.
