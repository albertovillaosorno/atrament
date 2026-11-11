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

// Keep fake CUPS rejection checks identical across backward xref chains.
// The candidate must remain byte-for-byte unchanged and the fake queue
// must never be invoked. The fixture is fail-fast on its own I/O.
#[cfg(unix)]
fn assert_fake_spool_rejected(
    pdf: &Path, request: &PrintRequest, fake_spool: &Path,
    spooled: &Path, candidate: &[u8],
) {
    fs::write(pdf, candidate).expect("write candidate PDF");
    assert!(submit_selected(request, fake_spool).is_err(),
        "invalid xref cannot reach a printer");
    assert!(!spooled.exists(), "no fake spool side effect");
    assert_eq!(fs::read(pdf).expect("unchanged source PDF"), candidate);
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

// Minimal externally readable one-page PDF 1.5, with a real binary xref
// stream containing free object zero and exact offsets for objects 1..4.
// Unlike the framing-only probes below, pdfinfo independently reads this
// construction as a one-page Letter PDF with no repair warnings.
fn one_page_xref_stream_pdf(version: &str, index: &str) -> Vec<u8> {
    let mut bytes = format!("%PDF-{version}\n").into_bytes();
    bytes.extend_from_slice(b"%\xd0\xd4\xc5\xd8\n");
    let mut offsets = [0usize; 5];
    for (object, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Count 1 /Kids [3 0 R] >>"),
        (3, "<< /Type /Page /Parent 2 0 R \
             /MediaBox [0 0 612 792] >>"),
    ] {
        *offsets.get_mut(object)
            .expect("known fixture object number") = bytes.len();
        bytes.extend_from_slice(format!(
            "{object} 0 obj\n{body}\nendobj\n"
        ).as_bytes());
    }
    offsets[4] = bytes.len();
    let mut rows = Vec::with_capacity(35);
    for (object, offset) in offsets.iter().enumerate() {
        rows.push(u8::from(object != 0));
        rows.extend_from_slice(&u32::try_from(*offset)
            .expect("tiny object offset fits u32").to_be_bytes());
        rows.extend_from_slice(&(if object == 0 { 65_535u16 } else { 0 })
            .to_be_bytes());
    }
    bytes.extend_from_slice(format!(
        "4 0 obj\n<< /Type /XRef /Size 5 /W [1 4 2]{index} \
         /Root 1 0 R /Length {} >>\nstream\n", rows.len(),
    ).as_bytes());
    bytes.extend_from_slice(&rows);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    bytes.extend_from_slice(format!(
        "startxref\n{}\n%%EOF\n", offsets[4],
    ).as_bytes());
    bytes
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
        vec!["--pdf", "--dry-run", "--printer", "queue"],
        vec!["--pdf", "--help", "--printer", "queue"],
        vec!["--pdf", "--printer", "--printer", "queue"],
        vec!["--pdf", "-document.pdf", "--printer", "queue"],
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
    let explicit_dash_basename = [
        "--pdf", "./--dry-run", "--printer", "queue",
    ].map(str::to_owned);
    assert_eq!(parse_args(&explicit_dash_basename)
        .expect("explicitly prefixed dash basename is a path")
        .expect("explicit print request").pdf, PathBuf::from("./--dry-run"));
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
        b"%PDF-1.7\n1 0 obj\n".as_slice(),
        b"<< /Type /XRef /Size 2 /Index [1 1] \
          /Length 1 /W [1 0 0] >>\n".as_slice(),
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
        " /Size 2 /Index [1 1] /Length 1 /W [1 0 0] \
          /DecodeParms << /Type /Page >> >>\n",
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
        concat!(
            "1 0 obj\n<< /Other << /Type /XRef /Size 2 ",
            "/Length 1 /W [1 0 0] >> >>\nstream\n",
        ),
        "1 0 obj\n<< /List [/Type /XRef] >>\nstream\n",
        concat!(
            "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 > >>\n",
            "stream\n\0\nendstream\nendobj\n",
        ),
        concat!(
            "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 ] >>\n",
            "stream\n\0\nendstream\nendobj\n",
        ),
        "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 /W [1 0 0] >>\ndownstream\n",
        "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 /W [1 0 0] >>\n%stream\n",
        // A plausible header alone is not a complete terminal stream.
        "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 /W [1 0 0] >>\nstream\n",
        concat!(
            "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 ",
            "/W [1 0 0] >>\nstream\n\0\nendobj\n",
        ),
        concat!(
            "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 ",
            "/W [1 0 0] >>\nstream\n\0\nendstream\n",
        ),
        concat!(
            "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 /W [1 0 0] >>\n",
            "stream\n\0\nendstreamX\nendobj\n",
        ),
        concat!(
            "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 /W [1 0 0] >>\nstream\n",
            "\0\nendstream\nendobjFake\n",
        ),
        concat!(
            "1 0 obj\n<< /Type /XRef /Size 2 /Length 1 /W [1 0 0] >>\nstream\n",
            "\0\nendstream\nendobj\njunk\n",
        ),
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
        "1 1\n0000000000 65535 f \n",
        "trailer\n<< /Size 2 /Root 1 0 R >>\n",
        "startxref\n9\n%%EOF\n",
    ].concat();
    assert!(supported_pdf_cross_reference(multi_section.as_bytes()),
        "multiple table subsections with numbered entries remain eligible");
    let high_section = concat!(
        "%PDF-1.4\nxref\n100 1\n0000000000 65535 f \n",
        "trailer\n<< /Size 101 >>\nstartxref\n9\n%%EOF\n",
    );
    assert!(supported_pdf_cross_reference(high_section.as_bytes()),
        "trailer Size covers highest subsection entry");

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
        concat!(
            "xref\n0 2\n0000000000 65535 f \n",
            "0000000000 00000 n \ntrailer\n<< /Size 1 >>\n",
        ),
        "xref\n100 1\n0000000044 00000 n \ntrailer\n<< /Size 100 >>\n",
        "xref\n0 1\n9999999999 00000 n \ntrailer\n<< /Size 1 >>\n",
        "xref\n0 1\n0000000000 00000 n \ntrailer\n<< /Size 1 >>\n",
        "xref\n0 1\n9999999999 65535 f \ntrailer\n<< /Size 1 >>\n",
        "xref\n0 1\n0000000000 99999 f \ntrailer\n<< /Size 1 >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 > >>\n",
        "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 1 ] >>\n",
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
fn xref_stream_advertised_size_must_be_bounded_and_unique() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS fixture");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");
    let framed = |dictionary: &str| format!(
        "%PDF-1.7\n1 0 obj\n<< /Length 1 /W [1 0 0] \
         /Index [1 1] {dictionary} >>\n\
         stream\n\0\nendstream\nendobj\nstartxref\n9\n%%EOF\n",
    ).into_bytes();
    for valid in [
        framed("/Type /XRef /Size 2"),
        framed(&format!("/Type /XRef /Size {MAX_SUPPORTED_XREF_OBJECTS}")),
        framed("/Meta << /Size 9999999999 >> /Size 2 /Type /XRef"),
    ] {
        assert!(supported_pdf_cross_reference(&valid),
            "valid bounded top-level /Size remains eligible");
    }
    for dictionary in [
        "/Type /XRef",
        "/Type /XRef /Size 0",
        "/Type /XRef /Size -2",
        "/Type /XRef /Size 9999999999",
        "/Type /XRef /Size 2 /Size 3",
        "/Type /XRef /Meta << /Size 2 >>",
        "/Type /XRef /Meta [/Size 2]",
        "/Type /XRef /Size 2.5",
        "/Type /XRef /Size 2oops",
        "/Type /XRef /Size +2",
    ] {
        let invalid = framed(dictionary);
        assert!(!supported_pdf_cross_reference(&invalid),
            "invalid xref-stream object count must refuse: {dictionary}");
        fs::write(&pdf, &invalid).expect("selected malformed PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(),
            "bad stream /Size must fail selected-file admission");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "bad stream /Size cannot contact CUPS");
        assert!(!spooled.exists(), "fake CUPS was never invoked");
        assert_eq!(fs::read(&pdf).expect("source unchanged"), invalid);
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_stream_object_identity_must_fit_declared_size() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS script");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let framed = |object: &str, generation: &str, size: usize| {
        format!(
            "%PDF-1.7\n{object} {generation} obj\n\
             << /Type /XRef /Size {size} /Index [{object} 1] \
             /Length 1 /W [1 0 0] >>\nstream\n\0\n\
             endstream\nendobj\nstartxref\n9\n%%EOF\n",
        ).into_bytes()
    };
    for valid in [
        framed("1", "0", 2),
        framed("2", "65535", 3),
        framed("999", "0", 1000),
    ] {
        assert!(supported_pdf_cross_reference(&valid),
            "bounded indirect stream object remains eligible");
    }
    for (object, generation, size) in [
        ("0", "0", 2),
        ("1", "0", 1),
        ("2", "0", 2),
        ("9999999999", "0", 2),
        ("1", "65536", 2),
        ("1", "9999999999", 2),
    ] {
        let invalid = framed(object, generation, size);
        assert!(!supported_pdf_cross_reference(&invalid),
            "cross-reference stream object must fit Size and generation");
        fs::write(&pdf, &invalid).expect("selected malformed PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "invalid stream identity refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "invalid stream identity cannot contact CUPS");
        assert!(!spooled.exists(), "fake CUPS was not contacted");
        assert_eq!(fs::read(&pdf).expect("source unchanged"), invalid);
    }
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_stream_length_must_match_selected_raw_bytes() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS script");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let framed = |dictionary: &str, data: &[u8]| [
        format!(
            "%PDF-1.7\n1 0 obj\n\
             << /Type /XRef /Size 2 /Index [1 1] \
             /W [1 0 0] {dictionary} >>\nstream\n",
        ).into_bytes(),
        data.to_vec(),
        b"\nendstream\nendobj\nstartxref\n9\n%%EOF\n".to_vec(),
    ].concat();
    for valid in [
        framed("/Length 1", b"\0"),
        framed("/Length 3 /Filter /FlateDecode", b"xyz"),
        framed("/Meta << /Length 99999999 >> /Length 1", b"\0"),
        framed("/Length 16 /Filter /FlateDecode", b"\0\nendstream\nxxxx"),
    ] {
        assert!(supported_pdf_cross_reference(&valid),
            "exact bounded raw stream data remains eligible");
    }
    for (dictionary, data) in [
        ("", b"\0".as_slice()),
        ("/Length 0", b"\0"),
        ("/Length -1", b"\0"),
        ("/Length 25000000", b"\0"),
        ("/Length 1 /Length 1", b"\0"),
        ("/Meta << /Length 1 >>", b"\0"),
        ("/Length 1x", b"\0"),
        ("/Length 2", b"\0"),
        ("/Length 1", b"xyz"),
    ] {
        let invalid = framed(dictionary, data);
        assert!(!supported_pdf_cross_reference(&invalid),
            "xref-stream length must be explicit and exact: {dictionary}");
        fs::write(&pdf, &invalid).expect("write malformed stream");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "malformed length refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "malformed length cannot reach CUPS");
        assert!(!spooled.exists(), "no fake CUPS invocation");
        assert_eq!(fs::read(&pdf).expect("selected bytes unchanged"), invalid);
    }
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn unfiltered_xref_stream_length_must_cover_all_indexed_rows() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("truncated-xref.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS script");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    let original = one_page_xref_stream_pdf("1.5", " /Index [0 5]");
    assert!(supported_pdf_cross_reference(&original),
        "complete five-row xref stream must remain eligible");
    let mut short = original.clone();
    let length = short.windows(b"/Length 35".len())
        .position(|part| part == b"/Length 35")
        .expect("known xref length");
    short[length..length + b"/Length 35".len()]
        .copy_from_slice(b"/Length 34");
    let data = short.windows(b"stream\n".len())
        .position(|part| part == b"stream\n")
        .expect("xref stream start") + b"stream\n".len();
    let _removed_byte = short.remove(data + 34);
    assert!(!supported_pdf_cross_reference(&short),
        "the final unfiltered row cannot lose a byte");
    assert_fake_spool_rejected(
        &pdf, &request, &fake_spool, &spooled, &short);
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
// Test-only, fully valid zlib stream with an uncompressed Deflate block.
// This avoids test dependencies while proving an encoded PDF is readable.
fn zlib_stored_block(bytes: &[u8]) -> Vec<u8> {
    let length = u16::try_from(bytes.len()).expect("tiny xref fixture");
    let mut encoded = vec![0x78, 0x01, 0x01];
    encoded.extend_from_slice(&length.to_le_bytes());
    encoded.extend_from_slice(&(!length).to_le_bytes());
    encoded.extend_from_slice(bytes);
    let (mut a, mut b) = (1u32, 0u32);
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    encoded.extend_from_slice(&((b << 16) | a).to_be_bytes());
    encoded
}

