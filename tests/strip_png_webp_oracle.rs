//! SPEC-039, DEC-023: ExifTool write parity (I3) for the PNG and WebP strip.
//!
//! Two layers over committed, replayable records
//! (`tests/oracle-fixtures/exiftool-strip-{png,webp}/`, one file per fixture):
//!
//! * Layer A, the judge (`support/i3_judge.rs`): ExifTool's reading of our
//!   output against its reading of the input, by category, both ways, for
//!   every fixture under every recorded policy that strips.
//! * Layer B, the differential: `Everything` against `exiftool -all=` and
//!   `Gps` against `exiftool -gps:all=`, ExifTool's readings of the two
//!   outputs compared, every difference pinned (`DIFFERENCES`, `OUTCOMES`).
//!
//! Replay needs no ExifTool: it recomputes our `strip` and fails on a length
//! or digest mismatch, so a record never vouches for bytes the writer no
//! longer makes. The live tests re-run ExifTool and must reproduce each record
//! byte for byte. Without ExifTool they print `SKIP` and pass, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`. ExifTool is run, never read or linked.
//!
//! The comparators are shown red against the committed records with one line
//! edited in memory (`comparator_is_red_when_*`, no ExifTool needed).

#[path = "support/i3_judge.rs"]
mod i3_judge;
#[path = "support/i3_records.rs"]
mod i3_records;
#[path = "support/i3_shapes.rs"]
mod i3_shapes;
#[path = "support/oracle_png.rs"]
mod oracle_png;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/oracle_webp.rs"]
mod oracle_webp;
#[path = "support/tools.rs"]
mod tools;
#[path = "support/webp_carriers.rs"]
mod webp_carriers;
#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use i3_judge::{judge, Uses};
use i3_records::{Line, Record, Section, WriteResult};
use i3_shapes::Fixture;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

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

fn fixtures() -> &'static Vec<Fixture> {
    static F: OnceLock<Vec<Fixture>> = OnceLock::new();
    F.get_or_init(i3_shapes::fixtures)
}

fn fixture(name: &str) -> &'static Fixture {
    fixtures()
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no I3 fixture named {name}"))
}

fn is_core(name: &str) -> bool {
    fixtures()
        .iter()
        .position(|f| f.name == name)
        .is_some_and(|i| i < i3_shapes::ORACLE_COUNT)
}

fn record_path(fx: &Fixture) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(i3_records::record_dir(fx.ext))
        .join(format!("{}.txt", fx.name))
}

fn committed_text(fx: &Fixture) -> String {
    let path = record_path(fx);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (run `just oracle-record`)",
            path.display()
        )
    })
}

fn committed(fx: &Fixture) -> Record {
    i3_records::parse_record(&committed_text(fx)).unwrap_or_else(|e| panic!("{}: {e}", fx.name))
}

/// Fresh ExifTool records of every fixture, made once per process.
fn live_texts() -> &'static BTreeMap<String, String> {
    static LIVE: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    LIVE.get_or_init(|| {
        let version = tools::exiftool_version().expect("exiftool -ver");
        i3_records::record_all(fixtures(), &version, 8)
            .into_iter()
            .map(|(name, _, text)| (name, text))
            .collect()
    })
}

fn live(fx: &Fixture) -> Record {
    let text = live_texts().get(&fx.name).expect("live record");
    i3_records::parse_record(text).expect("live record parses")
}

/// The sections a record must have: the input, every recorded policy for which
/// the library strips, then ExifTool's two writes.
fn expected_sections(fx: &Fixture) -> Vec<String> {
    let mut want = vec!["input".to_string()];
    for (name, policy) in i3_records::policies() {
        if exif_core::strip(&fx.bytes, policy).is_ok() {
            want.push(name.to_string());
        }
    }
    want.extend(i3_records::WRITE_OPS.iter().map(|(n, _)| n.to_string()));
    want
}

// ── AC2: the records ────────────────────────────────────────────────────────

