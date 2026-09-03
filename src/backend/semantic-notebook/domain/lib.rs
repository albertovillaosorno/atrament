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
use std::num::{NonZeroU32, NonZeroU64};

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

impl<Identity> Table<Identity>
where
    Identity: Copy,
{
    /// Validate logical row/column spans as one complete rectangular grid.
    ///
    /// The first row establishes logical width. Later rows fill the first
    /// unoccupied columns while inherited row spans reserve their coverage.
    ///
    /// # Errors
    ///
    /// Returns the first semantic cell or row whose span cannot participate in
    /// one complete non-overlapping rectangular table grid.
    pub fn validate_grid(&self) -> Result<(), TableGridError<Identity>> {
        let width = table_grid_width(self)?;
        let row_count = u64::try_from(self.rows.len()).unwrap_or(u64::MAX);
        let mut active = Vec::<TableActiveSpan>::new();
        let mut current_row = 0u64;
        for row in &self.rows {
            active.retain(|span| current_row < span.until_row);
            active.sort_unstable_by_key(|span| span.start);
            let mut cursor = 0u64;
            let mut additions = Vec::<TableActiveSpan>::new();
            for cell in &row.cells {
                cursor = table_grid_advance_cursor(cursor, &active);
                let columns = NonZeroU64::from(cell.span.columns).get();
                let end = cursor.checked_add(columns).ok_or(
                    TableGridError::ColumnSpan { cell: cell.id },
                )?;
                if end > width
                    || active
                        .iter()
                        .any(|span| span.start < end && cursor < span.end)
                {
                    return Err(TableGridError::ColumnSpan { cell: cell.id });
                }
                let until_row = current_row
                    .checked_add(NonZeroU64::from(cell.span.rows).get())
                    .ok_or(TableGridError::RowSpan { cell: cell.id })?;
                if until_row > row_count {
                    return Err(TableGridError::RowSpan { cell: cell.id });
                }
                if cell.span.rows.get() > 1 {
                    additions.push(TableActiveSpan {
                        end,
                        start: cursor,
                        until_row,
                    });
                }
                cursor = end;
            }
            cursor = table_grid_advance_cursor(cursor, &active);
            if cursor != width {
                return Err(TableGridError::RowWidth { row: row.id });
            }
            active.extend(additions);
            current_row = current_row.saturating_add(1);
        }
        Ok(())
    }
}

/// One semantic table cell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableCell<Identity> {
    /// Semantic child blocks retained inside this cell.
    pub blocks: Vec<Block<Identity>>,
    /// Stable or candidate-local semantic identity.
    pub id: Identity,
    /// Logical rectangular coverage in table rows and columns.
    pub span: TableCellSpan,
}

/// Nonzero logical row and column coverage for one semantic table cell.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TableCellSpan {
    /// Number of logical table columns covered by the cell.
    pub columns: NonZeroU32,
    /// Number of logical table rows covered by the cell.
    pub rows: NonZeroU32,
}

impl TableCellSpan {
    /// Ordinary unmerged table-cell coverage.
    pub const SINGLE: Self = Self {
        columns: NonZeroU32::MIN,
        rows: NonZeroU32::MIN,
    };
}

impl Default for TableCellSpan {
    fn default() -> Self {
        Self::SINGLE
    }
}

/// Typed structural failure for one logical merged-cell table grid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableGridError<Identity> {
    /// One cell cannot fit the table's logical columns without overlap.
    ColumnSpan {
        /// Semantic cell identity with invalid horizontal coverage.
        cell: Identity,
    },
    /// One cell's logical row coverage extends beyond the table.
    RowSpan {
        /// Semantic cell identity with invalid vertical coverage.
        cell: Identity,
    },
    /// One row leaves logical table columns uncovered.
    RowWidth {
        /// Semantic row identity that does not cover the table width.
        row: Identity,
    },
}

#[derive(Clone, Copy)]
struct TableActiveSpan {
    end: u64,
    start: u64,
    until_row: u64,
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

/// Read-only semantic identity descriptor in one notebook snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticIdentityDescriptor<Identity> {
    /// Semantic kind owned by this identity.
    pub kind: SemanticIdentityKind,
    /// Direct structural owner, or `None` for the notebook root.
    pub owner: Option<Identity>,
}

/// Reason semantic content remains unresolved.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UnresolvedReason {
    /// Source admits more than one semantic interpretation.
    Ambiguous,
    /// Source uses a semantic family unsupported by the active model version.
    Unsupported,
}

#[derive(Clone, Copy)]
enum SemanticDescriptorFrame<'notebook, Identity> {
    Blocks {
        blocks: &'notebook [Block<Identity>],
        owner: Identity,
    },
    ListItems {
        items: &'notebook [ListItem<Identity>],
        owner: Identity,
    },
    TableCells {
        cells: &'notebook [TableCell<Identity>],
        owner: Identity,
    },
    TableRows {
        owner: Identity,
        rows: &'notebook [TableRow<Identity>],
    },
}

