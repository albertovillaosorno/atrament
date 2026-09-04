// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
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
use atrament_mathematics_source::{
    FormulaMode, MathSyntaxError, MathSyntaxErrorKind, MathTokenKind,
    SupportedCommand, analyze,
};

fn reconstructed(
    analyzed: &atrament_mathematics_source::AnalyzedFormula,
) -> String {
    analyzed
        .tokens
        .iter()
        .map(|token| analyzed.token_source(*token).expect("token boundary"))
        .collect()
}

#[test]
fn unicode_school_formula_is_preserved_byte_for_byte() {
    let source = "y′ = 5(3x² + 1)⁴ · 6x";
    let analyzed =
        analyze(source, FormulaMode::Display).expect("unicode formula");
    assert!(analyzed.is_supported());
    assert_eq!(analyzed.source, source);
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(analyzed.tokens.len(), 1);
    assert_eq!(analyzed.tokens[0].kind, MathTokenKind::Literal);
}

#[test]
fn escaped_braces_are_supported_without_corrupting_group_indexing() {
    let source = r"\frac{\{x\}}{2}";
    let analyzed =
        analyze(source, FormulaMode::Display).expect("escaped braces formula");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| {
                token.kind
                    == MathTokenKind::Command(SupportedCommand::EscapedSpecial)
            })
            .count(),
        2,
    );
}

#[test]
fn escaped_tex_specials_remain_literal_in_math_and_text() {
    let source = r"\{x \& y\} = 50\% \#1 + a\_b + \$5 + \text{A \& B}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("escaped specials formula");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| {
                token.kind
                    == MathTokenKind::Command(SupportedCommand::EscapedSpecial)
            })
            .count(),
        8,
    );
    assert!(!analyzed
        .tokens
        .iter()
        .any(|token| token.kind == MathTokenKind::AlignmentPoint));
}

#[test]
fn named_operator_vocabulary_is_supported_without_rewriting() {
    for source in [
        r"\arccos", r"\arcsin", r"\arctan", r"\cos", r"\exp",
        r"\int", r"\lim", r"\ln", r"\log", r"\max", r"\min",
        r"\prod", r"\sin", r"\sum", r"\tan",
    ] {
        let analyzed = analyze(source, FormulaMode::Inline)
            .expect("named operator formula");
        assert!(analyzed.is_supported(), "{source}");
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(
            analyzed
                .tokens
                .iter()
                .filter(|token| {
                    token.kind == MathTokenKind::Command(
                        SupportedCommand::NamedOperator,
                    )
                })
                .count(),
            1,
            "{source}",
        );
    }

    let prefixed = analyze(r"\sinewave", FormulaMode::Inline)
        .expect("balanced unsupported operator prefix");
    assert!(!prefixed.is_supported());
    assert_eq!(prefixed.unsupported[0].name, r"\sinewave");
}

#[test]
fn named_operators_compose_with_symbols_and_groups() {
    let source = r"\sin(\theta) + \log(x) \le \max{y}";
    let analyzed =
        analyze(source, FormulaMode::Display).expect("operator expression");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| {
                token.kind
                    == MathTokenKind::Command(SupportedCommand::NamedOperator)
            })
            .count(),
        3,
    );
}

#[test]
fn calculus_commands_compose_with_scripts_without_rewriting() {
    let source = concat!(
        r"\sum_{i=1}^n i + \prod_{k=1}^m k + ",
        r"\int_0^1 x dx + \lim_{x \to 0} x",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("calculus expression");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| {
                token.kind
                    == MathTokenKind::Command(SupportedCommand::NamedOperator)
            })
            .count(),
        4,
    );

    let supported = r"\partial f + \nabla g";
    let analyzed = analyze(supported, FormulaMode::Inline)
        .expect("calculus symbol expression");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), supported);
}

