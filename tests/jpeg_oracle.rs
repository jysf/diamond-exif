//! ExifTool read-parity oracle for JPEG (SPEC-011).
//!
//! The same shape as `tests/metadata_oracle.rs`: live tests run the installed
//! ExifTool (`-T -n -s3`) over the hand-built JPEG fixtures and the committed
//! `jpeg_segments` seeds and compare with what `exif_core::jpeg::read` reads;
//! replay tests run the SAME comparator against the committed records
//! (`tests/oracle-fixtures/exiftool/jpeg-*.txt`) with no tool installed. A live
//! test with no ExifTool prints `SKIP` and passes, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`.
//!
//! What is compared: the 20 Exif value selectors of `oracle::SELECTORS`, read
//! through the harness's model of ExifTool's several-Exif rule
//! (`oracle::read_ours_jpeg`: union of tags, the later block wins), plus the
//! presence of an XMP packet and an IPTC resource on the hand-built fixtures.
//! Every measured difference is a `KNOWN_DIVERGENCES` entry, and the tests
//! require each one to be PRESENT, so the list cannot rot.
//!
//! ExifTool is run, never read or linked. Every claim about its output is in a
//! committed record or in a `KNOWN_DIVERGENCES` entry with a command and a
//! version.

#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/tools.rs"]
mod tools;

use exif_core::jpeg::{self, SegmentKind};
use exif_core::tiff::Container;
use exif_core::Error;
use oracle::{Mismatch, Selector, SELECTORS};
use std::path::PathBuf;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn record_path(name: &str) -> PathBuf {
    root()
        .join("tests/oracle-fixtures/exiftool")
        .join(format!("{name}.txt"))
}

fn read_record(name: &str) -> tools::Record {
    let path = record_path(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (run `just oracle-record`)",
            path.display()
        )
    });
    tools::parse_record(&text).unwrap_or_else(|| panic!("{}: bad record header", path.display()))
}

/// The committed `jpeg_segments` seeds, sorted by name.
fn jpeg_seed_files() -> Vec<(String, Vec<u8>)> {
    let dir = root().join("fuzz/seeds/jpeg_segments");
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .expect("read seeds dir")
        .map(|e| {
            let path = e.expect("dir entry").path();
            let name = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("read seed"))
        })
        .collect();
    out.sort();
    out
}

