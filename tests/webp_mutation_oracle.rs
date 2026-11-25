//! Red-proofs for the WebP ExifTool comparison (SPEC-034): a FAULT in the WebP
//! layer of the library, or a corruption of a WebP, must turn the comparison
//! with ExifTool's committed answers red, naming the selectors it should.
//!
//! Same apparatus as `tests/png_mutation_oracle.rs`, unchanged:
//! `tests/support/probe.rs` stages a throw-away copy of `src/`, injects one
//! expression marked `RED-PROOF INJECTION` located by a needle that must occur
//! exactly once, builds a probe crate and runs it over each fixture; the probe
//! reads a file that starts `RIFF` through `webp::read` and the harness model
//! (`oracle::read_webp_all`). `just mutation-oracle` runs the three files.
//!
//! Every library fault has the oracle-must-be-shown-red shape: a CONTROL first
//! (the unmutated apparatus equals ExifTool's committed record on the same
//! fixtures, zero unexplained mismatches), then the mutant, which must produce
//! the expected selectors and no others. Everything compares against the
//! COMMITTED records, so the red-proofs need no ExifTool; only the live variants
//! of the byte mutations use it and skip loudly without it
//! (`DIAMOND_EXIF_REQUIRE_ORACLE=1` makes that a failure). `src/` is never
//! modified.

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

const MAKE: &str = "IFD0:Make";
const MODEL: &str = "IFD0:Model";
const XMP: &str = "XMP-xmp:CreatorTool";
const ICC: &str = "ICC-header:ProfileFileSignature";

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

