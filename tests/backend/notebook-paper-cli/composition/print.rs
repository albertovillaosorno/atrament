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
        b"%PDF-1.7\n1 0 obj\n".as_slice(),
        b"<< /Type /XRef /Size 2 /Length 1 /W [1 0 0] >>\n".as_slice(),
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
        " /Size 2 /Length 1 /W [1 0 0] /DecodeParms << /Type /Page >> >>\n",
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
        "%PDF-1.7\n1 0 obj\n<< /Length 1 /W [1 0 0] {dictionary} >>\n\
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
             << /Type /XRef /Size {size} /Length 1 /W [1 0 0] >>\nstream\n\0\n\
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
             << /Type /XRef /Size 2 /W [1 0 0] {dictionary} >>\nstream\n",
        ).into_bytes(),
        data.to_vec(),
        b"\nendstream\nendobj\nstartxref\n9\n%%EOF\n".to_vec(),
    ].concat();
    for valid in [
        framed("/Length 1", b"\0"),
        framed("/Length 3", b"xyz"),
        framed("/Meta << /Length 99999999 >> /Length 1", b"\0"),
        framed("/Length 16", b"\0\nendstream\nxxxx"),
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
    let framed = |dictionary: &str| format!(
        "%PDF-1.7\n1 0 obj\n\
         << /Type /XRef /Size 2 /Length 1 {dictionary} >>\n\
         stream\n\0\nendstream\nendobj\nstartxref\n9\n%%EOF\n",
    ).into_bytes();
    for valid in [
        "/W [1 2 1]",
        "/W[0 2 0]",
        "/Meta << /W [9] >> /W [1% comment\n 2 1]",
    ] {
        let source = framed(valid);
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
        let source = framed(invalid);
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
    let framed = |index: &str| format!(
        "%PDF-1.7\n1 0 obj\n\
         << /Type /XRef /Size 3 /W [1 0 0] /Length 1 {index} >>\n\
         stream\n\0\nendstream\nendobj\nstartxref\n9\n%%EOF\n",
    ).into_bytes();
    for valid in [
        "", "/Index [1 1]", "/Index[0 1 2 1]",
        "/Index [0% row comment\n 1 2 1]",
        "/Meta << /Index [9 9] >> /Index [1 1]",
    ] {
        let candidate = framed(valid);
        assert!(supported_pdf_cross_reference(&candidate),
            "disjoint or omitted stream ranges remain admitted: {valid:?}");
    }
    for invalid in [
        "/Index []", "/Index [0]", "/Index [0 0]",
        "/Index [0 -1]", "/Index [0 +1]",
        "/Index [0 2 1]", "/Index [0 2 1 1]",
        "/Index [2 2]", "/Index [3 1]",
        "/Index [9999999999 1]", "/Index [0 9999999999]",
        "/Index [0 1] /Index [1 1]",
        "/Index (0 1)", "/Index [0 1 /Other]",
        "/Index [0 1.0]", "/Index [18446744073709551616 1]",
    ] {
        let candidate = framed(invalid);
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
        fs::write(&pdf, &candidate).expect("write invalid PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(selected_pdf(&pdf).is_err(), "invalid Prev refused");
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "invalid Prev cannot contact CUPS");
        assert!(!spooled.exists(), "no fake spool contact");
        assert_eq!(fs::read(&pdf).expect("selected bytes intact"),
            candidate);
    }
    // An earlier bare "xref" token is not an xref section. These appended
    // pretexts previously passed /Prev even though no first entry existed.
    for fragment in [
        b"xref\nnot-a-subsection\n".as_slice(),
        b"xref\n0 1\n".as_slice(),
        b"xref\n0 1\n0000000000 65535 z \n".as_slice(),
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
            "fake predecessor must not be accepted");
        fs::write(&pdf, &candidate).expect("write forged PDF");
        let request = PrintRequest {
            pdf: pdf.clone(), printer: "Queue_1".into(),
            dry_run: false, expected_sha256: None,
        };
        assert!(submit_selected(&request, &fake_spool).is_err(),
            "fake predecessor cannot reach CUPS");
        assert!(!spooled.exists(), "refusal does not launch fake CUPS");
        assert_eq!(fs::read(&pdf).expect("selected PDF unchanged"),
            candidate);
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