#[test]
fn named_symbol_vocabulary_is_supported_without_rewriting() {
    for source in [
        r"\Delta", r"\Gamma", r"\Lambda", r"\Leftrightarrow", r"\Omega",
        r"\Phi", r"\Pi", r"\Psi", r"\Rightarrow", r"\Sigma", r"\Theta",
        r"\Upsilon", r"\Xi", r"\alpha", r"\approx", r"\beta", r"\cap",
        r"\cdot", r"\chi", r"\cong", r"\cup", r"\delta", r"\emptyset",
        r"\epsilon", r"\equiv", r"\eta", r"\exists", r"\forall", r"\gamma",
        r"\ge", r"\geq", r"\in", r"\infty", r"\iota", r"\kappa",
        r"\lambda", r"\land", r"\le", r"\leq", r"\leftarrow", r"\lor",
        r"\mid", r"\mu", r"\nabla", r"\ne", r"\neg", r"\neq", r"\notin",
        r"\nu", r"\omega", r"\parallel", r"\partial", r"\perp", r"\phi",
        r"\pi", r"\pm", r"\propto", r"\psi", r"\rho", r"\rightarrow",
        r"\sigma", r"\sim", r"\subset",
        r"\subseteq", r"\supset", r"\supseteq", r"\tau", r"\theta",
        r"\times", r"\to", r"\upsilon", r"\varepsilon", r"\varphi",
        r"\varpi", r"\varrho", r"\varsigma", r"\vartheta", r"\xi", r"\zeta",
    ] {
        let analyzed =
            analyze(source, FormulaMode::Inline).expect("named symbol formula");
        assert!(analyzed.is_supported(), "{source}");
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
            1,
            "{source}",
        );
    }
}

#[test]
fn variant_greek_symbols_compose_without_rewriting() {
    let source = concat!(
        r"\varepsilon + \varphi + \vartheta + ",
        r"\varrho + \varsigma + \varpi",
    );
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("variant Greek symbol expression");
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
}

#[test]
fn common_relation_and_logic_symbols_compose_without_rewriting() {
    let source = concat!(
        r"a \equiv b \sim c \cong d; x \propto y; ",
        r"u \perp v, p \parallel q, a \mid b; ",
        r"P \land Q \lor \neg R",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("relation and logic expression");
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
}

#[test]
fn standard_greek_symbols_compose_without_rewriting() {
    let source = concat!(
        r"\Delta x = \Sigma_i \Gamma_i + \Omega; ",
        r"\eta + \iota + \kappa + \nu + \xi + \tau + ",
        r"\upsilon + \chi + \psi + \zeta",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("standard Greek symbol expression");
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
}

#[test]
fn set_logic_and_arrow_symbols_compose_without_rewriting() {
    let source = concat!(
        r"\forall x \in A \cup B, x \notin \emptyset \Rightarrow ",
        r"A \subseteq B \Leftrightarrow B \supseteq A",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("set and logic expression");
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
    let analyzed = analyze(arrows, FormulaMode::Inline)
        .expect("arrow expression");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), arrows);
}

#[test]
fn named_symbols_compose_with_scripts_and_relations() {
    let source = concat!(
        r"A = \pi r^2; \theta_1 \le \phi \pm \infty; ",
        r"a \times b \approx c \cdot d",
    );
    let analyzed =
        analyze(source, FormulaMode::Display).expect("symbol expression");
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
        .expect("balanced unsupported symbol prefix");
    assert!(!prefixed.is_supported());
    assert_eq!(prefixed.unsupported[0].name, r"\pioneer");
}

#[test]
fn admitted_control_words_never_match_longer_ascii_names() {
    for (source, unsupported) in [
        (r"\binomial{n}{k}", r"\binomial"),
        (r"\integral_0^1", r"\integral"),
        (r"\infinite", r"\infinite"),
        (r"\subsetequal", r"\subsetequal"),
        (r"\overlined{x}", r"\overlined"),
        (r"\vectored{x}", r"\vectored"),
        (r"\Rightarrowed", r"\Rightarrowed"),
        (r"\emptysets", r"\emptysets"),
        (r"\nablax", r"\nablax"),
    ] {
        let analyzed = analyze(source, FormulaMode::Inline)
            .expect("balanced longer control word");
        assert!(!analyzed.is_supported(), "{source}");
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1, "{source}");
        assert_eq!(analyzed.unsupported[0].name, unsupported, "{source}");
    }
}

