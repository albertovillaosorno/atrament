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
//   - Side effects: Process-local test allocations only.
// - Split-When:
//   - Assets, history, previews, or derived plans join the session owner.
// - Merge-When:
//   - Full process lifecycle fixtures supersede this application-level proof.
// - Summary:
//   - Verifies draft and accepted notebook state share one disposable owner.
// - Description:
//   - Proves the live-session application can hold both state classes and a
//     fresh application cannot observe either after the prior owner is dropped.
// - Usage:
//   - Compile against the session application and semantic inbound-port crates.
// - Defaults:
//   - Starts with empty draft fields and no accepted semantic revision.
//
use atrament_semantic_notebook::{
    CandidateIdentity, IdentityAllocator, Notebook,
};
use atrament_semantic_notebook_port::AcceptanceOutcome;
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
        assert!(first.accepted_revision().is_some());
    }

    let fresh = application::SessionApplication::default();
    for field in [DraftField::Candidate, DraftField::Source, DraftField::Task] {
        assert_eq!(fresh.value(field), "");
    }
    assert!(fresh.accepted_revision().is_none());
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
