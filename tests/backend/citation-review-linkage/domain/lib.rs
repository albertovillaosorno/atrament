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
//   - Regression evidence for exact cited-claim source review linkage.
// - Must-Not:
//   - Fetch sources, infer claims, judge source quality, or render citation UI.
// - Allows:
//   - Inputs: Deterministic claim, provenance, source, and metadata fixtures.
//   - Outputs: Assertions over exact linkage and deterministic failure order.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Source retrieval or citation parsing gains independent fixtures.
// - Merge-When:
//   - Citation evidence moves into semantic candidate acceptance tests.
// - Summary:
//   - Proves cited claims resolve to exact reviewable source metadata.
// - Description:
//   - Covers unknown, duplicate, mismatched, non-cited, and missing links.
// - Usage:
//   - Compile directly against the citation-review-linkage domain.
// - Defaults:
//   - No source relationship is synthesized by a fixture helper.
//

//! Source and claim citation linkage regression tests.

use std::ptr;

use atrament_citation_review_linkage::{
    CitationClaimLink, CitationReviewLinkage, CitationReviewLinkageError,
    CitationSource, ClaimProvenance, citation_provenance_for_claim,
    citation_source_identities_for_claim, citation_sources_for_claim,
    validate_citation_review_linkage, validate_citation_review_linkage_view,
};
use atrament_semantic_notebook::{Provenance, ProvenanceKind};

// Test-only guards assert result shapes before inspecting borrowed evidence.
macro_rules! require_ok {
    ($expression:expr) => {{
        let outcome = $expression;
        assert!(
            outcome.is_ok(),
            "expected admitted linkage: {:?}",
            outcome.as_ref().err()
        );
        let Ok(value) = outcome else {
            return;
        };
        value
    }};
}

macro_rules! require_some {
    ($expression:expr) => {{
        let found = $expression;
        assert!(
            found.is_some(),
            "fixed citation fixture must contain the record"
        );
        let Some(value) = found else {
            return;
        };
        value
    }};
}

type Claim = ClaimProvenance<u8, u8>;
type Link = CitationClaimLink<u8, u8, u8>;
type Source = CitationSource<&'static str, u8>;
type Review = CitationReviewLinkage<Claim, Link, Provenance<u8>, Source>;

const fn provenance(id: u8, kind: ProvenanceKind) -> Provenance<u8> {
    Provenance {
        id,
        kind,
        reference: None,
    }
}

fn review() -> Review {
    CitationReviewLinkage {
        claims: vec![
            ClaimProvenance {
                claim_identity: 1,
                provenance_identity: 11,
            },
            ClaimProvenance {
                claim_identity: 2,
                provenance_identity: 12,
            },
        ],
        links: vec![CitationClaimLink {
            claim_identity: 1,
            provenance_identity: 11,
            source_identity: 21,
        }],
        provenance: vec![
            provenance(11, ProvenanceKind::Cited),
            provenance(12, ProvenanceKind::Derived),
        ],
        sources: vec![CitationSource {
            metadata: "doi:10.1000/example; title=Example Source",
            source_identity: 21,
        }],
    }
}

#[test]
fn cited_claim_resolves_through_exact_provenance_to_reviewable_source() {
    let review = review();
    assert_eq!(validate_citation_review_linkage(&review), Ok(()));
    let validated = require_ok!(validate_citation_review_linkage_view(&review));
    assert!(
        ptr::eq(validated.review(), &raw const review),
        "sealed linkage borrows exact revision",
    );
    assert!(ptr::eq(
        require_ok!(validated.provenance_for_claim(&1)),
        require_some!(review.provenance.first()),
    ));
    assert_eq!(validated.source_identities_for_claim(&1), Ok(vec![&21]),);
    let source_projection = require_ok!(validated.sources_for_claim(&1));
    assert!(ptr::eq(
        *require_some!(source_projection.first()),
        require_some!(review.sources.first()),
    ));
    assert_eq!(require_some!(review.links.first()).claim_identity, 1);
    assert_eq!(require_some!(review.links.first()).provenance_identity, 11);
    assert_eq!(
        require_some!(review.provenance.first()).kind,
        ProvenanceKind::Cited
    );
    assert_eq!(require_some!(review.links.first()).source_identity, 21);
    assert_eq!(
        require_some!(review.sources.first()).metadata,
        "doi:10.1000/example; title=Example Source",
    );
}

