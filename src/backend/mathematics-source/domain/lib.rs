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

// Keep command tables lexically sorted for reviewable vocabulary changes.
const NAMED_OPERATOR_COMMANDS: &[&str] = &[
    "\\Pr", "\\arccos", "\\arcsin", "\\arctan", "\\arg", "\\cos", "\\cosh",
    "\\cot", "\\coth", "\\csc", "\\deg", "\\det", "\\dim", "\\exp", "\\gcd",
    "\\hom", "\\inf", "\\int", "\\ker", "\\lg", "\\lim", "\\liminf",
    "\\limsup", "\\ln", "\\log", "\\max", "\\min", "\\prod", "\\sec", "\\sin",
    "\\sinh", "\\sum", "\\sup", "\\tan", "\\tanh",
];

const NAMED_SYMBOL_COMMANDS: &[&str] = &[
    "\\Delta", "\\Downarrow", "\\Gamma", "\\Lambda", "\\Leftarrow",
    "\\Leftrightarrow", "\\Longleftarrow", "\\Longleftrightarrow",
    "\\Longrightarrow", "\\Omega", "\\Phi", "\\Pi", "\\Psi", "\\Rightarrow",
    "\\Sigma", "\\Theta", "\\Uparrow", "\\Updownarrow", "\\Upsilon", "\\Vert",
    "\\Xi", "\\alpha", "\\approx", "\\ast", "\\beta", "\\bullet", "\\cap",
    "\\cdot", "\\cdots", "\\chi", "\\circ", "\\cong", "\\cup", "\\ddots",
    "\\delta", "\\div", "\\dots", "\\downarrow", "\\emptyset", "\\epsilon",
    "\\equiv", "\\eta", "\\exists", "\\forall", "\\gamma", "\\ge", "\\geq",
    "\\hookleftarrow", "\\hookrightarrow", "\\in", "\\infty", "\\iota",
    "\\kappa", "\\lambda", "\\land", "\\langle", "\\lbrace", "\\lceil",
    "\\ldots", "\\le", "\\leftarrow", "\\leftrightarrow", "\\leq", "\\lfloor",
    "\\longleftarrow", "\\longleftrightarrow", "\\longrightarrow", "\\lor",
    "\\mapsto", "\\mid", "\\mp", "\\mu", "\\nabla", "\\ne", "\\nearrow",
    "\\neg", "\\neq", "\\notin", "\\nu", "\\nwarrow", "\\omega", "\\oplus",
    "\\otimes", "\\parallel", "\\partial", "\\perp", "\\phi", "\\pi", "\\pm",
    "\\propto", "\\psi", "\\rangle", "\\rbrace", "\\rceil", "\\rfloor",
    "\\rho", "\\rightarrow", "\\searrow", "\\setminus", "\\sigma", "\\sim",
    "\\star", "\\subset", "\\subseteq", "\\supset", "\\supseteq", "\\swarrow",
    "\\tau", "\\theta", "\\times", "\\to", "\\uparrow", "\\updownarrow",
    "\\upsilon", "\\varepsilon", "\\varphi", "\\varpi", "\\varrho",
    "\\varsigma", "\\vartheta", "\\vdots", "\\vee", "\\vert", "\\wedge",
    "\\xi", "\\zeta",
];

const STRUCTURED_CONTROL_WORD_COMMANDS: &[(&str, SupportedCommand)] = &[
    ("\\bar", SupportedCommand::Bar),
    ("\\binom", SupportedCommand::Binomial),
    ("\\ddot", SupportedCommand::DoubleDot),
    ("\\dot", SupportedCommand::Dot),
    ("\\frac", SupportedCommand::Fraction),
    ("\\hat", SupportedCommand::Hat),
    ("\\mathbb", SupportedCommand::BlackboardBold),
    ("\\mathbf", SupportedCommand::Bold),
    ("\\mathcal", SupportedCommand::Calligraphic),
    ("\\mathfrak", SupportedCommand::Fraktur),
    ("\\mathit", SupportedCommand::Italic),
    ("\\mathrm", SupportedCommand::Roman),
    ("\\mathsf", SupportedCommand::SansSerif),
    ("\\mathtt", SupportedCommand::Typewriter),
    ("\\operatorname", SupportedCommand::OperatorName),
    ("\\overline", SupportedCommand::Overline),
    ("\\overset", SupportedCommand::Overset),
    ("\\sqrt", SupportedCommand::SquareRoot),
    ("\\text", SupportedCommand::Text),
    ("\\tilde", SupportedCommand::Tilde),
    ("\\underline", SupportedCommand::Underline),
    ("\\underset", SupportedCommand::Underset),
    ("\\vec", SupportedCommand::Vector),
];