#[cfg(unix)]
#[test]
fn compressed_xref_stream_accepts_encoded_length_and_validates_filter() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("compressed.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake CUPS script");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    let raw = one_page_xref_stream_pdf("1.5", " /Index [0 5]");
    let length_key = b"/Length 35";
    let length_at = raw.windows(length_key.len())
        .position(|part| part == length_key)
        .expect("known uncompressed stream length");
    let data = raw.windows(b"stream\n".len())
        .position(|part| part == b"stream\n")
        .expect("xref data start") + b"stream\n".len();
    let compressed_rows = zlib_stored_block(raw.get(data..data + 35)
        .expect("five original xref rows"));
    assert_ne!(compressed_rows.len(), 35,
        "encoded length differs from uncompressed row length");
    let mut compressed = raw;
    drop(compressed.splice(data..data + 35, compressed_rows.iter().copied()));
    let filter_and_length = format!(
        "/Filter /FlateDecode /Length {}", compressed_rows.len(),
    );
    drop(compressed.splice(length_at..length_at + length_key.len(),
        filter_and_length.bytes()));
    assert!(supported_pdf_cross_reference(&compressed),
        "a framed, genuinely Flate-encoded xref stream remains eligible");
    fs::write(&pdf, &compressed).expect("write valid compressed PDF");
    assert_eq!(submit_selected(&request, &fake_spool)
        .expect("compressed PDF reaches fake CUPS"),
        sha256_profile_bytes(&compressed).bytes());
    assert_eq!(fs::read(&spooled).expect("captured PDF"), compressed);
    fs::remove_file(&spooled).expect("clear capture");

    // A fake or repeated top-level filter must not disguise a short raw
    // stream. The nested Filter name cannot bypass the row-length check.
    for filter in [
        "/Filter 123", "/Filter []", "/Filter [/FlateDecode 2]",
        "/Filter /FlateDecode /Filter /FlateDecode",
        "/Filter /FlateDecode /Fil#74er /FlateDecode",
        "/Filter /", "/Filter [ ]",
    ] {
        let candidate = compressed
            .windows(filter_and_length.len())
            .position(|part| part == filter_and_length.as_bytes())
            .map(|at| {
                let mut changed = compressed.clone();
                drop(changed.splice(at..at + "/Filter /FlateDecode".len(),
                    filter.bytes()));
                changed
            }).expect("compressed filter dictionary");
        assert!(!supported_pdf_cross_reference(&candidate),
            "malformed Filter must refuse: {filter}");
        assert_fake_spool_rejected(
            &pdf, &request, &fake_spool, &spooled, &candidate);
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_stream_requires_one_bounded_top_level_width_array() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS script");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let framed = |dictionary: &str, length: usize| format!(
        "%PDF-1.7\n1 0 obj\n\
         << /Type /XRef /Size 2 /Index [1 1] \
         /Length {length} {dictionary} >>\n\
         stream\n{}\nendstream\nendobj\nstartxref\n9\n%%EOF\n",
        "\0".repeat(length),
    ).into_bytes();
    for (valid, length) in [
        ("/W [1 2 1]", 4),
        ("/W[0 2 0]", 2),
        ("/Meta << /W [9] >> /W [1% comment\n 2 1]", 4),
    ] {
        let source = framed(valid, length);
        assert!(supported_pdf_cross_reference(&source),
            "bounded stream widths remain admitted: {valid:?}");
    }
    for invalid in [
        "", "/W []", "/W [1 2]", "/W [1 2 3 4]",
        "/W [0 0 0]", "/W [-1 2 1]", "/W [+1 2 1]",
        "/W [1 2.0 1]", "/W [1 2 banana]", "/W (1 2 1)",
        "/W [1 2 1] /W [1 2 1]", "/Meta << /W [1 2 1] >>",
        "/W [1 2 999999]",
    ] {
        let source = framed(invalid, 1);
        assert!(!supported_pdf_cross_reference(&source),
            "invalid or hidden xref widths must refuse: {invalid:?}");
        fs::write(&pdf, &source).expect("write invalid selected PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(),
            "invalid widths fail selected-file admission");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "invalid widths cannot reach fake CUPS");
        assert!(!spooled.exists(), "no fake spool side effects");
        assert_eq!(fs::read(&pdf).expect("selected bytes unchanged"), source);
    }
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_stream_index_ranges_are_bounded_disjoint_pairs() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS script");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let framed = |index: &str, length: usize| format!(
        "%PDF-1.7\n1 0 obj\n\
         << /Type /XRef /Size 3 /W [1 0 0] /Length {length} {index} >>\n\
         stream\n{}\nendstream\nendobj\nstartxref\n9\n%%EOF\n",
        "\0".repeat(length),
    ).into_bytes();
    for (valid, length) in [
        ("", 3), ("/Index [1 1]", 1),
        ("/Index[0 2 2 1]", 3),
        ("/Index [0% row comment\n 2]", 2),
        ("/Meta << /Index [9 9] >> /Index [1 1]", 1),
    ] {
        let candidate = framed(valid, length);
        assert!(supported_pdf_cross_reference(&candidate),
            "disjoint or omitted stream ranges remain admitted: {valid:?}");
    }
    for invalid in [
        "/Index []", "/Index [0]", "/Index [0 0]",
        "/Index [0 -1]", "/Index [0 +1]",
        "/Index [0 2 1]", "/Index [0 2 1 1]",
        "/Index [0 1 2 1]", "/Index [0 1]",
        "/Index [2 2]", "/Index [3 1]",
        "/Index [9999999999 1]", "/Index [0 9999999999]",
        "/Index [0 1] /Index [1 1]",
        "/Index (0 1)", "/Index [0 1 /Other]",
        "/Index [0 1.0]", "/Index [18446744073709551616 1]",
    ] {
        let candidate = framed(invalid, 1);
        assert!(!supported_pdf_cross_reference(&candidate),
            "malformed xref-stream index ranges refuse: {invalid:?}");
        fs::write(&pdf, &candidate).expect("write malformed PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "malformed index refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "malformed index cannot contact fake CUPS");
        assert!(!spooled.exists(), "no fake spool contact");
        assert_eq!(fs::read(&pdf).expect("selected bytes unchanged"),
            candidate);
    }
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn genuine_xref_stream_pdf_spools_exact_reviewed_bytes() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("one-page-stream.pdf");
    let spooled = directory.join("spooled.pdf");
    let script = directory.join("lp.sh");
    fs::write(&script, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake CUPS script");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");

    // /Index is optional (implicit [0 /Size]), or may provide an explicit
    // nonoverlapping partition covering all five real indirect objects.
    for (version, index) in [
        ("1.5", " /Index [0 5]"),
        ("1.5", ""),
        ("1.5", " /Index [0 2 2 3]"),
        ("2.0", " /Index [0 5]"),
    ] {
        let bytes = one_page_xref_stream_pdf(version, index);
        fs::write(&pdf, &bytes).expect("write actual single-page PDF");
        assert_eq!(selected_pdf(&pdf).expect("admit genuine PDF"), bytes);
        let digest = sha256_profile_bytes(&bytes).bytes();
        if version == "1.5" && index == " /Index [0 5]" {
            assert_eq!(bytes.len(), 353, "independently parsed PDF size");
            assert_eq!(sha256_hex(digest).expect("format digest"),
                concat!(
                    "50c8e019842de806caf6570923a244a1",
                    "075fb6199d05f0e5de249420807e3d80",
                ));
        }
        let dry_run = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: true, expected_sha256: Some(digest),
        };
        assert_eq!(submit_selected(&dry_run, &script)
            .expect("validate genuine PDF without contact"), digest);
        assert!(!spooled.exists(), "review must not spool");
        let submit_request = PrintRequest {
            dry_run: false, ..dry_run
        };
        assert_eq!(submit_selected(&submit_request, &script)
            .expect("exact fake CUPS submission"), digest);
        assert_eq!(fs::read(&spooled).expect("captured fake-CUPS bytes"),
            bytes, "no source mutation or PDF rewriting");
        assert_eq!(fs::read(&pdf).expect("original document"), bytes);
        fs::remove_file(&spooled).expect("reset fake spool capture");
    }
    // The terminal stream object must identify itself in an explicit
    // /Index. Merely staying within /Size is insufficient: readers cannot
    // locate the xref stream from its own claimed object table otherwise.
    let missing_self = one_page_xref_stream_pdf("1.5", " /Index [0 4]");
    assert!(!supported_pdf_cross_reference(&missing_self),
        "a terminal xref stream cannot omit its own object from /Index");
    let invalid_request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert_fake_spool_rejected(
        &pdf, &invalid_request, &script, &spooled, &missing_self);

    // Cross-reference streams arrived in PDF 1.5. A document with this
    // exact binary structure and a 1.4 header may be repairable in a
    // viewer, but the local print preflight intentionally refuses it.
    let legacy = one_page_xref_stream_pdf("1.4", " /Index [0 5]");
    assert!(!supported_pdf_cross_reference(&legacy));
    fs::write(&pdf, &legacy).expect("write legacy-version stream");
    let invalid = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert!(selected_pdf(&pdf).is_err(), "legacy xref-stream refused");
    assert!(submit_selected(&invalid, &script).is_err(),
        "legacy stream cannot contact fake CUPS");
    assert!(!spooled.exists(), "no fake spool contact");
    assert_eq!(fs::read(&pdf).expect("original refused bytes"), legacy);
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_object_zero_is_always_the_permanent_free_head() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS executable");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let valid = concat!(
        "%PDF-1.7\nxref\n0 2\n0000000001 65535 f \n",
        "0000000000 00001 f \ntrailer\n<< /Size 2 >>\n",
        "startxref\n9\n%%EOF\n",
    );
    assert!(supported_pdf_cross_reference(valid.as_bytes()),
        "free object zero can name an in-range free-list successor");

    let header = b"%PDF-1.7\n0 0 obj\nnull\nendobj\n";
    let bad_in_use = [header.as_slice(), format!(
        "xref\n0 1\n{offset:010} 00000 n \n\
         trailer\n<< /Size 1 >>\n\
         startxref\n{xref}\n%%EOF\n",
        offset = b"%PDF-1.7\n".len(), xref = header.len(),
    ).as_bytes()].concat();
    let bad_free = b"%PDF-1.7\nxref\n0 1\n0000000000 00000 f \n\
        trailer\n<< /Size 1 >>\nstartxref\n9\n%%EOF\n".to_vec();
    for invalid in [bad_in_use, bad_free] {
        assert!(!supported_pdf_cross_reference(&invalid),
            "object zero must be free with generation 65535");
        fs::write(&pdf, &invalid).expect("save invalid zero record");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "reserved object zero refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "reserved object zero cannot contact CUPS");
        assert!(!spooled.exists(), "fake CUPS not contacted");
        assert_eq!(fs::read(&pdf).expect("source retained"), invalid);
    }
    fs::remove_dir_all(&directory).expect("remove owned fixtures");
}

