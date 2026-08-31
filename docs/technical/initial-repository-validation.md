# Initial repository validation

## Status

Verified on 2026-08-31 against the tracked initial authority and current `main`.

## Purpose

This record closes the initial repository-validation contract with reproducible
evidence. It identifies the first tracked authority commit, required local hook,
validator identity, and commands that establish a clean planning-generation
baseline before Rust or TypeScript source policy is activated.

## Scope

The evidence applies to the repository design generation: root governance,
ADRs, technical contracts, Jig configuration, Git attributes, and the required
commit-message hook. Rust and TypeScript language gates are intentionally not
claimed here because the workspace scaffold remains a separate P0 task.

The root authority commit is:

```text
400ea870d08e8a4868c637f619de61e36a299a44
```

Its subject is:

```text
docs(repo): establish Atrament design contract
```

That commit changed the repository from an unborn `main` branch with untracked
design files into a tracked authority that Jig could validate canonically.

## Contract

### Tracked authorities

The initial commit tracks the repository-level authorities required to interpret
the design generation, including:

```text
.editorconfig
.gitattributes
.gitignore
.jig/jig.toml
.jig/taxonomy.json
LICENSE-MIT
README.md
TODO.md
docs/adr/index.yml
docs/technical/index.yml
```

The remaining `.jig/` settings, spelling projection, validator version pin, and
all accepted ADR records are tracked in the same commit. There is no policy
exception that makes an untracked root authority appear canonical.

### Commit hook

The configured required hook is `commit-msg`. The repository-local hook exists
at `.git/hooks/commit-msg`, is executable, and delegates to:

```text
jig commit-message --root <repository-root> --file <message-file>
```

The hook is active evidence rather than a nominal configuration entry. During
the initial commit, it rejected an overlong commit-message body; a wrapped
message was then accepted without bypassing the hook.

### Validator identity

The clean checks in this record use Jig `26.3.0` with configuration schema 21.
The validator identity reported by `jig doctor` is:

```text
build revision:
70a105f4977eb47b87f1539528117c02a31d3f92

validator SHA-256:
358009854c48f83f3aba1e3fea9bca6f4d36a9b02e3e6bb2725d0f6fd1d11218
```

Jig is executed from the user's external installation. `jig doctor` therefore
reports `degraded` because this repository does not contain a Jig development
or production installation and does not yet have a compiled graph cache. That
status is informational for this planning generation: configuration parsing is
clean and `jig check` completes successfully.

### Clean exhaustive validation

From the repository root, the canonical validation commands are:

```text
jig doctor --root /home/albertovillaosorno/Developer/atrament
jig check --root /home/albertovillaosorno/Developer/atrament
```

On 2026-08-31, `jig check` returned:

```text
jig check: clean
validator SHA-256:
358009854c48f83f3aba1e3fea9bca6f4d36a9b02e3e6bb2725d0f6fd1d11218
```

The check includes configured evidence for Git bootstrap, working copy, hooks,
CSpell, snapshot state, native validation, linters, planned work, exact units,
and hexagonal-source policy before executing the registered rule set.

### Planning-generation boundary

A clean planning-generation check does not pre-approve implementation source.
The first Rust or TypeScript scaffold must activate its language, formatter,
linter, package, source-layout, and native-validation policy before that P0 task
can close.

This distinction prevents the initial clean result from becoming an exception
for unfinished implementation. New source must satisfy the policy applicable to
that source rather than inheriting the design-only applicability decisions.

## Failure Modes

This baseline is invalid if a required root authority becomes untracked, the
commit hook is removed or bypassed, `.gitattributes` stops being canonical, Jig
configuration no longer parses, or `jig check` fails on the committed tree.

It is also invalid to cite this record as evidence that future Rust, TypeScript,
transport generation, rendering, or hardware gates passed. Those gates must be
activated and validated when their corresponding surfaces are introduced.

The `jig doctor` degraded installation status must not be misreported as a clean
doctor state. Conversely, it is not a substitute for a failing `jig check`:
canonical repository validation must remain clean independently.

## Verification

Reproduce the tracked-authority evidence with:

```text
git show --no-patch --format='%H %s' 400ea87

git ls-files --error-unmatch \
  .editorconfig .gitattributes .gitignore \
  .jig/jig.toml .jig/taxonomy.json \
  LICENSE-MIT README.md TODO.md \
  docs/adr/index.yml docs/technical/index.yml
```

Verify the hook without modifying a commit:

```text
test -x .git/hooks/commit-msg
grep -F 'jig commit-message' .git/hooks/commit-msg
```

Then run the canonical repository gates:

```text
jig doctor --root /home/albertovillaosorno/Developer/atrament
jig check --root /home/albertovillaosorno/Developer/atrament
git diff --check
```

The acceptance condition is a tracked authority set, executable required hook,
parseable Jig configuration, clean `jig check`, and no whitespace errors. The
working tree must contain no unexplained validation artifacts outside the
repository-owned runtime roots.
