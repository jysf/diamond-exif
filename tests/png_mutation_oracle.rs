//! Red-proofs for the PNG ExifTool comparison (SPEC-031): a FAULT in the PNG
//! layer of the library, or a corruption of a PNG, must turn the comparison with
//! ExifTool's committed answers red, naming the selectors it should.
//!
//! This is a separate file from `tests/mutation_oracle.rs` (the builder's choice
//! SPEC-031 allows) so the TIFF and JPEG red-proofs do not move; it reuses the
//! same apparatus unchanged: `tests/support/probe.rs` stages a throw-away copy of
//! `src/`, injects one expression marked `RED-PROOF INJECTION` located by a
//! needle that must occur exactly once, builds a probe crate and runs it over
//! each fixture; `just mutation-oracle` runs both files. The probe reads a file
//! that starts with the PNG signature through `png::read` and the harness model
//! (`oracle::read_png_all`).
//!
//! Every library fault has the `oracle-must-be-shown-red` shape: a CONTROL first
//! (the same apparatus on the unmutated copy equals ExifTool's committed record
//! on the same fixtures, zero unexplained mismatches), then the mutant, which must
//! produce the expected selectors and no others. Everything compares against the
//! COMMITTED records, so the red-proofs need no ExifTool; only the live variants
//! of the byte mutations use it and skip loudly without it
//! (`DIAMOND_EXIF_REQUIRE_ORACLE=1` makes that a failure). Library code under
//! `src/` is never modified.

#[path = "support/i3_judge.rs"]
mod i3_judge;
#[path = "support/i3_records.rs"]
mod i3_records;
#[path = "support/i3_shapes.rs"]
mod i3_shapes;
#[path = "support/mutate.rs"]
mod mutate;
#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_png.rs"]
mod oracle_png;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/oracle_webp.rs"]
mod oracle_webp;
#[path = "support/probe.rs"]
mod probe;
#[path = "support/tools.rs"]
mod tools;
#[path = "support/webp_carriers.rs"]
mod webp_carriers;
#[path = "support/webp_shapes.rs"]
mod webp_shapes;
#[path = "support/writer_mutants.rs"]
mod writer_mutants;

use oracle::{JpegComparison, Mismatch};
use probe::{TempDir, MARKER};
use std::path::{Path, PathBuf};

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

/// The two selectors a Composite GPS fault names.
const GPS_SELECTORS: [&str; 2] = ["Composite:GPSLatitude", "Composite:GPSLongitude"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn marked(code: &str) -> String {
    format!("{code} {MARKER}")
}

fn named(m: &[Mismatch]) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = m.iter().map(|m| m.selector).collect();
    v.sort_unstable();
    v
}

