# English and Spanish Unicode baseline

## Status

Accepted.

## Decision ID

`atrament.language.english-spanish-unicode-baseline`

## Context

School notes need more than the basic English alphabet. Spanish requires
diacritics and opening punctuation, while both languages use mathematical
symbols, typographic quotations, apostrophes, dashes, and mixed-language names.
Missing characters must not become empty boxes or silent substitutions.

## Decision

The first complete language baseline is English and Spanish with normalized
Unicode text and grapheme-aware editing. Required coverage includes their Latin
letters, uppercase and lowercase diacritics, numerals, punctuation, curly
quotes, guillemets, apostrophes, ellipsis, en dash, em dash, and the admitted
mathematical symbol inventory.

A handwriting profile declares coverage per grapheme or compositional rule.
Missing handwriting coverage is a blocking diagnostic unless the user accepts
a visible, declared fallback style. No renderer may replace an unsupported
character with a visually similar one without consent.

Other languages and scripts are future capability packs with their own shaping,
direction, segmentation, calibration, and verification requirements. Generic
Unicode storage does not constitute verified writing support.

## Consequences

- Spanish punctuation and accents work from the first complete release.
- Editing, measurement, wrapping, and cursor movement operate on graphemes.
- Profiles need explicit punctuation and diacritic calibration.
- Claims about additional scripts remain narrow and evidence-based.

## Rejected Alternatives

- ASCII-only input was rejected because it cannot represent ordinary Spanish.
- Claiming all Unicode scripts from generic text storage was rejected because
  shaping and handwriting synthesis differ by script.
- Silent font fallback was rejected because it would break the personal-writing
  contract.

## Verification

The corpus must cover English and Spanish prose, names, quotations, questions,
exclamations, en and em dashes, combining marks, normalized equivalents, and
mixed mathematics. Every required grapheme must render, measure, wrap, edit,
serialize, and round-trip through CLI and MCP.

### Implementation evidence

The backend now owns one dependency-free visible-text grapheme inventory for the
first English/Spanish handwriting baseline. It freezes all 52 ASCII Latin
letters, the 14 precomposed Spanish diacritic letters `ÁÉÍÑÓÚÜáéíñóúü`, their
14 exact decomposed Unicode equivalents, all ten decimal numerals, and 24 core
prose punctuation forms covering sentence punctuation, Spanish opening
punctuation, straight and curly quotation/apostrophe forms, guillemets,
ellipsis, en/em dash, hyphen, parentheses, and square brackets.

The inventory preserves precomposed and decomposed spellings as distinct exact
graphemes; it does not implement the normalization policy still required by this
ADR. Whitespace/layout separators remain measurement and layout authority rather
than handwriting glyph declarations. Mathematical symbols also remain outside
this inventory because their admitted set is owned by the mathematics authority.
The pinned Unicode segmentation adapter reports all 114 visible-text
requirements as exactly one extended grapheme with exact UTF-8 start/end
boundaries.

A separate cursor-position application resolves those same boundaries to exact
UTF-8 offsets without defining movement, selection, clamping, keybindings, or
normalization; each frozen visible-text grapheme admits only its start and end
positions. The same boundary can resolve a caller-ordered anchor/focus pair
against one validated provider snapshot, preserving forward, reverse, or
collapsed endpoint order without choosing selection-extension behavior. The
active `SessionApplication` can resolve one position against an exact current
text target read-only, with semantic rejection preceding provider access.

The generic handwriting-coverage report also consumes the complete 114-entry
inventory. An empty profile reports every requirement in exact order, while a
profile declaring all 114 exact graphemes reports none.

A language-owned rule matcher maps exactly the 14 decomposed Spanish
requirements
to acute, tilde, or diaeresis composition without normalizing their source. A
profile declaring the other 100 requirements exactly plus those three rules has
complete coverage; arbitrary profile-specific composition remains separate.

A checked-in corpus fixture now supplies exact UTF-8 content for every required
verification scenario plus a 114-token sweep that must equal the visible-text
grapheme inventory in exact order. Its normalized-equivalent fixture keeps both
precomposed and decomposed spellings visible, while its mixed-mathematics text
remains uninterpreted source rather than a mathematical glyph-coverage claim.
Cross-surface render, measure, wrap, edit, serialize, CLI, and MCP verification
remain open.