#[cfg(unix)]
#[test]
fn xref_subsection_count_is_unsigned_decimal_digits_only() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS executable");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let framed = |count: &str| format!(
        "%PDF-1.7\nxref\n0 {count}\n0000000000 65535 f \n\
         trailer\n<< /Size 1 >>\nstartxref\n9\n%%EOF\n",
    ).into_bytes();
    assert!(supported_pdf_cross_reference(&framed("1")),
        "canonical decimal xref count remains eligible");
    for count in ["+1", "+0001"] {
        let invalid = framed(count);
        assert!(!supported_pdf_cross_reference(&invalid),
            "prefixed xref subsection count must refuse: {count}");
        fs::write(&pdf, &invalid).expect("write forged table count");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "signed count refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "signed count cannot contact CUPS");
        assert!(!spooled.exists(), "no fake CUPS contact");
        assert_eq!(fs::read(&pdf).expect("source intact"), invalid);
    }
    let header = |subsection: &str| format!(
        "%PDF-1.7\nxref\n{subsection}\n0000000000 65535 f \n\
         trailer\n<< /Size 1 >>\nstartxref\n9\n%%EOF\n",
    ).into_bytes();
    for valid in ["0 1", "0\t1", "0   1"] {
        assert!(supported_pdf_cross_reference(&header(valid)),
            "complete horizontal-space header remains admitted");
    }
    // The xref keyword itself must also end before the first subsection.
    let inline_xref = b"%PDF-1.7\nxref 0 1\n0000000000 65535 f \n\
         trailer\n<< /Size 1 >>\nstartxref\n9\n%%EOF\n";
    assert!(!supported_pdf_cross_reference(inline_xref),
        "xref marker cannot share its subsection line");
    fs::write(&pdf, inline_xref).expect("write inline xref keyword");
    let inline_request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert!(submit_selected(&inline_request, &fake_spool).is_err());
    assert!(!spooled.exists(), "inline xref never reaches fake CUPS");
    assert_eq!(fs::read(&pdf).expect("unchanged inline PDF"),
        inline_xref);
    for invalid in ["0\n1", "0\r\n1", "0  \n1", "0 %comment\n1"] {
        let candidate = header(invalid);
        assert!(!supported_pdf_cross_reference(&candidate),
            "subsection object index and count must share one line");
        fs::write(&pdf, &candidate).expect("write malformed header");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "split header cannot contact CUPS");
        assert!(!spooled.exists(), "no fake CUPS effect");
        assert_eq!(fs::read(&pdf).expect("intact input"), candidate);
    }
    fs::remove_dir_all(&directory).expect("remove owned fixtures");
}

