//! ExifTool read-parity oracle for the TIFF/EXIF container (SPEC-005).
//!
//! Live tests run the installed ExifTool (`-T -n -s3`) over hand-built fixtures
//! and the committed `tiff_ifd` seeds and compare field by field with what
//! `exif-core` reads. Replay tests run the SAME comparator against committed
//! ExifTool output (`tests/oracle-fixtures/exiftool/`) with no tool installed,
//! and prove the comparator can fail: perturb one value and exactly that field
//! is named. A live test with no ExifTool prints `SKIP` and passes, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`, in which case it fails.
//!
//! Provenance: replay-and-perturb shape from irradiance
//! `tests/metadata_oracle.rs:458-545` (edited port, DEC-003). ExifTool is run,
//! never read or linked.

#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/tools.rs"]
mod tools;

use exif_core::tiff::Container;
use oracle::{Mismatch, Norm, SEED_SELECTORS, SELECTORS};
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

fn seed_files() -> Vec<(String, Vec<u8>)> {
    let dir = root().join("fuzz/seeds/tiff_ifd");
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

/// Gate for live tests: `true` to proceed. No ExifTool: print a loud `SKIP` and
/// return `false`, or panic when `DIAMOND_EXIF_REQUIRE_ORACLE=1`.
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

/// Print the live version, and a loud NOTE when it differs from the committed
/// record headers (not a failure).
fn note_version(test: &str, recorded: &str) {
    let live = tools::exiftool_version().unwrap_or_else(|e| format!("unknown ({e})"));
    println!("{test}: live exiftool {live}");
    if live != recorded {
        println!("NOTE {test}: live exiftool {live} differs from recorded {recorded}");
    }
}

fn recorded_version() -> String {
    read_record("exif-basic-le").version
}

fn names_of(selectors: &[oracle::Selector]) -> Vec<&'static str> {
    oracle::names(selectors)
}

/// Compare one fixture's bytes against ExifTool's fields; reported
/// divergences are removed (and printed).
fn fixture_mismatches(name: &str, bytes: &[u8], theirs: &[Option<String>]) -> Vec<Mismatch> {
    let container = Container::parse(bytes).unwrap_or_else(|e| panic!("{name}: parse: {e:?}"));
    let ours = oracle::read_ours(&container, SELECTORS);
    oracle::diff(&ours, theirs, SELECTORS)
        .into_iter()
        .filter(|m| match oracle::known_divergence(name, m.selector) {
            Some(d) => {
                println!("KNOWN DIVERGENCE {name} {} ({})", m.selector, d.question_id);
                false
            }
            None => true,
        })
        .collect()
}

/// The seed rule: where ours is `Ok`, compare the three selectors; where ours
/// is `Err` (fatal), ExifTool's three fields must all be `None`.
fn seed_mismatches(
    name: &str,
    bytes: &[u8],
    theirs_stdout: &str,
) -> Result<Vec<Mismatch>, tools::ToolError> {
    let names = names_of(SEED_SELECTORS);
    let theirs = tools::parse_fields(&names, theirs_stdout)?;
    let found = match Container::parse(bytes) {
        Ok(c) => oracle::diff(
            &oracle::read_ours(&c, SEED_SELECTORS),
            &theirs,
            SEED_SELECTORS,
        ),
        Err(_) => SEED_SELECTORS
            .iter()
            .zip(&theirs)
            .filter(|(_, t)| t.is_some())
            .map(|(s, t)| Mismatch {
                selector: s.exiftool,
                ours: None,
                theirs: t.clone(),
            })
            .collect(),
    };
    Ok(found
        .into_iter()
        .filter(|m| match oracle::known_divergence(name, m.selector) {
            Some(d) => {
                println!("KNOWN DIVERGENCE {name} {} ({})", m.selector, d.question_id);
                false
            }
            None => true,
        })
        .collect())
}

