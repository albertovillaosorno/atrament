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
    ("\\acute", SupportedCommand::Acute),
    ("\\bar", SupportedCommand::Bar),
    ("\\binom", SupportedCommand::Binomial),
    ("\\boxed", SupportedCommand::Boxed),
    ("\\breve", SupportedCommand::Breve),
    ("\\check", SupportedCommand::Check),
    ("\\dbinom", SupportedCommand::DisplayBinomial),
    ("\\ddot", SupportedCommand::DoubleDot),
    ("\\dfrac", SupportedCommand::DisplayFraction),
    ("\\dot", SupportedCommand::Dot),
    ("\\frac", SupportedCommand::Fraction),
    ("\\grave", SupportedCommand::Grave),
    ("\\hat", SupportedCommand::Hat),
    ("\\mathbb", SupportedCommand::BlackboardBold),
    ("\\mathbf", SupportedCommand::Bold),
    ("\\mathcal", SupportedCommand::Calligraphic),
    ("\\mathfrak", SupportedCommand::Fraktur),
    ("\\mathit", SupportedCommand::Italic),
    ("\\mathring", SupportedCommand::MathRing),
    ("\\mathrm", SupportedCommand::Roman),
    ("\\mathsf", SupportedCommand::SansSerif),
    ("\\mathtt", SupportedCommand::Typewriter),
    ("\\operatorname", SupportedCommand::OperatorName),
    ("\\overbrace", SupportedCommand::Overbrace),
    ("\\overleftarrow", SupportedCommand::OverLeftArrow),
    ("\\overleftrightarrow", SupportedCommand::OverLeftRightArrow),
    ("\\overline", SupportedCommand::Overline),
    ("\\overrightarrow", SupportedCommand::OverRightArrow),
    ("\\overset", SupportedCommand::Overset),
    ("\\sqrt", SupportedCommand::SquareRoot),
    ("\\stackrel", SupportedCommand::StackRelation),
    ("\\substack", SupportedCommand::Substack),
    ("\\tbinom", SupportedCommand::TextBinomial),
    ("\\text", SupportedCommand::Text),
    ("\\tfrac", SupportedCommand::TextFraction),
    ("\\tilde", SupportedCommand::Tilde),
    ("\\underbrace", SupportedCommand::Underbrace),
    ("\\underline", SupportedCommand::Underline),
    ("\\underset", SupportedCommand::Underset),
    ("\\vec", SupportedCommand::Vector),
    ("\\widehat", SupportedCommand::WideHat),
    ("\\widetilde", SupportedCommand::WideTilde),
];