#[cfg(unix)]
#[test]
fn xref_table_entries_require_one_complete_record_line() {
    use std::iter;
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS executable");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let framed = |record: &str| format!(
        "%PDF-1.7\nxref\n0 1\n{record}\
         trailer\n<< /Size 1 >>\nstartxref\n9\n%%EOF\n",
    ).into_bytes();
    for valid in [
        "0000000000 65535 f \n",
        "0000000000 65535 f \r\n",
    ] {
        assert!(supported_pdf_cross_reference(&framed(valid)),
            "a complete record with one line ending remains eligible");
    }
    let header_and_entry_on_one_line =
        b"%PDF-1.7\nxref\n0 1 0000000000 65535 f \n\
          trailer\n<< /Size 1 >>\nstartxref\n9\n%%EOF\n".to_vec();
    let invalid_entries = [
        "0000000000\n65535 f \n",
        "0000000000 65535\nf \n",
        "0000000000 65535 f trailer\n",
        "0000000000 65535 f ",
        "0000000000 65535 f extra\n",
    ];
    for invalid in invalid_entries.into_iter().map(framed)
        .chain(iter::once(header_and_entry_on_one_line)) {
        assert!(!supported_pdf_cross_reference(&invalid),
            "xref record requires a separate complete table line");
        fs::write(&pdf, &invalid).expect("save malformed table");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "broken xref record refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "broken record cannot contact CUPS");
        assert!(!spooled.exists(), "no fake spool side effect");
        assert_eq!(fs::read(&pdf).expect("source unchanged"), invalid);
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn pdf_whitespace_in_xref_metadata_and_eof_is_admitted_exactly() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spooler executable");

    let native = real_pdf();
    let size = native.windows(b"/Size ".len())
        .position(|part| part == b"/Size ")
        .expect("native trailer declares a size");
    let number_start = size + b"/Size ".len();
    let number_end = number_start + native.get(number_start..)
        .expect("native decimal suffix").iter()
        .take_while(|byte| byte.is_ascii_digit()).count();
    assert_eq!(native.get(number_end), Some(&b' '),
        "space after native size");

    let mut before_number = native.clone();
    *before_number.get_mut(number_start - 1)
        .expect("separator before native size") = b'\0';
    let mut after_number = native.clone();
    *after_number.get_mut(number_end)
        .expect("separator after native size") = b'\0';
    let mut after_eof = native.clone();
    after_eof.push(b'\0');

    let mut stream = one_page_xref_stream_pdf("1.5", " /Index [0 5]");
    let width = stream.windows(b"/W [".len())
        .position(|part| part == b"/W [")
        .expect("xref stream has direct widths");
    *stream.get_mut(width + b"/W".len())
        .expect("stream width separator") = b'\0';

    for (label, candidate) in [
        ("NUL before numeric trailer value", before_number),
        ("NUL after numeric trailer value", after_number),
        ("NUL after final EOF", after_eof),
        ("NUL within stream dictionary", stream),
    ] {
        assert!(supported_pdf_cross_reference(&candidate),
            "PDF-defined whitespace admitted: {label}");
        fs::write(&pdf, &candidate).expect("write selected PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert_eq!(submit_selected(&request, &fake_spool)
            .expect("PDF whitespace admitted to fake CUPS"),
            sha256_profile_bytes(&candidate).bytes(), "{label}");
        assert_eq!(fs::read(&spooled).expect("fake spool bytes"),
            candidate, "exact selected bytes: {label}");
        fs::remove_file(&spooled).expect("clear fake spool");
    }

    // Vertical tab (0B) is ASCII whitespace but not PDF whitespace.
    // A forged numeric delimiter must not qualify for print admission.
    let mut vertical_tab = native;
    *vertical_tab.get_mut(number_end)
        .expect("native size trailing separator") = b'\x0b';
    assert!(!supported_pdf_cross_reference(&vertical_tab),
        "non-PDF vertical tab cannot delimit trailer size");
    fs::write(&pdf, &vertical_tab).expect("write invalid PDF");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert!(submit_selected(&request, &fake_spool).is_err(),
        "non-PDF separator must not reach fake CUPS");
    assert!(!spooled.exists(), "invalid PDF cannot contact fake CUPS");
    assert_eq!(fs::read(&pdf).expect("source unchanged"), vertical_tab);
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn startxref_decimal_line_uses_pdf_whitespace_not_vertical_tab() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");

    let native = real_pdf();
    let locator = native.windows(b"startxref\n".len())
        .rposition(|part| part == b"startxref\n")
        .expect("native PDF has a final startxref locator");
    let number_start = locator + b"startxref\n".len();
    let number_end = number_start + native.get(number_start..)
        .expect("native locator suffix").iter()
        .take_while(|byte| byte.is_ascii_digit()).count();
    assert_eq!(native.get(number_end), Some(&b'\n'),
        "decimal locator line ending");

    let mut leading_nul = native.clone();
    leading_nul.insert(number_start, b'\0');
    let mut trailing_nul = native.clone();
    trailing_nul.insert(number_end, b'\0');
    let mut both_nul = native.clone();
    both_nul.insert(number_end, b'\0');
    both_nul.insert(number_start, b'\0');
    for (label, candidate) in [
        ("NUL before startxref integer", leading_nul),
        ("NUL after startxref integer", trailing_nul),
        ("NUL on both sides of integer", both_nul),
    ] {
        assert!(supported_pdf_cross_reference(&candidate),
            "PDF-defined locator whitespace admitted: {label}");
        fs::write(&pdf, &candidate).expect("save admitted PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert_eq!(submit_selected(&request, &fake_spool)
            .expect("PDF locator whitespace reaches fake CUPS"),
            sha256_profile_bytes(&candidate).bytes(), "{label}");
        assert_eq!(fs::read(&spooled).expect("spooled selected PDF"),
            candidate, "source bytes unchanged: {label}");
        fs::remove_file(&spooled).expect("clear fake spool");
    }

    let mut vertical_tab = native;
    vertical_tab.insert(number_start, b'\x0b');
    assert!(!supported_pdf_cross_reference(&vertical_tab),
        "vertical tab is not legal PDF whitespace before locator number");
    fs::write(&pdf, &vertical_tab).expect("save malformed locator PDF");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert!(submit_selected(&request, &fake_spool).is_err(),
        "invalid locator delimiter must not reach CUPS");
    assert!(!spooled.exists(), "invalid locator cannot contact fake CUPS");
    assert_eq!(fs::read(&pdf).expect("selected bytes intact"), vertical_tab);
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[test]
fn xref_size_below_resource_ceiling_remains_admissible() {
    let sparse = format!(
        "%PDF-1.7\nxref\n0 1\n0000000000 65535 f \n\
         trailer\n<< /Size {MAX_SUPPORTED_XREF_OBJECTS} >>\n\
         startxref\n9\n%%EOF\n",
    );
    assert!(supported_pdf_cross_reference(sparse.as_bytes()),
        "accepted resource ceiling is inclusive for sparse object IDs");
}

#[cfg(unix)]
#[test]
fn oversized_xref_trailer_size_refuses_before_cups() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("huge-size.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let script = directory.join("fake-lp.sh");
    fs::write(&script, format!(
        "#!/bin/sh\nprintf contact > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700))
        .expect("make fake spooler executable");
    let malformed = concat!(
        "%PDF-1.7\nxref\n0 1\n0000000000 65535 f \n",
        "trailer\n<< /Size 9999999999 >>\n",
        "startxref\n9\n%%EOF\n",
    ).as_bytes();
    assert!(!supported_pdf_cross_reference(malformed),
        "gigantic advertised xref table must refuse on tiny input");
    fs::write(&pdf, malformed).expect("save malformed PDF");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert!(submit_selected(&request, &script).is_err(),
        "oversized xref cannot reach CUPS");
    assert!(!spooled.exists(), "fake CUPS never contacted");
    assert_eq!(fs::read(&pdf).expect("source intact"), malformed);
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn overlapping_xref_subsections_refuse_before_cups() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("overlap.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let script = directory.join("fake-lp.sh");
    fs::write(&script, format!(
        "#!/bin/sh\nprintf contact > '{}'\n", spooled.display(),
    )).expect("write fake local spooler");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700))
        .expect("make fake local spooler executable");
    for body in [
        concat!(
            "xref\n0 1\n0000000000 65535 f \n",
            "0 1\n0000000000 65535 f \n",
            "trailer\n<< /Size 1 >>\n",
        ),
        concat!(
            "xref\n0 2\n0000000000 65535 f \n",
            "0000000000 00001 f \n",
            "1 1\n0000000000 00001 f \n",
            "trailer\n<< /Size 2 >>\n",
        ),
    ] {
        let malformed = format!(
            "%PDF-1.7\n{body}startxref\n9\n%%EOF\n",
        ).into_bytes();
        assert!(!supported_pdf_cross_reference(&malformed),
            "duplicate subsection object indices must refuse");
        fs::write(&pdf, &malformed).expect("save malformed PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(submit_selected(&request, &script).is_err(),
            "overlapping PDF cannot reach spooler");
        assert!(!spooled.exists(), "fake CUPS never contacted");
        assert_eq!(fs::read(&pdf).expect("source intact"), malformed);
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[test]
fn xref_disjoint_subsections_can_leave_sparse_gaps() {
    let sections = concat!(
        "%PDF-1.4\nxref\n0 1\n0000000000 65535 f \n",
        "3 1\n0000000000 00000 f \n",
        "trailer\n<< /Size 4 >>\nstartxref\n9\n%%EOF\n",
    );
    assert!(supported_pdf_cross_reference(sections.as_bytes()),
        "ordered disjoint table subsections may skip object numbers");
}

#[test]
fn xref_free_chain_within_declared_size_is_admissible() {
    let free_chain = concat!(
        "%PDF-1.4\nxref\n0 2\n0000000001 65535 f \n",
        "0000000000 00001 f \ntrailer\n<< /Size 2 >>\n",
        "startxref\n9\n%%EOF\n",
    );
    assert!(supported_pdf_cross_reference(free_chain.as_bytes()),
        "free-list object indices within Size remain eligible");
}

#[cfg(unix)]
#[test]
fn xref_in_use_offsets_must_match_indirect_object_headers() {
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

    let object = b"%PDF-1.7\n1 0 obj\n(null)\nendobj\n";
    let object_offset = b"%PDF-1.7\n".len();
    let xref_offset = object.len();
    let framed = |index: usize, generation: usize, offset: usize| {
        [object.as_slice(), format!(
            "xref\n{index} 1\n{offset:010} {generation:05} n \n\
             trailer\n<< /Size 3 >>\nstartxref\n{xref_offset}\n%%EOF\n",
        ).as_bytes()].concat()
    };
    let valid = framed(1, 0, object_offset);
    assert!(supported_pdf_cross_reference(&valid),
        "an exact in-use object header remains an eligible table entry");
    for invalid in [
        framed(1, 0, xref_offset),
        framed(1, 0, object_offset + b"1 0 obj\n".len()),
        framed(2, 0, object_offset),
        framed(1, 1, object_offset),
    ] {
        assert!(!supported_pdf_cross_reference(&invalid),
            "in-use offset must point at the recorded object/generation");
        fs::write(&pdf, &invalid).expect("write malformed PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "bad object pointer refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "bad pointer cannot contact CUPS");
        assert!(!spooled.exists(), "no spool side effect");
        assert_eq!(fs::read(&pdf).expect("source intact"), invalid);
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_in_use_object_cannot_point_into_terminal_trailer() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS fixture");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");

    // The record points at a header-like literal in its own trailer,
    // not at an indirect object that precedes the terminal xref table.
    let prefix = b"%PDF-1.7\nxref\n1 1\n";
    let record = b"0000000000 00000 n \n";
    let trailer_prefix = b"trailer\n<< /Size 2 /Memo (";
    let fake_object_offset = prefix.len() + record.len()
        + trailer_prefix.len();
    let bytes = [
        prefix.as_slice(),
        format!("{fake_object_offset:010} 00000 n \n").as_bytes(),
        trailer_prefix.as_slice(),
        b"1 0 obj ) >>\nstartxref\n9\n%%EOF\n".as_slice(),
    ].concat();
    assert!(!supported_pdf_cross_reference(&bytes),
        "trailer text cannot impersonate an earlier indirect object");
    fs::write(&pdf, &bytes).expect("write selected forged PDF");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert!(selected_pdf(&pdf).is_err(), "forged object pointer refused");
    assert!(submit_selected(&request, &fake_spool).is_err(),
        "xref pointing into its own trailer cannot contact CUPS");
    assert!(!spooled.exists(), "no spool side effect");
    assert_eq!(fs::read(&pdf).expect("source remains intact"), bytes);
    assert!(supported_pdf_cross_reference(&real_pdf()),
        "native two-page output remains admissible");
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn malformed_pdf_name_escapes_refuse_in_table_and_stream_metadata() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake CUPS fixture");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    let samples = |name: &str| [
        format!(concat!(
            "%PDF-1.7\nxref\n0 1\n0000000000 65535 f \n",
            "trailer\n<< /Size 1 /{} /Value >>\n",
            "startxref\n9\n%%EOF\n",
        ), name).into_bytes(),
        one_page_xref_stream_pdf("1.7", &format!(" /{name} /Value")),
    ];
    for valid in ["CustomName", "Custom#20Name", "Custom#23Name"] {
        for candidate in samples(valid) {
            assert!(supported_pdf_cross_reference(&candidate), "{valid}");
            fs::write(&pdf, &candidate).expect("write valid PDF");
            let digest = submit_selected(&request, &fake_spool)
                .expect("valid PDF reaches explicit fake CUPS queue");
            assert_eq!(digest, sha256_profile_bytes(&candidate).bytes());
            assert_eq!(fs::read(&spooled).expect("spooled PDF"), candidate);
        }
    }
    fs::remove_file(&spooled).expect("clear fake spool");
    for invalid in ["Custom#", "Custom#2", "Custom#G0", "Custom#9g"] {
        for candidate in samples(invalid) {
            assert!(!supported_pdf_cross_reference(&candidate),
                "malformed key must refuse: {invalid}");
            fs::write(&pdf, &candidate).expect("write malformed PDF");
            assert!(selected_pdf(&pdf).is_err());
            assert!(submit_selected(&request, &fake_spool).is_err(),
                "invalid PDF must not contact CUPS");
            assert!(!spooled.exists(), "fake spool must remain untouched");
            assert_eq!(fs::read(&pdf).expect("source unchanged"),
                candidate);
        }
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture");
}

#[cfg(unix)]
#[test]
fn malformed_hex_metadata_refuses_before_spooling_table_or_stream() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake CUPS fixture");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    let samples = |hex: &str| [
        format!(concat!(
            "%PDF-1.7\nxref\n0 1\n0000000000 65535 f \n",
            "trailer\n<< /Size 1 /Meta {hex} >>\n",
            "startxref\n9\n%%EOF\n",
        ), hex = hex).into_bytes(),
        one_page_xref_stream_pdf("1.7", &format!(" /Meta {hex}")),
    ];
    // PDF hexadecimal strings admit an odd final nibble and PDF whitespace.
    for valid in ["<>", "<A>", "<a0 3F>", "<A0\n3f>"] {
        for candidate in samples(valid) {
            assert!(supported_pdf_cross_reference(&candidate),
                "valid PDF hex string {valid:?} must stay eligible");
            fs::write(&pdf, &candidate).expect("write valid PDF");
            assert_eq!(submit_selected(&request, &fake_spool)
                .expect("valid hex PDF admitted to fake CUPS"),
                sha256_profile_bytes(&candidate).bytes());
            assert_eq!(fs::read(&spooled).expect("exact fake-spooled PDF"),
                candidate, "fake CUPS must consume complete PDF bytes");
            fs::remove_file(&spooled).expect("clear fake spool");
        }
    }
    for invalid in ["<G0>", "<0G>", "<A/B>", "<00!>",
        "<00<01>", "<FF\u{000b}>"] {
        for candidate in samples(invalid) {
            assert!(!supported_pdf_cross_reference(&candidate),
                "malformed PDF hex string must refuse: {invalid:?}");
            fs::write(&pdf, &candidate).expect("write malformed PDF");
            assert!(selected_pdf(&pdf).is_err(),
                "invalid lexical metadata cannot be selected");
            assert!(submit_selected(&request, &fake_spool).is_err(),
                "invalid PDF must not contact CUPS");
            assert!(!spooled.exists(), "fake queue remains untouched");
            assert_eq!(fs::read(&pdf).expect("source still present"),
                candidate, "refusal must preserve selected bytes");
        }
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture");
}

#[cfg(unix)]
#[test]
fn xref_escaped_pdf_names_cannot_hide_duplicate_size_keys() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf contacted > '{}'\n", spooled.display(),
    )).expect("write fake CUPS fixture");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");

    // PDF name escapes decode to the same keys, not distinct metadata.
    let table = concat!(
        "%PDF-1.7\nxref\n0 1\n0000000000 65535 f \n",
        "trailer\n<< /Size 1 /S#69ze 999999999 >>\n",
        "startxref\n9\n%%EOF\n",
    ).as_bytes();
    let stream = concat!(
        "%PDF-1.7\n1 0 obj\n",
        "<< /Type /XRef /Size 2 /Si#7Ae 999999999 ",
        "/W [1 0 0] /Length 1 >>\n",
        "stream\n\0\nendstream\nendobj\n",
        "startxref\n9\n%%EOF\n",
    ).as_bytes();
    for bytes in [table, stream] {
        assert!(!supported_pdf_cross_reference(bytes),
            "PDF name escapes must not bypass duplicate Size refusal");
        fs::write(&pdf, bytes).expect("write forged PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "duplicate Size refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "alias Size cannot contact CUPS");
        assert!(!spooled.exists(), "no spool side effect");
        assert_eq!(fs::read(&pdf).expect("selected bytes intact"), bytes);
    }
    // A single encoded spelling must be interpreted as the same PDF name,
    // including the mandatory type, width, length, index and size fields.
    let single_table = concat!(
        "%PDF-1.7\nxref\n0 1\n0000000000 65535 f \n",
        "trailer\n<< /S#69ze 1 >>\nstartxref\n9\n%%EOF\n",
    );
    let single_stream = concat!(
        "%PDF-1.7\n1 0 obj\n",
        "<< /T#79pe /X#52ef /Si#7Ae 2 /#57 [1 0 0] ",
        "/L#65ngth 1 /Ind#65x [1 1] >>\n",
        "stream\n\0\nendstream\nendobj\n",
        "startxref\n9\n%%EOF\n",
    );
    assert!(supported_pdf_cross_reference(single_table.as_bytes()),
        "one escaped Size name in a table trailer is valid");
    assert!(supported_pdf_cross_reference(single_stream.as_bytes()),
        "a genuine encoded spelling of each required key is valid");

    let stream_with = |extra: &str| format!(
        "%PDF-1.7\n1 0 obj\n<< /Type /XRef /Size 2 \
         /W [1 0 0] /Length 1 /Index [1 1] {extra} >>\n\
         stream\n\0\nendstream\nendobj\n\
         startxref\n9\n%%EOF\n",
    ).into_bytes();
    for extra in [
        "/T#79pe /Page", "/#57 [1 0 0]",
        "/L#65ngth 2", "/Ind#65x [1 1]",
    ] {
        let candidate = stream_with(extra);
        assert!(!supported_pdf_cross_reference(&candidate),
            "escaped duplicate stream key must refuse: {extra}");
        fs::write(&pdf, &candidate).expect("write malformed stream");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "escaped duplicate must not contact CUPS");
        assert!(!spooled.exists(), "no spool side effect");
        assert_eq!(fs::read(&pdf).expect("selected bytes intact"),
            candidate);
    }
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[test]
fn previous_xref_free_link_accepts_last_bounded_object_index() {
    // Object indices are zero-based; an earlier /Prev free-list pointer
    // may name the final admitted object, but never the count itself.
    let largest_index = MAX_SUPPORTED_XREF_OBJECTS - 1;
    let head = format!(
        "xref\n0 1\n{largest_index:010} 65535 f \n",
    );
    let prefix = b"%PDF-1.7\n";
    let document = [prefix.as_slice(), head.as_bytes()].concat();
    assert!(previous_xref_table_header(
        head.as_bytes(), &document, prefix.len(),
    ), "earlier free-list link may use last admitted object index");
    let out_of_range = format!(
        "xref\n0 1\n{MAX_SUPPORTED_XREF_OBJECTS:010} 65535 f \n",
    );
    let invalid = [prefix.as_slice(), out_of_range.as_bytes()].concat();
    assert!(!previous_xref_table_header(
        out_of_range.as_bytes(), &invalid, prefix.len(),
    ), "an object count is not a valid zero-based free-list index");
}

