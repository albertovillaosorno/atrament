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

fn assert_analysis_invariants(
    analyzed: &atrament_mathematics_source::AnalyzedFormula,
    source: &str,
    mode: FormulaMode,
) {
    assert_eq!(analyzed.source, source);
    assert_eq!(analyzed.mode, mode);
    assert_eq!(analyzed.is_supported(), analyzed.unsupported.is_empty());

    let mut cursor = 0usize;
    for token in &analyzed.tokens {
        assert_eq!(token.start, cursor, "token gap or overlap: {source:?}");
        assert!(token.start < token.end, "empty token: {source:?}");
        assert!(source.is_char_boundary(token.start));
        assert!(source.is_char_boundary(token.end));
        cursor = token.end;
    }
    assert_eq!(cursor, source.len(), "incomplete token coverage: {source:?}");
    assert_eq!(reconstructed(analyzed), source);

    let mut unsupported_end = 0usize;
    for item in &analyzed.unsupported {
        assert!(item.start >= unsupported_end, "unsupported order: {source:?}");
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
            "unsupported command must retain one exact literal token: {:?}",
            source,
        );
        unsupported_end = item.end;
    }
}

#[test]
fn small_mixed_sources_preserve_analysis_invariants() {
    const FRAGMENTS: &[&str] = &[
        "a",
        "ñ",
        "🙂",
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
                                &analyzed,
                                &source,
                                mode,
                            );
                        }
                    }
                }
            }
        }
    }
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
        r"\Pr", r"\arccos", r"\arcsin", r"\arctan", r"\arg", r"\bigcap",
        r"\bigcup", r"\bmod", r"\cos", r"\cosh", r"\cot", r"\coth", r"\csc",
        r"\deg", r"\det", r"\dim", r"\exp", r"\gcd", r"\hom", r"\iiint",
        r"\iint", r"\inf", r"\int", r"\ker", r"\lg", r"\lim", r"\liminf",
        r"\limsup", r"\ln", r"\log", r"\max", r"\min", r"\mod", r"\oint",
        r"\prod", r"\sec", r"\sin", r"\sinh", r"\sum", r"\sup", r"\tan",
        r"\tanh",
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
fn extended_named_operators_compose_without_rewriting() {
    let source = concat!(
        r"\det(A) + \gcd(a,b) + \ker(T) + \dim(V); ",
        r"\sinh(x) + \cosh(x) + \tanh(x); ",
        r"\liminf a_n \le \limsup a_n; \Pr(A)",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("extended named operator expression");
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
        10,
    );
}

#[test]
fn modular_arithmetic_commands_preserve_structure() {
    let source = r"a \bmod m; b \mod n; x \pmod{17}";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("modular arithmetic expression");
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
        2,
    );
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind
            == MathTokenKind::Command(SupportedCommand::ParenthesizedModulo)
            && analyzed.token_source(*token) == Some(r"\pmod")
    }));
    assert_eq!(
        analyze(r"\pmod", FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: r"\pmod".len(),
            kind: MathSyntaxErrorKind::MissingRequiredGroup,
        }),
    );
    let prefixed = analyze(r"\pmodulus{17}", FormulaMode::Inline)
        .expect("balanced unsupported pmod prefix");
    assert!(!prefixed.is_supported());
    assert_eq!(prefixed.unsupported[0].name, r"\pmodulus");
}

