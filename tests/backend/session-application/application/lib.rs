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
//   - Regression evidence for one disposable active-session application owner.
// - Must-Not:
//   - Exercise HTTP transport, persistence, or semantic validation details.
// - Allows:
//   - Inputs: Bounded draft text and a minimal valid semantic candidate.
//   - Outputs: Assertions over shared lifecycle ownership and fresh defaults.
//   - Side effects: Test subprocesses and process-local allocations only.
// - Split-When:
//   - Assets, previews, renders, or derived plans join the session owner.
// - Merge-When:
//   - Full process lifecycle fixtures supersede this application-level proof.
// - Summary:
//   - Verifies draft, accepted notebook, and history share one owner.
// - Description:
//   - Proves the live-session application can hold both state classes and a
//     fresh application cannot observe them after the prior owner is dropped.
// - Usage:
//   - Compile against the session application and semantic inbound-port crates.
// - Defaults:
//   - Starts with empty draft fields and no accepted semantic revision.
//
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation, PageProfile,
    PaperMarkAppearance, PaperMarkJoin, PaperMarkLayer, PaperPattern, Rect,
    SheetSize,
};
use atrament_semantic_notebook::{
    Asset, Block, BlockContent, CandidateIdentity, Constraint, ConstraintKind,
    Figure, Flow, FormulaMode, IdentityAllocator, InlineSpan, List, ListItem,
    Notebook, Page, PaperProfile, Provenance,
    ProvenanceKind, SemanticBlockKind, SemanticIdentityDescriptor,
    SemanticIdentityKind, Style, TableCellSpan, TableRowRole,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CommandBehaviorVersion,
    CommandCapabilityCompatibilityOutcome, CommandFamilyAdmissionOutcome,
    CommandGraphLimits,
    CommandTargetMaterialOutcome, CommandTargetPreconditionOutcome,
    CommandTargetPreconditions, DirectEditBatchApplyOutcome,
    DirectEditBatchCommand, DirectEditBatchGraphLimitsOutcome,
    DirectEditBatchGraphSizeOutcome, DirectEditBatchProposal,
    DirectEditBatchSelectionBoundedOutcome,
    DirectEditBatchSelectionRequirementsOutcome,
    DirectEditBatchSelectionSummaryOutcome, DirectEditBatchSimulationOutcome,
    DirectEditChangePreviewOutcome, DirectEditEffectClass, DirectEditProposal,
    DirectEditSemanticChange,
    DirectEditProposalOutcome, DirectEditSimulationOutcome,
    EditableSemanticValue, EditableValuePreconditionOutcome, FormulaEditOutcome,
    HistoryAvailability, HistoryAvailabilityOutcome, HistoryDirection,
    HistoryTraversalOutcome,
    IdentityAncestryCompleteness, IdentityAncestryEntry,
    IdentityAncestryInspectOutcome, IdentityInspectOutcome,
    IdentityKindInspectOutcome, IdentityOwnerExpectation,
    IdentityPrecondition, IdentityPreconditionOutcome, PageProfileEditOutcome,
    SemanticCommandFamily, TableCellSpanEditOutcome, TableRowRoleEditOutcome,
    TextEditOutcome,
};
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};

#[allow(dead_code)]
#[path = "../../../../src/backend/session-application/application/lib.rs"]
mod application;

const CURRENT_COMMAND_BEHAVIOR_VERSION: CommandBehaviorVersion =
    CommandBehaviorVersion(26);

fn physical_page_profile() -> PageProfile {
    PageProfile {
        binding_edge: BindingEdge::Left,
        border_shape: BorderShape::Rectangle,
        corner_roundness: Length::ZERO,
        orientation: Orientation::Portrait,
        outer_margin: Length::from_micrometres(10_000),
        paper_mark_appearance: PaperMarkAppearance {
            join: PaperMarkJoin::Sharp,
            maximum_ruler_error: Length::ZERO,
        },
        paper_mark_layer: PaperMarkLayer::BelowInk,
        paper_pattern: PaperPattern::Blank,
        printable_region: Rect {
            height: Length::from_micrometres(277_000),
            width: Length::from_micrometres(190_000),
            x: Length::from_micrometres(10_000),
            y: Length::from_micrometres(10_000),
        },
        sheet: SheetSize {
            height: Length::from_micrometres(297_000),
            width: Length::from_micrometres(210_000),
        },
        top_clearance: Length::from_micrometres(10_000),
        writing_inset: Length::from_micrometres(5_000),
    }
}

fn editable_text_candidate(
    identities: &IdentityAllocator,
    text: &str,
) -> (Notebook<CandidateIdentity>, CandidateIdentity) {
    let notebook = identities.allocate_candidate().expect("notebook id");
    let profile = identities.allocate_candidate().expect("profile id");
    let page = identities.allocate_candidate().expect("page id");
    let flow = identities.allocate_candidate().expect("flow id");
    let block = identities.allocate_candidate().expect("block id");
    let span = identities.allocate_candidate().expect("span id");
    (
        Notebook {
            assets: vec![],
            constraints: vec![],
            extensions: vec![],
            id: notebook,
            output_profiles: vec![],
            page_profiles: vec![PaperProfile {
                geometry: physical_page_profile(),
                id: profile,
            }],
            pages: vec![Page {
                flows: vec![Flow {
                    blocks: vec![Block {
                        content: BlockContent::Paragraph(vec![InlineSpan {
                            id: span,
                            provenance: None,
                            style: None,
                            text: text.to_owned(),
                        }]),
                        extensions: vec![],
                        id: block,
                        provenance: None,
                        style: None,
                    }],
                    id: flow,
                }],
                id: page,
                page_profile: profile,
            }],
            provenance: vec![],
            styles: vec![],
        },
        span,
    )
}

fn list_ordering_candidate(
    identities: &IdentityAllocator,
) -> (
    Notebook<CandidateIdentity>,
    CandidateIdentity,
    CandidateIdentity,
    CandidateIdentity,
) {
    let (mut candidate, _) = editable_text_candidate(identities, "list child");
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let list = identities.allocate_candidate().expect("list id");
    let item = identities.allocate_candidate().expect("list item id");
    let content = std::mem::replace(
        &mut candidate.pages[0].flows[0].blocks[0].content,
        BlockContent::Rule,
    );
    let child = identities.allocate_candidate().expect("list child block id");
    candidate.pages[0].flows[0].blocks[0].content = BlockContent::List(List {
        id: list,
        items: vec![ListItem {
            blocks: vec![Block {
                content,
                extensions: vec![],
                id: child,
                provenance: None,
                style: None,
            }],
            id: item,
        }],
        ordered: false,
    });
    (candidate, block, flow, list)
}

fn block_style_candidate(
    identities: &IdentityAllocator,
) -> (
    Notebook<CandidateIdentity>,
    CandidateIdentity,
    CandidateIdentity,
    CandidateIdentity,
) {
    let (mut candidate, _) = editable_text_candidate(identities, "styled");
    let block = candidate.pages[0].flows[0].blocks[0].id;
    let flow = candidate.pages[0].flows[0].id;
    let style = identities.allocate_candidate().expect("style id");
    candidate.styles.push(Style {
        id: style,
        name: String::from("body-emphasis"),
    });
    (candidate, block, flow, style)
}

fn page_profile_reference_candidate(
    identities: &IdentityAllocator,
) -> (
    Notebook<CandidateIdentity>,
    CandidateIdentity,
    CandidateIdentity,
    CandidateIdentity,
    CandidateIdentity,
) {
    let (mut candidate, _) = editable_text_candidate(identities, "retarget");
    let notebook = candidate.id;
    let page = candidate.pages[0].id;
    let first = candidate.page_profiles[0].id;
    let second = identities.allocate_candidate().expect("second profile id");
    let mut geometry = physical_page_profile();
    geometry.top_clearance = Length::from_micrometres(12_000);
    candidate.page_profiles.push(PaperProfile {
        geometry,
        id: second,
    });
    (candidate, notebook, page, first, second)
}

