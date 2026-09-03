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
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

use atrament_semantic_notebook::{
    CandidateIdentity, IdentityAllocator, Notebook, SemanticIdentityDescriptor,
    SemanticIdentityKind,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CommandBehaviorVersion, DirectEditBatchApplyOutcome,
    DirectEditBatchProposal, HistoryAvailability, HistoryAvailabilityOutcome,
    IdentityAncestryCompleteness, IdentityAncestryEntry,
    IdentityAncestryInspectOutcome, IdentityInspectOutcome,
};
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};

#[allow(dead_code)]
#[path = "../../../../src/backend/session-application/application/lib.rs"]
mod application;

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
    assert_eq!(mode, "populated");
    let identities = IdentityAllocator::new();
    let mut session = application::SessionApplication::default();
    assert!(matches!(
        session.accept_candidate(minimal_candidate(&identities)),
        AcceptanceOutcome::Accepted { .. }
    ));
    assert!(matches!(
        session.accept_candidate(minimal_candidate(&identities)),
        AcceptanceOutcome::Accepted { .. }
    ));
    assert!(matches!(
        session.history_availability(),
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_undo: true,
            ..
        })
    ));
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

    let mut forced = spawn_process_fixture("populated");
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
fn application_routes_atomic_batch_apply_through_owned_semantic_authority() {
    let identities = IdentityAllocator::new();
    let mut session = application::SessionApplication::default();
    let AcceptanceOutcome::Accepted { revision, .. } =
        session.accept_candidate(minimal_candidate(&identities))
    else {
        panic!("minimal candidate must be accepted");
    };
    let empty = DirectEditBatchProposal::<u32> {
        base: revision,
        capability_version: CommandBehaviorVersion(1),
        commands: Vec::new(),
    };

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
fn application_debug_does_not_expose_private_draft_text() {
    let mut session = application::SessionApplication::default();
    let private = "session-only private source";
    assert_eq!(
        session.replace(DraftField::Source, String::from(private)),
        DraftMutation::Applied,
    );

    let debug = format!("{session:?}");
    assert!(debug.contains("SessionApplication"));
    assert!(!debug.contains(private));
}