fn fixture(name: &str) -> Vec<u8> {
    oracle_jpeg::fixtures()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no JPEG fixture named {name}"))
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

fn note_version(test: &str, recorded: &str) {
    let live = tools::exiftool_version().unwrap_or_else(|e| format!("unknown ({e})"));
    println!("{test}: live exiftool {live}");
    if live != recorded {
        println!("NOTE {test}: live exiftool {live} differs from recorded {recorded}");
    }
}

fn recorded_version() -> String {
    read_record("jpeg-exif-basic-le").version
}

/// The ExifTool fields of a hand-built fixture's record (22: Exif, then regions).
fn theirs_fixture(name: &str) -> Vec<Option<String>> {
    let rec = read_record(name);
    tools::parse_fields(&oracle::jpeg_names(), &rec.stdout)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The ExifTool fields of a seed's record (20: Exif only), padded with the two
/// region slots as `None` so the 22-wide comparator applies.
fn theirs_seed(name: &str) -> Vec<Option<String>> {
    let rec = read_record(&format!("jpeg-seed-{name}"));
    let mut v = tools::parse_fields(&oracle::names(SELECTORS), &rec.stdout)
        .unwrap_or_else(|e| panic!("seed {name}: {e}"));
    v.extend(oracle::REGION_SELECTORS.iter().map(|_| None));
    v
}

/// Our 22-wide reading of a hand-built fixture: all `None` when the input is not
/// a JPEG at all.
fn ours_fixture(bytes: &[u8]) -> Vec<Option<String>> {
    match jpeg::read(bytes) {
        Ok(m) => oracle::read_jpeg_all(&m),
        Err(_) => vec![None; SELECTORS.len() + oracle::REGION_SELECTORS.len()],
    }
}

/// Our reading of a seed: the Exif selectors only (regions are not compared on
/// seeds, which carry packets that need not hold the compared tag).
fn ours_seed(bytes: &[u8]) -> Vec<Option<String>> {
    let mut v = match jpeg::read(bytes) {
        Ok(m) => oracle::read_ours_jpeg(&m, SELECTORS),
        Err(_) => vec![None; SELECTORS.len()],
    };
    v.extend(oracle::REGION_SELECTORS.iter().map(|_| None));
    v
}

fn describe(m: &[Mismatch]) -> String {
    m.iter()
        .map(|m| format!("{m}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn named(m: &[Mismatch]) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = m.iter().map(|m| m.selector).collect();
    v.sort_unstable();
    v
}

/// The index of an Exif selector by its ExifTool spelling.
fn index_of(selector: &str) -> usize {
    oracle::jpeg_names()
        .iter()
        .position(|n| *n == selector)
        .unwrap_or_else(|| panic!("{selector} is not a JPEG selector"))
}

fn hand_built_names() -> Vec<String> {
    oracle_jpeg::fixtures()
        .into_iter()
        .map(|(n, _)| n)
        .collect()
}

/// ExifTool's Exif fields of the record that pins a wildcard fixture, if any.
fn pin_for(name: &str) -> Option<Vec<Option<String>>> {
    let record = oracle::exif_pin_record(name)?;
    let rec = read_record(record);
    Some(
        rec.stdout
            .trim_end_matches('\n')
            .split('\t')
            .take(SELECTORS.len())
            .map(|f| if f == "-" { None } else { Some(f.to_string()) })
            .collect(),
    )
}

/// `oracle::compare_jpeg` with the pin a wildcard fixture needs (M1).
fn compare(
    name: &str,
    ours: &[Option<String>],
    theirs: &[Option<String>],
) -> oracle::JpegComparison {
    oracle::compare_jpeg_pinned(name, ours, theirs, pin_for(name).as_deref())
}

// ── Fixtures ───────────────────────────────────────────────────────────────

#[test]
fn jpeg_fixtures_are_deterministic_and_uniquely_named() {
    let a = oracle_jpeg::fixtures();
    let b = oracle_jpeg::fixtures();
    assert_eq!(a, b, "two calls must give identical bytes");
    let mut names: Vec<&str> = a.iter().map(|(n, _)| n.as_str()).collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "fixture names must be unique");
    assert_eq!(count, 12 + 30, "12 wrapped TIFFs and 30 shapes");
    // Record files are keyed by name: no collision with a TIFF fixture, a TIFF
    // seed, or the `jpeg-seed-` records.
    for (tiff, _) in oracle_tiff::fixtures() {
        assert!(
            !names.contains(&tiff),
            "{tiff} collides with a TIFF fixture"
        );
    }
    assert!(names.iter().all(|n| n.starts_with("jpeg-")));
    assert!(names.iter().all(|n| !n.starts_with("jpeg-seed-")));
    // The normative shape names are all there.
    for shape in [
        "jpeg-jfif-then-exif",
        "jpeg-exif-then-jfif",
        "jpeg-exif-in-app2",
        "jpeg-exif-sixth-byte-nonzero",
        "jpeg-exif-fifth-byte-nonzero",
        "jpeg-exif-five-byte-payload",
        "jpeg-two-exif-same-tag",
        "jpeg-two-exif-same-tag-reversed",
        "jpeg-two-exif-disjoint-tags",
        "jpeg-seventeen-exif-last-differs",
        "jpeg-exif-after-scan",
        "jpeg-exif-after-eoi",
        "jpeg-exif-last-segment-at-eof",
        "jpeg-exif-then-com-at-eof",
        "jpeg-exif-eoi-no-sos",
        "jpeg-truncated-between-app1-and-sos",
        "jpeg-truncated-mid-scan",
        "jpeg-truncated-mid-app1",
        "jpeg-stray-bytes",
        "jpeg-bad-length",
        "jpeg-bad-tiff-in-exif",
        "jpeg-xmp",
        "jpeg-xmp-in-wrong-marker",
        "jpeg-iptc",
        "jpeg-iptc-without-0404",
        "jpeg-extended-xmp-pair",
        "jpeg-exif-good-then-fatal",
        "jpeg-exif-fatal-then-good",
        "jpeg-exif-before-and-after-scan",
        "jpeg-gps-split-across-two-blocks",
    ] {
        assert!(names.contains(&shape), "missing fixture {shape}");
    }
    // Every fixture is a JPEG by its first two bytes.
    for (n, bytes) in &a {
        assert_eq!(bytes.get(..2), Some(&[0xFF, 0xD8][..]), "{n}");
    }
}

#[test]
fn every_wrapped_tiff_fixture_is_present_and_matches_its_tiff_bytes() {
    let wrapped = oracle_jpeg::wrapped();
    assert_eq!(wrapped.len(), oracle_tiff::fixtures().len());
    for (tiff_name, tiff) in oracle_tiff::fixtures() {
        let name = format!("jpeg-{tiff_name}");
        let bytes = wrapped
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, b)| b)
            .unwrap_or_else(|| panic!("{name} is not a wrapped fixture"));
        let j = jpeg::Jpeg::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(j.is_clean(), "{name}: {:?}", j.issues());
        let exif: Vec<_> = j
            .segments()
            .iter()
            .filter(|s| s.kind() == SegmentKind::Exif)
            .collect();
        assert_eq!(exif.len(), 1, "{name}: one Exif segment");
        let payload = exif[0].payload();
        assert_eq!(payload.get(..6), Some(&b"Exif\0\0"[..]), "{name}");
        assert_eq!(
            payload.get(6..),
            Some(&tiff[..]),
            "{name}: payload after the signature"
        );
        assert!(!exif[0].after_scan());
        assert_eq!(
            oracle_jpeg::unwrapped_name(&name),
            Some(tiff_name),
            "{name}"
        );
    }
}

// ── Replay (no tool needed) ────────────────────────────────────────────────

#[test]
fn replay_jpeg_records_are_clean_on_an_unmodified_reading() {
    let mut fixtures = 0usize;
    for (name, bytes) in oracle_jpeg::fixtures() {
        let cmp = compare(&name, &ours_fixture(&bytes), &theirs_fixture(&name));
        assert!(
            cmp.is_clean(),
            "{name}: unexplained [{}] stale {:?}",
            describe(&cmp.unexplained),
            cmp.stale
        );
        for m in &cmp.explained {
            println!("KNOWN DIVERGENCE {name} {}", m.selector);
        }
        fixtures += 1;
    }
    let mut seeds = 0usize;
    for (seed, bytes) in jpeg_seed_files() {
        let name = format!("jpeg-seed-{seed}");
        let rec = read_record(&name);
        assert!(
            rec.outcome.is_none() && oracle::not_run_divergence(&name).is_none(),
            "{name}: no JPEG seed is a not-run input"
        );
        let cmp = compare(&name, &ours_seed(&bytes), &theirs_seed(&seed));
        assert!(
            cmp.is_clean(),
            "{name}: unexplained [{}] stale {:?}",
            describe(&cmp.unexplained),
            cmp.stale
        );
        for m in &cmp.explained {
            println!("KNOWN DIVERGENCE {name} {}", m.selector);
        }
        seeds += 1;
    }
    assert_eq!(fixtures, 42);
    assert_eq!(seeds, 37);
}

#[test]
fn replay_jpeg_headers_record_command_version_and_stderr() {
    let fixture_cmd = tools::exiftool_command_line(&oracle::jpeg_names());
    let seed_cmd = tools::exiftool_command_line(&oracle::names(SELECTORS));
    let mut all: Vec<(String, String, String)> = hand_built_names()
        .into_iter()
        .map(|n| (n.clone(), fixture_cmd.clone(), format!("fixture {n}")))
        .collect();
    all.extend(jpeg_seed_files().into_iter().map(|(n, _)| {
        (
            format!("jpeg-seed-{n}"),
            seed_cmd.clone(),
            format!("seed jpeg_segments/{n}"),
        )
    }));
    let mut with_warnings = 0usize;
    for (name, cmd, source) in all {
        let rec = read_record(&name);
        assert_eq!(rec.command, cmd, "{name}: recorded command");
        assert_eq!(rec.source, source, "{name}: recorded source");
        assert_eq!(rec.version, "13.55", "{name}: recorded version");
        // Warnings are recorded with the path replaced: no absolute path, no
        // temp-directory name, and ExifTool's own prefix.
        for line in &rec.stderr {
            assert!(
                line.starts_with("Warning: ") || line.starts_with("Error: "),
                "{name}: {line:?}"
            );
            assert!(
                !line.contains("/Users") && !line.contains("/tmp") && !line.contains("/var"),
                "{name}: the path leaked into {line:?}"
            );
            assert!(line.ends_with(tools::FILE_PLACEHOLDER), "{name}: {line:?}");
        }
        if !rec.stderr.is_empty() {
            with_warnings += 1;
        }
    }
    // The warnings are not all absent (the recorder really captured stderr).
    assert!(
        with_warnings >= 10,
        "only {with_warnings} records have warnings"
    );
}

/// FNV-1a 64 of every TIFF record as committed before SPEC-011. The TIFF records
/// must be byte-identical (the CI job `oracle-records` diffs them), so a stderr
/// capture added for JPEG cannot leak into them.
const TIFF_RECORD_HASHES: [(&str, u64); 41] = [
    ("ascii-count-past-eof", 0x79e5bf42df00b8ac),
    ("ascii-edges", 0xf65f8c527883ff8e),
    ("ascii-shapes", 0x848c6dbffc48ee87),
    ("chain-be", 0xacf0cb4535868af2),
    ("chain", 0x69389d8e521587ae),
    ("corrupt-subifd", 0xc2bdaef8cfc2c2a4),
    ("count-overflow-be", 0xc0f070639491719f),
    ("count-overflow", 0xa30617cde6c10ae5),
    ("deep-nesting", 0x93e85e0840d5ba7e),
    ("depth-limit-exact", 0x698437094860321a),
    ("entry-count-past-eof", 0x51cf96191aa10fef),
    ("exif-basic-be", 0x4f5dfcbdc9ffd5b2),
    ("exif-basic-le", 0xf15687e6310fb37c),
    ("exif-gps-full-le", 0x5d005bbc9c3bd39d),
    ("exif-gps-interop", 0x9543dd3371fd046c),
    ("exif-gps-ne-le", 0x200213b7db4f4b42),
    ("exif-gps-nw-le", 0xfbc77992ab83f9b3),
    ("exif-gps-se-be", 0xbe7cac24c2a4ec26),
    ("exif-gps-sw-be", 0xd2d4550bb2975ee3),
    ("exif-points-at-ifd0", 0x40eeeeddae8f2b7d),
    ("gps-corrupt-exif-ok", 0xd0a4cdc254b677dd),
    ("gps-past-eof-exif-ok", 0x9a122437ab1596ff),
    ("ifd0-offset-zero", 0x719852073e2e32af),
    ("ifd1-corrupt", 0xbb91d0ce1818840e),
    ("leaf-next-word", 0x78b80209148a3c2e),
    ("malformed-subifds-tag", 0xad15d98b40e631b2),
    ("next-past-eof", 0x414e768fd731d370),
    ("payload-past-eof-be", 0x488e10efe866a5e0),
    ("payload-past-eof", 0x97e2d3649c26c27e),
    ("pointer-bad-count", 0x2edc1519d5238d1a),
    ("self-cycle", 0x08bc8d1b106ce626),
    ("signed-and-float", 0xfea308cb93380378),
    ("signed-float-be", 0x4eaa0e5baf1cc8c8),
    ("signed-float-le", 0x13f0c74d9b84a30a),
    ("subifd-be", 0x7907d79c74cf0b68),
    ("subifd", 0xaef48c028e0b4d14),
    ("subifds-count-over-cap", 0xe770cce2034c9dd3),
    ("truncated-header", 0x02bac1febad20af1),
    ("two-hop-cycle", 0x85d904043923ed71),
    ("valid-be", 0x2a8146e15f893d30),
    ("valid-le", 0xe015beb701611dbe),
];

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[test]
fn tiff_records_are_byte_identical_to_the_committed_set() {
    for (name, hash) in TIFF_RECORD_HASHES {
        let bytes = std::fs::read(record_path(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(fnv1a_64(&bytes), hash, "{name}: the TIFF record changed");
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("# stderr:"),
            "{name}: a TIFF record has stderr lines"
        );
    }
    // And there is nothing else that is not a JPEG record, apart from the
    // SPEC-012 record `audit-tag-names.txt` (the names ExifTool gives the DEC-008
    // tag ids; exempted by its exact name, SPEC-013 F1),
    // which is neither a TIFF nor a JPEG record.
    let dir = root().join("tests/oracle-fixtures/exiftool");
    let others = std::fs::read_dir(&dir)
        .expect("read records")
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| !n.starts_with("jpeg-") && n != "audit-tag-names.txt")
        .count();
    // 41 older TIFF records, plus the SPEC-017 ones: one for each of the three
    // new `tiff_ifd` seeds and `hostile-subifds-overlap` (whose `# stderr:`
    // line is the ExifTool warning; it is not in `TIFF_RECORD_HASHES`).
    assert_eq!(others, 41 + 3 + 1);
    let jpeg_records = std::fs::read_dir(&dir)
        .expect("read records")
        .filter(|e| {
            e.as_ref()
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .starts_with("jpeg-")
        })
        .count();
    assert_eq!(jpeg_records, 42 + 37);
}

/// Replay-and-perturb on JPEG records: for every field our reading has, change
/// exactly that value (or, for a region, remove it) and require exactly one
/// mismatch naming it. Fixtures with a known divergence are left out (their
/// reading is already, by design, not equal to the record).
#[test]
fn oracle_names_the_one_field_perturbed_in_a_jpeg_record() {
    let mut perturbed_fields = 0usize;
    let mut norms_seen: Vec<std::mem::Discriminant<oracle::Norm>> = Vec::new();
    for (name, bytes) in oracle_jpeg::fixtures() {
        if oracle::KNOWN_DIVERGENCES.iter().any(|d| d.fixture == name) {
            continue;
        }
        let theirs = theirs_fixture(&name);
        let ours = ours_fixture(&bytes);
        assert!(compare(&name, &ours, &theirs).is_clean(), "{name}");
        for (i, v) in ours.iter().enumerate() {
            let Some(v) = v else { continue };
            if v == oracle::SKIPPED {
                continue;
            }
            let mut p = ours.clone();
            p[i] = if i < SELECTORS.len() {
                norms_seen.push(std::mem::discriminant(&SELECTORS[i].norm));
                Some(oracle::perturb(SELECTORS[i].norm, v))
            } else {
                None // a region we located is removed
            };
            let cmp = compare(&name, &p, &theirs);
            assert_eq!(
                cmp.unexplained.len(),
                1,
                "{name} field {i}: {}",
                describe(&cmp.unexplained)
            );
            assert_eq!(
                cmp.unexplained[0].selector,
                oracle::jpeg_names()[i],
                "{name}"
            );
            perturbed_fields += 1;
        }
    }
    assert!(
        perturbed_fields > 100,
        "only {perturbed_fields} fields perturbed"
    );
    // Every Norm with a selector was perturbed against a real JPEG record.
    for needed in [
        oracle::Norm::Ascii,
        oracle::Norm::Uint,
        oracle::Norm::Sint,
        oracle::Norm::Float,
        oracle::Norm::Rational,
        oracle::Norm::SRational,
        oracle::Norm::GpsCoord { ref_tag: 1 },
    ] {
        assert!(
            norms_seen.contains(&std::mem::discriminant(&needed)),
            "no JPEG record exercised {needed:?}"
        );
    }
    // A region we did NOT locate but ExifTool did, and the converse.
    let theirs = theirs_fixture("jpeg-xmp");
    let mut ours = ours_fixture(&fixture("jpeg-xmp"));
    let xmp = index_of("XMP-xmp:CreatorTool");
    ours[xmp] = None;
    let cmp = compare("jpeg-xmp", &ours, &theirs);
    assert_eq!(named(&cmp.unexplained), vec!["XMP-xmp:CreatorTool"]);
    let theirs = theirs_fixture("jpeg-xmp-in-wrong-marker");
    let mut ours = ours_fixture(&fixture("jpeg-xmp-in-wrong-marker"));
    ours[xmp] = Some(oracle::PRESENT.to_string());
    let cmp = compare("jpeg-xmp-in-wrong-marker", &ours, &theirs);
    assert_eq!(named(&cmp.unexplained), vec!["XMP-xmp:CreatorTool"]);
}

/// Container independence, offline: every wrapped fixture's committed JPEG
/// record carries exactly the Exif fields (and no region field) of the committed
/// record of its bare TIFF, and our reading agrees bare and wrapped, with the
/// same set of tags.
#[test]
fn container_independence_every_wrapped_fixture_reads_the_same_bare_and_in_jpeg() {
    let tiff_names = oracle::names(SELECTORS);
    for (tiff_name, tiff) in oracle_tiff::fixtures() {
        let bare_theirs = tools::parse_fields(&tiff_names, &read_record(tiff_name).stdout)
            .unwrap_or_else(|e| panic!("{tiff_name}: {e}"));
        let name = format!("jpeg-{tiff_name}");
        let jpeg_theirs = theirs_fixture(&name);
        assert_eq!(
            &jpeg_theirs[..SELECTORS.len()],
            &bare_theirs[..],
            "{name}: ExifTool's tags differ bare and in a JPEG"
        );
        assert!(
            jpeg_theirs[SELECTORS.len()..].iter().all(Option::is_none),
            "{name}: no region fields"
        );

        let bare = Container::parse(&tiff).unwrap_or_else(|e| panic!("{tiff_name}: {e:?}"));
        let bare_ours = oracle::read_ours(&bare, SELECTORS);
        let bytes = fixture(&name);
        let meta = jpeg::read(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let wrapped_ours = oracle::read_ours_jpeg(&meta, SELECTORS);
        assert_eq!(
            wrapped_ours, bare_ours,
            "{name}: our reading, bare vs in a JPEG"
        );
        let has = |v: &[Option<String>]| v.iter().map(Option::is_some).collect::<Vec<_>>();
        assert_eq!(has(&wrapped_ours), has(&bare_ours), "{name}: set of tags");
        assert_eq!(
            has(&wrapped_ours),
            has(&bare_theirs),
            "{name}: set of tags vs ExifTool"
        );

        // Both comparisons are clean on their own committed records.
        assert!(oracle::diff(&bare_ours, &bare_theirs, SELECTORS).is_empty());
        assert!(
            compare(&name, &ours_fixture(&bytes), &jpeg_theirs).is_clean(),
            "{name}"
        );
    }
}

#[test]
fn multi_exif_is_compared_against_the_union_with_the_later_block_winning() {
    let make = index_of("IFD0:Make");
    let model = index_of("IFD0:Model");
    for (name, want_make, want_model) in [
        // A (Make AlphaCam, Model OnlyInA) then B (Make BravoCam): union, B's Make.
        ("jpeg-two-exif-same-tag", "BravoCam", Some("OnlyInA")),
        // B then A: A is later, so A's Make wins and A's Model is the only one.
        (
            "jpeg-two-exif-same-tag-reversed",
            "AlphaCam",
            Some("OnlyInA"),
        ),
        // Disjoint tags: both are kept.
        (
            "jpeg-two-exif-disjoint-tags",
            "AlphaCam",
            Some("BravoModel"),
        ),
    ] {
        let theirs = theirs_fixture(name);
        assert_eq!(
            theirs[make].as_deref(),
            Some(want_make),
            "{name}: ExifTool Make"
        );
        assert_eq!(
            theirs[model].as_deref(),
            want_model,
            "{name}: ExifTool Model"
        );
        let bytes = fixture(name);
        let meta = jpeg::read(&bytes).expect("a JPEG");
        assert_eq!(meta.exif_blocks().len(), 2, "{name}");
        let ours = ours_fixture(&bytes);
        assert_eq!(
            ours[make].as_deref(),
            Some(want_make),
            "{name}: our model Make"
        );
        assert_eq!(
            ours[model].as_deref(),
            want_model,
            "{name}: our model Model"
        );
        assert!(compare(name, &ours, &theirs).is_clean(), "{name}");

        // The library's own policy is different and unchanged: the FIRST block
        // is primary and the second costs an ExtraExif issue (DEC-006). That is
        // exactly why the comparison uses the harness model.
        let primary = meta.exif().expect("a primary Exif");
        let primary_only = oracle::read_ours(primary, SELECTORS);
        assert_ne!(
            &primary_only[..],
            &theirs[..SELECTORS.len()],
            "{name}: if the primary alone agreed with ExifTool the model would be unneeded"
        );
        assert!(meta
            .jpeg_issues()
            .iter()
            .any(|i| matches!(i.error(), Error::ExtraExif { .. })));
    }
    // Without the model the fixture would be unexplained: comparing the
    // first-block-only reading with ExifTool names IFD0:Make.
    let bytes = fixture("jpeg-two-exif-same-tag");
    let meta = jpeg::read(&bytes).expect("a JPEG");
    let mut first_only = ours_fixture(&bytes);
    first_only[make] = oracle::read_one(meta.exif().expect("primary"), &SELECTORS[2]);
    let cmp = compare(
        "jpeg-two-exif-same-tag",
        &first_only,
        &theirs_fixture("jpeg-two-exif-same-tag"),
    );
    assert_eq!(named(&cmp.unexplained), vec!["IFD0:Make"]);
}

#[test]
fn exif_signature_is_five_bytes_per_exiftool() {
    let make = index_of("IFD0:Make");
    // (fixture, ExifTool reads Make, our library classifies an Exif block)
    for (name, read) in [
        ("jpeg-exif-sixth-byte-nonzero", true),
        ("jpeg-exif-fifth-byte-nonzero", false),
        ("jpeg-exif-five-byte-payload", false),
        ("jpeg-exif-in-app2", false),
    ] {
        let theirs = theirs_fixture(name);
        let bytes = fixture(name);
        let ours = ours_fixture(&bytes);
        assert_eq!(theirs[make].is_some(), read, "{name}: ExifTool");
        assert_eq!(ours[make].is_some(), read, "{name}: ours");
        if read {
            assert_eq!(theirs[make].as_deref(), Some("DiamondCam"));
        }
        let cmp = compare(name, &ours, &theirs);
        assert!(
            cmp.is_clean() && cmp.explained.is_empty(),
            "{name}: {cmp:?}"
        );
        let meta = jpeg::read(&bytes).expect("a JPEG");
        assert_eq!(!meta.exif_blocks().is_empty(), read, "{name}: blocks");
    }
}

#[test]
fn exif_after_scan_is_a_recorded_divergence_and_still_present() {
    for (name, theirs, bytes) in [
        (
            "jpeg-exif-after-scan",
            theirs_fixture("jpeg-exif-after-scan"),
            fixture("jpeg-exif-after-scan"),
        ),
        (
            "jpeg-seed-exif-after-scan",
            theirs_seed("exif-after-scan"),
            std::fs::read(root().join("fuzz/seeds/jpeg_segments/exif-after-scan")).expect("seed"),
        ),
    ] {
        assert!(
            theirs.iter().all(Option::is_none),
            "{name}: ExifTool reads nothing"
        );
        let meta = jpeg::read(&bytes).expect("a JPEG");
        let exif: Vec<_> = meta
            .jpeg()
            .segments()
            .iter()
            .filter(|s| s.kind() == SegmentKind::Exif)
            .collect();
        assert_eq!(exif.len(), 1, "{name}");
        assert!(
            exif[0].after_scan(),
            "{name}: the Exif is after the first SOS"
        );
        let ours = if name.starts_with("jpeg-seed-") {
            ours_seed(&bytes)
        } else {
            ours_fixture(&bytes)
        };
        assert!(ours[index_of("IFD0:Make")].is_some(), "{name}: we read it");
        let cmp = compare(name, &ours, &theirs);
        assert!(cmp.is_clean(), "{name}: {cmp:?}");
        assert!(
            !cmp.explained.is_empty(),
            "{name}: the divergence must be present"
        );
        // The divergence is the wildcard entry, one direction only.
        assert!(cmp
            .explained
            .iter()
            .all(|m| m.ours.is_some() && m.theirs.is_none()));
        assert!(oracle::KNOWN_DIVERGENCES
            .iter()
            .any(|d| d.fixture == name && d.selector == oracle::EXIF_ANY));
    }
    // Control: the same TIFF BEFORE the scan is read by both sides.
    let before = theirs_fixture("jpeg-exif-basic-le");
    assert!(before[index_of("IFD0:Make")].is_some());
}

#[test]
fn last_segment_at_eof_is_a_recorded_divergence_and_still_present() {
    let name = "jpeg-exif-last-segment-at-eof";
    let rec = read_record(name);
    assert!(
        rec.stderr.iter().any(|l| l.contains("JPEG format error")),
        "{name}: ExifTool warns: {:?}",
        rec.stderr
    );
    let theirs = theirs_fixture(name);
    assert!(theirs.iter().all(Option::is_none), "ExifTool reads nothing");
    let bytes = fixture(name);
    let meta = jpeg::read(&bytes).expect("a JPEG");
    let kinds: Vec<&Error> = meta.jpeg_issues().iter().map(|i| i.error()).collect();
    assert_eq!(kinds, [&Error::MissingSos, &Error::MissingEoi]);
    assert_eq!(meta.exif_blocks().len(), 1);
    let ours = ours_fixture(&bytes);
    assert!(ours[index_of("IFD0:Make")].is_some());
    let cmp = compare(name, &ours, &theirs);
    assert!(cmp.is_clean() && !cmp.explained.is_empty(), "{cmp:?}");

    // Controls: end the file after a COM segment, or after an EOI, and both
    // sides read the Exif again.
    for control in ["jpeg-exif-then-com-at-eof", "jpeg-exif-eoi-no-sos"] {
        let theirs = theirs_fixture(control);
        let ours = ours_fixture(&fixture(control));
        assert!(
            theirs[index_of("IFD0:Make")].is_some(),
            "{control}: ExifTool"
        );
        let cmp = compare(control, &ours, &theirs);
        assert!(
            cmp.is_clean() && cmp.explained.is_empty(),
            "{control}: {cmp:?}"
        );
    }
}

#[test]
fn seventeenth_exif_block_divergence_is_recorded_and_still_present() {
    let name = "jpeg-seventeen-exif-last-differs";
    let make = index_of("IFD0:Make");
    let theirs = theirs_fixture(name);
    assert_eq!(theirs[make].as_deref(), Some("LastOnly17"));
    let bytes = fixture(name);
    let meta = jpeg::read(&bytes).expect("a JPEG");
    assert_eq!(meta.exif_blocks().len(), jpeg::MAX_EXIF_BLOCKS);
    let extra = meta
        .jpeg_issues()
        .iter()
        .filter(|i| matches!(i.error(), Error::ExtraExif { .. }))
        .count();
    assert_eq!(extra, 16, "sixteen ExtraExif issues for seventeen segments");
    let ours = ours_fixture(&bytes);
    assert_eq!(ours[make].as_deref(), Some("SameFirst16"));
    let cmp = compare(name, &ours, &theirs);
    assert!(cmp.is_clean(), "{cmp:?}");
    assert_eq!(named(&cmp.explained), vec!["IFD0:Make"]);
    assert!(cmp.explained[0].ours.is_some() && cmp.explained[0].theirs.is_some());
}

#[test]
fn region_presence_xmp_and_iptc_agree_with_exiftool() {
    let xmp = index_of("XMP-xmp:CreatorTool");
    let iptc = index_of("IPTC:ObjectName");
    for (name, want_xmp, want_iptc) in [
        ("jpeg-xmp", true, false),
        ("jpeg-extended-xmp-pair", true, false),
        ("jpeg-iptc", false, true),
    ] {
        let theirs = theirs_fixture(name);
        let ours = ours_fixture(&fixture(name));
        assert_eq!(theirs[xmp].is_some(), want_xmp, "{name}: ExifTool XMP");
        assert_eq!(theirs[iptc].is_some(), want_iptc, "{name}: ExifTool IPTC");
        assert_eq!(ours[xmp].is_some(), want_xmp, "{name}: our XMP region");
        assert_eq!(ours[iptc].is_some(), want_iptc, "{name}: our IPTC region");
        assert!(compare(name, &ours, &theirs).is_clean(), "{name}");
    }
    // Presence only: the values are ExifTool's, not compared.
    assert_eq!(
        theirs_fixture("jpeg-xmp")[xmp].as_deref(),
        Some("DiamondTool")
    );
    assert_eq!(
        theirs_fixture("jpeg-iptc")[iptc].as_deref(),
        Some("DiamondTitle")
    );
}

#[test]
fn iptc_without_0404_and_xmp_in_wrong_marker_are_absent_on_both_sides() {
    for name in ["jpeg-iptc-without-0404", "jpeg-xmp-in-wrong-marker"] {
        let theirs = theirs_fixture(name);
        assert!(
            theirs.iter().all(Option::is_none),
            "{name}: ExifTool reports nothing"
        );
        let bytes = fixture(name);
        let meta = jpeg::read(&bytes).expect("a JPEG");
        assert!(
            meta.regions().is_empty(),
            "{name}: no region: {:?}",
            meta.regions()
        );
        assert!(meta.exif_blocks().is_empty());
        let ours = ours_fixture(&bytes);
        assert!(ours.iter().all(Option::is_none), "{name}");
    }
    // The segments really are there, under the wrong id or marker.
    let bytes = fixture("jpeg-iptc-without-0404");
    let j = jpeg::Jpeg::parse(&bytes).expect("a JPEG");
    assert!(j.segments().iter().any(|s| s.kind() == SegmentKind::Iptc));
    let bytes = fixture("jpeg-xmp-in-wrong-marker");
    let j = jpeg::Jpeg::parse(&bytes).expect("a JPEG");
    assert!(j.segments().iter().any(|s| s.kind() == SegmentKind::App(2)));
}

/// One row of the warning characterisation table: per hand-built fixture, the
/// JPEG-layer issue kinds of ours, the shape of our Exif blocks, and the
/// substring ExifTool 13.55 printed on stderr (`None`: it printed nothing).
///
/// A characterisation, asserted both ways: a changed library issue or a changed
/// ExifTool warning fails. It does NOT claim the two are equivalent.
///
/// Info (verify, HANDOFF-022): with two Exif segments `exiftool -Warning` says
/// `[minor] File contains multi-segment EXIF`, which the `-T` stderr never shows;
/// and "union of tags, the later block wins" is a model FITTED to what was
/// measured (2, 17 and 20 blocks, fatal or damaged later blocks), not ExifTool's
/// documented rule.
///
/// Measured finding that shapes this table: ExifTool's command line prints its
/// warnings to stderr only when none of the requested tags was found. Every
/// warning in the committed records is on an input whose fields are all `-`; a
/// file with the same damage AND a readable requested tag (for example
/// `jpeg-stray-bytes`, which has an Exif) prints no warning.
type WarningRow = (
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);

const WARNING_TABLE: &[WarningRow] = &[
    // (fixture, our JPEG-layer issue kinds, our Exif blocks, ExifTool stderr)
    ("jpeg-jfif-then-exif", "", "ok", None),
    ("jpeg-exif-then-jfif", "", "ok", None),
    ("jpeg-exif-in-app2", "", "", None),
    ("jpeg-exif-sixth-byte-nonzero", "", "ok", None),
    ("jpeg-exif-fifth-byte-nonzero", "", "", None),
    ("jpeg-exif-five-byte-payload", "", "", None),
    ("jpeg-two-exif-same-tag", "ExtraExif", "ok*2", None),
    ("jpeg-two-exif-same-tag-reversed", "ExtraExif", "ok*2", None),
    ("jpeg-two-exif-disjoint-tags", "ExtraExif", "ok*2", None),
    (
        "jpeg-seventeen-exif-last-differs",
        "ExtraExif*16",
        "ok*16",
        None,
    ),
    // Two Exif segments are `[minor] File contains multi-segment EXIF` to
    // `exiftool -Warning`; the `-T` command line prints nothing here.
    ("jpeg-exif-good-then-fatal", "ExtraExif", "ok,fatal", None),
    ("jpeg-exif-fatal-then-good", "ExtraExif", "fatal,ok", None),
    ("jpeg-exif-before-and-after-scan", "ExtraExif", "ok*2", None),
    (
        "jpeg-gps-split-across-two-blocks",
        "ExtraExif",
        "ok*2",
        None,
    ),
    ("jpeg-exif-after-scan", "", "ok", None),
    (
        "jpeg-exif-after-eoi",
        "MissingSos",
        "",
        Some("Missing JPEG SOS"),
    ),
    (
        "jpeg-exif-last-segment-at-eof",
        "MissingSos,MissingEoi",
        "ok",
        Some("JPEG format error"),
    ),
    (
        "jpeg-exif-then-com-at-eof",
        "MissingSos,MissingEoi",
        "ok",
        None,
    ),
    ("jpeg-exif-eoi-no-sos", "MissingSos", "ok", None),
    (
        "jpeg-truncated-between-app1-and-sos",
        "MissingSos,MissingEoi",
        "ok",
        None,
    ),
    ("jpeg-truncated-mid-scan", "MissingEoi", "ok", None),
    (
        "jpeg-truncated-mid-app1",
        "Truncated,MissingSos,MissingEoi",
        "",
        Some("JPEG format error"),
    ),
    ("jpeg-stray-bytes", "StrayBytes", "ok", None),
    (
        "jpeg-bad-length",
        "BadSegmentLength",
        "",
        Some("JPEG format error"),
    ),
    (
        "jpeg-bad-tiff-in-exif",
        "",
        "fatal",
        Some("Malformed APP1 EXIF segment"),
    ),
    ("jpeg-xmp", "", "", None),
    ("jpeg-xmp-in-wrong-marker", "", "", None),
    ("jpeg-iptc", "", "", None),
    ("jpeg-iptc-without-0404", "", "", None),
    ("jpeg-extended-xmp-pair", "", "", None),
    // The wrapped TIFF fixtures: clean at the JPEG layer on both sides; two of
    // them carry a TIFF-layer issue of ours that ExifTool says nothing about.
    ("jpeg-exif-basic-le", "", "ok", None),
    ("jpeg-exif-basic-be", "", "ok", None),
    ("jpeg-exif-gps-ne-le", "", "ok", None),
    ("jpeg-exif-gps-nw-le", "", "ok", None),
    ("jpeg-exif-gps-se-be", "", "ok", None),
    ("jpeg-exif-gps-sw-be", "", "ok", None),
    ("jpeg-ascii-edges", "", "ok", None),
    ("jpeg-gps-corrupt-exif-ok", "", "issues=1", None),
    ("jpeg-ifd1-corrupt", "", "issues=1", None),
    ("jpeg-exif-gps-full-le", "", "ok", None),
    ("jpeg-signed-float-le", "", "ok", None),
    ("jpeg-signed-float-be", "", "ok", None),
];

/// `a,b,b,b` -> `a,b*3`.
fn run_length(items: &[String]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < items.len() {
        let mut j = i;
        while j < items.len() && items[j] == items[i] {
            j += 1;
        }
        if j - i > 1 {
            out.push(format!("{}*{}", items[i], j - i));
        } else {
            out.push(items[i].clone());
        }
        i = j;
    }
    out.join(",")
}

fn issue_kind(e: &Error) -> String {
    let d = format!("{e:?}");
    d.split([' ', '{', '('])
        .next()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn warning_table_pins_our_issue_kinds_beside_exiftools_warnings() {
    // The table covers every hand-built fixture exactly once.
    let mut table_names: Vec<&str> = WARNING_TABLE.iter().map(|r| r.0).collect();
    table_names.sort_unstable();
    let mut want = hand_built_names();
    want.sort_unstable();
    assert_eq!(table_names, want, "the table must list every fixture once");

    for (name, jpeg_kinds, blocks, warning) in WARNING_TABLE {
        let bytes = fixture(name);
        let meta = jpeg::read(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let ours_jpeg = run_length(
            &meta
                .jpeg_issues()
                .iter()
                .map(|i| issue_kind(i.error()))
                .collect::<Vec<_>>(),
        );
        assert_eq!(&ours_jpeg, jpeg_kinds, "{name}: our JPEG-layer issues");
        let ours_blocks = run_length(
            &meta
                .exif_blocks()
                .iter()
                .map(|b| match b.container() {
                    Ok(c) if c.issues().is_empty() => "ok".to_string(),
                    Ok(c) => format!("issues={}", c.issues().len()),
                    Err(_) => "fatal".to_string(),
                })
                .collect::<Vec<_>>(),
        );
        assert_eq!(&ours_blocks, blocks, "{name}: our Exif blocks");

        let rec = read_record(name);
        match warning {
            Some(sub) => {
                assert_eq!(rec.stderr.len(), 1, "{name}: {:?}", rec.stderr);
                assert!(
                    rec.stderr[0].contains(sub),
                    "{name}: ExifTool printed {:?}, expected it to contain {sub:?}",
                    rec.stderr
                );
            }
            None => assert!(rec.stderr.is_empty(), "{name}: {:?}", rec.stderr),
        }
    }
}

/// Every divergence is tracked: its `question_id` is a question in
/// `guidance/questions.yaml` (process rule 2 of SPEC-011).
#[test]
fn every_known_divergence_has_a_question_entry() {
    let questions = std::fs::read_to_string(root().join("guidance/questions.yaml"))
        .expect("read guidance/questions.yaml");
    for d in oracle::KNOWN_DIVERGENCES {
        assert!(
            questions.contains(&format!("- id: {}\n", d.question_id)),
            "{} {}: no question `{}` in guidance/questions.yaml",
            d.fixture,
            d.selector,
            d.question_id
        );
        assert!(!d.command.is_empty() && !d.measured_stdout.is_empty() && !d.right_side.is_empty());
        assert_eq!(d.exiftool_version, "13.55");
    }
    assert!(oracle::KNOWN_DIVERGENCES.len() >= 5);
}

#[test]
fn a_divergence_that_stops_being_one_fails() {
    // Every divergence flips to a failure the moment the two sides agree.
    for name in [
        "jpeg-exif-after-scan",
        "jpeg-exif-last-segment-at-eof",
        "jpeg-seventeen-exif-last-differs",
    ] {
        let ours = ours_fixture(&fixture(name));
        let theirs = theirs_fixture(name);
        let honest = compare(name, &ours, &theirs);
        assert!(
            honest.is_clean() && !honest.explained.is_empty(),
            "{name}: {honest:?}"
        );
        // Flip: ExifTool is made to agree with us (as if it were fixed).
        let flipped = compare(name, &ours, &ours);
        assert!(!flipped.is_clean(), "{name}: the list must not rot");
        assert!(!flipped.stale.is_empty(), "{name}");
        assert!(flipped.explained.is_empty() && flipped.unexplained.is_empty());
        // And the same divergence on a fixture with NO entry is just a failure.
        let stranger = compare("jpeg-exif-basic-le", &ours, &theirs);
        assert!(
            !stranger.unexplained.is_empty(),
            "{name}: not covered elsewhere"
        );
    }
    // The wildcard covers one direction only: ExifTool reading a value we do
    // not is NOT explained by "ExifTool ignores Exif after the scan".
    let name = "jpeg-exif-after-scan";
    let make = index_of("IFD0:Make");
    let ours = ours_fixture(&fixture(name));
    let mut theirs = theirs_fixture(name);
    theirs[make] = Some("Surprise".to_string());
    let mut blind = ours.clone();
    blind[make] = None;
    let cmp = compare(name, &blind, &theirs);
    assert!(cmp.unexplained.iter().any(|m| m.selector == "IFD0:Make"));
    // A value that differs (both Some) is not covered by the wildcard either.
    let cmp = compare(name, &ours, &{
        let mut t = vec![None; ours.len()];
        t[make] = Some("Other".to_string());
        t
    });
    assert!(cmp.unexplained.iter().any(|m| m.selector == "IFD0:Make"));
    // A mismatch outside the Exif selectors (a region) is never covered.
    let xmp = index_of("XMP-xmp:CreatorTool");
    let mut ours_x = ours.clone();
    ours_x[xmp] = Some(oracle::PRESENT.to_string());
    let cmp = compare(name, &ours_x, &theirs_fixture(name));
    assert!(cmp
        .unexplained
        .iter()
        .any(|m| m.selector == "XMP-xmp:CreatorTool"));
}

// ── Live (skip loudly without ExifTool) ────────────────────────────────────

/// Run ExifTool on `bytes` under the name `name` (a `.jpg` temp file) with the
/// given selectors; returns (fields, stdout, warnings).
fn live_run(
    name: &str,
    bytes: &[u8],
    selectors: &[&str],
) -> (Vec<Option<String>>, String, Vec<String>) {
    let path = tools::temp_file_with_ext(name, "jpg", bytes).expect("temp file");
    let run = tools::run_exiftool_full(&path, selectors);
    let _ = std::fs::remove_file(&path);
    let (stdout, stderr) = run.unwrap_or_else(|e| panic!("{name}: {e}"));
    let fields = tools::parse_fields(selectors, &stdout).unwrap_or_else(|e| panic!("{name}: {e}"));
    (fields, stdout, tools::warning_lines(&stderr, &path))
}

#[test]
fn live_container_independence_against_exiftool() {
    let test = "live_container_independence_against_exiftool";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = oracle::names(SELECTORS);
    let mut compared = 0usize;
    for (tiff_name, tiff) in oracle_tiff::fixtures() {
        let bare_path = tools::temp_file_for(tiff_name, &tiff).expect("temp file");
        let bare = tools::run_exiftool(&bare_path, &names);
        let _ = std::fs::remove_file(&bare_path);
        let bare = bare.unwrap_or_else(|e| panic!("{tiff_name}: {e}"));

        let name = format!("jpeg-{tiff_name}");
        let (wrapped, _, _) = live_run(&name, &fixture(&name), &names);
        assert_eq!(wrapped, bare, "{name}: live ExifTool, bare vs in a JPEG");
        compared += wrapped.iter().filter(|v| v.is_some()).count();
    }
    println!("{test}: {compared} ExifTool values identical bare and in a JPEG");
    assert!(compared > 60, "only {compared} values");
}

#[test]
fn live_jpeg_parity_on_hand_built_fixtures() {
    let test = "live_jpeg_parity_on_hand_built_fixtures";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = oracle::jpeg_names();
    let (mut compared, mut explained) = (0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    for (name, bytes) in oracle_jpeg::fixtures() {
        let (theirs, stdout, warnings) = live_run(&name, &bytes, &names);
        let cmp = compare(&name, &ours_fixture(&bytes), &theirs);
        for m in &cmp.unexplained {
            failures.push(format!("{name}: {m}"));
        }
        for s in &cmp.stale {
            failures.push(s.clone());
        }
        explained += cmp.explained.len();
        compared += theirs.iter().filter(|t| t.is_some()).count();
        // The committed record is what the live tool prints today.
        let rec = read_record(&name);
        if rec.stdout != stdout || rec.stderr != warnings {
            failures.push(format!(
                "{name}: the committed record is stale (run `just oracle-record`): stdout {:?} vs \
                 {:?}, stderr {:?} vs {:?}",
                rec.stdout, stdout, rec.stderr, warnings
            ));
        }
    }
    println!(
        "{test}: {} fixtures, {compared} ExifTool values compared, {explained} explained by a \
         known divergence",
        oracle_jpeg::fixtures().len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn live_jpeg_parity_on_the_jpeg_fuzz_seeds() {
    let test = "live_jpeg_parity_on_the_jpeg_fuzz_seeds";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = oracle::names(SELECTORS);
    let (mut compared, mut explained, mut run) = (0usize, 0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    let seeds = jpeg_seed_files();
    for (seed, bytes) in &seeds {
        let name = format!("jpeg-seed-{seed}");
        if oracle::not_run_divergence(&name).is_some() {
            println!("NOT RUN {name}: a known not-return input");
            continue;
        }
        // ExifTool is run on the committed file itself, alone, under the
        // wall-clock limit; a timeout is a failure, not a skip.
        let path = root().join("fuzz/seeds/jpeg_segments").join(seed);
        let (stdout, stderr) =
            tools::run_exiftool_full(&path, &names).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut theirs =
            tools::parse_fields(&names, &stdout).unwrap_or_else(|e| panic!("{name}: {e}"));
        compared += theirs.iter().filter(|t| t.is_some()).count();
        theirs.extend(oracle::REGION_SELECTORS.iter().map(|_| None));
        let cmp = compare(&name, &ours_seed(bytes), &theirs);
        for m in &cmp.unexplained {
            failures.push(format!("{name}: {m}"));
        }
        for s in &cmp.stale {
            failures.push(s.clone());
        }
        explained += cmp.explained.len();
        let rec = read_record(&name);
        let warnings = tools::warning_lines(&stderr, &path);
        if rec.stdout != stdout || rec.stderr != warnings {
            failures.push(format!("{name}: the committed record is stale"));
        }
        run += 1;
    }
    println!(
        "{test}: {} seeds, {run} run live, {compared} ExifTool values compared, {explained} \
         explained by a known divergence",
        seeds.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

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
fn live_jpeg_skip_is_loud_and_required_mode_fails() {
    for test in [
        "live_container_independence_against_exiftool",
        "live_jpeg_parity_on_hand_built_fixtures",
        "live_jpeg_parity_on_the_jpeg_fuzz_seeds",
    ] {
        let out = run_self_without_exiftool(test, false);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{test}: should pass when skipped\n{stdout}"
        );
        assert!(
            stdout.contains(&format!("SKIP {test}")) && stdout.contains("exiftool not on PATH"),
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
                "{test}: exiftool is not on PATH and {REQUIRE_VAR}=1"
            )),
            "{test}: failed, but not with the required-mode message\n{both}"
        );
    }
}

// Silence an unused-import warning on the one type only some builds use.
#[allow(dead_code)]
fn _selector_type_is_used(_: &Selector) {}

// ── Fix round (verify punch list, HANDOFF-022) ─────────────────────────────

/// M1: a wildcard (`exif:*`) divergence must still pin OUR values. For each
/// wildcard fixture, our Exif reading has to equal ExifTool's reading of the same
/// TIFF in a place both tools read (`oracle::EXIF_PINS`); a wrong value, or a
/// dropped field, or comparing without the pin, is a failure.
#[test]
fn wildcard_divergences_pin_our_values_against_the_same_tiff_read_normally() {
    let make = index_of("IFD0:Make");
    let model = index_of("IFD0:Model");
    let cases = [
        ("jpeg-exif-after-scan", false),
        ("jpeg-exif-last-segment-at-eof", false),
        ("jpeg-seed-exif-after-scan", true),
    ];
    for (name, seed) in cases {
        let (ours, theirs) = if seed {
            let bytes = std::fs::read(root().join("fuzz/seeds/jpeg_segments/exif-after-scan"))
                .expect("seed");
            (ours_seed(&bytes), theirs_seed("exif-after-scan"))
        } else {
            (ours_fixture(&fixture(name)), theirs_fixture(name))
        };
        let pin = pin_for(name).expect("a pinned fixture");
        assert!(
            pin.iter().filter(|p| p.is_some()).count() >= 2,
            "{name}: pin"
        );

        // Honest: clean, with the divergence present.
        let honest = compare(name, &ours, &theirs);
        assert!(
            honest.is_clean() && !honest.explained.is_empty(),
            "{name}: {honest:?}"
        );

        // A wrong value of ours (and nothing else lost) is NOT explained.
        let mut wrong = ours.clone();
        wrong[make] = Some("WRONG-VALUE".to_string());
        let cmp = compare(name, &wrong, &theirs);
        assert!(!cmp.is_clean(), "{name}: a wrong Make passed: {cmp:?}");
        assert!(cmp.unexplained.iter().any(|m| m.selector == "IFD0:Make"));

        // Every field dropped but Make is NOT explained.
        let mut dropped = vec![None; ours.len()];
        dropped[make] = ours[make].clone();
        let cmp = compare(name, &dropped, &theirs);
        assert!(!cmp.is_clean(), "{name}: dropped fields passed: {cmp:?}");
        assert!(
            cmp.unexplained.len() + 1 >= pin.iter().filter(|p| p.is_some()).count(),
            "{name}: {}",
            describe(&cmp.unexplained)
        );

        // One dropped field is named exactly.
        let one_at = ours
            .iter()
            .take(SELECTORS.len())
            .position(|o| o.is_some() && o != &ours[make])
            .expect("a second populated field");
        let mut one = ours.clone();
        one[one_at] = None;
        let cmp = compare(name, &one, &theirs);
        assert!(!cmp.is_clean(), "{name}: one dropped field passed");
        assert_eq!(named(&cmp.unexplained), vec![SELECTORS[one_at].exiftool]);

        // A second field, wrong.
        if ours[model].is_some() {
            let mut w = ours.clone();
            w[model] = Some("WRONG-MODEL".to_string());
            assert!(!compare(name, &w, &theirs).is_clean());
        }

        // No pin, no pass.
        let unpinned = oracle::compare_jpeg(name, &ours, &theirs);
        assert!(!unpinned.is_clean(), "{name}: compared without its pin");
        assert!(unpinned.stale.iter().any(|s| s.contains("pinned reading")));
    }
}

/// L1: Exif before the scan AND a different Exif after it. ExifTool ignores the
/// second; the model (like the library) reads it, so the later block wins on
/// Make: a per-selector divergence, with the value pinned on both sides.
#[test]
fn exif_before_and_after_the_scan_is_a_recorded_per_selector_divergence() {
    let name = "jpeg-exif-before-and-after-scan";
    let make = index_of("IFD0:Make");
    let model = index_of("IFD0:Model");
    let theirs = theirs_fixture(name);
    assert_eq!(theirs[make].as_deref(), Some("AlphaCam"));
    assert_eq!(theirs[model].as_deref(), Some("OnlyInA"));
    let bytes = fixture(name);
    let meta = jpeg::read(&bytes).expect("a JPEG");
    let after: Vec<bool> = meta
        .jpeg()
        .segments()
        .iter()
        .filter(|s| s.kind() == SegmentKind::Exif)
        .map(|s| s.after_scan())
        .collect();
    assert_eq!(after, [false, true]);
    let ours = ours_fixture(&bytes);
    assert_eq!(ours[make].as_deref(), Some("BravoCam"));
    assert_eq!(ours[model].as_deref(), Some("OnlyInA"));
    let cmp = compare(name, &ours, &theirs);
    assert!(cmp.is_clean(), "{cmp:?}");
    assert_eq!(named(&cmp.explained), vec!["IFD0:Make"]);
    // Both sides Some: the entry is per selector, not the wildcard.
    assert!(cmp.explained[0].ours.is_some() && cmp.explained[0].theirs.is_some());
    assert!(oracle::KNOWN_DIVERGENCES
        .iter()
        .any(|d| d.fixture == name && d.selector == "IFD0:Make"));
}

/// L2: ExifTool derives the Composite GPS coordinate across blocks; the model
/// derives it per block. A documented limit, recorded as a divergence.
#[test]
fn composite_gps_across_two_blocks_is_a_recorded_limit_of_the_model() {
    let name = "jpeg-gps-split-across-two-blocks";
    let lat = index_of("Composite:GPSLatitude");
    let theirs = theirs_fixture(name);
    assert_eq!(theirs[lat].as_deref(), Some("-37.5"));
    let bytes = fixture(name);
    let ours = ours_fixture(&bytes);
    assert_eq!(ours[lat].as_deref(), Some("37.5"));
    assert_eq!(jpeg::read(&bytes).expect("a JPEG").exif_blocks().len(), 2);
    let cmp = compare(name, &ours, &theirs);
    assert!(cmp.is_clean(), "{cmp:?}");
    assert_eq!(named(&cmp.explained), vec!["Composite:GPSLatitude"]);
}

/// L3 / V3: a fatal (not-a-TIFF) Exif block, before or after a good one, must
/// not stop the model's search: both orders read the good block, as ExifTool does.
#[test]
fn a_fatal_exif_block_does_not_stop_the_model_either_order() {
    let make = index_of("IFD0:Make");
    let model = index_of("IFD0:Model");
    for name in ["jpeg-exif-good-then-fatal", "jpeg-exif-fatal-then-good"] {
        let theirs = theirs_fixture(name);
        assert_eq!(
            theirs[make].as_deref(),
            Some("AlphaCam"),
            "{name}: ExifTool"
        );
        assert_eq!(
            theirs[model].as_deref(),
            Some("OnlyInA"),
            "{name}: ExifTool"
        );
        let bytes = fixture(name);
        let meta = jpeg::read(&bytes).expect("a JPEG");
        let fatal = meta
            .exif_blocks()
            .iter()
            .filter(|b| b.container().is_err())
            .count();
        assert_eq!((meta.exif_blocks().len(), fatal), (2, 1), "{name}");
        let ours = ours_fixture(&bytes);
        assert_eq!(ours[make].as_deref(), Some("AlphaCam"), "{name}: model");
        assert!(compare(name, &ours, &theirs).is_clean(), "{name}");
    }
}

/// L3 / V5: the IPTC negative fixture uses resource id 0x0405 (an id next to the
/// IPTC one), so an id compare that accepts it is caught.
#[test]
fn the_iptc_negative_fixture_uses_resource_id_0405() {
    let bytes = fixture("jpeg-iptc-without-0404");
    let at = bytes
        .windows(4)
        .position(|w| w == b"8BIM")
        .expect("an 8BIM resource");
    assert_eq!(&bytes[at + 4..at + 6], &[0x04, 0x05]);
    assert_eq!(
        bytes.windows(4).filter(|w| *w == b"8BIM").count(),
        1,
        "exactly one resource"
    );
}