/// Resolve one identity's semantic kind and direct structural owner.
///
/// This is a read-only semantic inspection primitive. It exposes no serialized
/// offsets, memory addresses, storage paths, page pixels, or adapter state.
#[must_use]
pub fn semantic_identity_descriptor<Identity>(
    notebook: &Notebook<Identity>,
    target: Identity,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    if notebook.id == target {
        return Some(SemanticIdentityDescriptor {
            kind: SemanticIdentityKind::Notebook,
            owner: None,
        });
    }
    for asset in &notebook.assets {
        if asset.id == target {
            return Some(descriptor(SemanticIdentityKind::Asset, notebook.id));
        }
    }
    for constraint in &notebook.constraints {
        if constraint.id == target {
            return Some(descriptor(
                SemanticIdentityKind::Constraint,
                notebook.id,
            ));
        }
    }
    for profile in &notebook.output_profiles {
        if profile.id == target {
            return Some(descriptor(
                SemanticIdentityKind::OutputProfile,
                notebook.id,
            ));
        }
    }
    for profile in &notebook.page_profiles {
        if profile.id == target {
            return Some(descriptor(
                SemanticIdentityKind::PageProfile,
                notebook.id,
            ));
        }
    }
    for page in &notebook.pages {
        if page.id == target {
            return Some(descriptor(SemanticIdentityKind::Page, notebook.id));
        }
        for flow in &page.flows {
            if flow.id == target {
                return Some(descriptor(SemanticIdentityKind::Flow, page.id));
            }
            if let Some(found) =
                semantic_blocks_descriptor(&flow.blocks, target, flow.id)
            {
                return Some(found);
            }
        }
    }
    for provenance in &notebook.provenance {
        if provenance.id == target {
            return Some(descriptor(
                SemanticIdentityKind::Provenance,
                notebook.id,
            ));
        }
    }
    for style in &notebook.styles {
        if style.id == target {
            return Some(descriptor(SemanticIdentityKind::Style, notebook.id));
        }
    }
    None
}

/// Resolve only the semantic kind owned by one identity in a notebook snapshot.
#[must_use]
pub fn semantic_identity_kind<Identity>(
    notebook: &Notebook<Identity>,
    target: Identity,
) -> Option<SemanticIdentityKind>
where
    Identity: Copy + Eq,
{
    semantic_identity_descriptor(notebook, target).map(|found| found.kind)
}

const fn descriptor<Identity>(
    kind: SemanticIdentityKind,
    owner: Identity,
) -> SemanticIdentityDescriptor<Identity>
where
    Identity: Copy,
{
    SemanticIdentityDescriptor { kind, owner: Some(owner) }
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

fn semantic_blocks_descriptor<Identity>(
    root_blocks: &[Block<Identity>],
    target: Identity,
    root_owner: Identity,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    let mut stack = vec![SemanticDescriptorFrame::Blocks {
        blocks: root_blocks,
        owner: root_owner,
    }];
    while let Some(frame) = stack.pop() {
        if let Some(found) =
            semantic_descriptor_frame(frame, target, &mut stack)
        {
            return Some(found);
        }
    }
    None
}

fn semantic_descriptor_frame<'notebook, Identity>(
    frame: SemanticDescriptorFrame<'notebook, Identity>,
    target: Identity,
    stack: &mut Vec<SemanticDescriptorFrame<'notebook, Identity>>,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    match frame {
        SemanticDescriptorFrame::Blocks {
            blocks: current_blocks,
            owner: current_owner,
        } => semantic_block_frame_descriptor(
            current_blocks,
            target,
            current_owner,
            stack,
        ),
        SemanticDescriptorFrame::ListItems {
            items: current_items,
            owner: current_owner,
        } => semantic_list_item_frame_descriptor(
            current_items,
            target,
            current_owner,
            stack,
        ),
        SemanticDescriptorFrame::TableCells {
            cells: current_cells,
            owner: current_owner,
        } => semantic_table_cell_frame_descriptor(
            current_cells,
            target,
            current_owner,
            stack,
        ),
        SemanticDescriptorFrame::TableRows {
            owner: current_owner,
            rows: current_rows,
        } => semantic_table_row_frame_descriptor(
            current_rows,
            target,
            current_owner,
            stack,
        ),
    }
}

fn semantic_block_frame_descriptor<'notebook, Identity>(
    current_blocks: &'notebook [Block<Identity>],
    target: Identity,
    current_owner: Identity,
    stack: &mut Vec<SemanticDescriptorFrame<'notebook, Identity>>,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    let (block, remaining) = current_blocks.split_first()?;
    if !remaining.is_empty() {
        stack.push(SemanticDescriptorFrame::Blocks {
            blocks: remaining,
            owner: current_owner,
        });
    }
    if block.id == target {
        return Some(descriptor(
            SemanticIdentityKind::Block(semantic_block_kind(&block.content)),
            current_owner,
        ));
    }
    semantic_block_content_descriptor(block, target, stack)
}