fn webp_fixture(name: &str) -> Vec<u8> {
    oracle_webp::fixtures()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no WebP fixture named {name}"))
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
    tools::parse_fields(&oracle::webp_names(), &rec.stdout)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Our in-process reading of a WebP (22 fields).
fn ours_in_process(bytes: &[u8]) -> Vec<Option<String>> {
    match exif_core::webp::read(bytes) {
        Ok(m) => oracle::read_webp_all(&m),
        Err(_) => vec![None; oracle::webp_names().len()],
    }
}

/// Stage + build a probe crate, run it on each WebP fixture and compare with the
/// committed record (divergences applied). `Err` only for apparatus failures.
fn webp_probe(
    label: &str,
    fixtures: &[&str],
    inject: Option<&dyn Fn(&Path)>,
) -> Result<Vec<(String, JpegComparison)>, String> {
    let files = TempDir::new(&format!("{label}-files"));
    let paths: Vec<(String, PathBuf)> = fixtures
        .iter()
        .map(|n| {
            let path = files.path().join(format!("{n}.webp"));
            std::fs::write(&path, webp_fixture(n)).expect("write fixture");
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
            oracle::webp_names().len(),
            "{name}: the probe's WebP mode prints one field per WebP selector"
        );
        out.push((
            name.clone(),
            oracle::compare_webp(name, &ours, &theirs(name)),
        ));
    }
    Ok(out)
}

fn assert_control_clean(label: &str, fixtures: &[&str]) {
    let control = webp_probe(&format!("{label}-control"), fixtures, None)
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
    webp_probe(&format!("{label}-mutant"), fixtures, Some(inject))
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
        oracle::webp_names()
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

/// A fixture the mutant must leave clean.
fn assert_clean(all: &[(String, JpegComparison)], fixture: &str) {
    let c = cmp_for(all, fixture);
    assert!(c.is_clean(), "{fixture}: {}", describe(&c.unexplained));
}

/// Exactly these selectors are named, nothing stale.
fn assert_names(all: &[(String, JpegComparison)], fixture: &str, want: &[&'static str]) {
    let c = cmp_for(all, fixture);
    assert_eq!(
        named(&c.unexplained),
        sorted(want.to_vec()),
        "{fixture}: {}",
        describe(&c.unexplained)
    );
    assert!(c.stale.is_empty(), "{fixture}: {:?}", c.stale);
}

/// Every fixture any WebP mutant uses.
const MUTANT_FIXTURES: &[&str] = &[
    "webp-exif-basic",
    "webp-exif-gps-ne-le",
    "webp-exif-prefix",
    "webp-exif-good-then-prefixed",
    "webp-exif-two-same-tag",
    "webp-exif-two-same-tag-reversed",
    "webp-exif-bad-then-good",
    "webp-riff-size-0",
    "webp-riff-size-small",
    "webp-riff-size-10",
    "webp-riff-size-ffff-fff0",
    "webp-riff-size-exact",
    "webp-xmp-odd-unpadded-then-exif",
    "webp-xmp-nul-id",
    "webp-xmp-nul-id-1-bytes",
    "webp-xmp-basic",
    "webp-icc-valid-flag-set",
    "webp-icc-garbage",
    "webp-icc-empty",
    "webp-exif-kind-capital-e",
    "webp-exif-kind-lower",
    "webp-exif-kind-last-letter-lower",
    "webp-xmp-kind-lower",
    "webp-xmp-kind-dash",
    "webp-icc-kind-lower",
];

#[test]
fn controls_are_green() {
    // One control over every fixture, the hand-built list and the wrapped TIFFs:
    // the unmutated apparatus (copy, build, WebP-mode probe, rendering)
    // reproduces the committed ExifTool answers, with the known divergences
    // exercised.
    // Not the fixtures `webp::read` rejects (11 bytes, `RIFX`, `WAVE`, `AVI `):
    // the probe reports a parse error with exit status 3 for them, which is the
    // apparatus saying "not a WebP"; their all-`None` reading and divergences
    // are the replay test's.
    let all: Vec<String> = oracle_webp::fixtures()
        .into_iter()
        .filter(|(_, b)| exif_core::webp::read(b).is_ok())
        .map(|(n, _)| n)
        .collect();
    for used in MUTANT_FIXTURES {
        assert!(all.iter().any(|n| n == used), "{used} is not a fixture");
    }
    let refs: Vec<&str> = all.iter().map(String::as_str).collect();
    assert_control_clean("webp-all-fixtures", &refs);

    // And the control is not vacuous: the probe's reading is the in-process one.
    let files = TempDir::new("webp-control-same-files");
    let crate_dir = TempDir::new("webp-control-same");
    probe::stage_probe_crate(crate_dir.path(), None);
    let bin = probe::build_probe(crate_dir.path()).expect("build");
    for name in [
        "webp-exif-gps-ne-le",
        "webp-exif-two-same-tag",
        "webp-xmp-basic",
        "webp-icc-valid-flag-set",
        "webp-all-three",
    ] {
        let path = files.path().join(format!("{name}.webp"));
        std::fs::write(&path, webp_fixture(name)).expect("write");
        let via_probe = probe::run_probe(&bin, &path).expect("run");
        assert_eq!(via_probe, ours_in_process(&webp_fixture(name)), "{name}");
    }
}

// Mutant w1. Needle (src/webp/metadata.rs, `Extract::chunk`, the EXIF arm):
// `None => (data, body_at),`. The TIFF slice starts one byte late, so no TIFF
// header is found and no tag is read.
#[test]
fn mutant_exif_tiff_offset_wrong_by_one_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("webp/metadata.rs"),
            "None => (data, body_at),",
            &marked("None => (data.get(1..).unwrap_or(data), body_at),"),
        );
    };
    let fixtures = ["webp-exif-basic", "webp-exif-gps-ne-le"];
    let got = control_then_mutant("webp-w1-offset", &fixtures, &inject);
    for f in fixtures {
        assert_reads_nothing(&got, f);
    }
    assert!(cmp_for(&got, "webp-exif-basic")
        .unexplained
        .iter()
        .any(|m| m.selector == MAKE));
}

// Mutant w2: the LAST block is not the winner (the first wins instead, the PNG
// rule). Needle: `self.blocks.push(WebpExifBlock {`. Each new block is put in
// FRONT, so the harness model (which reads the last block first) reads the first
// block in file order. Visible on the two-block fixtures, in either order.
#[test]
fn mutant_first_block_wins_instead_of_the_last_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("webp/metadata.rs"),
            "self.blocks.push(WebpExifBlock {",
            &marked("self.blocks.insert(0, WebpExifBlock {"),
        );
    };
    let fixtures = [
        "webp-exif-two-same-tag",
        "webp-exif-two-same-tag-reversed",
        "webp-exif-good-then-prefixed",
        "webp-exif-basic",
    ];
    let got = control_then_mutant("webp-w2-order", &fixtures, &inject);
    // Make differs (the wrong block wins); Model is in A only and survives the merge.
    for f in &fixtures[..3] {
        assert_names(&got, f, &[MAKE]);
    }
    // A single block has no order to get wrong.
    assert_clean(&got, "webp-exif-basic");
}

