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
//   - Exact-source TeX-compatible mathematical structural analysis.
// - Must-Not:
//   - Rewrite authored mathematics, choose glyphs, measure formulas, or render.
//   - Claim unsupported TeX commands are equivalent to supported notation.
// - Allows:
//   - Inputs: UTF-8 mathematical source plus one semantic presentation mode.
//   - Outputs: Source-preserving tokens or typed unsupported/syntax evidence.
//   - Side effects: Process-local result allocation only.
// - Split-When:
//   - Expression semantics or formula measurement becomes independently
//     complex.
// - Merge-When:
//   - The semantic notebook fully owns mathematical source parsing.
// - Summary:
//   - Preserves editable math source while exposing admitted structure safely.
// - Description:
//   - Recognizes a narrow first-release TeX-compatible structural vocabulary.
// - Usage:
//   - Analyze source before admitting it as supported semantic mathematics.
// - Defaults:
//   - Unknown commands are explicit unsupported constructs, never substituted.
//

//! Exact-source structural analysis for editable mathematical content.

/// Complete source-preserving analysis of one mathematical unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyzedFormula {
    /// Semantic presentation family supplied by the owning notebook.
    pub mode: FormulaMode,
    /// Exact authored UTF-8 source, byte-for-byte unchanged.
    pub source: String,
    /// Ordered structural tokens whose spans cover the complete source.
    pub tokens: Vec<MathToken>,
    /// Unsupported commands in source order; empty means admitted vocabulary.
    pub unsupported: Vec<UnsupportedConstruct>,
}

/// Semantic presentation family for one mathematical source unit.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FormulaMode {
    /// Multi-row mathematics with explicit alignment points.
    Aligned,
    /// Standalone displayed formula.
    Display,
    /// Formula embedded in surrounding prose.
    Inline,
}

/// Typed source-shape failure that must not be silently repaired.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathSyntaxError {
    /// UTF-8 byte offset where validation first proved the source malformed.
    pub byte_offset: usize,
    /// Structural failure class.
    pub kind: MathSyntaxErrorKind,
}

/// Structural failure classes for the admitted TeX-compatible subset.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MathSyntaxErrorKind {
    /// Alignment marker appears outside aligned or matrix content.
    AlignmentOutsideStructure,
    /// A closing group appears without a matching open group.
    ExtraGroupClose,
    /// Matrix environment closes without a matching matrix start.
    ExtraMatrixEnd,
    /// Matrix environment remains open at end of source.
    MissingMatrixEnd,
    /// A supported command is missing one or more required braced arguments.
    MissingRequiredGroup,
    /// An ordinary source group remains open at end of source.
    UnclosedGroup,
}

/// One source-preserving structural token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathToken {
    /// Exclusive UTF-8 byte offset in the unchanged source.
    pub end: usize,
    /// Structural meaning admitted for this span.
    pub kind: MathTokenKind,
    /// Inclusive UTF-8 byte offset in the unchanged source.
    pub start: usize,
}

/// Structural token kind with exact source span retained separately.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MathTokenKind {
    /// Column alignment point in aligned or matrix content.
    AlignmentPoint,
    /// Supported TeX-compatible control sequence.
    Command(SupportedCommand),
    /// Closing brace of an ordinary source group.
    GroupClose,
    /// Opening brace of an ordinary source group.
    GroupOpen,
    /// Ordinary literal source that is not structural syntax.
    Literal,
    /// Explicit mathematical row break.
    RowBreak,
    /// Subscript marker.
    Subscript,
    /// Superscript marker.
    Superscript,
}

#[derive(Clone, Copy)]
struct ScanState {
    group_depth: usize,
    index: usize,
    literal_start: usize,
    matrix_depth: usize,
}

#[derive(Clone, Copy)]
struct ScannedCommand {
    end: usize,
    kind: ScannedCommandKind,
}

#[derive(Clone, Copy)]
enum ScannedCommandKind {
    RowBreak,
    Supported(SupportedCommand),
    Unsupported,
}