#[cfg(unix)]
#[test]
fn hybrid_xref_table_pointer_requires_a_complete_prior_stream() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("hybrid.pdf");
    let spooled = directory.join("fake-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");

    let original = one_page_xref_stream_pdf("1.5", " /Index [0 5]");
    let prior = original.windows(b"4 0 obj\n".len())
        .position(|part| part == b"4 0 obj\n")
        .expect("real xref stream object");
    let locator = original.windows(b"startxref\n".len())
        .rposition(|part| part == b"startxref\n")
        .expect("original terminal locator");
    let prefix = original.get(..locator)
        .expect("prefix before native startxref");
    let revised = |source: &[u8], trailer: &str| [
        source,
        format!(
            "xref\n0 1\n0000000000 65535 f \n\
             trailer\n<< /Size 5 /Root 1 0 R {trailer} >>\n\
             startxref\n{}\n%%EOF\n", source.len(),
        ).as_bytes(),
    ].concat();
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    for trailer in [
        format!("/XRefStm {prior}"),
        format!("/XR#65fStm {prior}"),
        format!("/XRefStm {prior} /Meta << /XRefStm 0 >>"),
    ] {
        let candidate = revised(prefix, &trailer);
        assert!(supported_pdf_cross_reference(&candidate),
            "framed hybrid stream should remain eligible: {trailer}");
        fs::write(&pdf, &candidate).expect("write reviewed PDF");
        assert_eq!(submit_selected(&request, &fake_spool)
            .expect("valid hybrid table reaches fake CUPS"),
            sha256_profile_bytes(&candidate).bytes());
        assert_eq!(fs::read(&spooled).expect("spooled bytes"), candidate);
        fs::remove_file(&spooled).expect("clear test spool");
    }
    // In a hybrid file the traditional table may carry the xref stream's
    // own object entry. The stream's /Index then need not repeat it.
    let mut hybrid_stream = one_page_xref_stream_pdf(
        "1.5", " /Index [0 4]",
    );
    let length_key = b"/Length 35";
    let length_at = hybrid_stream.windows(length_key.len())
        .position(|part| part == length_key)
        .expect("five-row source length");
    hybrid_stream[length_at..length_at + length_key.len()]
        .copy_from_slice(b"/Length 28");
    let row_start = hybrid_stream.windows(b"stream\n".len())
        .position(|part| part == b"stream\n")
        .expect("one encoded xref stream") + b"stream\n".len();
    let _removed_xref_row: Vec<_> =
        hybrid_stream.drain(row_start + 28..row_start + 35).collect();
    let locator = hybrid_stream.windows(b"startxref\n".len())
        .rposition(|part| part == b"startxref\n")
        .expect("terminal pointer follows stream");
    hybrid_stream.truncate(locator);
    let hybrid = [
        hybrid_stream.as_slice(),
        format!(
            "xref\n4 1\n{prior:010} 00000 n \n\
             trailer\n<< /Size 5 /Root 1 0 R /XRefStm {prior} >>\n\
             startxref\n{}\n%%EOF\n", hybrid_stream.len(),
        ).as_bytes(),
    ].concat();
    assert!(supported_pdf_cross_reference(&hybrid),
        "table-owned stream must not require a duplicate /Index entry");
    fs::write(&pdf, &hybrid).expect("hybrid PDF with table-owned stream");
    assert_eq!(submit_selected(&request, &fake_spool)
        .expect("table-owned stream reaches fake spool"),
        sha256_profile_bytes(&hybrid).bytes());
    assert_eq!(fs::read(&spooled).expect("hybrid spool bytes"), hybrid);
    fs::remove_file(&spooled).expect("clear hybrid spool");

    // The owning entry can appear after a disjoint free-list subsection;
    // a bounded second pass must not stop at the first subsection.
    let multi_section_hybrid = [
        hybrid_stream.as_slice(),
        format!(
            "xref\n0 1\n0000000000 65535 f \n\
             4 1\n{prior:010} 00000 n \n\
             trailer\n<< /Size 5 /Root 1 0 R /XRefStm {prior} >>\n\
             startxref\n{}\n%%EOF\n", hybrid_stream.len(),
        ).as_bytes(),
    ].concat();
    assert!(supported_pdf_cross_reference(&multi_section_hybrid),
        "hybrid ownership in a later table subsection remains eligible");
    fs::write(&pdf, &multi_section_hybrid)
        .expect("hybrid PDF with two sections");
    assert_eq!(submit_selected(&request, &fake_spool)
        .expect("multi-section hybrid reaches fake spool"),
        sha256_profile_bytes(&multi_section_hybrid).bytes());
    assert_eq!(fs::read(&spooled).expect("spooled bytes"),
        multi_section_hybrid);
    fs::remove_file(&spooled).expect("clear multi-section spool");

    // A sparse hybrid stream may omit its own /Index row only when the
    // traditional table actually owns that indirect object. A free-only
    // table cannot justify the missing self entry merely by naming the
    // stream through /XRefStm.
    let unowned = [
        hybrid_stream.as_slice(),
        format!(
            "xref\n0 1\n0000000000 65535 f \n\
             trailer\n<< /Size 5 /Root 1 0 R /XRefStm {prior} >>\n\
             startxref\n{}\n%%EOF\n", hybrid_stream.len(),
        ).as_bytes(),
    ].concat();
    assert!(!supported_pdf_cross_reference(&unowned),
        "hybrid stream without a self-index or table entry must refuse");
    assert_fake_spool_rejected(
        &pdf, &request, &fake_spool, &spooled, &unowned);

    for trailer in [
        "/XRefStm 0".into(), "/XRefStm -1".into(),
        "/XRefStm abc".into(),
        format!("/XRefStm {}", prior + 1),
        format!("/XRefStm {}", prefix.len()),
        format!("/XRefStm {prior} /XRefStm {prior}"),
        format!("/XRefStm {prior} /XR#65fStm {prior}"),
    ] {
        let candidate = revised(prefix, &trailer);
        assert!(!supported_pdf_cross_reference(&candidate),
            "invalid hybrid pointer must refuse: {trailer}");
        assert_fake_spool_rejected(
            &pdf, &request, &fake_spool, &spooled, &candidate);
    }
    let last_endobj = prefix.windows(b"endobj\n".len())
        .rposition(|part| part == b"endobj\n")
        .expect("prior stream closing object");
    let missing_endobj = [
        prefix.get(..last_endobj).expect("prefix before endobj"),
        prefix.get(last_endobj + b"endobj\n".len()..)
            .expect("prefix after endobj"),
    ].concat();
    let forged = revised(&missing_endobj, &format!("/XRefStm {prior}"));
    assert!(!supported_pdf_cross_reference(&forged),
        "hybrid pointer cannot name a stream without its endobj");
    assert_fake_spool_rejected(
        &pdf, &request, &fake_spool, &spooled, &forged);

    // A prior stream's own /Prev must have a direct bounded numeric value,
    // even when we deliberately do not follow older revision chains.
    let length_key = b"/Length ";
    let before_length = prefix.windows(length_key.len())
        .rposition(|part| part == length_key)
        .expect("prior xref stream has direct Length");
    for invalid_prev in [
        "abc".to_owned(), "-1".to_owned(),
        "0".to_owned(), "9999999999".to_owned(),
        prior.to_string(), (prior + 1).to_string(),
    ] {
        let malformed = [
            prefix.get(..before_length).expect("prior stream prefix"),
            format!("/Prev {invalid_prev} ").as_bytes(),
            prefix.get(before_length..).expect("prior stream suffix"),
        ].concat();
        let candidate = revised(&malformed, &format!("/XRefStm {prior}"));
        assert!(!supported_pdf_cross_reference(&candidate),
            "prior stream /Prev must be a bounded decimal: {invalid_prev}");
        assert_fake_spool_rejected(
            &pdf, &request, &fake_spool, &spooled, &candidate);
    }

    let mut obsolete = revised(prefix, &format!("/XRefStm {prior}"));
    *obsolete.get_mut(7).expect("PDF minor version") = b'4';
    assert!(!supported_pdf_cross_reference(&obsolete),
        "PDF 1.4 cannot declare a hybrid-reference stream");
    assert_fake_spool_rejected(
        &pdf, &request, &fake_spool, &spooled, &obsolete);
    fs::remove_dir_all(&directory).expect("remove test fixtures");
}