#[test]
fn unsupported_control_symbols_preserve_utf8_and_trailing_slash() {
    for (source, unsupported) in [(r"x \🙂 y", r"\🙂"), (r"z \", r"\")] {
        let analyzed = analyze(source, FormulaMode::Inline)
            .expect("balanced unsupported control symbol");
        assert!(!analyzed.is_supported(), "{source:?}");
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1, "{source:?}");
        assert_eq!(analyzed.unsupported[0].name, unsupported, "{source:?}");
        assert_eq!(
            source.get(
                analyzed.unsupported[0].start..analyzed.unsupported[0].end
            ),
            Some(unsupported),
        );
    }
}

#[test]
fn unknown_control_symbol_stays_explicitly_unsupported() {
    let source = r"x \@ y";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("balanced unknown control symbol");
    assert!(!analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\@");
}

#[test]
fn required_groups_allow_ascii_whitespace_without_rewriting() {
    for source in [
        "\\frac \t{1}  {2}",
        "\\binom\n{n}\r\n{k}",
        "\\sqrt \t{x}",
        "\\operatorname  {rank}",
    ] {
        let analyzed = analyze(source, FormulaMode::Display)
            .expect("whitespace-separated required groups");
        assert!(analyzed.is_supported(), "{source:?}");
        assert_eq!(reconstructed(&analyzed), source);
    }
}

#[test]
fn common_accents_are_structural_and_require_one_group() {
    let source = r"\bar{x} + \hat{p} + \dot{x} + \ddot{x} + \tilde{y}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("common accent formula");
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
        .expect("balanced longer accent command");
    assert!(!prefixed.is_supported());
    assert_eq!(prefixed.unsupported[0].name, r"\hatted");
}

#[test]
fn grouped_math_alphabets_are_structural_and_require_one_group() {
    let source = r"x \in \mathbb{R}, \mathbf{v}, \mathcal{F}, \mathit{x}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("grouped mathematical alphabets");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    for (spelling, kind) in [
        (r"\mathbb", SupportedCommand::BlackboardBold),
        (r"\mathbf", SupportedCommand::Bold),
        (r"\mathcal", SupportedCommand::Calligraphic),
        (r"\mathit", SupportedCommand::Italic),
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
        .expect("balanced longer math-alphabet command");
    assert!(!prefixed.is_supported());
    assert_eq!(prefixed.unsupported[0].name, r"\mathbboard");
}

#[test]
fn custom_operator_name_is_structural_and_requires_one_group() {
    let source = r"\operatorname{Var}(X) + \operatorname{Cov}(X,Y)";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("custom operator formula");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| {
                token.kind
                    == MathTokenKind::Command(SupportedCommand::OperatorName)
            })
            .count(),
        2,
    );
    assert_eq!(
        analyze(r"\operatorname", FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: 13,
            kind: MathSyntaxErrorKind::MissingRequiredGroup,
        }),
    );

    let prefixed = analyze(r"\operatornamed{rank}", FormulaMode::Inline)
        .expect("balanced longer operator-name command");
    assert!(!prefixed.is_supported());
    assert_eq!(prefixed.unsupported[0].name, r"\operatornamed");
}

#[test]
fn grouped_decorations_are_structural_and_require_one_group() {
    let source = r"\vec{v} + \overline{AB} + \underline{x_1}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("grouped decoration formula");
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
    for (source, byte_offset) in [
        (r"\vec", 4),
        (r"\overline", 9),
        (r"\underline", 10),
    ] {
        assert_eq!(
            analyze(source, FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset,
                kind: MathSyntaxErrorKind::MissingRequiredGroup,
            }),
            "{source}",
        );
    }
}

#[test]
fn square_root_is_structural_and_requires_one_group() {
    let source = r"x = \sqrt{a^2 + b^2}";
    let analyzed =
        analyze(source, FormulaMode::Display).expect("square-root formula");
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
}

