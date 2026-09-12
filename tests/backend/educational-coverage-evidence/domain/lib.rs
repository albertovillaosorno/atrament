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
//   - Regression evidence for first-release educational exercise coverage.
// - Must-Not:
//   - Author facts, measure readability/density, choose languages, or render.
// - Allows:
//   - Inputs: Deterministic subject, artifact, and evidence-state fixtures.
//   - Outputs: Assertions over completeness and deterministic failure order.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Educational quality measurement gains independent fixtures.
// - Merge-When:
//   - Coverage evidence moves into renderer regression fixtures.
// - Summary:
//   - Proves six subject families require bilingual dense/readable evidence.
// - Description:
//   - Covers all subjects, each property gate, and caller-owned identities.
// - Usage:
//   - Compile directly against the educational-coverage-evidence domain.
// - Defaults:
//   - No evidence state is synthesized from an artifact identity.
//
use atrament_educational_coverage_evidence::{
    EducationalCoverageError, EducationalCoverageObservation,
    EducationalEvidenceStatus, EducationalSubject,
    REQUIRED_EDUCATIONAL_SUBJECTS as SUBJECTS, validate_educational_coverage,
};

fn observation(
    subject: EducationalSubject,
) -> EducationalCoverageObservation<String, EducationalEvidenceStatus> {
    EducationalCoverageObservation {
        artifact_identity: format!("artifact-{subject:?}"),
        bilingual: EducationalEvidenceStatus::Established,
        dense_organization: EducationalEvidenceStatus::Established,
        readable_organization: EducationalEvidenceStatus::Established,
        subject,
    }
}

#[test]
fn complete_subject_set_with_all_three_properties_is_admitted() {
    let observations = SUBJECTS.map(observation);
    assert_eq!(validate_educational_coverage(&observations), Ok(()));
    assert_eq!(observations.len(), 6);
    assert_eq!(observations[0].subject, EducationalSubject::Biology);
    assert_eq!(observations[5].subject, EducationalSubject::Physics);
}

#[test]
fn every_required_subject_is_independently_required() {
    for missing in SUBJECTS {
        let observations = SUBJECTS
            .into_iter()
            .filter(|subject| *subject != missing)
            .map(observation)
            .collect::<Vec<_>>();
        assert_eq!(
            validate_educational_coverage(&observations),
            Err(EducationalCoverageError::SubjectAbsent(missing)),
            "missing subject {missing:?}",
        );
    }
}

#[test]
fn property_evidence_fails_in_bilingual_dense_readable_order() {
    let mut observations = SUBJECTS.map(observation);
    observations[2].bilingual = EducationalEvidenceStatus::NotEstablished;
    observations[2].dense_organization =
        EducationalEvidenceStatus::NotEstablished;
    observations[2].readable_organization =
        EducationalEvidenceStatus::NotEstablished;
    assert_eq!(
        validate_educational_coverage(&observations),
        Err(EducationalCoverageError::BilingualEvidenceMissing {
            observation_index: 2,
        }),
    );

    observations[2].bilingual = EducationalEvidenceStatus::Established;
    assert_eq!(
        validate_educational_coverage(&observations),
        Err(EducationalCoverageError::DenseOrganizationEvidenceMissing {
            observation_index: 2,
        }),
    );

    observations[2].dense_organization = EducationalEvidenceStatus::Established;
    assert_eq!(
        validate_educational_coverage(&observations),
        Err(EducationalCoverageError::ReadableOrganizationEvidenceMissing {
            observation_index: 2,
        }),
    );
}

#[test]
fn extra_observations_and_exact_artifact_identity_remain_caller_owned() {
    let mut observations = SUBJECTS.map(observation).to_vec();
    observations.push(EducationalCoverageObservation {
        artifact_identity: String::from("historia: México — 1910 🇲🇽"),
        bilingual: EducationalEvidenceStatus::Established,
        dense_organization: EducationalEvidenceStatus::Established,
        readable_organization: EducationalEvidenceStatus::Established,
        subject: EducationalSubject::History,
    });
    assert_eq!(validate_educational_coverage(&observations), Ok(()));
    assert_eq!(
        observations[6].artifact_identity,
        "historia: México — 1910 🇲🇽",
    );
}

#[test]
fn every_subject_presence_mask_matches_first_missing_subject_oracle() {
    let mut cases = 0_u8;
    for subject_mask in 0_u8..64 {
        let observations = SUBJECTS
            .into_iter()
            .enumerate()
            .filter(|(index, _)| subject_mask & (1_u8 << index) != 0)
            .map(|(_, subject)| observation(subject))
            .collect::<Vec<_>>();
        let expected = SUBJECTS
            .into_iter()
            .enumerate()
            .find(|(index, _)| subject_mask & (1_u8 << index) == 0)
            .map_or(Ok(()), |(_, subject)| {
                Err(EducationalCoverageError::SubjectAbsent(subject))
            });
        assert_eq!(
            validate_educational_coverage(&observations),
            expected,
            "subject presence mask {subject_mask:#08b}",
        );
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 64);
}

#[test]
fn every_property_mask_at_every_position_matches_evidence_precedence() {
    let mut cases = 0_u8;
    for observation_index in 0..SUBJECTS.len() {
        for established_mask in 0_u8..8 {
            let mut observations = SUBJECTS.map(observation);
            let current = observations
                .get_mut(observation_index)
                .expect("six enumerated observations exist");
            current.bilingual = if established_mask & 0b001 != 0 {
                EducationalEvidenceStatus::Established
            } else {
                EducationalEvidenceStatus::NotEstablished
            };
            current.dense_organization = if established_mask & 0b010 != 0 {
                EducationalEvidenceStatus::Established
            } else {
                EducationalEvidenceStatus::NotEstablished
            };
            current.readable_organization = if established_mask & 0b100 != 0 {
                EducationalEvidenceStatus::Established
            } else {
                EducationalEvidenceStatus::NotEstablished
            };
            let expected = if established_mask & 0b001 == 0 {
                Err(EducationalCoverageError::BilingualEvidenceMissing {
                    observation_index,
                })
            } else if established_mask & 0b010 == 0 {
                Err(EducationalCoverageError::DenseOrganizationEvidenceMissing {
                    observation_index,
                })
            } else if established_mask & 0b100 == 0 {
                Err(
                    EducationalCoverageError::
                        ReadableOrganizationEvidenceMissing {
                            observation_index,
                        },
                )
            } else {
                Ok(())
            };
            assert_eq!(
                validate_educational_coverage(&observations),
                expected,
                "observation {observation_index}, established mask \
                 {established_mask:#05b}",
            );
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 48);
}