#[test]
fn the_i3_fixture_set_is_the_designed_one() {
    let f = fixtures();
    assert_eq!(f.len(), 238);
    assert_eq!(f.iter().filter(|x| x.ext == "png").count(), 91);
    assert_eq!(f.iter().filter(|x| x.ext == "webp").count(), 147);
    let names: BTreeSet<_> = f.iter().map(|x| x.name.to_ascii_lowercase()).collect();
    assert_eq!(names.len(), f.len(), "names are unique even case-folded");
    assert!(f[..i3_shapes::ORACLE_COUNT]
        .iter()
        .all(|x| !x.name.starts_with("png-carrier-")));
    // New shapes go at the end: the last two are the exception (c) fixtures.
    let last: Vec<&str> = f[236..].iter().map(|x| x.name.as_str()).collect();
    assert_eq!(last, ["png-xmp-plain-gps", "webp-xmp-plain-gps"]);
}

#[test]
fn every_i3_fixture_has_a_record_with_the_expected_sections() {
    for fx in fixtures() {
        let rec = committed(fx);
        assert_eq!(rec.fixture, fx.name);
        assert_eq!(rec.names(), expected_sections(fx), "{}", fx.name);
        for s in &rec.sections {
            let is_policy = i3_records::policies().iter().any(|(n, _)| *n == s.name);
            assert_eq!(s.output.is_some(), is_policy, "{} {}", fx.name, s.name);
            assert!(
                s.refused.is_none() || s.name.starts_with("exiftool:"),
                "{} {}",
                fx.name,
                s.name
            );
            assert!(
                s.refused.is_none() || s.lines.is_empty(),
                "{} {}: a refused write has no tag lines",
                fx.name,
                s.name
            );
        }
        // No machine path in a record (a temp dir, a home directory).
        let text = committed_text(fx);
        for needle in ["/var/", "/tmp/", "/Users/", "/private/", "diamond-exif-"] {
            assert!(!text.contains(needle), "{}: path text {needle}", fx.name);
        }
    }
}

/// `strip`'s output of `fx` under `policy` agrees with the record's length and
/// digest, or says why not (the fixture and the policy are named).
fn check_digest(fx: &Fixture, name: &str, section: &Section, bytes: &[u8]) -> Result<(), String> {
    let (len, digest) = section
        .output
        .ok_or_else(|| format!("{} under {name}: the record has no output line", fx.name))?;
    if bytes.len() != len || i3_records::fnv1a64(bytes) != digest {
        return Err(format!(
            "{} under {name}: the writer's output is {} bytes, digest {:016x}; the record read \
             {len} bytes, digest {digest:016x}",
            fx.name,
            bytes.len(),
            i3_records::fnv1a64(bytes)
        ));
    }
    Ok(())
}

#[test]
fn records_replay_and_bind_to_todays_output_bytes() {
    let mut checked = 0usize;
    for fx in fixtures() {
        let rec = committed(fx);
        for (name, policy) in i3_records::policies() {
            let Ok(out) = exif_core::strip(&fx.bytes, policy) else {
                continue;
            };
            let s = rec
                .section(name)
                .unwrap_or_else(|| panic!("{}: no {name} section", fx.name));
            if let Err(e) = check_digest(fx, name, s, out.bytes()) {
                panic!("{e}");
            }
            checked += 1;
        }
    }
    assert!(
        checked >= 2000,
        "only {checked} output digests were checked"
    );
}

#[test]
fn records_reproduce_live() {
    if !live_gate("records_reproduce_live") {
        return;
    }
    for fx in fixtures() {
        let have = committed_text(fx);
        let now = live_texts().get(&fx.name).expect("live record");
        assert_eq!(
            &have, now,
            "{}: re-record with `just oracle-record`",
            fx.name
        );
    }
}

// ── AC3: the refusal pins ───────────────────────────────────────────────────

fn actual_refusals() -> BTreeSet<(String, String, String)> {
    let mut out = BTreeSet::new();
    for fx in fixtures() {
        for (name, policy) in i3_records::policies() {
            if let Err(e) = exif_core::strip(&fx.bytes, policy) {
                out.insert((
                    fx.name.clone(),
                    name.to_string(),
                    i3_records::variant_name(&e),
                ));
            }
        }
    }
    out
}

fn pinned_refusals() -> BTreeSet<(String, String, String)> {
    let mut out = BTreeSet::new();
    for (fixture, variant, policies) in i3_records::REFUSALS {
        for p in *policies {
            assert!(
                out.insert((fixture.to_string(), p.to_string(), variant.to_string())),
                "{fixture} {p}: pinned twice"
            );
        }
    }
    out
}

