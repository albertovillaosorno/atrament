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
//   - Transport-independent semantic notebook values and opaque identities.
// - Must-Not:
//   - Choose serialized ID syntax, storage paths, DOM identity, or layout
//     pixels.
// - Allows:
//   - Inputs: Typed semantic content, candidate-local identities, and accepted
//     identities allocated by one active application authority.
//   - Outputs: Candidate notebooks and accepted immutable revision values.
//   - Side effects: Process-memory identity allocation only.
// - Split-When:
//   - One semantic family becomes an independently versioned domain authority.
// - Merge-When:
//   - Notebook semantics stop requiring a distinct authoritative model.
// - Summary:
//   - Defines typed semantic notebook authority independent from presentation.
// - Description:
//   - Keeps accepted identity and document meaning separate from wire encoding.
// - Usage:
//   - Build candidate values, then allocate accepted identities at commit.
// - Defaults:
//   - Identity sequences are active-session local and never recycled.
//

//! Typed semantic notebook values for candidate and accepted application state.

use std::cell::Cell;
use std::num::NonZeroU64;

pub use atrament_mathematics_source::{
    FormulaMode, MathSyntaxError, MathSyntaxErrorKind,
};
pub use atrament_physical_page_profile::{
    PageProfile as PhysicalPageProfile,
    PageProfileError as PhysicalPageProfileError,
};

/// Opaque semantic identity admitted only after an application commit.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AcceptedIdentity(NonZeroU64);

/// One immutable accepted semantic notebook revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedRevision {
    /// Commit-owned revision identity.
    pub id: RevisionIdentity,
    /// Semantic notebook state owned by this revision.
    pub notebook: Notebook<AcceptedIdentity>,
}

/// One session-owned asset semantic reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Asset<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Declared media type without storage-path authority.
    pub media_type: String,
}

/// One typed semantic block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block<Identity> {
    /// Typed block content.
    pub content: BlockContent<Identity>,
    /// Explicitly admitted extension data retained with this block.
    pub extensions: Vec<ExtensionData>,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Optional provenance identity owned by the same notebook.
    pub provenance: Option<Identity>,
    /// Optional style identity owned by the same notebook.
    pub style: Option<Identity>,
}

/// Semantic content families admitted by the first-release notebook model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlockContent<Identity> {
    /// Bounded emphasized region containing semantic child blocks.
    Callout(Vec<Block<Identity>>),
    /// Semantic date content retained as editable spans.
    Date(Vec<InlineSpan<Identity>>),
    /// Figure backed by an admitted asset and semantic caption.
    Figure(Figure<Identity>),
    /// Explicit freeform region containing semantic child blocks.
    Freeform(Vec<Block<Identity>>),
    /// Heading content retained as editable spans.
    Heading(Vec<InlineSpan<Identity>>),
    /// Ordered or unordered semantic list.
    List(List<Identity>),
    /// Structured mathematical source.
    Mathematics(Formula<Identity>),
    /// Ordinary prose paragraph.
    Paragraph(Vec<InlineSpan<Identity>>),
    /// Semantic ruler or divider.
    Rule,
    /// Structured semantic table.
    Table(Table<Identity>),
    /// Unsupported or ambiguous semantic content retained without guessing.
    Unresolved(UnresolvedBlock),
}

/// Opaque identity used only inside one unaccepted candidate notebook.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CandidateIdentity(NonZeroU64);

/// One semantic constraint attached to a notebook owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Constraint<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Broad semantic constraint family; detailed geometry belongs elsewhere.
    pub kind: ConstraintKind,
    /// Semantic owner constrained by this value.
    pub target: Identity,
}

/// Broad constraint families retained by semantic authority.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConstraintKind {
    /// Constraint governing semantic layout intent.
    Layout,
    /// Constraint governing an output capability choice.
    Output,
    /// Constraint governing paper or writable-region intent.
    Paper,
    /// Constraint governing placement of a semantic owner.
    Placement,
    /// Constraint governing style intent.
    Style,
}

/// Explicitly admitted opaque extension data preserved by the semantic model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionData {
    /// Versioned extension namespace identity.
    pub namespace: String,
    /// Opaque extension payload retained exactly by the domain value.
    pub payload: Vec<u8>,
}

/// One semantic figure reference and caption.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Figure<Identity> {
    /// Optional admitted asset identity referenced by the figure.
    pub asset: Option<Identity>,
    /// Editable semantic caption spans.
    pub caption: Vec<InlineSpan<Identity>>,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
}

/// One ordered flow of semantic blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Flow<Identity> {
    /// Blocks in semantic reading order.
    pub blocks: Vec<Block<Identity>>,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
}

