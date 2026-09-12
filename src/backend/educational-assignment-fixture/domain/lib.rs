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
//   - Checked-in bilingual assignment source fixtures for the six first-release
//     educational subject families.
// - Must-Not:
//   - Assert curriculum correctness, verify natural-language identity, measure
//     density/readability, lay out pages, render, or score pedagogy.
// - Allows:
//   - Inputs: Frozen educational subject/evidence vocabulary.
//   - Outputs: Exact English/Spanish assignment source plus declared structural
//     coverage observations and fixture-completeness validation.
//   - Side effects: None.
// - Split-When:
//   - Rendered educational acceptance or curriculum/source checking gains
//     executable authority.
// - Merge-When:
//   - Assignment fixtures become inseparable from a complete education harness.
// - Summary:
//   - Supplies concrete cross-subject bilingual assignment source artifacts.
// - Description:
//   - Authors prompts rather than answers and preserves caller-declared
//     coverage evidence without interpreting it.
// - Usage:
//   - Feed the exact source fixtures into later semantic/layout/render tests.
// - Defaults:
//   - Declared evidence is not a readability metric or correctness judgment.
//

//! Checked-in bilingual assignment source for first-release education coverage.

use atrament_educational_coverage_evidence::{
    EducationalCoverageError, EducationalCoverageObservation,
    EducationalEvidenceStatus, EducationalSubject,
    validate_educational_coverage,
};

/// Exact checked-in assignment prompts in the evidence domain's subject order.
pub const REQUIRED_ASSIGNMENT_FIXTURES: [EducationalAssignmentFixture; 6] = [
    EducationalAssignmentFixture {
        artifact_identity: "education-biology-bilingual-v1",
        subject: EducationalSubject::Biology,
        english_source: concat!(
            "Prompt: Compare mitosis and meiosis using the supplied ",
            "class notes.\n",
            "Work: Organize purpose, chromosome count, and divisions ",
            "in a table.\n",
            "Check: Mark any claim not supported by the supplied notes ",
            "as unverified.",
        ),
        spanish_source: concat!(
            "Consigna: Compara mitosis y meiosis usando los apuntes ",
            "proporcionados.\n",
            "Trabajo: Organiza propósito, cromosomas y divisiones ",
            "en una tabla.\n",
            "Revisión: Marca como no verificada toda afirmación sin respaldo.",
        ),
    },
    EducationalAssignmentFixture {
        artifact_identity: "education-chemistry-bilingual-v1",
        subject: EducationalSubject::Chemistry,
        english_source: concat!(
            "Prompt: Balance Fe + O2 -> Fe2O3 from the supplied exercise.\n",
            "Work: Label reactants/products and compare atom counts ",
            "by element.\n",
            "Check: Keep experimental conclusions separate from ",
            "equation balancing.",
        ),
        spanish_source: concat!(
            "Consigna: Balancea Fe + O2 -> Fe2O3 del ejercicio ",
            "proporcionado.\n",
            "Trabajo: Etiqueta reactivos/productos y cuenta átomos ",
            "por elemento.\n",
            "Revisión: Separa conclusiones experimentales del ",
            "balanceo de la ecuación.",
        ),
    },
    EducationalAssignmentFixture {
        artifact_identity: "education-history-bilingual-v1",
        subject: EducationalSubject::History,
        english_source: concat!(
            "Prompt: Compare the two supplied historical source ",
            "excerpts only.\n",
            "Work: Track each author's claim, date, and source provenance.\n",
            "Check: Label unsupported causal inferences as unverified.",
        ),
        spanish_source: concat!(
            "Consigna: Compara únicamente los dos fragmentos ",
            "históricos dados.\n",
            "Trabajo: Registra afirmación, fecha y procedencia de ",
            "cada autor.\n",
            "Revisión: Marca como no verificadas las inferencias ",
            "causales sin fuente.",
        ),
    },
    EducationalAssignmentFixture {
        artifact_identity: "education-language-bilingual-v1",
        subject: EducationalSubject::Language,
        english_source: concat!(
            "Prompt: Analyze the supplied paragraph without adding ",
            "outside facts.\n",
            "Work: Identify its thesis, one metaphor, and two ",
            "transition words.\n",
            "Check: Write concise English and Spanish summaries of ",
            "the same passage.",
        ),
        spanish_source: concat!(
            "Consigna: Analiza el párrafo dado sin agregar datos externos.\n",
            "Trabajo: Identifica la tesis, una metáfora y dos conectores.\n",
            "Revisión: Resume el mismo pasaje de forma breve en ",
            "español e inglés.",
        ),
    },
    EducationalAssignmentFixture {
        artifact_identity: "education-mathematics-bilingual-v1",
        subject: EducationalSubject::Mathematics,
        english_source: concat!(
            "Prompt: Solve 2x + 3 = 11 and show each inverse operation.\n",
            "Work: Verify the result by substitution; compare 1/2 with 0.5.\n",
            "Check: Preserve exact authored mathematical source in each step.",
        ),
        spanish_source: concat!(
            "Consigna: Resuelve 2x + 3 = 11 y muestra cada operación ",
            "inversa.\n",
            "Trabajo: Verifica por sustitución; compara 1/2 con 0.5.\n",
            "Revisión: Conserva la fuente matemática escrita en cada paso.",
        ),
    },
    EducationalAssignmentFixture {
        artifact_identity: "education-physics-bilingual-v1",
        subject: EducationalSubject::Physics,
        english_source: concat!(
            "Prompt: A cart goes from rest to 6 m/s in 3 s in the ",
            "supplied problem.\n",
            "Work: Organize knowns, unknowns, equation choice, ",
            "units, and setup.\n",
            "Check: Leave the final numerical evaluation for the learner.",
        ),
        spanish_source: concat!(
            "Consigna: Un carrito pasa del reposo a 6 m/s en 3 s ",
            "en el problema dado.\n",
            "Trabajo: Organiza datos, incógnitas, ecuación, unidades ",
            "y planteamiento.\n",
            "Revisión: Deja la evaluación numérica final para el estudiante.",
        ),
    },
];