#[test]
fn binomial_is_structural_and_requires_two_groups() {
    let source = r"P(X=k) = \binom{n}{k}p^k(1-p)^{n-k}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("binomial formula");
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
}

#[test]
fn fraction_scripts_and_roman_units_are_structural_without_rewriting() {
    let source = r"E = \frac{1}{2}mv^2\mathrm{m/s^2}";
    let analyzed =
        analyze(source, FormulaMode::Display).expect("supported formula");
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
}

#[test]
fn text_fragments_preserve_unicode_and_require_one_group() {
    let source = r"E = mc2\text{energía {trabajo_a} & dirección^2 \\ línea}";
    let analyzed =
        analyze(source, FormulaMode::Display).expect("text fragment formula");
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
}

#[test]
fn text_group_scope_ends_before_following_math_structure() {
    let source = r"\text{{texto_a}&valor} + x^2 & y";
    let analyzed =
        analyze(source, FormulaMode::Aligned).expect("text then aligned math");
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
    assert!(!analyzed
        .tokens
        .iter()
        .any(|token| token.kind == MathTokenKind::Subscript));
}

#[test]
fn text_fragments_do_not_admit_unknown_control_words() {
    let source = r"\text{literal \mystery{value}}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("balanced text with unknown command remains inspectable");
    assert!(!analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\mystery");
}

#[test]
fn longer_text_control_word_is_not_accepted_as_text_prefix() {
    let analyzed = analyze(r"\textual{x}", FormulaMode::Display)
        .expect("balanced unsupported text-like source");
    assert!(!analyzed.is_supported());
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\textual");
    assert_eq!(reconstructed(&analyzed), r"\textual{x}");
}

#[test]
fn aligned_derivation_preserves_rows_and_alignment_points() {
    let source = "y &= (3x^2 + 1)^5 \\\\ y' &= 30x(3x^2 + 1)^4";
    let analyzed =
        analyze(source, FormulaMode::Aligned).expect("aligned formula");
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
}

#[test]
fn matrix_admits_alignment_inside_display_mode() {
    let source = r"A = \begin{matrix}a & b \\ c & d\end{matrix}";
    let analyzed =
        analyze(source, FormulaMode::Display).expect("matrix formula");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::BeginMatrix)
    }));
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::EndMatrix)
    }));
}

#[test]
fn nested_matrix_depth_balances_before_outer_alignment_resumes() {
    let source = concat!(
        r"\begin{matrix}a & \begin{matrix}b & c \\ d & e",
        r"\end{matrix} \\ f & g\end{matrix}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("nested matrix formula");
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
}

#[test]
fn unknown_command_remains_exact_explicit_unsupported_input() {
    let source = r"x + \mystery{y}";
    let analyzed =
        analyze(source, FormulaMode::Inline).expect("balanced source");
    assert!(!analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\mystery");
    assert_eq!(
        source.get(analyzed.unsupported[0].start..analyzed.unsupported[0].end),
        Some(r"\mystery"),
    );
}

#[test]
fn alignment_is_rejected_outside_aligned_or_matrix_structure() {
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
fn token_source_is_sliced_only_from_its_own_unchanged_analysis() {
    let analyzed = analyze("ñ^2", FormulaMode::Inline).expect("unicode source");
    let script = analyzed
        .tokens
        .iter()
        .find(|token| token.kind == MathTokenKind::Superscript)
        .expect("superscript");
    assert_eq!(analyzed.token_source(*script), Some("^"));
    assert_eq!(reconstructed(&analyzed), "ñ^2");
}

#[test]
fn longer_unknown_control_word_is_not_accepted_as_supported_prefix() {
    let analyzed = analyze(r"\fraction{x}", FormulaMode::Display)
        .expect("balanced unsupported source");
    assert!(!analyzed.is_supported());
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\fraction");
    assert_eq!(reconstructed(&analyzed), r"\fraction{x}");
}
