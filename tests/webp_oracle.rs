//! ExifTool read-parity oracle for WebP (SPEC-034).
//!
//! The same shape as `tests/png_oracle.rs`: live tests run the installed
//! ExifTool (`-T -n -s3`) over the hand-built WebP fixtures (a variant grid, see
//! `tests/support/oracle_webp.rs`) and the committed `webp_riff` seeds and
//! compare with what `exif_core::webp::read` reads; replay tests run the SAME
//! comparator against the committed records
//! (`tests/oracle-fixtures/exiftool/webp-*.txt`) with no tool installed. A live
//! test with no ExifTool prints `SKIP` and passes, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`.
//!
//! What is compared: the 20 Exif value selectors of `oracle::SELECTORS`, read
//! through the harness's model of ExifTool's several-`EXIF` rule
//! (`oracle::read_ours_webp`: the tags of all blocks merge and the LATER block
//! wins, which is the JPEG rule and NOT the PNG one), plus, on the hand-built
//! fixtures, the presence of a non-empty `XMP ` region (against
//! `XMP-xmp:CreatorTool`) and of a non-empty `ICCP` region (against
//! `ICC-header:ProfileFileSignature`). Every measured difference is a
//! `KNOWN_DIVERGENCES` entry, and the tests require each one to be PRESENT.
//!
//! ExifTool is run, never read or linked. Every claim about its output is in a
//! committed record or in a `KNOWN_DIVERGENCES` entry with a command and a
//! version.

#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_png.rs"]
mod oracle_png;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/oracle_webp.rs"]
mod oracle_webp;
#[path = "support/tools.rs"]
mod tools;

use exif_core::tiff::Container;
use exif_core::webp::{self, WebpRegion};
use exif_core::Error;
use oracle::{Mismatch, SELECTORS};
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
            "cannot read {} ({e}; run `just oracle-record`)",
            path.display()
        )
    });
    tools::parse_record(&text).unwrap_or_else(|| panic!("{}: bad record header", path.display()))
}

/// The committed `webp_riff` seeds, sorted by name.
fn webp_seed_files() -> Vec<(String, Vec<u8>)> {
    let dir = root().join("fuzz/seeds/webp_riff");
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
    oracle_webp::fixtures()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no WebP fixture named {name}"))
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
    read_record("webp-exif-basic").version
}

