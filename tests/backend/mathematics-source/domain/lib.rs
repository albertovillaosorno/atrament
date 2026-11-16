// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Regression evidence for exact-source mathematical structure analysis.
// - Must-Not:
//   - Test glyph rendering, measurement, layout, or silent source rewriting.
// - Allows:
//   - Inputs: Deterministic Unicode and TeX-compatible mathematical source.
//   - Outputs: Assertions over tokens, support state, and typed syntax
//     failures.
//   - Side effects: Process-local test allocations only.
// - Split-When:
//   - Mathematical expression semantics receive an independent fixture corpus.
// - Merge-When:
//   - Math parsing becomes fully covered by semantic-notebook acceptance tests.
// - Summary:
//   - Proves admitted math structure preserves every authored source byte.
// - Description:
//   - Covers Unicode, fractions, scripts, units, alignment, matrix, and errors.
// - Usage:
//   - Compile directly against the dependency-free mathematics-source domain.
// - Defaults:
//   - Unknown TeX commands are unsupported data, not syntax substitutions.
//

//! Mathematics source parser regression tests.

use std::fmt;

use atrament_mathematics_source::{
    FormulaMode, MathSyntaxError, MathSyntaxErrorKind, MathTokenKind,
    SupportedCommand, analyze,
};

trait RequiredMathFixture {
    type Output;

    fn required(self, context: &str) -> Result<Self::Output, String>;
}

impl<T, E: fmt::Debug> RequiredMathFixture for Result<T, E> {
    type Output = T;

    fn required(self, context: &str) -> Result<T, String> {
        self.map_err(|error| format!("{context}: {error:?}"))
    }
}

impl<T> RequiredMathFixture for Option<T> {
    type Output = T;

    fn required(self, context: &str) -> Result<T, String> {
        self.ok_or_else(|| context.to_owned())
    }
}

fn reconstructed(
    analyzed: &atrament_mathematics_source::AnalyzedFormula,
) -> String {
    analyzed
        .tokens
        .iter()
        .map(|token| analyzed.token_source(*token))
        .collect::<Option<Vec<_>>>()
        .map_or_else(
            || "<missing source token>".to_owned(),
            |pieces| pieces.concat(),
        )
}

fn assert_analysis_invariants(
    analyzed: &atrament_mathematics_source::AnalyzedFormula,
    source: &str,
    mode: FormulaMode,
) {
    assert_eq!(
        analyzed.source, source,
        "analysis must borrow the exact source"
    );
    assert_eq!(
        analyzed.mode, mode,
        "analysis must retain the chosen math mode"
    );
    assert_eq!(
        analyzed.is_supported(),
        analyzed.unsupported.is_empty(),
        "support status must agree with unsupported spans"
    );

    let mut cursor = 0usize;
    for token in &analyzed.tokens {
        assert_eq!(token.start, cursor, "token gap or overlap: {source:?}");
        assert!(token.start < token.end, "empty token: {source:?}");
        assert!(
            source.is_char_boundary(token.start),
            "token boundary must preserve UTF-8: {source:?}"
        );
        assert!(
            source.is_char_boundary(token.end),
            "token boundary must preserve UTF-8: {source:?}"
        );
        cursor = token.end;
    }
    assert_eq!(
        cursor,
        source.len(),
        "incomplete token coverage: {source:?}"
    );
    assert_eq!(
        reconstructed(analyzed),
        source,
        "source reconstruction must remain lossless"
    );

    let mut unsupported_end = 0usize;
    for item in &analyzed.unsupported {
        assert!(
            item.start >= unsupported_end,
            "unsupported order: {source:?}"
        );
        assert!(item.start < item.end, "empty unsupported span: {source:?}");
        assert_eq!(
            source.get(item.start..item.end),
            Some(item.name.as_str()),
            "unsupported source mismatch: {source:?}",
        );
        assert!(
            analyzed.tokens.iter().any(|token| {
                token.start == item.start
                    && token.end == item.end
                    && token.kind == MathTokenKind::Literal
            }),
            "unsupported token must match exact source: {source:?}",
        );
        unsupported_end = item.end;
    }
}