fn sorted(mut v: Vec<&'static str>) -> Vec<&'static str> {
    v.sort_unstable();
    v
}

fn describe(m: &[Mismatch]) -> String {
    m.iter()
        .map(|m| format!("{m}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn png_fixture(name: &str) -> Vec<u8> {
    oracle_png::fixtures()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no PNG fixture named {name}"))
}

/// ExifTool's fields from a committed record: 22 wide.
fn theirs(name: &str) -> Vec<Option<String>> {
    let path = root()
        .join("tests/oracle-fixtures/exiftool")
        .join(format!("{name}.txt"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); run `just oracle-record`",
            path.display()
        )
    });
    let rec = tools::parse_record(&text).unwrap_or_else(|| panic!("{name}: bad record header"));
    tools::parse_fields(&oracle::png_names(), &rec.stdout).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// `oracle::compare_png` with the pin an unread-wildcard fixture needs.
fn compare(
    name: &str,
    ours: &[Option<String>],
    theirs_fields: &[Option<String>],
) -> JpegComparison {
    let pin = oracle::png_pin_record(name).map(theirs);
    oracle::compare_png(name, ours, theirs_fields, pin.as_deref())
}

/// Our in-process reading of a PNG (22 fields).
fn ours_in_process(bytes: &[u8]) -> Vec<Option<String>> {
    match exif_core::png::read(bytes) {
        Ok(m) => oracle::read_png_all(&m),
        Err(_) => vec![None; oracle::png_names().len()],
    }
}

/// Stage + build a probe crate, run it on each PNG fixture and compare with the
/// committed record (divergences applied). `Err` only for apparatus failures.
fn png_probe(
    label: &str,
    fixtures: &[&str],
    inject: Option<&dyn Fn(&Path)>,
) -> Result<Vec<(String, JpegComparison)>, String> {
    let files = TempDir::new(&format!("{label}-files"));
    let paths: Vec<(String, PathBuf)> = fixtures
        .iter()
        .map(|n| {
            let path = files.path().join(format!("{n}.png"));
            std::fs::write(&path, png_fixture(n)).expect("write fixture");
            ((*n).to_string(), path)
        })
        .collect();
    let crate_dir = TempDir::new(label);
    probe::stage_probe_crate(crate_dir.path(), inject);
    let bin = probe::build_probe(crate_dir.path())?;
    let mut out = Vec::new();
    for (name, path) in &paths {
        let ours = probe::run_probe(&bin, path)?;
        assert_eq!(
            ours.len(),
            oracle::png_names().len(),
            "{name}: the probe's PNG mode prints one field per PNG selector"
        );
        out.push((name.clone(), compare(name, &ours, &theirs(name))));
    }
    Ok(out)
}

fn assert_control_clean(label: &str, fixtures: &[&str]) {
    let control = png_probe(&format!("{label}-control"), fixtures, None)
        .unwrap_or_else(|e| panic!("{label}: the CONTROL could not be built or run: {e}"));
    for (name, cmp) in control {
        assert!(
            cmp.is_clean(),
            "{label}: the unmutated control is not clean on {name}: unexplained [{}] stale {:?}",
            describe(&cmp.unexplained),
            cmp.stale
        );
    }
}

/// Control first (clean), then the mutant.
fn control_then_mutant(
    label: &str,
    fixtures: &[&str],
    inject: &dyn Fn(&Path),
) -> Vec<(String, JpegComparison)> {
    assert_control_clean(label, fixtures);
    png_probe(&format!("{label}-mutant"), fixtures, Some(inject))
        .unwrap_or_else(|e| panic!("{label}: the MUTANT could not be built or run: {e}"))
}

fn cmp_for<'a>(all: &'a [(String, JpegComparison)], fixture: &str) -> &'a JpegComparison {
    let c = all
        .iter()
        .find(|(n, _)| n == fixture)
        .map(|(_, c)| c)
        .unwrap_or_else(|| panic!("no result for {fixture}"));
    // Printed so the red result is visible in `--nocapture` output.
    println!(
        "MUTANT RESULT {fixture}: unexplained [{}] stale {:?} explained [{}]",
        describe(&c.unexplained),
        c.stale,
        describe(&c.explained)
    );
    c
}

/// The selectors ExifTool reported on a fixture (those a "reads nothing" fault
/// must turn into mismatches).
fn reported(fixture: &str) -> Vec<&'static str> {
    sorted(
        oracle::png_names()
            .into_iter()
            .zip(theirs(fixture))
            .filter(|(_, t)| t.is_some())
            .map(|(n, _)| n)
            .collect(),
    )
}

/// A mutant that makes us read nothing of what ExifTool reads: exactly those
/// selectors, all `ours = None, theirs = Some`, nothing stale.
fn assert_reads_nothing(all: &[(String, JpegComparison)], fixture: &str) {
    let c = cmp_for(all, fixture);
    assert_eq!(
        named(&c.unexplained),
        reported(fixture),
        "{fixture}: {}",
        describe(&c.unexplained)
    );
    assert!(
        c.unexplained
            .iter()
            .all(|m| m.ours.is_none() && m.theirs.is_some()),
        "{fixture}: {}",
        describe(&c.unexplained)
    );
    assert!(c.stale.is_empty(), "{fixture}: {:?}", c.stale);
}

/// Every fixture any PNG mutant uses.
const MUTANT_FIXTURES: &[&str] = &[
    "png-exif-basic-le",
    "png-exif-gps-ne-le",
    "png-exif-prefix",
    "png-two-exif-same-tag",
    "png-two-exif-same-tag-reversed",
    "png-two-exif-disjoint-tags",
    "png-exif-after-iend",
    "png-trailer-bytes",
    "png-xmp-itxt",
    "png-xmp-text",
    "png-xmp-itxt-compressed",
    "png-xmp-itxt-flag-2",
    "png-iccp-valid",
    "png-iccp-bad-header",
];