// Mutant w3: the `Exif\0\0` prefix is not skipped. Needle: `data.strip_prefix(EXIF_PREFIX)`.
#[test]
fn mutant_exif_prefix_is_not_skipped_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("webp/metadata.rs"),
            "data.strip_prefix(EXIF_PREFIX)",
            &marked("data.strip_prefix(b\"\\xFF\\xFF\\xFF\\xFF\\xFF\\xFF\")"),
        );
    };
    let fixtures = [
        "webp-exif-prefix",
        "webp-exif-good-then-prefixed",
        "webp-exif-basic",
    ];
    let got = control_then_mutant("webp-w3-prefix", &fixtures, &inject);
    assert_reads_nothing(&got, "webp-exif-prefix");
    // The later (prefixed) block is lost: A's Make instead of B's.
    assert_names(&got, "webp-exif-good-then-prefixed", &[MAKE]);
    // The canonical (prefix-free) payload is unaffected.
    assert_clean(&got, "webp-exif-basic");
}

// Mutant w4: the declared RIFF size limits the walk. Needle (src/webp/mod.rs,
// `Walk::walk`): `let len = self.data.len();`. The walk stops at the declared
// size, as a strict RIFF reader would; ExifTool does not (DEC-019).
#[test]
fn mutant_declared_riff_size_limits_the_walk_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("webp/mod.rs"),
            "let len = self.data.len();",
            &marked(
                "let len = self.data.len().min(le_u32(self.data, 4).map_or(0, |d| \
                 usize::try_from(d).unwrap_or(0)).saturating_add(8));",
            ),
        );
    };
    let fixtures = [
        "webp-riff-size-0",
        "webp-riff-size-small",
        "webp-riff-size-10",
        "webp-riff-size-ffff-fff0",
        "webp-riff-size-exact",
    ];
    let got = control_then_mutant("webp-w4-size", &fixtures, &inject);
    for f in [
        "webp-riff-size-0",
        "webp-riff-size-small",
        "webp-riff-size-10",
    ] {
        assert_reads_nothing(&got, f);
        assert_names(&got, f, &[MAKE, MODEL]);
    }
    // A size that is exact, or larger than the file, never limits anything.
    assert_clean(&got, "webp-riff-size-ffff-fff0");
    assert_clean(&got, "webp-riff-size-exact");
}

// Mutant w5: the pad rule is removed. Needle (src/webp/mod.rs): `let end = if odd
// && data_end < len {`. The pad byte of an odd chunk is read as the start of the
// next chunk: every fixture whose `VP8L` (5 bytes) is followed by a chunk loses it.
#[test]
fn mutant_pad_rule_removed_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("webp/mod.rs"),
            "let end = if odd && data_end < len {",
            &marked("let end = if false && odd && data_end < len {"),
        );
    };
    let fixtures = [
        "webp-exif-basic",
        "webp-exif-no-vp8x",
        "webp-xmp-odd-unpadded-then-exif",
    ];
    let got = control_then_mutant("webp-w5-pad", &fixtures, &inject);
    assert_reads_nothing(&got, "webp-exif-basic");
    assert_names(&got, "webp-exif-basic", &[MAKE, MODEL]);
    assert_reads_nothing(&got, "webp-exif-no-vp8x");
    // The misaligned VP8L pad also costs the XMP before the (already lost)
    // EXIF in the unpadded-XMP shape: it is named on the XMP selector.
    assert_names(&got, "webp-xmp-odd-unpadded-then-exif", &[XMP]);
}

// Mutant w6: `XMP\0` is not accepted. Needle: `b"XMP\0" => (ChunkKind::Xmp, true),`.
#[test]
fn mutant_xmp_nul_id_not_accepted_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("webp/mod.rs"),
            "b\"XMP\\0\" => (ChunkKind::Xmp, true),",
            &marked("b\"XMP\\0\" => (ChunkKind::Other, false),"),
        );
    };
    let fixtures = [
        "webp-xmp-nul-id",
        "webp-xmp-nul-id-1-bytes",
        "webp-xmp-basic",
    ];
    let got = control_then_mutant("webp-w6-xmp-nul", &fixtures, &inject);
    assert_reads_nothing(&got, "webp-xmp-nul-id");
    assert_names(&got, "webp-xmp-nul-id", &[XMP]);
    // The recorded divergence (a located garbage packet) disappears: stale.
    let short = cmp_for(&got, "webp-xmp-nul-id-1-bytes");
    assert!(short.unexplained.is_empty(), "{:?}", short.unexplained);
    assert_eq!(short.stale.len(), 1, "{:?}", short.stale);
    assert_clean(&got, "webp-xmp-basic");
}

