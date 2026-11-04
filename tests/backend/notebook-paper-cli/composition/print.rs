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
    assert_eq!(request.expected_sha256, None);
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
        vec!["--pdf", "file.pdf", "--printer", "q", "--expect-sha256", "ff"],
        vec!["--pdf", "file.pdf", "--printer", "q", "--expect-sha256", "Z"],
        vec!["--pdf", "file.pdf", "--printer", "q", "--expect-sha256", ""],
        vec!["--pdf", "file.pdf", "--printer", "q", "--expect-sha256"],
        vec!["--pdf", "file.pdf", "--printer", "q", "--expect-sha256",
            "00", "--expect-sha256", "11"],
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
fn reviewed_pdf_hash_requires_an_exact_valid_digest() {
    let checksum = sha256_hex(sha256_profile_bytes(b"abc").bytes())
        .expect("hex encoder");
    assert_eq!(checksum,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(parse_sha256(&checksum).expect("valid lowercase digest"),
        parse_sha256(&checksum.to_uppercase()).expect("valid upper digest"));
    let checksum_args = [
        "--pdf", "notebook.pdf", "--printer", "Local_Printer-2",
        "--expect-sha256", &checksum,
    ].map(str::to_owned);
    assert_eq!(parse_args(&checksum_args).expect("digest option")
        .expect("request").expected_sha256,
        Some(parse_sha256(&checksum).expect("hex digest")));
    let twice = [
        "--pdf", "file.pdf", "--printer", "q",
        "--expect-sha256", &checksum,
        "--expect-sha256", &checksum,
    ].map(str::to_owned);
    assert!(parse_args(&twice).is_err(), "duplicate valid digest refused");
    for digest in [
        "0".repeat(63),
        "0".repeat(65),
        format!("{}\n", "0".repeat(63)),
        format!("{}\u{e9}", "0".repeat(63)),
        format!("{}g", "0".repeat(63)),
    ] {
        assert!(parse_sha256(&digest).is_err(), "refused {digest:?}");
    }
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
    // A framing admission check accepts real PDF version labels, but
    // does not claim to parse the modified document structure.
    for header in [b"%PDF-1.0", b"%PDF-1.7", b"%PDF-2.0"] {
        let mut labeled = bytes.clone();
        labeled.get_mut(..header.len())
            .expect("native PDF has a complete version header")
            .copy_from_slice(header);
        fs::write(&pdf, &labeled).expect("save version-labeled fixture");
        assert_eq!(selected_pdf(&pdf).expect("supported PDF header"),
            labeled);
    }

    for data in [
        b"not a PDF".as_slice(),
        b"%PDF-1.4\nmissing EOF",
        b"%PDF-1.4\n%%EOF\nnonwhitespace",
        b"%PDF-1.8\n%%EOF",
        b"%PDF-2.1\n%%EOF",
        b"%PDF-0.9\n%%EOF",
        b"%PDF-1.\n%%EOF",
        b"%PDF-1.4x\n%%EOF",
        b"%PDF-1.4%%EOF",
        b"%PDF-1.4 \n%%EOF",
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
fn assert_legacy_xref_stream_refused(
    stream_framing: &[u8], pdf: &Path, fake_spool: &Path, spooled: &Path,
) {
    for &minor in b"04" {
        let mut obsolete = stream_framing.to_vec();
        if let Some(version_minor) = obsolete.get_mut(7) {
            *version_minor = minor;
        }
        assert!(!supported_pdf_cross_reference(&obsolete),
            "xref streams require PDF 1.5+ even with plausible framing");
        fs::write(pdf, &obsolete).expect("selected unsupported version");
        let request = PrintRequest {
            pdf: pdf.to_path_buf(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(submit_selected(&request, fake_spool).is_err(),
            "older PDF-version xref streams cannot reach CUPS");
        assert!(!spooled.exists(), "no spoofed stream spool side effect");
        assert_eq!(fs::read(pdf).expect("source unchanged"), obsolete);
    }
    let mut earliest_stream = stream_framing.to_vec();
    if let Some(version_minor) = earliest_stream.get_mut(7) {
        *version_minor = b'5';
    }
    assert!(supported_pdf_cross_reference(&earliest_stream),
        "PDF 1.5 is the earliest admitted xref-stream version");
    let mut pdf_two_stream = stream_framing.to_vec();
    if let Some(major) = pdf_two_stream.get_mut(5) {
        *major = b'2';
    }
    if let Some(minor) = pdf_two_stream.get_mut(7) {
        *minor = b'0';
    }
    assert!(supported_pdf_cross_reference(&pdf_two_stream),
        "PDF 2.0 still admits exact xref-stream framing");
}

#[cfg(unix)]
#[test]
fn fake_pdf_trailers_and_retargeted_xrefs_cannot_reach_a_printer() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    let bytes = real_pdf();
    let mut old_table = bytes.clone();
    if let Some(minor) = old_table.get_mut(7) {
        *minor = b'0';
    }
    assert!(supported_pdf_cross_reference(&old_table),
        "PDF 1.0 still admits legacy table-based framing");
    let xref_marker = b"startxref\n";
    let locator = bytes.windows(xref_marker.len())
        .rposition(|window| window == xref_marker)
        .expect("native vector PDF has an xref locator");
    let number_start = locator.checked_add(xref_marker.len())
        .expect("locator index within input");
    let number_end = number_start.checked_add(bytes.get(number_start..)
        .expect("digits follow native locator").iter()
        .position(|byte| *byte == b'\n').expect("xref digit line"))
        .expect("digit line end within input");
    let rewritten_offset = |replacement: &[u8]| {
        [bytes.get(..number_start).expect("locator prefix"), replacement,
            bytes.get(number_end..).expect("locator suffix")].concat()
    };
    // The old admission logic accepted the first four of these inputs:
    // they have a supported header and terminal EOF, but no usable xref.
    let bad_sources = [
        b"%PDF-1.7\n%%EOF\n".to_vec(),
        b"%PDF-1.7\nstartxref\n0\n%%EOF\n".to_vec(),
        b"%PDF-1.7\nxref\nstartxref\n99999\n%%EOF\n".to_vec(),
        b"%PDF-1.7\nxref\nstartxref\nabc\n%%EOF\n".to_vec(),
        rewritten_offset(b"0"),
        rewritten_offset(b"999999999999999999999999"),
        rewritten_offset(b"12junk"),
    ];
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS fixture");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");
    for candidate in bad_sources {
        fs::write(&pdf, &candidate).expect("write selected candidate");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(),
            "malformed xref must fail source admission");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "malformed xref must refuse before subprocess launch");
        assert!(!spooled.exists(), "no spool side effect");
        assert_eq!(fs::read(&pdf).expect("source unchanged"), candidate);
    }
    // PDF 1.5+ may use an indirect cross-reference *stream* rather than
    // an ASCII xref table; its offset form remains eligible for admission.
    let stream_prefix = [
        b"%PDF-1.7\n1 0 obj\n<< /Type /XRef >>\n".as_slice(),
        b"stream\n\x00\nendstream\nendobj\n".as_slice(),
    ].concat();
    let stream_framing = [
        stream_prefix.as_slice(),
        format!("startxref\n{}\n%%EOF\n", b"%PDF-1.7\n".len()).as_bytes(),
    ].concat();
    assert!(supported_pdf_cross_reference(&stream_framing),
        "indirect xref-stream locator is an admitted framing form");
    assert_legacy_xref_stream_refused(
        &stream_framing, &pdf, &fake_spool, &spooled,
    );
    let ordinary_object = stream_framing.windows(b"/Type /XRef".len())
        .position(|part| part == b"/Type /XRef")
        .expect("xref stream dictionary type");
    let mut wrong_type = stream_framing.clone();
    let type_value = ordinary_object.checked_add(b"/Type /".len())
        .expect("dictionary type index");
    wrong_type.get_mut(type_value..)
        .and_then(|tail| tail.get_mut(..b"XRef".len()))
        .expect("four-letter type value").copy_from_slice(b"Page");
    assert!(!supported_pdf_cross_reference(&wrong_type),
        "a normal indirect page object is not an xref stream");
    let mut wrong_name = stream_framing;
    drop(wrong_name.splice(type_value..type_value + 4,
        b"XRefShadow".iter().copied()));
    assert!(!supported_pdf_cross_reference(&wrong_name),
        "a longer PDF name cannot impersonate the exact /XRef type");
    fs::write(&pdf, &wrong_name).expect("selected invalid stream type");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert!(selected_pdf(&pdf).is_err(), "invalid type refused");
    assert!(submit_selected(&request, &fake_spool).is_err(),
        "invalid stream type must refuse before CUPS process launch");
    assert!(!spooled.exists(), "no fake spool side effect");
    assert_eq!(fs::read(&pdf).expect("source unchanged"), wrong_name);
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_stream_tokens_cannot_hide_in_comments_or_string_literals() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS fixture");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");
    // Valid xref stream dictionaries may include comments, nested maps,
    // parenthesized strings and CRLF stream boundaries.
    let nested_stream = [
        "%PDF-1.7\n1 0 obj\n<< /Note (/Type /Page)\n",
        " /Type % ordinary PDF dictionary comment\n /XRef",
        " /DecodeParms << /Type /Page >> >>\n",
        "stream\r\n\u{0000}\nendstream\nendobj\n",
        "startxref\n9\n%%EOF\n",
    ].concat().into_bytes();
    assert!(supported_pdf_cross_reference(&nested_stream),
        "legitimate nested xref stream dictionary remains admitted");
    let commented_tail = String::from_utf8(nested_stream)
        .expect("synthetic xref stream is UTF-8")
        .replace("endstream\nendobj\n",
            "endstream\n% accepted comment\nendobj\n% endobj ignored\n")
        .into_bytes();
    assert!(supported_pdf_cross_reference(&commented_tail),
        "comments between stream and object terminators stay eligible");
    // A byte substring is not proof of an xref stream dictionary. Its
    // /Type must be an actual top-level name, and stream an actual keyword.
    for fake_body in [
        "1 0 obj\n/Type /XRef\nstream\n",
        "1 0 obj\n<< /Memo (/Type /XRef) >>\nstream\n",
        "1 0 obj\n<< % /Type /XRef\n /Type /Page >>\nstream\n",
        "1 0 obj\n<< /Other << /Type /XRef >> >>\nstream\n",
        "1 0 obj\n<< /List [/Type /XRef] >>\nstream\n",
        "1 0 obj\n<< /Type /XRef >>\ndownstream\n",
        "1 0 obj\n<< /Type /XRef >>\n%stream\n",
        // A plausible header alone is not a complete terminal stream.
        "1 0 obj\n<< /Type /XRef >>\nstream\n",
        "1 0 obj\n<< /Type /XRef >>\nstream\n\0\nendobj\n",
        "1 0 obj\n<< /Type /XRef >>\nstream\n\0\nendstream\n",
        "1 0 obj\n<< /Type /XRef >>\nstream\n\0\nendstreamX\nendobj\n",
        "1 0 obj\n<< /Type /XRef >>\nstream\n\0\nendstream\nendobjFake\n",
        "1 0 obj\n<< /Type /XRef >>\nstream\n\0\nendstream\nendobj\njunk\n",
    ] {
        let fake = format!(
            "%PDF-1.7\n{fake_body}startxref\n9\n%%EOF\n"
        ).into_bytes();
        assert!(!supported_pdf_cross_reference(&fake),
            "not an actual xref stream: {fake_body:?}");
        fs::write(&pdf, &fake).expect("save fake xref stream");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "fake stream refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "fake stream cannot reach CUPS");
        assert!(!spooled.exists(), "no fake spool side effect");
        assert_eq!(fs::read(&pdf).expect("source unchanged"), fake);
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_tables_require_a_real_subsection_and_trailer() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS fixture");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");
    let plausible = b"%PDF-1.4\nxref\n0 1\n0000000000 65535 f \n\
        trailer\n<< /Size 1 >>\nstartxref\n9\n%%EOF\n";
    assert!(supported_pdf_cross_reference(plausible),
        "traditional table framing remains eligible for PDF 1.4");
    let multi_section = [
        "%PDF-1.7\nxref\n0 1\n0000000000 65535 f \n",
        "1 1\n0000000044 00000 n \n",
        "trailer\n<< /Size 2 /Root 1 0 R >>\n",
        "startxref\n9\n%%EOF\n",
    ].concat();
    assert!(supported_pdf_cross_reference(multi_section.as_bytes()),
        "multiple table subsections with numbered entries remain eligible");
    assert!(supported_pdf_cross_reference(&real_pdf()),
        "native physical-page PDF xref table remains eligible");
    let with_comment_and_nested = concat!(
        "%PDF-1.4\nxref\n0 1\n0000000000 65535 f \n",
        "trailer\n<< /Meta << /Memo (literal >> text) >> /Size 1 >>\n",
        "% allowed PDF comment\nstartxref\n9\n%%EOF\n",
    ).as_bytes();
    assert!(supported_pdf_cross_reference(with_comment_and_nested),
        "balanced trailer metadata and terminal comments remain admissible");
    let compact_size = b"%PDF-1.4\nxref\n0 1\n0000000000 65535 f \n\
        trailer\n<< /Size 1>>\nstartxref\n9\n%%EOF\n";
    assert!(supported_pdf_cross_reference(compact_size),
        "compact numeric Size stays a valid trailer value");
    let compact_name = compact_size.windows(b"<< /Size".len())
        .position(|word| word == b"<< /Size")
        .expect("compact Size fixture has named trailer");
    let compact_name_bytes = [
        compact_size.get(..compact_name + 2).expect("first delimiter"),
        compact_size.get(compact_name + 3..).expect("remaining name"),
    ].concat();
    assert!(supported_pdf_cross_reference(&compact_name_bytes),
        "a trailer dictionary may attach its first name to <<");
    for fake_body in [
        "xref\n",
        "xref\ntrailer\n<< /Size 1 >>\n",
        "xref\n0 1\ninvalid 00000 n\ntrailer\n<< /Size 1 >>\n",
        "xref\n0 2\n0000000000 65535 f \ntrailer\n<< >>\n",
        "xref\n0 1\n0000000000 65535 z \ntrailer\n<< >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1\n",
        "xref\n0 1\n0000000000 65535 f \n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Root 1 0 R >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size x >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size -1 >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 0 >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 /Size 2 >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 >>\nINJECTED\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 >>\n/Size 2 >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Meta << /Size 1 >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Meta << /Size 1 >> >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Memo (x) >>\n% /Size 1\n",
    ] {
        let fake = format!(
            "%PDF-1.7\n{fake_body}startxref\n9\n%%EOF\n"
        ).into_bytes();
        assert!(!supported_pdf_cross_reference(&fake),
            "not a framed xref table: {fake_body:?}");
        fs::write(&pdf, &fake).expect("save fake table");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "invalid table refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "invalid table cannot reach CUPS");
        assert!(!spooled.exists(), "no fake spool side effect");
        assert_eq!(fs::read(&pdf).expect("source unchanged"), fake);
    }
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
    for invalid in [b"%PDF-1.8\n%%EOF".as_slice(),
        b"%PDF-2.1\n%%EOF", b"%PDF-1.4fake\n%%EOF"] {
        fs::write(&pdf, invalid).expect("save malformed selected PDF");
        let request = PrintRequest {
            pdf: pdf.clone(),
            printer: "Queue_1".into(),
            dry_run: false,
            expected_sha256: None,
        };
        assert!(submit_selected(&request, &script).is_err(),
            "unsupported PDF header cannot reach the spooler");
        assert!(!copied.exists() && !arguments.exists(),
            "refused framing cannot launch fake CUPS");
        assert_eq!(fs::read(&pdf).expect("malformed source retained"),
            invalid, "refusal must never mutate selected bytes");
    }
    fs::write(&pdf, &bytes).expect("restore native PDF");
    let digest = sha256_profile_bytes(&bytes).bytes();
    let dry_request = PrintRequest {
        pdf: pdf.clone(),
        printer: "Queue_1".into(),
        dry_run: true,
        expected_sha256: Some(digest),
    };
    assert_eq!(submit_selected(&dry_request, &script)
        .expect("bounded PDF checked without spooling"), digest);
    assert!(!copied.exists(), "dry-run never invokes lp");
    let print_request = PrintRequest {
        dry_run: false,
        ..dry_request
    };
    let invalid_expected = PrintRequest {
        expected_sha256: Some([0u8; 32]),
        ..print_request.clone()
    };
    assert!(submit_selected(&invalid_expected, &script).is_err(),
        "a mismatched reviewed checksum refuses before spawning lp");
    assert!(!copied.exists(), "invalid digest must never spool");
    assert_eq!(submit_selected(&print_request, &script)
        .expect("fake local CUPS acceptance"), digest);
    assert_eq!(fs::read(&copied).expect("spooled bytes"), bytes);
    assert_eq!(
        fs::read_to_string(&arguments).expect("arguments"),
        "-d\nQueue_1\n-\n",
        "no command option injection"
    );

    let different_valid_pdf = real_pdf_with_marks(true);
    fs::write(&pdf, &different_valid_pdf)
        .expect("new structurally valid but altered selected PDF");
    assert!(submit_selected(&print_request, &script).is_err(),
        "stale checksum must refuse a changed valid PDF");
    assert_eq!(fs::read(&copied).expect("old spool unchanged"), bytes);
    fs::write(&script, "#!/bin/sh\nexit 7\n")
        .expect("replace fake spool with refusing transport");
    assert!(submit_with(&script, "Queue_1", &selected).is_err());
    let linked = directory.join("linked.pdf");
    symlink(&pdf, &linked).expect("linked fixture");
    assert!(selected_pdf(&linked).is_err(), "source links fail closed");
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}