/// A seed ExifTool is not run on (a reported `"*"` divergence): check that the
/// record says so and that our side still behaves as the divergence entry
/// states (parses, records an issue, reads `IFD0:ImageWidth = 1`), so the entry
/// cannot go stale unnoticed. Prints a loud `NOT RUN` line.
fn check_not_run_seed(name: &str, bytes: &[u8]) -> Result<(), String> {
    let d = oracle::not_run_divergence(name).ok_or("not a not-run seed")?;
    let rec = read_record(name);
    match rec.outcome.as_deref() {
        Some(o) if o == oracle::not_run_outcome(d) => {}
        other => return Err(format!("{name}: record outcome is {other:?}")),
    }
    let c = Container::parse(bytes).map_err(|e| format!("{name}: {e:?}"))?;
    if c.is_clean() {
        return Err(format!("{name}: expected an issue (the cycle), got none"));
    }
    let ours = oracle::read_ours(&c, SEED_SELECTORS);
    if ours.first().and_then(|o| o.as_deref()) != Some("1") {
        return Err(format!(
            "{name}: ours IFD0:ImageWidth is {ours:?}, entry says 1"
        ));
    }
    println!(
        "NOT RUN {name}: exiftool {} does not return on it (KNOWN DIVERGENCE {})",
        d.exiftool_version, d.question_id
    );
    Ok(())
}

// ── Fixtures ───────────────────────────────────────────────────────────────

#[test]
fn fixtures_are_deterministic_and_uniquely_named() {
    let a = oracle_tiff::fixtures();
    let b = oracle_tiff::fixtures();
    assert_eq!(a, b, "two calls must give identical bytes");
    let mut names: Vec<&str> = a.iter().map(|(n, _)| *n).collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "fixture names must be unique");
    // Record files are keyed by name, so names must also not collide with seeds.
    for (seed, _) in seed_files() {
        assert!(
            !names.contains(&seed.as_str()),
            "fixture name {seed} collides with a seed"
        );
    }
}

#[test]
fn every_fixture_parses_or_is_a_designed_failure() {
    for (name, bytes) in oracle_tiff::fixtures() {
        let parsed = Container::parse(&bytes);
        if oracle_tiff::DESIGNED_PARSE_FAILURES.contains(&name) {
            assert!(parsed.is_err(), "{name} is listed as a designed failure");
        } else {
            assert!(parsed.is_ok(), "{name}: {:?}", parsed.err());
        }
    }
}

#[test]
fn corrupt_fixtures_are_corrupt_in_the_designed_way() {
    // The two "corrupt" fixtures must really trip the tolerant walk, or the
    // parity claim over them proves nothing.
    for (name, bytes) in oracle_tiff::fixtures() {
        let c = Container::parse(&bytes).expect("parses");
        match name {
            "gps-corrupt-exif-ok" | "ifd1-corrupt" => {
                assert!(!c.is_clean(), "{name} must record an issue");
            }
            _ => assert!(c.is_clean(), "{name} must be clean: {:?}", c.issues()),
        }
    }
}

// ── Replay (no tool needed) ────────────────────────────────────────────────

#[test]
fn replay_is_clean_on_an_unmodified_reading() {
    let names = names_of(SELECTORS);
    for (name, bytes) in oracle_tiff::fixtures() {
        let rec = read_record(name);
        let theirs =
            tools::parse_fields(&names, &rec.stdout).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mismatches = fixture_mismatches(name, &bytes, &theirs);
        assert!(mismatches.is_empty(), "{name}: {mismatches:?}");
    }
    for (name, bytes) in seed_files() {
        if oracle::not_run_divergence(&name).is_some() {
            check_not_run_seed(&name, &bytes).unwrap_or_else(|e| panic!("{e}"));
            continue;
        }
        let rec = read_record(&name);
        let mismatches =
            seed_mismatches(&name, &bytes, &rec.stdout).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(mismatches.is_empty(), "{name}: {mismatches:?}");
    }
}

#[test]
fn replay_headers_record_command_and_version() {
    let fixture_cmd = tools::exiftool_command_line(&names_of(SELECTORS));
    let seed_cmd = tools::exiftool_command_line(&names_of(SEED_SELECTORS));
    let mut all: Vec<(String, String, String)> = oracle_tiff::fixtures()
        .into_iter()
        .map(|(n, _)| (n.to_string(), fixture_cmd.clone(), format!("fixture {n}")))
        .collect();
    all.extend(
        seed_files()
            .into_iter()
            .map(|(n, _)| (n.clone(), seed_cmd.clone(), format!("seed {n}"))),
    );
    for (name, cmd, source) in all {
        let rec = read_record(&name);
        assert_eq!(rec.command, cmd, "{name}: recorded command");
        assert_eq!(rec.source, source, "{name}: recorded source");
        assert!(
            !rec.version.is_empty() && rec.version.chars().all(|c| c.is_ascii_digit() || c == '.'),
            "{name}: version {:?}",
            rec.version
        );
    }
}

