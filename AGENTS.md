# Agent instructions

## Mandatory Jig-governed Rust formatting

**Prohibited:** bare `rustfmt`, `cargo fmt`, `cargo +nightly fmt`, or
`rustup run nightly rustfmt` without the explicit Jig-owned configuration.
Host-default formatting can rewrite hundreds of files and violate Jig's
canonical source and text contracts.

For Rust files, read `.jig/settings/tools.toml` and select the Jig-approved
pinned formatter.

Run it only with `--config-path .jig/lang/rust/.rustfmt.toml` (or the exact
admitted `.jig/lang/rust/rustfmt.toml`). With Cargo, pass that flag after `--`.

Do not substitute host defaults, inline `--config` overrides, or another
repository's profile. If no Jig format authority exists, do not format.

Format only files explicitly owned for the task. Inspect the exact Git diff,
verify the configured formatter before changing files, preserve concurrent
code and tests, and undo only proven formatting-only damage. Never run a
blind workspace-wide formatter. Jig's `.jig` settings are the sole authority;
a shell PATH guard is additional protection, not a substitute for validation.

Follow shared `agents` repository instructions when applicable.
