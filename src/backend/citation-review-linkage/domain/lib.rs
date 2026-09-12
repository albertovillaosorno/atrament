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
//   - Inputs: Caller-owned claim/provenance identities, revision-owned semantic
//     provenance records, source metadata, and exact citation links.
//   - Outputs: Deterministic structural review-link validation.
//   - Side effects: None.
// - Split-When:
//   - Source retrieval, citation parsing, or quality policy gains authority.
// - Merge-When:
//   - Citation linkage becomes inseparable from semantic-notebook validation.
// - Summary:
//   - Requires cited claims to resolve to exact reviewable source metadata.
// - Description:
//   - Resolves source status from semantic provenance instead of caller copies.
// - Usage:
//   - Validate candidate citation linkage before review or acceptance.
// - Defaults:
//   - No citation, source, or provenance relationship is inferred.
//

//! Citation linkage validation independent of source retrieval or formatting.

use atrament_semantic_notebook::{Provenance, ProvenanceKind};

/// One claim and the revision-owned provenance record assigned to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimProvenance<ClaimIdentity, ProvenanceIdentity> {
    /// Stable or candidate-local claim identity.
    pub claim_identity: ClaimIdentity,
    /// Exact provenance record identity assigned to this claim.
    pub provenance_identity: ProvenanceIdentity,
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
pub struct CitationReviewLinkage<Claim, Link, ProvenanceRecord, Source> {
    /// Claims whose provenance assignment participates in this review set.
    pub claims: Vec<Claim>,
    /// Citation links in caller-owned deterministic order.
    pub links: Vec<Link>,
    /// Revision-owned semantic provenance records used by claim assignments.
    pub provenance: Vec<ProvenanceRecord>,
    /// Reviewable source metadata in caller-owned deterministic order.
    pub sources: Vec<Source>,
}

/// Reviewable source metadata retained behind one exact source identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitationSource<Metadata, SourceIdentity> {
    /// Caller-owned source metadata presented for review.
    pub metadata: Metadata,
    /// Exact source identity referenced by citation links.
    pub source_identity: SourceIdentity,
}

/// Structural failure in one citation review linkage set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CitationReviewLinkageError<
    ClaimIdentity,
    ProvenanceIdentity,
    SourceIdentity,
