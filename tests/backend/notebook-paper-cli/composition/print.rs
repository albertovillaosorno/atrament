// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Explicit local printing admission and harmless fake-spool fixtures.
// - Must-Not:
//   - Submit an actual printer job or confuse CUPS acceptance with printing.
// - Allows:
//   - Inputs: Owned temporary PDFs, printer names and fake CUPS executable.
//   - Outputs: Assertions for accepted and refused CLI and spool operations.
//   - Side effects: Only temporary local test files and an owned fake spool.
// - Split-When:
//   - Real printer integration gains a dedicated opt-in test environment.
// - Merge-When:
//   - A physical printer adapter owns this print contract.
// - Summary:
//   - Proves exports stay separate from explicitly selected local printing.
// - Description:
//   - Verifies bounded PDF snapshots and shell-free argv/stderr behavior.
// - Usage:
//   - cargo test -p atrament_notebook_paper_cli --bin atrament-print
// - Defaults:
//   - No printer access, no default-queue submission.
//

//! Isolated print entrypoint contract tests; no real print queue is contacted.

#![expect(
    clippy::expect_used,
    reason = "test-only fail-fast printer admission fixtures"
)]

use super::*;
use std::fs;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn fixture_dir() -> PathBuf {
    let number = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let dir = env::temp_dir().join(format!(
        "atrament-print-fixture-{}-{number}",
        process::id(),
    ));
    fs::create_dir_all(&dir).expect("create owned fixture directory");
    dir
}

fn real_pdf_with_marks(include_marks: bool) -> Vec<u8> {
    use atrament_notebook_paper_cli::{
        NotebookInkLine, PaperDatePlaceholder,
        paper_profile_pdf_pages_with_ink,
    };

    let paper = atrament_notebook_paper_cli::default_paper_profile_json();
    let ink = [NotebookInkLine {
        page: 2,
        start_um: (25_400, 25_400),
        end_um: (50_800, 50_800),
        width_um: 700,
        rgb: [0xde, 0x11, 0x44],
    }];
    paper_profile_pdf_pages_with_ink(
        paper.as_bytes(), 2, include_marks,
        Some(PaperDatePlaceholder::LeftLine), true,
        5_000, Some("2026-11-04"), &ink,
    ).expect("native two-page printed-sheet PDF with ink and date")
}

fn real_pdf() -> Vec<u8> {
    real_pdf_with_marks(false)
}

#[test]
fn printing_requires_exact_file_and_named_queue() {
    let valid = ["--pdf", "notebook.pdf", "--printer", "Local_Printer-2"]
        .map(str::to_owned);
    let request = parse_args(&valid).expect("valid options").expect("print");
    assert_eq!(request.pdf, PathBuf::from("notebook.pdf"));
    assert_eq!(request.printer, "Local_Printer-2");
    assert!(!request.dry_run);
    let dry_args = [
        "--pdf",
        "notebook.pdf",
        "--printer",
        "Local_Printer-2",
        "--dry-run",
    ]
    .map(str::to_owned);
    assert!(
        parse_args(&dry_args)
            .expect("dry run parsed")
            .expect("request")
            .dry_run
    );
    assert_eq!(parse_args(&["--help".into()]).expect("help"), None);

    let refused = [
        vec![],
        vec!["--pdf", "notebook.pdf"],
        vec!["--printer", "local"],
        vec!["--pdf", "notebook.pdf", "--printer", ""],
        vec!["--pdf", "notebook.pdf", "--printer", "-bad"],
        vec!["--pdf", "notebook.pdf", "--printer", "local;touch"],
        vec!["--pdf", "notebook.pdf", "--printer", "bad queue"],
        vec!["--pdf", "notebook.pdf", "--printer", "\u{e9}"],
        vec![
            "--pdf",
            "notebook.pdf",
            "--printer",
            "queue",
            "--copies",
            "2",
        ],
        vec![
            "--pdf",
            "notebook.pdf",
            "--printer",
            "queue",
            "--printer",
            "other",
        ],
        vec![
            "--pdf",
            "notebook.pdf",
            "--pdf",
            "other.pdf",
            "--printer",
            "q",
        ],
        vec!["--help", "--pdf", "notebook.pdf"],
        vec![
            "--pdf",
            "file.pdf",
            "--printer",
            "queue",
            "--dry-run",
            "--dry-run",
        ],
    ];
    for flags in refused {
        let args = flags.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert!(parse_args(&args).is_err(), "refused {flags:?}");
    }
    assert!(!valid_printer(&"a".repeat(81)));
}

