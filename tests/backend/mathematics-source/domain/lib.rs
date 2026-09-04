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
fn named_symbol_vocabulary_is_supported_without_rewriting() {
    for source in [
        r"\alpha", r"\approx", r"\beta", r"\cdot", r"\delta",
        r"\epsilon", r"\gamma", r"\ge", r"\geq", r"\infty",
        r"\lambda", r"\le", r"\leq", r"\mu", r"\ne", r"\neq",
        r"\omega", r"\phi", r"\pi", r"\pm", r"\rho", r"\sigma",
        r"\theta", r"\times",
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