#[test]
fn controls_are_green() {
    // One control over every fixture, the hand-built list and the wrapped TIFFs:
    // the unmutated apparatus (copy, build, PNG-mode probe, rendering) reproduces
    // the committed ExifTool answers, with the known divergences exercised.
    let all: Vec<String> = oracle_png::fixtures().into_iter().map(|(n, _)| n).collect();
    for used in MUTANT_FIXTURES {
        assert!(all.iter().any(|n| n == used), "{used} is not a fixture");
    }
    let refs: Vec<&str> = all.iter().map(String::as_str).collect();
    assert_control_clean("png-all-fixtures", &refs);

    // And the control is not vacuous: the probe's reading is the in-process one.
    let files = TempDir::new("png-control-same-files");
    let crate_dir = TempDir::new("png-control-same");
    probe::stage_probe_crate(crate_dir.path(), None);
    let bin = probe::build_probe(crate_dir.path()).expect("build");
    for name in [
        "png-exif-gps-ne-le",
        "png-two-exif-same-tag",
        "png-xmp-itxt",
        "png-iccp-valid",
        "png-raw-exif-text",
    ] {
        let path = files.path().join(format!("{name}.png"));
        std::fs::write(&path, png_fixture(name)).expect("write");
        let via_probe = probe::run_probe(&bin, &path).expect("run");
        assert_eq!(via_probe, ours_in_process(&png_fixture(name)), "{name}");
    }
}

// Mutant p1. Needle (src/png/metadata.rs, `Extract::chunk`, the eXIf arm):
// `None => (data, body_at),`. The TIFF slice starts one byte late, so no TIFF
// header is found and no tag is read.
#[test]
fn mutant_exif_tiff_offset_wrong_by_one_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("png/metadata.rs"),
            "None => (data, body_at),",
            &marked("None => (data.get(1..).unwrap_or(data), body_at),"),
        );
    };
    let fixtures = ["png-exif-basic-le", "png-exif-gps-ne-le"];
    let got = control_then_mutant("png-p1-offset", &fixtures, &inject);
    for f in fixtures {
        assert_reads_nothing(&got, f);
    }
    assert!(named(&cmp_for(&got, "png-exif-basic-le").unexplained).contains(&"IFD0:Make"));
}

// Mutant p2: the first block is not primary. Needle: `self.blocks.push(PngExifBlock {`.
// Each new block is put in FRONT, so the block list is in reverse file order and
// the LAST `eXIf` becomes the first parsed one, which the harness model (and
// ExifTool, measured) read. Caught on the three several-block fixtures.
#[test]
fn mutant_first_block_is_not_primary_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("png/metadata.rs"),
            "self.blocks.push(PngExifBlock {",
            &marked("self.blocks.insert(0, PngExifBlock {"),
        );
    };
    let fixtures = [
        "png-two-exif-same-tag",
        "png-two-exif-same-tag-reversed",
        "png-two-exif-disjoint-tags",
        "png-exif-basic-le",
    ];
    let got = control_then_mutant("png-p2-order", &fixtures, &inject);
    // The wrong block is read: its Make differs and its Model is lost or invented.
    for f in &fixtures[..3] {
        let c = cmp_for(&got, f);
        assert_eq!(
            named(&c.unexplained),
            vec!["IFD0:Make", "IFD0:Model"],
            "{f}: {}",
            describe(&c.unexplained)
        );
    }
    // A single block has no order to get wrong.
    assert!(cmp_for(&got, "png-exif-basic-le").is_clean());
}

// Mutant p3: the `Exif\0\0` prefix is not skipped. Needle: `data.strip_prefix(EXIF_PREFIX)`.
#[test]
fn mutant_exif_prefix_is_not_skipped_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("png/metadata.rs"),
            "data.strip_prefix(EXIF_PREFIX)",
            &marked("data.strip_prefix(b\"\\xFF\\xFF\\xFF\\xFF\\xFF\\xFF\")"),
        );
    };
    let fixtures = ["png-exif-prefix", "png-exif-basic-le"];
    let got = control_then_mutant("png-p3-prefix", &fixtures, &inject);
    assert_reads_nothing(&got, "png-exif-prefix");
    // The canonical (prefix-free) payload is unaffected.
    assert!(cmp_for(&got, "png-exif-basic-le").is_clean());
}

