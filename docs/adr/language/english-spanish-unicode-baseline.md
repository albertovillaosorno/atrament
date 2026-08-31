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