#[test]
fn refusals_are_exactly_the_pinned_table() {
    let (actual, pinned) = (actual_refusals(), pinned_refusals());
    let unpinned: Vec<_> = actual.difference(&pinned).collect();
    let stale: Vec<_> = pinned.difference(&actual).collect();
    assert!(unpinned.is_empty(), "unpinned refusals: {unpinned:#?}");
    assert!(stale.is_empty(), "pinned, but now strips: {stale:#?}");
    // The design's measurement: of the five presets over the 174 oracle
    // fixtures, 13 fixtures are refused under all five and 16 under the three
    // that edit Exif entries.
    let presets = i3_judge::PRESET_NAMES;
    let mut by_fixture: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (f, p, _) in &actual {
        if is_core(f) && presets.contains(&p.as_str()) {
            by_fixture.entry(f).or_default().push(p);
        }
    }
    let all_five = by_fixture.values().filter(|p| p.len() == 5).count();
    let three = by_fixture
        .values()
        .filter(|p| {
            let mut s = (*p).clone();
            s.sort();
            s == ["all_pii", "gps", "gps_and_device"]
        })
        .count();
    assert_eq!((all_five, three), (13, 16));
    assert_eq!(
        by_fixture.values().map(Vec::len).sum::<usize>(),
        i3_records::DESIGN_REFUSED_PRESET_PAIRS
    );
}

#[test]
fn png_plte_is_structure_and_has_no_chunk_class() {
    // DEC-020 amendment 4: `PLTE` is never located or removed.
    assert_eq!(exif_core::png::chunk_class(*b"PLTE"), None);
}

// ── AC4: the judge ──────────────────────────────────────────────────────────

#[derive(Default)]
struct Totals {
    uses: Uses,
    sections: usize,
    removable_sections: usize,
    removable_tags: usize,
    failures: Vec<String>,
}

fn judge_record(fx: &Fixture, rec: &Record, totals: &mut Totals) {
    let input = rec.section("input").expect("input section");
    for (name, _) in i3_records::policies() {
        let Some(s) = rec.section(name) else {
            continue;
        };
        let v = judge(&fx.name, fx.ext, &fx.bytes, name, &input.lines, &s.lines);
        totals.sections += 1;
        totals.uses += v.uses;
        if v.removable_before > 0 {
            totals.removable_sections += 1;
            totals.removable_tags += v.removable_before;
        }
        totals.failures.extend(v.failures);
    }
}

fn judge_all(source: &dyn Fn(&Fixture) -> Record) -> Totals {
    let mut totals = Totals::default();
    for fx in fixtures() {
        judge_record(fx, &source(fx), &mut totals);
    }
    totals
}

fn report(t: &Totals, label: &str) {
    println!(
        "{label}: {} sections judged, {} with a removable tag ({} tags); exception (a) carrier \
         used {} times, (b) whole block {} times, (c) residual {} times",
        t.sections,
        t.removable_sections,
        t.removable_tags,
        t.uses.carrier,
        t.uses.whole_block,
        t.uses.residual
    );
}

fn assert_judge_holds(t: &Totals) {
    assert!(
        t.failures.is_empty(),
        "{} judge failures: {:#?}",
        t.failures.len(),
        t.failures
    );
    // Not vacuous: lower bounds, measured at build (the handback has the
    // actual numbers).
    assert!(t.sections >= 2000, "{} sections", t.sections);
    assert!(t.removable_sections >= 550, "{}", t.removable_sections);
    assert!(t.removable_tags >= 2000, "{}", t.removable_tags);
    assert!(t.uses.carrier >= 600, "carrier used {}", t.uses.carrier);
    assert!(
        t.uses.whole_block >= 300,
        "whole block {}",
        t.uses.whole_block
    );
    // Exception (c), the residual, is reached by the two plain-XMP-with-GPS
    // fixtures at the end of `i3_shapes` (SPEC-039 Part B).
    assert!(t.uses.residual >= 20, "residual used {}", t.uses.residual);
}