const STRUCTURED_ENVIRONMENT_COMMANDS: &[(&str, SupportedCommand)] = &[
    ("\\begin{matrix}", SupportedCommand::BeginMatrix),
    ("\\end{matrix}", SupportedCommand::EndMatrix),
];

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

#[derive(Debug)]
struct GroupIndex {
    pairs: Vec<(usize, usize)>,
}

#[derive(Clone, Copy)]
struct ScanState {
    group_depth: usize,
    index: usize,
    literal_start: usize,
    matrix_depth: usize,
    pending_text_group: bool,
    text_group_depth: Option<usize>,
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
    /// One-group bar accent.
    Bar,
    /// Start of an explicit matrix environment.
    BeginMatrix,
    /// Two-group binomial coefficient command.
    Binomial,
    /// One-group blackboard-bold mathematical alphabet.
    BlackboardBold,
    /// One-group bold mathematical alphabet.
    Bold,
    /// One-group calligraphic mathematical alphabet.
    Calligraphic,
    /// One-group dot accent.
    Dot,
    /// One-group double-dot accent.
    DoubleDot,
    /// End of an explicit matrix environment.
    EndMatrix,
    /// Escaped TeX special character preserved as literal source.
    EscapedSpecial,
    /// Two-group fraction command.
    Fraction,
    /// One-group Fraktur mathematical alphabet.
    Fraktur,
    /// One-group hat accent.
    Hat,
    /// One-group italic mathematical alphabet.
    Italic,
    /// Admitted no-argument named mathematical operator.
    NamedOperator,
    /// Admitted no-argument named mathematical symbol.
    NamedSymbol,
    /// One grouped custom mathematical operator name.
    OperatorName,
    /// One-group overline decoration.
    Overline,
    /// One annotation group placed above one grouped mathematical base.
    Overset,
    /// Roman/upright grouped content, useful for units and labels.
    Roman,
    /// One-group sans-serif mathematical alphabet.
    SansSerif,
    /// One-group square-root command.
    SquareRoot,
    /// One grouped text fragment preserved exactly inside mathematics.
    Text,
    /// One-group tilde accent.
    Tilde,
    /// One-group typewriter mathematical alphabet.
    Typewriter,
    /// One-group underline decoration.
    Underline,
    /// One annotation group placed below one grouped mathematical base.
    Underset,
    /// One-group vector decoration.
    Vector,
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
/// `\\frac{...}{...}`, `\\binom{...}{...}`, `\\sqrt{...}`,
/// grouped mathematical alphabets, common one-group accents, `\\mathrm{...}`,
/// `\\operatorname{...}`, `\\text{...}`, grouped vector, overline, and
/// underline decorations, escaped TeX special characters, common named
/// mathematical operators and symbols, aligned separators, and
/// `\\begin{matrix}...\\end{matrix}`.
/// Math-only alignment, script, and row-break markers remain literal inside
/// grouped text.
/// Other commands remain present in the token stream and are reported through
/// [`AnalyzedFormula::unsupported`].
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
    let group_index = build_group_index(source);
    let mut state = ScanState {
        group_depth: 0,
        index: 0,
        literal_start: 0,
        matrix_depth: 0,
        pending_text_group: false,
        text_group_depth: None,
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
            &group_index,
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

fn environment_command_matches(
    source: &str,
    start: usize,
    spelling: &str,
) -> bool {
    source
        .get(start..)
        .is_some_and(|tail| tail.starts_with(spelling))
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
    groups: &GroupIndex,
    source_len: usize,
    open: usize,
) -> Result<usize, MathSyntaxError> {
    let Ok(index) = groups
        .pairs
        .binary_search_by_key(&open, |(start, _)| *start)
    else {
        return Err(error(source_len, MathSyntaxErrorKind::UnclosedGroup));
    };
    let Some((_, end)) = groups.pairs.get(index) else {
        return Err(error(source_len, MathSyntaxErrorKind::UnclosedGroup));
    };
    Ok(*end)
}

fn push_literal(tokens: &mut Vec<MathToken>, start: usize, end: usize) {
    if start < end {
        tokens.push(token(start, end, MathTokenKind::Literal));
    }
}

fn build_group_index(source: &str) -> GroupIndex {
    let mut pairs = Vec::new();
    let mut slash_run = 0usize;
    let mut stack = Vec::new();
    for (index, byte) in source.as_bytes().iter().copied().enumerate() {
        if byte == b'\\' {
            slash_run = slash_run.saturating_add(1);
            continue;
        }
        let escaped = slash_run & 1 == 1;
        slash_run = 0;
        if escaped {
            continue;
        }
        match byte {
            b'{' => stack.push(index),
            b'}' => {
                if let Some(open) = stack.pop() {
                    pairs.push((open, index.saturating_add(1)));
                }
            },
            _ => {},
        }
    }
    pairs.sort_unstable_by_key(|(open, _)| *open);
    GroupIndex { pairs }
}

fn scan_ascii_control_word_end(
    source: &str,
    after_slash: usize,
) -> Option<usize> {
    let tail = source.get(after_slash..)?;
    let mut end = after_slash;
    for (offset, character) in tail.char_indices() {
        if !character.is_ascii_alphabetic() {
            break;
        }
        end = after_slash
            .saturating_add(offset)
            .saturating_add(character.len_utf8());
    }
    (end != after_slash).then_some(end)
}

fn scan_named_command(
    source: &str,
    start: usize,
    end: usize,
    spellings: &[&str],
    supported: SupportedCommand,
) -> Option<ScannedCommand> {
    let spelling = source.get(start..end)?;
    spellings
        .binary_search(&spelling)
        .is_ok()
        .then_some(ScannedCommand {
            end,
            kind: ScannedCommandKind::Supported(supported),
        })
}

fn scan_structured_control_word(
    source: &str,
    start: usize,
    end: usize,
) -> Option<ScannedCommand> {
    let spelling = source.get(start..end)?;
    let index = STRUCTURED_CONTROL_WORD_COMMANDS
        .binary_search_by(|(candidate, _)| candidate.cmp(&spelling))
        .ok()?;
    let (_, supported) = STRUCTURED_CONTROL_WORD_COMMANDS.get(index)?;
    Some(ScannedCommand {
        end,
        kind: ScannedCommandKind::Supported(*supported),
    })
}

fn scan_structured_environment_command(
    source: &str,
    start: usize,
) -> Option<ScannedCommand> {
    STRUCTURED_ENVIRONMENT_COMMANDS
        .iter()
        .find_map(|(spelling, supported)| {
            environment_command_matches(source, start, spelling)
                .then(|| ScannedCommand {
                    end: start.saturating_add(spelling.len()),
                    kind: ScannedCommandKind::Supported(*supported),
                })
        })
}

fn scan_command(source: &str, start: usize) -> ScannedCommand {
    let after_slash = start.saturating_add(1);
    if source.as_bytes().get(after_slash) == Some(&b'\\') {
        return ScannedCommand {
            end: after_slash.saturating_add(1),
            kind: ScannedCommandKind::RowBreak,
        };
    }
    if source.as_bytes().get(after_slash).is_some_and(|byte| {
        matches!(byte, b'{' | b'}' | b'%' | b'$' | b'#' | b'&' | b'_')
    }) {
        return ScannedCommand {
            end: after_slash.saturating_add(1),
            kind: ScannedCommandKind::Supported(
                SupportedCommand::EscapedSpecial,
            ),
        };
    }
    if let Some(end) = scan_ascii_control_word_end(source, after_slash) {
        if let Some(command) = scan_named_command(
            source,
            start,
            end,
            NAMED_OPERATOR_COMMANDS,
            SupportedCommand::NamedOperator,
        ) {
            return command;
        }
        if let Some(command) = scan_named_command(
            source,
            start,
            end,
            NAMED_SYMBOL_COMMANDS,
            SupportedCommand::NamedSymbol,
        ) {
            return command;
        }
        if let Some(command) = scan_structured_control_word(
            source, start, end,
        ) {
            return command;
        }
        if let Some(command) =
            scan_structured_environment_command(source, start)
        {
            return command;
        }
        return ScannedCommand {
            end,
            kind: ScannedCommandKind::Unsupported,
        };
    }
    if let Some(command) = scan_structured_environment_command(source, start) {
        return command;
    }
    let tail = source.get(after_slash..).unwrap_or_default();
    let end = tail.chars().next().map_or(after_slash, |character| {
        after_slash.saturating_add(character.len_utf8())
    });
    ScannedCommand {
        end,
        kind: ScannedCommandKind::Unsupported,
    }
}

fn scan_structural(
    source: &str,
    mode: FormulaMode,
    character: char,
    groups: &GroupIndex,
    state: &mut ScanState,
    tokens: &mut Vec<MathToken>,
    unsupported: &mut Vec<UnsupportedConstruct>,
) -> Result<(), MathSyntaxError> {
    let width = character.len_utf8();
    match character {
        '&' | '^' | '_' if state.text_group_depth.is_some() => {
            scan_marker(state, tokens, width, MathTokenKind::Literal);
            Ok(())
        },
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
            if state.pending_text_group {
                if state.text_group_depth.is_none() {
                    state.text_group_depth = Some(state.group_depth);
                }
                state.pending_text_group = false;
            }
            scan_marker(state, tokens, width, MathTokenKind::GroupOpen);
            Ok(())
        },
        '}' => scan_group_close(state, tokens, width),
        '\\' => scan_slash(source, groups, mode, state, tokens, unsupported),
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
    let closes_text = state.text_group_depth == Some(state.group_depth);
    state.group_depth = state.group_depth.saturating_sub(1);
    if closes_text {
        state.text_group_depth = None;
    }
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
    groups: &GroupIndex,
    mode: FormulaMode,
    state: &mut ScanState,
    tokens: &mut Vec<MathToken>,
    unsupported: &mut Vec<UnsupportedConstruct>,
) -> Result<(), MathSyntaxError> {
    let command = scan_command(source, state.index);
    match command.kind {
        ScannedCommandKind::RowBreak => {
            if state.text_group_depth.is_some() {
                tokens.push(token(
                    state.index,
                    command.end,
                    MathTokenKind::Literal,
                ));
            } else {
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
            }
        },
        ScannedCommandKind::Supported(supported) => {
            scan_supported_command(
                source, groups, state, tokens, command, supported,
            )?;
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
    groups: &GroupIndex,
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
    validate_command_groups(source, groups, command.end, supported)?;
    if supported == SupportedCommand::Text {
        state.pending_text_group = true;
    }
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
    groups: &GroupIndex,
    command_end: usize,
    command: SupportedCommand,
) -> Result<(), MathSyntaxError> {
    let required = match command {
        SupportedCommand::BeginMatrix
        | SupportedCommand::EndMatrix
        | SupportedCommand::EscapedSpecial
        | SupportedCommand::NamedOperator
        | SupportedCommand::NamedSymbol => 0usize,
        SupportedCommand::Binomial
        | SupportedCommand::Fraction
        | SupportedCommand::Overset
        | SupportedCommand::Underset => 2usize,
        SupportedCommand::Bar
        | SupportedCommand::BlackboardBold
        | SupportedCommand::Bold
        | SupportedCommand::Calligraphic
        | SupportedCommand::Dot
        | SupportedCommand::DoubleDot
        | SupportedCommand::Fraktur
        | SupportedCommand::Hat
        | SupportedCommand::Italic
        | SupportedCommand::OperatorName
        | SupportedCommand::Overline
        | SupportedCommand::Roman
        | SupportedCommand::SansSerif
        | SupportedCommand::SquareRoot
        | SupportedCommand::Text
        | SupportedCommand::Tilde
        | SupportedCommand::Typewriter
        | SupportedCommand::Underline
        | SupportedCommand::Vector => 1usize,
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
        cursor = matching_group_end(groups, source.len(), cursor)?;
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