#[test]
fn print_admits_real_two_page_pdf_and_refuses_bad_sources() {
    let directory = fixture_dir();
    let pdf = directory.join("two-pages.pdf");
    let bytes = real_pdf();
    let vector = String::from_utf8_lossy(&bytes);
    assert!(vector.contains("/Count 2"));
    assert_eq!(vector.matches("0.871 0.067 0.267 RG").count(), 1,
        "page-specific colored pen ink must survive preprinted export");
    assert!(vector.contains("0.188 0.259 0.345 rg"),
        "printed Gregorian date uses its own filled vector digits");
    let ruled = real_pdf_with_marks(true);
    let ruled_vector = String::from_utf8_lossy(&ruled);
    let paper_color = "0.773 0.831 0.906 RG";
    assert!(ruled_vector.matches(paper_color).count()
        > vector.matches(paper_color).count(),
        "preprinted PDF suppresses existing page rules, retaining borders");
    fs::write(&pdf, &bytes).expect("write source PDF");
    assert_eq!(selected_pdf(&pdf).expect("real PDF accepted"), bytes);

    for data in [
        b"not a PDF".as_slice(),
        b"%PDF-1.4\nmissing EOF",
        b"%PDF-1.4\n%%EOF\nnonwhitespace",
    ] {
        fs::write(&pdf, data).expect("replace bad fixture");
        assert!(selected_pdf(&pdf).is_err(), "refuse invalid PDF");
    }
    fs::write(&pdf, &bytes).expect("restore fixture");
    assert!(selected_pdf(&directory).is_err(), "no directory input");
    assert!(selected_pdf(&directory.join("missing.pdf")).is_err());
    fs::write(&pdf, vec![b'X'; MAX_PAPER_OUTPUT_BYTES + 1])
        .expect("write oversize fixture");
    assert!(
        selected_pdf(&pdf).is_err(),
        "enforce native output byte limit"
    );
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn printer_is_only_contacted_on_explicit_selection_and_receives_exact_pdf() {
    use std::os::unix::fs::{PermissionsExt as _, symlink};

    let directory = fixture_dir();
    let pdf = directory.join("two-pages.pdf");
    let copied = directory.join("spooled.pdf");
    let arguments = directory.join("args.txt");
    let script = directory.join("fake-lp.sh");
    let bytes = real_pdf();
    fs::write(&pdf, &bytes).expect("source PDF");
    fs::write(
        &script,
        format!(
            concat!(
                "#!/bin/sh\n",
                "printf '%s\\n' \"$1\" \"$2\" \"$3\" > '{}'\n",
                "cat > '{}'\n",
            ),
            arguments.display(),
            copied.display(),
        ),
    )
    .expect("fake spool script");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");

    let selected = selected_pdf(&pdf).expect("complete input PDF");
    assert!(!copied.exists(), "reading PDF never silently prints");
    let dry_request = PrintRequest {
        pdf: pdf.clone(),
        printer: "Queue_1".into(),
        dry_run: true,
    };
    submit_selected(&dry_request, &script)
        .expect("bounded PDF checked without spooling");
    assert!(!copied.exists(), "dry-run never invokes lp");
    let print_request = PrintRequest {
        dry_run: false,
        ..dry_request
    };
    submit_selected(&print_request, &script)
        .expect("fake local CUPS acceptance");
    assert_eq!(fs::read(&copied).expect("spooled bytes"), bytes);
    assert_eq!(
        fs::read_to_string(&arguments).expect("arguments"),
        "-d\nQueue_1\n-\n",
        "no command option injection"
    );

    fs::write(&script, "#!/bin/sh\nexit 7\n")
        .expect("replace fake spool with refusing transport");
    assert!(submit_with(&script, "Queue_1", &selected).is_err());
    let linked = directory.join("linked.pdf");
    symlink(&pdf, &linked).expect("linked fixture");
    assert!(selected_pdf(&linked).is_err(), "source links fail closed");
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}