#[test]
fn the_judge_holds_both_ways_for_every_fixture_and_policy() {
    let t = judge_all(&committed);
    report(&t, "replay");
    assert_judge_holds(&t);
    if live_gate("the_judge_holds_both_ways_for_every_fixture_and_policy (live)") {
        let t = judge_all(&live);
        report(&t, "live");
        assert_judge_holds(&t);
    }
}

// ── AC5: the differential ───────────────────────────────────────────────────

/// `(our policy, ExifTool's section)`, by the pair's short name.
const PAIRS: [(&str, &str, &str); 2] = [
    ("all", "everything", "exiftool:all"),
    ("gps", "gps", "exiftool:gps:all"),
];

fn counts(lines: &[Line], values: bool) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for l in lines {
        let k = if values {
            format!("{}:{}={}", l.group, l.tag, l.value)
        } else {
            l.key()
        };
        *m.entry(k).or_insert(0) += 1;
    }
    m
}

fn minus(a: &BTreeMap<String, usize>, b: &BTreeMap<String, usize>) -> Vec<String> {
    let mut out = Vec::new();
    for (k, n) in a {
        for _ in b.get(k).copied().unwrap_or(0)..*n {
            out.push(k.clone());
        }
    }
    out
}

/// One pair's result in the form the pin tables use: `None` when the two
/// agree (or both refuse, counted by the caller), else a canonical string.
#[derive(Debug, PartialEq, Eq)]
enum Pair {
    Both,
    Same,
    Finding(String),
}

fn compare(
    fixture: &str,
    rec: &Record,
    ours: &str,
    theirs: &str,
    variant: &dyn Fn() -> String,
) -> Pair {
    let t = rec
        .section(theirs)
        .unwrap_or_else(|| panic!("{fixture}: no {theirs} section"));
    let o = rec.section(ours);
    match (o, &t.refused) {
        (None, Some(_)) => Pair::Both,
        (None, None) => Pair::Finding(format!("we-refuse|{}", variant())),
        (Some(_), Some(line)) => Pair::Finding(format!("exiftool-refuses|{line}")),
        (Some(o), None) => {
            let (ok, tk) = (counts(&o.lines, false), counts(&t.lines, false));
            if ok != tk {
                return Pair::Finding(format!("keys|{:?}|{:?}", minus(&ok, &tk), minus(&tk, &ok)));
            }
            let (ov, tv) = (counts(&o.lines, true), counts(&t.lines, true));
            if ov != tv {
                return Pair::Finding(format!(
                    "values|{:?}|{:?}",
                    minus(&ov, &tv),
                    minus(&tv, &ov)
                ));
            }
            Pair::Same
        }
    }
}

/// The pinned findings, by `(fixture, pair)`.
fn pinned_findings() -> BTreeMap<(String, String), String> {
    let mut m = BTreeMap::new();
    for d in i3_records::DIFFERENCES {
        for f in d.fixtures {
            let canon = format!("{}|{:?}|{:?}", d.level, d.ours_only, d.theirs_only);
            assert!(
                m.insert((f.to_string(), d.pair.to_string()), canon)
                    .is_none(),
                "{f} {}: pinned twice",
                d.pair
            );
        }
    }
    for (f, pair, who, detail) in i3_records::OUTCOMES {
        let canon = format!("{who}|{detail}");
        assert!(
            m.insert((f.to_string(), pair.to_string()), canon).is_none(),
            "{f} {pair}: pinned twice"
        );
    }
    m
}

/// Every difference of `fx`'s record: the findings not pinned, the pins of this
/// fixture that no longer occur, and how many pairs agree / both refuse.
struct Diffed {
    unpinned: Vec<String>,
    stale: Vec<String>,
    same: usize,
    both: usize,
}