#[test]
fn unknown_and_mismatched_identities_fail_in_documented_order() {
    let mut unknown_assignment = review();
    require_some!(unknown_assignment.claims.first_mut()).provenance_identity =
        99;
    assert_eq!(
        validate_citation_review_linkage(&unknown_assignment),
        Err(CitationReviewLinkageError::UnknownProvenance { provenance: 99 }),
    );

    let mut unknown_claim = review();
    require_some!(unknown_claim.links.first_mut()).claim_identity = 9;
    assert_eq!(
        validate_citation_review_linkage(&unknown_claim),
        Err(CitationReviewLinkageError::UnknownClaim { claim: 9 }),
    );

    let mut wrong_provenance = review();
    require_some!(wrong_provenance.links.first_mut()).provenance_identity = 12;
    assert_eq!(
        validate_citation_review_linkage(&wrong_provenance),
        Err(CitationReviewLinkageError::ProvenanceIdentityMismatch {
            claim: 1,
        }),
    );

    let mut unknown_source = review();
    require_some!(unknown_source.links.first_mut()).source_identity = 99;
    assert_eq!(
        validate_citation_review_linkage(&unknown_source),
        Err(CitationReviewLinkageError::UnknownSource { source: 99 }),
    );
}

#[test]
fn only_cited_provenance_records_admit_links_and_each_cited_claim_needs_one() {
    let mut non_cited = review();
    require_some!(non_cited.links.first_mut()).claim_identity = 2;
    require_some!(non_cited.links.first_mut()).provenance_identity = 12;
    assert_eq!(
        validate_citation_review_linkage(&non_cited),
        Err(CitationReviewLinkageError::NonCitedClaimLink { claim: 2 }),
    );

    let mut missing = review();
    missing.links.clear();
    assert_eq!(
        validate_citation_review_linkage(&missing),
        Err(CitationReviewLinkageError::MissingCitationLink { claim: 1 }),
    );
}

#[test]
fn duplicate_claim_provenance_source_and_link_reject_deterministically() {
    let mut duplicate_claim = review();
    let claims_copy = require_some!(duplicate_claim.claims.first()).clone();
    duplicate_claim.claims.push(claims_copy);
    assert_eq!(
        validate_citation_review_linkage(&duplicate_claim),
        Err(CitationReviewLinkageError::DuplicateClaim { claim: 1 }),
    );

    let mut duplicate_provenance = review();
    let provenance_copy =
        require_some!(duplicate_provenance.provenance.first()).clone();
    duplicate_provenance.provenance.push(provenance_copy);
    assert_eq!(
        validate_citation_review_linkage(&duplicate_provenance),
        Err(CitationReviewLinkageError::DuplicateProvenance { provenance: 11 }),
    );

    let mut duplicate_source = review();
    let sources_copy = require_some!(duplicate_source.sources.first()).clone();
    duplicate_source.sources.push(sources_copy);
    assert_eq!(
        validate_citation_review_linkage(&duplicate_source),
        Err(CitationReviewLinkageError::DuplicateSource { source: 21 }),
    );

    let mut duplicate_link = review();
    let links_copy = require_some!(duplicate_link.links.first()).clone();
    duplicate_link.links.push(links_copy);
    assert_eq!(
        validate_citation_review_linkage(&duplicate_link),
        Err(CitationReviewLinkageError::DuplicateCitationLink {
            claim: 1,
            source: 21,
        }),
    );
}

fn assert_direct_and_sealed_rejection(
    review: &Review,
    expected: CitationReviewLinkageError<u8, u8, u8>,
) {
    assert_eq!(
        validate_citation_review_linkage(review),
        Err(expected.clone()),
        "direct validation must retain exact structural rejection",
    );
    assert_eq!(
        validate_citation_review_linkage_view(review).map(|_| ()),
        Err(expected),
        "sealed view must retain the same structural rejection",
    );
}

