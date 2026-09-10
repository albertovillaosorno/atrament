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
//   - Transport-neutral first-release educational coverage evidence.
// - Must-Not:
//   - Author curriculum facts, choose languages, measure density/readability,
//     choose thresholds, lay out pages, render output, or score pedagogy.
// - Allows:
//   - Inputs: Caller-owned artifact identity plus bilingual, density, and
//     readability evidence for the required subject families.
//   - Outputs: Deterministic structural coverage validation.
//   - Side effects: None.
// - Split-When:
//   - Readability measurement or educational quality policy gains authority.
// - Merge-When:
//   - Coverage evidence becomes inseparable from renderer regression evidence.
// - Summary:
//   - Requires all first-release educational subject exercises to be evidenced.
// - Description:
//   - Checks subject completeness and caller-established exercise properties.
// - Usage:
//   - Validate a cross-subject exercise set before claiming P6 coverage.
// - Defaults:
//   - No language, density, readability, or content result is inferred.
//

//! Educational coverage evidence without curriculum or rendering authority.

/// Whether caller-owned evidence establishes one required exercise property.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EducationalEvidenceStatus {
    /// Caller-owned evidence establishes the property.
    Established,
    /// Caller-owned evidence does not establish the property.
    NotEstablished,
}

/// First-release subject families required by the P6 coverage task.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EducationalSubject {
    /// Biology assignment evidence.
    Biology,
    /// Chemistry assignment evidence.
    Chemistry,
    /// History assignment evidence.
    History,
    /// Language assignment evidence.
    Language,
    /// Mathematics assignment evidence.
    Mathematics,
    /// Physics assignment evidence.
    Physics,
}

/// One caller-produced educational exercise observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EducationalCoverageObservation<ArtifactIdentity, Evidence> {
    /// Exact artifact or fixture identity whose behavior was exercised.
    pub artifact_identity: ArtifactIdentity,
    /// Evidence that bilingual content was exercised.
    pub bilingual: Evidence,
    /// Evidence that dense page organization was exercised.
    pub dense_organization: Evidence,
    /// Evidence that page organization remained readable.
    pub readable_organization: Evidence,
    /// Subject family exercised by this observation.
    pub subject: EducationalSubject,
}

/// Why an educational exercise set cannot claim complete first-release
/// coverage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EducationalCoverageError {
    /// One observation lacks bilingual exercise evidence.
    BilingualEvidenceMissing {
        /// Zero-based caller observation index.
        observation_index: usize,
    },
    /// One observation lacks dense-organization evidence.
    DenseOrganizationEvidenceMissing {
        /// Zero-based caller observation index.
        observation_index: usize,
    },
    /// One observation lacks readable-organization evidence.
    ReadableOrganizationEvidenceMissing {
        /// Zero-based caller observation index.
        observation_index: usize,
    },
    /// No fully evidenced observation exists for one required subject.
    SubjectAbsent(EducationalSubject),
}

/// Validate all required subject exercises and their caller-owned evidence.
///
/// Observations are checked in caller order for bilingual, dense-organization,
/// then readable-organization evidence. Subject completeness is checked in the
/// fixed [`EducationalSubject`] order after every observation is admissible.
///
/// # Errors
///
/// Returns [`EducationalCoverageError`] for the first incomplete requirement.
pub fn validate_educational_coverage<ArtifactIdentity>(
    observations: &[EducationalCoverageObservation<
        ArtifactIdentity,
        EducationalEvidenceStatus,
    >],
) -> Result<(), EducationalCoverageError> {
    for (observation_index, observation) in observations.iter().enumerate() {
        if observation.bilingual == EducationalEvidenceStatus::NotEstablished {
            return Err(EducationalCoverageError::BilingualEvidenceMissing {
                observation_index,
            });
        }
        if observation.dense_organization
            == EducationalEvidenceStatus::NotEstablished
        {
            return Err(
                EducationalCoverageError::DenseOrganizationEvidenceMissing {
                    observation_index,
                },
            );
        }
        if observation.readable_organization
            == EducationalEvidenceStatus::NotEstablished
        {
            return Err(
                EducationalCoverageError::ReadableOrganizationEvidenceMissing {
                    observation_index,
                },
            );
        }
    }
    for subject in [
        EducationalSubject::Biology,
        EducationalSubject::Chemistry,
        EducationalSubject::History,
        EducationalSubject::Language,
        EducationalSubject::Mathematics,
        EducationalSubject::Physics,
    ] {
        if !observations
            .iter()
            .any(|observation| observation.subject == subject)
        {
            return Err(EducationalCoverageError::SubjectAbsent(subject));
        }
    }
    Ok(())
}