fn diff_fixture(fx: &Fixture, rec: &Record, pins: &BTreeMap<(String, String), String>) -> Diffed {
    let mut d = Diffed {
        unpinned: Vec::new(),
        stale: Vec::new(),
        same: 0,
        both: 0,
    };
    for (pair, ours, theirs) in PAIRS {
        let variant = || {
            let policy = i3_records::policies()
                .into_iter()
                .find(|(n, _)| *n == ours)
                .map(|(_, p)| p)
                .expect("policy");
            match exif_core::strip(&fx.bytes, policy) {
                Err(e) => i3_records::variant_name(&e),
                Ok(_) => "it strips".to_string(),
            }
        };
        let key = (fx.name.clone(), pair.to_string());
        let pinned = pins.get(&key);
        match (compare(&fx.name, rec, ours, theirs, &variant), pinned) {
            (Pair::Both, None) => d.both += 1,
            (Pair::Same, None) => d.same += 1,
            (Pair::Finding(f), Some(p)) if &f == p => {}
            (Pair::Finding(f), p) => d.unpinned.push(format!(
                "{} {pair} ({ours} against {theirs}): unpinned difference {f} (pinned: {p:?})",
                fx.name
            )),
            (r, Some(p)) => d.stale.push(format!(
                "{} {pair} ({ours} against {theirs}): stale pin {p} (the pair now gives {r:?})",
                fx.name
            )),
        }
    }
    d
}

fn differential(source: &dyn Fn(&Fixture) -> Record) {
    let pins = pinned_findings();
    let (mut unpinned, mut stale) = (Vec::new(), Vec::new());
    let (mut same, mut both) = (0usize, 0usize);
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for fx in fixtures() {
        let d = diff_fixture(fx, &source(fx), &pins);
        unpinned.extend(d.unpinned);
        stale.extend(d.stale);
        if is_core(&fx.name) {
            same += d.same;
            both += d.both;
        }
        for (pair, _, _) in PAIRS {
            seen.insert((fx.name.clone(), pair.to_string()));
        }
    }
    // A pin for a fixture that does not exist is stale too.
    for k in pins.keys() {
        assert!(seen.contains(k), "{k:?}: pinned for no fixture");
    }
    assert!(unpinned.is_empty(), "{unpinned:#?}");
    assert!(stale.is_empty(), "{stale:#?}");

    // The design's counts, over the 174 oracle fixtures.
    let core_pairs = i3_shapes::ORACLE_COUNT * 2;
    assert_eq!(core_pairs, i3_records::DESIGN_PAIRS);
    let mut by_class: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_value_class: BTreeMap<&str, usize> = BTreeMap::new();
    let mut outcomes = 0usize;
    for d in i3_records::DIFFERENCES {
        let n = d.fixtures.iter().filter(|f| is_core(f)).count();
        if d.level == "keys" {
            *by_class.entry(d.class).or_insert(0) += n;
        } else {
            *by_value_class.entry(d.class).or_insert(0) += n;
        }
    }
    let value_level: usize = by_value_class.values().sum();
    for (f, _, _, _) in i3_records::OUTCOMES {
        if is_core(f) {
            outcomes += 1;
        }
    }
    for (class, n) in i3_records::DESIGN_KEY_COUNTS {
        assert_eq!(
            by_class.get(class).copied().unwrap_or(0),
            n,
            "class {class}"
        );
    }
    assert_eq!(outcomes, i3_records::DESIGN_OUTCOME_DIFFERENCES);
    assert_eq!(both, i3_records::DESIGN_BOTH_REFUSE);
    for (class, n) in i3_records::DESIGN_VALUE_COUNTS {
        assert_eq!(
            by_value_class.get(class).copied().unwrap_or(0),
            n,
            "value-level class {class}"
        );
    }
    assert_eq!(by_value_class.len(), i3_records::DESIGN_VALUE_COUNTS.len());
    // The design first measured 191 identical comparing `(group, tag)` keys
    // only; 34 of those differ in value (DEC-023 rule 4, amended 2026-11-25).
    assert_eq!(same, i3_records::DESIGN_IDENTICAL);
    assert_eq!(same + value_level, 191);
    println!(
        "differential: {same} identical pairs, {value_level} equal-key pairs whose values \
         differ, key-level differences {by_class:?}, value-level {by_value_class:?}, {outcomes} outcome differences, {both} \
         pairs both tools refuse (the 174 oracle fixtures)"
    );
}

#[test]
fn ours_and_exiftools_writer_agree_except_the_pinned_differences() {
    differential(&committed);
    if live_gate("ours_and_exiftools_writer_agree_except_the_pinned_differences (live)") {
        differential(&live);
    }
}

// ── AC6: the write-mode exit codes ──────────────────────────────────────────

