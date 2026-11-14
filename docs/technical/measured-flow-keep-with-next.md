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

### Measured paragraph opening-and-tail balancing (partial) — 2026-11-13

`FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible` is an opt-in
extension of the existing tail-balanced policy. Complete measured units still
move together to the earliest physically valid page when possible. Only a
unit that has already paginated successfully with separate first and second
rows qualifies for the opening-row preference, and it must contain at least
three fragments. This prevents moving an unrelated predecessor's row.

The paginator checks the physical sheet that originally received the second
row. If the first two rows fit its exact height and width, it retries the
entire unchanged unit from that sheet in isolated process-local state. Only a
completely successful retry replaces the original placement and cursor;
otherwise the original successful rows and typed behavior remain intact. The
ordinary lone-final-row guard then runs on the selected placement.

If a subsequently measured block cannot fit after an opening-row move, Rust
first replays the source flow using the established tail-only behavior. A
valid replay wins, so the opening preference cannot itself exhaust the page
budget. A final independent-fragment replay may then rescue a later blocked
row if any remaining keep-group preference spent a page unnecessarily.
If all attempts fail, the original typed failure is preserved.

This can span three or more physical pages, with skipped narrow pages
retained in `covered_pages`. Later measured units start at the final
adjusted cursor.

The deterministic three-page suite independently compares 52,488 integer
fixtures against a separate placement oracle; over 100 cases must actually
move an opening row, while over 100 ineligible cases must retain the earlier
placement. The randomized 10,000-case mixed-policy oracle now exercises all
five policies. Four direct physical-geometry regressions cover paired
opening and trailing moves, whole-flow exhaustion-safe fallback, pen-width
limits, nonsequential page IDs, and skipped narrow sheets. An accepted-revision
three-page regression preserves source indices, semantic block owners,
actual writable rectangles, revision, and the cursor for subsequent blocks.

The policy does not infer Markdown semantics or rewrite a stored notebook.
It neither solves fixed-obstacle geometry nor changes the independent native
CLI's existing paragraph-tail behavior. The full source-to-PDF notebook
journey and automatic policy selection remain open.

### Whole-flow rescue from advisory page skipping (partial) — 2026-11-13

Keep-with-next and keep-together preferences can place an intact measured
unit on a later sheet even when its opening fragment fits the current one.
That valid local preference may strand a later unit on the final physical
page, despite a complete source-ordered layout existing if the group splits.
The pure paginator now retries the **complete** measured flow independently
only after a preferred layout has refused, and after the established tail-only
replay when opening balancing is selected. An already successful preferred
plan is never replaced, and no extra page is invented or newly admitted.

The final replay uses the same checked input page regions and original
measured fragments; each retains its owner, width, height, unit index and
fragment index. It disables all advisory keep/balance moves rather than
mutating accepted semantic policy or silently clipping a row. If the simple
independent greedy placement also fails, the initial typed error is returned.

This is a bounded recovery, **not** an exhaustive globally optimal layout
search. Invalid page/fragment inputs retain their normal typed refusals.

Four direct domain regressions demonstrate a skipped narrow sheet, all
three keep-together variants, unchanged preferred successes and refusal of
an indivisible overlarge final row. A 209,952-case two-page reference sweep
checks both keep-with-next and keep-together policies against independent
fragment placement, requiring real recovered outcomes in both families.
An accepted-revision application regression checks actual 277-mm writable
sheets, semantic owner/source identity, exact physical row tops and sealed
flow/revision authority under a rescued heading group.

This recovery only affects caller-measured vertical flow placement. It does
not connect the CLI Markdown parser, persist semantic paragraphs, resolve
fixed obstacles or produce a complete notebook PDF/PNG Export.

### Mandatory measured page starts (partial) — 2026-11-14

`FlowUnitPolicy::StartOnNewPage` allows an upstream measured semantic block to
request a physical page boundary before its first fragment. If the current
page already contains placed flow, the paginator advances to the next supplied
page; it then places the unit's fragments normally, preserving measurements,
source indices, page identities and actual writable rectangles. A nonempty
unit at the beginning of flow uses the first page without wasting paper, and
an empty unit has no page-break effect. Consecutive nonempty requests each
start a separate physical page.

This boundary is **mandatory**, not a keep-together preference. If the last
sheet already contains flow, the next request refuses with the existing
`NoPageAvailable` error for the first fragment. No fallback replay may erase
an explicit break: advisory keep-together or keep-with-next moves may be
recovered by independent fragment placement, but every requested page start
remains.

A preceding keep-with-next heading cannot reserve across an explicit
boundary. The result includes any skipped physical pages in `covered_pages`
without inventing a sheet or silently clipping content.

Eight focused domain tests cover nonsequential page identities and differing
page widths, empty or initial starts, heading boundaries, typed exhaustion,
both independent and tail-only replay recovery, and a 2,187-case integer
reference sweep. The existing 10,000-case randomized all-policy oracle
exercises this sixth policy as well. An accepted-revision application test
proves exact sheet placement, source ownership and typed refusal on actual
accepted page-profile authority.

This exposes a measured layout policy only. The independently implemented
native CLI Markdown flow now admits a standalone export-only page-break
directive, but does not use this semantic `FlowUnitPolicy` or persist page
breaks in notebook state. Neither slice completes the saved-notebook layout
journey.