#[test]
fn every_structural_error_rejects_direct_and_sealed_admission_identically() {
    let mut cases = 0u8;

    {
        let mut value = review();
        let duplicate = require_some!(value.claims.first()).clone();
        value.claims.push(duplicate);
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::DuplicateClaim { claim: 1 },
        );
        cases += 1;
    }

    {
        let mut value = review();
        let duplicate = require_some!(value.provenance.first()).clone();
        value.provenance.push(duplicate);
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::DuplicateProvenance { provenance: 11 },
        );
        cases += 1;
    }

    {
        let mut value = review();
        let duplicate = require_some!(value.sources.first()).clone();
        value.sources.push(duplicate);
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::DuplicateSource { source: 21 },
        );
        cases += 1;
    }

    {
        let mut value = review();
        let duplicate = require_some!(value.links.first()).clone();
        value.links.push(duplicate);
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::DuplicateCitationLink {
                claim: 1,
                source: 21,
            },
        );
        cases += 1;
    }

    {
        let mut value = review();
        value.links.clear();
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::MissingCitationLink { claim: 1 },
        );
        cases += 1;
    }

    {
        let mut value = review();
        require_some!(value.links.first_mut()).claim_identity = 2;
        require_some!(value.links.first_mut()).provenance_identity = 12;
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::NonCitedClaimLink { claim: 2 },
        );
        cases += 1;
    }

    {
        let mut value = review();
        require_some!(value.links.first_mut()).provenance_identity = 12;
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::ProvenanceIdentityMismatch { claim: 1 },
        );
        cases += 1;
    }

    {
        let mut value = review();
        require_some!(value.links.first_mut()).claim_identity = 99;
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::UnknownClaim { claim: 99 },
        );
        cases += 1;
    }

    {
        let mut value = review();
        require_some!(value.claims.first_mut()).provenance_identity = 99;
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::UnknownProvenance { provenance: 99 },
        );
        cases += 1;
    }

    {
        let mut value = review();
        require_some!(value.links.first_mut()).source_identity = 99;
        assert_direct_and_sealed_rejection(
            &value,
            CitationReviewLinkageError::UnknownSource { source: 99 },
        );
        cases += 1;
    }

    assert_eq!(cases, 10);
}

#[test]
fn one_cited_claim_may_link_to_multiple_distinct_sources() {
    let mut review = review();
    review.sources.push(CitationSource {
        metadata: "book:isbn-2",
        source_identity: 22,
    });
    review.links.push(CitationClaimLink {
        claim_identity: 1,
        provenance_identity: 11,
        source_identity: 22,
    });
    assert_eq!(validate_citation_review_linkage(&review), Ok(()));
}

#[test]
fn validated_claim_provenance_projection_borrows_exact_revision_record() {
    let mut value = review();
    require_some!(value.provenance.first_mut()).reference =
        Some(String::from("doi:10.1000/example"));
    let cited = require_ok!(citation_provenance_for_claim(&value, &1));
    assert!(ptr::eq(cited, require_some!(value.provenance.first())));
    assert_eq!(cited.id, 11);
    assert_eq!(cited.kind, ProvenanceKind::Cited);
    assert_eq!(cited.reference.as_deref(), Some("doi:10.1000/example"));

    let derived = require_ok!(citation_provenance_for_claim(&value, &2));
    assert!(ptr::eq(derived, require_some!(value.provenance.get(1))));
    assert_eq!(derived.kind, ProvenanceKind::Derived);
    assert_eq!(
        citation_provenance_for_claim(&value, &99),
        Err(CitationReviewLinkageError::UnknownClaim { claim: 99 }),
    );
}

#[test]
fn claim_provenance_projection_preserves_structural_error_precedence() {
    let mut value = review();
    value.sources.clear();
    assert_eq!(
        citation_provenance_for_claim(&value, &99),
        Err(CitationReviewLinkageError::UnknownSource { source: 21 }),
    );
}

#[test]
fn validated_claim_source_projection_preserves_link_order_and_empty_non_cited()
{
    let mut value = review();
    value.sources.push(CitationSource {
        metadata: "source metadata b",
        source_identity: 22,
    });
    value.links.push(CitationClaimLink {
        claim_identity: 1,
        provenance_identity: 11,
        source_identity: 22,
    });
    assert_eq!(
        citation_source_identities_for_claim(&value, &1),
        Ok(vec![&21, &22]),
    );

    assert_eq!(
        citation_source_identities_for_claim(&value, &2),
        Ok(Vec::<&u8>::new()),
    );
    assert_eq!(
        citation_source_identities_for_claim(&value, &99),
        Err(CitationReviewLinkageError::UnknownClaim { claim: 99 }),
    );
}