/// Replay-and-perturb: for every field ExifTool reported on a fixture, change
/// exactly that value on our side and require exactly one mismatch, naming it.
#[test]
fn oracle_names_the_one_field_that_was_perturbed() {
    let names = names_of(SELECTORS);
    let mut norms_seen: Vec<Norm> = Vec::new();
    for (name, bytes) in oracle_tiff::fixtures() {
        let theirs = tools::parse_fields(&names, &read_record(name).stdout).expect("record");
        let c = Container::parse(&bytes).expect("parses");
        let ours = oracle::read_ours(&c, SELECTORS);
        for (i, sel) in SELECTORS.iter().enumerate() {
            let Some(v) = ours.get(i).and_then(|o| o.clone()) else {
                continue;
            };
            let mut perturbed = ours.clone();
            if let Some(slot) = perturbed.get_mut(i) {
                *slot = Some(oracle::perturb(sel.norm, &v));
            }
            let got = oracle::diff(&perturbed, &theirs, SELECTORS);
            assert_eq!(got.len(), 1, "{name} {}: {got:?}", sel.exiftool);
            assert_eq!(got[0].selector, sel.exiftool, "{name}");
            let kind = std::mem::discriminant(&sel.norm);
            if !norms_seen.iter().any(|n| std::mem::discriminant(n) == kind) {
                norms_seen.push(sel.norm);
            }
        }
    }
    // Every Norm that has a live selector was red-proved against real records.
    // Sint and Float have one since SPEC-008 (the `signed-float-*` fixtures);
    // before that they were perturbed on synthetic readings only
    // (`oracle_names_a_perturbed_sint_and_float` below still does).
    for needed in [
        Norm::Ascii,
        Norm::Uint,
        Norm::Sint,
        Norm::Float,
        Norm::Rational,
        Norm::SRational,
        Norm::GpsCoord { ref_tag: 1 },
    ] {
        assert!(
            norms_seen
                .iter()
                .any(|n| std::mem::discriminant(n) == std::mem::discriminant(&needed)),
            "no real record exercised {needed:?}"
        );
    }
}

#[test]
fn oracle_names_a_perturbed_sint_and_float() {
    use exif_core::tiff::IfdKind;
    let sels = [
        oracle::Selector {
            exiftool: "Synthetic:Sint",
            kind: IfdKind::Ifd0,
            tag: 1,
            norm: Norm::Sint,
        },
        oracle::Selector {
            exiftool: "Synthetic:Float",
            kind: IfdKind::Ifd0,
            tag: 2,
            norm: Norm::Float,
        },
    ];
    let theirs = vec![Some("-7".to_string()), Some("0.25".to_string())];
    assert!(oracle::diff(&theirs, &theirs, &sels).is_empty());
    for i in 0..2 {
        let mut ours = theirs.clone();
        if let Some(slot) = ours.get_mut(i) {
            let v = slot.clone().unwrap_or_default();
            *slot = Some(oracle::perturb(sels[i].norm, &v));
        }
        let got = oracle::diff(&ours, &theirs, &sels);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].selector, sels[i].exiftool);
    }
}

#[test]
fn comparator_catches_a_missing_and_an_extra_field_on_a_real_record() {
    let names = names_of(SELECTORS);
    let (name, bytes) = oracle_tiff::fixtures()
        .into_iter()
        .next()
        .expect("at least one fixture");
    let theirs = tools::parse_fields(&names, &read_record(name).stdout).expect("record");
    let c = Container::parse(&bytes).expect("parses");
    let ours = oracle::read_ours(&c, SELECTORS);
    let make = SELECTORS
        .iter()
        .position(|s| s.exiftool == "IFD0:Make")
        .expect("Make selector");

    // Missing: we read nothing where ExifTool read a value.
    let mut missing = ours.clone();
    if let Some(slot) = missing.get_mut(make) {
        *slot = None;
    }
    let got = oracle::diff(&missing, &theirs, SELECTORS);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!((got[0].selector, got[0].ours.clone()), ("IFD0:Make", None));
    assert!(got[0].theirs.is_some());

    // Extra: we read a value where ExifTool read nothing.
    let mut extra_theirs = theirs.clone();
    if let Some(slot) = extra_theirs.get_mut(make) {
        *slot = None;
    }
    let got = oracle::diff(&ours, &extra_theirs, SELECTORS);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].selector, "IFD0:Make");
    assert!(got[0].ours.is_some() && got[0].theirs.is_none());
}

