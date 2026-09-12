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
use atrament_citation_review_linkage::{
    CitationClaimLink, CitationReviewLinkage, CitationReviewLinkageError,
    CitationSource, ClaimProvenance, citation_provenance_for_claim,
    citation_source_identities_for_claim, citation_sources_for_claim,
    validate_citation_review_linkage,
};
use atrament_semantic_notebook::{Provenance, ProvenanceKind};

type Claim = ClaimProvenance<u8, u8>;
type Link = CitationClaimLink<u8, u8, u8>;
type Source = CitationSource<&'static str, u8>;
type Review = CitationReviewLinkage<Claim, Link, Provenance<u8>, Source>;

fn provenance(id: u8, kind: ProvenanceKind) -> Provenance<u8> {
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
    assert_eq!(review.links[0].claim_identity, 1);
    assert_eq!(review.links[0].provenance_identity, 11);
    assert_eq!(review.provenance[0].kind, ProvenanceKind::Cited);
    assert_eq!(review.links[0].source_identity, 21);
    assert_eq!(
        review.sources[0].metadata,
        "doi:10.1000/example; title=Example Source",
    );
}

#[test]
fn unknown_and_mismatched_identities_fail_in_documented_order() {
    let mut unknown_assignment = review();
    unknown_assignment.claims[0].provenance_identity = 99;
    assert_eq!(
        validate_citation_review_linkage(&unknown_assignment),
        Err(CitationReviewLinkageError::UnknownProvenance {
            provenance: 99,
        }),
    );

    let mut unknown_claim = review();
    unknown_claim.links[0].claim_identity = 9;
    assert_eq!(
        validate_citation_review_linkage(&unknown_claim),
        Err(CitationReviewLinkageError::UnknownClaim { claim: 9 }),
    );

    let mut wrong_provenance = review();
    wrong_provenance.links[0].provenance_identity = 12;
    assert_eq!(
        validate_citation_review_linkage(&wrong_provenance),
        Err(CitationReviewLinkageError::ProvenanceIdentityMismatch {
            claim: 1,
        }),
    );

    let mut unknown_source = review();
    unknown_source.links[0].source_identity = 99;
    assert_eq!(
        validate_citation_review_linkage(&unknown_source),
        Err(CitationReviewLinkageError::UnknownSource { source: 99 }),
    );
}

#[test]
fn only_cited_provenance_records_admit_links_and_each_cited_claim_needs_one() {
    let mut non_cited = review();
    non_cited.links[0].claim_identity = 2;
    non_cited.links[0].provenance_identity = 12;
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
    duplicate_claim.claims.push(duplicate_claim.claims[0].clone());
    assert_eq!(
        validate_citation_review_linkage(&duplicate_claim),
        Err(CitationReviewLinkageError::DuplicateClaim { claim: 1 }),
    );

    let mut duplicate_provenance = review();
    duplicate_provenance
        .provenance
        .push(duplicate_provenance.provenance[0].clone());
    assert_eq!(
        validate_citation_review_linkage(&duplicate_provenance),
        Err(CitationReviewLinkageError::DuplicateProvenance {
            provenance: 11,
        }),
    );

    let mut duplicate_source = review();
    duplicate_source.sources.push(duplicate_source.sources[0].clone());
    assert_eq!(
        validate_citation_review_linkage(&duplicate_source),
        Err(CitationReviewLinkageError::DuplicateSource { source: 21 }),
    );

    let mut duplicate_link = review();
    duplicate_link.links.push(duplicate_link.links[0].clone());
    assert_eq!(
        validate_citation_review_linkage(&duplicate_link),
        Err(CitationReviewLinkageError::DuplicateCitationLink {
            claim: 1,
            source: 21,
        }),
    );
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
    value.provenance[0].reference = Some(String::from("doi:10.1000/example"));
    let cited = citation_provenance_for_claim(&value, &1)
        .expect("cited claim provenance must project");
    assert!(std::ptr::eq(cited, &value.provenance[0]));
    assert_eq!(cited.id, 11);
    assert_eq!(cited.kind, ProvenanceKind::Cited);
    assert_eq!(cited.reference.as_deref(), Some("doi:10.1000/example"));

    let derived = citation_provenance_for_claim(&value, &2)
        .expect("non-cited claim still has exact semantic provenance");
    assert!(std::ptr::eq(derived, &value.provenance[1]));
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
    value.links.insert(0, CitationClaimLink {
        claim_identity: 1,
        provenance_identity: 11,
        source_identity: 22,
    });
    let sources = citation_sources_for_claim(&value, &1)
        .expect("cited claim source records must project");
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].source_identity, 22);
    assert_eq!(sources[0].metadata, "source metadata b");
    assert_eq!(sources[1].source_identity, 21);
    assert_eq!(
        sources[1].metadata,
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
    let mut cases = 0_u8;
    let mut saw = [false; 5];
    for kind in kinds {
        for link_present in [false, true] {
            for source_present in [false, true] {
                for provenance_matches in [false, true] {
                    let review = CitationReviewLinkage {
                        claims: vec![ClaimProvenance {
                            claim_identity: 1,
                            provenance_identity: 11,
                        }],
                        links: link_present
                            .then(|| CitationClaimLink {
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
                            .then(|| CitationSource {
                                metadata: "source-21",
                                source_identity: 21,
                            })
                            .into_iter()
                            .collect(),
                    };
                    let expected = if !link_present {
                        if kind == ProvenanceKind::Cited {
                            saw[0] = true;
                            Err(
                                CitationReviewLinkageError::
                                    MissingCitationLink { claim: 1 },
                            )
                        } else {
                            saw[1] = true;
                            Ok(())
                        }
                    } else if !provenance_matches {
                        saw[2] = true;
                        Err(
                            CitationReviewLinkageError::
                                ProvenanceIdentityMismatch { claim: 1 },
                        )
                    } else if !source_present {
                        saw[3] = true;
                        Err(CitationReviewLinkageError::UnknownSource {
                            source: 21,
                        })
                    } else if kind != ProvenanceKind::Cited {
                        saw[4] = true;
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
                    cases = cases.saturating_add(1);
                }
            }
        }
    }
    assert_eq!(cases, 32);
    assert!(saw.into_iter().all(|seen| seen));
}