// Mutant p4: the trailer scan is disabled. Needle: `for _ in 0..MAX_PNG_TRAILER_CHUNKS {`.
// An `eXIf` after `IEND` is lost.
#[test]
fn mutant_trailer_scan_disabled_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("png/metadata.rs"),
            "for _ in 0..MAX_PNG_TRAILER_CHUNKS {",
            &marked("for _ in 0..0 {"),
        );
    };
    let fixtures = ["png-exif-after-iend", "png-trailer-bytes"];
    let got = control_then_mutant("png-p4-trailer", &fixtures, &inject);
    assert_reads_nothing(&got, "png-exif-after-iend");
    // The same TIFF before IEND (with arbitrary trailer bytes) is unaffected.
    assert!(cmp_for(&got, "png-trailer-bytes").is_clean());
}

// Mutant p5: `XML:com.adobe.xmp` routing is broken. Needle:
// `let region = if keyword == XMP_KEYWORD && !tagged {`. No XMP is found.
#[test]
fn mutant_xmp_keyword_routing_broken_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("png/metadata.rs"),
            "let region = if keyword == XMP_KEYWORD && !tagged {",
            &marked("let region = if keyword == b\"XML:com.adobe.xmx\" {"),
        );
    };
    let fixtures = ["png-xmp-itxt", "png-xmp-text", "png-exif-basic-le"];
    let got = control_then_mutant("png-p5-xmp", &fixtures, &inject);
    for f in ["png-xmp-itxt", "png-xmp-text"] {
        assert_reads_nothing(&got, f);
        assert_eq!(
            named(&cmp_for(&got, f).unexplained),
            vec!["XMP-xmp:CreatorTool"]
        );
    }
    assert!(cmp_for(&got, "png-exif-basic-le").is_clean());
}

// Mutant p6: `iCCP` is not located. Needle: `ChunkKind::Iccp => {`.
#[test]
fn mutant_iccp_not_located_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("png/metadata.rs"),
            "ChunkKind::Iccp => {",
            &marked("ChunkKind::Iccp if false => {"),
        );
    };
    let fixtures = ["png-iccp-valid", "png-iccp-bad-header", "png-xmp-itxt"];
    let got = control_then_mutant("png-p6-iccp", &fixtures, &inject);
    // Both selectors go for the valid profile. For the bad header ExifTool prints
    // no ICC-header tag, so only the name selector goes, and the recorded
    // divergence on the ICC-header selector turns stale (we no longer locate).
    assert_reads_nothing(&got, "png-iccp-valid");
    assert_eq!(
        named(&cmp_for(&got, "png-iccp-valid").unexplained),
        vec!["ICC-header:ProfileFileSignature", "PNG:ProfileName"]
    );
    let bad = cmp_for(&got, "png-iccp-bad-header");
    assert_eq!(named(&bad.unexplained), vec!["PNG:ProfileName"]);
    assert_eq!(bad.stale.len(), 1, "{:?}", bad.stale);
    assert!(cmp_for(&got, "png-xmp-itxt").is_clean());
}

// Mutant p7: the `iTXt` compression flag is ignored. Needle: `let compressed = flag != 0;`.
// A compressed XMP now looks readable, so the recorded divergence
// (`png-compressed-xmp-not-inflated`) DISAPPEARS: a stale entry is red, exactly
// like a new mismatch; and an `iTXt` with the invalid flag 2 (ExifTool cannot read
// it) now looks readable and is named.
#[test]
fn mutant_itxt_compression_flag_ignored_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("png/metadata.rs"),
            "let compressed = flag != 0;",
            &marked("let compressed = false;"),
        );
    };
    let fixtures = [
        "png-xmp-itxt-compressed",
        "png-xmp-itxt-flag-2",
        "png-xmp-itxt",
    ];
    let got = control_then_mutant("png-p7-flag", &fixtures, &inject);
    let c = cmp_for(&got, "png-xmp-itxt-compressed");
    assert!(c.unexplained.is_empty(), "{}", describe(&c.unexplained));
    assert_eq!(c.stale.len(), 1, "{:?}", c.stale);
    assert!(
        c.stale[0].contains("png-compressed-xmp-not-inflated"),
        "{:?}",
        c.stale
    );
    let c = cmp_for(&got, "png-xmp-itxt-flag-2");
    assert_eq!(named(&c.unexplained), vec!["XMP-xmp:CreatorTool"]);
    assert!(c
        .unexplained
        .iter()
        .all(|m| m.ours.is_some() && m.theirs.is_none()));
    assert!(cmp_for(&got, "png-xmp-itxt").is_clean());
}