/// The signed-integer and float selectors and the values the `signed-float-*`
/// fixtures put in them. ExifTool 13.55 printed exactly these for the
/// committed records (SPEC-008 measurement).
const SIGNED_FLOAT_EXPECTED: &[(&str, &str)] = &[
    ("ExifIFD:ExposureTime", "0.125"),
    ("ExifIFD:ExposureProgram", "-2"),
    ("ExifIFD:FocalLength", "4.5"),
    ("ExifIFD:ExifImageWidth", "-5"),
];

fn signed_float_values(readings: &[Option<String>]) -> Vec<(&'static str, Option<String>)> {
    SIGNED_FLOAT_EXPECTED
        .iter()
        .map(|(name, _)| {
            let i = SELECTORS
                .iter()
                .position(|s| s.exiftool == *name)
                .unwrap_or_else(|| panic!("{name} is not in the selector table"));
            (*name, readings.get(i).cloned().flatten())
        })
        .collect()
}

/// Offline half of the signed/float case: the committed ExifTool record has a
/// value for each of the four selectors AND our reading agrees with it, so the
/// SSHORT-read-as-unsigned survivor of SPEC-005 has a selector that can see it.
#[test]
fn replay_signed_and_float_selectors_carry_values() {
    let names = names_of(SELECTORS);
    for fixture in ["signed-float-le", "signed-float-be"] {
        let theirs = tools::parse_fields(&names, &read_record(fixture).stdout).expect("record");
        let want: Vec<(&str, Option<String>)> = SIGNED_FLOAT_EXPECTED
            .iter()
            .map(|(n, v)| (*n, Some((*v).to_string())))
            .collect();
        assert_eq!(signed_float_values(&theirs), want, "{fixture}: record");

        let bytes = oracle_tiff::fixtures()
            .into_iter()
            .find(|(n, _)| *n == fixture)
            .map(|(_, b)| b)
            .expect("fixture");
        let c = Container::parse(&bytes).expect("parses");
        let ours = oracle::read_ours(&c, SELECTORS);
        assert_eq!(signed_float_values(&ours), want, "{fixture}: ours");
        assert!(oracle::diff(&ours, &theirs, SELECTORS).is_empty());
    }
}

// ── Live (skip loudly without ExifTool) ────────────────────────────────────

/// The live half: run the installed ExifTool on both signed/float fixtures and
/// require the four selectors to be present and equal to ours.
#[test]
fn live_signed_and_float_selectors_match_exiftool() {
    let test = "live_signed_and_float_selectors_match_exiftool";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = names_of(SELECTORS);
    for fixture in ["signed-float-le", "signed-float-be"] {
        let bytes = oracle_tiff::fixtures()
            .into_iter()
            .find(|(n, _)| *n == fixture)
            .map(|(_, b)| b)
            .expect("fixture");
        let path = tools::temp_file_for(fixture, &bytes).expect("temp file");
        let theirs = tools::run_exiftool(&path, &names);
        let _ = std::fs::remove_file(&path);
        let theirs = theirs.unwrap_or_else(|e| panic!("{fixture}: {e}"));
        let want: Vec<(&str, Option<String>)> = SIGNED_FLOAT_EXPECTED
            .iter()
            .map(|(n, v)| (*n, Some((*v).to_string())))
            .collect();
        assert_eq!(signed_float_values(&theirs), want, "{fixture}: live");
        let mismatches = fixture_mismatches(fixture, &bytes, &theirs);
        assert!(mismatches.is_empty(), "{fixture}: {mismatches:?}");
    }
}