/// One structured mathematical source unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Formula<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Semantic presentation family for this mathematical unit.
    pub mode: FormulaMode,
    /// Exact authored mathematical source retained without rewriting.
    pub source: String,
}

/// Active-session identity allocation authority.
#[derive(Debug)]
pub struct IdentityAllocator {
    accepted: Cell<Option<NonZeroU64>>,
    candidate: Cell<Option<NonZeroU64>>,
    revision: Cell<Option<NonZeroU64>>,
}

/// Exhaustion of one opaque process-local identity sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityExhausted {
    /// Accepted semantic identity sequence exhausted.
    Accepted,
    /// Candidate-local semantic identity sequence exhausted.
    Candidate,
    /// Accepted revision identity sequence exhausted.
    Revision,
}

impl Default for IdentityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentityAllocator {
    /// Allocate one accepted semantic identity.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityExhausted::Accepted`] after the active-session
    /// sequence has exhausted every non-zero internal ordinal.
    pub fn allocate_accepted(
        &self,
    ) -> Result<AcceptedIdentity, IdentityExhausted> {
        allocate_next(&self.accepted, IdentityExhausted::Accepted)
            .map(AcceptedIdentity)
    }

    /// Allocate one candidate-local semantic identity.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityExhausted::Candidate`] after the active-session
    /// sequence has exhausted every non-zero internal ordinal.
    pub fn allocate_candidate(
        &self,
    ) -> Result<CandidateIdentity, IdentityExhausted> {
        allocate_next(&self.candidate, IdentityExhausted::Candidate)
            .map(CandidateIdentity)
    }

    /// Allocate one accepted revision identity.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityExhausted::Revision`] after the active-session
    /// sequence has exhausted every non-zero internal ordinal.
    pub fn allocate_revision(
        &self,
    ) -> Result<RevisionIdentity, IdentityExhausted> {
        allocate_next(&self.revision, IdentityExhausted::Revision)
            .map(RevisionIdentity)
    }

    /// Construct fresh non-recycling identity sequences for one active session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            accepted: Cell::new(Some(NonZeroU64::MIN)),
            candidate: Cell::new(Some(NonZeroU64::MIN)),
            revision: Cell::new(Some(NonZeroU64::MIN)),
        }
    }
}

/// One editable inline text span with its own semantic identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InlineSpan<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Optional provenance identity owned by the same notebook.
    pub provenance: Option<Identity>,
    /// Optional style identity owned by the same notebook.
    pub style: Option<Identity>,
    /// Exact authored Unicode text.
    pub text: String,
}

/// One ordered or unordered semantic list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct List<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Ordered semantic list items.
    pub items: Vec<ListItem<Identity>>,
    /// Whether list ordering is semantically significant.
    pub ordered: bool,
}

/// One semantic list item containing one or more blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListItem<Identity> {
    /// Semantic child blocks in reading order.
    pub blocks: Vec<Block<Identity>>,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
}

/// Complete semantic notebook authority independent from layout and rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notebook<Identity> {
    /// Session-owned semantic asset references.
    pub assets: Vec<Asset<Identity>>,
    /// Revision-owned semantic constraints.
    pub constraints: Vec<Constraint<Identity>>,
    /// Explicitly admitted opaque extension data.
    pub extensions: Vec<ExtensionData>,
    /// Stable or candidate-local notebook identity.
    pub id: Identity,
    /// Output-profile semantic references.
    pub output_profiles: Vec<OutputProfile<Identity>>,
    /// Physical page profiles owned by this accepted semantic revision.
    pub page_profiles: Vec<PaperProfile<Identity>>,
    /// Pages in semantic notebook order.
    pub pages: Vec<Page<Identity>>,
    /// Provenance records referenced by semantic content.
    pub provenance: Vec<Provenance<Identity>>,
    /// Reusable semantic style records.
    pub styles: Vec<Style<Identity>>,
}

/// One semantic output-profile reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputProfile<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Human-readable profile label; not a filesystem path.
    pub name: String,
}

/// One semantic notebook page containing reading flows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Page<Identity> {
    /// Semantic flows in page-owned reading order.
    pub flows: Vec<Flow<Identity>>,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Physical page-profile identity used by this page.
    pub page_profile: Identity,
}

/// One semantic identity owning a complete physical page profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaperProfile<Identity> {
    /// Exact validated physical paper geometry and page-mark intent.
    pub geometry: PhysicalPageProfile,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
}

/// One semantic provenance record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provenance<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Provenance meaning retained independently from presentation.
    pub kind: ProvenanceKind,
    /// Optional caller-visible source reference, never an internal storage
    /// path.
    pub reference: Option<String>,
}