// Mutant w7: `ICCP` is not located. Needle (src/webp/metadata.rs):
// `ChunkKind::Iccp => self.regions.push(WebpRegion::Icc { offset, data }),`.
#[test]
fn mutant_iccp_not_located_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("webp/metadata.rs"),
            "ChunkKind::Iccp => self.regions.push(WebpRegion::Icc { offset, data }),",
            &marked(
                "ChunkKind::Iccp if false => self.regions.push(WebpRegion::Icc { offset, data }),",
            ),
        );
    };
    let fixtures = [
        "webp-icc-valid-flag-set",
        "webp-icc-garbage",
        "webp-xmp-basic",
    ];
    let got = control_then_mutant("webp-w7-iccp", &fixtures, &inject);
    assert_reads_nothing(&got, "webp-icc-valid-flag-set");
    assert_names(&got, "webp-icc-valid-flag-set", &[ICC]);
    // For the garbage profile ExifTool prints nothing, so only the recorded
    // divergence turns stale (we no longer locate).
    let bad = cmp_for(&got, "webp-icc-garbage");
    assert!(bad.unexplained.is_empty(), "{:?}", bad.unexplained);
    assert_eq!(bad.stale.len(), 1, "{:?}", bad.stale);
    assert_clean(&got, "webp-xmp-basic");
}

// Mutant w8: four-character codes are compared case-insensitively. Needle
// (src/webp/mod.rs, `kind_of`): `match &code {`. `Exif`, `exif`, `EXIf`, `xmp `
// and `iccp` are now read; ExifTool reads none of them.
#[test]
fn mutant_chunk_codes_case_insensitive_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("webp/mod.rs"),
            "match &code {",
            &marked("match &code.map(|b| b.to_ascii_uppercase()) {"),
        );
    };
    let fixtures = [
        "webp-exif-kind-capital-e",
        "webp-exif-kind-lower",
        "webp-exif-kind-last-letter-lower",
        "webp-xmp-kind-lower",
        "webp-icc-kind-lower",
        "webp-xmp-kind-dash",
        "webp-exif-basic",
    ];
    let got = control_then_mutant("webp-w8-case", &fixtures, &inject);
    for f in [
        "webp-exif-kind-capital-e",
        "webp-exif-kind-lower",
        "webp-exif-kind-last-letter-lower",
    ] {
        assert_names(&got, f, &[MAKE, MODEL]);
    }
    assert_names(&got, "webp-xmp-kind-lower", &[XMP]);
    assert_names(&got, "webp-icc-kind-lower", &[ICC]);
    // A code that differs by more than case is untouched.
    assert_clean(&got, "webp-xmp-kind-dash");
    assert_clean(&got, "webp-exif-basic");
}

// ── Byte-mutation red-proofs (no probe crate: mutate the bytes in process) ──

/// Where the TIFF blob of the (single) `EXIF` chunk starts in a fixture's bytes,
/// and its length, found WITHOUT the library under test: the first `EXIF` code
/// plus 8, the length being the little-endian word after the code.
fn exif_chunk_at(webp: &[u8]) -> (usize, usize) {
    let at = webp
        .windows(4)
        .position(|w| w == b"EXIF")
        .expect("an EXIF chunk");
    let len = u32::from_le_bytes(webp[at + 4..at + 8].try_into().expect("4 bytes"));
    (at + 8, usize::try_from(len).expect("fits"))
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
    let path = tools::temp_file_with_ext(name, "webp", bytes).expect("temp file");
    let run = tools::run_exiftool(&path, &oracle::webp_names());
    let _ = std::fs::remove_file(&path);
    run.unwrap_or_else(|e| panic!("{name}: {e}"))
}