#[test]
fn multiple_integral_and_large_set_operators_compose_without_rewriting() {
    let source = concat!(
        r"\iint_D f dA + \iiint_V \rho dV + \oint_C F \cdot dr; ",
        r"\bigcup_i A_i \supseteq \bigcap_i A_i",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("multiple integral and large-set operator expression");
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
        5,
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
        r"\Delta", r"\Downarrow", r"\Gamma", r"\Im", r"\Lambda", r"\Leftarrow",
        r"\Leftrightarrow", r"\Longleftarrow", r"\Longleftrightarrow",
        r"\Longrightarrow", r"\Omega", r"\Phi", r"\Pi", r"\Psi", r"\Re",
        r"\Rightarrow",
        r"\Sigma", r"\Theta", r"\Uparrow", r"\Updownarrow", r"\Upsilon",
        r"\Vert", r"\Xi", r"\alpha", r"\angle", r"\approx", r"\ast",
        r"\backslash", r"\because", r"\beta", r"\bullet", r"\cap", r"\cdot",
        r"\cdots",
        r"\chi", r"\circ", r"\cong", r"\cup", r"\dashv", r"\ddots", r"\delta",
        r"\div", r"\dots", r"\downarrow", r"\ell", r"\emptyset", r"\epsilon",
        r"\equiv", r"\eta", r"\exists", r"\forall", r"\gamma", r"\ge", r"\geq",
        r"\geqslant", r"\gets", r"\gg", r"\gtrsim", r"\hbar", r"\hookleftarrow",
        r"\hookrightarrow", r"\iff", r"\imath", r"\implies", r"\in", r"\infty",
        r"\iota", r"\jmath", r"\kappa", r"\lambda", r"\land", r"\langle",
        r"\lbrace", r"\lbrack", r"\lceil", r"\ldots", r"\le", r"\leftarrow",
        r"\leftrightarrow", r"\leq", r"\leqslant", r"\lesssim", r"\lfloor",
        r"\ll", r"\longleftarrow", r"\longleftrightarrow", r"\longrightarrow",
        r"\lor", r"\mapsto", r"\mid", r"\models", r"\mp", r"\mu", r"\nabla",
        r"\ne", r"\nearrow", r"\neg", r"\neq", r"\nexists", r"\ni", r"\notin",
        r"\nu", r"\nwarrow", r"\omega", r"\oplus", r"\otimes", r"\parallel",
        r"\partial", r"\perp", r"\phi", r"\pi", r"\pm", r"\prec", r"\preceq",
        r"\prime", r"\propto", r"\psi", r"\rangle", r"\rbrace", r"\rbrack",
        r"\rceil", r"\rfloor",
        r"\rho", r"\rightarrow", r"\searrow", r"\setminus", r"\sigma", r"\sim",
        r"\simeq", r"\star", r"\subset", r"\subseteq", r"\subsetneq", r"\succ",
        r"\succeq", r"\supset", r"\supseteq", r"\supsetneq", r"\swarrow",
        r"\tau", r"\therefore", r"\theta", r"\times", r"\to", r"\triangle",
        r"\uparrow", r"\updownarrow", r"\upsilon", r"\varepsilon", r"\varphi",
        r"\varpi", r"\varrho", r"\varsigma", r"\vartheta", r"\vdash", r"\vdots",
        r"\vee", r"\vert", r"\wedge", r"\xi", r"\zeta",
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
fn complex_and_prime_symbols_compose_without_rewriting() {
    let source = r"z = \Re(w) + i\Im(w); f^\prime(x)";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("complex and prime symbol expression");
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
}

#[test]
fn geometry_and_proof_symbols_compose_without_rewriting() {
    let source = concat!(
        r"\angle ABC \simeq \triangle DEF; ",
        r"P \implies Q \iff R; \because x \nexists A \therefore y; ",
        r"v \vdash P, M \models Q; \ell + \hbar + \imath + \jmath",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("geometry and proof symbol expression");
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
fn ellipsis_symbols_compose_in_matrix_notation_without_rewriting() {
    let source = concat!(
        r"\begin{matrix}a_1 & \cdots & a_n \\ ",
        r"\vdots & \ddots & \vdots \\ ",
        r"b_1 & \dots & b_n\end{matrix}; x \ldots y",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("ellipsis matrix expression");
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
fn common_arrow_symbols_compose_without_rewriting() {
    let source = concat!(
        r"A \Leftarrow B \leftrightarrow C \Longrightarrow D; ",
        r"x \mapsto y; u \uparrow v \downarrow w; ",
        r"p \nearrow q \searrow r; a \hookrightarrow b",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("arrow symbol expression");
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
}

#[test]
fn layout_and_style_commands_remain_explicitly_unsupported() {
    for (source, unsupported) in [
        (r"\phantom{x}", r"\phantom"),
        (r"\hspace{1em}", r"\hspace"),
        (r"\color{red}{x}", r"\color"),
        (r"x\,y", r"\,"),
        (r"x\!y", r"\!"),
    ] {
        let analyzed = analyze(source, FormulaMode::Inline)
            .expect("balanced layout-affecting source");
        assert!(!analyzed.is_supported(), "{source}");
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1, "{source}");
        assert_eq!(analyzed.unsupported[0].name, unsupported, "{source}");
    }
}

#[test]
fn sized_delimiter_commands_remain_explicitly_unsupported() {
    let source = r"\left\langle x \right\rangle";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("balanced sized-delimiter source");
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
}

#[test]
fn named_delimiters_compose_without_rewriting() {
    let source = concat!(
        r"\langle x, y \rangle; \lceil x \rceil; ",
        r"\lfloor y \rfloor; \lbrace A \rbrace; ",
        r"\lbrack z \rbrack; A \backslash B; ",
        r"\vert x \vert + \Vert v \Vert",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("named delimiter expression");
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
        15,
    );
}

#[test]
fn common_binary_operator_symbols_compose_without_rewriting() {
    let source = concat!(
        r"a \ast b + c \bullet d + e \circ f; ",
        r"x \div y \mp z; A \oplus B \otimes C; ",
        r"S \setminus T; p \star q; P \vee Q \wedge R",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("binary operator symbol expression");
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
fn order_and_set_relation_symbols_compose_without_rewriting() {
    let source = concat!(
        r"a \ll b \leqslant c \lesssim d; x \gg y \geqslant z \gtrsim w; ",
        r"p \prec q \preceq r; s \succ t \succeq u; ",
        r"A \subsetneq B, B \supsetneq A, x \ni y; P \dashv Q",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("order and set relation expression");
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
        (r"\angles", r"\angles"),
        (r"\binomial{n}{k}", r"\binomial"),
        (r"\bmodulo", r"\bmodulo"),
        (r"\iints", r"\iints"),
        (r"\impliesx", r"\impliesx"),
        (r"\infinite", r"\infinite"),
        (r"\integral_0^1", r"\integral"),
        (r"\leqslanted", r"\leqslanted"),
        (r"\nablax", r"\nablax"),
        (r"\overlined{x}", r"\overlined"),
        (r"\pmodulus{17}", r"\pmodulus"),
        (r"\Rightarrowed", r"\Rightarrowed"),
        (r"\subsetequal", r"\subsetequal"),
        (r"\thereforex", r"\thereforex"),
        (r"\vectored{x}", r"\vectored"),
        (r"\emptysets", r"\emptysets"),
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
        "\\pmod \t{17}",
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
fn extended_accents_are_structural_and_require_one_group() {
    let source = concat!(
        r"\acute{x}+\breve{y}+\check{z}+",
        r"\grave{a}+\mathring{b}",
    );
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("extended grouped accents");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    for (spelling, kind) in [
        (r"\acute", SupportedCommand::Acute),
        (r"\breve", SupportedCommand::Breve),
        (r"\check", SupportedCommand::Check),
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
}

#[test]
fn wide_accents_are_structural_and_require_one_group() {
    let source = r"\widehat{ABC}+\widetilde{xyz}";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("grouped wide accents");
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
}

#[test]
fn grouped_math_alphabets_are_structural_and_require_one_group() {
    let source = concat!(
        r"x \in \mathbb{R}, \mathbf{v}, \mathcal{F}, \mathit{x}, ",
        r"\mathfrak{g}, \mathsf{A}, \mathtt{id}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("grouped mathematical alphabets");
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
fn directional_over_arrows_are_structural_and_require_one_group() {
    let source = concat!(
        r"\overleftarrow{AB}+",
        r"\overleftrightarrow{CD}+",
        r"\overrightarrow{EF}",
    );
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("grouped directional over-arrows");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    for (spelling, kind) in [
        (r"\overleftarrow", SupportedCommand::OverLeftArrow),
        (
            r"\overleftrightarrow",
            SupportedCommand::OverLeftRightArrow,
        ),
        (r"\overrightarrow", SupportedCommand::OverRightArrow),
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
}

#[test]
fn boxed_and_brace_decorations_preserve_grouped_structure() {
    let source = concat!(
        r"\boxed{x+1} + \overbrace{a+b}^{sum} + ",
        r"\underbrace{x+y}_{pair}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("boxed and brace decoration formula");
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
fn stacked_relation_is_structural_and_requires_two_groups() {
    let source = r"a\stackrel{!}{=}b";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("stacked relation annotation");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::StackRelation)
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
}

#[test]
fn stacked_annotations_are_structural_and_require_two_groups() {
    let source = r"\overset{!}{=} + \underset{n\to\infty}{\lim}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("stacked annotation formula");
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
    for source in [r"\overset{!}", r"\underset{n}"] {
        assert_eq!(
            analyze(source, FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: source.len(),
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
fn indexed_square_root_preserves_structural_index_and_radicand() {
    let source = r"x = \sqrt[3]{a^2 + b^2}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("indexed square-root formula");
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
}

#[test]
fn indexed_square_root_scans_index_content_and_preserves_whitespace() {
    let source = r"\sqrt [\alpha+{3]}] {x}";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("indexed root with protected closing bracket");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::NamedSymbol)
            && analyzed.token_source(*token) == Some(r"\alpha")
    }));

    let unsupported = r"\sqrt[\unknown]{x}";
    let analyzed = analyze(unsupported, FormulaMode::Inline)
        .expect("unsupported index command remains analyzable");
    assert!(!analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), unsupported);
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\unknown");
}

#[test]
fn indexed_square_root_closure_respects_backslash_parity() {
    let protected = r"\sqrt[1\]2]{x}";
    let analyzed = analyze(protected, FormulaMode::Inline)
        .expect("control symbol protects root-index closing bracket");
    assert!(!analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), protected);
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\]");
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
    let analyzed = analyze(even_slashes, FormulaMode::Aligned)
        .expect("row break ends before root-index closing bracket");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), even_slashes);
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| token.kind == MathTokenKind::RowBreak)
            .count(),
        1,
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
        analyze(even_slashes, FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: r"\sqrt[1".len(),
            kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
        }),
    );
}

#[test]
fn indexed_square_root_boundaries_are_scoped_without_global_brackets() {
    let nested = r"\sqrt[{\sqrt[3]{x}}]{y}";
    let analyzed = analyze(nested, FormulaMode::Inline)
        .expect("brace-contained nested indexed root");
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
        .expect("ordinary square brackets remain literal");
    assert_eq!(ordinary.tokens.len(), 1);
    assert_eq!(ordinary.tokens[0].kind, MathTokenKind::Literal);

    let overlapping = r"\sqrt[\sqrt[3]{x}]{y}";
    assert_eq!(
        analyze(overlapping, FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: overlapping.len(),
            kind: MathSyntaxErrorKind::MissingRootIndexEnd,
        }),
    );
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
        .expect("deep nested required groups");
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
}

#[test]
fn deep_indexed_square_roots_are_iterative_and_balanced() {
    let depth = 4_096usize;
    let mut source = String::new();
    for _ in 0..depth {
        source.push_str(r"\sqrt[{");
    }
    source.push('2');
    for _ in 0..depth {
        source.push_str(r"}]{x}");
    }
    let analyzed = analyze(&source, FormulaMode::Inline)
        .expect("deep indexed square-root source");
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
}

#[test]
fn styled_binomials_are_structural_and_require_two_groups() {
    let source = r"\dbinom{n}{k}+\tbinom{a}{b}";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("styled binomial coefficients");
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
        for malformed in [spelling.to_owned(), format!("{spelling}{{a}}")]
        {
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
fn styled_fractions_are_structural_and_require_two_groups() {
    let source = r"\dfrac{a}{b} + \tfrac{x+1}{y-1}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("styled fraction formula");
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
    for source in [r"\dfrac{a}", r"\tfrac{x}"] {
        assert_eq!(
            analyze(source, FormulaMode::Inline),
            Err(MathSyntaxError {
                byte_offset: source.len(),
                kind: MathSyntaxErrorKind::MissingRequiredGroup,
            }),
            "{source}",
        );
    }
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
fn substack_rows_are_scoped_to_their_group() {
    let source = r"\sum_{\substack{i=1\\j=2}} a_{ij}";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("substack row formula");
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
    let analyzed = analyze(nested, FormulaMode::Inline)
        .expect("nested substack rows");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), nested);
    assert_eq!(
        analyzed
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
}

#[test]
fn outer_environments_do_not_leak_alignment_into_substack_scope() {
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
                byte_offset: source.find('&').expect("alignment marker"),
                kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
            }),
            "{source}",
        );
    }
}

#[test]
fn substack_scope_composes_with_text_and_matrix_structure() {
    let with_text = r"\substack{a\\\text{b\\c}\\d}";
    let analyzed = analyze(with_text, FormulaMode::Inline)
        .expect("substack containing text scope");
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
    let analyzed = analyze(with_matrix, FormulaMode::Inline)
        .expect("substack containing matrix");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), with_matrix);
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| token.kind == MathTokenKind::RowBreak)
            .count(),
        2,
    );
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| token.kind == MathTokenKind::AlignmentPoint)
            .count(),
        2,
    );

    let in_matrix = r"\begin{matrix}\substack{a\\b} & c\end{matrix}";
    let analyzed = analyze(in_matrix, FormulaMode::Display)
        .expect("matrix containing substack");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), in_matrix);
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| token.kind == MathTokenKind::RowBreak)
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
}