/// The ExifTool fields of a hand-built fixture's record (22: Exif, then regions).
fn theirs_fixture(name: &str) -> Vec<Option<String>> {
    let rec = read_record(name);
    tools::parse_fields(&oracle::webp_names(), &rec.stdout)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The ExifTool fields of a seed's record (20: Exif only), padded with the two
/// region slots as `None` so the 22-wide comparator applies.
fn theirs_seed(name: &str) -> Vec<Option<String>> {
    let rec = read_record(&format!("webp-seed-{name}"));
    let mut v = tools::parse_fields(&oracle::names(SELECTORS), &rec.stdout)
        .unwrap_or_else(|e| panic!("seed {name}: {e}"));
    v.extend(oracle::WEBP_REGION_SELECTORS.iter().map(|_| None));
    v
}

/// Our 22-wide reading of a hand-built fixture: all `None` when the input is not
/// a WebP at all (`Err`).
fn ours_fixture(bytes: &[u8]) -> Vec<Option<String>> {
    match webp::read(bytes) {
        Ok(m) => oracle::read_webp_all(&m),
        Err(_) => vec![None; SELECTORS.len() + oracle::WEBP_REGION_SELECTORS.len()],
    }
}

/// Our reading of a seed: the Exif selectors only (regions are not compared on
/// seeds, whose packets need not carry the compared tags).
fn ours_seed(bytes: &[u8]) -> Vec<Option<String>> {
    let mut v = match webp::read(bytes) {
        Ok(m) => oracle::read_ours_webp(&m, SELECTORS),
        Err(_) => vec![None; SELECTORS.len()],
    };
    v.extend(oracle::WEBP_REGION_SELECTORS.iter().map(|_| None));
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

/// The index of a selector by its ExifTool spelling.
fn index_of(selector: &str) -> usize {
    oracle::webp_names()
        .iter()
        .position(|n| *n == selector)
        .unwrap_or_else(|| panic!("{selector} is not a WebP selector"))
}

fn hand_built_names() -> Vec<String> {
    oracle_webp::fixtures()
        .into_iter()
        .map(|(n, _)| n)
        .collect()
}

fn compare(
    name: &str,
    ours: &[Option<String>],
    theirs: &[Option<String>],
) -> oracle::JpegComparison {
    oracle::compare_webp(name, ours, theirs)
}

/// Compare one fixture, ours against its record, and require it clean.
fn assert_clean(name: &str) {
    let cmp = compare(name, &ours_fixture(&fixture(name)), &theirs_fixture(name));
    assert!(
        cmp.is_clean(),
        "{name}: unexplained [{}] stale {:?}",
        describe(&cmp.unexplained),
        cmp.stale
    );
}

// ── Fixtures ───────────────────────────────────────────────────────────────

#[test]
fn webp_fixtures_are_deterministic_and_uniquely_named() {
    let a = oracle_webp::fixtures();
    let b = oracle_webp::fixtures();
    assert_eq!(a, b, "two calls must give identical bytes");
    let mut names: Vec<&str> = a.iter().map(|(n, _)| n.as_str()).collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "fixture names must be unique");
    assert_eq!(oracle_webp::wrapped().len(), oracle_tiff::fixtures().len());
    assert!(
        count >= 12 + 80,
        "{count} fixtures: the variant grid shrank"
    );
    for (tiff, _) in oracle_tiff::fixtures() {
        assert!(
            !names.contains(&tiff),
            "{tiff} collides with a TIFF fixture"
        );
    }
    assert!(names.iter().all(|n| n.starts_with("webp-")));
    assert!(names.iter().all(|n| !n.starts_with("webp-seed-")));
    // The normative shapes of the spec are all there.
    for shape in [
        "webp-exif-basic",
        "webp-exif-prefix",
        "webp-exif-two-same-tag",
        "webp-exif-two-same-tag-reversed",
        "webp-exif-bad-then-good",
        "webp-exif-good-then-bad",
        "webp-exif-empty",
        "webp-exif-one-byte",
        "webp-exif-garbage",
        "webp-exif-good-then-prefixed",
        "webp-exif-before-vp8x",
        "webp-exif-after-anim",
        "webp-exif-inside-anmf",
        "webp-exif-anmf-then-top-level",
        "webp-exif-odd-padded",
        "webp-exif-odd-unpadded-last",
        "webp-exif-odd-unpadded-then-xmp",
        "webp-exif-kind-capital-e",
        "webp-exif-kind-lower",
        "webp-exif-kind-last-letter-lower",
        "webp-xmp-basic",
        "webp-xmp-nul-id",
        "webp-xmp-empty",
        "webp-xmp-nul-id-empty",
        "webp-xmp-nul-id-1-bytes",
        "webp-xmp-nul-id-7-bytes",
        "webp-xmp-two",
        "webp-xmp-empty-then-real",
        "webp-xmp-garbage",
        "webp-xmp-kind-lower",
        "webp-xmp-kind-dash",
        "webp-xmp-kind-x",
        "webp-xmp-odd-unpadded-last",
        "webp-xmp-odd-unpadded-then-exif",
        "webp-icc-valid-flag-set",
        "webp-icc-valid-flag-unset",
        "webp-icc-garbage",
        "webp-icc-stub-132",
        "webp-icc-empty",
        "webp-icc-kind-lower",
        "webp-icc-valid-then-garbage",
        "webp-icc-garbage-then-valid",
        "webp-all-three",
        "webp-riff-size-0",
        "webp-riff-size-10",
        "webp-riff-size-odd",
        "webp-riff-size-ffff-fff0",
        "webp-riff-size-exact",
        "webp-tail-12-bytes",
        "webp-tail-3-bytes",
        "webp-tail-1-bytes",
        "webp-chunk-length-past-eof",
        "webp-chunk-length-ffffffff",
        "webp-chunk-length-7fffffff",
        "webp-partial-header-5",
        "webp-partial-header-7",
        "webp-vp8x-len-0",
        "webp-vp8x-len-3",
        "webp-vp8x-len-9",
        "webp-vp8x-len-10",
        "webp-vp8x-len-18",
        "webp-vp8x-two",
        "webp-vp8x-short-then-full",
        "webp-vp8x-late",
        "webp-flags-all-wrong",
        "webp-alph-before-vp8",
        "webp-simple-vp8-exif",
        "webp-header-11-bytes",
        "webp-header-12-bytes",
        "webp-header-rifx",
        "webp-header-wave",
        "webp-header-avi",
        "webp-junk-first",
        "webp-animated-three-anmf",
    ] {
        assert!(names.contains(&shape), "missing fixture {shape}");
    }
}

#[test]
fn every_wrapped_tiff_fixture_is_present_and_matches_its_tiff_bytes() {
    let wrapped = oracle_webp::wrapped();
    assert_eq!(wrapped.len(), oracle_tiff::fixtures().len());
    for (tiff_name, tiff) in oracle_tiff::fixtures() {
        let name = format!("webp-{tiff_name}");
        let bytes = wrapped
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, b)| b)
            .unwrap_or_else(|| panic!("{name} is not a wrapped fixture"));
        let meta = webp::read(bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(meta.webp().is_clean(), "{name}: {:?}", meta.webp_issues());
        assert_eq!(meta.exif_blocks().len(), 1, "{name}: one EXIF chunk");
        assert_eq!(meta.exif_blocks()[0].tiff(), &tiff[..], "{name}: the TIFF");
        assert_eq!(
            oracle_webp::unwrapped_name(&name),
            Some(tiff_name),
            "{name}"
        );
    }
}

// ── Replay (no tool needed) ────────────────────────────────────────────────