#[cfg(unix)]
#[test]
fn hybrid_xref_stream_admits_a_genuine_backward_revision_chain() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("revised-hybrid.pdf");
    let spooled = directory.join("spooled.pdf");
    let script = directory.join("lp.sh");
    fs::write(&script, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700))
        .expect("make fake spooler executable");

    let original = one_page_xref_stream_pdf("1.5", " /Index [0 5]");
    let previous = original.windows(b"4 0 obj\n".len())
        .position(|part| part == b"4 0 obj\n")
        .expect("real first xref stream");
    let new_stream = original.len();
    let mut row = vec![1];
    row.extend_from_slice(&u32::try_from(new_stream)
        .expect("bounded xref byte offset").to_be_bytes());
    row.extend_from_slice(&0u16.to_be_bytes());
    let prefix = [
        original.as_slice(),
        format!(
            "5 0 obj\n<< /Type /XRef /Size 6 /Index [5 1] \
             /W [1 4 2] /Prev {previous} /Root 1 0 R \
             /Length 7 >>\nstream\n",
        ).as_bytes(),
        row.as_slice(),
        b"\nendstream\nendobj\n".as_slice(),
    ].concat();
    let candidate = [
        prefix.as_slice(),
        format!(
            "xref\n0 1\n0000000000 65535 f \n\
             5 1\n{new_stream:010} 00000 n \n\
             trailer\n<< /Size 6 /Root 1 0 R \
             /Prev {previous} /XRefStm {new_stream} >>\n\
             startxref\n{}\n%%EOF\n", prefix.len(),
        ).as_bytes(),
    ].concat();
    assert!(supported_pdf_cross_reference(&candidate),
        "valid two-stream hybrid incremental PDF stays admissible");
    fs::write(&pdf, &candidate).expect("write reviewed hybrid PDF");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert_eq!(submit_selected(&request, &script)
        .expect("hybrid revision reaches fake CUPS"),
        sha256_profile_bytes(&candidate).bytes());
    assert_eq!(fs::read(&spooled).expect("spooled PDF"), candidate);
    assert_eq!(fs::read(&pdf).expect("selected PDF"), candidate);
    fs::remove_dir_all(&directory).expect("remove owned fixtures");
}