#[test]
fn deep_substack_scopes_are_iterative_and_do_not_leak() {
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
        .expect("deep nested substack source");
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
}

#[test]
fn text_scope_still_makes_substack_row_breaks_literal() {
    let source = r"\text{label \substack{a\\b}}";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("substack command nested in text scope");
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
fn escaped_braces_do_not_close_text_scope() {
    let source = r"\text{\{label_a\}} + x_1";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("escaped braces inside text");
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
    assert_eq!(
        analyzed
            .tokens
            .iter()
            .filter(|token| token.kind == MathTokenKind::Subscript)
            .count(),
        1,
    );
}

#[test]
fn nested_text_groups_keep_math_markers_literal_until_outer_close() {
    let source = concat!(
        r"\text{outer_a^2 & \text{inner_b^3 & value} tail_c} + ",
        r"y^2 & z",
    );
    let analyzed = analyze(source, FormulaMode::Aligned)
        .expect("nested text followed by aligned mathematics");
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
    assert!(!analyzed
        .tokens
        .iter()
        .any(|token| token.kind == MathTokenKind::Subscript));
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
fn aligned_environment_admits_rows_and_alignment_in_display_mode() {
    let source = concat!(
        r"\begin{aligned}y &= (3x^2+1)^5 \\ ",
        r"y' &= 30x(3x^2+1)^4\end{aligned}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("aligned environment in display formula");
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
}

#[test]
fn braced_matrix_admits_rows_and_alignment() {
    let source = r"A=\begin{Bmatrix}a & b \\ c & d\end{Bmatrix}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("brace-delimited matrix formula");
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
}

#[test]
fn bracketed_matrix_admits_rows_and_alignment() {
    let source = r"A=\begin{bmatrix}a & b \\ c & d\end{bmatrix}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("bracketed matrix formula");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind
            == MathTokenKind::Command(SupportedCommand::BeginBracketedMatrix)
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
}

#[test]
fn parenthesized_matrix_admits_rows_and_alignment() {
    let source = r"A=\begin{pmatrix}a & b \\ c & d\end{pmatrix}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("parenthesized matrix formula");
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
}

#[test]
fn double_vertical_matrix_admits_rows_and_alignment() {
    let source = r"D=\begin{Vmatrix}a & b \\ c & d\end{Vmatrix}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("double-vertical-bar matrix formula");
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
            == MathTokenKind::Command(SupportedCommand::EndDoubleVerticalMatrix)
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
}

#[test]
fn vertical_matrix_admits_rows_and_alignment() {
    let source = r"D=\begin{vmatrix}a & b \\ c & d\end{vmatrix}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("vertical-bar matrix formula");
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
}

#[test]
fn small_matrix_admits_rows_and_alignment_with_distinct_identity() {
    let source = r"A=\begin{smallmatrix}a & b \\ c & d\end{smallmatrix}";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("compact small-matrix formula");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::BeginSmallMatrix)
    }));
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::EndSmallMatrix)
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
}

