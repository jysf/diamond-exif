//! ExifTool read-parity oracle for PNG (SPEC-031).
//!
//! The same shape as `tests/jpeg_oracle.rs`: live tests run the installed ExifTool
//! (`-T -n -s3`) over the hand-built PNG fixtures and the committed `png_chunks`
//! seeds and compare with what `exif_core::png::read` reads; replay tests run the
//! SAME comparator against the committed records
//! (`tests/oracle-fixtures/exiftool/png-*.txt`) with no tool installed. A live
//! test with no ExifTool prints `SKIP` and passes, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`.
//!
//! What is compared: the 20 Exif value selectors of `oracle::SELECTORS`, read
//! through the harness's model of ExifTool's several-`eXIf` rule
//! (`oracle::read_ours_png`: the first block that is a TIFF wins and later ones
//! are ignored: measured, it differs from the JPEG rule), plus, on the
//! hand-built fixtures, the presence of readable XMP text and of an `iCCP` chunk.
//! Every measured difference is a `KNOWN_DIVERGENCES` entry, and the tests
//! require each one to be PRESENT, so the list cannot rot.
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
#[path = "support/tools.rs"]
mod tools;

use exif_core::png::{self, ChunkKind, PngRegion};
use exif_core::tiff::Container;
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

/// The committed `png_chunks` seeds, sorted by name.
fn png_seed_files() -> Vec<(String, Vec<u8>)> {
    let dir = root().join("fuzz/seeds/png_chunks");
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
    oracle_png::fixtures()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no PNG fixture named {name}"))
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
    read_record("png-exif-basic-le").version
}

/// The ExifTool fields of a hand-built fixture's record (22: Exif, then regions).
fn theirs_fixture(name: &str) -> Vec<Option<String>> {
    let rec = read_record(name);
    tools::parse_fields(&oracle::png_names(), &rec.stdout).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The ExifTool fields of a seed's record (20: Exif only), padded with the two
/// region slots as `None` so the 22-wide comparator applies.
fn theirs_seed(name: &str) -> Vec<Option<String>> {
    let rec = read_record(&format!("png-seed-{name}"));
    let mut v = tools::parse_fields(&oracle::names(SELECTORS), &rec.stdout)
        .unwrap_or_else(|e| panic!("seed {name}: {e}"));
    v.extend(oracle::PNG_REGION_SELECTORS.iter().map(|_| None));
    v
}

/// Our 22-wide reading of a hand-built fixture: all `None` when the input is not
/// a PNG at all.
fn ours_fixture(bytes: &[u8]) -> Vec<Option<String>> {
    match png::read(bytes) {
        Ok(m) => oracle::read_png_all(&m),
        Err(_) => vec![None; SELECTORS.len() + oracle::PNG_REGION_SELECTORS.len()],
    }
}

/// Our reading of a seed: the Exif selectors only (regions are not compared on
/// seeds, which carry text chunks that need not hold the compared tags).
fn ours_seed(bytes: &[u8]) -> Vec<Option<String>> {
    let mut v = match png::read(bytes) {
        Ok(m) => oracle::read_ours_png(&m, SELECTORS),
        Err(_) => vec![None; SELECTORS.len()],
    };
    v.extend(oracle::PNG_REGION_SELECTORS.iter().map(|_| None));
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
    oracle::png_names()
        .iter()
        .position(|n| *n == selector)
        .unwrap_or_else(|| panic!("{selector} is not a PNG selector"))
}

fn hand_built_names() -> Vec<String> {
    oracle_png::fixtures().into_iter().map(|(n, _)| n).collect()
}

/// ExifTool's Exif fields of the record that pins an unread-wildcard fixture.
fn pin_for(name: &str) -> Option<Vec<Option<String>>> {
    let record = oracle::png_pin_record(name)?;
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

/// `oracle::compare_png` with the pin a wildcard fixture needs.
fn compare(
    name: &str,
    ours: &[Option<String>],
    theirs: &[Option<String>],
) -> oracle::JpegComparison {
    oracle::compare_png(name, ours, theirs, pin_for(name).as_deref())
}

// ── Fixtures ───────────────────────────────────────────────────────────────

#[test]
fn png_fixtures_are_deterministic_and_uniquely_named() {
    let a = oracle_png::fixtures();
    let b = oracle_png::fixtures();
    assert_eq!(a, b, "two calls must give identical bytes");
    let mut names: Vec<&str> = a.iter().map(|(n, _)| n.as_str()).collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "fixture names must be unique");
    assert_eq!(count, 12 + 46, "12 wrapped TIFFs and 46 shapes");
    for (tiff, _) in oracle_tiff::fixtures() {
        assert!(
            !names.contains(&tiff),
            "{tiff} collides with a TIFF fixture"
        );
    }
    assert!(names.iter().all(|n| n.starts_with("png-")));
    assert!(names.iter().all(|n| !n.starts_with("png-seed-")));
    // The normative shapes of the spec are all there.
    for shape in [
        "png-exif-after-idat",
        "png-exif-after-iend",
        "png-exif-prefix",
        "png-exif-empty",
        "png-exif-not-tiff",
        "png-two-exif-same-tag",
        "png-two-exif-same-tag-reversed",
        "png-two-exif-disjoint-tags",
        "png-exif-bad-crc",
        "png-no-ihdr",
        "png-no-iend",
        "png-length-past-eof",
        "png-length-80000000",
        "png-xmp-itxt",
        "png-xmp-itxt-compressed",
        "png-xmp-text",
        "png-xmp-ztxt",
        "png-xmp-other-keyword",
        "png-xmp-lowercase-keyword",
        "png-iccp-valid",
        "png-iccp-bad-header",
        "png-raw-exif-text",
        "png-raw-exif-ztxt",
        "png-raw-exif-itxt",
        "png-raw-xmp-text",
        "png-raw-xmp-ztxt",
        "png-raw-xmp-itxt",
        "png-keyword-80",
        "png-xmp-itxt-flag-2",
        "png-exif-and-raw-exif",
        "png-trailer-bytes",
        "png-exif-before-ihdr",
        "png-iccp-name-79",
        "png-iccp-name-80",
        "png-iccp-name-empty",
        "png-xmp-itxt-lang",
        "png-xmp-itxt-lang-compressed",
        "png-xmp-itxt-translated",
        "png-xmp-itxt-lang-translated",
        "png-exif-xmp-iccp",
        "png-text-no-nul",
    ] {
        assert!(names.contains(&shape), "missing fixture {shape}");
    }
    // Every fixture is a PNG by its first eight bytes.
    for (n, bytes) in &a {
        assert_eq!(bytes.get(..8), Some(&oracle_png::SIGNATURE[..]), "{n}");
    }
}

/// The independent builders agree with the specification's published test
/// vectors (CRC-32 of "123456789" and the Adler-32 of "Wikipedia"), so the
/// oracle's own encoders are not wrong in the same way as nothing else.
#[test]
fn fixture_builders_match_published_vectors() {
    assert_eq!(oracle_png::crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(oracle_png::adler32(b"Wikipedia"), 0x11E6_0398);
    // The empty IEND chunk has the well-known CRC AE 42 60 82.
    assert_eq!(
        oracle_png::iend(),
        [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]
    );
}

#[test]
fn every_wrapped_tiff_fixture_is_present_and_matches_its_tiff_bytes() {
    let wrapped = oracle_png::wrapped();
    assert_eq!(wrapped.len(), oracle_tiff::fixtures().len());
    for (tiff_name, tiff) in oracle_tiff::fixtures() {
        let name = format!("png-{tiff_name}");
        let bytes = wrapped
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, b)| b)
            .unwrap_or_else(|| panic!("{name} is not a wrapped fixture"));
        let p = png::Png::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(p.is_clean(), "{name}: {:?}", p.issues());
        let exif: Vec<_> = p
            .chunks()
            .iter()
            .filter(|c| c.kind() == ChunkKind::Exif)
            .collect();
        assert_eq!(exif.len(), 1, "{name}: one eXIf chunk");
        assert_eq!(exif[0].data(), &tiff[..], "{name}: the chunk is the TIFF");
        assert!(!exif[0].after_idat());
        assert_eq!(oracle_png::unwrapped_name(&name), Some(tiff_name), "{name}");
    }
}