#[test]
fn small_mixed_sources_preserve_analysis_invariants() {
    const FRAGMENTS: &[&str] = &[
        "a",
        "\u{f1}",
        "\u{1f642}",
        "{",
        "}",
        "&",
        r"\\",
        r"\unknown",
        r"\acute",
        r"\sqrt[3]",
        r"\begin{gathered}",
        r"\end{gathered}",
    ];

    for first in FRAGMENTS {
        for second in FRAGMENTS {
            for third in FRAGMENTS {
                for fourth in FRAGMENTS {
                    let source = format!("{first}{second}{third}{fourth}");
                    for mode in [
                        FormulaMode::Inline,
                        FormulaMode::Display,
                        FormulaMode::Aligned,
                    ] {
                        if let Ok(analyzed) = analyze(&source, mode) {
                            assert_analysis_invariants(
                                &analyzed, &source, mode,
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "exhaustive deterministic TeX fragment and syntax-class matrix"
)]
fn generated_tex_corpus_is_deterministic_and_source_safe() {
    let check = || -> Result<(), String> {
        type Kind = MathSyntaxErrorKind;
        const FRAGMENTS: &[&str] = &[
            "a",
            "0",
            "+",
            "_",
            "^",
            " ",
            "\t",
            "\n",
            "\0",
            "\u{f1}",
            "\u{3b2}",
            "\u{1f642}",
            "{",
            "}",
            "[",
            "]",
            "&",
            r"\",
            r"\\",
            r"\alpha",
            r"\Doteq",
            r"\P",
            r"\S",
            r"\dag",
            r"\ddag",
            r"\doublecap",
            r"\doublecup",
            r"\frac",
            r"\gggtr",
            r"\llless",
            r"\mathsterling",
            r"\pounds",
            r"\restriction",
            r"\sqrt",
            r"\sqrt[",
            r"\unknown",
            r"\@",
            r"\begin{matrix}",
            r"\end{matrix}",
            r"\begin{cases}",
            r"\end{cases}",
            r"\operatorname*",
            r"\text",
            r"\left",
        ];

        const FRAGMENT_COUNT: usize = FRAGMENTS.len();
        let mut state = 0x9e37_79b9u32;
        let mut seen_fragment_counts = [false; 32];
        let mut seen_fragments = [false; FRAGMENT_COUNT];
        let mut seen_adjacent_pairs = [false; FRAGMENT_COUNT * FRAGMENT_COUNT];
        let mut seen_mode_success = [false; 3];
        let mut seen_mode_error = [false; 3];
        let mut seen_syntax_classes = [false; 9];
        let mut saw_supported_success = false;
        let mut saw_unsupported_success = false;
        for case_index in 0..4_096u32 {
            state = state
                .wrapping_mul(1_664_525)
                .wrapping_add(0x3c6e_f35f ^ case_index);
            let fragment_count = usize::try_from((state >> 27u32) + 1)
                .required("bounded fragment count")?;
            *seen_fragment_counts
                .get_mut(fragment_count - 1)
                .required("corpus counter within bounds")? = true;
            let mut source = String::new();
            let mut previous_fragment = None;
            for fragment_index in 0..fragment_count {
                let fragment_number = u32::try_from(fragment_index)
                    .required("bounded fragment index")?;
                state = state
                    .wrapping_mul(1_664_525)
                    .wrapping_add(0x3c6e_f35f ^ fragment_number);
                let index = usize::try_from(state >> 16u32)
                    .required("u16 fits usize on supported targets")?
                    .checked_rem(FRAGMENT_COUNT)
                    .required("nonempty fragment corpus")?;
                *seen_fragments
                    .get_mut(index)
                    .required("corpus counter within bounds")? = true;
                if let Some(previous) = previous_fragment {
                    *seen_adjacent_pairs
                        .get_mut(previous * FRAGMENT_COUNT + index)
                        .required("corpus counter within bounds")? = true;
                }
                previous_fragment = Some(index);
                source.push_str(
                    FRAGMENTS.get(index).required("known corpus fragment")?,
                );
            }

            for (mode_index, mode) in [
                FormulaMode::Inline,
                FormulaMode::Display,
                FormulaMode::Aligned,
            ]
            .into_iter()
            .enumerate()
            {
                let first = analyze(&source, mode);
                let second = analyze(&source, mode);
                assert_eq!(
                    first, second,
                    "nondeterministic case {case_index}: {source:?}",
                );
                match first {
                    Ok(analyzed) => {
                        *seen_mode_success
                            .get_mut(mode_index)
                            .required("corpus counter within bounds")? = true;
                        if analyzed.is_supported() {
                            saw_supported_success = true;
                        } else {
                            saw_unsupported_success = true;
                        }
                        assert_analysis_invariants(&analyzed, &source, mode);
                    },
                    Err(error) => {
                        *seen_mode_error
                            .get_mut(mode_index)
                            .required("corpus counter within bounds")? = true;
                        let class_index = match error.kind {
                            Kind::AlignmentOutsideStructure => Some(0),
                            Kind::ExtraCasesEnd => Some(1),
                            Kind::ExtraGroupClose => Some(2),
                            Kind::ExtraMatrixEnd => Some(3),
                            Kind::MissingCasesEnd => Some(4),
                            Kind::MissingMatrixEnd => Some(5),
                            Kind::MissingRequiredGroup => Some(6),
                            Kind::MissingRootIndexEnd => Some(7),
                            Kind::UnclosedGroup => Some(8),
                            Kind::EnvironmentClosesInsideGroup
                            | Kind::EnvironmentCrossesGroupClose
                            | Kind::ExtraAlignedEnd
                            | Kind::ExtraBracedMatrixEnd
                            | Kind::ExtraBracketedMatrixEnd
                            | Kind::ExtraDoubleVerticalMatrixEnd
                            | Kind::ExtraGatheredEnd
                            | Kind::ExtraParenthesizedMatrixEnd
                            | Kind::ExtraSmallMatrixEnd
                            | Kind::ExtraSplitEnd
                            | Kind::ExtraVerticalMatrixEnd
                            | Kind::MismatchedEnvironmentEnd
                            | Kind::MissingAlignedEnd
                            | Kind::MissingBracedMatrixEnd
                            | Kind::MissingBracketedMatrixEnd
                            | Kind::MissingDoubleVerticalMatrixEnd
                            | Kind::MissingGatheredEnd
                            | Kind::MissingParenthesizedMatrixEnd
                            | Kind::MissingSmallMatrixEnd
                            | Kind::MissingSplitEnd
                            | Kind::MissingVerticalMatrixEnd
                            | Kind::UnescapedSpecialCharacter => None,
                        };
                        if let Some(selected_class_index) = class_index {
                            *seen_syntax_classes
                                .get_mut(selected_class_index)
                                .required("corpus counter within bounds")? =
                                true;
                        }
                        assert!(
                            error.byte_offset <= source.len(),
                            "bad offset {case_index}: {source:?}",
                        );
                        assert!(
                            source.is_char_boundary(error.byte_offset),
                            "non-UTF-8 boundary {case_index}: {source:?}",
                        );
                    },
                }
            }
        }
        assert!(seen_fragment_counts.into_iter().all(|seen| seen));
        assert!(seen_fragments.into_iter().all(|seen| seen));
        assert!(seen_adjacent_pairs.into_iter().all(|seen| seen));
        assert!(seen_mode_success.into_iter().all(|seen| seen));
        assert!(seen_mode_error.into_iter().all(|seen| seen));
        assert!(seen_syntax_classes.into_iter().all(|seen| seen));
        assert!(saw_supported_success);
        assert!(saw_unsupported_success);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn named_and_control_symbol_mixes_preserve_analysis_invariants() {
    const FRAGMENTS: &[&str] = &[
        "\u{f1}",
        "{",
        "}",
        r"\intop",
        r"\mathdollar",
        r"\not",
        r"\|",
        r"\unknown",
    ];

    for first in FRAGMENTS {
        for second in FRAGMENTS {
            for third in FRAGMENTS {
                let source = format!("{first}{second}{third}");
                for mode in [
                    FormulaMode::Inline,
                    FormulaMode::Display,
                    FormulaMode::Aligned,
                ] {
                    if let Ok(analyzed) = analyze(&source, mode) {
                        assert_analysis_invariants(&analyzed, &source, mode);
                    }
                }
            }
        }
    }
}

#[test]
fn long_unknown_control_word_is_one_exact_unsupported_span() {
    let check = || -> Result<(), String> {
        let source = format!(r"\{}", "a".repeat(100_000));
        let analyzed = analyze(&source, FormulaMode::Inline)
            .required("long balanced unknown control word")?;
        assert_analysis_invariants(&analyzed, &source, FormulaMode::Inline);
        assert!(!analyzed.is_supported());
        assert_eq!(analyzed.tokens.len(), 1);
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .start,
            0
        );
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .end,
            source.len()
        );
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            source
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn unicode_school_formula_is_preserved_byte_for_byte() {
    let check = || -> Result<(), String> {
        let source = "y\u{2032} = 5(3x\u{b2} + 1)\u{2074} \u{b7} 6x";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("unicode formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(analyzed.source, source);
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.tokens.len(), 1);
        assert_eq!(
            analyzed
                .tokens
                .first()
                .required("first analyzed entry")?
                .kind,
            MathTokenKind::Literal
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn escaped_braces_are_supported_without_corrupting_group_indexing() {
    let check = || -> Result<(), String> {
        let source = r"\frac{\{x\}}{2}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("escaped braces formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::EscapedSpecial,
                        )
                })
                .count(),
            2,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn escaped_tex_specials_remain_literal_in_math_and_text() {
    let check = || -> Result<(), String> {
        let source = r"\{x \& y\} = 50\% \#1 + a\_b + \$5 + \text{A \& B}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("escaped specials formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::EscapedSpecial,
                        )
                })
                .count(),
            8,
        );
        assert!(
            !analyzed
                .tokens
                .iter()
                .any(|token| token.kind == MathTokenKind::AlignmentPoint)
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "complete named-operator table is an exhaustive fixture"
)]
fn named_operator_vocabulary_is_supported_without_rewriting() {
    let check = || -> Result<(), String> {
        for candidate_source in [
            r"\Pr",
            r"\arccos",
            r"\arcsin",
            r"\arctan",
            r"\arg",
            r"\bigcap",
            r"\bigcup",
            r"\bigodot",
            r"\bigoplus",
            r"\bigotimes",
            r"\bigsqcup",
            r"\biguplus",
            r"\bigvee",
            r"\bigwedge",
            r"\bmod",
            r"\coprod",
            r"\cos",
            r"\cosh",
            r"\cot",
            r"\coth",
            r"\csc",
            r"\deg",
            r"\det",
            r"\dim",
            r"\exp",
            r"\gcd",
            r"\hom",
            r"\iiiint",
            r"\iiint",
            r"\iint",
            r"\inf",
            r"\injlim",
            r"\int",
            r"\intop",
            r"\ker",
            r"\lg",
            r"\lim",
            r"\liminf",
            r"\limsup",
            r"\ln",
            r"\log",
            r"\max",
            r"\min",
            r"\mod",
            r"\oint",
            r"\ointop",
            r"\prod",
            r"\projlim",
            r"\sec",
            r"\sin",
            r"\sinh",
            r"\sum",
            r"\sup",
            r"\tan",
            r"\tanh",
            r"\varinjlim",
            r"\varliminf",
            r"\varlimsup",
            r"\varprojlim",
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Inline)
                .required("named operator formula")?;
            assert!(analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(
                analyzed
                    .tokens
                    .iter()
                    .filter(|token| {
                        token.kind
                            == MathTokenKind::Command(
                                SupportedCommand::NamedOperator,
                            )
                    })
                    .count(),
                1,
                "{candidate_source}",
            );

            for suffix in ['A', 'Z', 'x'] {
                let longer = format!("{candidate_source}{suffix}");
                let prefixed = analyze(&longer, FormulaMode::Inline)
                    .required("balanced longer named operator")?;
                assert!(!prefixed.is_supported(), "{longer}");
                assert_eq!(reconstructed(&prefixed), longer);
                assert_eq!(prefixed.unsupported.len(), 1, "{longer}");
                assert_eq!(
                    prefixed
                        .unsupported
                        .first()
                        .required("first analyzed entry")?
                        .name,
                    longer,
                    "{longer}"
                );
            }
        }

        let prefixed = analyze(r"\sinewave", FormulaMode::Inline)
            .required("balanced unsupported operator prefix")?;
        assert!(!prefixed.is_supported());
        assert_eq!(
            prefixed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\sinewave"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn named_operators_compose_with_symbols_and_groups() {
    let check = || -> Result<(), String> {
        let source = r"\sin(\theta) + \log(x) \le \max{y}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("operator expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            3,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn extended_named_operators_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\det(A) + \gcd(a,b) + \ker(T) + \dim(V); ",
            r"\sinh(x) + \cosh(x) + \tanh(x); ",
            r"\liminf a_n \le \limsup a_n; \Pr(A)",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("extended named operator expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            10,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn direct_inverse_and_decorated_limit_operators_preserve_source() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\injlim_i A_i + \projlim_j B_j; ",
            r"\varinjlim_i C_i + \varprojlim_j D_j; ",
            r"\varliminf_n a_n + \varlimsup_n b_n",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("direct, inverse, and decorated limit operators")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            6,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn modular_arithmetic_commands_preserve_structure() {
    let check = || -> Result<(), String> {
        let source = r"a \bmod m; b \mod n; x \pmod{17}; y \pod{23}";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("modular arithmetic expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            2,
        );
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::ParenthesizedModulo)
                && analyzed.token_source(*token) == Some(r"\pmod")
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(
                    SupportedCommand::ParenthesizedModuloOperand,
                )
                && analyzed.token_source(*token) == Some(r"\pod")
        }));
        for spelling in [r"\pmod", r"\pod"] {
            assert_eq!(
                analyze(spelling, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: spelling.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{spelling}",
            );
        }
        for (candidate_source, unsupported) in [
            (r"\pmodulus{17}", r"\pmodulus"),
            (r"\podcast{23}", r"\podcast"),
        ] {
            let prefixed = analyze(candidate_source, FormulaMode::Inline)
                .required("balanced unsupported modular prefix")?;
            assert!(!prefixed.is_supported(), "{candidate_source}");
            assert_eq!(
                prefixed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn punctuation_and_integral_atoms_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\intop_0^1 f(x) dx; \ointop_C F ds; ",
            r"x \colon y \ldotp z \cdotp w",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("punctuation and integral-operator atom expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            3,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn multiple_integral_and_large_set_operators_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\iint_D f dA + \iiint_V \rho dV + \oint_C F \cdot dr; ",
            r"\bigcup_i A_i \supseteq \bigcap_i A_i",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("multiple integral and large-set operator expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            5,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn reverse_implication_and_fourfold_integral_preserve_source() {
    let check = || -> Result<(), String> {
        let source = r"P \impliedby Q; \iiiint_A f(w,x,y,z) dw dx dy dz";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("reverse implication and fourfold integral expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                && analyzed.token_source(*token) == Some(r"\impliedby")
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::NamedOperator)
                && analyzed.token_source(*token) == Some(r"\iiiint")
        }));
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn calculus_commands_compose_with_scripts_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\sum_{i=1}^n i + \prod_{k=1}^m k + ",
            r"\int_0^1 x dx + \lim_{x \to 0} x",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("calculus expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            4,
        );

        let supported = r"\partial f + \nabla g";
        let other_analysis_1 = analyze(supported, FormulaMode::Inline)
            .required("calculus symbol expression")?;
        assert!(other_analysis_1.is_supported());
        assert_eq!(reconstructed(&other_analysis_1), supported);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "complete named-symbol table is an exhaustive fixture"
)]
fn named_symbol_vocabulary_is_supported_without_rewriting() {
    let check = || -> Result<(), String> {
        for candidate_source in [
            r"\Bbbk",
            r"\Box",
            r"\Bumpeq",
            r"\Cap",
            r"\Cup",
            r"\Delta",
            r"\Diamond",
            r"\Doteq",
            r"\Downarrow",
            r"\Finv",
            r"\Game",
            r"\Gamma",
            r"\Im",
            r"\Join",
            r"\Lambda",
            r"\Leftarrow",
            r"\Leftrightarrow",
            r"\Lleftarrow",
            r"\Longleftarrow",
            r"\Longleftrightarrow",
            r"\Longrightarrow",
            r"\Lsh",
            r"\Omega",
            r"\P",
            r"\Phi",
            r"\Pi",
            r"\Psi",
            r"\Re",
            r"\Rightarrow",
            r"\Rrightarrow",
            r"\Rsh",
            r"\S",
            r"\Sigma",
            r"\Subset",
            r"\Supset",
            r"\Theta",
            r"\Uparrow",
            r"\Updownarrow",
            r"\Upsilon",
            r"\Vdash",
            r"\Vert",
            r"\Vvdash",
            r"\Xi",
            r"\aleph",
            r"\alpha",
            r"\amalg",
            r"\angle",
            r"\approx",
            r"\approxeq",
            r"\ast",
            r"\asymp",
            r"\backepsilon",
            r"\backprime",
            r"\backsim",
            r"\backsimeq",
            r"\backslash",
            r"\barwedge",
            r"\because",
            r"\beta",
            r"\beth",
            r"\between",
            r"\bigcirc",
            r"\bigstar",
            r"\bigtriangledown",
            r"\bigtriangleup",
            r"\blacklozenge",
            r"\blacksquare",
            r"\blacktriangle",
            r"\blacktriangledown",
            r"\blacktriangleleft",
            r"\blacktriangleright",
            r"\bot",
            r"\bowtie",
            r"\boxdot",
            r"\boxminus",
            r"\boxplus",
            r"\boxtimes",
            r"\bullet",
            r"\bumpeq",
            r"\cap",
            r"\cdot",
            r"\cdotp",
            r"\cdots",
            r"\centerdot",
            r"\checkmark",
            r"\chi",
            r"\circ",
            r"\circeq",
            r"\circlearrowleft",
            r"\circlearrowright",
            r"\circledR",
            r"\circledS",
            r"\circledast",
            r"\circledcirc",
            r"\circleddash",
            r"\clubsuit",
            r"\colon",
            r"\complement",
            r"\cong",
            r"\cup",
            r"\curlyeqprec",
            r"\curlyeqsucc",
            r"\curlyvee",
            r"\curlywedge",
            r"\curvearrowleft",
            r"\curvearrowright",
            r"\dag",
            r"\dagger",
            r"\daleth",
            r"\dashv",
            r"\ddag",
            r"\ddagger",
            r"\ddots",
            r"\delta",
            r"\diagdown",
            r"\diagup",
            r"\diamond",
            r"\diamondsuit",
            r"\digamma",
            r"\div",
            r"\divideontimes",
            r"\doteq",
            r"\doteqdot",
            r"\dotplus",
            r"\dots",
            r"\doublebarwedge",
            r"\doublecap",
            r"\doublecup",
            r"\downarrow",
            r"\downdownarrows",
            r"\downharpoonleft",
            r"\downharpoonright",
            r"\ell",
            r"\emptyset",
            r"\epsilon",
            r"\eqcirc",
            r"\eqsim",
            r"\eqslantgtr",
            r"\eqslantless",
            r"\equiv",
            r"\eta",
            r"\eth",
            r"\exists",
            r"\fallingdotseq",
            r"\flat",
            r"\forall",
            r"\frown",
            r"\gamma",
            r"\ge",
            r"\geq",
            r"\geqq",
            r"\geqslant",
            r"\gets",
            r"\gg",
            r"\ggg",
            r"\gggtr",
            r"\gimel",
            r"\gnapprox",
            r"\gneq",
            r"\gneqq",
            r"\gnsim",
            r"\gtrapprox",
            r"\gtrdot",
            r"\gtreqless",
            r"\gtreqqless",
            r"\gtrless",
            r"\gtrsim",
            r"\gvertneqq",
            r"\hbar",
            r"\heartsuit",
            r"\hookleftarrow",
            r"\hookrightarrow",
            r"\hslash",
            r"\iff",
            r"\imath",
            r"\impliedby",
            r"\implies",
            r"\in",
            r"\infty",
            r"\intercal",
            r"\iota",
            r"\jmath",
            r"\kappa",
            r"\lVert",
            r"\lambda",
            r"\land",
            r"\langle",
            r"\lbrace",
            r"\lbrack",
            r"\lceil",
            r"\ldotp",
            r"\ldots",
            r"\le",
            r"\leadsto",
            r"\leftarrow",
            r"\leftarrowtail",
            r"\leftharpoondown",
            r"\leftharpoonup",
            r"\leftleftarrows",
            r"\leftrightarrow",
            r"\leftrightarrows",
            r"\leftrightharpoons",
            r"\leftrightsquigarrow",
            r"\leftthreetimes",
            r"\leq",
            r"\leqq",
            r"\leqslant",
            r"\lessapprox",
            r"\lessdot",
            r"\lesseqgtr",
            r"\lesseqqgtr",
            r"\lessgtr",
            r"\lesssim",
            r"\lfloor",
            r"\lhd",
            r"\ll",
            r"\llcorner",
            r"\lll",
            r"\llless",
            r"\lnapprox",
            r"\lneq",
            r"\lneqq",
            r"\lnot",
            r"\lnsim",
            r"\longleftarrow",
            r"\longleftrightarrow",
            r"\longmapsto",
            r"\longrightarrow",
            r"\looparrowleft",
            r"\looparrowright",
            r"\lor",
            r"\lozenge",
            r"\lrcorner",
            r"\ltimes",
            r"\lvert",
            r"\lvertneqq",
            r"\maltese",
            r"\mapsto",
            r"\mathdollar",
            r"\mathellipsis",
            r"\mathparagraph",
            r"\mathsection",
            r"\mathsterling",
            r"\measuredangle",
            r"\mho",
            r"\mid",
            r"\models",
            r"\mp",
            r"\mu",
            r"\multimap",
            r"\nLeftarrow",
            r"\nLeftrightarrow",
            r"\nRightarrow",
            r"\nVDash",
            r"\nVdash",
            r"\nabla",
            r"\natural",
            r"\ncong",
            r"\ne",
            r"\nearrow",
            r"\neg",
            r"\neq",
            r"\nexists",
            r"\ngeq",
            r"\ngeqq",
            r"\ngeqslant",
            r"\ngtr",
            r"\ni",
            r"\nleftarrow",
            r"\nleftrightarrow",
            r"\nleq",
            r"\nleqq",
            r"\nleqslant",
            r"\nless",
            r"\nmid",
            r"\notin",
            r"\nparallel",
            r"\nprec",
            r"\npreceq",
            r"\nrightarrow",
            r"\nshortmid",
            r"\nshortparallel",
            r"\nsim",
            r"\nsubseteq",
            r"\nsubseteqq",
            r"\nsucc",
            r"\nsucceq",
            r"\nsupseteq",
            r"\nsupseteqq",
            r"\ntriangleleft",
            r"\ntrianglelefteq",
            r"\ntriangleright",
            r"\ntrianglerighteq",
            r"\nu",
            r"\nvDash",
            r"\nvdash",
            r"\nwarrow",
            r"\odot",
            r"\omega",
            r"\ominus",
            r"\oplus",
            r"\oslash",
            r"\otimes",
            r"\owns",
            r"\parallel",
            r"\partial",
            r"\perp",
            r"\phi",
            r"\pi",
            r"\pitchfork",
            r"\pm",
            r"\pounds",
            r"\prec",
            r"\precapprox",
            r"\preccurlyeq",
            r"\preceq",
            r"\precnapprox",
            r"\precneqq",
            r"\precnsim",
            r"\precsim",
            r"\prime",
            r"\propto",
            r"\psi",
            r"\rVert",
            r"\rangle",
            r"\rbrace",
            r"\rbrack",
            r"\rceil",
            r"\restriction",
            r"\rfloor",
            r"\rhd",
            r"\rho",
            r"\rightarrow",
            r"\rightarrowtail",
            r"\rightharpoondown",
            r"\rightharpoonup",
            r"\rightleftarrows",
            r"\rightleftharpoons",
            r"\rightrightarrows",
            r"\rightsquigarrow",
            r"\rightthreetimes",
            r"\risingdotseq",
            r"\rtimes",
            r"\rvert",
            r"\searrow",
            r"\setminus",
            r"\sharp",
            r"\shortmid",
            r"\shortparallel",
            r"\sigma",
            r"\sim",
            r"\simeq",
            r"\smallfrown",
            r"\smallsetminus",
            r"\smallsmile",
            r"\smile",
            r"\spadesuit",
            r"\sphericalangle",
            r"\sqcap",
            r"\sqcup",
            r"\sqsubseteq",
            r"\sqsupseteq",
            r"\square",
            r"\star",
            r"\subset",
            r"\subseteq",
            r"\subseteqq",
            r"\subsetneq",
            r"\subsetneqq",
            r"\succ",
            r"\succapprox",
            r"\succcurlyeq",
            r"\succeq",
            r"\succnapprox",
            r"\succneqq",
            r"\succnsim",
            r"\succsim",
            r"\supset",
            r"\supseteq",
            r"\supseteqq",
            r"\supsetneq",
            r"\supsetneqq",
            r"\surd",
            r"\swarrow",
            r"\tau",
            r"\therefore",
            r"\theta",
            r"\thickapprox",
            r"\thicksim",
            r"\times",
            r"\to",
            r"\top",
            r"\triangle",
            r"\triangledown",
            r"\triangleleft",
            r"\trianglelefteq",
            r"\triangleq",
            r"\triangleright",
            r"\trianglerighteq",
            r"\twoheadleftarrow",
            r"\twoheadrightarrow",
            r"\ulcorner",
            r"\unlhd",
            r"\unrhd",
            r"\uparrow",
            r"\updownarrow",
            r"\upharpoonleft",
            r"\upharpoonright",
            r"\uplus",
            r"\upsilon",
            r"\upuparrows",
            r"\urcorner",
            r"\vDash",
            r"\varDelta",
            r"\varGamma",
            r"\varLambda",
            r"\varOmega",
            r"\varPhi",
            r"\varPi",
            r"\varPsi",
            r"\varSigma",
            r"\varTheta",
            r"\varUpsilon",
            r"\varXi",
            r"\varbigtriangledown",
            r"\varbigtriangleup",
            r"\varepsilon",
            r"\varkappa",
            r"\varnothing",
            r"\varphi",
            r"\varpi",
            r"\varpropto",
            r"\varrho",
            r"\varsigma",
            r"\varsubsetneq",
            r"\varsubsetneqq",
            r"\varsupsetneq",
            r"\varsupsetneqq",
            r"\vartheta",
            r"\vartriangle",
            r"\vartriangleleft",
            r"\vartriangleright",
            r"\vdash",
            r"\vdots",
            r"\vee",
            r"\veebar",
            r"\vert",
            r"\wedge",
            r"\wp",
            r"\wr",
            r"\xi",
            r"\yen",
            r"\zeta",
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Inline)
                .required("named symbol formula")?;
            assert!(analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(
                analyzed
                    .tokens
                    .iter()
                    .filter(|token| {
                        token.kind
                            == MathTokenKind::Command(
                                SupportedCommand::NamedSymbol,
                            )
                    })
                    .count(),
                1,
                "{candidate_source}",
            );

            for suffix in ['A', 'Z', 'x'] {
                let longer = format!("{candidate_source}{suffix}");
                let prefixed = analyze(&longer, FormulaMode::Inline)
                    .required("balanced longer named symbol")?;
                assert!(!prefixed.is_supported(), "{longer}");
                assert_eq!(reconstructed(&prefixed), longer);
                assert_eq!(prefixed.unsupported.len(), 1, "{longer}");
                assert_eq!(
                    prefixed
                        .unsupported
                        .first()
                        .required("first analyzed entry")?
                        .name,
                    longer,
                    "{longer}"
                );
            }
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn latexsym_join_and_triangle_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \Join B; C \lhd D; E \unlhd F; ",
            r"G \rhd H; I \unrhd J",
        );
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("latexsym join and triangle expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            5,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn latex_base_ordinary_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\varbigtriangleup A + \varbigtriangledown B; ",
            r"\mathsection + \mathparagraph + \mathdollar + \mathsterling + ",
            r"\pounds",
        );
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("LaTeX-base ordinary symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            7,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn latex_base_math_mode_glyph_aliases_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = r"\P + \S + \dag + \ddag";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("LaTeX-base math-mode glyph aliases")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            4,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn nary_card_and_music_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\coprod_i A_i + \biguplus_j B_j + \bigvee_k P_k + ",
            r"\bigwedge_l Q_l; \clubsuit + \diamondsuit + \heartsuit + ",
            r"\spadesuit; \flat + \natural + \sharp; \lnot P; x \owns A",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("n-ary, card, and music symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            4,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            9,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn extended_binary_and_relation_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\bigodot_i A_i + \bigoplus_j B_j + \bigotimes_k C_k + ",
            r"\bigsqcup_l D_l; a \amalg b, a \asymp b, \bigcirc x; ",
            r"\bigtriangledown + \bigtriangleup + a \bowtie b; ",
            r"\dagger + \ddagger + a \doteq b + x \odot y + x \ominus y + ",
            r"x \oslash y + A \sqcap B + A \sqcup B + A \sqsubseteq B + ",
            r"A \sqsupseteq B + a \triangleleft b + a \triangleright b + ",
            r"A \uplus B + a \wr b",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("extended binary and relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::NamedOperator,
                        )
                })
                .count(),
            4,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            20,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn surd_symbol_composes_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = r"\surd + x";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("surd symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                && analyzed.token_source(*token) == Some(r"\surd")
        }));
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn foundational_named_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\aleph_0 + \wp(A); \top, \bot; ",
            r"x \smile y, x \frown y; \diamond P",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("foundational named-symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            7,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn complex_and_prime_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = r"z = \Re(w) + i\Im(w); f^\prime(x)";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("complex and prime symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            3,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn latex_base_math_ellipsis_alias_preserves_exact_source() {
    let check = || -> Result<(), String> {
        let source = r"a_1, a_2, \mathellipsis, a_n";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("LaTeX-base fixed math ellipsis alias")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                && analyzed.token_source(*token) == Some(r"\mathellipsis")
        }));
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn geometry_and_proof_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\angle ABC \simeq \triangle DEF; ",
            r"P \implies Q \iff R; \because x \nexists A \therefore y; ",
            r"v \vdash P, M \models Q; \ell + \hbar + \imath + \jmath",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("geometry and proof symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            14,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ellipsis_symbols_compose_in_matrix_notation_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{matrix}a_1 & \cdots & a_n \\ ",
            r"\vdots & \ddots & \vdots \\ ",
            r"b_1 & \dots & b_n\end{matrix}; x \ldots y",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("ellipsis matrix expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            6,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn harpoon_and_mapped_arrows_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \leftharpoonup B \leftharpoondown C; ",
            r"D \rightharpoonup E \rightharpoondown F; ",
            r"G \rightleftharpoons H; x \longmapsto y; p \leadsto q",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("harpoon and mapped-arrow expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            7,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_positive_arrow_relations_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \circlearrowleft B \circlearrowright C; ",
            r"D \curvearrowleft E \curvearrowright F; ",
            r"G \twoheadleftarrow H \twoheadrightarrow I; ",
            r"J \leftleftarrows K \rightrightarrows L; ",
            r"M \upuparrows N \downdownarrows O; ",
            r"P \upharpoonleft Q \upharpoonright R; ",
            r"S \downharpoonleft T \downharpoonright U; ",
            r"V \leftarrowtail W \rightarrowtail X; ",
            r"Y \leftrightarrows Z \rightleftarrows A; ",
            r"B \leftrightharpoons C; D \Lsh E \Rsh F; ",
            r"G \rightsquigarrow H \leftrightsquigarrow I; ",
            r"J \looparrowleft K \looparrowright L; ",
            r"M \multimap N; O \Lleftarrow P \Rrightarrow Q",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS positive arrow relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            28,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn negated_ams_arrow_relations_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \nleftarrow B \nrightarrow C; ",
            r"D \nLeftarrow E \nRightarrow F; ",
            r"G \nleftrightarrow H \nLeftrightarrow I",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("negated AMS arrow relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            6,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn common_arrow_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \Leftarrow B \leftrightarrow C \Longrightarrow D; ",
            r"x \mapsto y; u \uparrow v \downarrow w; ",
            r"p \nearrow q \searrow r; a \hookrightarrow b",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("arrow symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            9,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn layout_and_style_commands_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (r"\phantom{x}", r"\phantom"),
            (r"\hspace{1em}", r"\hspace"),
            (r"\color{red}{x}", r"\color"),
            (r"\mathunderscore", r"\mathunderscore"),
            (r"\skew{3}{\hat{x}}{y}", r"\skew"),
            (r"x\enspace y", r"\enspace"),
            (r"x\enskip y", r"\enskip"),
            (r"x\quad y", r"\quad"),
            (r"x\qquad y", r"\qquad"),
            (r"x\thinspace y", r"\thinspace"),
            (r"x\negthinspace y", r"\negthinspace"),
            (r"x\medspace y", r"\medspace"),
            (r"x\negmedspace y", r"\negmedspace"),
            (r"x\thickspace y", r"\thickspace"),
            (r"x\negthickspace y", r"\negthickspace"),
            (r"x\,y", r"\,"),
            (r"x\!y", r"\!"),
            (r"x\:y", r"\:"),
            (r"x\;y", r"\;"),
            (r"x\ y", r"\ "),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Inline)
                .required("balanced layout-affecting candidate_source")?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn context_sensitive_math_commands_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for candidate_source in [
            r"\displaylimits",
            r"\dotsb",
            r"\dotsc",
            r"\dotsi",
            r"\dotsm",
            r"\dotso",
            r"\idotsint",
            r"\limits",
            r"\nolimits",
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required(
                    "balanced context-sensitive mathematical candidate_source",
                )?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                candidate_source,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn equation_reference_and_display_controls_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (concat!(r"\eqref{", "eq:energy}"), r"\eqref"),
            (r"\tag{energy}", r"\tag"),
            (r"\notag", r"\notag"),
            (r"\nonumber", r"\nonumber"),
            (r"\intertext{therefore}", r"\intertext"),
            (r"\displaybreak[2]", r"\displaybreak"),
            (r"\allowdisplaybreaks[3]", r"\allowdisplaybreaks"),
            (r"\raisetag{2pt}", r"\raisetag"),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required(
                    "balanced equation-reference or display-control source",
                )?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn layout_sensitive_amsmath_commands_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (r"\cfrac[l]{1}{2}", r"\cfrac"),
            (r"\genfrac{}{}{}{}{n}{2}", r"\genfrac"),
            (r"\leftroot{-2}", r"\leftroot"),
            (r"\uproot{2}", r"\uproot"),
            (r"\xleftarrow[below]{above}", r"\xleftarrow"),
            (r"\xrightarrow{above}", r"\xrightarrow"),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required(
                    "balanced layout-sensitive amsmath candidate_source",
                )?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn primitive_tex_fraction_and_root_controls_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (r"x \over y", vec![r"\over"]),
            (r"x \atop y", vec![r"\atop"]),
            (r"x \above 1pt y", vec![r"\above"]),
            (r"x \overwithdelims() y", vec![r"\overwithdelims"]),
            (r"x \atopwithdelims[] y", vec![r"\atopwithdelims"]),
            (r"x \abovewithdelims() 1pt y", vec![r"\abovewithdelims"]),
            (r"\root 3 \of{x}", vec![r"\root", r"\of"]),
            (r"\buildrel{a}\over{=}", vec![r"\buildrel", r"\over"]),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required(
                    "balanced primitive TeX structural candidate_source",
                )?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(
                analyzed
                    .unsupported
                    .iter()
                    .map(|item| item.name.as_str())
                    .collect::<Vec<_>>(),
                unsupported,
                "{candidate_source}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn advanced_amsmath_layout_controls_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (r"x\mspace{5mu}y", r"\mspace"),
            (r"x\nobreakdash-y", r"\nobreakdash"),
            (r"\overunderset{a}{b}{=}", r"\overunderset"),
            (r"\sideset{_a}{^b}{\sum}", r"\sideset"),
            (r"\smash[b]{x}", r"\smash"),
            (r"\hdotsfor{3}", r"\hdotsfor"),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required(
                    "balanced advanced amsmath layout candidate_source",
                )?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn latex_base_construction_atoms_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for candidate_source in [
            r"\braceld",
            r"\bracerd",
            r"\bracelu",
            r"\braceru",
            r"\downbracefill",
            r"\joinrel",
            r"\leftarrowfill",
            r"\lhook",
            r"\mapstochar",
            r"\relbar",
            r"\Relbar",
            r"\rhook",
            r"\rightarrowfill",
            r"\upbracefill",
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Inline)
                .required("balanced LaTeX-base construction atom")?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                candidate_source,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn structural_and_size_sensitive_commands_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (r"a \not= b", r"\not"),
            (r"\smallint_0^1 f(x) dx", r"\smallint"),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required(
                    "balanced structural or size-sensitive candidate_source",
                )?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn extensible_delimiter_components_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for candidate_source in [
            r"\Arrowvert",
            r"\arrowvert",
            r"\bracevert",
            r"\lgroup",
            r"\lmoustache",
            r"\rgroup",
            r"\rmoustache",
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Inline)
                .required("balanced extensible delimiter component")?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                candidate_source,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn sized_delimiter_commands_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        let source = r"\left\langle x \right\rangle";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("balanced paired sized-delimiter source")?;
        assert!(!analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .unsupported
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            [r"\left", r"\right"],
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            2,
        );

        for (candidate_source, unsupported) in [
            (r"\big(x)", r"\big"),
            (r"\Big[x]", r"\Big"),
            (r"\bigg\langle x\rangle", r"\bigg"),
            (r"\Bigg\lbrace x\rbrace", r"\Bigg"),
        ] {
            let other_analysis_1 =
                analyze(candidate_source, FormulaMode::Display).required(
                    "balanced fixed sized-delimiter candidate_source",
                )?;
            assert!(!other_analysis_1.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&other_analysis_1), candidate_source);
            assert_eq!(
                other_analysis_1.unsupported.len(),
                1,
                "{candidate_source}"
            );
            assert_eq!(
                other_analysis_1
                    .unsupported
                    .first()
                    .required("first other_analysis_1 entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn named_delimiters_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\langle x, y \rangle; \lceil x \rceil; ",
            r"\lfloor y \rfloor; \lbrace A \rbrace; ",
            r"\lbrack z \rbrack; A \backslash B; ",
            r"\vert x \vert + \Vert v \Vert; ",
            r"\lvert a \rvert + \lVert w \rVert",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("named delimiter expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            19,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_corner_delimiters_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source =
            concat!(r"\ulcorner A \urcorner + ", r"\llcorner B \lrcorner",);
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS corner delimiter expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            4,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_geometric_ordinary_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\square + \blacksquare + \lozenge + \blacklozenge; ",
            r"\bigstar + \blacktriangle + \blacktriangledown + \triangledown; ",
            concat!(
                r"\diagup + \diagdown + \measuredangle + ",
                r"\sphericalangle + \circledS"
            ),
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS geometric ordinary symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            13,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_binary_operator_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\Cap + \Cup + \barwedge + \boxdot + \boxminus + \boxplus + ",
            r"\boxtimes + \centerdot + \circledast + \circledcirc + ",
            r"\circleddash + ",
            r"\curlyvee + \curlywedge + \divideontimes + \dotplus + ",
            r"\doublebarwedge + \gtrdot + \intercal + \leftthreetimes + ",
            r"\lessdot + \ltimes + \rightthreetimes + \rtimes + ",
            r"\smallsetminus + \veebar",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS binary operator symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            25,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn common_binary_operator_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"a \ast b + c \bullet d + e \circ f; ",
            r"x \div y \mp z; A \oplus B \otimes C; ",
            r"S \setminus T; p \star q; P \vee Q \wedge R",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("binary operator symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            11,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_special_symbols_and_large_aliases_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\Box + \Diamond + \checkmark + ",
            r"\circledR + \maltese + \yen",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS special-symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            6,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_symbol_aliases_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\restriction + \Doteq + \doublecup + \doublecap + ",
            r"\llless + \gggtr",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS symbol-alias expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            6,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_ordinary_and_letter_like_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\Bbbk + \Finv + \Game + \backprime + \beth + \complement + ",
            r"\daleth + \digamma + \eth + \gimel + \hslash + \mho + ",
            r"\varkappa + \varnothing",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS ordinary and letter-like symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            14,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn variant_greek_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\varepsilon + \varphi + \vartheta + ",
            r"\varrho + \varsigma + \varpi",
        );
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("variant Greek symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            6,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn negated_ams_order_and_equality_relations_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"a \nleq b \ngeq c \nless d \ngtr e; ",
            r"f \nleqq g \ngeqq h \nleqslant i \ngeqslant j; ",
            r"k \lneq l \gneq m \lneqq n \gneqq o; ",
            r"p \lvertneqq q \gvertneqq r; ",
            r"s \nprec t \nsucc u \npreceq v \nsucceq w; ",
            r"x \precneqq y \succneqq z; ",
            r"a \precnsim b \succnsim c; ",
            r"d \precnapprox e \succnapprox f; ",
            r"g \lnsim h \gnsim i \lnapprox j \gnapprox k; ",
            r"l \nsim m \ncong n",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("negated AMS order and equality relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            30,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_order_and_equality_relations_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"a \circeq b \doteqdot c \eqcirc d \eqsim e; ",
            r"p \preccurlyeq q \precsim r \precapprox s; ",
            r"u \succcurlyeq v \succsim w \succapprox x; ",
            r"a \leqq b \lessgtr c \lessapprox d \lesseqgtr e \lesseqqgtr f; ",
            r"g \geqq h \gtrless i \gtrapprox j \gtreqless k \gtreqqless l; ",
            r"m \eqslantless n \eqslantgtr o; ",
            r"p \curlyeqprec q \curlyeqsucc r; ",
            r"s \risingdotseq t \fallingdotseq u; ",
            r"v \bumpeq w \Bumpeq x \triangleq y; ",
            r"z \backsim a \backsimeq b \thicksim c \thickapprox d \approxeq e",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS order and equality relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            34,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn negated_ams_structural_relations_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \nmid B \nparallel C; D \nshortmid E \nshortparallel F; ",
            r"G \nvdash H \nVdash I \nvDash J \nVDash K; ",
            r"L \ntriangleleft M \ntriangleright N; ",
            r"O \ntrianglelefteq P \ntrianglerighteq Q",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("negated AMS structural relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            12,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn remaining_positive_ams_relations_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \Vdash B \Vvdash C \vDash D; E \backepsilon F; ",
            r"G \between H \pitchfork I; J \varpropto K; ",
            r"L \shortmid M \shortparallel N; ",
            r"O \smallsmile P \smallfrown Q; R \lll S \ggg T",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("remaining positive AMS relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            13,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn common_relation_and_logic_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"a \equiv b \sim c \cong d; x \propto y; ",
            r"u \perp v, p \parallel q, a \mid b; ",
            r"P \land Q \lor \neg R",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("relation and logic expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            10,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn italic_capital_greek_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\varGamma + \varDelta + \varTheta + \varLambda + \varXi + ",
            r"\varPi + \varSigma + \varUpsilon + \varPhi + \varPsi + \varOmega",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("italic capital Greek symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            11,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn negated_ams_containment_relations_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \nsubseteq B \nsubseteqq C; D \nsupseteq E \nsupseteqq F; ",
            r"G \subsetneqq H \supsetneqq I; ",
            r"J \varsubsetneq K \varsupsetneq L; ",
            r"M \varsubsetneqq N \varsupsetneqq O",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("negated AMS containment relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            10,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ams_triangle_and_containment_relations_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A \Subset B \subseteqq C; D \Supset E \supseteqq F; ",
            r"G \vartriangleleft H \trianglelefteq I; ",
            r"J \vartriangleright K \trianglerighteq L; ",
            r"M \blacktriangleleft N \blacktriangleright O; P \vartriangle Q",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("AMS triangle and containment relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            11,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn order_and_set_relation_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"a \ll b \leqslant c \lesssim d; x \gg y \geqslant z \gtrsim w; ",
            r"p \prec q \preceq r; s \succ t \succeq u; ",
            r"A \subsetneq B, B \supsetneq A, x \ni y; P \dashv Q",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("order and set relation expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            14,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn standard_greek_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\Delta x = \Sigma_i \Gamma_i + \Omega; ",
            r"\eta + \iota + \kappa + \nu + \xi + \tau + ",
            r"\upsilon + \chi + \psi + \zeta",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("standard Greek symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            14,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn set_logic_and_arrow_symbols_compose_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\forall x \in A \cup B, x \notin \emptyset \Rightarrow ",
            r"A \subseteq B \Leftrightarrow B \supseteq A",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("set and logic expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            9,
        );

        let arrows = r"a \leftarrow b \rightarrow c \to d";
        let other_analysis_1 = analyze(arrows, FormulaMode::Inline)
            .required("arrow expression")?;
        assert!(other_analysis_1.is_supported());
        assert_eq!(reconstructed(&other_analysis_1), arrows);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn named_symbols_compose_with_scripts_and_relations() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"A = \pi r^2; \theta_1 \le \phi \pm \infty; ",
            r"a \times b \approx c \cdot d",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            9,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::Superscript)
                .count(),
            1,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::Subscript)
                .count(),
            1,
        );

        let prefixed = analyze(r"\pioneer", FormulaMode::Inline)
            .required("balanced unsupported symbol prefix")?;
        assert!(!prefixed.is_supported());
        assert_eq!(
            prefixed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\pioneer"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn admitted_control_words_never_match_longer_ascii_names() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (r"\alephs", r"\alephs"),
            (r"\angles", r"\angles"),
            (r"\bigodots", r"\bigodots"),
            (r"\biguplusx", r"\biguplusx"),
            (r"\cdotpx", r"\cdotpx"),
            (r"\binomial{n}{k}", r"\binomial"),
            (r"\bmodulo", r"\bmodulo"),
            (r"\clubsuits", r"\clubsuits"),
            (r"\colonial", r"\colonial"),
            (r"\coproduct", r"\coproduct"),
            (r"\diamondsuited", r"\diamondsuited"),
            (r"\iints", r"\iints"),
            (r"\intops", r"\intops"),
            (r"\impliesx", r"\impliesx"),
            (r"\infinite", r"\infinite"),
            (r"\integral_0^1", r"\integral"),
            (r"\leftharpoonups", r"\leftharpoonups"),
            (r"\leqslanted", r"\leqslanted"),
            (r"\ldotpx", r"\ldotpx"),
            (r"\longmapstox", r"\longmapstox"),
            (r"\mathdollars", r"\mathdollars"),
            (r"\mathparagraphs", r"\mathparagraphs"),
            (r"\mathsections", r"\mathsections"),
            (r"\nablax", r"\nablax"),
            (r"\naturally", r"\naturally"),
            (r"\ointops", r"\ointops"),
            (r"\overlined{x}", r"\overlined"),
            (r"\pmodulus{17}", r"\pmodulus"),
            (r"\rightleftharpoonsx", r"\rightleftharpoonsx"),
            (r"\Rightarrowed", r"\Rightarrowed"),
            (r"\sqsubseteqx", r"\sqsubseteqx"),
            (r"\subsetequal", r"\subsetequal"),
            (r"\thereforex", r"\thereforex"),
            (r"\varbigtriangledowns", r"\varbigtriangledowns"),
            (r"\varbigtriangleups", r"\varbigtriangleups"),
            (r"\vectored{x}", r"\vectored"),
            (r"\emptysets", r"\emptysets"),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Inline)
                .required("balanced longer control word")?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn package_only_strict_square_relations_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (r"A \sqsubset B", r"\sqsubset"),
            (r"A \sqsupset B", r"\sqsupset"),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Inline)
                .required("balanced package-only square relation")?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }

        let admitted = r"A \sqsubseteq B; C \sqsupseteq D";
        let other_analysis_1 = analyze(admitted, FormulaMode::Inline)
            .required("base square relations")?;
        assert!(other_analysis_1.is_supported());
        assert_eq!(reconstructed(&other_analysis_1), admitted);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn parallel_control_symbol_is_supported_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = r"a \| b + c \parallel d";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("parallel control-symbol expression")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                })
                .count(),
            2,
        );
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                && analyzed.token_source(*token) == Some(r"\|")
        }));
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn unsupported_control_symbols_preserve_utf8_and_trailing_slash() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in
            [("x \\\u{1f642} y", "\\\u{1f642}"), (r"z \", r"\")]
        {
            let analyzed = analyze(candidate_source, FormulaMode::Inline)
                .required("balanced unsupported control symbol")?;
            assert!(!analyzed.is_supported(), "{candidate_source:?}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source:?}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source:?}"
            );
            assert_eq!(
                candidate_source.get(
                    analyzed
                        .unsupported
                        .first()
                        .required("first analyzed entry")?
                        .start
                        ..analyzed
                            .unsupported
                            .first()
                            .required("first analyzed entry")?
                            .end
                ),
                Some(unsupported),
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn unknown_control_symbol_stays_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        let source = r"x \@ y";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("balanced unknown control symbol")?;
        assert!(!analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\@"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn required_groups_allow_ascii_whitespace_without_rewriting() {
    let check = || -> Result<(), String> {
        for candidate_source in [
            "\\frac \t{1}  {2}",
            "\\binom\n{n}\r\n{k}",
            "\\sqrt \t{x}",
            "\\operatorname  {rank}",
            "\\operatorname* \t{argmax}",
            "\\pmod \t{17}",
            "\\pod \t{23}",
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required("whitespace-separated required groups")?;
            assert!(analyzed.is_supported(), "{candidate_source:?}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn common_accents_are_structural_and_require_one_group() {
    let check = || -> Result<(), String> {
        let source = r"\bar{x} + \hat{p} + \dot{x} + \ddot{x} + \tilde{y}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("common accent formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\bar", SupportedCommand::Bar),
            (r"\hat", SupportedCommand::Hat),
            (r"\dot", SupportedCommand::Dot),
            (r"\ddot", SupportedCommand::DoubleDot),
            (r"\tilde", SupportedCommand::Tilde),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
            assert_eq!(
                analyze(spelling, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: spelling.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{spelling}",
            );
        }

        let prefixed = analyze(r"\hatted{x}", FormulaMode::Inline)
            .required("balanced longer accent command")?;
        assert!(!prefixed.is_supported());
        assert_eq!(
            prefixed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\hatted"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn extended_accents_are_structural_and_require_one_group() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\acute{x}+\breve{y}+\check{z}+",
            r"\ddddot{q}+\dddot{r}+\grave{a}+\mathring{b}",
        );
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("extended grouped accents")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\acute", SupportedCommand::Acute),
            (r"\breve", SupportedCommand::Breve),
            (r"\check", SupportedCommand::Check),
            (r"\ddddot", SupportedCommand::QuadrupleDot),
            (r"\dddot", SupportedCommand::TripleDot),
            (r"\grave", SupportedCommand::Grave),
            (r"\mathring", SupportedCommand::MathRing),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
            assert_eq!(
                analyze(spelling, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: spelling.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{spelling}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn wide_accents_are_structural_and_require_one_group() {
    let check = || -> Result<(), String> {
        let source = r"\widehat{ABC}+\widetilde{xyz}";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("grouped wide accents")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\widehat", SupportedCommand::WideHat),
            (r"\widetilde", SupportedCommand::WideTilde),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
            assert_eq!(
                analyze(spelling, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: spelling.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{spelling}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn grouped_math_alphabets_are_structural_and_require_one_group() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"x \in \mathbb{R}, \mathbf{v}, \mathcal{F}, \mathit{x}, ",
            r"\mathfrak{g}, \mathsf{A}, \mathtt{id}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("grouped mathematical alphabets")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\mathbb", SupportedCommand::BlackboardBold),
            (r"\mathbf", SupportedCommand::Bold),
            (r"\mathcal", SupportedCommand::Calligraphic),
            (r"\mathfrak", SupportedCommand::Fraktur),
            (r"\mathit", SupportedCommand::Italic),
            (r"\mathsf", SupportedCommand::SansSerif),
            (r"\mathtt", SupportedCommand::Typewriter),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
            assert_eq!(
                analyze(spelling, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: spelling.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{spelling}",
            );
        }

        let prefixed = analyze(r"\mathbboard{R}", FormulaMode::Inline)
            .required("balanced longer math-alphabet command")?;
        assert!(!prefixed.is_supported());
        assert_eq!(
            prefixed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\mathbboard"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn custom_operator_name_is_structural_and_requires_one_group() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\operatorname{Var}(X) + ",
            r"\operatorname*{argmax}_{x \in A} f(x)",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("custom operator formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::OperatorName,
                        )
                })
                .count(),
            1,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::OperatorNameWithLimits,
                        )
                })
                .count(),
            1,
        );
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(
                    SupportedCommand::OperatorNameWithLimits,
                )
                && analyzed.token_source(*token) == Some(r"\operatorname*")
        }));
        for spelling in [r"\operatorname", r"\operatorname*"] {
            assert_eq!(
                analyze(spelling, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: spelling.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{spelling}",
            );
        }

        let separated = analyze(r"\operatorname *{rank}", FormulaMode::Inline);
        assert_eq!(
            separated,
            Err(MathSyntaxError {
                byte_offset: r"\operatorname ".len(),
                kind: MathSyntaxErrorKind::MissingRequiredGroup,
            }),
        );

        let prefixed = analyze(r"\operatornamed{rank}", FormulaMode::Inline)
            .required("balanced longer operator-name command")?;
        assert!(!prefixed.is_supported());
        assert_eq!(
            prefixed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\operatornamed"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn directional_over_arrows_are_structural_and_require_one_group() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\overleftarrow{AB}+\overleftrightarrow{CD}+",
            r"\overrightarrow{EF}+\underleftarrow{GH}+",
            r"\underleftrightarrow{IJ}+\underrightarrow{KL}",
        );
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("grouped directional over- and under-arrows")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\overleftarrow", SupportedCommand::OverLeftArrow),
            (r"\overleftrightarrow", SupportedCommand::OverLeftRightArrow),
            (r"\overrightarrow", SupportedCommand::OverRightArrow),
            (r"\underleftarrow", SupportedCommand::UnderLeftArrow),
            (
                r"\underleftrightarrow",
                SupportedCommand::UnderLeftRightArrow,
            ),
            (r"\underrightarrow", SupportedCommand::UnderRightArrow),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
            assert_eq!(
                analyze(spelling, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: spelling.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{spelling}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn boxed_and_brace_decorations_preserve_grouped_structure() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\boxed{x+1} + \overbrace{a+b}^{sum} + ",
            r"\underbrace{x+y}_{pair}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("boxed and brace decoration formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\boxed", SupportedCommand::Boxed),
            (r"\overbrace", SupportedCommand::Overbrace),
            (r"\underbrace", SupportedCommand::Underbrace),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
            assert_eq!(
                analyze(spelling, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: spelling.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{spelling}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn grouped_decorations_are_structural_and_require_one_group() {
    let check = || -> Result<(), String> {
        let source = r"\vec{v} + \overline{AB} + \underline{x_1}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("grouped decoration formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\vec", SupportedCommand::Vector),
            (r"\overline", SupportedCommand::Overline),
            (r"\underline", SupportedCommand::Underline),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
        }
        for (candidate_source, byte_offset) in
            [(r"\vec", 4), (r"\overline", 9), (r"\underline", 10)]
        {
            assert_eq!(
                analyze(candidate_source, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset,
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{candidate_source}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn stacked_relation_is_structural_and_requires_two_groups() {
    let check = || -> Result<(), String> {
        let source = r"a\stackrel{!}{=}b";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("stacked relation annotation")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::StackRelation)
                && analyzed.token_source(*token) == Some(r"\stackrel")
        }));
        for malformed in [r"\stackrel", r"\stackrel{!}"] {
            assert_eq!(
                analyze(malformed, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: malformed.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn stacked_annotations_are_structural_and_require_two_groups() {
    let check = || -> Result<(), String> {
        let source = r"\overset{!}{=} + \underset{n\to\infty}{\lim}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("stacked annotation formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\overset", SupportedCommand::Overset),
            (r"\underset", SupportedCommand::Underset),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
        }
        for candidate_source in [r"\overset{!}", r"\underset{n}"] {
            assert_eq!(
                analyze(candidate_source, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: candidate_source.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{candidate_source}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn square_root_is_structural_and_requires_one_group() {
    let check = || -> Result<(), String> {
        let source = r"x = \sqrt{a^2 + b^2}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("square-root formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::SquareRoot)
                && analyzed.token_source(*token) == Some(r"\sqrt")
        }));
        assert_eq!(
            analyze(r"\sqrt", FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: 5,
                kind: MathSyntaxErrorKind::MissingRequiredGroup,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn indexed_square_root_preserves_structural_index_and_radicand() {
    let check = || -> Result<(), String> {
        let source = r"x = \sqrt[3]{a^2 + b^2}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("indexed square-root formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::RootIndexOpen
                && analyzed.token_source(*token) == Some("[")
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::RootIndexClose
                && analyzed.token_source(*token) == Some("]")
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Superscript
                && analyzed.token_source(*token) == Some("^")
        }));
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn indexed_square_root_scans_index_content_and_preserves_whitespace() {
    let check = || -> Result<(), String> {
        let source = r"\sqrt [\alpha+{3]}] {x}";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("indexed root with protected closing bracket")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::NamedSymbol)
                && analyzed.token_source(*token) == Some(r"\alpha")
        }));

        let unsupported = r"\sqrt[\unknown]{x}";
        let other_analysis_1 = analyze(unsupported, FormulaMode::Inline)
            .required("unsupported index command remains analyzable")?;
        assert!(!other_analysis_1.is_supported());
        assert_eq!(reconstructed(&other_analysis_1), unsupported);
        assert_eq!(other_analysis_1.unsupported.len(), 1);
        assert_eq!(
            other_analysis_1
                .unsupported
                .first()
                .required("first other_analysis_1 entry")?
                .name,
            r"\unknown"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn indexed_square_root_closure_respects_backslash_parity() {
    let check = || -> Result<(), String> {
        let protected = r"\sqrt[1\]2]{x}";
        let analyzed = analyze(protected, FormulaMode::Inline)
            .required("control symbol protects root-index closing bracket")?;
        assert!(!analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), protected);
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\]"
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RootIndexClose)
                .count(),
            1,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .find(|token| token.kind == MathTokenKind::RootIndexClose)
                .and_then(|token| analyzed.token_source(*token)),
            Some("]"),
        );

        let even_slashes = r"\sqrt[1\\]{x}";
        let other_analysis_1 = analyze(even_slashes, FormulaMode::Aligned)
            .required("row break ends before root-index closing bracket")?;
        assert!(other_analysis_1.is_supported());
        assert_eq!(reconstructed(&other_analysis_1), even_slashes);
        assert_eq!(
            other_analysis_1
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        assert_eq!(
            other_analysis_1
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RootIndexClose)
                .count(),
            1,
        );
        assert_eq!(
            analyze(even_slashes, FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: r"\sqrt[1".len(),
                kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn indexed_square_root_boundaries_are_scoped_without_global_brackets() {
    let check = || -> Result<(), String> {
        let nested = r"\sqrt[{\sqrt[3]{x}}]{y}";
        let analyzed = analyze(nested, FormulaMode::Inline)
            .required("brace-contained nested indexed root")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), nested);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RootIndexOpen)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RootIndexClose)
                .count(),
            2,
        );

        let ordinary = analyze("[a,b]", FormulaMode::Inline)
            .required("ordinary square brackets remain literal")?;
        assert_eq!(ordinary.tokens.len(), 1);
        assert_eq!(
            ordinary
                .tokens
                .first()
                .required("first analyzed entry")?
                .kind,
            MathTokenKind::Literal
        );

        let overlapping = r"\sqrt[\sqrt[3]{x}]{y}";
        assert_eq!(
            analyze(overlapping, FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: overlapping.len(),
                kind: MathSyntaxErrorKind::MissingRootIndexEnd,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn malformed_indexed_square_root_is_typed() {
    let missing_close = r"\sqrt[3{x}";
    assert_eq!(
        analyze(missing_close, FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: missing_close.len(),
            kind: MathSyntaxErrorKind::MissingRootIndexEnd,
        }),
    );
    let missing_radicand = r"\sqrt[3]x";
    assert_eq!(
        analyze(missing_radicand, FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: 8,
            kind: MathSyntaxErrorKind::MissingRequiredGroup,
        }),
    );
}

#[test]
fn deep_required_group_commands_are_iterative_and_balanced() {
    let check = || -> Result<(), String> {
        let depth = 4_096usize;
        let mut source = String::new();
        for _ in 0..depth {
            source.push_str(r"\widehat{");
        }
        source.push('x');
        for _ in 0..depth {
            source.push('}');
        }

        let analyzed = analyze(&source, FormulaMode::Inline)
            .required("deep nested required groups")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::WideHat)
                })
                .count(),
            depth,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn deep_indexed_square_roots_are_iterative_and_balanced() {
    let check = || -> Result<(), String> {
        let depth = 4_096usize;
        let mut source = String::new();
        for _ in 0..depth {
            source.push_str(r"\sqrt[{");
        }
        source.push('2');
        for _ in 0..depth {
            source.push_str("}]{x}");
        }
        let analyzed = analyze(&source, FormulaMode::Inline)
            .required("deep indexed square-root source")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::SquareRoot)
                })
                .count(),
            depth,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RootIndexOpen)
                .count(),
            depth,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RootIndexClose)
                .count(),
            depth,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn styled_binomials_are_structural_and_require_two_groups() {
    let check = || -> Result<(), String> {
        let source = r"\dbinom{n}{k}+\tbinom{a}{b}";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("styled binomial coefficients")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\dbinom", SupportedCommand::DisplayBinomial),
            (r"\tbinom", SupportedCommand::TextBinomial),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
            for malformed in [spelling.to_owned(), format!("{spelling}{{a}}")] {
                assert_eq!(
                    analyze(&malformed, FormulaMode::Inline),
                    Err(MathSyntaxError {
                        byte_offset: malformed.len(),
                        kind: MathSyntaxErrorKind::MissingRequiredGroup,
                    }),
                    "{malformed}",
                );
            }
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn binomial_is_structural_and_requires_two_groups() {
    let check = || -> Result<(), String> {
        let source = r"P(X=k) = \binom{n}{k}p^k(1-p)^{n-k}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("binomial formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::Binomial)
                && analyzed.token_source(*token) == Some(r"\binom")
        }));
        assert_eq!(
            analyze(r"\binom{n}", FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: 9,
                kind: MathSyntaxErrorKind::MissingRequiredGroup,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn styled_fractions_are_structural_and_require_two_groups() {
    let check = || -> Result<(), String> {
        let source = r"\dfrac{a}{b} + \tfrac{x+1}{y-1}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("styled fraction formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        for (spelling, kind) in [
            (r"\dfrac", SupportedCommand::DisplayFraction),
            (r"\tfrac", SupportedCommand::TextFraction),
        ] {
            assert!(analyzed.tokens.iter().any(|token| {
                token.kind == MathTokenKind::Command(kind)
                    && analyzed.token_source(*token) == Some(spelling)
            }));
        }
        for candidate_source in [r"\dfrac{a}", r"\tfrac{x}"] {
            assert_eq!(
                analyze(candidate_source, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset: candidate_source.len(),
                    kind: MathSyntaxErrorKind::MissingRequiredGroup,
                }),
                "{candidate_source}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn fraction_scripts_and_roman_units_are_structural_without_rewriting() {
    let check = || -> Result<(), String> {
        let source = r"E = \frac{1}{2}mv^2\mathrm{m/s^2}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("supported formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::Fraction)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::Roman)
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::Superscript)
                .count(),
            2,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn substack_rows_are_scoped_to_their_group() {
    let check = || -> Result<(), String> {
        let source = r"\sum_{\substack{i=1\\j=2}} a_{ij}";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("substack row formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::Substack)
                && analyzed.token_source(*token) == Some(r"\substack")
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        assert_eq!(
            analyze(r"\substack", FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: 9,
                kind: MathSyntaxErrorKind::MissingRequiredGroup,
            }),
        );

        let nested = r"\substack{a\\\substack{b\\c}\\d}";
        let other_analysis_1 = analyze(nested, FormulaMode::Inline)
            .required("nested substack rows")?;
        assert!(other_analysis_1.is_supported());
        assert_eq!(reconstructed(&other_analysis_1), nested);
        assert_eq!(
            other_analysis_1
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            3,
        );

        let aligned_column = r"\substack{a & b}";
        assert_eq!(
            analyze(aligned_column, FormulaMode::Aligned),
            Err(MathSyntaxError {
                byte_offset: r"\substack{a ".len(),
                kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
            }),
        );

        let escaped_scope = r"\substack{a\\b}\\c";
        assert_eq!(
            analyze(escaped_scope, FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: r"\substack{a\\b}".len(),
                kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn outer_environments_do_not_leak_alignment_into_substack_scope() {
    let check = || -> Result<(), String> {
        for (begin, end) in [
            (r"\begin{Bmatrix}", r"\end{Bmatrix}"),
            (r"\begin{Vmatrix}", r"\end{Vmatrix}"),
            (r"\begin{aligned}", r"\end{aligned}"),
            (r"\begin{bmatrix}", r"\end{bmatrix}"),
            (r"\begin{cases}", r"\end{cases}"),
            (r"\begin{matrix}", r"\end{matrix}"),
            (r"\begin{pmatrix}", r"\end{pmatrix}"),
            (r"\begin{smallmatrix}", r"\end{smallmatrix}"),
            (r"\begin{split}", r"\end{split}"),
            (r"\begin{vmatrix}", r"\end{vmatrix}"),
        ] {
            let source = format!(r"{begin}\substack{{a & b}}{end}");
            assert_eq!(
                analyze(&source, FormulaMode::Display),
                Err(MathSyntaxError {
                    byte_offset: source
                        .find('&')
                        .required("alignment marker")?,
                    kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
                }),
                "{source}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn substack_scope_composes_with_text_and_matrix_structure() {
    let check = || -> Result<(), String> {
        let with_text = r"\substack{a\\\text{b\\c}\\d}";
        let analyzed = analyze(with_text, FormulaMode::Inline)
            .required("substack containing text scope")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), with_text);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            2,
        );

        let with_matrix = concat!(
            r"\substack{\begin{matrix}a & b\\c & d\end{matrix}",
            r"\\e}",
        );
        let other_analysis_1 = analyze(with_matrix, FormulaMode::Inline)
            .required("substack containing matrix")?;
        assert!(other_analysis_1.is_supported());
        assert_eq!(reconstructed(&other_analysis_1), with_matrix);
        assert_eq!(
            other_analysis_1
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            2,
        );
        assert_eq!(
            other_analysis_1
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );

        let in_matrix = r"\begin{matrix}\substack{a\\b} & c\end{matrix}";
        let other_analysis_2 = analyze(in_matrix, FormulaMode::Display)
            .required("matrix containing substack")?;
        assert!(other_analysis_2.is_supported());
        assert_eq!(reconstructed(&other_analysis_2), in_matrix);
        assert_eq!(
            other_analysis_2
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        assert_eq!(
            other_analysis_2
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn deep_substack_scopes_are_iterative_and_do_not_leak() {
    let check = || -> Result<(), String> {
        let depth = 1_024usize;
        let mut source = String::new();
        for _ in 0..depth {
            source.push_str(r"\substack{a\\");
        }
        source.push('x');
        for _ in 0..depth {
            source.push_str(r"\\b}");
        }
        let analyzed = analyze(&source, FormulaMode::Inline)
            .required("deep nested substack source")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            depth * 2,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn text_scope_still_makes_substack_row_breaks_literal() {
    let check = || -> Result<(), String> {
        let source = r"\text{label \substack{a\\b}}";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("substack command nested in text scope")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            0,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn text_fragments_preserve_unicode_and_require_one_group() {
    let check = || -> Result<(), String> {
        let source = concat!(
            "E = mc2\\text{energ\u{ed}a {trabajo_a} & ",
            "direcci\u{f3}n^2 \\\\ l\u{ed}nea}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("text fragment formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::Text)
        }));
        assert!(!analyzed.tokens.iter().any(|token| matches!(
            token.kind,
            MathTokenKind::AlignmentPoint
                | MathTokenKind::RowBreak
                | MathTokenKind::Subscript
                | MathTokenKind::Superscript
        )));
        assert_eq!(
            analyze(r"x + \text", FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: 9,
                kind: MathSyntaxErrorKind::MissingRequiredGroup,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn unescaped_tex_specials_are_typed_syntax_failures() {
    let check = || -> Result<(), String> {
        for candidate_source in ["x % y", "x # y", "x $ y", r"\text{50% done}"]
        {
            let byte_offset = candidate_source
                .char_indices()
                .find_map(|(offset, character)| {
                    matches!(character, '%' | '#' | '$').then_some(offset)
                })
                .required("fixture must contain one unescaped special")?;
            assert_eq!(
                analyze(candidate_source, FormulaMode::Inline),
                Err(MathSyntaxError {
                    byte_offset,
                    kind: MathSyntaxErrorKind::UnescapedSpecialCharacter,
                }),
                "{candidate_source}",
            );
        }

        let escaped = r"x \% y + \#1 + \$5";
        let analyzed = analyze(escaped, FormulaMode::Inline)
            .required("escaped TeX specials remain admitted")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), escaped);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn escaped_braces_do_not_close_text_scope() {
    let check = || -> Result<(), String> {
        let source = r"\text{\{label_a\}} + x_1";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("escaped braces inside text")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(
                            SupportedCommand::EscapedSpecial,
                        )
                })
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::Subscript)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn nested_text_groups_keep_math_markers_literal_until_outer_close() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\text{outer_a^2 & \text{inner_b^3 & value} tail_c} + ",
            r"y^2 & z",
        );
        let analyzed = analyze(source, FormulaMode::Aligned)
            .required("nested text followed by aligned mathematics")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind == MathTokenKind::Command(SupportedCommand::Text)
                })
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::Superscript)
                .count(),
            1,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            1,
        );
        assert!(
            !analyzed
                .tokens
                .iter()
                .any(|token| token.kind == MathTokenKind::Subscript)
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn text_group_scope_ends_before_following_math_structure() {
    let check = || -> Result<(), String> {
        let source = r"\text{{texto_a}&valor} + x^2 & y";
        let analyzed = analyze(source, FormulaMode::Aligned)
            .required("text then aligned math")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::Superscript)
                .count(),
            1,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            1,
        );
        assert!(
            !analyzed
                .tokens
                .iter()
                .any(|token| token.kind == MathTokenKind::Subscript)
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn text_fragments_do_not_admit_unknown_control_words() {
    let check = || -> Result<(), String> {
        let source = r"\text{literal \mystery{value}}";
        let analyzed = analyze(source, FormulaMode::Display).required(
            "balanced text with unknown command remains inspectable",
        )?;
        assert!(!analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\mystery"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn longer_text_control_word_is_not_accepted_as_text_prefix() {
    let check = || -> Result<(), String> {
        let analyzed = analyze(r"\textual{x}", FormulaMode::Display)
            .required("balanced unsupported text-like source")?;
        assert!(!analyzed.is_supported());
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\textual"
        );
        assert_eq!(reconstructed(&analyzed), r"\textual{x}");
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn aligned_derivation_preserves_rows_and_alignment_points() {
    let check = || -> Result<(), String> {
        let source = "y &= (3x^2 + 1)^5 \\\\ y' &= 30x(3x^2 + 1)^4";
        let analyzed = analyze(source, FormulaMode::Aligned)
            .required("aligned formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn aligned_environment_admits_rows_and_alignment_in_display_mode() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{aligned}y &= (3x^2+1)^5 \\ ",
            r"y' &= 30x(3x^2+1)^4\end{aligned}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("aligned environment in display formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::BeginAligned)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::EndAligned)
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn braced_matrix_admits_rows_and_alignment() {
    let check = || -> Result<(), String> {
        let source = r"A=\begin{Bmatrix}a & b \\ c & d\end{Bmatrix}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("brace-delimited matrix formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::BeginBracedMatrix)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::EndBracedMatrix)
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn bracketed_matrix_admits_rows_and_alignment() {
    let check = || -> Result<(), String> {
        let source = r"A=\begin{bmatrix}a & b \\ c & d\end{bmatrix}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("bracketed matrix formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(
                    SupportedCommand::BeginBracketedMatrix,
                )
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::EndBracketedMatrix)
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn parenthesized_matrix_admits_rows_and_alignment() {
    let check = || -> Result<(), String> {
        let source = r"A=\begin{pmatrix}a & b \\ c & d\end{pmatrix}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("parenthesized matrix formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(
                    SupportedCommand::BeginParenthesizedMatrix,
                )
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(
                    SupportedCommand::EndParenthesizedMatrix,
                )
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn double_vertical_matrix_admits_rows_and_alignment() {
    let check = || -> Result<(), String> {
        let source = r"D=\begin{Vmatrix}a & b \\ c & d\end{Vmatrix}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("double-vertical-bar matrix formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(
                    SupportedCommand::BeginDoubleVerticalMatrix,
                )
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(
                    SupportedCommand::EndDoubleVerticalMatrix,
                )
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn vertical_matrix_admits_rows_and_alignment() {
    let check = || -> Result<(), String> {
        let source = r"D=\begin{vmatrix}a & b \\ c & d\end{vmatrix}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("vertical-bar matrix formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::BeginVerticalMatrix)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::EndVerticalMatrix)
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn small_matrix_admits_rows_and_alignment_with_distinct_identity() {
    let check = || -> Result<(), String> {
        let source = r"A=\begin{smallmatrix}a & b \\ c & d\end{smallmatrix}";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("compact small-matrix formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::BeginSmallMatrix)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::EndSmallMatrix)
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn split_environment_admits_derivation_rows_and_alignment() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{split}f(x) &= x^2 + 1 \\ ",
            r"f'(x) &= 2x\end{split}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("split derivation environment")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::BeginSplit)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::EndSplit)
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn gathered_scope_yields_to_nested_split_alignment() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{gathered}\begin{split}a &= b \\ c &= d\end{split}",
            r" \\ e\end{gathered}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("split nested inside gathered rows")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn gathered_environment_admits_rows_but_not_column_alignment() {
    let check = || -> Result<(), String> {
        let source = r"\begin{gathered}a \\ b\end{gathered}";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("gathered row structure")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::BeginGathered)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::EndGathered)
        }));

        let column = r"\begin{gathered}a & b\end{gathered}";
        assert_eq!(
            analyze(column, FormulaMode::Aligned),
            Err(MathSyntaxError {
                byte_offset: column.find('&').required("alignment marker")?,
                kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn gathered_scope_yields_to_nested_column_environment() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{gathered}\begin{matrix}a & b\end{matrix} \\ ",
            r"c\end{gathered}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("matrix nested inside gathered rows")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            1,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn gathered_scope_overrides_outer_column_environment() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{aligned}x &= \begin{gathered}a & b",
            r"\end{gathered}\end{aligned}",
        );
        assert_eq!(
            analyze(source, FormulaMode::Display),
            Err(MathSyntaxError {
                byte_offset: source
                    .rfind('&')
                    .required("gathered alignment marker")?,
                kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn text_scope_remains_literal_inside_gathered_environment() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{gathered}\text{a & b \\ c} \\ ",
            r"d\end{gathered}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("gathered rows containing text scope")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            0,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn gathered_row_scope_does_not_leak_after_close() {
    let prefix = r"\begin{gathered}a \\ b\end{gathered}";
    let source = format!(r"{prefix} \\ c");
    assert_eq!(
        analyze(&source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: prefix.len() + 1,
            kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
        }),
    );
}

#[test]
fn cases_environment_admits_rows_and_alignment() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"f(x)=\begin{cases}x & x>0 \\ ",
            r"-x & x\le 0\end{cases}",
        );
        let analyzed =
            analyze(source, FormulaMode::Display).required("cases formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::BeginCases)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::EndCases)
        }));
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn aligned_cases_and_matrix_environments_nest_in_order() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{aligned}f(x) &= \begin{cases}x & x>0 \\ ",
            r"\begin{matrix}a & b \\ c & d\end{matrix} & x<=0",
            r"\end{cases}\end{aligned}",
        );
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("ordered nested aligned cases and matrix environments")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn small_and_plain_matrices_keep_distinct_nesting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{smallmatrix}a & \begin{matrix}b & c \\ d & e",
            r"\end{matrix} \\ f & g\end{smallmatrix}",
        );
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("nested compact and plain matrices")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);

        let prefix = r"\begin{smallmatrix}\begin{matrix}x";
        let crossed = format!(r"{prefix}\end{{smallmatrix}}");
        assert_eq!(
            analyze(&crossed, FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: prefix.len(),
                kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn single_and_double_vertical_matrices_keep_distinct_nesting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{Vmatrix}a & \begin{vmatrix}b & c \\ d & e",
            r"\end{vmatrix} \\ f & g\end{Vmatrix}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("nested double- and single-vertical-bar matrices")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);

        let prefix = r"\begin{Vmatrix}\begin{vmatrix}x";
        let crossed = format!(r"{prefix}\end{{Vmatrix}}");
        assert_eq!(
            analyze(&crossed, FormulaMode::Display),
            Err(MathSyntaxError {
                byte_offset: prefix.len(),
                kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn vertical_and_bracketed_matrices_keep_distinct_nesting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{vmatrix}a & \begin{bmatrix}b & c \\ d & e",
            r"\end{bmatrix} \\ f & g\end{vmatrix}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("nested vertical-bar and bracketed matrices")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);

        let prefix = r"\begin{vmatrix}\begin{bmatrix}x";
        let crossed = format!(r"{prefix}\end{{vmatrix}}");
        assert_eq!(
            analyze(&crossed, FormulaMode::Display),
            Err(MathSyntaxError {
                byte_offset: prefix.len(),
                kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn braced_and_bracketed_matrices_keep_distinct_nesting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{Bmatrix}a & \begin{bmatrix}b & c \\ d & e",
            r"\end{bmatrix} \\ f & g\end{Bmatrix}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("nested brace-delimited and bracketed matrices")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);

        let prefix = r"\begin{Bmatrix}\begin{bmatrix}x";
        let crossed = format!(r"{prefix}\end{{Bmatrix}}");
        assert_eq!(
            analyze(&crossed, FormulaMode::Display),
            Err(MathSyntaxError {
                byte_offset: prefix.len(),
                kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn bracketed_and_parenthesized_matrices_keep_distinct_nesting() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{bmatrix}a & \begin{pmatrix}b & c \\ d & e",
            r"\end{pmatrix} \\ f & g\end{bmatrix}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("nested bracketed and parenthesized matrices")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);

        let prefix = r"\begin{bmatrix}\begin{pmatrix}x";
        let crossed = format!(r"{prefix}\end{{bmatrix}}");
        assert_eq!(
            analyze(&crossed, FormulaMode::Display),
            Err(MathSyntaxError {
                byte_offset: prefix.len(),
                kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn parenthesized_and_plain_matrices_nest_in_order() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{pmatrix}a & \begin{matrix}b & c \\ d & e",
            r"\end{matrix} \\ f & g\end{pmatrix}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("nested parenthesized and plain matrices")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn cases_and_matrix_environments_nest_in_order() {
    let check =
        || -> Result<(), String> {
            let source = concat!(
                r"\begin{cases}a & \begin{matrix}b & c \\ d & e",
                r"\end{matrix} \\ f & g\end{cases}",
            );
            let analyzed = analyze(source, FormulaMode::Display)
                .required("nested cases and matrix formula")?;
            assert!(analyzed.is_supported());
            assert_eq!(reconstructed(&analyzed), source);
            for kind in [
                SupportedCommand::BeginCases,
                SupportedCommand::BeginMatrix,
                SupportedCommand::EndCases,
                SupportedCommand::EndMatrix,
            ] {
                assert!(analyzed.tokens.iter().any(|token| {
                    token.kind == MathTokenKind::Command(kind)
                }));
            }
            Ok(())
        };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn crossed_environment_closes_are_typed() {
    let parenthesized_first = r"\begin{pmatrix}\begin{matrix}x";
    let parenthesized_source = format!(r"{parenthesized_first}\end{{pmatrix}}");
    assert_eq!(
        analyze(&parenthesized_source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: parenthesized_first.len(),
            kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
        }),
    );

    let aligned_first = r"\begin{aligned}\begin{cases}x";
    let aligned_source = format!(r"{aligned_first}\end{{aligned}}");
    assert_eq!(
        analyze(&aligned_source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: aligned_first.len(),
            kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
        }),
    );

    let first = r"\begin{matrix}\begin{cases}x";
    let source = format!(r"{first}\end{{matrix}}");
    assert_eq!(
        analyze(&source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: first.len(),
            kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
        }),
    );

    let other_first = r"\begin{cases}\begin{matrix}x";
    let other_source = format!(r"{other_first}\end{{cases}}");
    assert_eq!(
        analyze(&other_source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: other_first.len(),
            kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
        }),
    );
}

#[test]
fn mismatched_environment_end_precedes_group_depth_failure() {
    let prefix = r"\begin{split}{\begin{matrix}x";
    let source = format!(r"{prefix}\end{{split}}}}");
    assert_eq!(
        analyze(&source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: prefix.len(),
            kind: MathSyntaxErrorKind::MismatchedEnvironmentEnd,
        }),
    );
}

#[test]
fn environments_cannot_close_inside_a_later_group() {
    let check = || -> Result<(), String> {
        for (begin, end) in [
            (r"\begin{Bmatrix}", r"\end{Bmatrix}"),
            (r"\begin{Vmatrix}", r"\end{Vmatrix}"),
            (r"\begin{aligned}", r"\end{aligned}"),
            (r"\begin{bmatrix}", r"\end{bmatrix}"),
            (r"\begin{cases}", r"\end{cases}"),
            (r"\begin{gathered}", r"\end{gathered}"),
            (r"\begin{matrix}", r"\end{matrix}"),
            (r"\begin{pmatrix}", r"\end{pmatrix}"),
            (r"\begin{smallmatrix}", r"\end{smallmatrix}"),
            (r"\begin{split}", r"\end{split}"),
            (r"\begin{vmatrix}", r"\end{vmatrix}"),
        ] {
            let prefix = format!("{begin}{{a");
            let source = format!("{prefix}{end}}}");
            assert_eq!(
                analyze(&source, FormulaMode::Display),
                Err(MathSyntaxError {
                    byte_offset: prefix.len(),
                    kind: MathSyntaxErrorKind::EnvironmentClosesInsideGroup,
                }),
                "{source}",
            );
        }

        let properly_nested = r"\begin{matrix}{a}\end{matrix}";
        let analyzed = analyze(properly_nested, FormulaMode::Display)
            .required("group closes before containing environment")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), properly_nested);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn environments_must_close_before_their_owning_group() {
    let check = || -> Result<(), String> {
        for (begin, end) in [
            (r"\begin{Bmatrix}", r"\end{Bmatrix}"),
            (r"\begin{Vmatrix}", r"\end{Vmatrix}"),
            (r"\begin{aligned}", r"\end{aligned}"),
            (r"\begin{bmatrix}", r"\end{bmatrix}"),
            (r"\begin{cases}", r"\end{cases}"),
            (r"\begin{gathered}", r"\end{gathered}"),
            (r"\begin{matrix}", r"\end{matrix}"),
            (r"\begin{pmatrix}", r"\end{pmatrix}"),
            (r"\begin{smallmatrix}", r"\end{smallmatrix}"),
            (r"\begin{split}", r"\end{split}"),
            (r"\begin{vmatrix}", r"\end{vmatrix}"),
        ] {
            let prefix = format!("{{{begin}a");
            let source = format!("{prefix}}}{end}");
            assert_eq!(
                analyze(&source, FormulaMode::Display),
                Err(MathSyntaxError {
                    byte_offset: prefix.len(),
                    kind: MathSyntaxErrorKind::EnvironmentCrossesGroupClose,
                }),
                "{source}",
            );
        }

        let nested_group = r"{\begin{matrix}{a}\end{matrix}}";
        let analyzed = analyze(nested_group, FormulaMode::Display)
            .required("ordinary group nested inside environment")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), nested_group);
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn environment_alignment_scope_does_not_leak_after_close() {
    let prefix = r"\begin{cases}a & b\end{cases} ";
    let source = format!("{prefix}& c");
    assert_eq!(
        analyze(&source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: prefix.len(),
            kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
        }),
    );
}

#[test]
fn text_scope_remains_literal_inside_cases_environment() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{cases}\text{a & b \\ c} & d \\ ",
            r"e & f\end{cases}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("cases containing text scope")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| token.kind == MathTokenKind::RowBreak)
                .count(),
            1,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn deep_ordered_environments_are_iterative_and_balanced() {
    let check = || -> Result<(), String> {
        let depth = 1_024usize;
        let mut source = String::new();
        for level in 0..depth {
            match level
                .checked_rem(11)
                .required("nonzero environment cycle")?
            {
                0 => source.push_str(r"\begin{Bmatrix}"),
                1 => source.push_str(r"\begin{Vmatrix}"),
                2 => source.push_str(r"\begin{aligned}"),
                3 => source.push_str(r"\begin{bmatrix}"),
                4 => source.push_str(r"\begin{cases}"),
                5 => source.push_str(r"\begin{gathered}"),
                6 => source.push_str(r"\begin{matrix}"),
                7 => source.push_str(r"\begin{pmatrix}"),
                8 => source.push_str(r"\begin{smallmatrix}"),
                9 => source.push_str(r"\begin{split}"),
                _ => source.push_str(r"\begin{vmatrix}"),
            }
        }
        source.push('x');
        for level in (0..depth).rev() {
            match level
                .checked_rem(11)
                .required("nonzero environment cycle")?
            {
                0 => source.push_str(r"\end{Bmatrix}"),
                1 => source.push_str(r"\end{Vmatrix}"),
                2 => source.push_str(r"\end{aligned}"),
                3 => source.push_str(r"\end{bmatrix}"),
                4 => source.push_str(r"\end{cases}"),
                5 => source.push_str(r"\end{gathered}"),
                6 => source.push_str(r"\end{matrix}"),
                7 => source.push_str(r"\end{pmatrix}"),
                8 => source.push_str(r"\end{smallmatrix}"),
                9 => source.push_str(r"\end{split}"),
                _ => source.push_str(r"\end{vmatrix}"),
            }
        }
        let analyzed = analyze(&source, FormulaMode::Display)
            .required("deep ordered environments")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| matches!(
                    token.kind,
                    MathTokenKind::Command(
                        SupportedCommand::BeginAligned
                            | SupportedCommand::BeginBracketedMatrix
                            | SupportedCommand::BeginBracedMatrix
                            | SupportedCommand::BeginDoubleVerticalMatrix
                            | SupportedCommand::BeginCases
                            | SupportedCommand::BeginGathered
                            | SupportedCommand::BeginMatrix
                            | SupportedCommand::BeginParenthesizedMatrix
                            | SupportedCommand::BeginSmallMatrix
                            | SupportedCommand::BeginSplit
                            | SupportedCommand::BeginVerticalMatrix
                    )
                ))
                .count(),
            depth,
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn matrix_admits_alignment_inside_display_mode() {
    let check = || -> Result<(), String> {
        let source = r"A = \begin{matrix}a & b \\ c & d\end{matrix}";
        let analyzed =
            analyze(source, FormulaMode::Display).required("matrix formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::BeginMatrix)
        }));
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::EndMatrix)
        }));
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn nested_matrix_depth_balances_before_outer_alignment_resumes() {
    let check = || -> Result<(), String> {
        let source = concat!(
            r"\begin{matrix}a & \begin{matrix}b & c \\ d & e",
            r"\end{matrix} \\ f & g\end{matrix}",
        );
        let analyzed = analyze(source, FormulaMode::Display)
            .required("nested matrix formula")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::BeginMatrix)
                })
                .count(),
            2,
        );
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind
                        == MathTokenKind::Command(SupportedCommand::EndMatrix)
                })
                .count(),
            2,
        );

        let balanced = r"\begin{matrix}\begin{matrix}x\end{matrix}\end{matrix}";
        let extra = concat!(
            r"\begin{matrix}\begin{matrix}x\end{matrix}\end{matrix}",
            r"\end{matrix}",
        );
        assert_eq!(
            analyze(extra, FormulaMode::Display),
            Err(MathSyntaxError {
                byte_offset: balanced.len(),
                kind: MathSyntaxErrorKind::ExtraMatrixEnd,
            }),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn unknown_command_remains_exact_explicit_unsupported_input() {
    let check = || -> Result<(), String> {
        let source = r"x + \mystery{y}";
        let analyzed =
            analyze(source, FormulaMode::Inline).required("balanced source")?;
        assert!(!analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\mystery"
        );
        assert_eq!(
            source.get(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .start
                    ..analyzed
                        .unsupported
                        .first()
                        .required("first analyzed entry")?
                        .end
            ),
            Some(r"\mystery"),
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn alignment_is_rejected_outside_admitted_structure() {
    assert_eq!(
        analyze("a & b", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 2,
            kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
        }),
    );
    assert_eq!(
        analyze(r"a \\ b", FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: 2,
            kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
        }),
    );
}

#[test]
fn malformed_groups_and_required_arguments_are_typed() {
    assert_eq!(
        analyze("a}", FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: 1,
            kind: MathSyntaxErrorKind::ExtraGroupClose,
        }),
    );
    assert_eq!(
        analyze("{a", FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: 2,
            kind: MathSyntaxErrorKind::UnclosedGroup,
        }),
    );
    assert_eq!(
        analyze(r"\frac{1}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 8,
            kind: MathSyntaxErrorKind::MissingRequiredGroup,
        }),
    );
    assert_eq!(
        analyze(r"\mathrm", FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: 7,
            kind: MathSyntaxErrorKind::MissingRequiredGroup,
        }),
    );
}

#[test]
fn environment_names_require_exact_braced_spelling() {
    let check = || -> Result<(), String> {
        for (candidate_source, unsupported) in [
            (r"\begin{Bmatrixx}a & b", r"\begin"),
            (r"a & b\end{Bmatrixx}", r"\end"),
            (r"\begin{Vmatrixx}a & b", r"\begin"),
            (r"a & b\end{Vmatrixx}", r"\end"),
            (r"\begin{alignedx}a & b", r"\begin"),
            (r"a & b\end{alignedx}", r"\end"),
            (r"\begin{bmatrixx}a & b", r"\begin"),
            (r"a & b\end{bmatrixx}", r"\end"),
            (r"\begin{casesx}a & b", r"\begin"),
            (r"a & b\end{casesx}", r"\end"),
            (r"\begin{gatheredx}a \\ b", r"\begin"),
            (r"a \\ b\end{gatheredx}", r"\end"),
            (r"\begin{matrixx}a & b", r"\begin"),
            (r"a & b\end{matrixx}", r"\end"),
            (r"\begin{pmatrixx}a & b", r"\begin"),
            (r"a & b\end{pmatrixx}", r"\end"),
            (r"\begin{smallmatrixx}a & b", r"\begin"),
            (r"a & b\end{smallmatrixx}", r"\end"),
            (r"\begin{splitx}a & b", r"\begin"),
            (r"a & b\end{splitx}", r"\end"),
            (r"\begin{vmatrixx}a & b", r"\begin"),
            (r"a & b\end{vmatrixx}", r"\end"),
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Aligned)
                .required("balanced near-match environment candidate_source")?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(analyzed.unsupported.len(), 1, "{candidate_source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                unsupported,
                "{candidate_source}"
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn optional_environment_position_requires_unowned_presentation_semantics() {
    let check = || -> Result<(), String> {
        for (source, begin) in [
            (r"\begin{aligned}[t]a&=b\end{aligned}", r"\begin{aligned}"),
            (r"\begin{aligned} [b]a&=b\end{aligned}", r"\begin{aligned}"),
            (
                r"\begin{gathered}[b]a\\b\end{gathered}",
                r"\begin{gathered}",
            ),
            (
                r"\begin{gathered} [t]a\\b\end{gathered}",
                r"\begin{gathered}",
            ),
        ] {
            let analyzed = analyze(source, FormulaMode::Display)
                .required("balanced optional environment position")?;
            assert!(!analyzed.is_supported(), "{source}");
            assert_eq!(reconstructed(&analyzed), source);
            assert_eq!(analyzed.unsupported.len(), 1, "{source}");
            assert_eq!(
                analyzed
                    .unsupported
                    .first()
                    .required("first analyzed entry")?
                    .name,
                begin,
                "{source}"
            );
            assert_eq!(
                analyzed
                    .token_source(
                        *analyzed
                            .tokens
                            .first()
                            .required("first analyzed entry")?
                    )
                    .required("begin environment token")?,
                begin,
                "{source}",
            );
        }

        for candidate_source in [
            r"\begin{matrix}[c]x\end{matrix}",
            r"\begin{smallmatrix}[c]x\end{smallmatrix}",
        ] {
            let other_analysis_1 =
                analyze(candidate_source, FormulaMode::Display)
                    .required("brackets are ordinary matrix content")?;
            assert!(other_analysis_1.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&other_analysis_1), candidate_source);
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn unmodeled_inner_math_environments_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for candidate_source in [
            r"\begin{alignedat}{2}x=1\end{alignedat}",
            r"\begin{subarray}{c}x\end{subarray}",
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required("balanced unmodeled inner math environment")?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(
                analyzed
                    .unsupported
                    .iter()
                    .map(|item| item.name.as_str())
                    .collect::<Vec<_>>(),
                [r"\begin", r"\end"],
                "{candidate_source}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn document_level_math_environments_remain_explicitly_unsupported() {
    let check = || -> Result<(), String> {
        for candidate_source in [
            r"\begin{equation}x=1\end{equation}",
            r"\begin{equation*}x=1\end{equation*}",
            r"\begin{align}x=1\end{align}",
            r"\begin{align*}x=1\end{align*}",
            r"\begin{alignat}{2}x=1\end{alignat}",
            r"\begin{alignat*}{2}x=1\end{alignat*}",
            r"\begin{xalignat}{2}x=1\end{xalignat}",
            r"\begin{xalignat*}{2}x=1\end{xalignat*}",
            r"\begin{xxalignat}{2}x=1\end{xxalignat}",
            r"\begin{flalign}x=1\end{flalign}",
            r"\begin{flalign*}x=1\end{flalign*}",
            r"\begin{gather}x=1\end{gather}",
            r"\begin{gather*}x=1\end{gather*}",
            r"\begin{multline}x=1\end{multline}",
            r"\begin{multline*}x=1\end{multline*}",
            r"\begin{subequations}x=1\end{subequations}",
        ] {
            let analyzed = analyze(candidate_source, FormulaMode::Display)
                .required("balanced document-level math environment")?;
            assert!(!analyzed.is_supported(), "{candidate_source}");
            assert_eq!(reconstructed(&analyzed), candidate_source);
            assert_eq!(
                analyzed
                    .unsupported
                    .iter()
                    .map(|item| item.name.as_str())
                    .collect::<Vec<_>>(),
                [r"\begin", r"\end"],
                "{candidate_source}",
            );
        }
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn unknown_matrix_environment_remains_explicit_unsupported_source() {
    let check = || -> Result<(), String> {
        let source = r"\begin{array}a & b";
        let analyzed = analyze(source, FormulaMode::Aligned)
            .required("balanced unknown matrix environment source")?;
        assert!(!analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\begin"
        );
        assert!(!analyzed.tokens.iter().any(|token| {
            token.kind == MathTokenKind::Command(SupportedCommand::BeginMatrix)
        }));
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn missing_environment_reports_innermost_open_boundary() {
    let cases_inside_matrix = r"\begin{matrix}\begin{cases}x";
    assert_eq!(
        analyze(cases_inside_matrix, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: cases_inside_matrix.len(),
            kind: MathSyntaxErrorKind::MissingCasesEnd,
        }),
    );

    let matrix_inside_cases = r"\begin{cases}\begin{matrix}x";
    assert_eq!(
        analyze(matrix_inside_cases, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: matrix_inside_cases.len(),
            kind: MathSyntaxErrorKind::MissingMatrixEnd,
        }),
    );
}

#[test]
fn malformed_aligned_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{aligned}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraAlignedEnd,
        }),
    );
    let source = r"\begin{aligned}a &= b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingAlignedEnd,
        }),
    );
}

#[test]
fn malformed_cases_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{cases}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraCasesEnd,
        }),
    );
    let source = r"\begin{cases}a & b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingCasesEnd,
        }),
    );
}

#[test]
fn malformed_gathered_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{gathered}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraGatheredEnd,
        }),
    );
    let source = r"\begin{gathered}a \\ b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingGatheredEnd,
        }),
    );
}

#[test]
fn malformed_matrix_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{matrix}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraMatrixEnd,
        }),
    );
    let source = r"\begin{matrix}a & b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingMatrixEnd,
        }),
    );
}

#[test]
fn malformed_braced_matrix_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{Bmatrix}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraBracedMatrixEnd,
        }),
    );
    let source = r"\begin{Bmatrix}a & b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingBracedMatrixEnd,
        }),
    );
}

#[test]
fn malformed_bracketed_matrix_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{bmatrix}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraBracketedMatrixEnd,
        }),
    );
    let source = r"\begin{bmatrix}a & b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingBracketedMatrixEnd,
        }),
    );
}

#[test]
fn malformed_parenthesized_matrix_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{pmatrix}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraParenthesizedMatrixEnd,
        }),
    );
    let source = r"\begin{pmatrix}a & b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingParenthesizedMatrixEnd,
        }),
    );
}

#[test]
fn malformed_double_vertical_matrix_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{Vmatrix}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraDoubleVerticalMatrixEnd,
        }),
    );
    let source = r"\begin{Vmatrix}a & b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingDoubleVerticalMatrixEnd,
        }),
    );
}