fn global_constraint_candidate(
    identities: &IdentityAllocator,
) -> (Notebook<CandidateIdentity>, CandidateIdentity, CandidateIdentity) {
    let (mut candidate, _) = editable_text_candidate(identities, "authored");
    let constraint = identities
        .allocate_candidate()
        .expect("constraint id");
    let notebook = candidate.id;
    candidate.constraints.push(Constraint {
        id: constraint,
        kind: ConstraintKind::Paper,
        target: notebook,
    });
    (candidate, constraint, notebook)
}

fn provenance_claim_candidate(
    identities: &IdentityAllocator,
) -> (Notebook<CandidateIdentity>, CandidateIdentity, CandidateIdentity) {
    let (mut candidate, claim) =
        editable_text_candidate(identities, "Energy is conserved.");
    let provenance = identities
        .allocate_candidate()
        .expect("provenance id");
    candidate.provenance.push(Provenance {
        id: provenance,
        kind: ProvenanceKind::Supplied,
        reference: Some(String::from("source:old")),
    });
    let BlockContent::Paragraph(spans) =
        &mut candidate.pages[0].flows[0].blocks[0].content
    else {
        panic!("provenance candidate must contain paragraph");
    };
    spans[0].provenance = Some(provenance);
    (candidate, claim, provenance)
}

fn asset_figure_candidate(
    identities: &IdentityAllocator,
) -> (
    Notebook<CandidateIdentity>,
    CandidateIdentity,
    CandidateIdentity,
    CandidateIdentity,
) {
    let notebook = identities.allocate_candidate().expect("notebook id");
    let profile = identities.allocate_candidate().expect("profile id");
    let page = identities.allocate_candidate().expect("page id");
    let flow = identities.allocate_candidate().expect("flow id");
    let block = identities.allocate_candidate().expect("block id");
    let figure = identities.allocate_candidate().expect("figure id");
    let first_asset = identities.allocate_candidate().expect("first asset id");
    let second_asset = identities
        .allocate_candidate()
        .expect("second asset id");
    let candidate = Notebook {
        assets: vec![
            Asset {
                id: first_asset,
                media_type: String::from("image/png"),
            },
            Asset {
                id: second_asset,
                media_type: String::from("image/webp"),
            },
        ],
        constraints: vec![],
        extensions: vec![],
        id: notebook,
        output_profiles: vec![],
        page_profiles: vec![PaperProfile {
            geometry: physical_page_profile(),
            id: profile,
        }],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![Block {
                    content: BlockContent::Figure(Figure {
                        asset: Some(first_asset),
                        caption: vec![],
                        id: figure,
                    }),
                    extensions: vec![],
                    id: block,
                    provenance: None,
                    style: None,
                }],
                id: flow,
            }],
            id: page,
            page_profile: profile,
        }],
        provenance: vec![],
        styles: vec![],
    };
    (candidate, figure, first_asset, second_asset)
}

fn minimal_candidate(
    identities: &IdentityAllocator,
) -> Notebook<CandidateIdentity> {
    Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: identities.allocate_candidate().expect("candidate notebook id"),
        output_profiles: vec![],
        page_profiles: vec![],
        pages: vec![],
        provenance: vec![],
        styles: vec![],
    }
}

const PROCESS_FIXTURE_MODE: &str = "ATRAMENT_SESSION_APPLICATION_FIXTURE";
const PROCESS_POPULATED_READY: &str = "atrament-populated-session-ready";
const PROCESS_FRESH_EMPTY: &str = "atrament-fresh-session-empty";
const PROCESS_TEST_NAME: &str =
    "process_restart_drops_accepted_revision_and_history";

