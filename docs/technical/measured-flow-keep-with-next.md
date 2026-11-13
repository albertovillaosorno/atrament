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

### Measured paragraph-tail balancing (partial) — 2026-11-13

`FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible` retains the existing
complete-group preference. If the entire unit fits a current or later
physical page, it moves there without splitting. If the unit is too tall,
ordinary per-fragment pagination runs first, preserving each row's measured
physical height, width, owner and source index.

When a split leaves **exactly one final row** at the beginning of a fresh
physical page, the paginator may move the penultimate row onto that sheet.
It does so only if at least three rows of **this same unit** finished on the
preceding physical page, and the two final rows both fit the actual last
page's height and width. The original page then retains at least two rows.
This also works across an intervening page too narrow to accept the row;
skipped pages remain in `covered_pages` in original sheet order.

A three-row split of two plus one is not rebalanced to one plus two. A later
page unable to contain both rows or the penultimate row's width leaves the
original one-row tail unchanged. The preference never creates a new page,
rewrites source identities, or relaxes typed page exhaustion and geometry
refusals.

Subsequent measured units use the adjusted last-page cursor, not
its pre-balance height. Keep-with-next headings reserve the successor's full
extent under this policy, just as with ordinary keep-together units.

Five focused domain regressions exercise full and fallback placement, mixed
preceding/following units, narrow and skipped pages, non-sorted accepted
page IDs and unbalanced refusal. A separate **6,561-case exhaustive
small-integer oracle** requires actual balance outcomes; the existing
10,000-case randomized mixed-policy oracle now includes the fourth policy
and up to five measured fragments per unit. A revision-bound semantic
application test proves accepted page identities, immutable row sources,
exact writable geometry, and unchanged flow/revision ownership.

This is an explicit policy for **already measured** semantic paragraphs,
not automatic Markdown tagging. The native CLI's independently implemented
wrapped-tail guard is not replaced; accepted notebook content measurement,
fixed-object resolution and native notebook PDF/PNG remain incomplete.