#[test]
fn write_runs_classify_by_exit_code_and_output_file() {
    let out = Path::new("<out>");
    assert_eq!(
        i3_records::classify_write(Some(0), true, "Warning: x", out),
        WriteResult::Written(out.to_path_buf())
    );
    assert_eq!(
        i3_records::classify_write(
            Some(1),
            false,
            "Warning: w\nError: Bad CRC - <file>\nError: second",
            out
        ),
        WriteResult::Refused("Error: Bad CRC - <file>".to_string())
    );
    for (code, exists) in [
        (Some(0), false),
        (Some(1), true),
        (Some(2), false),
        (Some(2), true),
        (None, false),
    ] {
        assert!(
            matches!(
                i3_records::classify_write(code, exists, "", out),
                WriteResult::ToolError(_)
            ),
            "exit {code:?}, file exists {exists}"
        );
    }
}

#[test]
fn a_live_write_to_an_existing_path_is_a_tool_error_and_never_passes_m() {
    // The runner always uses a fresh path; the rule would catch a stale one
    // (exit 1 with the file present is not `Refused`). `-m` is not in the
    // source of the runner: the recorded command line says so.
    assert!(!i3_records::writes_line().contains(" -m"));
    for (_, op) in i3_records::WRITE_OPS {
        assert!(!op.contains('m') || op.starts_with("-gps"), "{op}");
    }
    if !live_gate("a_live_write_to_an_existing_path_is_a_tool_error_and_never_passes_m") {
        return;
    }
    let fx = fixture("png-exif-basic-le");
    let input = tools::temp_file_with_ext("i3-exists-in", "png", &fx.bytes).expect("temp");
    let existing = tools::temp_file_with_ext("i3-exists-out", "png", b"x").expect("temp");
    let args = vec![
        "-all=".to_string(),
        "-o".to_string(),
        existing.display().to_string(),
    ];
    let run = tools::run_tool("exiftool", &args, &input).expect("run");
    let _ = std::fs::remove_file(&input);
    let verdict = i3_records::classify_write(
        run.status.code(),
        existing.exists(),
        &String::from_utf8_lossy(&run.stderr),
        &existing,
    );
    let _ = std::fs::remove_file(&existing);
    assert!(
        matches!(verdict, WriteResult::ToolError(_)),
        "an existing output path must not look like a refusal: {verdict:?}"
    );
}

// ── AC7: the comparators are red, against the committed records ─────────────

fn line(group: &str, tag: &str, value: &str) -> Line {
    Line {
        group: group.into(),
        tag: tag.into(),
        value: value.into(),
    }
}

/// Judge `section` of `fixture`'s committed record, with `plant` applied to a
/// copy of that section first. Returns the failures.
fn judged_with(
    fixture: &str,
    policy: &str,
    plant: &dyn Fn(&mut Vec<Line>, &[Line]),
) -> Vec<String> {
    judged_as(fixture, policy, policy, &fixture_bytes(fixture), plant)
}

fn fixture_bytes(name: &str) -> Vec<u8> {
    fixture(name).bytes.clone()
}

fn judged_as(
    fixture_name: &str,
    section: &str,
    policy: &str,
    bytes: &[u8],
    plant: &dyn Fn(&mut Vec<Line>, &[Line]),
) -> Vec<String> {
    let fx = fixture(fixture_name);
    let rec = committed(fx);
    let input = rec.section("input").expect("input");
    let mut out = rec.section(section).expect("section").lines.clone();
    plant(&mut out, &input.lines);
    judge(fixture_name, fx.ext, bytes, policy, &input.lines, &out).failures
}

fn contains_all(failures: &[String], needles: &[&str]) -> bool {
    failures
        .iter()
        .any(|f| needles.iter().all(|n| f.contains(n)))
}

const GPS_FIXTURE: &str = "png-exif-gps-full-le";

#[test]
fn comparator_is_red_when_a_removed_tag_is_left_in() {
    // Control: the genuine record passes.
    assert!(judged_with(GPS_FIXTURE, "gps", &|_, _| {}).is_empty());
    let f = judged_with(GPS_FIXTURE, "gps", &|out, input| {
        let gps = input.iter().find(|l| l.group == "GPS").expect("a GPS tag");
        out.push(gps.clone());
    });
    assert!(
        contains_all(
            &f,
            &[GPS_FIXTURE, "under gps", "removed tag left in", "GPS:GPS"]
        ),
        "{f:?}"
    );
    println!("AC7 (i) red: {}", f.first().cloned().unwrap_or_default());
}