// ── Byte-mutation red-proofs (no probe crate: mutate the bytes in process) ──

/// Where the TIFF blob of the (single) `eXIf` chunk starts in a fixture's bytes,
/// and its length, found WITHOUT the library under test: the first `eXIf` type
/// plus 4, the length being the big-endian word before it.
fn exif_chunk_at(png: &[u8]) -> (usize, usize) {
    let at = png
        .windows(4)
        .position(|w| w == b"eXIf")
        .expect("an eXIf chunk");
    let len = u32::from_be_bytes(png[at - 4..at].try_into().expect("4 bytes"));
    (at + 4, usize::try_from(len).expect("fits"))
}

/// Recompute the CRC of the `eXIf` chunk after its data changed, with the
/// oracle's own CRC-32, so the mutated PNG stays well formed.
fn fix_exif_crc(png: &mut [u8]) {
    let (start, len) = exif_chunk_at(png);
    let crc = oracle_png::crc32(&png[start - 4..start + len]);
    png[start + len..start + len + 4].copy_from_slice(&crc.to_be_bytes());
}

fn live_gate(test: &str) -> bool {
    if tools::exiftool_available() {
        return true;
    }
    if std::env::var(REQUIRE_VAR).as_deref() == Ok("1") {
        panic!("{test}: exiftool is not on PATH and {REQUIRE_VAR}=1 makes that a failure");
    }
    println!("SKIP {test} \u{2014} exiftool not on PATH");
    false
}

fn live_run(name: &str, bytes: &[u8]) -> Vec<Option<String>> {
    let path = tools::temp_file_with_ext(name, "png", bytes).expect("temp file");
    let run = tools::run_exiftool(&path, &oracle::png_names());
    let _ = std::fs::remove_file(&path);
    run.unwrap_or_else(|e| panic!("{name}: {e}"))
}