#[test]
fn replay_webp_records_are_clean_on_an_unmodified_reading() {
    let mut fixtures = 0usize;
    for (name, bytes) in oracle_webp::fixtures() {
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
    for (seed, bytes) in webp_seed_files() {
        let name = format!("webp-seed-{seed}");
        let rec = read_record(&name);
        assert!(
            rec.outcome.is_none() && oracle::not_run_divergence(&name).is_none(),
            "{name}: no WebP seed is a not-run input (every one was probed and returned)"
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
    assert_eq!(fixtures, oracle_webp::fixtures().len());
    assert_eq!(seeds, 53, "the committed webp_riff seeds");
}

#[test]
fn replay_webp_headers_record_command_version_and_stderr() {
    let fixture_cmd = tools::exiftool_command_line(&oracle::webp_names());
    let seed_cmd = tools::exiftool_command_line(&oracle::names(SELECTORS));
    let mut all: Vec<(String, String, String)> = hand_built_names()
        .into_iter()
        .map(|n| (n.clone(), fixture_cmd.clone(), format!("fixture {n}")))
        .collect();
    all.extend(webp_seed_files().into_iter().map(|(n, _)| {
        (
            format!("webp-seed-{n}"),
            seed_cmd.clone(),
            format!("seed webp_riff/{n}"),
        )
    }));
    let mut with_warnings = 0usize;
    for (name, cmd, source) in all {
        let rec = read_record(&name);
        assert_eq!(rec.command, cmd, "{name}: recorded command");
        assert_eq!(rec.source, source, "{name}: recorded source");
        assert_eq!(rec.version, "13.55", "{name}: recorded version");
        for line in &rec.stderr {
            assert!(
                line.starts_with("Warning: ") || line.starts_with("Error: "),
                "{name}: {line:?}"
            );
            assert!(
                !line.contains("/Users") && !line.contains("/tmp") && !line.contains("/var"),
                "{name}: the path leaked into {line:?}"
            );
        }
        if !rec.stderr.is_empty() {
            with_warnings += 1;
        }
    }
    assert!(
        with_warnings >= 10,
        "only {with_warnings} records have warnings"
    );
}

#[test]
fn the_webp_record_set_is_exactly_the_fixtures_and_the_seeds() {
    let dir = root().join("tests/oracle-fixtures/exiftool");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("read records")
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| n.starts_with("webp-"))
        .collect();
    on_disk.sort();
    let mut want: Vec<String> = hand_built_names()
        .into_iter()
        .map(|n| format!("{n}.txt"))
        .collect();
    want.extend(
        webp_seed_files()
            .into_iter()
            .map(|(n, _)| format!("webp-seed-{n}.txt")),
    );
    // And the one extra record: ExifTool's warnings for the warning table.
    want.push(oracle_webp::WARNINGS_RECORD.to_string());
    want.sort();
    assert_eq!(on_disk, want);
}

/// Replay-and-perturb: for every field our reading has, change exactly that value
/// (or, for a region, remove it) and require exactly one mismatch naming it.
/// Control first: the unperturbed reading is clean. Fixtures with a known
/// divergence are left out (their reading is already, by design, not equal to
/// the record).
#[test]
fn oracle_names_the_one_field_perturbed_in_a_webp_record() {
    let mut perturbed_fields = 0usize;
    let mut norms_seen: Vec<std::mem::Discriminant<oracle::Norm>> = Vec::new();
    for (name, bytes) in oracle_webp::fixtures() {
        if oracle::KNOWN_DIVERGENCES.iter().any(|d| d.fixture == name) {
            continue;
        }
        let theirs = theirs_fixture(&name);
        let ours = ours_fixture(&bytes);
        assert!(compare(&name, &ours, &theirs).is_clean(), "control {name}");
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
                oracle::webp_names()[i],
                "{name}"
            );
            perturbed_fields += 1;
        }
    }
    assert!(
        perturbed_fields > 100,
        "only {perturbed_fields} fields perturbed"
    );
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
            "no WebP record exercised {needed:?}"
        );
    }
    // Both region selectors, perturbed in both directions on real records.
    let xmp = index_of("XMP-xmp:CreatorTool");
    let icc = index_of("ICC-header:ProfileFileSignature");
    for (name, idx) in [("webp-xmp-basic", xmp), ("webp-icc-valid-flag-set", icc)] {
        let theirs = theirs_fixture(name);
        let mut ours = ours_fixture(&fixture(name));
        assert!(compare(name, &ours, &theirs).is_clean(), "control {name}");
        ours[idx] = None;
        let cmp = compare(name, &ours, &theirs);
        assert_eq!(named(&cmp.unexplained), vec![oracle::webp_names()[idx]]);
    }
    for (name, idx) in [("webp-xmp-kind-lower", xmp), ("webp-exif-basic", icc)] {
        let theirs = theirs_fixture(name);
        let mut ours = ours_fixture(&fixture(name));
        ours[idx] = Some(oracle::PRESENT.to_string());
        let cmp = compare(name, &ours, &theirs);
        assert_eq!(named(&cmp.unexplained), vec![oracle::webp_names()[idx]]);
    }
}