#[test]
fn comparator_is_red_when_a_kept_tag_is_missing() {
    let f = judged_with(GPS_FIXTURE, "gps", &|out, _| {
        let at = out.iter().position(|l| l.tag == "Make").expect("Make kept");
        out.remove(at);
    });
    assert!(
        contains_all(
            &f,
            &[GPS_FIXTURE, "under gps", "kept tag missing", "IFD0:Make"]
        ),
        "{f:?}"
    );
    println!("AC7 (ii) red: {}", f.first().cloned().unwrap_or_default());
}

#[test]
fn comparator_is_red_when_a_kept_value_changed() {
    let f = judged_with(GPS_FIXTURE, "gps", &|out, _| {
        let l = out.iter_mut().find(|l| l.tag == "Make").expect("Make kept");
        l.value = "CANARY-MAKE".to_string();
    });
    assert!(
        contains_all(
            &f,
            &[GPS_FIXTURE, "under gps", "kept value changed", "IFD0:Make"]
        ),
        "{f:?}"
    );
    println!(
        "AC7 (iii) red: {}",
        f.iter()
            .find(|m| m.contains("value changed"))
            .cloned()
            .unwrap_or_default()
    );
}

#[test]
fn comparator_is_red_when_a_phantom_tag_is_added() {
    let f = judged_with(GPS_FIXTURE, "gps", &|out, _| {
        out.push(line("IFD0", "PhantomTag", "1"));
    });
    assert!(
        contains_all(
            &f,
            &[GPS_FIXTURE, "under gps", "phantom tag", "IFD0:PhantomTag"]
        ),
        "{f:?}"
    );
    println!("AC7 (iv) red: {}", f.first().cloned().unwrap_or_default());
}

#[test]
fn comparator_is_red_when_the_carrier_exception_is_claimed_without_a_carrier() {
    let plant = |out: &mut Vec<Line>, _: &[Line]| {
        let at = out.iter().position(|l| l.tag == "Make").expect("Make kept");
        out.remove(at);
    };
    // Control: the same missing tag is excused on a file that holds a carrier
    // (the exception exists), so the red below is the exception's guard.
    let carrier = fixture_bytes("png-carrier-zxIf");
    let excused = judged_as(GPS_FIXTURE, "gps", "gps", &carrier, &plant);
    assert!(
        !excused.iter().any(|f| f.contains("kept tag missing")),
        "the carrier rule excuses a missing Exif tag on a carrier file: {excused:?}"
    );
    let f = judged_with(GPS_FIXTURE, "gps", &plant);
    assert!(
        contains_all(
            &f,
            &[GPS_FIXTURE, "under gps", "kept tag missing", "IFD0:Make"]
        ),
        "{f:?}"
    );
    println!("AC7 (v) red: {}", f.first().cloned().unwrap_or_default());
}

#[test]
fn comparator_is_red_when_exception_c_is_claimed_without_an_xmp_source() {
    // Control: under `xmp`, the plain XMP packet's `Composite:GPS*` go with it
    // and the judge excuses that (exception (c)) ...
    assert!(judged_with("png-xmp-plain-gps", "xmp", &|_, _| {}).is_empty());
    // ... but a file whose GPS comes from an Exif block has no XMP source: the
    // same loss of `Composite:GPSLatitude` under `xmp` is a failure.
    let f = judged_with(GPS_FIXTURE, "xmp", &|out, _| {
        let at = out
            .iter()
            .position(|l| l.group == "Composite" && l.tag == "GPSLatitude")
            .expect("a Composite GPS tag kept under xmp");
        out.remove(at);
    });
    assert!(
        contains_all(
            &f,
            &[
                GPS_FIXTURE,
                "under xmp",
                "kept tag missing",
                "Composite:GPSLatitude"
            ]
        ),
        "{f:?}"
    );
    println!("AC7 (x) red: {}", f.first().cloned().unwrap_or_default());
}