/// One supported TeX-compatible control sequence or environment marker.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SupportedCommand {
    /// Start of an explicit matrix environment.
    BeginMatrix,
    /// End of an explicit matrix environment.
    EndMatrix,
    /// Two-group fraction command.
    Fraction,
    /// Roman/upright grouped content, useful for units and labels.
    Roman,
    /// One-group square-root command.
    SquareRoot,
}

/// One unknown TeX-like command retained as unsupported input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedConstruct {
    /// Exclusive UTF-8 byte offset in the unchanged source.
    pub end: usize,
    /// Exact command name including its leading backslash.
    pub name: String,
    /// Inclusive UTF-8 byte offset in the unchanged source.
    pub start: usize,
}

impl AnalyzedFormula {
    /// Whether every recognized command belongs to the admitted vocabulary.
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        self.unsupported.is_empty()
    }

    /// Return one token span from this analysis's unchanged source.
    #[must_use]
    pub fn token_source(&self, token: MathToken) -> Option<&str> {
        self.source.get(token.start..token.end)
    }
}

/// Analyze one exact mathematical source without normalizing or rewriting it.
///
/// Supported source includes ordinary Unicode mathematics, groups, scripts,
/// `\\frac{...}{...}`, `\\sqrt{...}`, `\\mathrm{...}`, aligned separators,
/// and `\\begin{matrix}...\\end{matrix}`. Other commands remain present in the
/// token stream and are reported through [`AnalyzedFormula::unsupported`].
///
/// # Errors
///
/// Returns the first typed structural failure for unmatched groups or matrix
/// boundaries, missing required braced arguments, or alignment syntax used in a
/// context that does not admit it.
pub fn analyze(
    source: &str,
    mode: FormulaMode,
) -> Result<AnalyzedFormula, MathSyntaxError> {
    let mut state = ScanState {
        group_depth: 0,
        index: 0,
        literal_start: 0,
        matrix_depth: 0,
    };
    let mut tokens = Vec::new();
    let mut unsupported = Vec::new();
    while state.index < source.len() {
        let Some(tail) = source.get(state.index..) else {
            return Err(error(state.index, MathSyntaxErrorKind::UnclosedGroup));
        };
        let Some(character) = tail.chars().next() else {
            break;
        };
        if !is_structural(character) {
            state.index = state.index.saturating_add(character.len_utf8());
            continue;
        }
        push_literal(&mut tokens, state.literal_start, state.index);
        scan_structural(
            source,
            mode,
            character,
            &mut state,
            &mut tokens,
            &mut unsupported,
        )?;
        state.literal_start = state.index;
    }
    push_literal(&mut tokens, state.literal_start, source.len());
    validate_final_state(source.len(), state)?;
    debug_assert!(
        token_coverage(&tokens) == source.len(),
        "mathematical token spans must cover the unchanged source"
    );
    Ok(AnalyzedFormula {
        mode,
        source: source.to_owned(),
        tokens,
        unsupported,
    })
}

fn command_matches(source: &str, start: usize, spelling: &str) -> bool {
    let Some(tail) = source.get(start..) else {
        return false;
    };
    if !tail.starts_with(spelling) {
        return false;
    }
    if spelling.ends_with('}') {
        return true;
    }
    tail.get(spelling.len()..)
        .and_then(|rest| rest.chars().next())
        .is_none_or(|character| !character.is_ascii_alphabetic())
}

const fn error(
    byte_offset: usize,
    kind: MathSyntaxErrorKind,
) -> MathSyntaxError {
    MathSyntaxError { byte_offset, kind }
}

const fn is_structural(character: char) -> bool {
    matches!(character, '{' | '}' | '^' | '_' | '&' | '\\')
}

fn matching_group_end(
    source: &str,
    open: usize,
) -> Result<usize, MathSyntaxError> {
    let Some(bytes) = source.as_bytes().get(open..) else {
        return Err(error(open, MathSyntaxErrorKind::MissingRequiredGroup));
    };
    let mut depth = 0usize;
    for (offset, byte) in bytes.iter().copied().enumerate() {
        match byte {
            b'{' => depth = depth.saturating_add(1),
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(open.saturating_add(offset).saturating_add(1));
                }
            },
            _ => {},
        }
    }
    Err(error(source.len(), MathSyntaxErrorKind::UnclosedGroup))
}