// Byte mutation b1. `webp-exif-gps-full-le` has IFD0, an Exif IFD and a GPS IFD
// in its `EXIF` payload. The GPS pointer VALUE (an offset relative to the TIFF
// blob) is set past the end (a WebP chunk has no CRC, so nothing else changes).
// Compared with the ORIGINAL fixture's committed record: exactly the GPS
// selectors go red, nothing else.
#[test]
fn byte_mutation_corrupt_ifd_offset_in_exif_payload_is_named() {
    let test = "byte_mutation_corrupt_ifd_offset_in_exif_payload_is_named";
    let name = "webp-exif-gps-full-le";
    let original = webp_fixture(name);
    let record = theirs(name);
    // Control: the unmutated bytes agree with the record.
    assert!(oracle::compare_webp(name, &ours_in_process(&original), &record).is_clean());

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
    assert_ne!(mutated, original);
    assert_eq!(
        mutated.len(),
        original.len(),
        "only the pointer value changed"
    );

    let ours = ours_in_process(&mutated);
    let cmp = oracle::compare_webp(name, &ours, &record);
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
    // The WebP layer stays clean; the damage is one issue on the Exif block.
    let meta = exif_core::webp::read(&mutated).expect("a damaged Exif is not fatal");
    assert!(meta.webp().is_clean(), "{:?}", meta.webp_issues());
    assert!(!meta.is_clean());
    // Everything else (IFD0, the Exif IFD) still matches.
    let kept = oracle::webp_names()
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
    let live = live_run("mutated-webp-gps-pointer", &mutated);
    for (n, v) in oracle::webp_names().iter().zip(&live) {
        if GPS_SELECTORS.contains(n) {
            assert!(v.is_none(), "live ExifTool still reports {n}: {v:?}");
        }
    }
    let live_cmp = oracle::compare_webp(name, &ours, &live);
    assert!(
        live_cmp.unexplained.is_empty(),
        "ours and live ExifTool must agree on the mutated bytes: {}",
        describe(&live_cmp.unexplained)
    );
    println!("{test} (live variant): ExifTool agrees, GPS selectors are `-`");
}

