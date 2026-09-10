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
    validate_educational_coverage,
};

const SUBJECTS: [EducationalSubject; 6] = [
    EducationalSubject::Biology,
    EducationalSubject::Chemistry,
    EducationalSubject::History,
    EducationalSubject::Language,
    EducationalSubject::Mathematics,
    EducationalSubject::Physics,
];

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