fn push_literal(tokens: &mut Vec<MathToken>, start: usize, end: usize) {
    if start < end {
        tokens.push(token(start, end, MathTokenKind::Literal));
    }
}

fn scan_command(source: &str, start: usize) -> ScannedCommand {
    let after_slash = start.saturating_add(1);
    if source.as_bytes().get(after_slash) == Some(&b'\\') {
        return ScannedCommand {
            end: after_slash.saturating_add(1),
            kind: ScannedCommandKind::RowBreak,
        };
    }
    for (spelling, supported) in [
        ("\\begin{matrix}", SupportedCommand::BeginMatrix),
        ("\\end{matrix}", SupportedCommand::EndMatrix),
        ("\\frac", SupportedCommand::Fraction),
        ("\\mathrm", SupportedCommand::Roman),
        ("\\sqrt", SupportedCommand::SquareRoot),
    ] {
        if command_matches(source, start, spelling) {
            return ScannedCommand {
                end: start.saturating_add(spelling.len()),
                kind: ScannedCommandKind::Supported(supported),
            };
        }
    }
    let mut end = after_slash;
    let tail = source.get(after_slash..).unwrap_or_default();
    for (offset, character) in tail.char_indices() {
        if !character.is_ascii_alphabetic() {
            break;
        }
        end = after_slash
            .saturating_add(offset)
            .saturating_add(character.len_utf8());
    }
    if end == after_slash {
        end = tail.chars().next().map_or(after_slash, |character| {
            after_slash.saturating_add(character.len_utf8())
        });
    }
    ScannedCommand {
        end,
        kind: ScannedCommandKind::Unsupported,
    }
}

fn scan_structural(
    source: &str,
    mode: FormulaMode,
    character: char,
    state: &mut ScanState,
    tokens: &mut Vec<MathToken>,
    unsupported: &mut Vec<UnsupportedConstruct>,
) -> Result<(), MathSyntaxError> {
    let width = character.len_utf8();
    match character {
        '&' => scan_alignment(mode, state, tokens, width),
        '^' => {
            scan_marker(state, tokens, width, MathTokenKind::Superscript);
            Ok(())
        },
        '_' => {
            scan_marker(state, tokens, width, MathTokenKind::Subscript);
            Ok(())
        },
        '{' => {
            state.group_depth = state.group_depth.saturating_add(1);
            scan_marker(state, tokens, width, MathTokenKind::GroupOpen);
            Ok(())
        },
        '}' => scan_group_close(state, tokens, width),
        '\\' => scan_slash(source, mode, state, tokens, unsupported),
        _ => {
            state.index = state.index.saturating_add(width);
            Ok(())
        },
    }
}

fn scan_alignment(
    mode: FormulaMode,
    state: &mut ScanState,
    tokens: &mut Vec<MathToken>,
    width: usize,
) -> Result<(), MathSyntaxError> {
    if mode != FormulaMode::Aligned && state.matrix_depth == 0 {
        return Err(error(
            state.index,
            MathSyntaxErrorKind::AlignmentOutsideStructure,
        ));
    }
    scan_marker(state, tokens, width, MathTokenKind::AlignmentPoint);
    Ok(())
}

fn scan_group_close(
    state: &mut ScanState,
    tokens: &mut Vec<MathToken>,
    width: usize,
) -> Result<(), MathSyntaxError> {
    if state.group_depth == 0 {
        return Err(error(state.index, MathSyntaxErrorKind::ExtraGroupClose));
    }
    state.group_depth = state.group_depth.saturating_sub(1);
    scan_marker(state, tokens, width, MathTokenKind::GroupClose);
    Ok(())
}

