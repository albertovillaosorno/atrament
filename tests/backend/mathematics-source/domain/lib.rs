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
fn escaped_braces_do_not_corrupt_required_group_indexing() {
    let source = r"\frac{\{x\}}{2}";
    let analyzed = analyze(source, FormulaMode::Display)
        .expect("escaped brace commands remain balanced unsupported input");
    assert!(!analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert_eq!(
        analyzed
            .unsupported
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec![r"\{", r"\}"],
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
    let source = r"E = mc^2\text{ — energía total}";
    let analyzed =
        analyze(source, FormulaMode::Display).expect("text fragment formula");
    assert!(analyzed.is_supported());
    assert_eq!(reconstructed(&analyzed), source);
    assert!(analyzed.tokens.iter().any(|token| {
        token.kind == MathTokenKind::Command(SupportedCommand::Text)
    }));
    assert_eq!(
        analyze(r"x + \text", FormulaMode::Inline),
        Err(MathSyntaxError {
            byte_offset: 9,
            kind: MathSyntaxErrorKind::MissingRequiredGroup,
        }),
    );
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