#[test]
fn split_environment_admits_derivation_rows_and_alignment() {
    let source = concat!(
        r"\begin{split}f(x) &= x^2 + 1 \\ ",
        r"f'(x) &= 2x\end{split}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("split derivation environment");
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
}

#[test]
fn gathered_scope_yields_to_nested_split_alignment() {
    let source = concat!(
        r"\begin{gathered}\begin{split}a &= b \\ c &= d\end{split}",
        r" \\ e\end{gathered}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("split nested inside gathered rows");
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
}

#[test]
fn gathered_environment_admits_rows_but_not_column_alignment() {
    let source = r"\begin{gathered}a \\ b\end{gathered}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("gathered row structure");
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
        token.kind == MathTokenKind::Command(SupportedCommand::BeginGathered)
    }));
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::EndGathered)
    }));

    let column = r"\begin{gathered}a & b\end{gathered}";
    assert_eq!(
        analyze(column, FormulaMode::Aligned),
        Err(MathSyntaxError {
            byte_offset: column.find('&').expect("alignment marker"),
            kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
        }),
    );
}

#[test]
fn gathered_scope_yields_to_nested_column_environment() {
    let source = concat!(
        r"\begin{gathered}\begin{matrix}a & b\end{matrix} \\ ",
        r"c\end{gathered}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("matrix nested inside gathered rows");
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
}

#[test]
fn gathered_scope_overrides_outer_column_environment() {
    let source = concat!(
        r"\begin{aligned}x &= \begin{gathered}a & b",
        r"\end{gathered}\end{aligned}",
    );
    assert_eq!(
        analyze(source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: source.rfind('&').expect("gathered alignment marker"),
            kind: MathSyntaxErrorKind::AlignmentOutsideStructure,
        }),
    );
}