fn scan_marker(
    state: &mut ScanState,
    tokens: &mut Vec<MathToken>,
    width: usize,
    kind: MathTokenKind,
) {
    let end = state.index.saturating_add(width);
    tokens.push(token(state.index, end, kind));
    state.index = end;
}

fn scan_slash(
    source: &str,
    mode: FormulaMode,
    state: &mut ScanState,
    tokens: &mut Vec<MathToken>,
    unsupported: &mut Vec<UnsupportedConstruct>,
) -> Result<(), MathSyntaxError> {
    let command = scan_command(source, state.index);
    match command.kind {
        ScannedCommandKind::RowBreak => {
            if mode != FormulaMode::Aligned && state.matrix_depth == 0 {
                return Err(error(
                    state.index,
                    MathSyntaxErrorKind::AlignmentOutsideStructure,
                ));
            }
            tokens.push(token(
                state.index,
                command.end,
                MathTokenKind::RowBreak,
            ));
        },
        ScannedCommandKind::Supported(supported) => {
            scan_supported_command(source, state, tokens, command, supported)?;
        },
        ScannedCommandKind::Unsupported => {
            tokens.push(token(
                state.index,
                command.end,
                MathTokenKind::Literal,
            ));
            unsupported.push(UnsupportedConstruct {
                end: command.end,
                name: source
                    .get(state.index..command.end)
                    .unwrap_or_default()
                    .to_owned(),
                start: state.index,
            });
        },
    }
    state.index = command.end;
    Ok(())
}

fn scan_supported_command(
    source: &str,
    state: &mut ScanState,
    tokens: &mut Vec<MathToken>,
    command: ScannedCommand,
    supported: SupportedCommand,
) -> Result<(), MathSyntaxError> {
    if supported == SupportedCommand::EndMatrix {
        if state.matrix_depth == 0 {
            return Err(error(
                state.index,
                MathSyntaxErrorKind::ExtraMatrixEnd,
            ));
        }
        state.matrix_depth = state.matrix_depth.saturating_sub(1);
    }
    tokens.push(token(
        state.index,
        command.end,
        MathTokenKind::Command(supported),
    ));
    validate_command_groups(source, command.end, supported)?;
    if supported == SupportedCommand::BeginMatrix {
        state.matrix_depth = state.matrix_depth.saturating_add(1);
    }
    Ok(())
}

fn skip_ascii_whitespace(source: &str, mut cursor: usize) -> usize {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(u8::is_ascii_whitespace)
    {
        cursor = cursor.saturating_add(1);
    }
    cursor
}

const fn token(start: usize, end: usize, kind: MathTokenKind) -> MathToken {
    MathToken { end, kind, start }
}

fn token_coverage(tokens: &[MathToken]) -> usize {
    tokens.iter().fold(0usize, |total, item| {
        total.saturating_add(item.end.saturating_sub(item.start))
    })
}

fn validate_command_groups(
    source: &str,
    command_end: usize,
    command: SupportedCommand,
) -> Result<(), MathSyntaxError> {
    let required = match command {
        SupportedCommand::BeginMatrix | SupportedCommand::EndMatrix => 0usize,
        SupportedCommand::Fraction => 2usize,
        SupportedCommand::Roman | SupportedCommand::SquareRoot => 1usize,
    };
    let mut cursor = command_end;
    for _ in 0..required {
        cursor = skip_ascii_whitespace(source, cursor);
        if source.as_bytes().get(cursor) != Some(&b'{') {
            return Err(error(
                cursor,
                MathSyntaxErrorKind::MissingRequiredGroup,
            ));
        }
        cursor = matching_group_end(source, cursor)?;
    }
    Ok(())
}

const fn validate_final_state(
    source_len: usize,
    state: ScanState,
) -> Result<(), MathSyntaxError> {
    if state.group_depth != 0 {
        return Err(error(source_len, MathSyntaxErrorKind::UnclosedGroup));
    }
    if state.matrix_depth != 0 {
        return Err(error(source_len, MathSyntaxErrorKind::MissingMatrixEnd));
    }
    Ok(())
}