> {
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
    /// The same provenance identity appears more than once.
    DuplicateProvenance {
        /// Repeated semantic provenance identity.
        provenance: ProvenanceIdentity,
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
        /// Linked claim whose resolved provenance kind is not `Cited`.
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
    /// A claim assignment names no revision-owned semantic provenance record.
    UnknownProvenance {
        /// Missing provenance identity.
        provenance: ProvenanceIdentity,
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
    Provenance<ProvenanceIdentity>,
    CitationSource<Metadata, SourceIdentity>,
>;

type CitationReviewResult<ClaimIdentity, ProvenanceIdentity, SourceIdentity> =
    Result<
        (),
        CitationReviewLinkageError<
            ClaimIdentity,
            ProvenanceIdentity,
            SourceIdentity,
        >,
    >;

/// Exact reviewable source records projected for one validated claim.
pub type CitationSourceProjectionResult<
    'review,
    ClaimIdentity,
    Metadata,
    ProvenanceIdentity,
    SourceIdentity,
> = Result<
    Vec<&'review CitationSource<Metadata, SourceIdentity>>,
    CitationReviewLinkageError<
        ClaimIdentity,
        ProvenanceIdentity,
        SourceIdentity,
    >,
>;

/// Return exact reviewable source records linked to one claim after full
/// structural validation.
///
/// Records follow caller citation-link order. A valid non-cited claim returns
/// an empty collection. Metadata remains caller-owned opaque review data: this
/// projection does not fetch, parse, rank, normalize, or judge sources.
///
/// # Errors
///
/// Returns the first structural linkage error before projection, or
/// [`CitationReviewLinkageError::UnknownClaim`] when the requested claim is not
/// present in an otherwise valid review set.
pub fn citation_sources_for_claim<
    'review,
    ClaimIdentity,
    Metadata,
    ProvenanceIdentity,
    SourceIdentity,
>(
    review: &'review TypedCitationReviewLinkage<
        ClaimIdentity,
        Metadata,
        ProvenanceIdentity,
        SourceIdentity,
    >,
    claim_identity: &ClaimIdentity,
) -> CitationSourceProjectionResult<
    'review,
    ClaimIdentity,
    Metadata,
    ProvenanceIdentity,
    SourceIdentity,
>
where
    ClaimIdentity: Clone + Eq,
    ProvenanceIdentity: Clone + Eq,
    SourceIdentity: Clone + Eq,
{
    validate_citation_review_linkage(review)?;
    if !review
        .claims
        .iter()
        .any(|claim| claim.claim_identity == *claim_identity)
    {
        return Err(CitationReviewLinkageError::UnknownClaim {
            claim: claim_identity.clone(),
        });
    }
    let mut sources = Vec::new();
    for link in review
        .links
        .iter()
        .filter(|link| link.claim_identity == *claim_identity)
    {
        let Some(source) = review
            .sources
            .iter()
            .find(|source| source.source_identity == link.source_identity)
        else {
            return Err(CitationReviewLinkageError::UnknownSource {
                source: link.source_identity.clone(),
            });
        };
        sources.push(source);
    }
    Ok(sources)
}

/// Return source identities linked to one claim after full structural
/// validation.
///
/// Source identities follow caller link order. A valid non-cited claim returns
/// an empty collection. This projection does not fetch, rank, deduplicate, or
/// judge source metadata.
///
/// # Errors
///
/// Returns the first structural linkage error before projection, or
/// [`CitationReviewLinkageError::UnknownClaim`] when the requested claim is not
/// present in an otherwise valid review set.
pub fn citation_source_identities_for_claim<
    'review,
    ClaimIdentity,
    Metadata,
    ProvenanceIdentity,
    SourceIdentity,
>(
    review: &'review TypedCitationReviewLinkage<
        ClaimIdentity,
        Metadata,
        ProvenanceIdentity,
        SourceIdentity,
    >,
    claim_identity: &ClaimIdentity,
) -> Result<
    Vec<&'review SourceIdentity>,
    CitationReviewLinkageError<
        ClaimIdentity,
        ProvenanceIdentity,
        SourceIdentity,
    >,
>
where
    ClaimIdentity: Clone + Eq,
    ProvenanceIdentity: Clone + Eq,
    SourceIdentity: Clone + Eq,
{
    validate_citation_review_linkage(review)?;
    if !review
        .claims
        .iter()
        .any(|claim| claim.claim_identity == *claim_identity)
    {
        return Err(CitationReviewLinkageError::UnknownClaim {
            claim: claim_identity.clone(),
        });
    }
    Ok(review
        .links
        .iter()
        .filter_map(|link| {
            (link.claim_identity == *claim_identity)
                .then_some(&link.source_identity)
        })
        .collect())
}

/// Validate exact citation-to-claim-to-source metadata relationships.
///
/// Error precedence is duplicate claim/provenance/source inventories, then
/// claim-order provenance resolution, link-order identity/kind/duplicate
/// checks, then claim-order missing links.
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
) -> CitationReviewResult<ClaimIdentity, ProvenanceIdentity, SourceIdentity>
where
    ClaimIdentity: Clone + Eq,
    ProvenanceIdentity: Clone + Eq,
    SourceIdentity: Clone + Eq,
{
    validate_inventories_and_assignments(review)?;
    validate_links(review)?;
    validate_required_links(review)
}

fn validate_inventories_and_assignments<
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
) -> CitationReviewResult<ClaimIdentity, ProvenanceIdentity, SourceIdentity>
where
    ClaimIdentity: Clone + Eq,
    ProvenanceIdentity: Clone + Eq,
    SourceIdentity: Clone + Eq,
{
    for (index, claim) in review.claims.iter().enumerate() {
        if review
            .claims
            .iter()
            .take(index)
            .any(|prior| prior.claim_identity == claim.claim_identity)
        {
            return Err(CitationReviewLinkageError::DuplicateClaim {
                claim: claim.claim_identity.clone(),
            });
        }
    }
    for (index, provenance) in review.provenance.iter().enumerate() {
        if review
            .provenance
            .iter()
            .take(index)
            .any(|prior| prior.id == provenance.id)
        {
            return Err(CitationReviewLinkageError::DuplicateProvenance {
                provenance: provenance.id.clone(),
            });
        }
    }
    for (index, source) in review.sources.iter().enumerate() {
        if review
            .sources
            .iter()
            .take(index)
            .any(|prior| prior.source_identity == source.source_identity)
        {
            return Err(CitationReviewLinkageError::DuplicateSource {
                source: source.source_identity.clone(),
            });
        }
    }
    for claim in &review.claims {
        if !review
            .provenance
            .iter()
            .any(|provenance| provenance.id == claim.provenance_identity)
        {
            return Err(CitationReviewLinkageError::UnknownProvenance {
                provenance: claim.provenance_identity.clone(),
            });
        }
    }
    Ok(())
}

fn validate_links<ClaimIdentity, Metadata, ProvenanceIdentity, SourceIdentity>(
    review: &TypedCitationReviewLinkage<
        ClaimIdentity,
        Metadata,
        ProvenanceIdentity,
        SourceIdentity,
    >,
) -> CitationReviewResult<ClaimIdentity, ProvenanceIdentity, SourceIdentity>
where
    ClaimIdentity: Clone + Eq,
    ProvenanceIdentity: Clone + Eq,
    SourceIdentity: Clone + Eq,
{
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
        let provenance = review
            .provenance
            .iter()
            .find(|provenance| provenance.id == claim.provenance_identity);
        if !matches!(
            provenance.map(|record| record.kind),
            Some(ProvenanceKind::Cited)
        ) {
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
    Ok(())
}

fn validate_required_links<
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
) -> CitationReviewResult<ClaimIdentity, ProvenanceIdentity, SourceIdentity>
where
    ClaimIdentity: Clone + Eq,
    ProvenanceIdentity: Eq,
    SourceIdentity: Clone + Eq,
{
    for claim in &review.claims {
        let cited = review
            .provenance
            .iter()
            .find(|provenance| provenance.id == claim.provenance_identity)
            .is_some_and(|provenance| provenance.kind == ProvenanceKind::Cited);
        if cited
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