#[test]
fn text_scope_remains_literal_inside_gathered_environment() {
    let source = concat!(
        r"\begin{gathered}\text{a & b \\ c} \\ ",
        r"d\end{gathered}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("gathered rows containing text scope");
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
    let source = concat!(
        r"f(x)=\begin{cases}x & x>0 \\ ",
        r"-x & x\le 0\end{cases}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("cases formula");
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
}

#[test]
fn aligned_cases_and_matrix_environments_nest_in_order() {
    let source = concat!(
        r"\begin{aligned}f(x) &= \begin{cases}x & x>0 \\ ",
        r"\begin{matrix}a & b \\ c & d\end{matrix} & x<=0",
        r"\end{cases}\end{aligned}",
    );
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("ordered nested aligned cases and matrix environments");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
}

#[test]
fn small_and_plain_matrices_keep_distinct_nesting() {
    let source = concat!(
        r"\begin{smallmatrix}a & \begin{matrix}b & c \\ d & e",
        r"\end{matrix} \\ f & g\end{smallmatrix}",
    );
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("nested compact and plain matrices");
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
}

#[test]
fn single_and_double_vertical_matrices_keep_distinct_nesting() {
    let source = concat!(
        r"\begin{Vmatrix}a & \begin{vmatrix}b & c \\ d & e",
        r"\end{vmatrix} \\ f & g\end{Vmatrix}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("nested double- and single-vertical-bar matrices");
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
}

#[test]
fn vertical_and_bracketed_matrices_keep_distinct_nesting() {
    let source = concat!(
        r"\begin{vmatrix}a & \begin{bmatrix}b & c \\ d & e",
        r"\end{bmatrix} \\ f & g\end{vmatrix}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("nested vertical-bar and bracketed matrices");
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
}

#[test]
fn braced_and_bracketed_matrices_keep_distinct_nesting() {
    let source = concat!(
        r"\begin{Bmatrix}a & \begin{bmatrix}b & c \\ d & e",
        r"\end{bmatrix} \\ f & g\end{Bmatrix}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("nested brace-delimited and bracketed matrices");
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
}

#[test]
fn bracketed_and_parenthesized_matrices_keep_distinct_nesting() {
    let source = concat!(
        r"\begin{bmatrix}a & \begin{pmatrix}b & c \\ d & e",
        r"\end{pmatrix} \\ f & g\end{bmatrix}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("nested bracketed and parenthesized matrices");
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
}

#[test]
fn parenthesized_and_plain_matrices_nest_in_order() {
    let source = concat!(
        r"\begin{pmatrix}a & \begin{matrix}b & c \\ d & e",
        r"\end{matrix} \\ f & g\end{pmatrix}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("nested parenthesized and plain matrices");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
}

#[test]
fn cases_and_matrix_environments_nest_in_order() {
    let source = concat!(
        r"\begin{cases}a & \begin{matrix}b & c \\ d & e",
        r"\end{matrix} \\ f & g\end{cases}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("nested cases and matrix formula");
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
}

#[test]
fn crossed_environment_closes_are_typed() {
    let parenthesized_first = r"\begin{pmatrix}\begin{matrix}x";
    let parenthesized_source =
        format!(r"{parenthesized_first}\end{{pmatrix}}");
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

    let first = r"\begin{cases}\begin{matrix}x";
    let source = format!(r"{first}\end{{cases}}");
    assert_eq!(
        analyze(&source, FormulaMode::Display),
        Err(MathSyntaxError {
            byte_offset: first.len(),
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
        .expect("group closes before containing environment");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), properly_nested);
}

#[test]
fn environments_must_close_before_their_owning_group() {
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
        .expect("ordinary group nested inside environment");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), nested_group);
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
    let source = concat!(
        r"\begin{cases}\text{a & b \\ c} & d \\ ",
        r"e & f\end{cases}",
    );
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("cases containing text scope");
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
fn deep_ordered_environments_are_iterative_and_balanced() {
    let depth = 1_024usize;
    let mut source = String::new();
    for level in 0..depth {
        match level % 11 {
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
        match level % 11 {
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
        .expect("deep ordered environments");
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
    for (source, unsupported) in [
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
        let analyzed = analyze(source, FormulaMode::Aligned)
            .expect("balanced near-match environment source");
        assert!(!analyzed.is_supported(), "{source}");
        assert_eq!(reconstructed(&analyzed), source);
        assert_eq!(analyzed.unsupported.len(), 1, "{source}");
        assert_eq!(analyzed.unsupported[0].name, unsupported, "{source}");
    }
}

#[test]
fn unknown_matrix_environment_remains_explicit_unsupported_source() {
    let source = r"\begin{array}a & b";
    let analyzed = analyze(source, FormulaMode::Aligned)
        .expect("balanced unknown matrix environment source");
    assert!(!analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\begin");
    assert!(!analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::BeginMatrix)
    }));
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
fn ascii_control_words_remain_case_sensitive() {
    let source = r"\Pr(A) + \pr(A)";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("case-sensitive control-word source");
    assert!(!analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(analyzed.unsupported.len(), 1);
    assert_eq!(analyzed.unsupported[0].name, r"\pr");
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::NamedOperator)
            && analyzed.token_source(*token) == Some(r"\Pr")
    }));
}

#[test]
fn ascii_control_word_boundary_preserves_adjacent_unicode() {
    let source = r"\alphaβ + \detñ + \mathbb{R}λ";
    let analyzed = analyze(source, FormulaMode::Inline)
        .expect("supported commands followed by Unicode");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);

    let unsupported_source = r"\alphaZβ";
    let unsupported = analyze(unsupported_source, FormulaMode::Inline)
        .expect("balanced longer ASCII control word with Unicode suffix");
    assert!(!unsupported.is_supported());
    assert_eq!(reconstructed(&unsupported), unsupported_source);
    assert_eq!(unsupported.unsupported.len(), 1);
    assert_eq!(unsupported.unsupported[0].name, r"\alphaZ");
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