fn run_process_fixture_child(mode: &str) {
    if mode == "fresh" {
        let fresh = application::SessionApplication::default();
        assert!(fresh.accepted_revision().is_none());
        assert_eq!(
            fresh.history_availability(),
            HistoryAvailabilityOutcome::NoAcceptedRevision,
        );
        println!("{PROCESS_FRESH_EMPTY}");
        return;
    }
    assert!(matches!(mode, "populated" | "populated-redo"));
    let identities = IdentityAllocator::new();
    let (mut candidate, candidate_span) =
        editable_text_candidate(&identities, "process-private before");
    let candidate_figure = identities.allocate_candidate().expect("figure id");
    let candidate_figure_block =
        identities.allocate_candidate().expect("figure block id");
    let candidate_first_asset =
        identities.allocate_candidate().expect("first asset id");
    let candidate_second_asset =
        identities.allocate_candidate().expect("second asset id");
    candidate.assets.extend([
        Asset {
            id: candidate_first_asset,
            media_type: String::from("image/png"),
        },
        Asset {
            id: candidate_second_asset,
            media_type: String::from("image/webp"),
        },
    ]);
    candidate.pages[0].flows[0].blocks.push(Block {
        content: BlockContent::Figure(Figure {
            asset: Some(candidate_first_asset),
            caption: vec![],
            id: candidate_figure,
        }),
        extensions: vec![],
        id: candidate_figure_block,
        provenance: None,
        style: None,
    });
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("process fixture candidate must be accepted");
    };
    let accepted = |candidate| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("process fixture identity must map")
            .accepted
    };
    let span = accepted(candidate_span);
    let figure = accepted(candidate_figure);
    let first_asset = accepted(candidate_first_asset);
    let second_asset = accepted(candidate_second_asset);
    let TextEditOutcome::Applied {
        revision: text_revision,
        ..
    } = session.replace_text(
        base,
        span,
        String::from("process-private after"),
    ) else {
        panic!("process fixture text edit must apply");
    };
    let asset_batch = DirectEditBatchProposal {
        base: text_revision,
        capability_version: CURRENT_COMMAND_BEHAVIOR_VERSION,
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::AssetReference(
                    Some(first_asset),
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Figure),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::AssetReference,
            },
            requested: EditableSemanticValue::AssetReference(
                Some(second_asset),
            ),
            target: figure,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision, .. } =
        session.apply_direct_edit_batch(asset_batch)
    else {
        panic!("process fixture asset-reference edit must apply");
    };
    let current = session
        .accepted_revision()
        .expect("process fixture accepted revision");
    assert_eq!(current.notebook.assets.len(), 2);
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == first_asset));
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == second_asset));
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("process fixture first block must remain paragraph");
    };
    assert_eq!(spans[0].text, "process-private after");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[1].content
    else {
        panic!("process fixture second block must remain figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(second_asset));
    if mode == "populated-redo" {
        let HistoryTraversalOutcome::Traversed {
            revision: undone, ..
        } = session.traverse_history(revision, HistoryDirection::Undo)
        else {
            panic!("process fixture must create a Redo branch");
        };
        let current = session
            .accepted_revision()
            .expect("process fixture Undo revision");
        let BlockContent::Figure(current_figure) =
            &current.notebook.pages[0].flows[0].blocks[1].content
        else {
            panic!("process fixture Undo must retain figure");
        };
        assert_eq!(current_figure.id, figure);
        assert_eq!(current_figure.asset, Some(first_asset));
        assert_eq!(current.notebook.assets.len(), 2);
        assert_eq!(
            session.history_availability(),
            HistoryAvailabilityOutcome::Available(HistoryAvailability {
                can_redo: true,
                can_undo: true,
                revision: undone,
            }),
        );
    } else {
        assert_eq!(
            session.history_availability(),
            HistoryAvailabilityOutcome::Available(HistoryAvailability {
                can_redo: false,
                can_undo: true,
                revision,
            }),
        );
    }
    println!("{PROCESS_POPULATED_READY}");
    std::io::stdout().flush().expect("flush populated marker");
    let mut release = [0_u8; 1];
    let _ = std::io::stdin().read(&mut release);
}

fn spawn_process_fixture(mode: &str) -> std::process::Child {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg(PROCESS_TEST_NAME)
        .arg("--nocapture")
        .env(PROCESS_FIXTURE_MODE, mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn session process fixture")
}

fn await_populated_marker(
    child: &mut std::process::Child,
) -> BufReader<std::process::ChildStdout> {
    let stdout = child.stdout.take().expect("child stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        let read = reader.read_line(&mut line).expect("read child stdout");
        assert_ne!(read, 0, "child exited before populated marker");
        if line.contains(PROCESS_POPULATED_READY) {
            return reader;
        }
        line.clear();
    }
}

fn assert_fresh_process_empty() {
    let output = Command::new(
        std::env::current_exe().expect("current test executable"),
    )
    .arg("--exact")
    .arg(PROCESS_TEST_NAME)
    .arg("--nocapture")
    .env(PROCESS_FIXTURE_MODE, "fresh")
    .output()
    .expect("run fresh session process fixture");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("child stdout UTF-8");
    assert!(stdout.contains(PROCESS_FRESH_EMPTY));
}

#[test]
fn process_restart_drops_accepted_revision_and_history() {
    if let Some(mode) = std::env::var_os(PROCESS_FIXTURE_MODE) {
        run_process_fixture_child(&mode.to_string_lossy());
        return;
    }

    let mut orderly = spawn_process_fixture("populated");
    let mut orderly_stdout = await_populated_marker(&mut orderly);
    orderly
        .stdin
        .take()
        .expect("orderly child stdin")
        .write_all(b"x")
        .expect("release orderly child");
    let mut orderly_tail = String::new();
    orderly_stdout
        .read_to_string(&mut orderly_tail)
        .expect("drain orderly child stdout");
    assert!(orderly.wait().expect("wait orderly child").success());
    assert_fresh_process_empty();

    let mut forced = spawn_process_fixture("populated-redo");
    let mut forced_stdout = await_populated_marker(&mut forced);
    forced.kill().expect("force session child termination");
    let mut forced_tail = String::new();
    forced_stdout
        .read_to_string(&mut forced_tail)
        .expect("drain forced child stdout");
    let status = forced.wait().expect("wait forced child");
    assert!(!status.success());
    assert_fresh_process_empty();
}

#[test]
fn application_routes_bounded_inspection_through_owned_semantic_authority() {
    let identities = IdentityAllocator::new();
    let candidate = minimal_candidate(&identities);
    let notebook = candidate.id;
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept_candidate(candidate)
    else {
        panic!("minimal candidate must be accepted");
    };
    let notebook = mapping
        .iter()
        .find(|entry| entry.candidate == notebook)
        .expect("notebook identity must map")
        .accepted;

    let descriptor = SemanticIdentityDescriptor {
        kind: SemanticIdentityKind::Notebook,
        owner: None,
    };
    let snapshot = session.command_capability_snapshot();
    assert!(snapshot.admitted_applications.is_empty());
    assert!(snapshot.protocol_versions.is_empty());
    assert_eq!(snapshot.normalization_version, None);
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(revision, notebook)
    else {
        panic!("live owner must route exact command target material");
    };
    assert_eq!(material.descriptor, descriptor);
    assert_eq!(material.direct_edit_family, None);
    assert_eq!(material.editable_value, None);
    assert_eq!(material.revision, revision);
    assert_eq!(material.target, notebook);
    assert_eq!(
        session.inspect_identity(revision, notebook),
        IdentityInspectOutcome::Inspected {
            descriptor,
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.inspect_identity_ancestry_bounded(revision, notebook, 0),
        IdentityAncestryInspectOutcome::Inspected {
            completeness: IdentityAncestryCompleteness::Incomplete {
                remaining_identity: notebook,
            },
            entries: Vec::new(),
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.inspect_identity_ancestry_bounded(revision, notebook, 1),
        IdentityAncestryInspectOutcome::Inspected {
            completeness: IdentityAncestryCompleteness::Complete,
            entries: vec![IdentityAncestryEntry {
                descriptor,
                identity: notebook,
            }],
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.accepted_revision().map(|current| current.id),
        Some(revision),
    );
}

#[test]
fn application_routes_list_ordering_through_owned_authority() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_block, candidate_flow, candidate_list) =
        list_ordering_candidate(&identities);
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("list ordering candidate must be accepted");
    };
    let accepted = |candidate| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("list ordering identity must map")
            .accepted
    };
    let block = accepted(candidate_block);
    let flow = accepted(candidate_flow);
    let list = accepted(candidate_list);
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CURRENT_COMMAND_BEHAVIOR_VERSION);
    assert!(snapshot.family_capabilities.iter().any(|capability| {
        capability.family == SemanticCommandFamily::OrderingAndGrouping
            && capability.behavior_version == CommandBehaviorVersion(1)
    }));
    let before = EditableSemanticValue::ListOrdering(false);
    let requested = EditableSemanticValue::ListOrdering(true);
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(base, list)
    else {
        panic!("live owner must expose list ordering material");
    };
    assert_eq!(material.editable_value, Some(before.clone()));
    assert_eq!(material.descriptor, SemanticIdentityDescriptor {
        kind: SemanticIdentityKind::List,
        owner: Some(block),
    });
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::OrderingAndGrouping),
    );
    let batch = DirectEditBatchProposal {
        base,
        capability_version: snapshot.behavior_version,
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(before),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::List),
                    expected_owner: IdentityOwnerExpectation::Direct(block),
                },
                requested_family: SemanticCommandFamily::OrderingAndGrouping,
            },
            requested,
            target: list,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision: applied, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("live owner must apply list ordering edit");
    };
    let current = session.accepted_revision().expect("ordered list revision");
    let BlockContent::List(changed) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("live list block must remain a list");
    };
    assert_eq!(changed.id, list);
    assert!(changed.ordered);
    assert_eq!(
        session.inspect_identity(base, flow),
        IdentityInspectOutcome::StaleBase { current: applied },
    );
    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("live list ordering edit must Undo");
    };
    let current = session.accepted_revision().expect("list ordering Undo");
    let BlockContent::List(restored) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo live block must remain a list");
    };
    assert_eq!(restored.id, list);
    assert!(!restored.ordered);
}

#[test]
fn application_routes_block_style_through_owned_authority() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_block, candidate_flow, candidate_style) =
        block_style_candidate(&identities);
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("block style candidate must be accepted");
    };
    let accepted = |candidate| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("block style identity must map")
            .accepted
    };
    let block = accepted(candidate_block);
    let flow = accepted(candidate_flow);
    let style = accepted(candidate_style);
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CURRENT_COMMAND_BEHAVIOR_VERSION);
    assert!(snapshot.family_capabilities.iter().any(|capability| {
        capability.family == SemanticCommandFamily::StyleRole
            && capability.behavior_version == CommandBehaviorVersion(2)
    }));
    let before = EditableSemanticValue::StyleReference(None);
    let requested = EditableSemanticValue::StyleReference(Some(style));
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(base, block)
    else {
        panic!("live owner must expose block style material");
    };
    assert_eq!(material.editable_value, Some(before.clone()));
    assert_eq!(material.descriptor, SemanticIdentityDescriptor {
        kind: SemanticIdentityKind::Block(SemanticBlockKind::Paragraph),
        owner: Some(flow),
    });
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::StyleRole),
    );
    assert_eq!(
        session.simulate_direct_edit(base, block, requested.clone()),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::StyleRole,
            requested: requested.clone(),
            revision: base,
            target: block,
        },
    );
    let batch = DirectEditBatchProposal {
        base,
        capability_version: snapshot.behavior_version,
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(before),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Block(
                        SemanticBlockKind::Paragraph,
                    )),
                    expected_owner: IdentityOwnerExpectation::Direct(flow),
                },
                requested_family: SemanticCommandFamily::StyleRole,
            },
            requested,
            target: block,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision: applied, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("live owner must apply block style edit");
    };
    let current = session.accepted_revision().expect("styled revision");
    let changed = &current.notebook.pages[0].flows[0].blocks[0];
    assert_eq!(changed.id, block);
    assert_eq!(changed.style, Some(style));

    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("live block style edit must Undo");
    };
    let current = session.accepted_revision().expect("style Undo revision");
    let restored = &current.notebook.pages[0].flows[0].blocks[0];
    assert_eq!(restored.id, block);
    assert_eq!(restored.style, None);
}

