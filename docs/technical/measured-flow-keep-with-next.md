# Measured-flow keep-with-next pagination

## Status

Implemented in `atrament_flow_pagination` and admitted by the
revision-bound `atrament_semantic_flow_pagination` application. It is a
measured-geometry policy, not a Markdown parser, text shaper, or saved
notebook format.

## Purpose

Offer measured semantic notebook compilers a way to keep a heading with
its following content without shaping text or changing accepted revisions.

## Scope

The pure domain receives previously measured fragments and explicit page
regions. The accepted-revision application supplies authorized page bounds.
No new persisted field, CLI option, renderer or browser behavior is created.

## Contract

`FlowUnitPolicy::KeepWithNextWhenPossible` preserves a complete measured
unit on the same page as the next nonempty unit's first fragment when one
remaining page can hold them. Consecutive marked units form a whole-unit
chain. When the next nonempty unit uses `KeepTogetherWhenPossible`, the
entire successor is included in the reservation; otherwise only its first
fragment is required.

The engine first tries the current page remainder, then the earliest later
page whose writable width and height contain the entire reservation.
Empty measured units do not interrupt the chain. If there is no following
nonempty unit, the combined height overflows `u64`, or no remaining page
can contain the complete group, pagination falls back to independent
fragment placement with no speculative page skip. Other fragments still
obey their own policy and remain indivisible.

This is a page-placement preference, never an instruction to silently
clip content or override a physical page budget. Individual fragments
still fail with the existing typed non-fit or exhausted-page errors.
Every emitted fragment retains its exact measured owner, dimensions,
source unit/fragment index, writable rectangle and accepted page identity.
A revision-bound result also preserves its admitted flow and revision.

## Failure Modes

An impossible preference is advisory: each measured fragment still receives
ordinary pagination. Zero-height fragments, invalid page rectangles,
duplicate page identities, page exhaustion and oversized fragments retain
the existing typed errors. Overflowed sums are never rounded or truncated.

## Verification

`tests/backend/flow-pagination/domain/lib.rs` checks exact fits, full
successor chains, skipped narrow sheets, empty and trailing units, safe
fallback, `u64` overflow, a 16,384-unit chain, and 10,000 deterministic
mixed-policy reference cases. The reverse pass precomputes all successor
reservations in linear time in the number of measured fragments.

`tests/backend/semantic-flow-pagination/application/lib.rs` checks
actual accepted notebook page authority: a heading that was previously
orphaned on page one now moves with its next body row to page two without
changing accepted identities or source order.

Neither test proves compiled Markdown headings automatically select this
policy. That requires wiring a measured semantic notebook compiler to the
new option. This does not complete ATR-P2-011's saved two-page Markdown
journey or fixed-obstacle-aware layout requirements.