// Byte mutation b1. `png-exif-gps-full-le` has IFD0, an Exif IFD and a GPS IFD in
// its `eXIf` payload. The GPS pointer VALUE (an offset relative to the TIFF blob)
// is set past the end (the chunk CRC is recomputed, so the PNG stays valid).
// Compared with the ORIGINAL fixture's committed record: exactly the GPS
// selectors go red, nothing else.
#[test]
fn byte_mutation_corrupt_ifd_offset_in_exif_payload_is_named() {
    let test = "byte_mutation_corrupt_ifd_offset_in_exif_payload_is_named";
    let name = "png-exif-gps-full-le";
    let original = png_fixture(name);
    let record = theirs(name);
    // Control: the unmutated bytes agree with the record.
    assert!(compare(name, &ours_in_process(&original), &record).is_clean());

    let (start, len) = exif_chunk_at(&original);
    let tiff = &original[start..start + len];
    let order = mutate::order_of(tiff).expect("a TIFF inside the chunk");
    let ifd0 =
        usize::try_from(mutate::read_u32_at(tiff, 4, order).expect("IFD0 offset")).expect("fits");
    let loc = mutate::locate_entry(tiff, ifd0, 0x8825).expect("the GPS pointer is in IFD0");
    let mut mutated = original.clone();
    assert!(mutate::set_u32_at(
        &mut mutated,
        start + loc.value_at,
        0x00FF_0000,
        order
    ));
    fix_exif_crc(&mut mutated);
    assert_ne!(mutated, original);
    assert_eq!(
        mutated.len(),
        original.len(),
        "only the pointer value changed"
    );

    let ours = ours_in_process(&mutated);
    let cmp = compare(name, &ours, &record);
    println!(
        "{test}: GPS pointer value -> 0x00FF0000 at file offset {}: [{}]",
        start + loc.value_at,
        describe(&cmp.unexplained)
    );
    assert_eq!(
        named(&cmp.unexplained),
        sorted(GPS_SELECTORS.to_vec()),
        "{}",
        describe(&cmp.unexplained)
    );
    assert!(cmp
        .unexplained
        .iter()
        .all(|m| m.ours.is_none() && m.theirs.is_some()));
    assert!(cmp.stale.is_empty());
    // The PNG layer stays clean; the damage is one issue on the Exif block.
    let meta = exif_core::png::read(&mutated).expect("a damaged Exif is not fatal");
    assert!(meta.png_issues().is_empty(), "{:?}", meta.png_issues());
    assert!(!meta.is_clean());
    // Everything else (IFD0, the Exif IFD) still matches.
    let kept = oracle::png_names()
        .iter()
        .zip(&ours)
        .zip(&record)
        .filter(|((n, o), t)| o.is_some() && t.is_some() && !GPS_SELECTORS.contains(n))
        .count();
    assert!(kept >= 10, "only {kept} fields survived");

    // Live variant: ExifTool on the same mutated bytes agrees with ours.
    if !live_gate(&format!("{test} (live variant)")) {
        return;
    }
    let live = live_run("mutated-png-gps-pointer", &mutated);
    for (n, v) in oracle::png_names().iter().zip(&live) {
        if GPS_SELECTORS.contains(n) {
            assert!(v.is_none(), "live ExifTool still reports {n}: {v:?}");
        }
    }
    let live_cmp = compare(name, &ours, &live);
    assert!(
        live_cmp.unexplained.is_empty(),
        "ours and live ExifTool must agree on the mutated bytes: {}",
        describe(&live_cmp.unexplained)
    );
    println!("{test} (live variant): ExifTool agrees, GPS selectors are `-`");
}

// Byte mutation b2. `png-exif-gps-full-le` truncated between IHDR and IDAT. Cut
// AFTER the `eXIf` chunk: a typed outcome (the walk reports `MissingIend`, the
// Exif is read and equal to the original's), and clean against the original's
// record on the Exif selectors. Cut INSIDE the `eXIf` data: the chunk does not
// fit, we read nothing, and every selector ExifTool reports for the original goes
// red (live: ExifTool reads nothing from it either).
#[test]
fn byte_mutation_truncation_between_ihdr_and_idat_is_named() {
    let test = "byte_mutation_truncation_between_ihdr_and_idat_is_named";
    let name = "png-exif-gps-full-le";
    let original = png_fixture(name);
    let record = theirs(name);
    assert!(compare(name, &ours_in_process(&original), &record).is_clean());
    let (start, len) = exif_chunk_at(&original);

    // (a) After the chunk (data and CRC), before IDAT.
    let after = &original[..start + len + 4];
    let meta = exif_core::png::read(after).expect("a truncated PNG is not fatal");
    let kinds: Vec<&exif_core::Error> = meta.png_issues().iter().map(|i| i.error()).collect();
    assert_eq!(kinds, [&exif_core::Error::MissingIend]);
    assert_eq!(meta.exif_blocks().len(), 1);
    assert!(meta.exif_blocks()[0]
        .container()
        .is_ok_and(|c| c.is_clean()));
    let ours_after = ours_in_process(after);
    assert_eq!(
        &ours_after[..oracle::SELECTORS.len()],
        &ours_in_process(&original)[..oracle::SELECTORS.len()],
        "the cut loses no tag"
    );
    assert!(compare(name, &ours_after, &record).is_clean());

    // (b) Inside the chunk data.
    let inside = &original[..start + len / 2];
    let ours_inside = ours_in_process(inside);
    let cmp = compare(name, &ours_inside, &record);
    println!(
        "{test}: cut at file offset {} (inside the eXIf data): [{}]",
        start + len / 2,
        describe(&cmp.unexplained)
    );
    assert_eq!(
        named(&cmp.unexplained),
        reported(name),
        "{}",
        describe(&cmp.unexplained)
    );
    assert!(cmp
        .unexplained
        .iter()
        .all(|m| m.ours.is_none() && m.theirs.is_some()));
    // No prefix of the file panics, and every one is Ok or a typed fatal.
    for cut in 0..=original.len() {
        match exif_core::png::read(&original[..cut]) {
            Ok(m) => {
                let _ = oracle::read_png_all(&m);
                let _ = m.issues().count();
            }
            Err(e) => assert!(e.to_string().len() > 1, "{cut}: {e}"),
        }
    }

    // Live variant: ExifTool reads nothing from the cut-inside file either.
    if !live_gate(&format!("{test} (live variant)")) {
        return;
    }
    let live = live_run("truncated-png-inside-exif", inside);
    assert!(live.iter().all(Option::is_none), "{live:?}");
    assert!(compare(name, &ours_inside, &live).unexplained.is_empty());
    let live_after = live_run("truncated-png-after-exif", after);
    assert!(compare(name, &ours_after, &live_after)
        .unexplained
        .is_empty());
    println!("{test} (live variant): ExifTool agrees on both cuts");
}

