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
//   - Transport-neutral claim-to-citation-to-source review linkage.
// - Must-Not:
//   - Fetch sources, infer claims, parse citations, choose citation formats,
//     judge source quality, mutate notebooks, or define transaction provenance.
// - Allows:
//   - Inputs: Caller-owned claim/provenance identities, source metadata, and
//     citation links using the semantic notebook provenance-kind authority.
//   - Outputs: Deterministic structural review-link validation.
//   - Side effects: None.
// - Split-When:
//   - Source retrieval, citation parsing, or quality policy gains authority.
// - Merge-When:
//   - Citation linkage becomes inseparable from semantic-notebook validation.
// - Summary:
//   - Requires cited claims to resolve to exact reviewable source metadata.
// - Description:
//   - Validates identities and links without interpreting source content.
// - Usage:
//   - Validate candidate citation linkage before review or acceptance.
// - Defaults:
//   - No citation, source, or provenance relationship is inferred.
//

//! Citation linkage validation independent of source retrieval or formatting.

use atrament_semantic_notebook::ProvenanceKind;

/// One claim and the revision-owned provenance record assigned to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimProvenance<ClaimIdentity, ProvenanceIdentity> {
    /// Stable or candidate-local claim identity.
    pub claim_identity: ClaimIdentity,
    /// Provenance category already owned by semantic-notebook authority.
    pub kind: ProvenanceKind,
    /// Exact provenance record identity assigned to this claim.
    pub provenance_identity: ProvenanceIdentity,
}

/// Reviewable source metadata retained behind one exact source identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitationSource<Metadata, SourceIdentity> {
    /// Caller-owned source metadata presented for review.
    pub metadata: Metadata,
    /// Exact source identity referenced by citation links.
    pub source_identity: SourceIdentity,
}

/// One exact citation relationship from claim/provenance to source metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitationClaimLink<
    ClaimIdentity,
    ProvenanceIdentity,
    SourceIdentity,
> {
    /// Exact claim receiving the citation.
    pub claim_identity: ClaimIdentity,
    /// Exact provenance record assigned to that claim.
    pub provenance_identity: ProvenanceIdentity,
    /// Exact reviewable source metadata identity.
    pub source_identity: SourceIdentity,
}

/// Complete caller-owned citation review linkage before UI presentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitationReviewLinkage<Claim, Link, Source> {
    /// Claims whose provenance state participates in this review set.
    pub claims: Vec<Claim>,
    /// Citation links in caller-owned deterministic order.
    pub links: Vec<Link>,
    /// Reviewable source metadata in caller-owned deterministic order.
    pub sources: Vec<Source>,
}

/// Structural failure in one citation review linkage set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CitationReviewLinkageError<ClaimIdentity, SourceIdentity> {
    /// The same exact claim/source relationship appears more than once.
    DuplicateCitationLink {
        /// Repeated claim identity.
        claim: ClaimIdentity,
        /// Repeated source identity.
        source: SourceIdentity,
    },
    /// The same claim identity appears more than once in the claim inventory.
    DuplicateClaim {
        /// Repeated claim identity.
        claim: ClaimIdentity,
    },
    /// The same source identity appears more than once in source metadata.
    DuplicateSource {
        /// Repeated source identity.
        source: SourceIdentity,
    },
    /// A cited claim has no exact source-metadata link.
    MissingCitationLink {
        /// Cited claim lacking a source link.
        claim: ClaimIdentity,
    },
    /// A citation link targets a claim that is not marked cited.
    NonCitedClaimLink {
        /// Linked claim whose provenance kind is not `Cited`.
        claim: ClaimIdentity,
    },
    /// A citation link disagrees with the claim's assigned provenance identity.
    ProvenanceIdentityMismatch {
        /// Claim whose citation linkage disagrees.
        claim: ClaimIdentity,
    },
    /// A citation link names a claim absent from the review inventory.
    UnknownClaim {
        /// Missing claim identity.
        claim: ClaimIdentity,
    },
    /// A citation link names source metadata absent from the review inventory.
    UnknownSource {
        /// Missing source identity.
        source: SourceIdentity,
    },
}

/// Typed citation review linkage using the domain's claim/link/source records.
pub type TypedCitationReviewLinkage<
    ClaimIdentity,
    Metadata,
    ProvenanceIdentity,
    SourceIdentity,
> = CitationReviewLinkage<
    ClaimProvenance<ClaimIdentity, ProvenanceIdentity>,
    CitationClaimLink<ClaimIdentity, ProvenanceIdentity, SourceIdentity>,
    CitationSource<Metadata, SourceIdentity>,
>;

/// Validate exact citation-to-claim-to-source metadata relationships.
///
/// Error precedence is duplicate claims, duplicate sources, then link-order
/// claim/provenance/source/kind/duplicate checks, then claim-order missing
/// links.
///
/// # Errors
///
/// Returns [`CitationReviewLinkageError`] for the first structural violation.
pub fn validate_citation_review_linkage<
    ClaimIdentity,
    Metadata,
    ProvenanceIdentity,
    SourceIdentity,
>(
    review: &TypedCitationReviewLinkage<
        ClaimIdentity,
        Metadata,
        ProvenanceIdentity,
        SourceIdentity,
    >,
) -> Result<(), CitationReviewLinkageError<ClaimIdentity, SourceIdentity>>
where
    ClaimIdentity: Clone + Eq,
    ProvenanceIdentity: Eq,
    SourceIdentity: Clone + Eq,
{
    for (index, claim) in review.claims.iter().enumerate() {
        if review.claims.iter()
            .take(index)
            .any(|prior| prior.claim_identity == claim.claim_identity)
        {
            return Err(CitationReviewLinkageError::DuplicateClaim {
                claim: claim.claim_identity.clone(),
            });
        }
    }
    for (index, source) in review.sources.iter().enumerate() {
        if review.sources.iter()
            .take(index)
            .any(|prior| prior.source_identity == source.source_identity)
        {
            return Err(CitationReviewLinkageError::DuplicateSource {
                source: source.source_identity.clone(),
            });
        }
    }
    for (index, link) in review.links.iter().enumerate() {
        let Some(claim) = review
            .claims
            .iter()
            .find(|claim| claim.claim_identity == link.claim_identity)
        else {
            return Err(CitationReviewLinkageError::UnknownClaim {
                claim: link.claim_identity.clone(),
            });
        };
        if claim.provenance_identity != link.provenance_identity {
            return Err(CitationReviewLinkageError::ProvenanceIdentityMismatch {
                claim: link.claim_identity.clone(),
            });
        }
        if !review
            .sources
            .iter()
            .any(|source| source.source_identity == link.source_identity)
        {
            return Err(CitationReviewLinkageError::UnknownSource {
                source: link.source_identity.clone(),
            });
        }
        if claim.kind != ProvenanceKind::Cited {
            return Err(CitationReviewLinkageError::NonCitedClaimLink {
                claim: link.claim_identity.clone(),
            });
        }
        if review.links.iter().take(index).any(|prior| {
            prior.claim_identity == link.claim_identity
                && prior.source_identity == link.source_identity
        }) {
            return Err(CitationReviewLinkageError::DuplicateCitationLink {
                claim: link.claim_identity.clone(),
                source: link.source_identity.clone(),
            });
        }
    }
    for claim in &review.claims {
        if claim.kind == ProvenanceKind::Cited
            && !review
                .links
                .iter()
                .any(|link| link.claim_identity == claim.claim_identity)
        {
            return Err(CitationReviewLinkageError::MissingCitationLink {
                claim: claim.claim_identity.clone(),
            });
        }
    }
    Ok(())
}