#[test]
fn application_routes_page_profile_reference_through_owned_authority() {
    let identities = IdentityAllocator::new();
    let (
        candidate,
        candidate_notebook,
        candidate_page,
        candidate_first,
        second,
    ) = page_profile_reference_candidate(&identities);
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("page profile reference candidate must be accepted");
    };
    let accepted = |candidate| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("page profile reference identity must map")
            .accepted
    };
    let notebook = accepted(candidate_notebook);
    let page = accepted(candidate_page);
    let first = accepted(candidate_first);
    let second = accepted(second);
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CURRENT_COMMAND_BEHAVIOR_VERSION);
    assert!(snapshot.family_capabilities.iter().any(|capability| {
        capability.family == SemanticCommandFamily::DocumentConstraint
            && capability.behavior_version == CommandBehaviorVersion(3)
    }));
    let before = EditableSemanticValue::PageProfileReference(first);
    let requested = EditableSemanticValue::PageProfileReference(second);
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(base, page)
    else {
        panic!("live owner must expose page profile reference material");
    };
    assert_eq!(material.descriptor, SemanticIdentityDescriptor {
        kind: SemanticIdentityKind::Page,
        owner: Some(notebook),
    });
    assert_eq!(material.editable_value, Some(before.clone()));
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::DocumentConstraint),
    );
    assert_eq!(
        session.simulate_direct_edit(base, page, requested.clone()),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::DocumentConstraint,
            requested: requested.clone(),
            revision: base,
            target: page,
        },
    );
    let batch = DirectEditBatchProposal {
        base,
        capability_version: snapshot.behavior_version,
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(before),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Page),
                    expected_owner: IdentityOwnerExpectation::Direct(notebook),
                },
                requested_family: SemanticCommandFamily::DocumentConstraint,
            },
            requested,
            target: page,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("live owner must apply page profile reference edit");
    };
    let current = session.accepted_revision().expect("retargeted revision");
    assert_eq!(current.notebook.pages[0].id, page);
    assert_eq!(current.notebook.pages[0].page_profile, second);

    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(revision, HistoryDirection::Undo)
    else {
        panic!("live page profile reference edit must Undo");
    };
    let current = session.accepted_revision().expect("retarget Undo revision");
    assert_eq!(current.notebook.pages[0].id, page);
    assert_eq!(current.notebook.pages[0].page_profile, first);
}

#[test]
fn application_routes_global_constraint_through_owned_authority() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_constraint, candidate_notebook) =
        global_constraint_candidate(&identities);
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("global constraint candidate must be accepted");
    };
    let accepted = |candidate| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("global constraint identity must map")
            .accepted
    };
    let constraint = accepted(candidate_constraint);
    let notebook = accepted(candidate_notebook);
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CURRENT_COMMAND_BEHAVIOR_VERSION);
    assert!(snapshot.family_capabilities.iter().any(|capability| {
        capability.family == SemanticCommandFamily::DocumentConstraint
            && capability.behavior_version == CommandBehaviorVersion(3)
    }));
    let current_value = EditableSemanticValue::ConstraintKind(
        ConstraintKind::Paper,
    );
    let requested = EditableSemanticValue::ConstraintKind(
        ConstraintKind::Style,
    );
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(base, constraint)
    else {
        panic!("live owner must expose constraint material");
    };
    assert_eq!(material.editable_value, Some(current_value.clone()));
    assert_eq!(material.descriptor, SemanticIdentityDescriptor {
        kind: SemanticIdentityKind::Constraint,
        owner: Some(notebook),
    });
    assert_eq!(
        session.simulate_direct_edit(base, constraint, requested.clone()),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::DocumentConstraint,
            requested: requested.clone(),
            revision: base,
            target: constraint,
        },
    );
    let batch = DirectEditBatchProposal {
        base,
        capability_version: snapshot.behavior_version,
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(current_value),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Constraint),
                    expected_owner: IdentityOwnerExpectation::Direct(notebook),
                },
                requested_family: SemanticCommandFamily::DocumentConstraint,
            },
            requested,
            target: constraint,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision: applied, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("live owner must apply constraint kind edit");
    };
    let current = session
        .accepted_revision()
        .expect("global constraint revision");
    let changed = current
        .notebook
        .constraints
        .iter()
        .find(|value| value.id == constraint)
        .expect("global constraint value");
    assert_eq!(changed.kind, ConstraintKind::Style);
    assert_eq!(changed.target, notebook);

    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("live global constraint batch must Undo");
    };
    let current = session
        .accepted_revision()
        .expect("global constraint Undo");
    let restored = current
        .notebook
        .constraints
        .iter()
        .find(|value| value.id == constraint)
        .expect("restored global constraint");
    assert_eq!(restored.kind, ConstraintKind::Paper);
    assert_eq!(restored.target, notebook);
}

#[test]
fn application_routes_provenance_batch_through_owned_authority() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_claim, candidate_provenance) =
        provenance_claim_candidate(&identities);
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("provenance candidate must be accepted");
    };
    let accepted = |candidate| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("candidate identity must map")
            .accepted
    };
    let claim = accepted(candidate_claim);
    let provenance = accepted(candidate_provenance);
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CURRENT_COMMAND_BEHAVIOR_VERSION);
    assert!(snapshot.family_capabilities.iter().any(|capability| {
        capability.family == SemanticCommandFamily::Provenance
    }));
    let current_value = EditableSemanticValue::Provenance {
        kind: ProvenanceKind::Supplied,
        reference: Some(String::from("source:old")),
    };
    let requested = EditableSemanticValue::Provenance {
        kind: ProvenanceKind::Cited,
        reference: Some(String::from("doi:10.1000/example")),
    };
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(base, provenance)
    else {
        panic!("live owner must expose provenance material");
    };
    assert_eq!(material.editable_value, Some(current_value.clone()));
    assert_eq!(
        material.direct_edit_family,
        Some(SemanticCommandFamily::Provenance),
    );
    assert_eq!(
        session.simulate_direct_edit(base, provenance, requested.clone()),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::Provenance,
            requested: requested.clone(),
            revision: base,
            target: provenance,
        },
    );
    let DirectEditChangePreviewOutcome::Predicted { changes, .. } =
        session.preview_direct_edit_changes(base, provenance, requested.clone())
    else {
        panic!("live owner must preview provenance replacement");
    };
    assert_eq!(changes, vec![DirectEditSemanticChange {
        after: requested.clone(),
        before: current_value.clone(),
        family: SemanticCommandFamily::Provenance,
        target: provenance,
    }]);

    let batch = DirectEditBatchProposal {
        base,
        capability_version: snapshot.behavior_version,
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(current_value),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Provenance),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::Provenance,
            },
            requested,
            target: provenance,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision: applied, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("live owner must apply provenance replacement");
    };
    let current = session
        .accepted_revision()
        .expect("provenance revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("live claim must remain paragraph");
    };
    assert_eq!(spans[0].id, claim);
    assert_eq!(spans[0].text, "Energy is conserved.");
    assert_eq!(spans[0].provenance, Some(provenance));
    let changed = current
        .notebook
        .provenance
        .iter()
        .find(|record| record.id == provenance)
        .expect("live provenance record");
    assert_eq!(changed.kind, ProvenanceKind::Cited);
    assert_eq!(changed.reference.as_deref(), Some("doi:10.1000/example"));

    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("live provenance batch must Undo");
    };
    let current = session
        .accepted_revision()
        .expect("provenance Undo revision");
    let restored = current
        .notebook
        .provenance
        .iter()
        .find(|record| record.id == provenance)
        .expect("restored provenance record");
    assert_eq!(restored.kind, ProvenanceKind::Supplied);
    assert_eq!(restored.reference.as_deref(), Some("source:old"));
}