// Byte mutation b2. `webp-exif-gps-full-le` truncated between `VP8L` and `EXIF`
// (the cut is exactly at the start of the `EXIF` chunk): both sides read no tag,
// so every selector ExifTool reports for the original goes red. Cut AFTER the
// `EXIF` chunk: nothing is lost. Cut INSIDE the `EXIF` data: the chunk does not
// fit, we read nothing, and ExifTool reads nothing from it either (live).
#[test]
fn byte_mutation_truncation_between_vp8l_and_exif_is_named() {
    let test = "byte_mutation_truncation_between_vp8l_and_exif_is_named";
    let name = "webp-exif-gps-full-le";
    let original = webp_fixture(name);
    let record = theirs(name);
    assert!(oracle::compare_webp(name, &ours_in_process(&original), &record).is_clean());
    let (start, len) = exif_chunk_at(&original);

    // (a) Between VP8L and EXIF: the chunk is gone, the walk ends cleanly at the
    // end of the input.
    let before = &original[..start - 8];
    let meta = exif_core::webp::read(before).expect("a truncated WebP is not fatal");
    let kinds: Vec<String> = meta
        .webp_issues()
        .iter()
        .map(|i| format!("{:?}", i.error()))
        .collect();
    // The RIFF size no longer matches, and the VP8X EXIF flag has no chunk.
    assert_eq!(kinds.len(), 2, "{kinds:?}");
    assert!(kinds[0].starts_with("RiffSizeMismatch"), "{kinds:?}");
    assert!(kinds[1].starts_with("ChunkFlagMismatch"), "{kinds:?}");
    assert!(meta.exif_blocks().is_empty());
    let ours_before = ours_in_process(before);
    let cmp = oracle::compare_webp(name, &ours_before, &record);
    println!(
        "{test}: cut at file offset {} (before the EXIF chunk): [{}]",
        start - 8,
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

    // (b) After the chunk (its data and pad): nothing is lost on the Exif side.
    let after = &original[..start + len + len % 2];
    let ours_after = ours_in_process(after);
    assert_eq!(
        &ours_after[..oracle::SELECTORS.len()],
        &ours_in_process(&original)[..oracle::SELECTORS.len()],
        "the cut loses no tag"
    );
    assert!(oracle::compare_webp(name, &ours_after, &record).is_clean());

    // (c) Inside the chunk data.
    let inside = &original[..start + len / 2];
    let ours_inside = ours_in_process(inside);
    let cmp = oracle::compare_webp(name, &ours_inside, &record);
    assert_eq!(
        named(&cmp.unexplained),
        reported(name),
        "{}",
        describe(&cmp.unexplained)
    );
    // No prefix of the file panics, and every one is Ok or a typed fatal.
    for cut in 0..=original.len() {
        match exif_core::webp::read(&original[..cut]) {
            Ok(m) => {
                let _ = oracle::read_webp_all(&m);
                let _ = m.issues().count();
            }
            Err(e) => assert!(e.to_string().len() > 1, "{cut}: {e}"),
        }
    }

    // Live variant: ExifTool reads nothing from either cut file.
    if !live_gate(&format!("{test} (live variant)")) {
        return;
    }
    let live_before = live_run("truncated-webp-before-exif", before);
    assert!(live_before.iter().all(Option::is_none), "{live_before:?}");
    assert!(oracle::compare_webp(name, &ours_before, &live_before)
        .unexplained
        .is_empty());
    let live_inside = live_run("truncated-webp-inside-exif", inside);
    assert!(live_inside.iter().all(Option::is_none), "{live_inside:?}");
    assert!(oracle::compare_webp(name, &ours_inside, &live_inside)
        .unexplained
        .is_empty());
    println!("{test} (live variant): ExifTool agrees on both cuts");
}

// ── SPEC-039 AC10: writer mutants W2 and W4 ────────────────────────────────

use writer_mutants::Mutant;

/// W2: `SEAL` is dropped from `webp::metadata::chunk_class`. Audit and writer
/// both read it, so the self-check passes; ExifTool still decodes the chunk.
const W2: Mutant = Mutant {
    id: "W2",
    fixture: "webp-row-05-seal",
    policy: "gps",
    file: "webp/metadata.rs",
    needle:
        "b\"_PMX\" | b\"SEAL\" | b\"aXML\" | b\"iXML\" | b\"JUNQ\" => Some(ChunkClass::XmlCarrier),",
    replacement: "b\"_PMX\" | b\"aXML\" | b\"iXML\" | b\"JUNQ\" => Some(ChunkClass::XmlCarrier),",
};

/// W4: `C2PA` is an `XmlCarrier` instead of `Jumbf`. An over-removal under
/// `All`, which keeps `Jumbf` while a carrier goes under every preset.
const W4: Mutant = Mutant {
    id: "W4",
    fixture: "webp-row-22-c2pa",
    policy: "all",
    file: "webp/metadata.rs",
    needle: "b\"C2PA\" => Some(ChunkClass::Jumbf),",
    replacement: "b\"C2PA\" => Some(ChunkClass::XmlCarrier),",
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
fn writer_mutant_w2_seal_not_a_carrier_escapes_the_self_check_and_i3_names_it() {
    assert_four_results(
        &W2,
        &["webp-row-05-seal", "under gps", "carrier tag left in"],
    );
}

#[test]
fn writer_mutant_w4_c2pa_as_carrier_is_named() {
    assert_four_results(
        &W4,
        &["webp-row-22-c2pa", "under all", "kept tag missing JUMBF:"],
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
/// can only live in a staged copy. The scan is checked to cover the WebP files.
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
        "oracle_webp.rs",
        "oracle_tiff.rs",
        "tools.rs",
        "mutate.rs",
    ] {
        assert!(
            scanned.contains(&support.join(needed)),
            "{needed} is not in the scanned set"
        );
    }
    for file in ["src/webp/metadata.rs", "src/webp/mod.rs"] {
        assert!(
            scanned.contains(&root().join(file)),
            "{file}, a file the WebP mutants inject into, is scanned"
        );
    }
    // The WebP mutants' needles are in the file they inject into, exactly once
    // each (so a moved injection point fails here too, without a build).
    for (file, needles) in [
        (
            "src/webp/metadata.rs",
            vec![
                "None => (data, body_at),",
                "self.blocks.push(WebpExifBlock {",
                "data.strip_prefix(EXIF_PREFIX)",
                "ChunkKind::Iccp => self.regions.push(WebpRegion::Icc { offset, data }),",
            ],
        ),
        (
            "src/webp/mod.rs",
            vec![
                "let len = self.data.len();",
                "let end = if odd && data_end < len {",
                "b\"XMP\\0\" => (ChunkKind::Xmp, true),",
                "match &code {",
            ],
        ),
    ] {
        let src = std::fs::read_to_string(root().join(file)).expect("read");
        for needle in needles {
            assert_eq!(src.matches(needle).count(), 1, "{file}: needle `{needle}`");
        }
    }
    // The scan does catch a marker in a file named like a support file.
    let dir = TempDir::new("webp-marker-scan");
    let fake = dir.path().join("tests/support");
    std::fs::create_dir_all(&fake).expect("mkdir");
    for name in ["oracle.rs", "oracle_webp.rs"] {
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
        "byte_mutation_truncation_between_vp8l_and_exif_is_named",
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