// ── SPEC-039 AC10: writer mutants W1 and W3 ────────────────────────────────

use writer_mutants::Mutant;

/// W1: `tXMP` is dropped from `png::metadata::chunk_class`. The audit and the
/// writer both read `chunk_class`, so they agree the chunk is not metadata and
/// the self-check passes; ExifTool still decodes the chunk as XMP.
const W1: Mutant = Mutant {
    id: "W1",
    fixture: "png-carrier-tXMP",
    policy: "gps",
    file: "png/metadata.rs",
    needle: "b\"tXMP\" | b\"meTa\" | b\"seAl\" => Some(ChunkClass::XmlCarrier),",
    replacement: "b\"meTa\" | b\"seAl\" => Some(ChunkClass::XmlCarrier),",
};

/// W3: the `Copyright` keyword row maps to `FreeText`. An over-removal under
/// `AllPii`, which keeps `Rights`. Audit and writer both call
/// `text_keyword_category`.
const W3: Mutant = Mutant {
    id: "W3",
    fixture: "png-kw-04-copyright",
    policy: "all_pii",
    file: "audit/category.rs",
    needle: "(b\"Copyright\", Category::Rights),",
    replacement: "(b\"Copyright\", Category::FreeText),",
};

fn assert_four_results(m: &Mutant, name_needles: &[&str]) {
    let live = live_gate(&format!("writer mutant {} (live)", m.id));
    let r = writer_mutants::run(m, live);
    println!("AC10 {}", writer_mutants::report(m, &r));
    assert!(
        r.control_matches,
        "{}: the control must reproduce the committed digest",
        m.id
    );
    assert!(
        r.mutant_ok,
        "{}: the mutant's strip must return Ok (escape the self-check)",
        m.id
    );
    let replay = r
        .replay_red
        .as_deref()
        .unwrap_or_else(|| panic!("{}: replay must go red on the digest, and did not", m.id));
    println!("AC10 {} replay red: {replay}", m.id);
    if let Some(failures) = &r.live_failures {
        assert!(
            failures
                .iter()
                .any(|f| name_needles.iter().all(|n| f.contains(n))),
            "{}: live must go red naming {name_needles:?}, got {failures:?}",
            m.id
        );
        println!(
            "AC10 {} live red: {}",
            m.id,
            failures
                .iter()
                .find(|f| name_needles.iter().all(|n| f.contains(n)))
                .cloned()
                .unwrap_or_default()
        );
    }
}

#[test]
fn writer_mutant_w1_txmp_not_a_carrier_escapes_the_self_check_and_i3_names_it() {
    assert_four_results(
        &W1,
        &["png-carrier-tXMP", "under gps", "carrier tag left in XMP"],
    );
}

#[test]
fn writer_mutant_w3_copyright_as_free_text_is_named() {
    assert_four_results(
        &W3,
        &[
            "png-kw-04-copyright",
            "under all_pii",
            "kept tag missing PNG:Copyright",
        ],
    );
}

// ── The injection scaffolding is itself guarded ────────────────────────────

/// Every file under `dir` (recursively, skipping `target` and `.git`) that
/// contains the injection marker, except `skip` (by file name).
fn marker_hits(dir: &Path, skip: &[&str], hits: &mut Vec<PathBuf>, scanned: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        let name = e.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        if path.is_dir() {
            marker_hits(&path, skip, hits, scanned);
        } else if skip.iter().any(|s| name == *s) {
            continue;
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            scanned.push(path.clone());
            if text.contains(MARKER) {
                hits.push(path);
            }
        }
    }
}

