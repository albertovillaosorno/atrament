# Portable core and adapter boundaries

## Status

Accepted.

## Decision ID

`atrament.architecture.portable-core-and-adapter-boundaries`

## Context

The same notebook must be composed interactively, rendered in batch, operated
through agents, exported to files, and written by physical devices. Coupling
document meaning to one interface or hardware protocol would create divergent
behavior and make calibration evidence impossible to reuse.

## Decision

A Rust backend owns semantic documents, handwriting profiles, physical units,
layout decisions, stroke plans, CPU rendering, diagnostics, PDF compilation,
and reproducible output inputs. A TypeScript browser frontend owns interaction,
clipboard intake, direct manipulation, and preview composition.

The frontend connects only to the Rust process on the local loopback interface.
CLI and MCP are additional inbound adapters to the same application services;
PDF, image codecs, transcription processes, and writing machines are outbound
adapters. Device status returns through the owning outbound adapter and never
creates a second application authority.

Adapters may translate capabilities and failures but may not invent document,
layout, or handwriting policy. Python is permitted only behind a process or
protocol boundary when an admitted model integration, such as WhisperX,
requires its ecosystem; it does not become document authority.

Authoritative computation is CPU-only. Atrament does not require CUDA, WebGPU,
or another general-purpose GPU path; ordinary browser compositing does not
change that contract.

## Consequences

- Every interface observes the same notebook semantics.
- Hardware and model dependencies remain replaceable.
- The core can be tested without a graphical session, network, or machine.
- The TypeScript frontend cannot silently reimplement layout or stroke rules.
- Identical accepted inputs and seed produce the same authoritative plans.
- Some integrations require more explicit contracts than direct library calls.

## Rejected Alternatives

- A graphical application as the product authority was rejected because CLI,
  MCP, and batch rendering would duplicate behavior.
- Python as the orchestration authority was rejected because the intended core
  ownership and portable execution model belong in Rust.
- A one-language TypeScript application was rejected because browser state and
  physical rendering would compete for authority and hardware access.
- A GPU-first renderer was rejected because the product must run predictably on
  ordinary computers without a compute-GPU dependency.
- Device-specific commands inside layout code were rejected because motion
  capabilities vary independently from notebook composition.

## Verification

Architecture tests must run the same fixture through interactive, CLI, MCP,
and batch entry points and compare the resulting semantic and render plans.
Dependency checks must prevent adapters from becoming upstream authorities.