#[test]
fn application_routes_multifamily_span_references_through_owned_authority() {
    let identities = IdentityAllocator::new();
    let (mut candidate, candidate_claim, candidate_provenance) =
        provenance_claim_candidate(&identities);
    let candidate_block = candidate.pages[0].flows[0].blocks[0].id;
    let candidate_style = identities.allocate_candidate().expect("style id");
    candidate.styles.push(Style {
        id: candidate_style,
        name: String::from("claim-emphasis"),
    });
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("multi-family live candidate must be accepted");
    };
    let accepted = |candidate| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("multi-family live identity must map")
            .accepted
    };
    let block = accepted(candidate_block);
    let claim = accepted(candidate_claim);
    let provenance = accepted(candidate_provenance);
    let style = accepted(candidate_style);
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CURRENT_COMMAND_BEHAVIOR_VERSION);
    assert!(snapshot.family_capabilities.iter().any(|capability| {
        capability.family == SemanticCommandFamily::Provenance
            && capability.behavior_version == CommandBehaviorVersion(2)
    }));
    assert!(snapshot.family_capabilities.iter().any(|capability| {
        capability.family == SemanticCommandFamily::StyleRole
            && capability.behavior_version == CommandBehaviorVersion(2)
    }));

    let CommandTargetMaterialOutcome::Prepared { material: style_material } =
        session.command_target_material_for_family(
            base,
            claim,
            SemanticCommandFamily::StyleRole,
        )
    else {
        panic!("live span style material must be prepared");
    };
    assert_eq!(
        style_material.editable_value,
        Some(EditableSemanticValue::StyleReference(None)),
    );
    let CommandTargetMaterialOutcome::Prepared {
        material: provenance_material,
    } = session.command_target_material_for_family(
        base,
        claim,
        SemanticCommandFamily::Provenance,
    ) else {
        panic!("live span provenance material must be prepared");
    };
    assert_eq!(
        provenance_material.editable_value,
        Some(EditableSemanticValue::ProvenanceReference(Some(provenance))),
    );

    let batch = DirectEditBatchProposal {
        base,
        capability_version: snapshot.behavior_version,
        commands: vec![
            DirectEditBatchCommand {
                dependencies: vec![],
                id: 1_u32,
                preconditions: CommandTargetPreconditions {
                    expected_value: Some(EditableSemanticValue::StyleReference(
                        None,
                    )),
                    identity: IdentityPrecondition {
                        expected_kind: Some(SemanticIdentityKind::InlineSpan),
                        expected_owner: IdentityOwnerExpectation::Direct(block),
                    },
                    requested_family: SemanticCommandFamily::StyleRole,
                },
                requested: EditableSemanticValue::StyleReference(Some(style)),
                target: claim,
            },
            DirectEditBatchCommand {
                dependencies: vec![],
                id: 2_u32,
                preconditions: CommandTargetPreconditions {
                    expected_value: Some(
                        EditableSemanticValue::ProvenanceReference(Some(
                            provenance,
                        )),
                    ),
                    identity: IdentityPrecondition {
                        expected_kind: Some(SemanticIdentityKind::InlineSpan),
                        expected_owner: IdentityOwnerExpectation::Direct(block),
                    },
                    requested_family: SemanticCommandFamily::Provenance,
                },
                requested: EditableSemanticValue::ProvenanceReference(None),
                target: claim,
            },
        ],
    };
    let DirectEditBatchApplyOutcome::Applied {
        changes,
        revision: applied,
        ..
    } = session.apply_direct_edit_batch(batch)
    else {
        panic!("live multi-family span batch must apply");
    };
    assert_eq!(changes.len(), 2);
    let current = session
        .accepted_revision()
        .expect("live multi-family span revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("live multi-family claim must remain paragraph");
    };
    assert_eq!(spans[0].id, claim);
    assert_eq!(spans[0].style, Some(style));
    assert_eq!(spans[0].provenance, None);

    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("live multi-family span batch must Undo");
    };
    let current = session
        .accepted_revision()
        .expect("live multi-family span Undo");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo live multi-family claim must remain paragraph");
    };
    assert_eq!(spans[0].style, None);
    assert_eq!(spans[0].provenance, Some(provenance));
}

#[test]
fn application_routes_asset_reference_batch_through_owned_authority() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_figure, candidate_first, candidate_second) =
        asset_figure_candidate(&identities);
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("asset figure candidate must be accepted");
    };
    let accepted = |candidate| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("candidate identity must map")
            .accepted
    };
    let figure = accepted(candidate_figure);
    let first_asset = accepted(candidate_first);
    let second_asset = accepted(candidate_second);
    let snapshot = session.command_capability_snapshot();
    assert_eq!(snapshot.behavior_version, CURRENT_COMMAND_BEHAVIOR_VERSION);
    assert!(snapshot.family_capabilities.iter().any(|capability| {
        capability.family == SemanticCommandFamily::AssetReference
    }));
    let CommandTargetMaterialOutcome::Prepared { material } =
        session.command_target_material(base, figure)
    else {
        panic!("live owner must expose figure command material");
    };
    assert_eq!(
        material.editable_value,
        Some(EditableSemanticValue::AssetReference(Some(first_asset))),
    );
    assert_eq!(
        session.check_editable_value_precondition(
            base,
            figure,
            EditableSemanticValue::AssetReference(Some(second_asset)),
        ),
        EditableValuePreconditionOutcome::ValueMismatch {
            actual: EditableSemanticValue::AssetReference(Some(first_asset)),
            expected: EditableSemanticValue::AssetReference(Some(second_asset)),
            revision: base,
            target: figure,
        },
    );
    assert_eq!(
        session.simulate_direct_edit(
            base,
            figure,
            EditableSemanticValue::AssetReference(Some(figure)),
        ),
        DirectEditSimulationOutcome::InvalidAssetReference {
            actual: Some(SemanticIdentityKind::Figure),
            reference: figure,
            revision: base,
            target: figure,
        },
    );
    assert_eq!(
        session.accepted_revision().map(|current| current.id),
        Some(base),
    );
    assert_eq!(
        session.simulate_direct_edit(
            base,
            figure,
            EditableSemanticValue::AssetReference(Some(second_asset)),
        ),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::AssetReference,
            requested: EditableSemanticValue::AssetReference(
                Some(second_asset),
            ),
            revision: base,
            target: figure,
        },
    );

    let batch = DirectEditBatchProposal {
        base,
        capability_version: snapshot.behavior_version,
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::AssetReference(
                    Some(first_asset),
                )),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::Figure),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::AssetReference,
            },
            requested: EditableSemanticValue::AssetReference(
                Some(second_asset),
            ),
            target: figure,
        }],
    };
    let DirectEditBatchApplyOutcome::Applied { revision: applied, .. } =
        session.apply_direct_edit_batch(batch)
    else {
        panic!("live owner must apply admitted asset reference");
    };
    let current = session
        .accepted_revision()
        .expect("asset reference revision");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("live fixture must remain a figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(second_asset));
    assert_eq!(current.notebook.assets.len(), 2);
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == first_asset));
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == second_asset));

    let HistoryTraversalOutcome::Traversed {
        revision: undone, ..
    } = session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("live asset-reference batch must Undo");
    };
    let current = session.accepted_revision().expect("asset reference Undo");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("live Undo fixture must remain a figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(first_asset));
    assert_eq!(current.notebook.assets.len(), 2);
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == first_asset));
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == second_asset));

    let HistoryTraversalOutcome::Traversed { .. } =
        session.traverse_history(undone, HistoryDirection::Redo)
    else {
        panic!("live asset-reference batch must Redo");
    };
    let current = session.accepted_revision().expect("asset reference Redo");
    let BlockContent::Figure(current_figure) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("live Redo fixture must remain a figure");
    };
    assert_eq!(current_figure.id, figure);
    assert_eq!(current_figure.asset, Some(second_asset));
    assert_eq!(current.notebook.assets.len(), 2);
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == first_asset));
    assert!(current
        .notebook
        .assets
        .iter()
        .any(|asset| asset.id == second_asset));
}

