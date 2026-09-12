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
//   - Regression evidence for the six checked-in bilingual assignment fixtures.
// - Must-Not:
//   - Judge curriculum facts, verify language identity, measure readability or
//     density, lay out pages, render, or score pedagogy.
// - Allows:
//   - Inputs: Exact checked-in assignment source and declared coverage
//     evidence.
//   - Outputs: Assertions over subject/artifact completeness and source
//     linkage.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Rendered education acceptance gains dedicated fixtures.
// - Merge-When:
//   - One complete educational acceptance harness supersedes these checks.
// - Summary:
//   - Pins concrete source artifacts for all six first-release subjects.
// - Description:
//   - Proves exact artifact linkage without treating source prose as a metric.
// - Usage:
//   - Compile against the educational assignment-fixture domain.
// - Defaults:
//   - Coverage statuses remain explicit checked-in declarations.
//
use atrament_educational_assignment_fixture::{
    REQUIRED_ASSIGNMENT_FIXTURES, declared_educational_coverage_observations,
    validate_checked_in_educational_assignment_fixtures,
};
use atrament_educational_coverage_evidence::{
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

#[test]
fn checked_in_assignment_fixture_set_is_structurally_complete() {
    assert_eq!(
        validate_checked_in_educational_assignment_fixtures(),
        Ok(()),
    );
}

#[test]
fn all_six_subjects_have_stable_distinct_bilingual_source_artifacts() {
    let expected_identities = [
        "education-biology-bilingual-v1",
        "education-chemistry-bilingual-v1",
        "education-history-bilingual-v1",
        "education-language-bilingual-v1",
        "education-mathematics-bilingual-v1",
        "education-physics-bilingual-v1",
    ];
    for ((fixture, subject), artifact_identity) in REQUIRED_ASSIGNMENT_FIXTURES
        .iter()
        .zip(SUBJECTS)
        .zip(expected_identities)
    {
        assert_eq!(fixture.subject, subject);
        assert_eq!(fixture.artifact_identity, artifact_identity);
        assert!(!fixture.english_source.trim().is_empty());
        assert!(!fixture.spanish_source.trim().is_empty());
        assert_ne!(fixture.english_source, fixture.spanish_source);
    }
}

#[test]
fn every_assignment_source_has_three_explicit_authored_modules() {
    for fixture in REQUIRED_ASSIGNMENT_FIXTURES {
        assert_eq!(fixture.english_source.lines().count(), 3);
        assert_eq!(fixture.spanish_source.lines().count(), 3);
    }
}

#[test]
fn declared_coverage_observations_link_exactly_to_checked_in_artifacts() {
    let observations = declared_educational_coverage_observations();
    assert_eq!(validate_educational_coverage(&observations), Ok(()));
    for (fixture, observation) in
        REQUIRED_ASSIGNMENT_FIXTURES.iter().zip(observations)
    {
        assert_eq!(observation.artifact_identity, fixture.artifact_identity);
        assert_eq!(observation.subject, fixture.subject);
        assert_eq!(
            observation.bilingual,
            EducationalEvidenceStatus::Established
        );
        assert_eq!(
            observation.dense_organization,
            EducationalEvidenceStatus::Established,
        );
        assert_eq!(
            observation.readable_organization,
            EducationalEvidenceStatus::Established,
        );
    }
}

#[test]
fn assignment_prompts_include_source_and_uncertainty_constraints() {
    let biology = REQUIRED_ASSIGNMENT_FIXTURES[0];
    assert!(biology.english_source.contains("supplied class notes"));
    assert!(biology.spanish_source.contains("apuntes proporcionados"));

    let history = REQUIRED_ASSIGNMENT_FIXTURES[2];
    assert!(history.english_source.contains("unverified"));
    assert!(history.spanish_source.contains("no verificadas"));

    let language = REQUIRED_ASSIGNMENT_FIXTURES[3];
    assert!(
        language
            .english_source
            .contains("without adding outside facts")
    );
    assert!(
        language
            .spanish_source
            .contains("sin agregar datos externos")
    );
}
