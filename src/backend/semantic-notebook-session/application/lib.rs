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
//   - Active-session accepted semantic revision and candidate promotion.
// - Must-Not:
//   - Persist notebooks, parse transport text, perform layout, or reuse IDs.
// - Allows:
//   - Inputs: Complete candidate semantic notebook values from prior
//     validation.
//   - Outputs: Current accepted revision and candidate-to-accepted ID mapping.
//   - Side effects: Atomic process-memory accepted revision replacement only.
// - Split-When:
//   - Semantic command Apply or history requires independent transaction state.
// - Merge-When:
//   - One application authority subsumes all accepted semantic transactions.
// - Summary:
//   - Promotes one candidate notebook into accepted in-memory session
//     authority.
// - Description:
//   - Validates candidate identity graph before allocating accepted authority.
// - Usage:
//   - Own one service instance for one active disposable Atrament session.
// - Defaults:
//   - Starts without an accepted revision and never persists session state.
//

//! Atomic in-memory acceptance of complete semantic notebook candidates.

use std::collections::BTreeMap;
use std::fmt;

use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, Asset, Block, BlockContent,
    CandidateIdentity, Constraint, Figure, Flow, Formula, IdentityAllocator,
    IdentityExhausted, InlineSpan, List, ListItem, Notebook, OutputProfile,
    Page, Provenance, Style, Table, TableCell, TableRow,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CandidateGraphError, CandidateReferenceKind,
    IdentityMapping, SemanticNotebookSession, TextEditOutcome,
};

#[derive(Debug, Default)]
struct CandidateGraph {
    owners: Vec<CandidateIdentity>,
    references: Vec<(CandidateIdentity, CandidateReferenceKind)>,
    seen: BTreeMap<CandidateIdentity, CandidateReferenceKind>,
}

impl CandidateGraph {
    fn finish(self) -> Result<Vec<CandidateIdentity>, CandidateGraphError> {
        for (reference, expected) in self.references {
            let Some(observed) = self.seen.get(&reference) else {
                return Err(CandidateGraphError::MissingReference {
                    candidate: reference,
                });
            };
            if expected != CandidateReferenceKind::Semantic
                && *observed != expected
            {
                return Err(CandidateGraphError::ReferenceKindMismatch {
                    candidate: reference,
                    expected,
                });
            }
        }
        Ok(self.owners)
    }

    fn reference(
        &mut self,
        identity: Option<CandidateIdentity>,
        kind: CandidateReferenceKind,
    ) {
        if let Some(reference) = identity {
            self.references.push((reference, kind));
        }
    }

    fn register(
        &mut self,
        identity: CandidateIdentity,
        kind: CandidateReferenceKind,
    ) -> Result<(), CandidateGraphError> {
        let previous = self.seen.insert(identity, kind);
        if previous.is_some() {
            return Err(CandidateGraphError::Duplicate { candidate: identity });
        }
        self.owners.push(identity);
        Ok(())
    }
}

/// Process-local accepted semantic notebook authority for one active session.
#[derive(Default)]
pub struct SemanticNotebookSessionService {
    current: Option<AcceptedRevision>,
    identities: IdentityAllocator,
}

impl fmt::Debug for SemanticNotebookSessionService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticNotebookSessionService")
            .finish_non_exhaustive()
    }
}

impl SemanticNotebookSession for SemanticNotebookSessionService {
    fn accept(
        &mut self,
        candidate: Notebook<CandidateIdentity>,
    ) -> AcceptanceOutcome {
        let owners = match candidate_identities(&candidate) {
            Ok(owners) => owners,
            Err(reason) => {
                return AcceptanceOutcome::InvalidCandidate { reason };
            },
        };
        let (identity_map, mapping) = match self.allocate_mapping(&owners) {
            Ok(mapping) => mapping,
            Err(sequence) => {
                return AcceptanceOutcome::IdentityExhausted { sequence };
            },
        };
        let notebook = match accept_notebook(candidate, &identity_map) {
            Ok(notebook) => notebook,
            Err(reason) => {
                return AcceptanceOutcome::InvalidCandidate { reason };
            },
        };
        let revision = match self.identities.allocate_revision() {
            Ok(revision) => revision,
            Err(sequence) => {
                return AcceptanceOutcome::IdentityExhausted { sequence };
            },
        };
        self.current = Some(AcceptedRevision { id: revision, notebook });
        AcceptanceOutcome::Accepted { mapping, revision }
    }