#[test]
fn application_routes_all_direct_edit_no_effects_through_owned_authority() {
    let identities = IdentityAllocator::new();
    let candidate = minimal_candidate(&identities);
    let candidate_notebook = candidate.id;
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept_candidate(candidate)
    else {
        panic!("minimal candidate must be accepted");
    };
    let notebook = mapping
        .iter()
        .find(|entry| entry.candidate == candidate_notebook)
        .expect("notebook identity must map")
        .accepted;
    let before = session
        .accepted_revision()
        .expect("accepted revision")
        .clone();

    assert_eq!(
        session.replace_formula(
            revision,
            notebook,
            FormulaMode::Display,
            String::from("x"),
        ),
        FormulaEditOutcome::TargetNotFormula {
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.replace_page_profile(
            revision,
            notebook,
            physical_page_profile(),
        ),
        PageProfileEditOutcome::TargetNotPageProfile {
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.replace_table_cell_span(
            revision,
            notebook,
            TableCellSpan::SINGLE,
        ),
        TableCellSpanEditOutcome::TargetNotTableCell {
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.replace_table_row_role(
            revision,
            notebook,
            TableRowRole::Header,
        ),
        TableRowRoleEditOutcome::TargetNotTableRow {
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.replace_text(revision, notebook, String::from("text")),
        TextEditOutcome::TargetNotText {
            revision,
            target: notebook,
        },
    );
    assert_eq!(session.accepted_revision(), Some(&before));
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: false,
            revision,
        }),
    );
}

#[test]
fn application_routes_direct_text_edit_into_owned_history() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_span) =
        editable_text_candidate(&identities, "before direct edit");
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept_candidate(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = mapping
        .iter()
        .find(|entry| entry.candidate == candidate_span)
        .expect("span identity must map")
        .accepted;

    let TextEditOutcome::Applied {
        base: applied_base,
        revision: applied,
        target,
    } = session.replace_text(
        base,
        span,
        String::from("after direct edit"),
    )
    else {
        panic!("live-owner direct text edit must apply");
    };
    assert_eq!(applied_base, base);
    assert_eq!(target, span);
    assert_ne!(applied, base);
    let current = session.accepted_revision().expect("applied revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("fixture must remain a paragraph");
    };
    assert_eq!(spans[0].id, span);
    assert_eq!(spans[0].text, "after direct edit");
    assert_eq!(
        session.command_target_material(base, span),
        CommandTargetMaterialOutcome::StaleBase { current: applied },
    );
    assert_eq!(
        session.simulate_direct_edit(
            base,
            span,
            EditableSemanticValue::Text(String::from("later")),
        ),
        DirectEditSimulationOutcome::StaleBase { current: applied },
    );
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: true,
            revision: applied,
        }),
    );

    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("live-owner direct text edit must Undo");
    };
    let current = session.accepted_revision().expect("Undo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("Undo fixture must remain a paragraph");
    };
    assert_eq!(spans[0].id, span);
    assert_eq!(spans[0].text, "before direct edit");
    assert_ne!(undone, applied);
}

#[test]
fn application_routes_history_traversal_through_owned_semantic_authority() {
    let identities = IdentityAllocator::new();
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { revision: first, .. } =
        session.accept_candidate(minimal_candidate(&identities))
    else {
        panic!("first candidate must be accepted");
    };
    let AcceptanceOutcome::Accepted { revision: second, .. } =
        session.accept_candidate(minimal_candidate(&identities))
    else {
        panic!("second candidate must be accepted");
    };

    let atrament_semantic_notebook_port::HistoryTraversalOutcome::Traversed {
        base,
        direction,
        revision: undone,
    } = session.traverse_history(
        second,
        atrament_semantic_notebook_port::HistoryDirection::Undo,
    ) else {
        panic!("live owner must route Undo");
    };
    assert_eq!(base, second);
    assert_eq!(
        direction,
        atrament_semantic_notebook_port::HistoryDirection::Undo,
    );
    assert_ne!(undone, first);
    assert_eq!(
        session.accepted_revision().map(|revision| revision.id),
        Some(undone),
    );
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: true,
            can_undo: false,
            revision: undone,
        }),
    );

    let atrament_semantic_notebook_port::HistoryTraversalOutcome::Traversed {
        revision: redone,
        ..
    } = session.traverse_history(
        undone,
        atrament_semantic_notebook_port::HistoryDirection::Redo,
    ) else {
        panic!("live owner must route Redo");
    };
    assert_ne!(redone, second);
    assert_eq!(
        session.accepted_revision().map(|revision| revision.id),
        Some(redone),
    );
}

#[test]
fn application_reviews_editable_text_through_owned_semantic_authority() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_span) =
        editable_text_candidate(&identities, "before");
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept_candidate(candidate)
    else {
        panic!("text candidate must be accepted");
    };
    let span = mapping
        .iter()
        .find(|entry| entry.candidate == candidate_span)
        .expect("span identity must map")
        .accepted;
    let before_revision =
        session.accepted_revision().expect("accepted revision").clone();
    let preconditions = CommandTargetPreconditions {
        expected_value: Some(EditableSemanticValue::Text(String::from(
            "before",
        ))),
        identity: IdentityPrecondition {
            expected_kind: Some(SemanticIdentityKind::InlineSpan),
            expected_owner: IdentityOwnerExpectation::Any,
        },
        requested_family: SemanticCommandFamily::TextContent,
    };
    assert!(matches!(
        session.check_command_target_preconditions(
            revision,
            span,
            preconditions.clone(),
        ),
        CommandTargetPreconditionOutcome::Satisfied { .. }
    ));
    let requested = EditableSemanticValue::Text(String::from("after"));
    assert_eq!(
        session.simulate_direct_edit(revision, span, requested.clone()),
        DirectEditSimulationOutcome::Applicable {
            family: SemanticCommandFamily::TextContent,
            requested: requested.clone(),
            revision,
            target: span,
        },
    );
    let DirectEditChangePreviewOutcome::Predicted {
        changes,
        effect,
        impact_seeds,
        revision: preview_revision,
    } = session.preview_direct_edit_changes(revision, span, requested.clone())
    else {
        panic!("editable text preview must predict a change");
    };
    assert_eq!(preview_revision, revision);
    assert_eq!(effect, DirectEditEffectClass::Mutation);
    assert_eq!(changes, vec![DirectEditSemanticChange {
        after: requested.clone(),
        before: EditableSemanticValue::Text(String::from("before")),
        family: SemanticCommandFamily::TextContent,
        target: span,
    }]);
    assert!(!impact_seeds.is_empty());
    assert_eq!(
        session.simulate_direct_edit(
            revision,
            span,
            EditableSemanticValue::Text(String::from("before")),
        ),
        DirectEditSimulationOutcome::NoOp {
            family: SemanticCommandFamily::TextContent,
            revision,
            target: span,
        },
    );
    assert_eq!(
        session.simulate_direct_edit_proposal(DirectEditProposal {
            capability_version: CURRENT_COMMAND_BEHAVIOR_VERSION,
            preconditions,
            requested: requested.clone(),
            revision,
            target: span,
        }),
        DirectEditProposalOutcome::Simulated {
            outcome: DirectEditSimulationOutcome::Applicable {
                family: SemanticCommandFamily::TextContent,
                requested,
                revision,
                target: span,
            },
        },
    );
    assert_eq!(session.accepted_revision(), Some(&before_revision));
}