/// One exact bilingual assignment source artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EducationalAssignmentFixture {
    /// Stable checked-in artifact identity.
    pub artifact_identity: &'static str,
    /// English assignment source authored for this fixture.
    pub english_source: &'static str,
    /// Spanish assignment source authored for this fixture.
    pub spanish_source: &'static str,
    /// Required educational subject family represented by the fixture.
    pub subject: EducationalSubject,
}

/// Why the checked-in educational fixture set is structurally incomplete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EducationalAssignmentFixtureError {
    /// Existing educational coverage evidence rejected the declared set.
    Coverage(EducationalCoverageError),
    /// One fixture has no English source text.
    EmptyEnglishSource {
        /// Zero-based fixture index.
        fixture_index: usize,
    },
    /// One fixture has no Spanish source text.
    EmptySpanishSource {
        /// Zero-based fixture index.
        fixture_index: usize,
    },
    /// English and Spanish source fields are accidentally identical.
    IdenticalLanguageSources {
        /// Zero-based fixture index.
        fixture_index: usize,
    },
}

/// Complete declared coverage set linked to the six checked-in fixtures.
pub type EducationalAssignmentCoverageObservations =
    [EducationalCoverageObservation<&'static str, EducationalEvidenceStatus>;
        6];

/// Declared coverage evidence linked to the exact checked-in artifacts.
///
/// These statuses are checked-in caller declarations, not measurements inferred
/// from source text. Later acceptance owners remain responsible for language,
/// density, readability, layout, rendering, and educational quality evidence.
#[must_use]
pub fn declared_educational_coverage_observations()
-> EducationalAssignmentCoverageObservations {
    REQUIRED_ASSIGNMENT_FIXTURES.map(|fixture| EducationalCoverageObservation {
        artifact_identity: fixture.artifact_identity,
        bilingual: EducationalEvidenceStatus::Established,
        dense_organization: EducationalEvidenceStatus::Established,
        readable_organization: EducationalEvidenceStatus::Established,
        subject: fixture.subject,
    })
}

/// Validate checked-in assignment source and declared structural coverage.
///
/// This validation cannot establish natural-language correctness, curriculum
/// correctness, page density/readability, or rendered quality.
///
/// # Errors
///
/// Returns the first empty/identical language-source field in fixture order or
/// the existing coverage-domain rejection for the complete declared set.
pub fn validate_checked_in_educational_assignment_fixtures()
-> Result<(), EducationalAssignmentFixtureError> {
    for (fixture_index, fixture) in
        REQUIRED_ASSIGNMENT_FIXTURES.iter().enumerate()
    {
        if fixture.english_source.trim().is_empty() {
            return Err(
                EducationalAssignmentFixtureError::EmptyEnglishSource {
                    fixture_index,
                },
            );
        }
        if fixture.spanish_source.trim().is_empty() {
            return Err(
                EducationalAssignmentFixtureError::EmptySpanishSource {
                    fixture_index,
                },
            );
        }
        if fixture.english_source == fixture.spanish_source {
            return Err(
                EducationalAssignmentFixtureError::IdenticalLanguageSources {
                    fixture_index,
                },
            );
        }
    }
    validate_educational_coverage(&declared_educational_coverage_observations())
        .map_err(EducationalAssignmentFixtureError::Coverage)
}