    fn current(&self) -> Option<&AcceptedRevision> {
        self.current.as_ref()
    }

    fn replace_text(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        value: String,
    ) -> TextEditOutcome {
        let Some(current) = self.current.as_ref() else {
            return TextEditOutcome::NoAcceptedRevision;
        };
        if current.id != base {
            return TextEditOutcome::StaleBase { current: current.id };
        }
        let Some(existing) = text_value(&current.notebook, target) else {
            return TextEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        };
        if existing == value {
            return TextEditOutcome::NoOp {
                revision: current.id,
                target,
            };
        }
        let mut notebook = current.notebook.clone();
        let edited = replace_text_value(&mut notebook, target, value);
        if !edited {
            return TextEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.identities.allocate_revision() {
            Ok(revision) => revision,
            Err(sequence) => {
                return TextEditOutcome::IdentityExhausted { sequence };
            },
        };
        self.current = Some(AcceptedRevision { id: revision, notebook });
        TextEditOutcome::Applied { base, revision, target }
    }
}

impl SemanticNotebookSessionService {
    fn allocate_mapping(
        &self,
        owners: &[CandidateIdentity],
    ) -> Result<
        (
            BTreeMap<CandidateIdentity, AcceptedIdentity>,
            Vec<IdentityMapping>,
        ),
        IdentityExhausted,
    > {
        let mut identity_map = BTreeMap::new();
        let mut mapping = Vec::with_capacity(owners.len());
        for candidate in owners {
            let accepted = self.identities.allocate_accepted()?;
            let _previous: Option<AcceptedIdentity> =
                identity_map.insert(*candidate, accepted);
            mapping.push(IdentityMapping {
                accepted,
                candidate: *candidate,
            });
        }
        Ok((identity_map, mapping))
    }
}

fn accept_asset(
    asset: Asset<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Asset<AcceptedIdentity>, CandidateGraphError> {
    Ok(Asset {
        id: accepted_id(asset.id, identities)?,
        media_type: asset.media_type,
    })
}

fn accept_block(
    block: Block<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Block<AcceptedIdentity>, CandidateGraphError> {
    Ok(Block {
        content: accept_block_content(block.content, identities)?,
        extensions: block.extensions,
        id: accepted_id(block.id, identities)?,
        provenance: accepted_reference(block.provenance, identities)?,
        style: accepted_reference(block.style, identities)?,
    })
}

fn accept_block_content(
    content: BlockContent<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<BlockContent<AcceptedIdentity>, CandidateGraphError> {
    match content {
        BlockContent::Callout(blocks) => {
            Ok(BlockContent::Callout(accept_blocks(blocks, identities)?))
        },
        BlockContent::Date(spans) => {
            Ok(BlockContent::Date(accept_spans(spans, identities)?))
        },
        BlockContent::Figure(figure) => {
            Ok(BlockContent::Figure(accept_figure(figure, identities)?))
        },
        BlockContent::Freeform(blocks) => {
            Ok(BlockContent::Freeform(accept_blocks(blocks, identities)?))
        },
        BlockContent::Heading(spans) => {
            Ok(BlockContent::Heading(accept_spans(spans, identities)?))
        },
        BlockContent::List(list) => {
            Ok(BlockContent::List(accept_list(list, identities)?))
        },
        BlockContent::Mathematics(formula) => Ok(BlockContent::Mathematics(
            accept_formula(formula, identities)?,
        )),
        BlockContent::Paragraph(spans) => {
            Ok(BlockContent::Paragraph(accept_spans(spans, identities)?))
        },
        BlockContent::Rule => Ok(BlockContent::Rule),
        BlockContent::Table(table) => {
            Ok(BlockContent::Table(accept_table(table, identities)?))
        },
        BlockContent::Unresolved(unresolved) => {
            Ok(BlockContent::Unresolved(unresolved))
        },
    }
}

fn accept_blocks(
    blocks: Vec<Block<CandidateIdentity>>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Vec<Block<AcceptedIdentity>>, CandidateGraphError> {
    blocks
        .into_iter()
        .map(|block| accept_block(block, identities))
        .collect()
}

fn accept_constraint(
    constraint: &Constraint<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Constraint<AcceptedIdentity>, CandidateGraphError> {
    Ok(Constraint {
        id: accepted_id(constraint.id, identities)?,
        kind: constraint.kind,
        target: accepted_id(constraint.target, identities)?,
    })
}

fn accept_figure(
    figure: Figure<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Figure<AcceptedIdentity>, CandidateGraphError> {
    Ok(Figure {
        asset: accepted_reference(figure.asset, identities)?,
        caption: accept_spans(figure.caption, identities)?,
        id: accepted_id(figure.id, identities)?,
    })
}

fn accept_flow(
    flow: Flow<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Flow<AcceptedIdentity>, CandidateGraphError> {
    Ok(Flow {
        blocks: accept_blocks(flow.blocks, identities)?,
        id: accepted_id(flow.id, identities)?,
    })
}

fn accept_formula(
    formula: Formula<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Formula<AcceptedIdentity>, CandidateGraphError> {
    Ok(Formula {
        id: accepted_id(formula.id, identities)?,
        source: formula.source,
    })
}

fn accept_list(
    list: List<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<List<AcceptedIdentity>, CandidateGraphError> {
    let items = list
        .items
        .into_iter()
        .map(|item| accept_list_item(item, identities))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(List {
        id: accepted_id(list.id, identities)?,
        items,
        ordered: list.ordered,
    })
}

fn accept_list_item(
    item: ListItem<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<ListItem<AcceptedIdentity>, CandidateGraphError> {
    Ok(ListItem {
        blocks: accept_blocks(item.blocks, identities)?,
        id: accepted_id(item.id, identities)?,
    })
}

fn accept_notebook(
    notebook: Notebook<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Notebook<AcceptedIdentity>, CandidateGraphError> {
    Ok(Notebook {
        assets: notebook
            .assets
            .into_iter()
            .map(|asset| accept_asset(asset, identities))
            .collect::<Result<Vec<_>, _>>()?,
        constraints: notebook
            .constraints
            .iter()
            .map(|constraint| accept_constraint(constraint, identities))
            .collect::<Result<Vec<_>, _>>()?,
        extensions: notebook.extensions,
        id: accepted_id(notebook.id, identities)?,
        output_profiles: notebook
            .output_profiles
            .into_iter()
            .map(|profile| accept_output_profile(profile, identities))
            .collect::<Result<Vec<_>, _>>()?,
        pages: notebook
            .pages
            .into_iter()
            .map(|page| accept_page(page, identities))
            .collect::<Result<Vec<_>, _>>()?,
        provenance: notebook
            .provenance
            .into_iter()
            .map(|provenance| accept_provenance(provenance, identities))
            .collect::<Result<Vec<_>, _>>()?,
        styles: notebook
            .styles
            .into_iter()
            .map(|style| accept_style(style, identities))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn accept_output_profile(
    profile: OutputProfile<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<OutputProfile<AcceptedIdentity>, CandidateGraphError> {
    Ok(OutputProfile {
        id: accepted_id(profile.id, identities)?,
        name: profile.name,
    })
}

fn accept_page(
    page: Page<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Page<AcceptedIdentity>, CandidateGraphError> {
    Ok(Page {
        flows: page
            .flows
            .into_iter()
            .map(|flow| accept_flow(flow, identities))
            .collect::<Result<Vec<_>, _>>()?,
        id: accepted_id(page.id, identities)?,
    })
}

fn accept_provenance(
    provenance: Provenance<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Provenance<AcceptedIdentity>, CandidateGraphError> {
    Ok(Provenance {
        id: accepted_id(provenance.id, identities)?,
        kind: provenance.kind,
        reference: provenance.reference,
    })
}

fn accept_spans(
    spans: Vec<InlineSpan<CandidateIdentity>>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Vec<InlineSpan<AcceptedIdentity>>, CandidateGraphError> {
    spans
        .into_iter()
        .map(|span| {
            Ok(InlineSpan {
                id: accepted_id(span.id, identities)?,
                provenance: accepted_reference(span.provenance, identities)?,
                style: accepted_reference(span.style, identities)?,
                text: span.text,
            })
        })
        .collect()
}

fn accept_style(
    style: Style<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Style<AcceptedIdentity>, CandidateGraphError> {
    Ok(Style {
        id: accepted_id(style.id, identities)?,
        name: style.name,
    })
}

fn accept_table(
    table: Table<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Table<AcceptedIdentity>, CandidateGraphError> {
    Ok(Table {
        id: accepted_id(table.id, identities)?,
        rows: table
            .rows
            .into_iter()
            .map(|row| accept_table_row(row, identities))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn accept_table_cell(
    cell: TableCell<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<TableCell<AcceptedIdentity>, CandidateGraphError> {
    Ok(TableCell {
        blocks: accept_blocks(cell.blocks, identities)?,
        id: accepted_id(cell.id, identities)?,
    })
}

fn accept_table_row(
    row: TableRow<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<TableRow<AcceptedIdentity>, CandidateGraphError> {
    Ok(TableRow {
        cells: row
            .cells
            .into_iter()
            .map(|cell| accept_table_cell(cell, identities))
            .collect::<Result<Vec<_>, _>>()?,
        id: accepted_id(row.id, identities)?,
    })
}

fn accepted_id(
    candidate: CandidateIdentity,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<AcceptedIdentity, CandidateGraphError> {
    identities
        .get(&candidate)
        .copied()
        .ok_or(CandidateGraphError::MissingReference { candidate })
}

fn accepted_reference(
    candidate: Option<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Option<AcceptedIdentity>, CandidateGraphError> {
    candidate
        .map(|identity| accepted_id(identity, identities))
        .transpose()
}

fn replace_text_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    value: &mut Option<String>,
) -> bool {
    for block in blocks {
        if replace_text_content(&mut block.content, target, value) {
            return true;
        }
    }
    false
}

fn replace_text_content(
    content: &mut BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
    value: &mut Option<String>,
) -> bool {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            replace_text_blocks(blocks, target, value)
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => {
            replace_text_spans(spans, target, value)
        },
        BlockContent::Figure(figure) => {
            replace_text_spans(&mut figure.caption, target, value)
        },
        BlockContent::List(list) => {
            for item in &mut list.items {
                if replace_text_blocks(&mut item.blocks, target, value) {
                    return true;
                }
            }
            false
        },
        BlockContent::Mathematics(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => false,
        BlockContent::Table(table) => {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    if replace_text_blocks(&mut cell.blocks, target, value) {
                        return true;
                    }
                }
            }
            false
        },
    }
}

fn replace_text_spans(
    spans: &mut [InlineSpan<AcceptedIdentity>],
    target: AcceptedIdentity,
    value: &mut Option<String>,
) -> bool {
    for span in spans {
        if span.id == target {
            let Some(replacement) = value.take() else {
                return false;
            };
            span.text = replacement;
            return true;
        }
    }
    false
}

fn replace_text_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    value: String,
) -> bool {
    let mut replacement = Some(value);
    for page in &mut notebook.pages {
        for flow in &mut page.flows {
            if replace_text_blocks(&mut flow.blocks, target, &mut replacement) {
                return true;
            }
        }
    }
    false
}

fn text_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<&str> {
    for block in blocks {
        if let Some(value) = text_content_value(&block.content, target) {
            return Some(value);
        }
    }
    None
}

fn text_content_value(
    content: &BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&str> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            text_blocks_value(blocks, target)
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => text_spans_value(spans, target),
        BlockContent::Figure(figure) => {
            text_spans_value(&figure.caption, target)
        },
        BlockContent::List(list) => {
            for item in &list.items {
                if let Some(value) = text_blocks_value(&item.blocks, target) {
                    return Some(value);
                }
            }
            None
        },
        BlockContent::Mathematics(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => None,
        BlockContent::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    if let Some(value) = text_blocks_value(&cell.blocks, target)
                    {
                        return Some(value);
                    }
                }
            }
            None
        },
    }
}

fn text_spans_value(
    spans: &[InlineSpan<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<&str> {
    spans
        .iter()
        .find(|span| span.id == target)
        .map(|span| span.text.as_str())
}

fn text_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&str> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(value) = text_blocks_value(&flow.blocks, target) {
                return Some(value);
            }
        }
    }
    None
}

fn candidate_block(
    block: &Block<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(block.id, CandidateReferenceKind::Semantic)?;
    graph.reference(block.provenance, CandidateReferenceKind::Provenance);
    graph.reference(block.style, CandidateReferenceKind::Style);
    candidate_block_content(&block.content, graph)
}

fn candidate_block_content(
    content: &BlockContent<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            candidate_blocks(blocks, graph)
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => candidate_spans(spans, graph),
        BlockContent::Figure(figure) => candidate_figure(figure, graph),
        BlockContent::List(list) => candidate_list(list, graph),
        BlockContent::Mathematics(formula) => {
            graph.register(formula.id, CandidateReferenceKind::Semantic)
        },
        BlockContent::Rule | BlockContent::Unresolved(_) => Ok(()),
        BlockContent::Table(table) => candidate_table(table, graph),
    }
}

fn candidate_blocks(
    blocks: &[Block<CandidateIdentity>],
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    for block in blocks {
        candidate_block(block, graph)?;
    }
    Ok(())
}

fn candidate_figure(
    figure: &Figure<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(figure.id, CandidateReferenceKind::Semantic)?;
    graph.reference(figure.asset, CandidateReferenceKind::Asset);
    candidate_spans(&figure.caption, graph)
}

fn candidate_flow(
    flow: &Flow<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(flow.id, CandidateReferenceKind::Semantic)?;
    candidate_blocks(&flow.blocks, graph)
}

fn candidate_identities(
    notebook: &Notebook<CandidateIdentity>,
) -> Result<Vec<CandidateIdentity>, CandidateGraphError> {
    let mut graph = CandidateGraph::default();
    graph.register(notebook.id, CandidateReferenceKind::Semantic)?;
    for asset in &notebook.assets {
        graph.register(asset.id, CandidateReferenceKind::Asset)?;
    }
    for constraint in &notebook.constraints {
        graph.register(constraint.id, CandidateReferenceKind::Semantic)?;
        graph.reference(
            Some(constraint.target),
            CandidateReferenceKind::Semantic,
        );
    }
    for profile in &notebook.output_profiles {
        graph.register(profile.id, CandidateReferenceKind::Semantic)?;
    }
    for page in &notebook.pages {
        candidate_page(page, &mut graph)?;
    }
    for provenance in &notebook.provenance {
        graph.register(provenance.id, CandidateReferenceKind::Provenance)?;
    }
    for style in &notebook.styles {
        graph.register(style.id, CandidateReferenceKind::Style)?;
    }
    graph.finish()
}

fn candidate_list(
    list: &List<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(list.id, CandidateReferenceKind::Semantic)?;
    for item in &list.items {
        graph.register(item.id, CandidateReferenceKind::Semantic)?;
        candidate_blocks(&item.blocks, graph)?;
    }
    Ok(())
}

fn candidate_page(
    page: &Page<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(page.id, CandidateReferenceKind::Semantic)?;
    for flow in &page.flows {
        candidate_flow(flow, graph)?;
    }
    Ok(())
}

fn candidate_spans(
    spans: &[InlineSpan<CandidateIdentity>],
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    for span in spans {
        graph.register(span.id, CandidateReferenceKind::Semantic)?;
        graph.reference(span.provenance, CandidateReferenceKind::Provenance);
        graph.reference(span.style, CandidateReferenceKind::Style);
    }
    Ok(())
}

fn candidate_table(
    table: &Table<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(table.id, CandidateReferenceKind::Semantic)?;
    for row in &table.rows {
        graph.register(row.id, CandidateReferenceKind::Semantic)?;
        for cell in &row.cells {
            graph.register(cell.id, CandidateReferenceKind::Semantic)?;
            candidate_blocks(&cell.blocks, graph)?;
        }
    }
    Ok(())
}