#[test]
fn application_routes_local_command_review_through_owned_semantic_authority() {
    let identities = IdentityAllocator::new();
    let candidate = minimal_candidate(&identities);
    let candidate_notebook = candidate.id;
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept_candidate(candidate)
    else {
        panic!("minimal candidate must be accepted");
    };
    let notebook = mapping
        .iter()
        .find(|entry| entry.candidate == candidate_notebook)
        .expect("notebook identity must map")
        .accepted;
    let before = session
        .accepted_revision()
        .expect("accepted revision")
        .clone();
    let descriptor = SemanticIdentityDescriptor {
        kind: SemanticIdentityKind::Notebook,
        owner: None,
    };

    let snapshot = session.command_capability_snapshot();
    assert_eq!(
        session.check_command_capability_compatibility(
            snapshot.behavior_version,
        ),
        CommandCapabilityCompatibilityOutcome::Compatible {
            snapshot,
        },
    );
    assert_eq!(
        session.check_command_capability_compatibility(
            CommandBehaviorVersion(4),
        ),
        CommandCapabilityCompatibilityOutcome::Mismatch {
            current: CURRENT_COMMAND_BEHAVIOR_VERSION,
            expected: CommandBehaviorVersion(4),
        },
    );
    assert_eq!(
        session.inspect_identity_kind(revision, notebook),
        IdentityKindInspectOutcome::Inspected {
            kind: SemanticIdentityKind::Notebook,
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.check_identity_precondition(
            revision,
            notebook,
            IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::Notebook),
                expected_owner: IdentityOwnerExpectation::Root,
            },
        ),
        IdentityPreconditionOutcome::Satisfied {
            descriptor,
            revision,
            target: notebook,
        },
    );
    assert_eq!(
        session.check_command_family_admission(
            revision,
            notebook,
            SemanticCommandFamily::TextContent,
        ),
        CommandFamilyAdmissionOutcome::FamilyNotExecutable {
            available: None,
            requested: SemanticCommandFamily::TextContent,
            revision,
            target: notebook,
        },
    );
    let text = EditableSemanticValue::Text(String::from("notebook root text"));
    assert_eq!(
        session.check_editable_value_precondition(
            revision,
            notebook,
            text.clone(),
        ),
        EditableValuePreconditionOutcome::TargetNotEditableValue {
            kind: SemanticIdentityKind::Notebook,
            revision,
            target: notebook,
        },
    );
    let preconditions = CommandTargetPreconditions {
        expected_value: None,
        identity: IdentityPrecondition {
            expected_kind: Some(SemanticIdentityKind::Notebook),
            expected_owner: IdentityOwnerExpectation::Root,
        },
        requested_family: SemanticCommandFamily::TextContent,
    };
    let family_rejection =
        CommandTargetPreconditionOutcome::FamilyNotExecutable {
        available: None,
        requested: SemanticCommandFamily::TextContent,
        revision,
        target: notebook,
    };
    assert_eq!(
        session.check_command_target_preconditions(
            revision,
            notebook,
            preconditions.clone(),
        ),
        family_rejection.clone(),
    );
    let simulation = DirectEditSimulationOutcome::TargetNotEditableValue {
        kind: SemanticIdentityKind::Notebook,
        revision,
        target: notebook,
    };
    assert_eq!(
        session.simulate_direct_edit(revision, notebook, text.clone()),
        simulation.clone(),
    );
    assert_eq!(
        session.preview_direct_edit_changes(revision, notebook, text.clone()),
        DirectEditChangePreviewOutcome::Rejected {
            outcome: Box::new(simulation),
        },
    );
    assert_eq!(
        session.simulate_direct_edit_proposal(DirectEditProposal {
            capability_version: snapshot.behavior_version,
            preconditions,
            requested: text,
            revision,
            target: notebook,
        }),
        DirectEditProposalOutcome::PreconditionRejected {
            outcome: family_rejection,
        },
    );
    assert_eq!(session.accepted_revision(), Some(&before));
}

#[test]
fn application_routes_nonempty_selection_analysis_read_only() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_span) =
        editable_text_candidate(&identities, "selection base");
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept_candidate(candidate)
    else {
        panic!("selection candidate must be accepted");
    };
    let span = mapping
        .iter()
        .find(|entry| entry.candidate == candidate_span)
        .expect("selection span identity must map")
        .accepted;
    let command = |
        id: u32,
        dependencies: Vec<u32>,
        expected: &str,
        requested: &str,
    | DirectEditBatchCommand {
        dependencies,
        id,
        preconditions: CommandTargetPreconditions {
            expected_value: Some(EditableSemanticValue::Text(
                expected.to_owned(),
            )),
            identity: IdentityPrecondition {
                expected_kind: Some(SemanticIdentityKind::InlineSpan),
                expected_owner: IdentityOwnerExpectation::Any,
            },
            requested_family: SemanticCommandFamily::TextContent,
        },
        requested: EditableSemanticValue::Text(requested.to_owned()),
        target: span,
    };
    let batch = DirectEditBatchProposal {
        base: revision,
        capability_version: CURRENT_COMMAND_BEHAVIOR_VERSION,
        commands: vec![
            command(1, vec![], "selection base", "one"),
            command(2, vec![1], "one", "two"),
            command(3, vec![2], "two", "three"),
        ],
    };
    let selected = BTreeSet::from([3_u32]);
    let before = session.accepted_revision().expect("selection base").clone();

    let DirectEditBatchSelectionRequirementsOutcome::Requirements {
        missing,
        revision: requirement_revision,
    } = session.direct_edit_batch_selection_requirements(&batch, &selected)
    else {
        panic!("live owner must route nonempty selection requirements");
    };
    assert_eq!(requirement_revision, revision);
    assert_eq!(missing.len(), 2);
    assert_eq!(missing[0].command, 2);
    assert_eq!(missing[0].dependency, 1);
    assert_eq!(missing[1].command, 3);
    assert_eq!(missing[1].dependency, 2);
    assert!(matches!(
        session.direct_edit_batch_selection_requirements_bounded(
            &batch,
            &selected,
            1,
        ),
        DirectEditBatchSelectionBoundedOutcome::RequirementCountExceeded {
            actual: 2,
            limit: 1,
        }
    ));
    let DirectEditBatchSelectionBoundedOutcome::Requirements {
        missing: bounded_missing,
        revision: bounded_revision,
    } = session.direct_edit_batch_selection_requirements_bounded(
        &batch,
        &selected,
        2,
    ) else {
        panic!("exact selection edge bound must admit the report");
    };
    assert_eq!(bounded_revision, revision);
    assert_eq!(bounded_missing, missing);
    let DirectEditBatchSelectionSummaryOutcome::Summarized {
        revision: summary_revision,
        summary,
    } = session.direct_edit_batch_selection_summary(&batch, &selected)
    else {
        panic!("live owner must route nonempty selection summary");
    };
    assert_eq!(summary_revision, revision);
    assert_eq!(summary.selected_commands, 1);
    assert_eq!(summary.required_commands, 3);
    assert_eq!(summary.missing_dependency_edges, 2);
    assert_eq!(session.accepted_revision(), Some(&before));
}