/// Container independence, offline: every wrapped fixture's committed WebP
/// record carries exactly the Exif fields (and no region field) of the committed
/// record of its bare TIFF, and our reading agrees bare and wrapped.
#[test]
fn container_independence_every_wrapped_fixture_reads_the_same_bare_and_in_webp() {
    let tiff_names = oracle::names(SELECTORS);
    for (tiff_name, tiff) in oracle_tiff::fixtures() {
        let bare_theirs = tools::parse_fields(&tiff_names, &read_record(tiff_name).stdout)
            .unwrap_or_else(|e| panic!("{tiff_name}: {e}"));
        let name = format!("webp-{tiff_name}");
        let webp_theirs = theirs_fixture(&name);
        assert_eq!(
            &webp_theirs[..SELECTORS.len()],
            &bare_theirs[..],
            "{name}: ExifTool's tags differ bare and in a WebP"
        );
        assert!(
            webp_theirs[SELECTORS.len()..].iter().all(Option::is_none),
            "{name}: no region fields"
        );

        let bare = Container::parse(&tiff).unwrap_or_else(|e| panic!("{tiff_name}: {e:?}"));
        let bare_ours = oracle::read_ours(&bare, SELECTORS);
        let bytes = fixture(&name);
        let meta = webp::read(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let wrapped_ours = oracle::read_ours_webp(&meta, SELECTORS);
        assert_eq!(
            wrapped_ours, bare_ours,
            "{name}: our reading, bare vs in a WebP"
        );
        let has = |v: &[Option<String>]| v.iter().map(Option::is_some).collect::<Vec<_>>();
        assert_eq!(has(&wrapped_ours), has(&bare_theirs), "{name}: tag set");
        assert!(oracle::diff(&bare_ours, &bare_theirs, SELECTORS).is_empty());
        assert!(
            compare(&name, &ours_fixture(&bytes), &webp_theirs).is_clean(),
            "{name}"
        );
    }
}

// ── Live (skip loudly without ExifTool) ────────────────────────────────────

/// Run ExifTool on `bytes` under the name `name` (a `.webp` temp file) with the
/// given selectors; returns (fields, stdout, recorded warnings).
fn live_run(
    name: &str,
    bytes: &[u8],
    selectors: &[&str],
) -> (Vec<Option<String>>, String, Vec<String>) {
    let path = tools::temp_file_with_ext(name, "webp", bytes).expect("temp file");
    let run = tools::run_exiftool_full(&path, selectors);
    let _ = std::fs::remove_file(&path);
    let (stdout, stderr) = run.unwrap_or_else(|e| panic!("{name}: {e}"));
    let fields = tools::parse_fields(selectors, &stdout).unwrap_or_else(|e| panic!("{name}: {e}"));
    (
        fields,
        stdout,
        oracle::webp_warning_lines(tools::warning_lines(&stderr, &path)),
    )
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

        let name = format!("webp-{tiff_name}");
        let (wrapped, _, _) = live_run(&name, &fixture(&name), &names);
        assert_eq!(wrapped, bare, "{name}: live ExifTool, bare vs in a WebP");
        compared += wrapped.iter().filter(|v| v.is_some()).count();
    }
    println!("{test}: {compared} ExifTool values identical bare and in a WebP");
    assert!(compared > 60, "only {compared} values");
}