#[cfg(unix)]
#[test]
fn xref_table_prev_must_point_back_to_a_prior_xref() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spooler executable");

    let native = real_pdf();
    let earlier_xref = native.windows(b"xref\n".len())
        .position(|part| part == b"xref\n")
        .expect("native PDF has original xref table");
    let terminal_offset = native.len();
    let revised = |prev: &str| [
        native.as_slice(),
        format!(
            "xref\n0 1\n0000000000 65535 f \n\
             trailer\n<< /Size 10 /Root 1 0 R {prev} >>\n\
             startxref\n{terminal_offset}\n%%EOF\n",
        ).as_bytes(),
    ].concat();
    for valid in [
        format!("/Prev {earlier_xref}"),
        format!("/P#72ev {earlier_xref}"),
        format!("/Prev {earlier_xref} /Meta << /Prev 99999999 >>"),
    ] {
        let candidate = revised(&valid);
        assert!(supported_pdf_cross_reference(&candidate),
            "valid earlier xref and nested metadata accepted: {valid}");
        fs::write(&pdf, &candidate).expect("write incremental PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        let digest = submit_selected(&request, &fake_spool)
            .expect("valid incremental PDF should reach fake CUPS");
        assert_eq!(digest, sha256_profile_bytes(&candidate).bytes());
        assert_eq!(fs::read(&spooled).expect("fake CUPS bytes"),
            candidate);
        fs::remove_file(&spooled).expect("clear fake spool output");
    }
    for invalid in [
        "/Prev 0".into(), "/Prev -1".into(),
        "/Prev +1".into(), "/Prev abc".into(),
        "/Prev 9999999999".into(),
        format!("/Prev {}", earlier_xref + 1),
        format!("/Prev {terminal_offset}"),
        format!("/Prev {earlier_xref} /Prev {earlier_xref}"),
        format!("/Prev {earlier_xref} /P#72ev {earlier_xref}"),
    ] {
        let candidate = revised(&invalid);
        assert!(!supported_pdf_cross_reference(&candidate),
            "bad incremental xref link refuses: {invalid}");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert_fake_spool_rejected(
            &pdf, &request, &fake_spool, &spooled, &candidate);
        assert!(selected_pdf(&pdf).is_err(), "invalid Prev refused");
    }
    // A prior /Prev table must contain its complete first subsection,
    // not merely an xref keyword and one plausible first record. Forged
    // counts can otherwise consume the current revision as missing rows.
    for fragment in [
        b"xref\nnot-a-subsection\n".as_slice(),
        b"xref\n0 1\n".as_slice(),
        b"xref\n0 1\n0000000000 65535 z \n".as_slice(),
        b"xref\n0\n1\n0000000000 65535 f \n".as_slice(),
        b"xref 0 1\n0000000000 65535 f \n".as_slice(),
        b"xref\n0 2\n0000000000 65535 f \n".as_slice(),
        concat!("xref\n0 2\n0000000000 65535 f \n",
            "0000000000 00000 z \n").as_bytes(),
        concat!("xref\n0 2\n0000000000 65535 f \n",
            "0000000000 00000 n \n").as_bytes(),
        b"xref\n0 1\n9999999999 65535 f \n".as_slice(),
        concat!("xref\n0 2\n0000000000 65535 f \n",
            "9999999999 00000 f \n").as_bytes(),
    ] {
        let fake_previous = native.len();
        let terminal = fake_previous + fragment.len();
        let candidate = [
            native.as_slice(), fragment,
            format!(
                "xref\n0 1\n0000000000 65535 f \n\
                 trailer\n<< /Size 10 /Root 1 0 R \
                 /Prev {fake_previous} >>\n\
                 startxref\n{terminal}\n%%EOF\n",
            ).as_bytes(),
        ].concat();
        assert!(!supported_pdf_cross_reference(&candidate),
            "fake predecessor must not be accepted: {fragment:?}");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert_fake_spool_rejected(
            &pdf, &request, &fake_spool, &spooled, &candidate);
    }
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn xref_table_prev_accepts_earlier_xref_stream_on_pdf_15() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("spooled.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");
    let original = one_page_xref_stream_pdf("1.5", " /Index [0 5]");
    let previous = original.windows(b"4 0 obj\n".len())
        .position(|part| part == b"4 0 obj\n")
        .expect("prior xref stream indirect object header");
    let last = original.len();
    let bytes = [
        original.as_slice(),
        format!(
            "xref\n0 1\n0000000000 65535 f \n\
             trailer\n<< /Size 5 /Root 1 0 R /Prev {previous} >>\n\
             startxref\n{last}\n%%EOF\n",
        ).as_bytes(),
    ].concat();
    assert!(supported_pdf_cross_reference(&bytes),
        "traditional revision may refer to a valid earlier xref stream");
    fs::write(&pdf, &bytes).expect("selected PDF");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    let digest = submit_selected(&request, &fake_spool)
        .expect("incremental xref stream can reach fake CUPS");
    assert_eq!(digest, sha256_profile_bytes(&bytes).bytes());
    assert_eq!(fs::read(&spooled).expect("fake spool bytes"), bytes);
    assert_eq!(fs::read(&pdf).expect("original unchanged"), bytes);
    fs::remove_file(&spooled).expect("clear valid fake spool");
    // A plausible prior stream dictionary alone is not a complete xref
    // object. A forged /Prev must not accept a missing or fused terminator.
    for fragment in [
        b"5 0 obj\n<< /Type /XRef /Size 6 /W [1 4 2] \
          /Length 7 >>\nstream\n".as_slice(),
        b"5 0 obj\n<< /Type /XRef /Size 6 /W [1 4 2] \
          /Length 7 >>\nstream\n1234567\nendstream\n".as_slice(),
        b"5 0 obj\n<< /Type /XRef /Size 6 /W [1 4 2] \
          /Length 7 >>\nstream\n1234567\nendstream\nendobjjunk\n"
            .as_slice(),
    ] {
        let fake_previous = original.len();
        let terminal = fake_previous + fragment.len();
        let candidate = [
            original.as_slice(), fragment,
            format!(
                "xref\n0 1\n0000000000 65535 f \n\
                 trailer\n<< /Size 6 /Root 1 0 R \
                 /Prev {fake_previous} >>\n\
                 startxref\n{terminal}\n%%EOF\n",
            ).as_bytes(),
        ].concat();
        assert!(!supported_pdf_cross_reference(&candidate),
            "incomplete prior stream must not qualify");
        fs::write(&pdf, &candidate).expect("write forged PDF");
        assert!(selected_pdf(&pdf).is_err());
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "fake previous stream cannot reach CUPS");
        assert!(!spooled.exists(), "refusal has no spool side effect");
        assert_eq!(fs::read(&pdf).expect("unchanged PDF"), candidate);
    }
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn terminal_xref_stream_accepts_prior_native_xref_table() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("native-stream-revision.pdf");
    let spooled = directory.join("spooled.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");

    let mut original = real_pdf();
    assert!(original.starts_with(b"%PDF-1."),
        "native exporter uses a PDF 1.x version header");
    *original.get_mut(7).expect("version digit") = b'5';
    let previous = original.windows(b"xref\n".len())
        .position(|part| part == b"xref\n")
        .expect("native PDF has real traditional xref table");
    let terminal = original.len();
    let mut row = vec![1u8];
    row.extend_from_slice(&u32::try_from(terminal)
        .expect("small native PDF offset fits u32").to_be_bytes());
    row.extend_from_slice(&0u16.to_be_bytes());
    // /Index may be sparse across revisions: only this newly added xref
    // stream object is redefined; earlier entries remain in the old table.
    let candidate = [
        original.as_slice(),
        format!(
            "4095 0 obj\n<< /Type /XRef /Size 4096 \
             /Index [4095 1] /W [1 4 2] /Root 1 0 R \
             /Prev {previous} /Length 7 >>\nstream\n",
        ).as_bytes(),
        row.as_slice(),
        format!(
            "\nendstream\nendobj\nstartxref\n{terminal}\n%%EOF\n",
        ).as_bytes(),
    ].concat();
    assert!(supported_pdf_cross_reference(&candidate),
        "terminal stream can follow a native traditional xref table");
    fs::write(&pdf, &candidate).expect("selected incremental PDF");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert_eq!(
        submit_selected(&request, &fake_spool)
            .expect("native table-to-stream revision reaches fake CUPS"),
        sha256_profile_bytes(&candidate).bytes(),
    );
    assert_eq!(fs::read(&spooled).expect("exact fake spool bytes"),
        candidate);
    assert_eq!(fs::read(&pdf).expect("selected source unchanged"),
        candidate);
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn terminal_xref_stream_prev_must_point_back_to_a_prior_xref() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("unexpected-spool.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");

    let original = one_page_xref_stream_pdf("1.5", " /Index [0 5]");
    let previous = original.windows(b"4 0 obj\n".len())
        .position(|part| part == b"4 0 obj\n")
        .expect("original xref stream header");
    let terminal = original.len();
    let mut row = vec![1u8];
    row.extend_from_slice(&u32::try_from(terminal)
        .expect("small fixture offset fits u32").to_be_bytes());
    row.extend_from_slice(&0u16.to_be_bytes());
    let revised = |prev: &str| [
        original.as_slice(),
        format!(
            "5 0 obj\n<< /Type /XRef /Size 6 /Index [5 1] \
             /W [1 4 2] /Root 1 0 R {prev} /Length 7 >>\nstream\n",
        ).as_bytes(),
        row.as_slice(),
        format!(
            "\nendstream\nendobj\nstartxref\n{terminal}\n%%EOF\n",
        ).as_bytes(),
    ].concat();
    for valid in [
        format!("/Prev {previous}"),
        format!("/P#72ev {previous}"),
        format!("/Prev {previous} /Meta << /Prev 0 >>"),
    ] {
        let candidate = revised(&valid);
        assert!(supported_pdf_cross_reference(&candidate),
            "earlier xref stream should remain eligible: {valid}");
        fs::write(&pdf, &candidate).expect("write incremental PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert_eq!(
            submit_selected(&request, &fake_spool)
                .expect("valid stream revision reaches fake CUPS"),
            sha256_profile_bytes(&candidate).bytes(),
        );
        assert_eq!(fs::read(&spooled).expect("exact spool bytes"),
            candidate);
        fs::remove_file(&spooled).expect("clear fake spool output");
    }
    for invalid in [
        "/Prev 0".into(), "/Prev -1".into(),
        "/Prev +1".into(), "/Prev abc".into(),
        "/Prev 9999999999".into(),
        format!("/Prev {}", previous + 1),
        format!("/Prev {terminal}"),
        format!("/Prev {previous} /Prev {previous}"),
        format!("/Prev {previous} /P#72ev {previous}"),
    ] {
        let candidate = revised(&invalid);
        assert!(!supported_pdf_cross_reference(&candidate),
            "malformed stream predecessor must refuse: {invalid}");
        fs::write(&pdf, &candidate).expect("write invalid PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "invalid Prev refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "invalid Prev must not contact CUPS");
        assert!(!spooled.exists(), "no fake spool contact");
        assert_eq!(fs::read(&pdf).expect("selected bytes intact"),
            candidate);
    }
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn traditional_trailer_dictionary_delimiter_can_touch_keyword() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let pdf = directory.join("selected.pdf");
    let spooled = directory.join("spooled.pdf");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\ncat > '{}'\n", spooled.display(),
    )).expect("write fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");

    let native = real_pdf();
    let trailer = native.windows(b"trailer\n<<".len())
        .position(|part| part == b"trailer\n<<")
        .expect("native PDF carries a direct table trailer dictionary");
    let touching = [
        native.get(..trailer).expect("prefix").to_vec(),
        b"trailer<<".to_vec(),
        native.get(trailer + b"trailer\n<<".len()..)
            .expect("suffix").to_vec(),
    ].concat();
    assert!(supported_pdf_cross_reference(&touching),
        "PDF dictionary delimiter is a legal lexical token boundary");
    fs::write(&pdf, &touching).expect("write native touching trailer PDF");
    let request = PrintRequest {
        pdf: pdf.clone(), printer: "Queue_1".into(),
        dry_run: false, expected_sha256: None,
    };
    assert_eq!(submit_selected(&request, &fake_spool)
        .expect("direct trailer dictionary is accepted"),
        sha256_profile_bytes(&touching).bytes());
    assert_eq!(fs::read(&spooled).expect("fake CUPS exact bytes"),
        touching, "source and spool bytes match");

    let fake = [
        native.get(..trailer).expect("prefix").to_vec(),
        b"trailerX<<".to_vec(),
        native.get(trailer + b"trailer\n<<".len()..)
            .expect("suffix").to_vec(),
    ].concat();
    assert!(!supported_pdf_cross_reference(&fake),
        "a longer keyword must not impersonate trailer");
    fs::remove_file(&spooled).expect("clear prior fake queue");
    fs::write(&pdf, &fake).expect("write invalid PDF");
    assert!(submit_selected(&request, &fake_spool).is_err(),
        "invalid trailer keyword must not reach CUPS");
    assert!(!spooled.exists(), "no new fake spool side effect");
    assert_eq!(fs::read(&pdf).expect("selected bytes unchanged"), fake);
    fs::remove_dir_all(&directory).expect("remove fixture directory");
}