fn semantic_block_content_descriptor<'notebook, Identity>(
    block: &'notebook Block<Identity>,
    target: Identity,
    stack: &mut Vec<SemanticDescriptorFrame<'notebook, Identity>>,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    match &block.content {
        BlockContent::Callout(children) | BlockContent::Freeform(children) => {
            if !children.is_empty() {
                stack.push(SemanticDescriptorFrame::Blocks {
                    blocks: children,
                    owner: block.id,
                });
            }
            None
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => {
            semantic_spans_descriptor(spans, target, block.id)
        },
        BlockContent::Figure(figure) => {
            if figure.id == target {
                return Some(descriptor(
                    SemanticIdentityKind::Figure,
                    block.id,
                ));
            }
            semantic_spans_descriptor(&figure.caption, target, figure.id)
        },
        BlockContent::List(list) => {
            if list.id == target {
                return Some(descriptor(SemanticIdentityKind::List, block.id));
            }
            if !list.items.is_empty() {
                stack.push(SemanticDescriptorFrame::ListItems {
                    items: &list.items,
                    owner: list.id,
                });
            }
            None
        },
        BlockContent::Mathematics(formula) => (formula.id == target)
            .then_some(descriptor(SemanticIdentityKind::Formula, block.id)),
        BlockContent::Rule | BlockContent::Unresolved(_) => None,
        BlockContent::Table(table) => {
            if table.id == target {
                return Some(descriptor(SemanticIdentityKind::Table, block.id));
            }
            if !table.rows.is_empty() {
                stack.push(SemanticDescriptorFrame::TableRows {
                    owner: table.id,
                    rows: &table.rows,
                });
            }
            None
        },
    }
}

fn semantic_list_item_frame_descriptor<'notebook, Identity>(
    current_items: &'notebook [ListItem<Identity>],
    target: Identity,
    current_owner: Identity,
    stack: &mut Vec<SemanticDescriptorFrame<'notebook, Identity>>,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    let (item, remaining) = current_items.split_first()?;
    if !remaining.is_empty() {
        stack.push(SemanticDescriptorFrame::ListItems {
            items: remaining,
            owner: current_owner,
        });
    }
    if item.id == target {
        return Some(descriptor(SemanticIdentityKind::ListItem, current_owner));
    }
    if !item.blocks.is_empty() {
        stack.push(SemanticDescriptorFrame::Blocks {
            blocks: &item.blocks,
            owner: item.id,
        });
    }
    None
}

fn semantic_table_cell_frame_descriptor<'notebook, Identity>(
    current_cells: &'notebook [TableCell<Identity>],
    target: Identity,
    current_owner: Identity,
    stack: &mut Vec<SemanticDescriptorFrame<'notebook, Identity>>,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    let (cell, remaining) = current_cells.split_first()?;
    if !remaining.is_empty() {
        stack.push(SemanticDescriptorFrame::TableCells {
            cells: remaining,
            owner: current_owner,
        });
    }
    if cell.id == target {
        return Some(descriptor(
            SemanticIdentityKind::TableCell,
            current_owner,
        ));
    }
    if !cell.blocks.is_empty() {
        stack.push(SemanticDescriptorFrame::Blocks {
            blocks: &cell.blocks,
            owner: cell.id,
        });
    }
    None
}

fn semantic_table_row_frame_descriptor<'notebook, Identity>(
    current_rows: &'notebook [TableRow<Identity>],
    target: Identity,
    current_owner: Identity,
    stack: &mut Vec<SemanticDescriptorFrame<'notebook, Identity>>,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    let (row, remaining) = current_rows.split_first()?;
    if !remaining.is_empty() {
        stack.push(SemanticDescriptorFrame::TableRows {
            owner: current_owner,
            rows: remaining,
        });
    }
    if row.id == target {
        return Some(descriptor(SemanticIdentityKind::TableRow, current_owner));
    }
    if !row.cells.is_empty() {
        stack.push(SemanticDescriptorFrame::TableCells {
            cells: &row.cells,
            owner: row.id,
        });
    }
    None
}

fn semantic_spans_descriptor<Identity>(
    spans: &[InlineSpan<Identity>],
    target: Identity,
    owner: Identity,
) -> Option<SemanticIdentityDescriptor<Identity>>
where
    Identity: Copy + Eq,
{
    spans.iter().any(|span| span.id == target).then_some(
        SemanticIdentityDescriptor {
            kind: SemanticIdentityKind::InlineSpan,
            owner: Some(owner),
        },
    )
}

fn table_grid_advance_cursor(
    mut cursor: u64,
    active: &[TableActiveSpan],
) -> u64 {
    for span in active {
        if cursor < span.start {
            break;
        }
        if cursor < span.end {
            cursor = span.end;
        }
    }
    cursor
}

fn table_grid_width<Identity>(
    table: &Table<Identity>,
) -> Result<u64, TableGridError<Identity>>
where
    Identity: Copy,
{
    let Some(first_row) = table.rows.first() else {
        return Ok(0);
    };
    first_row.cells.iter().try_fold(0u64, |width, cell| {
        let columns = NonZeroU64::from(cell.span.columns).get();
        width
            .checked_add(columns)
            .ok_or(TableGridError::ColumnSpan { cell: cell.id })
    })
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