#[test]
fn live_webp_parity_on_fixtures() {
    let test = "live_webp_parity_on_fixtures";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = oracle::webp_names();
    let (mut compared, mut explained) = (0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    for (name, bytes) in oracle_webp::fixtures() {
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
        oracle_webp::fixtures().len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn live_webp_parity_on_the_webp_seeds() {
    let test = "live_webp_parity_on_the_webp_seeds";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = oracle::names(SELECTORS);
    let (mut compared, mut explained) = (0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    let seeds = webp_seed_files();
    for (seed, bytes) in &seeds {
        let name = format!("webp-seed-{seed}");
        // A `.webp` copy of the committed seed, alone, under the wall-clock
        // limit; a timeout is a failure, not a skip.
        let (mut theirs, stdout, warnings) = live_run(&name, bytes, &names);
        compared += theirs.iter().filter(|t| t.is_some()).count();
        theirs.extend(oracle::WEBP_REGION_SELECTORS.iter().map(|_| None));
        let cmp = compare(&name, &ours_seed(bytes), &theirs);
        for m in &cmp.unexplained {
            failures.push(format!("{name}: {m}"));
        }
        for s in &cmp.stale {
            failures.push(s.clone());
        }
        explained += cmp.explained.len();
        let rec = read_record(&name);
        if rec.stdout != stdout || rec.stderr != warnings {
            failures.push(format!("{name}: the committed record is stale"));
        }
    }
    println!(
        "{test}: {} seeds, {compared} ExifTool values compared, {explained} explained by a \
         known divergence",
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
fn live_webp_skip_is_loud_and_required_mode_fails() {
    for test in [
        "live_container_independence_against_exiftool",
        "live_webp_parity_on_fixtures",
        "live_webp_parity_on_the_webp_seeds",
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

// ── Measured behaviour (written from the committed records) ────────────────

/// The several-`EXIF` rule, measured (records `webp-exif-two-same-tag`,
/// `-reversed`, `webp-exif-bad-then-good`, `-good-then-bad`, `-good-then-
/// prefixed`): the tags of the blocks MERGE and the later block wins per tag (A
/// is Make AlphaCam + Model OnlyInA, B is Make BravoCam). That is the JPEG rule
/// and NOT the PNG one. The library's own `exif()` is the first block, so the
/// model is the harness's, as for JPEG.
#[test]
fn several_exif_model_is_measured_and_pinned() {
    let make = index_of("IFD0:Make");
    let model = index_of("IFD0:Model");
    for (name, want_make, want_model) in [
        // A then B: B's Make wins, A's Model survives (merged).
        ("webp-exif-two-same-tag", "BravoCam", Some("OnlyInA")),
        // B then A: A's Make wins.
        (
            "webp-exif-two-same-tag-reversed",
            "AlphaCam",
            Some("OnlyInA"),
        ),
        // A block that is not a TIFF is ignored, in either order.
        ("webp-exif-bad-then-good", "AlphaCam", Some("OnlyInA")),
        ("webp-exif-good-then-bad", "AlphaCam", Some("OnlyInA")),
        // A prefixed later block merges like any other.
        ("webp-exif-good-then-prefixed", "BravoCam", Some("OnlyInA")),
    ] {
        let theirs = theirs_fixture(name);
        assert_eq!(theirs[make].as_deref(), Some(want_make), "{name}: Make");
        assert_eq!(theirs[model].as_deref(), want_model, "{name}: Model");
        let ours = ours_fixture(&fixture(name));
        assert_eq!(ours[make].as_deref(), Some(want_make), "{name}: our Make");
        assert_eq!(ours[model].as_deref(), want_model, "{name}: our Model");
        assert_clean(name);
    }
    // The PNG model (first block only) would be wrong here, so the two models
    // are not interchangeable: the library's `exif()` reads A's Make for the
    // first fixture.
    let bytes = fixture("webp-exif-two-same-tag");
    let meta = webp::read(&bytes).expect("a WebP");
    let first = meta.exif().expect("primary exif");
    let first_make = oracle::read_one(first, &SELECTORS[make]);
    assert_eq!(first_make.as_deref(), Some("AlphaCam"));
    assert_eq!(meta.exif_blocks().len(), 2);
}

#[test]
fn region_presence_xmp_and_icc_agree_with_exiftool() {
    let xmp = index_of("XMP-xmp:CreatorTool");
    let icc = index_of("ICC-header:ProfileFileSignature");
    for (name, has_xmp, has_icc) in [
        ("webp-xmp-basic", true, false),
        ("webp-xmp-nul-id", true, false),
        ("webp-xmp-two", true, false),
        ("webp-xmp-empty-then-real", true, false),
        ("webp-icc-valid-flag-set", false, true),
        ("webp-icc-valid-flag-unset", false, true),
        ("webp-icc-valid-trailing-garbage", false, true),
        ("webp-all-three", true, true),
        ("webp-exif-basic", false, false),
    ] {
        let theirs = theirs_fixture(name);
        assert_eq!(theirs[xmp].is_some(), has_xmp, "{name}: ExifTool XMP");
        assert_eq!(theirs[icc].is_some(), has_icc, "{name}: ExifTool ICC");
        let ours = ours_fixture(&fixture(name));
        assert_eq!(ours[xmp].is_some(), has_xmp, "{name}: our XMP");
        assert_eq!(ours[icc].is_some(), has_icc, "{name}: our ICC");
        assert_clean(name);
    }
}

#[test]
fn empty_regions_never_count_as_present() {
    let xmp = index_of("XMP-xmp:CreatorTool");
    let icc = index_of("ICC-header:ProfileFileSignature");
    for (name, idx, located) in [
        ("webp-xmp-empty", xmp, 1usize),
        ("webp-xmp-nul-id-empty", xmp, 1),
        ("webp-icc-empty", icc, 1),
    ] {
        // The library locates the (empty) region ...
        let bytes = fixture(name);
        let meta = webp::read(&bytes).expect("a WebP");
        assert_eq!(meta.regions().len(), located, "{name}: located regions");
        assert!(meta.regions()[0].data().is_empty(), "{name}");
        // ... but neither the model nor ExifTool counts it.
        assert!(ours_fixture(&bytes)[idx].is_none(), "{name}: ours");
        assert!(theirs_fixture(name)[idx].is_none(), "{name}: theirs");
        assert_clean(name);
    }
}

#[test]
fn chunk_code_case_rules_agree() {
    let make = index_of("IFD0:Make");
    let xmp = index_of("XMP-xmp:CreatorTool");
    // Wrong-case and near-miss codes are read by neither side.
    for name in [
        "webp-exif-kind-capital-e",
        "webp-exif-kind-lower",
        "webp-exif-kind-last-letter-lower",
    ] {
        assert!(theirs_fixture(name)[make].is_none(), "{name}: ExifTool");
        assert!(ours_fixture(&fixture(name))[make].is_none(), "{name}: ours");
        assert_clean(name);
    }
    for name in [
        "webp-xmp-kind-lower",
        "webp-xmp-kind-dash",
        "webp-xmp-kind-x",
        "webp-icc-kind-lower",
    ] {
        assert!(theirs_fixture(name)[xmp].is_none(), "{name}: ExifTool");
        assert!(ours_fixture(&fixture(name))[xmp].is_none(), "{name}: ours");
        assert_clean(name);
    }
}

#[test]
fn declared_size_never_limits_the_walk_agrees() {
    let make = index_of("IFD0:Make");
    for name in [
        "webp-riff-size-0",
        "webp-riff-size-10",
        "webp-riff-size-small",
        "webp-riff-size-odd",
        "webp-riff-size-ffff-fff0",
        "webp-riff-size-exact",
    ] {
        assert_eq!(
            theirs_fixture(name)[make].as_deref(),
            Some("AlphaCam"),
            "{name}: ExifTool still reads the EXIF"
        );
        assert_eq!(
            ours_fixture(&fixture(name))[make].as_deref(),
            Some("AlphaCam"),
            "{name}: so do we"
        );
        assert_clean(name);
    }
}

/// The pad rule: a missing pad byte on the LAST chunk still reads; followed by
/// another chunk, that chunk is lost. Compared, not assumed: both sides must
/// lose the same chunk.
#[test]
fn padding_rules_agree() {
    let make = index_of("IFD0:Make");
    let xmp = index_of("XMP-xmp:CreatorTool");
    for (name, idx, read) in [
        ("webp-exif-odd-padded", make, true),
        ("webp-exif-odd-unpadded-last", make, true),
        ("webp-xmp-odd-padded", xmp, true),
        ("webp-xmp-odd-unpadded-last", xmp, true),
    ] {
        assert_eq!(
            theirs_fixture(name)[idx].is_some(),
            read,
            "{name}: ExifTool"
        );
        assert_eq!(
            ours_fixture(&fixture(name))[idx].is_some(),
            read,
            "{name}: ours"
        );
        assert_clean(name);
    }
    // The chunk after an unpadded odd chunk: lost by both sides alike.
    for name in [
        "webp-exif-odd-unpadded-then-xmp",
        "webp-xmp-odd-unpadded-then-exif",
    ] {
        let theirs = theirs_fixture(name);
        let ours = ours_fixture(&fixture(name));
        assert_eq!(
            theirs.iter().map(Option::is_some).collect::<Vec<_>>(),
            ours.iter().map(Option::is_some).collect::<Vec<_>>(),
            "{name}: the same chunks are read"
        );
        assert_clean(name);
    }
}

#[test]
fn vp8x_shapes_agree() {
    let make = index_of("IFD0:Make");
    for name in [
        "webp-vp8x-len-0",
        "webp-vp8x-len-3",
        "webp-vp8x-len-9",
        "webp-vp8x-len-10",
        "webp-vp8x-len-18",
        "webp-vp8x-two",
        "webp-vp8x-short-then-full",
        "webp-vp8x-late",
        "webp-flags-all-wrong",
        "webp-exif-flag-unset",
        "webp-exif-no-vp8x",
        "webp-exif-before-vp8x",
    ] {
        assert!(
            theirs_fixture(name)[make].is_some(),
            "{name}: ExifTool reads the EXIF whatever VP8X says"
        );
        assert!(ours_fixture(&fixture(name))[make].is_some(), "{name}: ours");
        assert_clean(name);
    }
}

/// `RIFF....WAVE`, `RIFF....AVI ` and `RIFX....WEBP` are not WebP: we return `Err` (all `None`),
/// ExifTool reads them as another format and prints Make. Both are recorded
/// divergences (we are right); the short headers agree.
#[test]
fn fatal_headers_are_recorded_divergences_where_exiftool_reads_another_format() {
    let make = index_of("IFD0:Make");
    for (name, tool_make) in [
        ("webp-header-rifx", "RifxCam"),
        ("webp-header-wave", "WaveCam"),
        ("webp-header-avi", "WaveCam"),
    ] {
        assert!(webp::read(&fixture(name)).is_err(), "{name}: ours is Err");
        assert_eq!(
            theirs_fixture(name)[make].as_deref(),
            Some(tool_make),
            "{name}: ExifTool reads it as another format"
        );
        let cmp = compare(name, &ours_fixture(&fixture(name)), &theirs_fixture(name));
        assert!(cmp.is_clean() && !cmp.explained.is_empty(), "{name}");
        assert!(
            oracle::known_divergence(name, "IFD0:Make")
                .is_some_and(|d| d.question_id == "webp-riff-other-forms-read-by-exiftool"),
            "{name}"
        );
    }
    // The short headers agree: 11 bytes is `Err`, 12 bytes is a WebP with no
    // chunks (`Ok`, nothing read); ExifTool reads nothing from either.
    assert!(webp::read(&fixture("webp-header-11-bytes")).is_err());
    assert!(webp::read(&fixture("webp-header-12-bytes")).is_ok());
    for name in ["webp-header-11-bytes", "webp-header-12-bytes"] {
        assert!(theirs_fixture(name).iter().all(Option::is_none), "{name}");
        assert_clean(name);
    }
}

fn issue_kind(e: &Error) -> String {
    let d = format!("{e:?}");
    d.split([' ', '{', '('])
        .next()
        .unwrap_or_default()
        .to_string()
}

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

/// Our WebP-layer issue kinds (the walk's and the extraction's), run-length
/// encoded, in report order.
fn our_webp_issue_kinds(bytes: &[u8]) -> String {
    match webp::read(bytes) {
        Ok(m) => run_length(
            &m.webp_issues()
                .iter()
                .map(|i| issue_kind(i.error()))
                .collect::<Vec<_>>(),
        ),
        Err(e) => format!("fatal:{}", issue_kind(&e)),
    }
}

/// Our Exif block states, run-length encoded.
fn our_block_states(bytes: &[u8]) -> String {
    match webp::read(bytes) {
        Ok(m) => run_length(
            &m.exif_blocks()
                .iter()
                .map(|b| match b.container() {
                    Ok(c) if c.issues().is_empty() => "ok".to_string(),
                    Ok(c) => format!("issues={}", c.issues().len()),
                    Err(_) => "fatal".to_string(),
                })
                .collect::<Vec<_>>(),
        ),
        Err(_) => "-".to_string(),
    }
}

/// `(fixture, our WebP-layer issue kinds, our Exif block states, substrings of
/// ExifTool's warnings: exactly one line each)`. See SPEC-034 for what is
/// silent and what is louder.
const WARNING_TABLE: &[(&str, &str, &str, &[&str])] = &[
    // The prefix: both sides warn.
    (
        "webp-exif-prefix",
        "ImproperExifHeader",
        "ok",
        &["Improper EXIF header"],
    ),
    // Truncated tail: ours is `Truncated` for all of them; ExifTool is silent
    // for 12 trailing bytes (measured, SPEC-034 expected a warning) and warns
    // "Error reading RIFF file" for 3 bytes, a length past the end and a
    // partial header.
    (
        "webp-tail-12-bytes",
        "RiffSizeMismatch,Truncated",
        "ok",
        &[],
    ),
    (
        "webp-tail-3-bytes",
        "RiffSizeMismatch,Truncated",
        "ok",
        &["Error reading RIFF file"],
    ),
    (
        "webp-chunk-length-past-eof",
        "RiffSizeMismatch,Truncated",
        "ok",
        &["Error reading RIFF file"],
    ),
    (
        "webp-partial-header-5",
        "RiffSizeMismatch,Truncated",
        "ok",
        &["Error reading RIFF file"],
    ),
    // The missing pad: both sides warn.
    (
        "webp-exif-odd-unpadded-last",
        "MissingPad",
        "ok",
        &["No padding on odd-sized EXIF chunk"],
    ),
    (
        "webp-xmp-odd-unpadded-last",
        "MissingPad",
        "",
        &["No padding on odd-sized XMP  chunk"],
    ),
    // `XMP\0`: both sides warn; a short one is also "Empty XMP" to ExifTool.
    (
        "webp-xmp-nul-id",
        "NonCanonicalChunkId",
        "",
        &["Incorrect XMP tag ID"],
    ),
    (
        "webp-xmp-nul-id-3-bytes",
        "NonCanonicalChunkId",
        "",
        &["Incorrect XMP tag ID", "Empty XMP"],
    ),
    // Where ExifTool is silent and we are not.
    ("webp-riff-size-0", "RiffSizeMismatch", "ok", &[]),
    ("webp-riff-size-small", "RiffSizeMismatch", "ok", &[]),
    ("webp-flags-all-wrong", "ChunkFlagMismatch*3", "ok", &[]),
    (
        "webp-vp8x-two",
        "MisplacedVp8x,ChunkFlagMismatch",
        "ok",
        &[],
    ),
    ("webp-vp8x-len-3", "BadVp8x", "ok", &[]),
    ("webp-exif-no-vp8x", "MetadataWithoutVp8x", "ok", &[]),
    // Where ExifTool is louder: it validates what we only locate.
    ("webp-icc-garbage", "", "", &["Bad length ICC_Profile"]),
    ("webp-icc-stub-132", "", "", &["Bad length ICC_Profile"]),
    ("webp-xmp-garbage", "", "", &["Empty XMP"]),
    // A file that is not a WebP.
    (
        "webp-header-11-bytes",
        "fatal:Truncated",
        "-",
        &["File format error"],
    ),
    (
        "webp-header-rifx",
        "fatal:NotWebp",
        "-",
        &["Processing TIFF-like data"],
    ),
    // Clean on both sides.
    ("webp-exif-basic", "", "ok", &[]),
    ("webp-xmp-basic", "", "", &[]),
    ("webp-icc-valid-flag-set", "", "", &[]),
];

/// The committed warning record: `## name` sections of ExifTool's
/// `[group] Warning : text` lines.
fn recorded_warnings(section: &str) -> Vec<String> {
    let text = std::fs::read_to_string(record_path("webp-warnings")).unwrap_or_else(|e| {
        panic!("cannot read webp-warnings.txt ({e}); run `just oracle-record`")
    });
    let mut in_section = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("## ") {
            in_section = name == section;
        } else if in_section && !line.is_empty() {
            out.push(line.to_string());
        }
    }
    out
}

#[test]
fn warning_table_pins_our_issue_kinds_beside_exiftools_warnings() {
    // The recorded set is exactly the table's fixtures, in the table's order.
    let text = std::fs::read_to_string(record_path("webp-warnings")).expect("webp-warnings.txt");
    let sections: Vec<&str> = text.lines().filter_map(|l| l.strip_prefix("## ")).collect();
    let rows: Vec<&str> = WARNING_TABLE.iter().map(|r| r.0).collect();
    assert_eq!(
        sections, rows,
        "webp-warnings.txt must hold the table's fixtures"
    );
    assert_eq!(
        rows,
        oracle_webp::WARNING_FIXTURES,
        "the recorder's list is the table"
    );
    let header = tools::parse_record(&text).expect("header");
    assert_eq!(header.version, "13.55");

    for (name, ours_kinds, blocks, warnings) in WARNING_TABLE {
        let bytes = fixture(name);
        assert_eq!(
            &our_webp_issue_kinds(&bytes),
            ours_kinds,
            "{name}: our WebP-layer issues"
        );
        assert_eq!(&our_block_states(&bytes), blocks, "{name}: our Exif blocks");
        let theirs = recorded_warnings(name);
        assert_eq!(
            theirs.len(),
            warnings.len(),
            "{name}: ExifTool printed {theirs:?}"
        );
        for sub in *warnings {
            assert!(
                theirs.iter().any(|l| l.contains(sub)),
                "{name}: ExifTool printed {theirs:?}, expected a line with {sub:?}"
            );
        }
    }
}

/// Every divergence is tracked: its `question_id` is a question in
/// `guidance/questions.yaml` (process rule 2 of SPEC-011).
#[test]
fn every_known_divergence_has_a_question_entry() {
    let questions = std::fs::read_to_string(root().join("guidance/questions.yaml"))
        .expect("read guidance/questions.yaml");
    let entries: Vec<_> = oracle::KNOWN_DIVERGENCES
        .iter()
        .filter(|d| d.fixture.starts_with("webp-"))
        .collect();
    for d in &entries {
        assert!(
            questions.contains(&format!("- id: {}\n", d.question_id)),
            "{} {}: no question `{}` in guidance/questions.yaml",
            d.fixture,
            d.selector,
            d.question_id
        );
        assert!(!d.command.is_empty() && !d.measured_stdout.is_empty() && !d.right_side.is_empty());
        assert_eq!(d.exiftool_version, "13.55");
        // The recorded command is the one the records carry.
        assert_eq!(
            d.command,
            tools::exiftool_command_line(&oracle::webp_names()),
            "{}: command",
            d.fixture
        );
    }
    // The three the spec names are present, with the fixtures it names.
    for (id, fixture) in [
        ("webp-iccp-profile-not-validated", "webp-icc-garbage"),
        ("webp-iccp-profile-not-validated", "webp-icc-stub-132"),
        ("webp-xmp-content-not-parsed", "webp-xmp-garbage"),
        ("webp-xmp-content-not-parsed", "webp-xmp-nul-id-1-bytes"),
        ("webp-xmp-content-not-parsed", "webp-xmp-nul-id-7-bytes"),
        ("webp-riff-other-forms-read-by-exiftool", "webp-header-wave"),
        ("webp-riff-other-forms-read-by-exiftool", "webp-header-rifx"),
    ] {
        assert!(
            entries
                .iter()
                .any(|d| d.question_id == id && d.fixture == fixture),
            "missing divergence {id} on {fixture}"
        );
    }
}

#[test]
fn a_divergence_that_stops_being_one_fails() {
    // Every divergence flips to a failure the moment the two sides agree.
    for d in oracle::KNOWN_DIVERGENCES
        .iter()
        .filter(|d| d.fixture.starts_with("webp-"))
    {
        let name = d.fixture;
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
        // The same difference on a fixture with NO entry is just a failure.
        let stranger = compare("webp-exif-basic", &ours, &theirs);
        assert!(
            !stranger.unexplained.is_empty(),
            "{name}: not covered elsewhere"
        );
    }
}

/// The located region and block bytes are what the builders wrote: the chunk's
/// code is at the reported offset and its data follows the 8-byte head.
#[test]
fn located_region_bytes_equal_what_the_builders_wrote() {
    let at = |bytes: &[u8], offset: usize, code: &[u8; 4], data: &[u8]| {
        assert_eq!(bytes.get(offset..offset + 4), Some(&code[..]), "code");
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().expect("4"));
        assert_eq!(size as usize, data.len(), "declared size");
        assert_eq!(&bytes[offset + 8..offset + 8 + data.len()], data, "data");
    };
    let packet = oracle_png::xmp_packet();
    let icc = oracle_png::icc_profile();

    let bytes = fixture("webp-all-three");
    let meta = webp::read(&bytes).expect("a WebP");
    assert_eq!(meta.regions().len(), 2);
    match meta.regions()[0] {
        WebpRegion::Icc { offset, data } => {
            assert_eq!(data, &icc[..]);
            at(&bytes, offset, b"ICCP", &icc);
        }
        other => panic!("expected the ICC region first, got {other:?}"),
    }
    match meta.regions()[1] {
        WebpRegion::Xmp { offset, data } => {
            assert_eq!(data, &packet[..]);
            at(&bytes, offset, b"XMP ", &packet);
        }
        other => panic!("expected the XMP region second, got {other:?}"),
    }
    assert_eq!(meta.exif_blocks().len(), 1);
    let block = &meta.exif_blocks()[0];
    assert_eq!(block.tiff(), &oracle_webp::tiff_a()[..]);
    at(&bytes, block.offset(), b"EXIF", &oracle_webp::tiff_a());
    assert_eq!(block.tiff_at(), block.offset() + 8);

    // `XMP\0` is located as XMP, with its own code at the offset.
    let bytes = fixture("webp-xmp-nul-id");
    let meta = webp::read(&bytes).expect("a WebP");
    match meta.regions()[0] {
        WebpRegion::Xmp { offset, data } => {
            assert_eq!(data, &packet[..]);
            at(&bytes, offset, b"XMP\0", &packet);
        }
        other => panic!("expected an XMP region, got {other:?}"),
    }

    // The prefix is skipped: the TIFF starts 6 bytes after the chunk data.
    let bytes = fixture("webp-exif-prefix");
    let meta = webp::read(&bytes).expect("a WebP");
    let block = &meta.exif_blocks()[0];
    assert_eq!(block.tiff(), &oracle_webp::tiff_a()[..]);
    assert_eq!(block.tiff_at(), block.offset() + 8 + 6);
}