/// Provenance categories required by the first-release source contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProvenanceKind {
    /// Claim backed by an explicitly supplied citation.
    Cited,
    /// Material derived by Atrament or an admitted external model.
    Derived,
    /// Material supplied directly by the user or imported source.
    Supplied,
    /// Material whose source status remains unresolved.
    Unresolved,
}

/// Opaque accepted revision identity allocated only at commit boundaries.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RevisionIdentity(NonZeroU64);

/// One reusable semantic style identity and label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Style<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Human-readable semantic style label.
    pub name: String,
}

/// One structured semantic table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Table<Identity> {
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Rows in semantic reading order.
    pub rows: Vec<TableRow<Identity>>,
}

/// One semantic table cell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableCell<Identity> {
    /// Semantic child blocks retained inside this cell.
    pub blocks: Vec<Block<Identity>>,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
}

/// One semantic table row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableRow<Identity> {
    /// Cells in semantic column order.
    pub cells: Vec<TableCell<Identity>>,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Semantic row role independent from visual styling.
    pub role: TableRowRole,
}

/// Semantic role of one table row.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TableRowRole {
    /// Ordinary table data row.
    Body,
    /// Header row semantically distinct from table data.
    Header,
}

/// Unsupported or ambiguous semantic content preserved without guessing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnresolvedBlock {
    /// Explicitly admitted extension data attached to the unresolved content.
    pub extensions: Vec<ExtensionData>,
    /// Typed reason this block remains unresolved.
    pub reason: UnresolvedReason,
    /// Exact caller-visible source retained for later resolution.
    pub source: String,
}

/// Semantic subtype owned by one block identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticBlockKind {
    /// Callout block containing nested semantic blocks.
    Callout,
    /// Date block containing inline text spans.
    Date,
    /// Figure block with a figure identity and optional caption spans.
    Figure,
    /// Explicit freeform semantic region.
    Freeform,
    /// Heading block containing inline text spans.
    Heading,
    /// Structured semantic list block.
    List,
    /// Structured mathematical source block.
    Mathematics,
    /// Paragraph block containing inline text spans.
    Paragraph,
    /// Semantic rule or divider block.
    Rule,
    /// Structured semantic table block.
    Table,
    /// Unsupported or ambiguous semantic content block.
    Unresolved,
}

/// Read-only semantic kind owned by one notebook identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticIdentityKind {
    /// Session-owned semantic asset reference.
    Asset,
    /// Block identity with its semantic subtype.
    Block(SemanticBlockKind),
    /// Revision-owned semantic constraint.
    Constraint,
    /// Figure value nested inside a figure block.
    Figure,
    /// Ordered semantic flow.
    Flow,
    /// Structured mathematical source unit.
    Formula,
    /// Editable inline Unicode text span.
    InlineSpan,
    /// Structured semantic list.
    List,
    /// One semantic list item.
    ListItem,
    /// Complete semantic notebook authority.
    Notebook,
    /// Semantic output-profile reference.
    OutputProfile,
    /// Semantic notebook page.
    Page,
    /// Physical page profile owned by semantic authority.
    PageProfile,
    /// Semantic provenance record.
    Provenance,
    /// Reusable semantic style.
    Style,
    /// Structured semantic table.
    Table,
    /// Semantic table cell.
    TableCell,
    /// Semantic table row.
    TableRow,
}

/// Reason semantic content remains unresolved.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UnresolvedReason {
    /// Source admits more than one semantic interpretation.
    Ambiguous,
    /// Source uses a semantic family unsupported by the active model version.
    Unsupported,
}

/// Resolve the semantic kind owned by one identity in a notebook snapshot.
///
/// This is a read-only semantic inspection primitive. It exposes no serialized
/// offsets, memory addresses, storage paths, page pixels, or adapter state.
#[must_use]
pub fn semantic_identity_kind<Identity>(
    notebook: &Notebook<Identity>,
    target: Identity,
) -> Option<SemanticIdentityKind>
where
    Identity: Copy + Eq,
{
    if notebook.id == target {
        return Some(SemanticIdentityKind::Notebook);
    }
    for asset in &notebook.assets {
        if asset.id == target {
            return Some(SemanticIdentityKind::Asset);
        }
    }
    for constraint in &notebook.constraints {
        if constraint.id == target {
            return Some(SemanticIdentityKind::Constraint);
        }
    }
    for profile in &notebook.output_profiles {
        if profile.id == target {
            return Some(SemanticIdentityKind::OutputProfile);
        }
    }
    for profile in &notebook.page_profiles {
        if profile.id == target {
            return Some(SemanticIdentityKind::PageProfile);
        }
    }
    for page in &notebook.pages {
        if page.id == target {
            return Some(SemanticIdentityKind::Page);
        }
        for flow in &page.flows {
            if flow.id == target {
                return Some(SemanticIdentityKind::Flow);
            }
            if let Some(kind) =
                semantic_blocks_identity_kind(&flow.blocks, target)
            {
                return Some(kind);
            }
        }
    }
    for provenance in &notebook.provenance {
        if provenance.id == target {
            return Some(SemanticIdentityKind::Provenance);
        }
    }
    for style in &notebook.styles {
        if style.id == target {
            return Some(SemanticIdentityKind::Style);
        }
    }
    None
}