#[cfg(unix)]
#[test]
fn stalled_cups_submission_is_bounded_even_for_a_full_stdin_pipe() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let contacted = directory.join("fake-lp-launches.txt");
    let fake_spool = directory.join("lp.sh");
    fs::write(&fake_spool, format!(
        "#!/bin/sh\nprintf X >> '{}'\nexec sleep 20\n",
        contacted.display(),
    )).expect("write stalled fake CUPS executable");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake CUPS executable");

    // A tiny payload stalls on wait; two megabytes also fill a nonreading
    // pipe and would hang the old synchronous write_all indefinitely.
    for (attempt, pdf) in [
        (1, vec![0x42u8; 16]),
        (2, vec![0x42u8; 2_000_000]),
    ] {
        let start = Instant::now();
        let error = submit_with_timeout(
            &fake_spool, "Queue_1", pdf, Duration::from_millis(350),
        ).expect_err("stalled fake CUPS must not block forever");
        assert!(error.contains("outcome uncertain"),
            "a deadline is not proof that the job was rejected: {error}");
        assert!(error.contains("inspect the queue before retrying"),
            "manual reconciliation required: {error}");
        assert!(start.elapsed() < Duration::from_secs(4),
            "deadline must bound blocked pipe writes and process waits");
        assert_eq!(fs::read(&contacted).expect("fake lp was launched"),
            vec![b'X'; attempt], "one attempt, never an automatic retry");
    }
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}

#[cfg(unix)]
#[test]
fn early_successful_lp_exit_during_transfer_is_an_uncertain_submission() {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = fixture_dir();
    let fake_spool = directory.join("early-lp.sh");
    fs::write(&fake_spool, "#!/bin/sh\nexec 0<&-\nexit 0\n")
        .expect("write early-success fake spooler");
    fs::set_permissions(&fake_spool, fs::Permissions::from_mode(0o700))
        .expect("make fake spool executable");

    // The fake lp reports success but closes its input before a complete
    // bounded PDF could be supplied. It might already have queued a job.
    let error = submit_with_timeout(
        &fake_spool, "Queue_1", vec![0x42u8; 2_000_000],
        Duration::from_secs(3),
    ).expect_err("an incomplete transfer cannot claim spooling success");
    assert!(error.contains("outcome uncertain"), "unexpected: {error}");
    assert!(error.contains("inspect the queue before retrying"),
        "an early lp exit must not invite a blind retry: {error}");
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
    assert!(submit_with(&script, "Queue_1", selected).is_err());
    let linked = directory.join("linked.pdf");
    symlink(&pdf, &linked).expect("linked fixture");
    assert!(selected_pdf(&linked).is_err(), "source links fail closed");
    fs::remove_dir_all(&directory).expect("remove owned fixture directory");
}