#[test]
fn validated_claim_source_records_preserve_link_order_and_metadata() {
    let mut value = review();
    value.sources.push(CitationSource {
        metadata: "source metadata b",
        source_identity: 22,
    });
    value.links.insert(
        0,
        CitationClaimLink {
            claim_identity: 1,
            provenance_identity: 11,
            source_identity: 22,
        },
    );
    let sources = require_ok!(citation_sources_for_claim(&value, &1));
    assert_eq!(sources.len(), 2);
    assert_eq!(require_some!(sources.first()).source_identity, 22);
    assert_eq!(require_some!(sources.first()).metadata, "source metadata b");
    assert_eq!(require_some!(sources.get(1)).source_identity, 21);
    assert_eq!(
        require_some!(sources.get(1)).metadata,
        "doi:10.1000/example; title=Example Source",
    );
    assert_eq!(
        citation_sources_for_claim(&value, &2),
        Ok(Vec::<&Source>::new()),
    );
    assert_eq!(
        citation_sources_for_claim(&value, &99),
        Err(CitationReviewLinkageError::UnknownClaim { claim: 99 }),
    );
}

#[test]
fn claim_source_record_projection_preserves_structural_error_precedence() {
    let mut value = review();
    value.sources.clear();
    assert_eq!(
        citation_sources_for_claim(&value, &99),
        Err(CitationReviewLinkageError::UnknownSource { source: 21 }),
    );
}

#[test]
fn claim_source_projection_preserves_structural_error_precedence() {
    let mut value = review();
    value.sources.clear();
    assert_eq!(
        citation_source_identities_for_claim(&value, &99),
        Err(CitationReviewLinkageError::UnknownSource { source: 21 }),
    );
}

#[test]
fn compact_provenance_link_state_space_matches_structural_oracle() {
    let kinds = [
        ProvenanceKind::Cited,
        ProvenanceKind::Derived,
        ProvenanceKind::Supplied,
        ProvenanceKind::Unresolved,
    ];
    let mut cases = 0u8;
    let mut saw = [false; 5];
    for kind in kinds {
        for link_present in [false, true] {
            for source_present in [false, true] {
                for provenance_matches in [false, true] {
                    let review: Review = CitationReviewLinkage {
                        claims: vec![ClaimProvenance {
                            claim_identity: 1,
                            provenance_identity: 11,
                        }],
                        links: link_present
                            .then_some(CitationClaimLink {
                                claim_identity: 1,
                                provenance_identity: if provenance_matches {
                                    11
                                } else {
                                    99
                                },
                                source_identity: 21,
                            })
                            .into_iter()
                            .collect(),
                        provenance: vec![provenance(11, kind)],
                        sources: source_present
                            .then_some(CitationSource {
                                metadata: "source-21",
                                source_identity: 21,
                            })
                            .into_iter()
                            .collect(),
                    };
                    let expected = if !link_present {
                        if kind == ProvenanceKind::Cited {
                            *require_some!(saw.first_mut()) = true;
                            Err(
                                CitationReviewLinkageError::
                                    MissingCitationLink { claim: 1 },
                            )
                        } else {
                            *require_some!(saw.get_mut(1)) = true;
                            Ok(())
                        }
                    } else if !provenance_matches {
                        *require_some!(saw.get_mut(2)) = true;
                        Err(
                            CitationReviewLinkageError::
                                ProvenanceIdentityMismatch { claim: 1 },
                        )
                    } else if !source_present {
                        *require_some!(saw.get_mut(3)) = true;
                        Err(CitationReviewLinkageError::UnknownSource {
                            source: 21,
                        })
                    } else if kind != ProvenanceKind::Cited {
                        *require_some!(saw.get_mut(4)) = true;
                        Err(CitationReviewLinkageError::NonCitedClaimLink {
                            claim: 1,
                        })
                    } else {
                        Ok(())
                    };
                    assert_eq!(
                        validate_citation_review_linkage(&review),
                        expected,
                        "kind {kind:?}, link {link_present}, source \
                         {source_present}, provenance match \
                         {provenance_matches}",
                    );
                    assert_eq!(
                        validate_citation_review_linkage_view(&review).map(
                            |validated| {
                                ptr::eq(validated.review(), &raw const review)
                            },
                        ),
                        expected.map(|()| true),
                        "sealed parity for kind {kind:?}, link {link_present}, \
                         source {source_present}, provenance match \
                         {provenance_matches}",
                    );
                    cases = cases.saturating_add(1);
                }
            }
        }
    }
    assert_eq!(cases, 32);
    assert!(
        saw.into_iter().all(|seen| seen),
        "all expected structural outcomes must be observed",
    );
}