// ── Replay (no tool needed) ────────────────────────────────────────────────

#[test]
fn replay_png_records_are_clean_on_an_unmodified_reading() {
    let mut fixtures = 0usize;
    for (name, bytes) in oracle_png::fixtures() {
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
    for (seed, bytes) in png_seed_files() {
        let name = format!("png-seed-{seed}");
        let rec = read_record(&name);
        assert!(
            rec.outcome.is_none() && oracle::not_run_divergence(&name).is_none(),
            "{name}: no PNG seed is a not-run input (every one was probed and returned)"
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
    assert_eq!(fixtures, 58);
    assert_eq!(seeds, 68);
}

#[test]
fn replay_png_headers_record_command_version_and_stderr() {
    let fixture_cmd = tools::exiftool_command_line(&oracle::png_names());
    let seed_cmd = tools::exiftool_command_line(&oracle::names(SELECTORS));
    let mut all: Vec<(String, String, String)> = hand_built_names()
        .into_iter()
        .map(|n| (n.clone(), fixture_cmd.clone(), format!("fixture {n}")))
        .collect();
    all.extend(png_seed_files().into_iter().map(|(n, _)| {
        (
            format!("png-seed-{n}"),
            seed_cmd.clone(),
            format!("seed png_chunks/{n}"),
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
        with_warnings >= 20,
        "only {with_warnings} records have warnings"
    );
}

#[test]
fn the_png_record_set_is_exactly_the_fixtures_and_the_seeds() {
    let dir = root().join("tests/oracle-fixtures/exiftool");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("read records")
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| n.starts_with("png-"))
        .collect();
    on_disk.sort();
    let mut want: Vec<String> = hand_built_names()
        .into_iter()
        .map(|n| format!("{n}.txt"))
        .collect();
    want.extend(
        png_seed_files()
            .into_iter()
            .map(|(n, _)| format!("png-seed-{n}.txt")),
    );
    // And the one extra record: ExifTool's warnings for the warning table.
    want.push(oracle_png::WARNINGS_RECORD.to_string());
    want.sort();
    assert_eq!(on_disk, want);
}

/// Replay-and-perturb: for every field our reading has, change exactly that value
/// (or, for a region, remove it) and require exactly one mismatch naming it.
/// Control first: the unperturbed reading is clean. Fixtures with a known
/// divergence are left out (their reading is already, by design, not equal to the
/// record).
#[test]
fn oracle_names_the_one_field_perturbed_in_a_png_record() {
    let mut perturbed_fields = 0usize;
    let mut norms_seen: Vec<std::mem::Discriminant<oracle::Norm>> = Vec::new();
    for (name, bytes) in oracle_png::fixtures() {
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
                oracle::png_names()[i],
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
            "no PNG record exercised {needed:?}"
        );
    }
    // Both region selectors, perturbed in both directions on real records.
    let xmp = index_of("XMP-xmp:CreatorTool");
    let icc = index_of("PNG:ProfileName");
    for (name, idx) in [("png-xmp-itxt", xmp), ("png-iccp-valid", icc)] {
        let theirs = theirs_fixture(name);
        let mut ours = ours_fixture(&fixture(name));
        assert!(compare(name, &ours, &theirs).is_clean(), "control {name}");
        ours[idx] = None;
        let cmp = compare(name, &ours, &theirs);
        assert_eq!(named(&cmp.unexplained), vec![oracle::png_names()[idx]]);
    }
    for (name, idx) in [("png-xmp-other-keyword", xmp), ("png-exif-basic-le", icc)] {
        let theirs = theirs_fixture(name);
        let mut ours = ours_fixture(&fixture(name));
        ours[idx] = Some(oracle::PRESENT.to_string());
        let cmp = compare(name, &ours, &theirs);
        assert_eq!(named(&cmp.unexplained), vec![oracle::png_names()[idx]]);
    }
}

/// Container independence, offline: every wrapped fixture's committed PNG record
/// carries exactly the Exif fields (and no region field) of the committed record
/// of its bare TIFF, and our reading agrees bare and wrapped, with the same set
/// of tags.
#[test]
fn container_independence_every_wrapped_fixture_reads_the_same_bare_and_in_png() {
    let tiff_names = oracle::names(SELECTORS);
    for (tiff_name, tiff) in oracle_tiff::fixtures() {
        let bare_theirs = tools::parse_fields(&tiff_names, &read_record(tiff_name).stdout)
            .unwrap_or_else(|e| panic!("{tiff_name}: {e}"));
        let name = format!("png-{tiff_name}");
        let png_theirs = theirs_fixture(&name);
        assert_eq!(
            &png_theirs[..SELECTORS.len()],
            &bare_theirs[..],
            "{name}: ExifTool's tags differ bare and in a PNG"
        );
        assert!(
            png_theirs[SELECTORS.len()..].iter().all(Option::is_none),
            "{name}: no region fields"
        );

        let bare = Container::parse(&tiff).unwrap_or_else(|e| panic!("{tiff_name}: {e:?}"));
        let bare_ours = oracle::read_ours(&bare, SELECTORS);
        let bytes = fixture(&name);
        let meta = png::read(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let wrapped_ours = oracle::read_ours_png(&meta, SELECTORS);
        assert_eq!(
            wrapped_ours, bare_ours,
            "{name}: our reading, bare vs in a PNG"
        );
        let has = |v: &[Option<String>]| v.iter().map(Option::is_some).collect::<Vec<_>>();
        assert_eq!(has(&wrapped_ours), has(&bare_ours), "{name}: set of tags");
        assert_eq!(
            has(&wrapped_ours),
            has(&bare_theirs),
            "{name}: set of tags vs ExifTool"
        );
        assert!(oracle::diff(&bare_ours, &bare_theirs, SELECTORS).is_empty());
        assert!(
            compare(&name, &ours_fixture(&bytes), &png_theirs).is_clean(),
            "{name}"
        );
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

/// Our PNG-layer issue kinds (the walk's and the extraction's), run-length
/// encoded, in report order.
fn our_png_issue_kinds(bytes: &[u8]) -> String {
    match png::read(bytes) {
        Ok(m) => run_length(
            &m.png_issues()
                .iter()
                .map(|i| issue_kind(i.error()))
                .collect::<Vec<_>>(),
        ),
        Err(e) => format!("fatal:{}", issue_kind(&e)),
    }
}

/// Our Exif block states, run-length encoded.
fn our_block_states(bytes: &[u8]) -> String {
    match png::read(bytes) {
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

// ── Live (skip loudly without ExifTool) ────────────────────────────────────

/// Run ExifTool on `bytes` under the name `name` (a `.png` temp file) with the
/// given selectors; returns (fields, stdout, recorded warnings).
fn live_run(
    name: &str,
    bytes: &[u8],
    selectors: &[&str],
) -> (Vec<Option<String>>, String, Vec<String>) {
    let path = tools::temp_file_with_ext(name, "png", bytes).expect("temp file");
    let run = tools::run_exiftool_full(&path, selectors);
    let _ = std::fs::remove_file(&path);
    let (stdout, stderr) = run.unwrap_or_else(|e| panic!("{name}: {e}"));
    let fields = tools::parse_fields(selectors, &stdout).unwrap_or_else(|e| panic!("{name}: {e}"));
    (
        fields,
        stdout,
        oracle::png_warning_lines(tools::warning_lines(&stderr, &path)),
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

        let name = format!("png-{tiff_name}");
        let (wrapped, _, _) = live_run(&name, &fixture(&name), &names);
        assert_eq!(wrapped, bare, "{name}: live ExifTool, bare vs in a PNG");
        compared += wrapped.iter().filter(|v| v.is_some()).count();
    }
    println!("{test}: {compared} ExifTool values identical bare and in a PNG");
    assert!(compared > 60, "only {compared} values");
}

#[test]
fn live_png_parity_on_fixtures() {
    let test = "live_png_parity_on_fixtures";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = oracle::png_names();
    let (mut compared, mut explained) = (0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    for (name, bytes) in oracle_png::fixtures() {
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
        oracle_png::fixtures().len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn live_png_parity_on_the_png_seeds() {
    let test = "live_png_parity_on_the_png_seeds";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = oracle::names(SELECTORS);
    let (mut compared, mut explained, mut run) = (0usize, 0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    let seeds = png_seed_files();
    for (seed, bytes) in &seeds {
        let name = format!("png-seed-{seed}");
        if oracle::not_run_divergence(&name).is_some() {
            println!("NOT RUN {name}: a known not-return input");
            continue;
        }
        // ExifTool is run on the committed file itself, alone, under the
        // wall-clock limit; a timeout is a failure, not a skip.
        let path = root().join("fuzz/seeds/png_chunks").join(seed);
        let (stdout, stderr) =
            tools::run_exiftool_full(&path, &names).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut theirs =
            tools::parse_fields(&names, &stdout).unwrap_or_else(|e| panic!("{name}: {e}"));
        compared += theirs.iter().filter(|t| t.is_some()).count();
        theirs.extend(oracle::PNG_REGION_SELECTORS.iter().map(|_| None));
        let cmp = compare(&name, &ours_seed(bytes), &theirs);
        for m in &cmp.unexplained {
            failures.push(format!("{name}: {m}"));
        }
        for s in &cmp.stale {
            failures.push(s.clone());
        }
        explained += cmp.explained.len();
        let rec = read_record(&name);
        let warnings = oracle::png_warning_lines(tools::warning_lines(&stderr, &path));
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
fn live_png_skip_is_loud_and_required_mode_fails() {
    for test in [
        "live_container_independence_against_exiftool",
        "live_png_parity_on_fixtures",
        "live_png_parity_on_the_png_seeds",
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
// SPEC-031 section 2 starts below.

/// A summary of the located regions of a PNG: `kind[compressed]` per region.
fn region_kinds(bytes: &[u8]) -> Vec<String> {
    let m = png::read(bytes).expect("a PNG");
    m.regions()
        .iter()
        .map(|r| match r {
            PngRegion::Xmp { compressed, .. } => format!("xmp(compressed={compressed})"),
            PngRegion::Icc { .. } => "icc".to_string(),
            PngRegion::RawProfile {
                profile,
                compressed,
                ..
            } => format!(
                "raw({},compressed={compressed})",
                String::from_utf8_lossy(profile)
            ),
            PngRegion::Text { compressed, .. } => format!("text(compressed={compressed})"),
            _ => "other".to_string(),
        })
        .collect()
}

/// The several-`eXIf` rule, measured (records `png-two-exif-*`,
/// `png-exif-good-then-fatal`, `png-exif-fatal-then-good`): ExifTool reads the
/// FIRST `eXIf` that is a TIFF and ignores the later ones, which is NOT the JPEG
/// rule (there the tags of all blocks merge and the later wins). The harness model
/// `oracle::read_ours_png` is that rule, and it equals the library's own `exif()`
/// (the first block that parsed), so no merged view is needed.
#[test]
#[allow(non_snake_case)] // the name is normative (SPEC-031)
fn several_eXIf_model_is_measured_and_pinned() {
    let make = index_of("IFD0:Make");
    let model = index_of("IFD0:Model");
    for (name, want_make, want_model) in [
        // A (Make AlphaCam, Model OnlyInA) then B (Make BravoCam): A only.
        ("png-two-exif-same-tag", "AlphaCam", Some("OnlyInA")),
        // B then A: B only, and B has no Model.
        ("png-two-exif-same-tag-reversed", "BravoCam", None),
        // Disjoint tags are NOT merged: the second block's Model is lost.
        ("png-two-exif-disjoint-tags", "AlphaCam", None),
        // A block that is not a TIFF is skipped, in either order.
        ("png-exif-good-then-fatal", "AlphaCam", Some("OnlyInA")),
        ("png-exif-fatal-then-good", "AlphaCam", Some("OnlyInA")),
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
        let meta = png::read(&bytes).expect("a PNG");
        assert_eq!(meta.exif_blocks().len(), 2, "{name}");
        let ours = ours_fixture(&bytes);
        assert_eq!(ours[make].as_deref(), Some(want_make), "{name}: our Make");
        assert_eq!(ours[model].as_deref(), want_model, "{name}: our Model");
        assert!(compare(name, &ours, &theirs).is_clean(), "{name}");
        // The model is the library's own primary block.
        let primary = oracle::read_ours(meta.exif().expect("a primary"), SELECTORS);
        assert_eq!(&primary[..], &ours[..SELECTORS.len()], "{name}: exif()");
        // The second block costs an ExtraExif issue.
        assert!(
            meta.png_issues()
                .iter()
                .any(|i| matches!(i.error(), Error::ExtraExif { .. })),
            "{name}"
        );
    }
    // The JPEG rule (the LAST block that has the tag wins, tags merge) would be
    // wrong for PNG: it names IFD0:Make on `png-two-exif-same-tag`.
    let bytes = fixture("png-two-exif-same-tag");
    let meta = png::read(&bytes).expect("a PNG");
    let jpeg_rule: Option<String> = meta.exif_blocks().iter().rev().find_map(|b| {
        b.container()
            .ok()
            .and_then(|c| oracle::read_one(c, &SELECTORS[2]))
    });
    let mut as_jpeg = ours_fixture(&bytes);
    as_jpeg[make] = jpeg_rule;
    let cmp = compare(
        "png-two-exif-same-tag",
        &as_jpeg,
        &theirs_fixture("png-two-exif-same-tag"),
    );
    assert_eq!(named(&cmp.unexplained), vec!["IFD0:Make"]);
    // And the JPEG merge rule would invent the Model of the second block on the
    // disjoint fixture (ExifTool loses it).
    let bytes = fixture("png-two-exif-disjoint-tags");
    let meta = png::read(&bytes).expect("a PNG");
    let merged: Option<String> = meta.exif_blocks().iter().rev().find_map(|b| {
        b.container()
            .ok()
            .and_then(|c| oracle::read_one(c, &SELECTORS[3]))
    });
    assert_eq!(merged.as_deref(), Some("BravoModel"));
    assert_eq!(ours_fixture(&bytes)[model], None);
}

/// The eXIf chunk and a raw-profile Exif together: ExifTool reads both and the
/// later (raw-profile) value wins; we read the eXIf only (a recorded
/// divergence, pinned in `raw_profile_and_compressed_xmp_...`). The several-block
/// model above is about `eXIf` chunks only.
#[test]
fn exif_and_raw_profile_exif_together_are_read_by_exiftool_only() {
    let name = "png-exif-and-raw-exif";
    let make = index_of("IFD0:Make");
    let theirs = theirs_fixture(name);
    assert_eq!(theirs[make].as_deref(), Some("RawCam"));
    let ours = ours_fixture(&fixture(name));
    assert_eq!(ours[make].as_deref(), Some("AlphaCam"));
    assert_eq!(region_kinds(&fixture(name)), ["raw(exif,compressed=false)"]);
}

#[test]
fn region_presence_xmp_and_icc_agree_with_exiftool() {
    let xmp = index_of("XMP-xmp:CreatorTool");
    let icc = index_of("PNG:ProfileName");
    // Present on both sides.
    for (name, idx) in [
        ("png-xmp-itxt", xmp),
        ("png-xmp-text", xmp),
        ("png-xmp-after-idat", xmp),
        ("png-iccp-valid", icc),
        ("png-iccp-bad-header", icc),
    ] {
        let theirs = theirs_fixture(name);
        let ours = ours_fixture(&fixture(name));
        assert!(theirs[idx].is_some(), "{name}: ExifTool");
        assert_eq!(ours[idx].as_deref(), Some(oracle::PRESENT), "{name}: ours");
        let cmp = compare(name, &ours, &theirs);
        assert!(
            cmp.is_clean() && cmp.explained.is_empty(),
            "{name}: {cmp:?}"
        );
    }
    // Absent on both sides.
    for (name, idx) in [
        ("png-exif-basic-le", xmp),
        ("png-exif-basic-le", icc),
        ("png-xmp-other-keyword", xmp),
        ("png-xmp-lowercase-keyword", xmp),
        ("png-xmp-itxt-flag-2", xmp),
        ("png-xmp-itxt", icc),
        ("png-iccp-valid", xmp),
    ] {
        let theirs = theirs_fixture(name);
        let ours = ours_fixture(&fixture(name));
        assert!(theirs[idx].is_none(), "{name}: ExifTool");
        assert!(ours[idx].is_none(), "{name}: ours");
    }
}

/// The XMP keyword rules agree: `XML:com.adobe.xmp` in `iTXt`, `tEXt` or `zTXt`
/// is XMP (located by us in all of them, readable by us only when not
/// compressed); another keyword, a lower-case keyword and an `iTXt` flag of 2 are
/// not XMP content for ExifTool, and ours is no readable XMP either.
#[test]
fn xmp_keyword_rules_agree() {
    let xmp = index_of("XMP-xmp:CreatorTool");
    for (name, theirs_reads, regions) in [
        ("png-xmp-itxt", true, "xmp(compressed=false)"),
        ("png-xmp-text", true, "xmp(compressed=false)"),
        ("png-xmp-ztxt", true, "xmp(compressed=true)"),
        ("png-xmp-itxt-compressed", true, "xmp(compressed=true)"),
        ("png-xmp-other-keyword", false, "text(compressed=false)"),
        ("png-xmp-lowercase-keyword", false, "text(compressed=false)"),
        ("png-xmp-itxt-flag-2", false, "xmp(compressed=true)"),
    ] {
        let theirs = theirs_fixture(name);
        assert_eq!(theirs[xmp].is_some(), theirs_reads, "{name}: ExifTool");
        assert_eq!(
            region_kinds(&fixture(name)),
            [regions],
            "{name}: our regions"
        );
        // Where the located region is uncompressed, the readable selector agrees.
        let ours = ours_fixture(&fixture(name));
        if regions == "xmp(compressed=false)" || !theirs_reads {
            assert_eq!(ours[xmp].is_some(), theirs_reads, "{name}: ours");
        } else {
            assert!(ours[xmp].is_none(), "{name}: located, not readable");
        }
    }
}

/// Every divergence of the PNG layer: ExifTool reads what we only locate
/// (DEC-018). Each is present now (the comparison explains it), our side is the
/// located region the DEC describes, and it is tracked by its question.
#[test]
fn raw_profile_and_compressed_xmp_are_recorded_divergences_and_still_present() {
    let xmp = index_of("XMP-xmp:CreatorTool");
    for (name, regions, question) in [
        (
            "png-raw-exif-text",
            "raw(exif,compressed=false)",
            "png-raw-profile-not-decoded",
        ),
        (
            "png-raw-exif-ztxt",
            "raw(exif,compressed=true)",
            "png-raw-profile-not-decoded",
        ),
        (
            "png-raw-exif-itxt",
            "raw(exif,compressed=true)",
            "png-raw-profile-not-decoded",
        ),
        (
            "png-raw-xmp-text",
            "raw(xmp,compressed=false)",
            "png-raw-profile-not-decoded",
        ),
        (
            "png-raw-xmp-ztxt",
            "raw(xmp,compressed=true)",
            "png-raw-profile-not-decoded",
        ),
        (
            "png-raw-xmp-itxt",
            "raw(xmp,compressed=true)",
            "png-raw-profile-not-decoded",
        ),
        (
            "png-xmp-itxt-compressed",
            "xmp(compressed=true)",
            "png-compressed-xmp-not-inflated",
        ),
        (
            "png-xmp-ztxt",
            "xmp(compressed=true)",
            "png-compressed-xmp-not-inflated",
        ),
        (
            "png-exif-and-raw-exif",
            "raw(exif,compressed=false)",
            "png-raw-profile-not-decoded",
        ),
    ] {
        let theirs = theirs_fixture(name);
        let ours = ours_fixture(&fixture(name));
        assert_eq!(region_kinds(&fixture(name)), [regions], "{name}");
        let cmp = compare(name, &ours, &theirs);
        assert!(cmp.is_clean(), "{name}: {cmp:?}");
        assert!(
            !cmp.explained.is_empty(),
            "{name}: the divergence must be present"
        );
        assert!(
            oracle::KNOWN_DIVERGENCES
                .iter()
                .any(|d| d.fixture == name && d.question_id == question),
            "{name}: entry for {question}"
        );
        if name.contains("raw-exif-") {
            // ExifTool reads every Exif value of the TIFF; ours reads none.
            assert!(
                ours[..SELECTORS.len()].iter().all(Option::is_none),
                "{name}"
            );
            let pin = pin_for(name).expect("a pin");
            assert_eq!(
                &theirs[..SELECTORS.len()],
                &pin[..],
                "{name}: pinned ExifTool reading"
            );
            assert!(cmp
                .explained
                .iter()
                .all(|m| m.ours.is_none() && m.theirs.is_some()));
        }
        if name.contains("xmp") && name != "png-exif-and-raw-exif" {
            assert!(theirs[xmp].is_some() && ours[xmp].is_none(), "{name}");
        }
    }
    // Controls: the same content, readable by us, agrees.
    for name in ["png-xmp-itxt", "png-xmp-text", "png-exif-basic-le"] {
        let cmp = compare(name, &ours_fixture(&fixture(name)), &theirs_fixture(name));
        assert!(
            cmp.is_clean() && cmp.explained.is_empty(),
            "{name}: {cmp:?}"
        );
    }
}

#[test]
fn exif_after_idat_and_after_iend_agree() {
    let make = index_of("IFD0:Make");
    for (name, after_idat, after_iend) in [
        ("png-exif-after-idat", true, false),
        ("png-exif-after-iend", true, true),
    ] {
        let theirs = theirs_fixture(name);
        let bytes = fixture(name);
        let ours = ours_fixture(&bytes);
        assert_eq!(
            theirs[make].as_deref(),
            Some("DiamondCam"),
            "{name}: ExifTool"
        );
        assert_eq!(ours[make].as_deref(), Some("DiamondCam"), "{name}: ours");
        let cmp = compare(name, &ours, &theirs);
        assert!(
            cmp.is_clean() && cmp.explained.is_empty(),
            "{name}: {cmp:?}"
        );
        let meta = png::read(&bytes).expect("a PNG");
        let b = &meta.exif_blocks()[0];
        assert_eq!(
            (b.after_idat(), b.after_iend()),
            (after_idat, after_iend),
            "{name}"
        );
    }
    // Control: the same TIFF before the image data.
    let before = theirs_fixture("png-exif-basic-le");
    assert_eq!(before[make].as_deref(), Some("DiamondCam"));
    // A chunk-order violation and a bad CRC are read by both sides.
    for name in [
        "png-exif-before-ihdr",
        "png-exif-bad-crc",
        "png-exif-prefix",
    ] {
        let cmp = compare(name, &ours_fixture(&fixture(name)), &theirs_fixture(name));
        assert!(
            cmp.is_clean() && cmp.explained.is_empty(),
            "{name}: {cmp:?}"
        );
        assert_eq!(
            theirs_fixture(name)[make].as_deref(),
            Some("DiamondCam"),
            "{name}"
        );
    }
}

// ── The warning table ──────────────────────────────────────────────────────

/// `(fixture, our PngIssue kinds, our Exif block states, the substrings of the
/// ExifTool warnings, empty when ExifTool is silent)`. ExifTool's warnings come
/// from the committed `png-warnings.txt` (a second run of the tool,
/// `-a -G1 -s -n -Warning -Error`, because a `-T -s3` run prints warnings to
/// stderr only when no requested tag was found).
type WarningRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
);

const WARNING_TABLE: &[WarningRow] = &[
    // Both are loud, in different words.
    (
        "png-exif-prefix",
        "ImproperExifHeader",
        "ok",
        &["Improper \"Exif00\" header in EXIF chunk"],
    ),
    ("png-no-iend", "MissingIend", "ok", &["Truncated PNG image"]),
    (
        "png-truncated-between-ihdr-and-idat",
        "MissingIend",
        "ok",
        &["Truncated PNG image"],
    ),
    (
        "png-length-past-eof",
        "Truncated",
        "",
        &["Corrupted PNG image"],
    ),
    (
        "png-length-80000000",
        "BadChunkLength",
        "",
        &["Invalid PNG chunk size"],
    ),
    (
        "png-no-ihdr",
        "MissingIhdr",
        "ok",
        &["PNG image did not start with IHDR"],
    ),
    (
        "png-exif-before-ihdr",
        "MissingIhdr",
        "ok",
        &["PNG image did not start with IHDR"],
    ),
    (
        "png-two-exif-same-tag",
        "ExtraExif",
        "ok*2",
        &["IFD0 pointer references previous IFD0 directory"],
    ),
    (
        "png-xmp-itxt-flag-2",
        "BadTextChunk",
        "",
        &["Error inflating XML:com.adobe.xmp"],
    ),
    // ExifTool is louder: it warns, we have no PNG-layer issue (the trailer and
    // the position after the image data are facts on the located item, not issues).
    (
        "png-trailer-bytes",
        "",
        "ok",
        &["Trailer data after PNG IEND chunk"],
    ),
    (
        "png-exif-after-iend",
        "",
        "ok",
        &["Trailer data after PNG IEND chunk", "found after PNG IDAT"],
    ),
    ("png-exif-after-idat", "", "ok", &["found after PNG IDAT"]),
    ("png-xmp-after-idat", "", "", &["found after PNG IDAT"]),
    ("png-exif-empty", "", "fatal", &["Invalid eXIf chunk"]),
    ("png-exif-not-tiff", "", "fatal", &["Invalid eXIf chunk"]),
    // ExifTool is silent: it does not check the CRC of the eXIf chunk nor the
    // keyword length, we report both.
    ("png-exif-bad-crc", "BadChunkCrc", "ok", &[]),
    ("png-keyword-80", "BadTextChunk", "", &[]),
    ("png-keyword-79", "", "", &[]),
    ("png-text-no-nul", "BadTextChunk", "", &[]),
    ("png-iccp-name-80", "BadIccpChunk", "", &[]),
    ("png-iccp-name-empty", "BadIccpChunk", "", &[]),
    // Clean on both sides.
    ("png-exif-basic-le", "", "ok", &[]),
    ("png-xmp-itxt", "", "", &[]),
    ("png-iccp-valid", "", "", &[]),
];

/// The committed warning record: `## name` sections of ExifTool's
/// `[group] Warning : text` lines.
fn recorded_warnings(section: &str) -> Vec<String> {
    let text = std::fs::read_to_string(record_path("png-warnings"))
        .unwrap_or_else(|e| panic!("cannot read png-warnings.txt ({e}); run `just oracle-record`"));
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
    let text = std::fs::read_to_string(record_path("png-warnings")).expect("png-warnings.txt");
    let sections: Vec<&str> = text.lines().filter_map(|l| l.strip_prefix("## ")).collect();
    let rows: Vec<&str> = WARNING_TABLE.iter().map(|r| r.0).collect();
    assert_eq!(
        sections, rows,
        "png-warnings.txt must hold the table's fixtures"
    );
    assert_eq!(
        rows,
        oracle_png::WARNING_FIXTURES,
        "the recorder's list is the table"
    );
    let header = tools::parse_record(&text).expect("header");
    assert_eq!(header.version, "13.55");

    for (name, ours_kinds, blocks, warnings) in WARNING_TABLE {
        let bytes = fixture(name);
        assert_eq!(
            &our_png_issue_kinds(&bytes),
            ours_kinds,
            "{name}: our PNG-layer issues"
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
    let png_entries: Vec<_> = oracle::KNOWN_DIVERGENCES
        .iter()
        .filter(|d| d.fixture.starts_with("png-"))
        .collect();
    for d in &png_entries {
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
            tools::exiftool_command_line(&oracle::png_names()),
            "{}: command",
            d.fixture
        );
    }
    assert_eq!(png_entries.len(), 9);
}

#[test]
fn a_divergence_that_stops_being_one_fails() {
    // Every divergence flips to a failure the moment the two sides agree.
    for d in oracle::KNOWN_DIVERGENCES
        .iter()
        .filter(|d| d.fixture.starts_with("png-"))
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
        let stranger = compare("png-exif-basic-le", &ours, &theirs);
        assert!(
            !stranger.unexplained.is_empty(),
            "{name}: not covered elsewhere"
        );
    }
    // The unread wildcard covers one direction only: a value of ours that
    // ExifTool does not print, or a different one, is not explained.
    let name = "png-raw-exif-text";
    let make = index_of("IFD0:Make");
    let theirs = theirs_fixture(name);
    let mut surplus = ours_fixture(name_bytes(name));
    surplus[make] = Some("Wrong".to_string());
    let cmp = compare(name, &surplus, &theirs);
    assert!(cmp.unexplained.iter().any(|m| m.selector == "IFD0:Make"));
    let mut blind = theirs.clone();
    blind[make] = None;
    let mut ours_with = ours_fixture(name_bytes(name));
    ours_with[make] = Some("DiamondCam".to_string());
    let cmp = compare(name, &ours_with, &blind);
    assert!(cmp.unexplained.iter().any(|m| m.selector == "IFD0:Make"));
    // The wildcard needs its pin: without it the fixture is not clean.
    let unpinned = oracle::compare_png(name, &ours_fixture(name_bytes(name)), &theirs, None);
    assert!(
        !unpinned.is_clean(),
        "an unread wildcard without its pin must fail"
    );
    // A pin that does not match ExifTool's reading fails too.
    let mut wrong_pin = pin_for(name).expect("pin");
    wrong_pin[make] = Some("Other".to_string());
    let bad = oracle::compare_png(
        name,
        &ours_fixture(name_bytes(name)),
        &theirs,
        Some(&wrong_pin),
    );
    assert!(bad.unexplained.iter().any(|m| m.selector == "IFD0:Make"));
    // A mismatch outside the Exif selectors is never covered by the wildcard.
    let xmp = index_of("XMP-xmp:CreatorTool");
    let mut ours_x = ours_fixture(name_bytes(name));
    ours_x[xmp] = Some(oracle::PRESENT.to_string());
    let cmp = compare(name, &ours_x, &theirs);
    assert!(cmp
        .unexplained
        .iter()
        .any(|m| m.selector == "XMP-xmp:CreatorTool"));
}

/// `fixture` as a borrowed slice that lives for the whole test run.
fn name_bytes(name: &str) -> &'static [u8] {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static ALL: OnceLock<HashMap<String, Vec<u8>>> = OnceLock::new();
    ALL.get_or_init(|| oracle_png::fixtures().into_iter().collect())
        .get(name)
        .unwrap_or_else(|| panic!("no PNG fixture named {name}"))
}

/// What we locate, as the comparable form of an `oracle_png::Expect`.
fn located(bytes: &[u8]) -> Vec<oracle_png::Expect> {
    let m = png::read(bytes).expect("a PNG");
    m.regions()
        .iter()
        .map(|r| match *r {
            PngRegion::Xmp {
                compressed, data, ..
            } => oracle_png::Expect {
                kind: "xmp",
                compressed,
                name: Vec::new(),
                data: data.to_vec(),
            },
            PngRegion::Icc {
                name, method, data, ..
            } => {
                assert_eq!(method, 0, "the builders write method 0");
                oracle_png::Expect {
                    kind: "icc",
                    // The builders' zlib stream is the compressed profile.
                    compressed: true,
                    name: name.to_vec(),
                    data: data.to_vec(),
                }
            }
            PngRegion::RawProfile {
                profile,
                compressed,
                data,
                ..
            } => oracle_png::Expect {
                kind: "raw",
                compressed,
                name: profile.to_vec(),
                data: data.to_vec(),
            },
            PngRegion::Text {
                keyword,
                compressed,
                data,
                ..
            } => oracle_png::Expect {
                kind: "text",
                compressed,
                name: keyword.to_vec(),
                data: data.to_vec(),
            },
            _ => panic!("an unknown region kind"),
        })
        .collect()
}

/// SPEC-031 verify M3: the comparator sees presence only, so a region we locate
/// with the wrong bytes would pass it. For every hand-built fixture the located
/// regions (kind, compression, name, and the exact bytes of the packet, profile
/// or text) equal what the independent builders wrote; a fixture with no
/// expectation locates nothing. No ExifTool needed.
#[test]
fn located_region_bytes_equal_what_the_builders_wrote() {
    let expected = oracle_png::expected_regions();
    let mut checked = 0usize;
    for (name, bytes) in oracle_png::fixtures() {
        let want: Vec<oracle_png::Expect> = expected
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, e)| e.clone())
            .unwrap_or_default();
        assert_eq!(located(&bytes), want, "{name}: located regions");
        checked += want.len();
    }
    for (n, _) in &expected {
        assert!(
            oracle_png::fixtures().iter().any(|(f, _)| f == n),
            "{n} is not a fixture"
        );
    }
    assert!(checked >= 25, "only {checked} regions checked");
}

/// Measured rules (verify M1 and M2), pinned beside the records: ExifTool reads
/// an `iCCP` whatever the length of its name (and prints no name when it is
/// empty), and treats an `iTXt` XMP keyword with a non-empty language tag as
/// plain text whatever the translated keyword.
#[test]
fn iccp_names_and_itxt_language_tags_agree_with_exiftool() {
    let icc = index_of("PNG:ProfileName");
    let xmp = index_of("XMP-xmp:CreatorTool");
    for (name, has_name) in [
        ("png-iccp-name-79", true),
        ("png-iccp-name-80", true),
        ("png-iccp-name-empty", false),
    ] {
        let theirs = theirs_fixture(name);
        let ours = ours_fixture(&fixture(name));
        assert_eq!(theirs[icc].is_some(), has_name, "{name}: ExifTool");
        assert_eq!(ours[icc].is_some(), has_name, "{name}: ours");
        // The chunk is located in every case.
        assert_eq!(located(&fixture(name)).len(), 1, "{name}");
    }
    for (name, is_xmp) in [
        ("png-xmp-itxt", true),
        ("png-xmp-itxt-translated", true),
        ("png-xmp-itxt-lang", false),
        ("png-xmp-itxt-lang-compressed", false),
        ("png-xmp-itxt-lang-translated", false),
    ] {
        let theirs = theirs_fixture(name);
        let ours = ours_fixture(&fixture(name));
        assert_eq!(theirs[xmp].is_some(), is_xmp, "{name}: ExifTool");
        assert_eq!(ours[xmp].is_some(), is_xmp, "{name}: ours");
        assert!(compare(name, &ours, &theirs).is_clean(), "{name}");
    }
}