#[test]
fn live_parity_on_hand_built_fixtures() {
    let test = "live_parity_on_hand_built_fixtures";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = names_of(SELECTORS);
    let mut compared = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (name, bytes) in oracle_tiff::fixtures() {
        let path = tools::temp_file_for(name, &bytes).expect("temp file");
        let theirs = tools::run_exiftool(&path, &names);
        let _ = std::fs::remove_file(&path);
        let theirs = theirs.unwrap_or_else(|e| panic!("{name}: {e}"));
        for m in fixture_mismatches(name, &bytes, &theirs) {
            failures.push(format!("{name}: {m}"));
        }
        compared += theirs.iter().filter(|t| t.is_some()).count();
    }
    println!(
        "{test}: {} fixtures, {compared} ExifTool values compared",
        oracle_tiff::fixtures().len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn live_parity_on_the_committed_seeds() {
    let test = "live_parity_on_the_committed_seeds";
    if !live_gate(test) {
        return;
    }
    note_version(test, &recorded_version());
    let names = names_of(SEED_SELECTORS);
    let (mut ours_ok, mut ours_err, mut values) = (0usize, 0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    let seeds = seed_files();
    let mut not_run = 0usize;
    for (name, bytes) in &seeds {
        if oracle::not_run_divergence(name).is_some() {
            match check_not_run_seed(name, bytes) {
                Ok(()) => not_run += 1,
                Err(e) => failures.push(e),
            }
            continue;
        }
        let path = root().join("fuzz/seeds/tiff_ifd").join(name);
        let stdout =
            tools::run_exiftool_raw(&path, &names).unwrap_or_else(|e| panic!("{name}: {e}"));
        match Container::parse(bytes) {
            Ok(_) => ours_ok += 1,
            Err(_) => ours_err += 1,
        }
        match seed_mismatches(name, bytes, &stdout) {
            Ok(found) => {
                values += tools::parse_fields(&names, &stdout)
                    .map(|v| v.iter().filter(|t| t.is_some()).count())
                    .unwrap_or(0);
                for m in found {
                    failures.push(format!("{name}: {m}"));
                }
            }
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }
    println!(
        "{test}: {} seeds, {} run live, {not_run} not run (known divergence); of the live ones \
         {ours_ok} parse Ok and {ours_err} are fatal; {values} ExifTool values compared",
        seeds.len(),
        seeds.len() - not_run
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The live red demonstration: change one byte of a fixture's ASCII payload in
/// a COPY, after our reading was taken from the original bytes, and require
/// the live comparison to go red naming exactly that field. Nothing is
/// committed; the copy lives in a temp file.
#[test]
fn live_demo_a_mutated_payload_is_caught() {
    let test = "live_demo_a_mutated_payload_is_caught";
    if !live_gate(test) {
        return;
    }
    let names = names_of(SELECTORS);
    let (name, original) = oracle_tiff::fixtures()
        .into_iter()
        .find(|(n, _)| *n == "exif-basic-le")
        .expect("exif-basic-le");
    // Our reading comes from the ORIGINAL bytes.
    let c = Container::parse(&original).expect("parses");
    let ours = oracle::read_ours(&c, SELECTORS);

    // Plant: first byte of the Make payload, 'D' -> 'X', in a copy.
    let needle = b"DiamondCam";
    let at = original
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("Make payload present");
    let mut mutated = original.clone();
    if let Some(b) = mutated.get_mut(at) {
        *b = b'X';
    }
    assert_ne!(mutated, original);
    let path = tools::temp_file_for("mutated-exif-basic-le", &mutated).expect("temp file");
    let theirs = tools::run_exiftool(&path, &names);
    let _ = std::fs::remove_file(&path);
    let theirs = theirs.expect("exiftool on the mutated copy");

    let got = oracle::diff(&ours, &theirs, SELECTORS);
    println!("{test}: {name} with Make[0] planted 'X' -> {got:?}");
    assert_eq!(
        got.len(),
        1,
        "exactly the planted field must go red: {got:?}"
    );
    assert_eq!(got[0].selector, "IFD0:Make");
    assert_eq!(got[0].ours.as_deref(), Some("DiamondCam"));
    assert_eq!(got[0].theirs.as_deref(), Some("XiamondCam"));
}

/// Runs the named live test in a child copy of this very test binary with
/// `PATH` pointing at nothing, so ExifTool cannot be found.
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
fn live_skip_is_loud_and_required_mode_fails() {
    let live_tests = [
        "live_parity_on_hand_built_fixtures",
        "live_parity_on_the_committed_seeds",
        "live_demo_a_mutated_payload_is_caught",
        "live_signed_and_float_selectors_match_exiftool",
    ];
    for test in live_tests {
        // Hidden ExifTool: a loud SKIP line, and the test passes.
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

        // Required mode: the same hidden ExifTool is a failure.
        let out = run_self_without_exiftool(test, true);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "{test}: {REQUIRE_VAR}=1 with no exiftool must fail\n{stdout}\n{stderr}"
        );
        // ...and for the right reason: the failure message names the variable.
        assert!(
            format!("{stdout}{stderr}").contains(&format!(
                "{test}: exiftool is not on PATH and {REQUIRE_VAR}=1"
            )),
            "{test}: failed, but not with the required-mode message\n{stdout}\n{stderr}"
        );
        assert!(
            !stdout.contains(&format!("SKIP {test}")),
            "{test}: must not skip"
        );
    }
}