#[test]
fn comparator_is_red_when_the_whole_block_exception_is_claimed_under_all_pii() {
    // Control: the output of `all` (the whole `eXIf` gone) judged as `all`.
    assert!(judged_as(
        GPS_FIXTURE,
        "all",
        "all",
        &fixture_bytes(GPS_FIXTURE),
        &|_, _| {}
    )
    .is_empty());
    // The same output judged as `all_pii`, which keeps the uncategorised Exif
    // tags: the whole-block exception does not apply, and the tags are missing.
    let f = judged_as(
        GPS_FIXTURE,
        "all",
        "all_pii",
        &fixture_bytes(GPS_FIXTURE),
        &|_, _| {},
    );
    assert!(
        contains_all(
            &f,
            &[GPS_FIXTURE, "under all_pii", "kept tag missing", "IFD0:"]
        ),
        "{f:?}"
    );
    println!("AC7 (vi) red: {}", f.first().cloned().unwrap_or_default());
}

/// The differential on one fixture's committed record, with the ExifTool
/// section edited by `plant`.
fn diff_with(name: &str, plant: &dyn Fn(&mut Record)) -> Diffed {
    let fx = fixture(name);
    let mut rec = committed(fx);
    plant(&mut rec);
    diff_fixture(fx, &rec, &pinned_findings())
}

#[test]
fn comparator_is_red_when_the_differential_meets_an_unpinned_difference() {
    // Control: the genuine record is fully pinned.
    let c = diff_with("png-exif-basic-le", &|_| {});
    assert!(c.unpinned.is_empty() && c.stale.is_empty());
    let d = diff_with("png-exif-basic-le", &|rec| {
        let s = rec
            .sections
            .iter_mut()
            .find(|s| s.name == "exiftool:gps:all")
            .expect("section");
        s.lines.push(line("IFD0", "PhantomTag", "1"));
    });
    assert!(
        d.unpinned.iter().any(|m| m.contains("png-exif-basic-le")
            && m.contains("gps")
            && m.contains("unpinned difference")
            && m.contains("IFD0:PhantomTag")),
        "{:?}",
        d.unpinned
    );
    println!(
        "AC7 (vii) red: {}",
        d.unpinned.first().cloned().unwrap_or_default()
    );
}

#[test]
fn comparator_is_red_when_the_differential_meets_a_stale_pin() {
    // `png-text-no-nul` under `Everything` is the pinned D5: ExifTool keeps
    // `PNG:TextualData`. Make ExifTool's section agree with ours and the pin
    // is stale.
    let c = diff_with("png-text-no-nul", &|_| {});
    assert!(c.unpinned.is_empty() && c.stale.is_empty());
    let d = diff_with("png-text-no-nul", &|rec| {
        let ours = rec.section("everything").expect("ours").lines.clone();
        let s = rec
            .sections
            .iter_mut()
            .find(|s| s.name == "exiftool:all")
            .expect("section");
        s.lines = ours;
    });
    assert!(
        d.stale
            .iter()
            .any(|m| m.contains("png-text-no-nul") && m.contains("all") && m.contains("stale pin")),
        "{:?}",
        d.stale
    );
    println!(
        "AC7 (viii) red: {}",
        d.stale.first().cloned().unwrap_or_default()
    );
}

#[test]
fn comparator_is_red_when_one_output_byte_is_flipped_against_the_digest() {
    let fx = fixture(GPS_FIXTURE);
    let rec = committed(fx);
    let policy = i3_records::policies()
        .into_iter()
        .find(|(n, _)| *n == "gps")
        .map(|(_, p)| p)
        .expect("gps");
    let out = exif_core::strip(&fx.bytes, policy).expect("strips");
    let section = rec.section("gps").expect("section");
    // Control: the genuine bytes match.
    check_digest(fx, "gps", section, out.bytes()).expect("control");
    let mut bytes = out.bytes().to_vec();
    let at = bytes.len() / 2;
    bytes[at] ^= 0x01;
    let e = check_digest(fx, "gps", section, &bytes).unwrap_err();
    assert!(e.contains(GPS_FIXTURE) && e.contains("under gps"), "{e}");
    println!(
        "AC7 (ix) red: {}",
        [e.clone()].first().cloned().unwrap_or_default()
    );
}