/// No `RED-PROOF INJECTION` marker in the library, the examples, the fuzz graph,
/// the scripts, or any support file (`probe.rs` alone DEFINES it): an injection
/// can only live in a staged copy. The PNG apparatus adds `oracle_png.rs` and the
/// PNG branch of `probe.rs`, so the scan is checked to cover the former.
#[test]
fn injection_marker_does_not_leak_into_src_or_tests_support() {
    let mut hits = Vec::new();
    let mut scanned = Vec::new();
    for dir in ["src", "examples", "fuzz", "scripts"] {
        marker_hits(&root().join(dir), &[], &mut hits, &mut scanned);
    }
    marker_hits(
        &root().join("tests/support"),
        &["probe.rs"],
        &mut hits,
        &mut scanned,
    );
    assert!(hits.is_empty(), "injection marker leaked into: {hits:?}");
    let support = root().join("tests/support");
    for needed in [
        "oracle.rs",
        "oracle_png.rs",
        "oracle_tiff.rs",
        "tools.rs",
        "mutate.rs",
    ] {
        assert!(
            scanned.contains(&support.join(needed)),
            "{needed} is not in the scanned set"
        );
    }
    assert!(
        scanned.contains(&root().join("src/png/metadata.rs")),
        "the file the PNG mutants inject into is scanned"
    );
    // The PNG mutants' needles are in the file they inject into, exactly once
    // each (so a moved injection point fails here too, without a build).
    let src = std::fs::read_to_string(root().join("src/png/metadata.rs")).expect("read");
    for needle in [
        "None => (data, body_at),",
        "self.blocks.push(PngExifBlock {",
        "data.strip_prefix(EXIF_PREFIX)",
        "for _ in 0..MAX_PNG_TRAILER_CHUNKS {",
        "let region = if keyword == XMP_KEYWORD && !tagged {",
        "ChunkKind::Iccp => {",
        "let compressed = flag != 0;",
    ] {
        assert_eq!(src.matches(needle).count(), 1, "needle `{needle}`");
    }
    // The scan does catch a marker in a file named like a support file.
    let dir = TempDir::new("png-marker-scan");
    let fake = dir.path().join("tests/support");
    std::fs::create_dir_all(&fake).expect("mkdir");
    for name in ["oracle.rs", "oracle_png.rs"] {
        std::fs::write(fake.join(name), format!("let x = 1; {MARKER}\n")).expect("write");
    }
    let mut found = Vec::new();
    marker_hits(&fake, &["probe.rs"], &mut found, &mut Vec::new());
    assert_eq!(found.len(), 2, "{found:?}");
}

// ── The live variants skip loudly without ExifTool ─────────────────────────

fn run_self_without_exiftool(test: &str, require: bool) -> std::process::Output {
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env("PATH", "/nonexistent-diamond-exif-path");
    if require {
        cmd.env(REQUIRE_VAR, "1");
    } else {
        cmd.env_remove(REQUIRE_VAR);
    }
    cmd.output().expect("spawn the test binary")
}

#[test]
fn the_live_variants_skip_loudly_and_required_mode_fails() {
    for test in [
        "byte_mutation_corrupt_ifd_offset_in_exif_payload_is_named",
        "byte_mutation_truncation_between_ihdr_and_idat_is_named",
    ] {
        let out = run_self_without_exiftool(test, false);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{test}: should pass when the live half is skipped\n{stdout}"
        );
        assert!(
            stdout.contains(&format!("SKIP {test} (live variant)")),
            "{test}: expected a SKIP line, got:\n{stdout}"
        );
        let out = run_self_without_exiftool(test, true);
        let both = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !out.status.success(),
            "{test}: {REQUIRE_VAR}=1 must fail\n{both}"
        );
        assert!(
            both.contains(&format!(
                "{test} (live variant): exiftool is not on PATH and {REQUIRE_VAR}=1"
            )),
            "{test}: failed, but not with the required-mode message\n{both}"
        );
    }
}