const STRUCTURED_ENVIRONMENTS: &[StructuredEnvironmentDefinition] = &[
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginBracedMatrix,
        begin_spelling: "\\begin{Bmatrix}",
        end_command: SupportedCommand::EndBracedMatrix,
        end_spelling: "\\end{Bmatrix}",
        extra_end_error: MathSyntaxErrorKind::ExtraBracedMatrixEnd,
        missing_end_error: MathSyntaxErrorKind::MissingBracedMatrixEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginDoubleVerticalMatrix,
        begin_spelling: "\\begin{Vmatrix}",
        end_command: SupportedCommand::EndDoubleVerticalMatrix,
        end_spelling: "\\end{Vmatrix}",
        extra_end_error: MathSyntaxErrorKind::ExtraDoubleVerticalMatrixEnd,
        missing_end_error: MathSyntaxErrorKind::MissingDoubleVerticalMatrixEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginAligned,
        begin_spelling: "\\begin{aligned}",
        end_command: SupportedCommand::EndAligned,
        end_spelling: "\\end{aligned}",
        extra_end_error: MathSyntaxErrorKind::ExtraAlignedEnd,
        missing_end_error: MathSyntaxErrorKind::MissingAlignedEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginBracketedMatrix,
        begin_spelling: "\\begin{bmatrix}",
        end_command: SupportedCommand::EndBracketedMatrix,
        end_spelling: "\\end{bmatrix}",
        extra_end_error: MathSyntaxErrorKind::ExtraBracketedMatrixEnd,
        missing_end_error: MathSyntaxErrorKind::MissingBracketedMatrixEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginCases,
        begin_spelling: "\\begin{cases}",
        end_command: SupportedCommand::EndCases,
        end_spelling: "\\end{cases}",
        extra_end_error: MathSyntaxErrorKind::ExtraCasesEnd,
        missing_end_error: MathSyntaxErrorKind::MissingCasesEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: false,
        begin_command: SupportedCommand::BeginGathered,
        begin_spelling: "\\begin{gathered}",
        end_command: SupportedCommand::EndGathered,
        end_spelling: "\\end{gathered}",
        extra_end_error: MathSyntaxErrorKind::ExtraGatheredEnd,
        missing_end_error: MathSyntaxErrorKind::MissingGatheredEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginMatrix,
        begin_spelling: "\\begin{matrix}",
        end_command: SupportedCommand::EndMatrix,
        end_spelling: "\\end{matrix}",
        extra_end_error: MathSyntaxErrorKind::ExtraMatrixEnd,
        missing_end_error: MathSyntaxErrorKind::MissingMatrixEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginParenthesizedMatrix,
        begin_spelling: "\\begin{pmatrix}",
        end_command: SupportedCommand::EndParenthesizedMatrix,
        end_spelling: "\\end{pmatrix}",
        extra_end_error: MathSyntaxErrorKind::ExtraParenthesizedMatrixEnd,
        missing_end_error: MathSyntaxErrorKind::MissingParenthesizedMatrixEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginSmallMatrix,
        begin_spelling: "\\begin{smallmatrix}",
        end_command: SupportedCommand::EndSmallMatrix,
        end_spelling: "\\end{smallmatrix}",
        extra_end_error: MathSyntaxErrorKind::ExtraSmallMatrixEnd,
        missing_end_error: MathSyntaxErrorKind::MissingSmallMatrixEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginSplit,
        begin_spelling: "\\begin{split}",
        end_command: SupportedCommand::EndSplit,
        end_spelling: "\\end{split}",
        extra_end_error: MathSyntaxErrorKind::ExtraSplitEnd,
        missing_end_error: MathSyntaxErrorKind::MissingSplitEnd,
    },
    StructuredEnvironmentDefinition {
        allows_alignment: true,
        begin_command: SupportedCommand::BeginVerticalMatrix,
        begin_spelling: "\\begin{vmatrix}",
        end_command: SupportedCommand::EndVerticalMatrix,
        end_spelling: "\\end{vmatrix}",
        extra_end_error: MathSyntaxErrorKind::ExtraVerticalMatrixEnd,
        missing_end_error: MathSyntaxErrorKind::MissingVerticalMatrixEnd,
    },
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
    /// Alignment marker appears outside aligned or environment content.
    AlignmentOutsideStructure,
    /// An environment closes inside a group opened after that environment.
    EnvironmentClosesInsideGroup,
    /// A group closes while an environment opened in it remains active.
    EnvironmentCrossesGroupClose,
    /// Aligned environment closes without a matching aligned start.
    ExtraAlignedEnd,
    /// Brace-delimited matrix closes without its matching start.
    ExtraBracedMatrixEnd,
    /// Bracketed-matrix environment closes without its matching start.
    ExtraBracketedMatrixEnd,
    /// Cases environment closes without a matching cases start.
    ExtraCasesEnd,
    /// Double-vertical-bar matrix closes without its matching start.
    ExtraDoubleVerticalMatrixEnd,
    /// Gathered environment closes without a matching gathered start.
    ExtraGatheredEnd,
    /// A closing group appears without a matching open group.
    ExtraGroupClose,
    /// Matrix environment closes without a matching matrix start.
    ExtraMatrixEnd,
    /// Parenthesized-matrix environment closes without its matching start.
    ExtraParenthesizedMatrixEnd,
    /// Compact small-matrix environment closes without its matching start.
    ExtraSmallMatrixEnd,
    /// Split environment closes without a matching split start.
    ExtraSplitEnd,
    /// Vertical-bar matrix closes without its matching start.
    ExtraVerticalMatrixEnd,
    /// An environment closes while a different environment is still innermost.
    MismatchedEnvironmentEnd,
    /// Aligned environment remains open at end of source.
    MissingAlignedEnd,
    /// Brace-delimited matrix remains open at end of source.
    MissingBracedMatrixEnd,
    /// Bracketed-matrix environment remains open at end of source.
    MissingBracketedMatrixEnd,
    /// Cases environment remains open at end of source.
    MissingCasesEnd,
    /// Double-vertical-bar matrix remains open at end of source.
    MissingDoubleVerticalMatrixEnd,
    /// Gathered environment remains open at end of source.
    MissingGatheredEnd,
    /// Matrix environment remains open at end of source.
    MissingMatrixEnd,
    /// Parenthesized-matrix environment remains open at end of source.
    MissingParenthesizedMatrixEnd,
    /// A supported command is missing one or more required braced arguments.
    MissingRequiredGroup,
    /// An indexed square root is missing its top-level closing bracket.
    MissingRootIndexEnd,
    /// Compact small-matrix environment remains open at end of source.
    MissingSmallMatrixEnd,
    /// Split environment remains open at end of source.
    MissingSplitEnd,
    /// Vertical-bar matrix remains open at end of source.
    MissingVerticalMatrixEnd,
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
    /// Column alignment point in aligned or admitted environment content.
    AlignmentPoint,
    /// Supported TeX-compatible control sequence.
    Command(SupportedCommand),
    /// Closing brace of an ordinary source group.
    GroupClose,
    /// Opening brace of an ordinary source group.
    GroupOpen,
    /// Ordinary literal source that is not structural syntax.
    Literal,
    /// Closing bracket of an indexed square-root argument.
    RootIndexClose,
    /// Opening bracket of an indexed square-root argument.
    RootIndexOpen,
    /// Explicit mathematical row break.
    RowBreak,
    /// Subscript marker.
    Subscript,
    /// Superscript marker.
    Superscript,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StructuredEnvironmentDefinition {
    allows_alignment: bool,
    begin_command: SupportedCommand,
    begin_spelling: &'static str,
    end_command: SupportedCommand,
    end_spelling: &'static str,
    extra_end_error: MathSyntaxErrorKind,
    missing_end_error: MathSyntaxErrorKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StructuredEnvironmentScope {
    definition: StructuredEnvironmentDefinition,
    group_depth: usize,
}

#[derive(Debug)]
struct GroupIndex {
    pairs: Vec<(usize, usize)>,
}

struct ScanState {
    environment_stack: Vec<StructuredEnvironmentScope>,
    group_depth: usize,
    index: usize,
    literal_start: usize,
    pending_root_index_open: Option<usize>,
    pending_substack_group: bool,
    pending_text_group: bool,
    root_index_close_offsets: Vec<usize>,
    substack_group_depths: Vec<usize>,
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
    /// One-group acute accent.
    Acute,
    /// One-group bar accent.
    Bar,
    /// Start of an explicit aligned environment.
    BeginAligned,
    /// Start of an explicit brace-delimited matrix environment.
    BeginBracedMatrix,
    /// Start of an explicit bracketed-matrix environment.
    BeginBracketedMatrix,
    /// Start of an explicit cases environment.
    BeginCases,
    /// Start of an explicit double-vertical-bar matrix environment.
    BeginDoubleVerticalMatrix,
    /// Start of an explicit gathered environment.
    BeginGathered,
    /// Start of an explicit matrix environment.
    BeginMatrix,
    /// Start of an explicit parenthesized-matrix environment.
    BeginParenthesizedMatrix,
    /// Start of an explicit compact small-matrix environment.
    BeginSmallMatrix,
    /// Start of an explicit split environment.
    BeginSplit,
    /// Start of an explicit vertical-bar matrix environment.
    BeginVerticalMatrix,
    /// Two-group binomial coefficient command.
    Binomial,
    /// One-group blackboard-bold mathematical alphabet.
    BlackboardBold,
    /// One-group bold mathematical alphabet.
    Bold,
    /// One-group boxed mathematical expression.
    Boxed,
    /// One-group breve accent.
    Breve,
    /// One-group calligraphic mathematical alphabet.
    Calligraphic,
    /// One-group check accent.
    Check,
    /// Two-group display-style binomial coefficient command.
    DisplayBinomial,
    /// Two-group display-style fraction command.
    DisplayFraction,
    /// One-group dot accent.
    Dot,
    /// One-group double-dot accent.
    DoubleDot,
    /// End of an explicit aligned environment.
    EndAligned,
    /// End of an explicit brace-delimited matrix environment.
    EndBracedMatrix,
    /// End of an explicit bracketed-matrix environment.
    EndBracketedMatrix,
    /// End of an explicit cases environment.
    EndCases,
    /// End of an explicit double-vertical-bar matrix environment.
    EndDoubleVerticalMatrix,
    /// End of an explicit gathered environment.
    EndGathered,
    /// End of an explicit matrix environment.
    EndMatrix,
    /// End of an explicit parenthesized-matrix environment.
    EndParenthesizedMatrix,
    /// End of an explicit compact small-matrix environment.
    EndSmallMatrix,
    /// End of an explicit split environment.
    EndSplit,
    /// End of an explicit vertical-bar matrix environment.
    EndVerticalMatrix,
    /// Escaped TeX special character preserved as literal source.
    EscapedSpecial,
    /// Two-group fraction command.
    Fraction,
    /// One-group Fraktur mathematical alphabet.
    Fraktur,
    /// One-group grave accent.
    Grave,
    /// One-group hat accent.
    Hat,
    /// One-group italic mathematical alphabet.
    Italic,
    /// One-group mathematical ring accent.
    MathRing,
    /// Admitted no-argument named mathematical operator.
    NamedOperator,
    /// Admitted no-argument named mathematical symbol.
    NamedSymbol,
    /// One grouped custom mathematical operator name.
    OperatorName,
    /// One-group left-pointing over-arrow decoration.
    OverLeftArrow,
    /// One-group bidirectional over-arrow decoration.
    OverLeftRightArrow,
    /// One-group right-pointing over-arrow decoration.
    OverRightArrow,
    /// One-group overbrace decoration.
    Overbrace,
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
    /// Two-group stacked relation annotation.
    StackRelation,
    /// One grouped multi-row substack expression.
    Substack,
    /// One grouped text fragment preserved exactly inside mathematics.
    Text,
    /// Two-group text-style binomial coefficient command.
    TextBinomial,
    /// Two-group text-style fraction command.
    TextFraction,
    /// One-group tilde accent.
    Tilde,
    /// One-group typewriter mathematical alphabet.
    Typewriter,
    /// One-group underbrace decoration.
    Underbrace,
    /// One-group underline decoration.
    Underline,
    /// One annotation group placed below one grouped mathematical base.
    Underset,
    /// One-group vector decoration.
    Vector,
    /// One-group wide hat accent.
    WideHat,
    /// One-group wide tilde accent.
    WideTilde,
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
/// `\\frac{...}{...}`, `\\binom{...}{...}`, `\\sqrt{...}`, and
/// indexed `\\sqrt[...]{...}`,
/// grouped mathematical alphabets, common one-group accents, `\\mathrm{...}`,
/// `\\operatorname{...}`, `\\text{...}`, grouped substacks, vector,
/// overline, and underline decorations, escaped TeX special characters, common
/// named mathematical operators and symbols, aligned separators, and ordered
/// aligned, brace-delimited-matrix, bracketed-matrix, cases,
/// double-vertical-bar-matrix, gathered, matrix, parenthesized-matrix,
/// small-matrix, split, and vertical-bar-matrix environments.
/// Math-only alignment, script, and row-break markers remain literal inside
/// grouped text.
/// Other commands remain present in the token stream and are reported through
/// [`AnalyzedFormula::unsupported`].
///
/// # Errors
///
/// Returns the first typed structural failure for unmatched groups or admitted
/// environment boundaries, missing required braced arguments, or alignment used
/// in a context that does not admit it.
pub fn analyze(
    source: &str,
    mode: FormulaMode,
) -> Result<AnalyzedFormula, MathSyntaxError> {
    let group_index = build_group_index(source);
    let mut state = ScanState {
        environment_stack: Vec::new(),
        group_depth: 0,
        index: 0,
        literal_start: 0,
        pending_root_index_open: None,
        pending_substack_group: false,
        pending_text_group: false,
        root_index_close_offsets: Vec::new(),
        substack_group_depths: Vec::new(),
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
        let is_root_index_marker =
            state.pending_root_index_open == Some(state.index)
                || state.root_index_close_offsets.last().copied()
                    == Some(state.index);
        if !is_structural(character) && !is_root_index_marker {
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
    validate_final_state(source.len(), &state)?;
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
    STRUCTURED_ENVIRONMENTS.iter().find_map(|environment| {
        let (spelling, supported) = if environment_command_matches(
            source,
            start,
            environment.begin_spelling,
        ) {
            (environment.begin_spelling, environment.begin_command)
        } else if environment_command_matches(
            source,
            start,
            environment.end_spelling,
        ) {
            (environment.end_spelling, environment.end_command)
        } else {
            return None;
        };
        Some(ScannedCommand {
            end: start.saturating_add(spelling.len()),
            kind: ScannedCommandKind::Supported(supported),
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
        '[' if state.pending_root_index_open == Some(state.index) => {
            state.pending_root_index_open = None;
            scan_marker(state, tokens, width, MathTokenKind::RootIndexOpen);
            Ok(())
        },
        ']' if state.root_index_close_offsets.last().copied()
            == Some(state.index) =>
        {
            let _: Option<usize> = state.root_index_close_offsets.pop();
            scan_marker(state, tokens, width, MathTokenKind::RootIndexClose);
            Ok(())
        },
        '{' => {
            state.group_depth = state.group_depth.saturating_add(1);
            if state.pending_substack_group {
                state.substack_group_depths.push(state.group_depth);
                state.pending_substack_group = false;
            }
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
    let environment = state.environment_stack.last().copied();
    let substack_depth = state.substack_group_depths.last().copied();
    let substack_owns_scope = substack_depth.is_some_and(|depth| {
        environment.is_none_or(|scope| scope.group_depth < depth)
    });
    let environment_allows_columns = environment
        .is_some_and(|scope| scope.definition.allows_alignment);
    let mode_allows_columns =
        environment.is_none() && mode == FormulaMode::Aligned;
    if substack_owns_scope
        || !(environment_allows_columns || mode_allows_columns)
    {
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
    if state
        .environment_stack
        .last()
        .is_some_and(|scope| scope.group_depth == state.group_depth)
    {
        return Err(error(
            state.index,
            MathSyntaxErrorKind::EnvironmentCrossesGroupClose,
        ));
    }
    let closes_substack = state
        .substack_group_depths
        .last()
        .copied()
        == Some(state.group_depth);
    let closes_text = state.text_group_depth == Some(state.group_depth);
    state.group_depth = state.group_depth.saturating_sub(1);
    if closes_substack {
        state.substack_group_depths.truncate(
            state.substack_group_depths.len().saturating_sub(1),
        );
    }
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
                if mode != FormulaMode::Aligned
                    && state.environment_stack.is_empty()
                    && state.substack_group_depths.is_empty()
                {
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
    if let Some(environment) = ending_environment(supported) {
        close_environment(state, environment)?;
    }
    tokens.push(token(
        state.index,
        command.end,
        MathTokenKind::Command(supported),
    ));
    if supported == SupportedCommand::SquareRoot {
        if let Some((open, close)) =
            root_index_bounds(source, groups, command.end)?
        {
            validate_required_groups(
                source,
                groups,
                close.saturating_add(1),
                1,
            )?;
            state.pending_root_index_open = Some(open);
            state.root_index_close_offsets.push(close);
        } else {
            validate_required_groups(source, groups, command.end, 1)?;
        }
    } else {
        validate_command_groups(source, groups, command.end, supported)?;
    }
    if supported == SupportedCommand::Substack {
        state.pending_substack_group = true;
    }
    if supported == SupportedCommand::Text {
        state.pending_text_group = true;
    }
    if let Some(environment) = beginning_environment(supported) {
        state.environment_stack.push(StructuredEnvironmentScope {
            definition: environment,
            group_depth: state.group_depth,
        });
    }
    Ok(())
}

fn beginning_environment(
    command: SupportedCommand,
) -> Option<StructuredEnvironmentDefinition> {
    STRUCTURED_ENVIRONMENTS
        .iter()
        .copied()
        .find(|environment| environment.begin_command == command)
}

fn ending_environment(
    command: SupportedCommand,
) -> Option<StructuredEnvironmentDefinition> {
    STRUCTURED_ENVIRONMENTS
        .iter()
        .copied()
        .find(|environment| environment.end_command == command)
}

fn close_environment(
    state: &mut ScanState,
    expected: StructuredEnvironmentDefinition,
) -> Result<(), MathSyntaxError> {
    let Some(actual) = state.environment_stack.last().copied() else {
        return Err(error(state.index, expected.extra_end_error));
    };
    if actual.definition != expected {
        return Err(error(
            state.index,
            MathSyntaxErrorKind::MismatchedEnvironmentEnd,
        ));
    }
    if actual.group_depth != state.group_depth {
        return Err(error(
            state.index,
            MathSyntaxErrorKind::EnvironmentClosesInsideGroup,
        ));
    }
    let _: Option<StructuredEnvironmentScope> = state.environment_stack.pop();
    Ok(())
}

fn root_index_bounds(
    source: &str,
    groups: &GroupIndex,
    command_end: usize,
) -> Result<Option<(usize, usize)>, MathSyntaxError> {
    let open = skip_ascii_whitespace(source, command_end);
    if source.as_bytes().get(open) != Some(&b'[') {
        return Ok(None);
    }
    let mut cursor = open.saturating_add(1);
    let mut slash_run = 0usize;
    while let Some(byte) = source.as_bytes().get(cursor).copied() {
        if byte == b'\\' {
            slash_run = slash_run.saturating_add(1);
            cursor = cursor.saturating_add(1);
            continue;
        }
        let escaped = slash_run & 1 == 1;
        slash_run = 0;
        if !escaped && byte == b'{' {
            let Ok(end) =
                matching_group_end(groups, source.len(), cursor)
            else {
                return Err(error(
                    source.len(),
                    MathSyntaxErrorKind::MissingRootIndexEnd,
                ));
            };
            cursor = end;
            continue;
        }
        if !escaped && byte == b']' {
            return Ok(Some((open, cursor)));
        }
        cursor = cursor.saturating_add(1);
    }
    Err(error(source.len(), MathSyntaxErrorKind::MissingRootIndexEnd))
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
        SupportedCommand::BeginAligned
        | SupportedCommand::BeginBracedMatrix
        | SupportedCommand::BeginBracketedMatrix
        | SupportedCommand::BeginCases
        | SupportedCommand::BeginDoubleVerticalMatrix
        | SupportedCommand::BeginGathered
        | SupportedCommand::BeginMatrix
        | SupportedCommand::BeginParenthesizedMatrix
        | SupportedCommand::BeginSmallMatrix
        | SupportedCommand::BeginSplit
        | SupportedCommand::BeginVerticalMatrix
        | SupportedCommand::EndAligned
        | SupportedCommand::EndBracedMatrix
        | SupportedCommand::EndBracketedMatrix
        | SupportedCommand::EndCases
        | SupportedCommand::EndDoubleVerticalMatrix
        | SupportedCommand::EndGathered
        | SupportedCommand::EndMatrix
        | SupportedCommand::EndParenthesizedMatrix
        | SupportedCommand::EndSmallMatrix
        | SupportedCommand::EndSplit
        | SupportedCommand::EndVerticalMatrix
        | SupportedCommand::EscapedSpecial
        | SupportedCommand::NamedOperator
        | SupportedCommand::SquareRoot
        | SupportedCommand::NamedSymbol => 0usize,
        SupportedCommand::Binomial
        | SupportedCommand::DisplayBinomial
        | SupportedCommand::DisplayFraction
        | SupportedCommand::Fraction
        | SupportedCommand::Overset
        | SupportedCommand::StackRelation
        | SupportedCommand::TextBinomial
        | SupportedCommand::TextFraction
        | SupportedCommand::Underset => 2usize,
        SupportedCommand::Acute
        | SupportedCommand::Bar
        | SupportedCommand::BlackboardBold
        | SupportedCommand::Bold
        | SupportedCommand::Boxed
        | SupportedCommand::Breve
        | SupportedCommand::Calligraphic
        | SupportedCommand::Check
        | SupportedCommand::Dot
        | SupportedCommand::DoubleDot
        | SupportedCommand::Fraktur
        | SupportedCommand::Grave
        | SupportedCommand::Hat
        | SupportedCommand::Italic
        | SupportedCommand::MathRing
        | SupportedCommand::OperatorName
        | SupportedCommand::Overbrace
        | SupportedCommand::OverLeftArrow
        | SupportedCommand::OverLeftRightArrow
        | SupportedCommand::Overline
        | SupportedCommand::OverRightArrow
        | SupportedCommand::Roman
        | SupportedCommand::SansSerif
        | SupportedCommand::Substack
        | SupportedCommand::Text
        | SupportedCommand::Tilde
        | SupportedCommand::Typewriter
        | SupportedCommand::Underbrace
        | SupportedCommand::Underline
        | SupportedCommand::Vector
        | SupportedCommand::WideHat
        | SupportedCommand::WideTilde => 1usize,
    };
    validate_required_groups(source, groups, command_end, required)
}

fn validate_required_groups(
    source: &str,
    groups: &GroupIndex,
    mut cursor: usize,
    required: usize,
) -> Result<(), MathSyntaxError> {
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

fn validate_final_state(
    source_len: usize,
    state: &ScanState,
) -> Result<(), MathSyntaxError> {
    if state.group_depth != 0 {
        return Err(error(source_len, MathSyntaxErrorKind::UnclosedGroup));
    }
    if state.pending_root_index_open.is_some()
        || !state.root_index_close_offsets.is_empty()
    {
        return Err(error(
            source_len,
            MathSyntaxErrorKind::MissingRootIndexEnd,
        ));
    }
    let Some(environment) = state.environment_stack.last().copied() else {
        return Ok(());
    };
    Err(error(
        source_len,
        environment.definition.missing_end_error,
    ))
}