#[test]
fn application_routes_atomic_batch_apply_through_owned_semantic_authority() {
    let identities = IdentityAllocator::new();
    let (candidate, candidate_span) =
        editable_text_candidate(&identities, "bounded before");
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept_candidate(candidate)
    else {
        panic!("editable candidate must be accepted");
    };
    let span = mapping
        .iter()
        .find(|entry| entry.candidate == candidate_span)
        .expect("bounded span identity must map")
        .accepted;
    let empty = DirectEditBatchProposal::<u32> {
        base: revision,
        capability_version: CURRENT_COMMAND_BEHAVIOR_VERSION,
        commands: Vec::new(),
    };

    let zero_limits = CommandGraphLimits {
        commands: 0,
        dependency_edges: 0,
    };
    let DirectEditBatchGraphSizeOutcome::Sized {
        revision: sized_revision,
        size,
    } = session.direct_edit_batch_graph_size(&empty)
    else {
        panic!("live owner must route exact graph sizing");
    };
    assert_eq!(sized_revision, revision);
    assert_eq!(size.commands, 0);
    assert_eq!(size.dependency_edges, 0);
    assert_eq!(
        session.direct_edit_batch_graph_limits(&empty, zero_limits),
        DirectEditBatchGraphLimitsOutcome::Admitted { revision, size },
    );
    let selected = BTreeSet::new();
    assert_eq!(
        session.direct_edit_batch_selection_requirements(&empty, &selected),
        DirectEditBatchSelectionRequirementsOutcome::Requirements {
            missing: Vec::new(),
            revision,
        },
    );
    assert_eq!(
        session.direct_edit_batch_selection_requirements_bounded(
            &empty,
            &selected,
            0,
        ),
        DirectEditBatchSelectionBoundedOutcome::Requirements {
            missing: Vec::new(),
            revision,
        },
    );
    let DirectEditBatchSelectionSummaryOutcome::Summarized {
        revision: summary_revision,
        summary,
    } = session.direct_edit_batch_selection_summary(&empty, &selected)
    else {
        panic!("live owner must route selection summary");
    };
    assert_eq!(summary_revision, revision);
    assert_eq!(summary.selected_commands, 0);
    assert_eq!(summary.required_commands, 0);
    assert_eq!(summary.missing_dependency_edges, 0);
    assert_eq!(
        session.simulate_direct_edit_batch_bounded(
            empty.clone(),
            zero_limits,
        ),
        DirectEditBatchSimulationOutcome::Predicted {
            changes: Vec::new(),
            commands: Vec::new(),
            effect: DirectEditEffectClass::NoOp,
            impact_seeds: Vec::new(),
            revision,
        },
    );
    assert_eq!(
        session.apply_direct_edit_batch_bounded(empty.clone(), zero_limits),
        DirectEditBatchApplyOutcome::NoOp {
            commands: Vec::new(),
            revision,
        },
    );

    let bounded = DirectEditBatchProposal {
        base: revision,
        capability_version: CURRENT_COMMAND_BEHAVIOR_VERSION,
        commands: vec![DirectEditBatchCommand {
            dependencies: vec![],
            id: 1_u32,
            preconditions: CommandTargetPreconditions {
                expected_value: Some(EditableSemanticValue::Text(String::from(
                    "bounded before",
                ))),
                identity: IdentityPrecondition {
                    expected_kind: Some(SemanticIdentityKind::InlineSpan),
                    expected_owner: IdentityOwnerExpectation::Any,
                },
                requested_family: SemanticCommandFamily::TextContent,
            },
            requested: EditableSemanticValue::Text(String::from(
                "bounded after",
            )),
            target: span,
        }],
    };
    let before_rejection =
        session.accepted_revision().expect("bounded base").clone();
    assert!(matches!(
        session.direct_edit_batch_graph_limits(&bounded, zero_limits),
        DirectEditBatchGraphLimitsOutcome::Rejected { .. }
    ));
    assert!(matches!(
        session.simulate_direct_edit_batch_bounded(
            bounded.clone(),
            zero_limits,
        ),
        DirectEditBatchSimulationOutcome::ResourceRejected { .. }
    ));
    assert!(matches!(
        session.apply_direct_edit_batch_bounded(bounded.clone(), zero_limits),
        DirectEditBatchApplyOutcome::ResourceRejected { .. }
    ));
    assert_eq!(session.accepted_revision(), Some(&before_rejection));

    assert_eq!(
        session.simulate_direct_edit_batch(empty.clone()),
        DirectEditBatchSimulationOutcome::Predicted {
            changes: Vec::new(),
            commands: Vec::new(),
            effect: DirectEditEffectClass::NoOp,
            impact_seeds: Vec::new(),
            revision,
        },
    );
    assert_eq!(
        session.apply_direct_edit_batch(empty.clone()),
        DirectEditBatchApplyOutcome::NoOp {
            commands: Vec::new(),
            revision,
        },
    );
    assert_eq!(
        session.accepted_revision().map(|current| current.id),
        Some(revision),
    );
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: false,
            revision,
        }),
    );

    let DirectEditBatchApplyOutcome::Applied {
        base: applied_base,
        changes,
        revision: applied,
        ..
    } = session.apply_direct_edit_batch(bounded)
    else {
        panic!("live-owner text batch must apply without caller bound");
    };
    assert_eq!(applied_base, revision);
    assert_eq!(changes.len(), 1);
    let current = session.accepted_revision().expect("batch revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("batch fixture must remain paragraph");
    };
    assert_eq!(spans[0].id, span);
    assert_eq!(spans[0].text, "bounded after");
    assert_eq!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: false,
            can_undo: true,
            revision: applied,
        }),
    );

    let HistoryTraversalOutcome::Traversed { revision: undone, .. } =
        session.traverse_history(applied, HistoryDirection::Undo)
    else {
        panic!("live-owner text batch must Undo as one transaction");
    };
    let current = session.accepted_revision().expect("batch Undo revision");
    let BlockContent::Paragraph(spans) =
        &current.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("batch Undo fixture must remain paragraph");
    };
    assert_eq!(spans[0].id, span);
    assert_eq!(spans[0].text, "bounded before");
    assert_ne!(undone, applied);
}

#[test]
fn one_application_owns_draft_and_accepted_revision_together() {
    let identities = IdentityAllocator::new();
    let mut session = application::SessionApplication::default();

    assert_eq!(
        session.replace(DraftField::Task, String::from("private task")),
        DraftMutation::Applied,
    );
    assert!(matches!(
        session.accept_candidate(minimal_candidate(&identities)),
        AcceptanceOutcome::Accepted { .. }
    ));

    assert_eq!(session.value(DraftField::Task), "private task");
    assert!(session.accepted_revision().is_some());
}

#[test]
fn dropping_application_leaves_a_fresh_session_empty() {
    let identities = IdentityAllocator::new();
    {
        let mut first = application::SessionApplication::default();
        assert_eq!(
            first.replace(DraftField::Source, String::from("private source")),
            DraftMutation::Applied,
        );
        assert!(matches!(
            first.accept_candidate(minimal_candidate(&identities)),
            AcceptanceOutcome::Accepted { .. }
        ));
        assert!(matches!(
            first.accept_candidate(minimal_candidate(&identities)),
            AcceptanceOutcome::Accepted { .. }
        ));
        assert!(first.accepted_revision().is_some());
        assert!(matches!(
            first.history_availability(),
            HistoryAvailabilityOutcome::Available(HistoryAvailability {
                can_undo: true,
                ..
            })
        ));
    }

    let fresh = application::SessionApplication::default();
    for field in [DraftField::Candidate, DraftField::Source, DraftField::Task] {
        assert_eq!(fresh.value(field), "");
    }
    assert!(fresh.accepted_revision().is_none());
    assert_eq!(
        fresh.history_availability(),
        HistoryAvailabilityOutcome::NoAcceptedRevision,
    );
}

#[test]
fn application_debug_does_not_expose_private_session_text() {
    let identities = IdentityAllocator::new();
    let mut session = application::SessionApplication::default();
    let private_draft = "session-only private source";
    let private_semantic = "accepted-only private paragraph";
    assert_eq!(
        session.replace(DraftField::Source, String::from(private_draft)),
        DraftMutation::Applied,
    );
    let (candidate, _) = editable_text_candidate(&identities, private_semantic);
    assert!(matches!(
        session.accept_candidate(candidate),
        AcceptanceOutcome::Accepted { .. }
    ));

    let debug = format!("{session:?}");
    assert!(debug.contains("SessionApplication"));
    assert!(!debug.contains(private_draft));
    assert!(!debug.contains(private_semantic));
}