#[test]
fn malformed_vertical_matrix_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{vmatrix}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraVerticalMatrixEnd,
        }),
    );
    let source = r"\begin{vmatrix}a & b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingVerticalMatrixEnd,
        }),
    );
}

#[test]
fn malformed_small_matrix_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{smallmatrix}", FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraSmallMatrixEnd,
        }),
    );
    let source = r"\begin{smallmatrix}a & b";
    assert_eq!(
        analyze(source, FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingSmallMatrixEnd,
        }),
    );
}

#[test]
fn malformed_split_boundaries_are_typed() {
    assert_eq!(
        analyze(r"\end{split}", FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: 0,
            kind: MathSyntaxErrorKind::ExtraSplitEnd,
        }),
    );
    let source = r"\begin{split}a &= b";
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.len(),
            kind: MathSyntaxErrorKind::MissingSplitEnd,
        }),
    );
}

#[test]
fn token_source_is_sliced_only_from_its_own_unchanged_analysis() {
    let check = || -> Result<(), String> {
        let analyzed = analyze("\u{f1}^2", FormulaMode::Inline)
            .required("unicode source")?;
        let script = analyzed
            .tokens
            .iter()
            .find(|token| token.kind == MathTokenKind::Superscript)
            .required("superscript")?;
        assert_eq!(analyzed.token_source(*script), Some("^"));
        assert_eq!(reconstructed(&analyzed), "\u{f1}^2");
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ascii_control_words_remain_case_sensitive() {
    let check = || -> Result<(), String> {
        let source = r"\Pr(A) + \pr(A)";
        let analyzed = analyze(source, FormulaMode::Display)
            .required("case-sensitive control-word source")?;
        assert!(!analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\pr"
        );
        assert!(analyzed.tokens.iter().any(|token| {
            token.kind
                == MathTokenKind::Command(SupportedCommand::NamedOperator)
                && analyzed.token_source(*token) == Some(r"\Pr")
        }));
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn ascii_control_word_boundary_preserves_adjacent_unicode() {
    let check = || -> Result<(), String> {
        let source = "\\alpha\u{3b2} + \\det\u{f1} + \\mathbb{R}\u{3bb}";
        let analyzed = analyze(source, FormulaMode::Inline)
            .required("supported commands followed by Unicode")?;
        assert!(analyzed.is_supported());
        assert_eq!(reconstructed(&analyzed), source);

        let unsupported_source = "\\alphaZ\u{3b2}";
        let unsupported = analyze(unsupported_source, FormulaMode::Inline)
            .required(
                "balanced longer ASCII control word with Unicode suffix",
            )?;
        assert!(!unsupported.is_supported());
        assert_eq!(reconstructed(&unsupported), unsupported_source);
        assert_eq!(unsupported.unsupported.len(), 1);
        assert_eq!(
            unsupported
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\alphaZ"
        );
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn longer_unknown_control_word_is_not_accepted_as_supported_prefix() {
    let check = || -> Result<(), String> {
        let analyzed = analyze(r"\fraction{x}", FormulaMode::Display)
            .required("balanced unsupported source")?;
        assert!(!analyzed.is_supported());
        assert_eq!(analyzed.unsupported.len(), 1);
        assert_eq!(
            analyzed
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\fraction"
        );
        assert_eq!(reconstructed(&analyzed), r"\fraction{x}");
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}

#[test]
fn finite_first_release_math_sources_are_lossless() {
    let check = || -> Result<(), String> {
        let examples = [
            ("inline", FormulaMode::Inline, "x^2 + y^2 = z^2"),
            ("display", FormulaMode::Display, r"\frac{a}{b} = 2"),
            ("alignment", FormulaMode::Aligned, r"a &= b \\ c &= d"),
            (
                "matrix",
                FormulaMode::Display,
                r"A=\begin{pmatrix}1 & 2 \\ 3 & 4\end{pmatrix}",
            ),
            ("unit", FormulaMode::Inline, r"v=5\mathrm{m/s}"),
            (
                "derivation",
                FormulaMode::Aligned,
                r"F &= ma \\ a &= \frac{F}{m}",
            ),
        ];
        for (family, mode, source) in examples {
            let analyzed = analyze(source, mode).required(family)?;
            assert_analysis_invariants(&analyzed, source, mode);
            assert!(
                analyzed.is_supported(),
                "{family}: unexpected unsupported"
            );
            assert_eq!(
                reconstructed(&analyzed),
                source,
                "{family}: source changed",
            );
        }

        let unsupported_source = r"\unknownfunction{x}";
        let unsupported = analyze(unsupported_source, FormulaMode::Display)
            .required("balanced unsupported formula remains inspectable")?;
        assert_analysis_invariants(
            &unsupported,
            unsupported_source,
            FormulaMode::Display,
        );
        assert!(!unsupported.is_supported());
        assert_eq!(unsupported.unsupported.len(), 1);
        assert_eq!(
            unsupported
                .unsupported
                .first()
                .required("first analyzed entry")?
                .name,
            r"\unknownfunction"
        );
        assert_eq!(reconstructed(&unsupported), unsupported_source);
        assert!(analyze(r"\frac{x}", FormulaMode::Display).is_err());
        Ok(())
    };
    let result = check();
    assert!(result.is_ok(), "fixture failed: {result:?}");
}