const fn semantic_block_kind<Identity>(
    content: &BlockContent<Identity>,
) -> SemanticBlockKind {
    match content {
        BlockContent::Callout(_) => SemanticBlockKind::Callout,
        BlockContent::Date(_) => SemanticBlockKind::Date,
        BlockContent::Figure(_) => SemanticBlockKind::Figure,
        BlockContent::Freeform(_) => SemanticBlockKind::Freeform,
        BlockContent::Heading(_) => SemanticBlockKind::Heading,
        BlockContent::List(_) => SemanticBlockKind::List,
        BlockContent::Mathematics(_) => SemanticBlockKind::Mathematics,
        BlockContent::Paragraph(_) => SemanticBlockKind::Paragraph,
        BlockContent::Rule => SemanticBlockKind::Rule,
        BlockContent::Table(_) => SemanticBlockKind::Table,
        BlockContent::Unresolved(_) => SemanticBlockKind::Unresolved,
    }
}

fn semantic_blocks_identity_kind<Identity>(
    blocks: &[Block<Identity>],
    target: Identity,
) -> Option<SemanticIdentityKind>
where
    Identity: Copy + Eq,
{
    for block in blocks {
        if block.id == target {
            return Some(SemanticIdentityKind::Block(semantic_block_kind(
                &block.content,
            )));
        }
        if let Some(kind) =
            semantic_content_identity_kind(&block.content, target)
        {
            return Some(kind);
        }
    }
    None
}

fn semantic_content_identity_kind<Identity>(
    content: &BlockContent<Identity>,
    target: Identity,
) -> Option<SemanticIdentityKind>
where
    Identity: Copy + Eq,
{
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            semantic_blocks_identity_kind(blocks, target)
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => {
            semantic_spans_identity_kind(spans, target)
        },
        BlockContent::Figure(figure) => {
            if figure.id == target {
                return Some(SemanticIdentityKind::Figure);
            }
            semantic_spans_identity_kind(&figure.caption, target)
        },
        BlockContent::List(list) => {
            if list.id == target {
                return Some(SemanticIdentityKind::List);
            }
            for item in &list.items {
                if item.id == target {
                    return Some(SemanticIdentityKind::ListItem);
                }
                if let Some(kind) =
                    semantic_blocks_identity_kind(&item.blocks, target)
                {
                    return Some(kind);
                }
            }
            None
        },
        BlockContent::Mathematics(formula) => {
            (formula.id == target).then_some(SemanticIdentityKind::Formula)
        },
        BlockContent::Rule | BlockContent::Unresolved(_) => None,
        BlockContent::Table(table) => {
            if table.id == target {
                return Some(SemanticIdentityKind::Table);
            }
            for row in &table.rows {
                if row.id == target {
                    return Some(SemanticIdentityKind::TableRow);
                }
                for cell in &row.cells {
                    if cell.id == target {
                        return Some(SemanticIdentityKind::TableCell);
                    }
                    if let Some(kind) =
                        semantic_blocks_identity_kind(&cell.blocks, target)
                    {
                        return Some(kind);
                    }
                }
            }
            None
        },
    }
}

fn semantic_spans_identity_kind<Identity>(
    spans: &[InlineSpan<Identity>],
    target: Identity,
) -> Option<SemanticIdentityKind>
where
    Identity: Copy + Eq,
{
    spans
        .iter()
        .any(|span| span.id == target)
        .then_some(SemanticIdentityKind::InlineSpan)
}

fn allocate_next(
    sequence: &Cell<Option<NonZeroU64>>,
    exhausted: IdentityExhausted,
) -> Result<NonZeroU64, IdentityExhausted> {
    let Some(current) = sequence.get() else {
        return Err(exhausted);
    };
    let next = current.get().checked_add(1).and_then(NonZeroU64::new);
    sequence.set(next);
    Ok(current)
}
