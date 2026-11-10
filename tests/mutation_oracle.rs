//! Red-proofs for the ExifTool comparison (SPEC-008): a FAULT in the library, or
//! a corruption of the input, must turn the comparison with ExifTool's answers
//! red, naming the selector it should.
//!
//! Library faults (tests 1-6) run on a mutated COPY of `src/`, staged and built
//! by `tests/support/probe.rs`. Each test has the same shape, which is the
//! `oracle-must-be-shown-red` bar: a CONTROL first (the same apparatus on the
//! unmutated copy must equal ExifTool's committed record, zero mismatches), then
//! the mutant, which must produce mismatches naming exactly the expected
//! selector(s). A mutation is one expression, marked `RED-PROOF INJECTION`, and
//! located by a needle that must occur exactly once in the staged file, so a
//! moved injection point fails loudly instead of turning a proof vacuous.
//!
//! Everything compares against the COMMITTED records in
//! `tests/oracle-fixtures/exiftool/`, so the red-proofs run with no ExifTool
//! installed. Only the live variant of the GPS-corruption test needs the tool
//! and skips loudly without it (`DIAMOND_EXIF_REQUIRE_ORACLE=1` makes that a
//! failure). Library code under `src/` is never modified: faults go into the
//! throw-away copy only.
//!
//! SPEC-011 points the same apparatus at JPEG: the probe binary reads a file that
//! starts `FF D8` through `jpeg::read` and the harness's several-Exif model, the
//! mutants m1-m7 fault the JPEG layer (`src/jpeg/`), and two byte mutations
//! corrupt an offset inside the APP1 payload and truncate between APP1 and SOS.
//!
//! The probe builds are slower than ordinary tests (each one compiles a small
//! crate); `just mutation-oracle` runs just this file.

#[path = "support/audit_tags.rs"]
mod audit_tags;
#[path = "support/inventory.rs"]
mod inventory;
#[path = "support/inventory_corpus.rs"]
mod inventory_corpus;
#[path = "support/mutate.rs"]
mod mutate;
#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/probe.rs"]
mod probe;
#[path = "support/real_shapes.rs"]
mod real_shapes;
#[path = "support/tools.rs"]
mod tools;

use exif_core::tiff::{Container, Link};
use exif_core::Error;
use oracle::{Mismatch, SELECTORS};
use probe::{TempDir, MARKER};
use std::path::{Path, PathBuf};

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_bytes(name: &str) -> Vec<u8> {
    oracle_tiff::fixtures()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
}

/// ExifTool's committed answer for a fixture: one entry per selector.
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
    tools::parse_fields(&oracle::names(SELECTORS), &rec.stdout)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The selectors a diff names, sorted.
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

// ── The apparatus: control, then mutant ────────────────────────────────────

/// Fixture files written once into a temp directory.
fn write_fixtures(dir: &TempDir, names: &[&str]) -> Vec<(String, PathBuf)> {
    names
        .iter()
        .map(|n| {
            let path = dir.path().join(format!("{n}.tif"));
            std::fs::write(&path, fixture_bytes(n)).expect("write fixture");
            ((*n).to_string(), path)
        })
        .collect()
}

/// Stage + build a probe crate, run it on each fixture and diff against the
/// committed ExifTool record. `Err` only for apparatus failures (build, run).
fn probe_mismatches(
    label: &str,
    fixtures: &[&str],
    inject: Option<&dyn Fn(&Path)>,
) -> Result<Vec<(String, Vec<Mismatch>)>, String> {
    let files = TempDir::new(&format!("{label}-files"));
    let paths = write_fixtures(&files, fixtures);
    let crate_dir = TempDir::new(label);
    probe::stage_probe_crate(crate_dir.path(), inject);
    let bin = probe::build_probe(crate_dir.path())?;
    let mut out = Vec::new();
    for (name, path) in &paths {
        let ours = probe::run_probe(&bin, path)?;
        out.push((name.clone(), oracle::diff(&ours, &theirs(name), SELECTORS)));
    }
    Ok(out)
}

/// The control: the unmutated staged copy must agree with ExifTool's committed
/// record on every given fixture. Without it a red mutant could be the
/// apparatus (the copy, the build, the probe's rendering) and not the fault.
fn assert_control_green(label: &str, fixtures: &[&str]) {
    let control = probe_mismatches(&format!("{label}-control"), fixtures, None)
        .unwrap_or_else(|e| panic!("{label}: the CONTROL could not be built or run: {e}"));
    for (name, mismatches) in control {
        assert!(
            mismatches.is_empty(),
            "{label}: the unmutated control is not green on {name}: {}",
            describe(&mismatches)
        );
    }
}

/// Control first (green), then the mutant; returns the mutant's mismatches per
/// fixture.
fn control_then_mutant(
    label: &str,
    fixtures: &[&str],
    inject: &dyn Fn(&Path),
) -> Vec<(String, Vec<Mismatch>)> {
    assert_control_green(label, fixtures);
    probe_mismatches(&format!("{label}-mutant"), fixtures, Some(inject))
        .unwrap_or_else(|e| panic!("{label}: the MUTANT could not be built or run: {e}"))
}

fn mismatches_for<'a>(all: &'a [(String, Vec<Mismatch>)], fixture: &str) -> &'a [Mismatch] {
    all.iter()
        .find(|(n, _)| n == fixture)
        .map(|(_, m)| m.as_slice())
        .unwrap_or_else(|| panic!("no result for {fixture}"))
}

fn marked(code: &str) -> String {
    format!("{code} {MARKER}")
}

/// Every fixture the mutants use, and then all the others: the unmutated
/// apparatus reproduces ExifTool's committed answer for each of them.
#[test]
fn controls_are_green_for_every_fixture_used() {
    let all: Vec<&str> = oracle_tiff::fixtures()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    for used in [
        "exif-basic-le",
        "exif-basic-be",
        "ascii-edges",
        "exif-gps-ne-le",
        "signed-float-le",
        "signed-float-be",
    ] {
        assert!(all.contains(&used), "{used} is not a fixture");
    }
    assert_control_green("all-fixtures", &all);
}

// ── Library-fault red-proofs ───────────────────────────────────────────────

// Mutant 1. Needle (src/tiff/types.rs, `Container::uints`, the SHORT arm):
//   out.push(u32::from(self.byte_order.u16(*chunk)));
// The mutation reads the SHORT little-endian whatever the file's byte order.
//
// Deviation from the spec's wording ("`ByteOrder::u16` big-endian arm reads
// little-endian"): that arm is also what `read_u16` uses for the header's
// version word and every IFD entry count, so mutating it breaks every
// big-endian file at the header and nothing reaches a SHORT-valued selector.
// The fault is therefore placed at the SHORT use site, which is the fault the
// test is about (a SHORT decoded in the wrong byte order).
#[test]
fn mutant_big_endian_short_read_is_caught() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("tiff/types.rs"),
            "out.push(u32::from(self.byte_order.u16(*chunk)));",
            &marked("out.push(u32::from(u16::from_le_bytes(*chunk)));"),
        );
    };
    let got = control_then_mutant("short-order", &["exif-basic-be", "exif-basic-le"], &inject);
    let be = mismatches_for(&got, "exif-basic-be");
    assert_eq!(
        named(be),
        sorted(vec!["ExifIFD:ISO", "IFD0:Orientation"]),
        "SHORT-valued selectors of the big-endian fixture: {}",
        describe(be)
    );
    // 6 read as 0x0600, 400 as 0x9001.
    assert!(be
        .iter()
        .any(|m| m.selector == "IFD0:Orientation" && m.ours.as_deref() == Some("1536")));
    assert!(be
        .iter()
        .any(|m| m.selector == "ExifIFD:ISO" && m.ours.as_deref() == Some("36865")));
    // The mutation cannot show on a little-endian file: the proof that this
    // test is reading the right fixture.
    let le = mismatches_for(&got, "exif-basic-le");
    assert!(
        le.is_empty(),
        "little-endian fixture must stay green: {}",
        describe(le)
    );
}

// Mutant 2. Needle (src/tiff/types.rs, `Container::ascii`): `Ok(Ascii::split(bytes))`.
// The mutation hands `split` the payload minus its last byte. A terminated
// string loses only its NUL (invisible); `ascii-edges` has an unterminated Make.
#[test]
fn mutant_ascii_drops_the_last_byte_is_caught() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("tiff/types.rs"),
            "Ok(Ascii::split(bytes))",
            &marked(
                "Ok(Ascii::split(bytes.get(..bytes.len().saturating_sub(1)).unwrap_or(bytes)))",
            ),
        );
    };
    let got = control_then_mutant("ascii-last-byte", &["ascii-edges"], &inject);
    let m = mismatches_for(&got, "ascii-edges");
    assert_eq!(named(m), vec!["IFD0:Make"], "{}", describe(m));
    assert_eq!(m[0].ours.as_deref(), Some("NoTerminato"));
    assert_eq!(m[0].theirs.as_deref(), Some("NoTerminator"));
}

// Mutant 3. Needles (src/tiff/types.rs, `Container::urationals`):
//   num: order.u32(num),   den: order.u32(den),
// The mutation swaps them. Each needle occurs once (the signed twin uses `i32`).
#[test]
fn mutant_rational_swaps_numerator_and_denominator_is_caught() {
    let inject = |src: &Path| {
        let file = src.join("tiff/types.rs");
        probe::inject_replace(
            &file,
            "num: order.u32(num),",
            &marked("num: order.u32(den),"),
        );
        probe::inject_replace(
            &file,
            "den: order.u32(den),",
            &marked("den: order.u32(num),"),
        );
    };
    let got = control_then_mutant(
        "rational-swap",
        &["exif-basic-le", "exif-gps-ne-le"],
        &inject,
    );
    let basic = mismatches_for(&got, "exif-basic-le");
    assert_eq!(
        named(basic),
        sorted(vec!["ExifIFD:FNumber", "IFD0:XResolution"]),
        "{}",
        describe(basic)
    );
    // XResolution 72/1 read as 1/72.
    assert!(basic.iter().any(|m| m.selector == "IFD0:XResolution"
        && m.ours
            .as_deref()
            .and_then(|s| s.parse::<f64>().ok())
            .is_some_and(|v| v < 0.02)));
    // GPS coordinates are three RATIONALs, so they go too.
    let gps = mismatches_for(&got, "exif-gps-ne-le");
    assert_eq!(
        named(gps),
        sorted(vec!["Composite:GPSLatitude", "Composite:GPSLongitude"]),
        "{}",
        describe(gps)
    );
}

// Mutant 4. Needle (src/tiff/mod.rs, the `POINTERS` table; two lines, because
// `tag: tags::GPS_INFO,` alone also occurs in this file's own tests):
//   tag: tags::GPS_INFO,\n        allowed_from: from_top_level_or_sub,
// The mutation points the GPS arm at a tag nothing uses, so the GPS pointer is
// never followed.
#[test]
fn mutant_gps_pointer_is_not_followed_is_caught() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("tiff/mod.rs"),
            "tag: tags::GPS_INFO,\n        allowed_from: from_top_level_or_sub,",
            &marked("tag: 0xFFFF,\n        allowed_from: from_top_level_or_sub,"),
        );
    };
    let got = control_then_mutant("gps-pointer", &["exif-gps-ne-le"], &inject);
    let m = mismatches_for(&got, "exif-gps-ne-le");
    assert_eq!(
        named(m),
        sorted(vec!["Composite:GPSLatitude", "Composite:GPSLongitude"]),
        "{}",
        describe(m)
    );
    // Ours has nothing where ExifTool has the coordinates.
    assert!(
        m.iter().all(|m| m.ours.is_none() && m.theirs.is_some()),
        "{}",
        describe(m)
    );
}

// Mutant 5. Needle (src/tiff/types.rs, `Container::srationals`):
//   num: order.i32(num),
// The mutation takes the numerator's absolute value: the sign is lost.
#[test]
fn mutant_signed_rational_sign_is_lost_is_caught() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("tiff/types.rs"),
            "num: order.i32(num),",
            &marked("num: order.i32(num).wrapping_abs(),"),
        );
    };
    let got = control_then_mutant("srational-sign", &["exif-basic-le"], &inject);
    let m = mismatches_for(&got, "exif-basic-le");
    assert_eq!(
        named(m),
        vec!["ExifIFD:ExposureCompensation"],
        "{}",
        describe(m)
    );
    // ExifTool -0.3333333333; ours +0.3333...
    assert!(
        m[0].ours.as_deref().is_some_and(|s| !s.starts_with('-')),
        "{}",
        describe(m)
    );
    assert!(
        m[0].theirs.as_deref().is_some_and(|s| s.starts_with('-')),
        "{}",
        describe(m)
    );
}

// Mutant 6: the SPEC-005 survivor. Needle (src/tiff/types.rs, `Container::sints`,
// the SSHORT arm):
//   out.push(i32::from(self.byte_order.i16(*chunk)));
// The mutation zero-extends the 16-bit value (`as u16`). It went unnoticed
// until SPEC-008 added a selector that reads an SSHORT: ExifIFD:ExposureProgram.
#[test]
fn mutant_signed_short_is_read_unsigned_is_caught() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("tiff/types.rs"),
            "out.push(i32::from(self.byte_order.i16(*chunk)));",
            &marked("out.push(i32::from(self.byte_order.i16(*chunk) as u16));"),
        );
    };
    let got = control_then_mutant(
        "sshort-unsigned",
        &["signed-float-le", "signed-float-be"],
        &inject,
    );
    for fixture in ["signed-float-le", "signed-float-be"] {
        let m = mismatches_for(&got, fixture);
        assert_eq!(
            named(m),
            vec!["ExifIFD:ExposureProgram"],
            "{fixture}: {}",
            describe(m)
        );
        // -2 as a zero-extended 16-bit value.
        assert_eq!(m[0].ours.as_deref(), Some("65534"), "{fixture}");
        assert_eq!(m[0].theirs.as_deref(), Some("-2"), "{fixture}");
    }
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

/// No `RED-PROOF INJECTION` marker in the library or any other non-test source:
/// an injection can only live in a staged copy. SPEC-011 widens the scan to
/// `tests/support/*.rs` (it had missed `tests/support/oracle.rs`, whose copy the
/// probe crate compiles): the one file allowed to hold the marker text is
/// `probe.rs`, which DEFINES it, and that file is checked to hold it exactly once.
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

    // The REAL scan covers what it claims (verify V12): every file under
    // tests/support is in the scanned set, except probe.rs, which defines the
    // marker and is exempt by name.
    let support = root().join("tests/support");
    let mut on_disk: Vec<PathBuf> = std::fs::read_dir(&support)
        .expect("read tests/support")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_file())
        .collect();
    on_disk.sort();
    assert!(on_disk.len() >= 6, "tests/support: {on_disk:?}");
    for file in &on_disk {
        let exempt = file.file_name().is_some_and(|n| n == "probe.rs");
        assert_eq!(
            scanned.contains(file),
            !exempt,
            "{} must be scanned unless it is probe.rs",
            file.display()
        );
    }
    for needed in [
        "oracle.rs",
        "oracle_jpeg.rs",
        "oracle_tiff.rs",
        "tools.rs",
        "mutate.rs",
    ] {
        assert!(
            scanned.contains(&support.join(needed)),
            "{needed} is not in the scanned set"
        );
    }

    // The scan covers what it says: the definition is in probe.rs exactly once,
    // and the scan does catch a marker in a file named like a support file.
    let probe_src = std::fs::read_to_string(root().join("tests/support/probe.rs")).expect("read");
    assert_eq!(probe_src.matches(MARKER).count(), 1, "the one definition");
    let dir = TempDir::new("marker-scan");
    let fake = dir.path().join("tests/support");
    std::fs::create_dir_all(&fake).expect("mkdir");
    for name in ["oracle.rs", "oracle_jpeg.rs", "mutate.rs", "tools.rs"] {
        std::fs::write(fake.join(name), format!("let x = 1; {MARKER}\n")).expect("write");
    }
    let mut found = Vec::new();
    marker_hits(&fake, &["probe.rs"], &mut found, &mut Vec::new());
    assert_eq!(found.len(), 4, "{found:?}");
    // The skip is by exact name: probe.rs is the only exemption.
    std::fs::write(fake.join("probe.rs"), MARKER).expect("write");
    let mut found = Vec::new();
    marker_hits(&fake, &["probe.rs"], &mut found, &mut Vec::new());
    assert_eq!(found.len(), 4, "probe.rs alone is skipped: {found:?}");
}

// ── Byte-mutation red-proofs (no probe crate: mutate the bytes in process) ──

fn ifd0_offset(bytes: &[u8]) -> usize {
    let order = mutate::order_of(bytes).expect("a TIFF fixture");
    let off = mutate::read_u32_at(bytes, 4, order).expect("header offset");
    usize::try_from(off).expect("offset fits")
}

/// Our reading of `bytes` (all `None` when the parse fails outright).
fn ours_of(bytes: &[u8]) -> Vec<Option<String>> {
    match Container::parse(bytes) {
        Ok(c) => oracle::read_ours(&c, SELECTORS),
        Err(_) => vec![None; SELECTORS.len()],
    }
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

const GPS_SELECTORS: [&str; 2] = ["Composite:GPSLatitude", "Composite:GPSLongitude"];

// Byte mutation 7. `exif-gps-full-le` has IFD0, an Exif IFD and a GPS IFD (a
// fixture added by SPEC-008 so the isolation claim has something to isolate).
// The GPS pointer VALUE is set past the end of the file. Compared with the
// ORIGINAL file's committed ExifTool record: exactly the GPS selectors go red,
// IFD0 and Exif selectors still match, and the container records one issue
// whose link is the GPS pointer.
#[test]
fn corrupting_the_gps_pointer_is_reported_and_isolated() {
    let test = "corrupting_the_gps_pointer_is_reported_and_isolated";
    let original = fixture_bytes("exif-gps-full-le");
    let record = theirs("exif-gps-full-le");
    let order = mutate::order_of(&original).expect("order");

    // Control: the unmutated bytes agree with the record.
    assert!(oracle::diff(&ours_of(&original), &record, SELECTORS).is_empty());

    let loc = mutate::locate_entry(&original, ifd0_offset(&original), 0x8825)
        .expect("the GPS pointer is in IFD0");
    let mut mutated = original.clone();
    assert!(mutate::set_u32_at(
        &mut mutated,
        loc.value_at,
        0x00FF_0000,
        order
    ));
    assert_ne!(mutated, original);
    assert_eq!(
        mutated.len(),
        original.len(),
        "only the pointer value changed"
    );

    let c = Container::parse(&mutated).expect("a bad GPS pointer is not fatal");
    assert_eq!(c.issues().len(), 1, "{:?}", c.issues());
    assert_eq!(c.issues()[0].link(), Link::Tag(0x8825));
    assert_eq!(c.issues()[0].offset(), Some(0x00FF_0000));

    let ours = oracle::read_ours(&c, SELECTORS);
    let m = oracle::diff(&ours, &record, SELECTORS);
    assert_eq!(
        named(&m),
        sorted(GPS_SELECTORS.to_vec()),
        "{}",
        describe(&m)
    );
    assert!(m.iter().all(|m| m.ours.is_none() && m.theirs.is_some()));
    // Isolation, stated directly: every IFD0 and Exif selector that ExifTool
    // reported has a value on our side too, and the diff did not name it (the
    // diff, not `==`, because rationals agree within a tolerance).
    let named_now = named(&m);
    let isolated: Vec<&str> = SELECTORS
        .iter()
        .zip(&ours)
        .zip(&record)
        .filter(|((s, o), t)| {
            (s.exiftool.starts_with("IFD0:") || s.exiftool.starts_with("ExifIFD:"))
                && o.is_some()
                && t.is_some()
                && !named_now.contains(&s.exiftool)
        })
        .map(|((s, _), _)| s.exiftool)
        .collect();
    assert!(
        isolated.contains(&"IFD0:Make")
            && isolated.contains(&"ExifIFD:ISO")
            && isolated.contains(&"ExifIFD:ExposureCompensation"),
        "{isolated:?}"
    );

    // Live variant: ExifTool on the same mutated bytes.
    if !live_gate(&format!("{test} (live variant)")) {
        return;
    }
    let path = tools::temp_file_for("mutated-gps-pointer", &mutated).expect("temp file");
    let live = tools::run_exiftool(&path, &oracle::names(SELECTORS));
    let _ = std::fs::remove_file(&path);
    let live = live.unwrap_or_else(|e| panic!("{test} (live variant): {e}"));
    for (s, v) in SELECTORS.iter().zip(&live) {
        if GPS_SELECTORS.contains(&s.exiftool) {
            assert!(
                v.is_none(),
                "live ExifTool still reports {}: {v:?}",
                s.exiftool
            );
        }
    }
    let live_m = oracle::diff(&ours, &live, SELECTORS);
    assert!(
        live_m.is_empty(),
        "ours and live ExifTool must agree on the mutated bytes: {}",
        describe(&live_m)
    );
    // And the IFD0/Exif values it returned are the original's.
    for ((s, l), r) in SELECTORS.iter().zip(&live).zip(&record) {
        if !GPS_SELECTORS.contains(&s.exiftool) {
            assert_eq!(l, r, "{}", s.exiftool);
        }
    }
    println!("{test} (live variant): ExifTool agrees, GPS selectors are `-`");
}

// Byte mutation 8. `II` flipped to `MM` in bytes 0..2 only; everything after
// is still little-endian. Measured: the header's version word is then read big
// endian, 42 stored as `2A 00` reads as 0x2A00, so the parse fails with
// `UnsupportedTiffVersion { version: 0x2A00 }`; no garbage values are read.
// The reading therefore differs from the record in every selector ExifTool
// reported, with no panic.
#[test]
fn swapping_the_byte_order_mark_changes_the_reading() {
    let original = fixture_bytes("exif-basic-le");
    let record = theirs("exif-basic-le");
    assert!(oracle::diff(&ours_of(&original), &record, SELECTORS).is_empty());

    let mut mutated = original.clone();
    assert!(mutate::swap_byte_order_mark(&mut mutated));
    assert_eq!(&mutated[..2], b"MM");
    assert_eq!(&mutated[2..], &original[2..], "only the mark changed");

    match Container::parse(&mutated) {
        Err(Error::UnsupportedTiffVersion { version }) => assert_eq!(version, 0x2A00),
        other => panic!(
            "expected UnsupportedTiffVersion, got {:?}",
            other.map(|c| c.ifds().len())
        ),
    }
    let m = oracle::diff(&ours_of(&mutated), &record, SELECTORS);
    let reported: Vec<&'static str> = SELECTORS
        .iter()
        .zip(&record)
        .filter(|(_, t)| t.is_some())
        .map(|(s, _)| s.exiftool)
        .collect();
    assert_eq!(named(&m), sorted(reported), "{}", describe(&m));
    assert!(m.iter().all(|m| m.ours.is_none()));
}

// Byte mutation 9. `exif-basic-le` cut inside the Exif IFD. IFD0 comes first
// in the file with all of its payloads, so it is untouched.
//   (a) cut inside the Exif IFD's entry table: the Exif IFD cannot be read, its
//       selectors vanish, the walk records one issue linked to the Exif
//       pointer, and IFD0 is intact.
//   (b) cut exactly where the Exif IFD's payload region starts (the table and
//       its next-word are whole): the IFD is read, no issue is recorded, and
//       only the selectors whose value lives in the payload region vanish; the
//       inline SHORT (ISO) survives. Measured below, not assumed.
#[test]
fn truncating_into_the_exif_ifd_loses_exactly_what_it_should() {
    let original = fixture_bytes("exif-basic-le");
    let record = theirs("exif-basic-le");
    let order = mutate::order_of(&original).expect("order");
    let loc = mutate::locate_entry(&original, ifd0_offset(&original), 0x8769)
        .expect("the Exif pointer is in IFD0");
    let exif_at = usize::try_from(
        mutate::read_u32_at(&original, loc.value_at, order).expect("pointer value"),
    )
    .expect("fits");
    let entries = 4usize; // FNumber, ISO, DateTimeOriginal, ExposureCompensation
    assert_eq!(
        mutate::read_u16_at(&original, exif_at, order),
        Some(u16::try_from(entries).expect("small"))
    );
    let table_end = exif_at + 2 + 12 * entries + 4;
    assert!(
        original.len() > table_end,
        "the Exif IFD has a payload region"
    );

    let ifd0_selectors: Vec<&str> = SELECTORS
        .iter()
        .map(|s| s.exiftool)
        .filter(|n| n.starts_with("IFD0:"))
        .collect();

    // (a) mid-table.
    let mut cut = original.clone();
    mutate::truncate_at(&mut cut, exif_at + 2 + 12 * 2 + 5);
    assert!(cut.len() < original.len());
    let c = Container::parse(&cut).expect("a cut Exif IFD is not fatal");
    assert_eq!(c.issues().len(), 1, "{:?}", c.issues());
    assert_eq!(c.issues()[0].link(), Link::Tag(0x8769));
    let ours = oracle::read_ours(&c, SELECTORS);
    let m = oracle::diff(&ours, &record, SELECTORS);
    assert_eq!(
        named(&m),
        sorted(vec![
            "ExifIFD:DateTimeOriginal",
            "ExifIFD:ExposureCompensation",
            "ExifIFD:FNumber",
            "ExifIFD:ISO",
        ]),
        "{}",
        describe(&m)
    );
    assert!(m.iter().all(|m| m.ours.is_none() && m.theirs.is_some()));
    for name in &ifd0_selectors {
        assert!(
            !named(&m).contains(name),
            "IFD0 selector {name} must be intact"
        );
    }

    // (b) at the start of the Exif IFD's payload region.
    let mut cut = original.clone();
    mutate::truncate_at(&mut cut, table_end);
    let c = Container::parse(&cut).expect("a missing payload region is not fatal");
    assert!(
        c.is_clean(),
        "table and next-word are whole: {:?}",
        c.issues()
    );
    let ours = oracle::read_ours(&c, SELECTORS);
    let m = oracle::diff(&ours, &record, SELECTORS);
    assert_eq!(
        named(&m),
        sorted(vec![
            "ExifIFD:DateTimeOriginal",
            "ExifIFD:ExposureCompensation",
            "ExifIFD:FNumber",
        ]),
        "{}",
        describe(&m)
    );
    let iso = SELECTORS
        .iter()
        .position(|s| s.exiftool == "ExifIFD:ISO")
        .expect("ISO");
    assert_eq!(ours[iso], record[iso], "the inline SHORT survives");
}

// ═══ JPEG (SPEC-011) ═══════════════════════════════════════════════════════
//
// The same two apparatuses, pointed at JPEG. A mutant is compared with the
// COMMITTED `jpeg-*.txt` records through `oracle::compare_jpeg`, so a divergence
// the library is supposed to have (ours reads, ExifTool does not) is part of the
// control, and a fault that makes such a divergence DISAPPEAR is a red result
// too (a stale entry), exactly like a new mismatch.

fn jpeg_theirs(name: &str) -> Vec<Option<String>> {
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
    tools::parse_fields(&oracle::jpeg_names(), &rec.stdout)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// `oracle::compare_jpeg` with the pin a wildcard fixture needs (M1).
fn compare_pinned(
    name: &str,
    ours: &[Option<String>],
    theirs: &[Option<String>],
) -> oracle::JpegComparison {
    let pin = oracle::exif_pin_record(name).map(jpeg_theirs_record);
    oracle::compare_jpeg_pinned(name, ours, theirs, pin.as_deref())
}

/// ExifTool's fields from a committed record, 20 or 22 wide (Exif part kept).
fn jpeg_theirs_record(name: &str) -> Vec<Option<String>> {
    let path = root()
        .join("tests/oracle-fixtures/exiftool")
        .join(format!("{name}.txt"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let rec = tools::parse_record(&text).unwrap_or_else(|| panic!("{name}: bad header"));
    rec.stdout
        .trim_end_matches('\n')
        .split('\t')
        .take(oracle::SELECTORS.len())
        .map(|f| if f == "-" { None } else { Some(f.to_string()) })
        .collect()
}

fn jpeg_fixture(name: &str) -> Vec<u8> {
    oracle_jpeg::fixtures()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no JPEG fixture named {name}"))
}

/// Our in-process reading of a JPEG fixture (22 fields).
fn jpeg_ours(bytes: &[u8]) -> Vec<Option<String>> {
    match exif_core::jpeg::read(bytes) {
        Ok(m) => oracle::read_jpeg_all(&m),
        Err(_) => vec![None; oracle::jpeg_names().len()],
    }
}

/// Stage + build a probe crate, run it on each JPEG fixture and compare with the
/// committed record (divergences applied). `Err` only for apparatus failures.
fn jpeg_probe(
    label: &str,
    fixtures: &[&str],
    inject: Option<&dyn Fn(&Path)>,
) -> Result<Vec<(String, oracle::JpegComparison)>, String> {
    let files = TempDir::new(&format!("{label}-files"));
    let paths: Vec<(String, PathBuf)> = fixtures
        .iter()
        .map(|n| {
            let path = files.path().join(format!("{n}.jpg"));
            std::fs::write(&path, jpeg_fixture(n)).expect("write fixture");
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
            oracle::jpeg_names().len(),
            "{name}: the probe's JPEG mode prints one field per JPEG selector"
        );
        out.push((
            name.clone(),
            compare_pinned(name, &ours, &jpeg_theirs(name)),
        ));
    }
    Ok(out)
}

fn assert_jpeg_control_clean(label: &str, fixtures: &[&str]) {
    let control = jpeg_probe(&format!("{label}-control"), fixtures, None)
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
fn jpeg_control_then_mutant(
    label: &str,
    fixtures: &[&str],
    inject: &dyn Fn(&Path),
) -> Vec<(String, oracle::JpegComparison)> {
    assert_jpeg_control_clean(label, fixtures);
    jpeg_probe(&format!("{label}-mutant"), fixtures, Some(inject))
        .unwrap_or_else(|e| panic!("{label}: the MUTANT could not be built or run: {e}"))
}

fn cmp_for<'a>(
    all: &'a [(String, oracle::JpegComparison)],
    fixture: &str,
) -> &'a oracle::JpegComparison {
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
        oracle::jpeg_names()
            .into_iter()
            .zip(jpeg_theirs(fixture))
            .filter(|(_, t)| t.is_some())
            .map(|(n, _)| n)
            .collect(),
    )
}

/// Every fixture any JPEG mutant uses.
const JPEG_MUTANT_FIXTURES: &[&str] = &[
    "jpeg-exif-basic-le",
    "jpeg-exif-gps-ne-le",
    "jpeg-exif-sixth-byte-nonzero",
    "jpeg-two-exif-same-tag",
    "jpeg-two-exif-same-tag-reversed",
    "jpeg-two-exif-disjoint-tags",
    "jpeg-exif-after-scan",
    "jpeg-seventeen-exif-last-differs",
    "jpeg-exif-in-app2",
    "jpeg-iptc",
    "jpeg-iptc-without-0404",
];

#[test]
fn jpeg_probe_control_run_is_clean_before_every_mutant() {
    // One control over every fixture the mutants use, then all the others: the
    // unmutated apparatus (copy, build, JPEG-mode probe, rendering) reproduces
    // the committed ExifTool answers, with the known divergences exercised.
    let all: Vec<String> = oracle_jpeg::fixtures()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    for used in JPEG_MUTANT_FIXTURES {
        assert!(all.iter().any(|n| n == used), "{used} is not a fixture");
    }
    let refs: Vec<&str> = all.iter().map(String::as_str).collect();
    assert_jpeg_control_clean("jpeg-all-fixtures", &refs);

    // And the control is not vacuous: the probe's reading is the in-process one.
    let files = TempDir::new("jpeg-control-same-files");
    let crate_dir = TempDir::new("jpeg-control-same");
    probe::stage_probe_crate(crate_dir.path(), None);
    let bin = probe::build_probe(crate_dir.path()).expect("build");
    for name in ["jpeg-exif-gps-ne-le", "jpeg-two-exif-same-tag", "jpeg-iptc"] {
        let path = files.path().join(format!("{name}.jpg"));
        std::fs::write(&path, jpeg_fixture(name)).expect("write");
        let via_probe = probe::run_probe(&bin, &path).expect("run");
        assert_eq!(via_probe, jpeg_ours(&jpeg_fixture(name)), "{name}");
    }
}

// Mutant m1. Needle (src/jpeg/metadata.rs, `extract`, Rule 1): `after(EXIF_HEADER_LEN)`.
// The TIFF slice starts at payload offset 5, one byte early: it begins with the
// sixth signature byte (a NUL), so no TIFF header is found and no tag is read.
#[test]
fn jpeg_mutant_tiff_slice_starts_at_five_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("jpeg/metadata.rs"),
            "after(EXIF_HEADER_LEN)",
            &marked("after(5)"),
        );
    };
    let fixtures = ["jpeg-exif-basic-le", "jpeg-exif-gps-ne-le"];
    let got = jpeg_control_then_mutant("jpeg-m1-slice-at-5", &fixtures, &inject);
    for f in fixtures {
        let c = cmp_for(&got, f);
        assert_eq!(
            named(&c.unexplained),
            reported(f),
            "{f}: {}",
            describe(&c.unexplained)
        );
        assert!(
            c.unexplained
                .iter()
                .all(|m| m.ours.is_none() && m.theirs.is_some()),
            "{f}: {}",
            describe(&c.unexplained)
        );
    }
    assert!(named(&cmp_for(&got, "jpeg-exif-basic-le").unexplained).contains(&"IFD0:Make"));
}

// Mutant m2: the DEC-007 regression. Needle (src/jpeg/mod.rs, `classify`): the
// Exif arm. The six-byte signature test is restored, so `Exif\0` + a non-zero
// sixth byte is no longer an Exif block.
#[test]
fn jpeg_mutant_six_byte_signature_restored_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("jpeg/mod.rs"),
            "0xE1 if payload.len() >= EXIF_HEADER_LEN && payload.starts_with(SIG_EXIF) => {",
            &marked("0xE1 if payload.starts_with(b\"Exif\\0\\0\") => {"),
        );
    };
    let fixtures = ["jpeg-exif-sixth-byte-nonzero", "jpeg-exif-basic-le"];
    let got = jpeg_control_then_mutant("jpeg-m2-six-byte", &fixtures, &inject);
    let c = cmp_for(&got, "jpeg-exif-sixth-byte-nonzero");
    assert_eq!(
        named(&c.unexplained),
        reported("jpeg-exif-sixth-byte-nonzero"),
        "{}",
        describe(&c.unexplained)
    );
    assert!(c
        .unexplained
        .iter()
        .all(|m| m.ours.is_none() && m.theirs.is_some()));
    assert!(named(&c.unexplained).contains(&"IFD0:Make"));
    // The canonical signature is unaffected.
    assert!(cmp_for(&got, "jpeg-exif-basic-le").is_clean());
}

// Mutant m3. Needle (src/jpeg/metadata.rs, `extract`): `exif_blocks.push(ExifBlock {`.
// Each new block is put in FRONT, so the block list is in reverse file order and
// the LAST file block becomes `exif()`'s primary. It is caught through the
// harness's several-Exif model (`oracle::read_ours_jpeg`: the later block in the
// list wins), by the two same-tag fixtures; the disjoint-tags fixture reads the
// same either way and stays clean.
#[test]
fn jpeg_mutant_last_block_primary_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("jpeg/metadata.rs"),
            "exif_blocks.push(ExifBlock {",
            &marked("exif_blocks.insert(0, ExifBlock {"),
        );
    };
    let fixtures = [
        "jpeg-two-exif-same-tag",
        "jpeg-two-exif-same-tag-reversed",
        "jpeg-two-exif-disjoint-tags",
        "jpeg-exif-basic-le",
    ];
    let got = jpeg_control_then_mutant("jpeg-m3-last-primary", &fixtures, &inject);
    let c = cmp_for(&got, "jpeg-two-exif-same-tag");
    assert_eq!(
        named(&c.unexplained),
        vec!["IFD0:Make"],
        "{}",
        describe(&c.unexplained)
    );
    assert_eq!(c.unexplained[0].ours.as_deref(), Some("AlphaCam"));
    assert_eq!(c.unexplained[0].theirs.as_deref(), Some("BravoCam"));
    let c = cmp_for(&got, "jpeg-two-exif-same-tag-reversed");
    assert_eq!(
        named(&c.unexplained),
        vec!["IFD0:Make"],
        "{}",
        describe(&c.unexplained)
    );
    assert_eq!(c.unexplained[0].ours.as_deref(), Some("BravoCam"));
    assert_eq!(c.unexplained[0].theirs.as_deref(), Some("AlphaCam"));
    assert!(cmp_for(&got, "jpeg-two-exif-disjoint-tags").is_clean());
    assert!(cmp_for(&got, "jpeg-exif-basic-le").is_clean());
}

// Mutant m4. Needle (src/jpeg/metadata.rs, `extract`): `SegmentKind::Exif => {`.
// An Exif segment after the first SOS is skipped. That makes the library AGREE
// with ExifTool on `jpeg-exif-after-scan`, so the named failure is the recorded
// divergence disappearing (a stale entry), not a new mismatch; every fixture with
// its Exif before the scan is unaffected.
#[test]
fn jpeg_mutant_exif_after_scan_skipped_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("jpeg/metadata.rs"),
            "SegmentKind::Exif => {",
            &marked(
                "SegmentKind::Exif if segment.after_scan() => {}\n            SegmentKind::Exif => {",
            ),
        );
    };
    let fixtures = ["jpeg-exif-after-scan", "jpeg-exif-basic-le"];
    let got = jpeg_control_then_mutant("jpeg-m4-after-scan", &fixtures, &inject);
    let c = cmp_for(&got, "jpeg-exif-after-scan");
    // Two reds: the pin (the wildcard fixture's reading must equal ExifTool's
    // reading of the same TIFF read normally: every field is now lost) and the
    // stale divergence.
    assert_eq!(
        named(&c.unexplained),
        reported("jpeg-exif-basic-le"),
        "{}",
        describe(&c.unexplained)
    );
    assert!(c.unexplained.iter().all(|m| m.ours.is_none()));
    assert_eq!(c.stale.len(), 1, "{:?}", c.stale);
    assert!(
        c.stale[0].contains("jpeg-exif-after-scan")
            && c.stale[0].contains(oracle::EXIF_ANY)
            && c.stale[0].contains("exiftool-ignores-exif-after-scan"),
        "{:?}",
        c.stale
    );
    assert!(cmp_for(&got, "jpeg-exif-basic-le").is_clean());
}

// Mutant m5. Needle (src/jpeg/metadata.rs): `pub const MAX_EXIF_BLOCKS: usize = 16;`.
// Off by one HIGH (17): the seventeenth block is parsed, the harness's last-block
// model reads `LastOnly17` and agrees with ExifTool, so the recorded divergence
// disappears (a stale entry naming it). Off by one LOW (15) is also tried and is
// reported honestly: the fixture's first sixteen blocks are identical, so ours
// still reads `SameFirst16` and the divergence is unchanged; a cap of 15 is
// caught by the library's own unit tests (`seventeen_exif...`), not by this
// fixture.
#[test]
fn jpeg_mutant_exif_block_cap_off_by_one_is_caught() {
    let fixtures = ["jpeg-seventeen-exif-last-differs", "jpeg-exif-basic-le"];
    let cap = |n: u32| {
        move |src: &Path| {
            probe::inject_replace(
                &src.join("jpeg/metadata.rs"),
                "pub const MAX_EXIF_BLOCKS: usize = 16;",
                &marked(&format!("pub const MAX_EXIF_BLOCKS: usize = {n};")),
            );
        }
    };
    let high = cap(17);
    let got = jpeg_control_then_mutant("jpeg-m5-cap-17", &fixtures, &high);
    let c = cmp_for(&got, "jpeg-seventeen-exif-last-differs");
    assert!(c.unexplained.is_empty(), "{}", describe(&c.unexplained));
    assert_eq!(c.stale.len(), 1, "{:?}", c.stale);
    assert!(
        c.stale[0].contains("jpeg-seventeen-exif-last-differs")
            && c.stale[0].contains("IFD0:Make")
            && c.stale[0].contains("exiftool-reads-17th-exif-block"),
        "{:?}",
        c.stale
    );
    assert!(cmp_for(&got, "jpeg-exif-basic-le").is_clean());

    // The low variant (control already run above for this apparatus).
    let low = cap(15);
    let got = jpeg_probe("jpeg-m5-cap-15", &fixtures, Some(&low)).expect("build and run");
    let c = cmp_for(&got, "jpeg-seventeen-exif-last-differs");
    assert!(
        c.is_clean() && named(&c.explained) == vec!["IFD0:Make"],
        "cap 15 leaves this fixture's reading unchanged: unexplained [{}] stale {:?}",
        describe(&c.unexplained),
        c.stale
    );
}

// Mutant m6. Needle (src/jpeg/mod.rs, `classify`): the Exif arm. The APP1 marker
// guard is relaxed to any APPn, so `Exif\0\0` + a TIFF under APP2 becomes an
// Exif block that ExifTool does not read.
#[test]
fn jpeg_mutant_app1_marker_guard_relaxed_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("jpeg/mod.rs"),
            "0xE1 if payload.len() >= EXIF_HEADER_LEN && payload.starts_with(SIG_EXIF) => {",
            &marked(
                "0xE0..=0xEF if payload.len() >= EXIF_HEADER_LEN && payload.starts_with(SIG_EXIF) => {",
            ),
        );
    };
    let fixtures = ["jpeg-exif-in-app2", "jpeg-exif-basic-le"];
    let got = jpeg_control_then_mutant("jpeg-m6-marker-guard", &fixtures, &inject);
    let c = cmp_for(&got, "jpeg-exif-in-app2");
    // Ours now reads exif-basic-le's values where ExifTool reads nothing: every
    // Exif selector that fixture's TIFF holds is named, in the one direction.
    let want = reported("jpeg-exif-basic-le");
    assert_eq!(named(&c.unexplained), want, "{}", describe(&c.unexplained));
    assert!(c
        .unexplained
        .iter()
        .all(|m| m.ours.is_some() && m.theirs.is_none()));
    assert!(named(&c.unexplained).contains(&"IFD0:Make"));
    assert!(cmp_for(&got, "jpeg-exif-basic-le").is_clean());
}

// Mutant m7 (reworked by SPEC-019: DEC-012 replaced the 0x0404-resource rule
// with "every Photoshop segment is an Iptc region"). Needle
// (src/jpeg/metadata.rs, `iptc`): the `out.push(Region::Iptc { .. })`
// statement. It is dropped: no Photoshop segment is located any more.
#[test]
fn jpeg_mutant_iptc_id_compare_wrong_is_named() {
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("jpeg/metadata.rs"),
            "out.push(Region::Iptc {\n            segment: index,\n            data: body,\n        });",
            &marked("let _ = &out;"),
        );
    };
    let fixtures = ["jpeg-iptc", "jpeg-iptc-without-0404", "jpeg-exif-basic-le"];
    let got = jpeg_control_then_mutant("jpeg-m7-iptc-id", &fixtures, &inject);
    let c = cmp_for(&got, "jpeg-iptc");
    assert_eq!(
        named(&c.unexplained),
        vec!["IPTC:ObjectName"],
        "{}",
        describe(&c.unexplained)
    );
    assert!(c.unexplained[0].ours.is_none() && c.unexplained[0].theirs.is_some());
    // The pinned DEC-012 divergence of this fixture (ours "present", theirs
    // nothing) stops being one: it is reported stale, in both directions.
    let c = cmp_for(&got, "jpeg-iptc-without-0404");
    assert!(c.unexplained.is_empty(), "{}", describe(&c.unexplained));
    assert!(!c.stale.is_empty(), "the pinned divergence goes stale");
    assert!(cmp_for(&got, "jpeg-exif-basic-le").is_clean());
}

// ── Probe wall-clock limit (SPEC-008 carry-over, SPEC-011) ─────────────────

/// A probe whose library loops forever must be killed within the limit and the
/// failure must be typed, not a hang. The loop is injected into the COPY's
/// `extract`; the limit is passed explicitly (a tiny one), so no test touches the
/// process environment. The build limit gets the same treatment with a limit too
/// small for any cargo run.
#[test]
fn probe_build_and_run_have_a_wall_clock_limit() {
    use std::time::{Duration, Instant};

    // The environment override is a positive integer or the default (pure).
    assert_eq!(
        probe::parse_limit_secs(Some("2"), probe::DEFAULT_RUN_SECS),
        2
    );
    assert_eq!(
        probe::parse_limit_secs(Some("0"), probe::DEFAULT_RUN_SECS),
        probe::DEFAULT_RUN_SECS
    );
    assert_eq!(probe::BUILD_LIMIT_VAR, "DIAMOND_EXIF_PROBE_BUILD_SECS");
    assert_eq!(probe::RUN_LIMIT_VAR, "DIAMOND_EXIF_PROBE_RUN_SECS");

    // Run: a library that never returns from `extract`.
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("jpeg/metadata.rs"),
            "fn extract<'a>(jpeg: Jpeg<'a>, data: &'a [u8], embedded: bool) -> JpegMetadata<'a> {",
            &marked(
                "fn extract<'a>(jpeg: Jpeg<'a>, data: &'a [u8], embedded: bool) -> JpegMetadata<'a> {\n    #[allow(clippy::empty_loop)]\n    loop {}",
            ),
        );
    };
    let files = TempDir::new("probe-limit-files");
    let file = files.path().join("jpeg-exif-basic-le.jpg");
    std::fs::write(&file, jpeg_fixture("jpeg-exif-basic-le")).expect("write");
    let crate_dir = TempDir::new("probe-limit");
    probe::stage_probe_crate(crate_dir.path(), Some(&inject));
    let bin = probe::build_probe(crate_dir.path()).expect("the looping probe still builds");
    let started = Instant::now();
    let err = probe::run_probe_with_limit(&bin, &file, Duration::from_secs(2))
        .expect_err("a probe that loops must not return a reading");
    let elapsed = started.elapsed();
    assert!(
        err.contains("killed") && err.contains(probe::RUN_LIMIT_VAR),
        "a typed timeout error: {err}"
    );
    assert!(
        elapsed >= Duration::from_secs(2) && elapsed < Duration::from_secs(15),
        "killed after about 2 s, took {elapsed:?}"
    );

    // Build: a limit no cargo run can meet.
    let dir = TempDir::new("probe-build-limit");
    probe::stage_probe_crate(dir.path(), None);
    let started = Instant::now();
    let err = probe::build_probe_with_limit(dir.path(), Duration::from_millis(1))
        .expect_err("a build cannot finish in 1 ms");
    assert!(
        err.contains("killed") && err.contains(probe::BUILD_LIMIT_VAR),
        "a typed timeout error: {err}"
    );
    assert!(started.elapsed() < Duration::from_secs(10));
}

// ── Byte-mutation red-proofs on a JPEG (no probe crate) ─────────────────────

/// Where the TIFF blob of the (single) Exif APP1 starts in a fixture's bytes,
/// found WITHOUT the library under test: the first `Exif\0\0` signature plus 6.
fn tiff_start(jpeg: &[u8]) -> usize {
    let sig = b"Exif\0\0";
    jpeg.windows(sig.len())
        .position(|w| w == sig)
        .expect("an Exif signature")
        + sig.len()
}

// Byte mutation J1 (design section 8, red-proof 2). `jpeg-exif-gps-full-le` has
// IFD0, an Exif IFD and a GPS IFD inside its APP1 payload. The GPS pointer VALUE
// (an offset relative to the TIFF blob) is set past the end, at file offset
// `segment + 10 + value_at` (here found as the signature position + 6 +
// `locate_entry`'s TIFF-relative offset). Compared with the ORIGINAL fixture's
// committed record: (a) exactly the GPS selectors go red, nothing else; (b) the
// JPEG layer stays clean and the damage is one issue on that Exif block (the
// layering of DEC-006), checked directly in the next test.
#[test]
fn jpeg_byte_mutation_corrupt_ifd_offset_in_app1_payload_is_named() {
    let test = "jpeg_byte_mutation_corrupt_ifd_offset_in_app1_payload_is_named";
    let name = "jpeg-exif-gps-full-le";
    let original = jpeg_fixture(name);
    let record = jpeg_theirs(name);

    // Control: the unmutated bytes agree with the record.
    assert!(compare_pinned(name, &jpeg_ours(&original), &record).is_clean());

    let start = tiff_start(&original);
    let tiff = &original[start..];
    let order = mutate::order_of(tiff).expect("a TIFF inside the APP1");
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

    let ours = jpeg_ours(&mutated);
    let cmp = compare_pinned(name, &ours, &record);
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
    // Everything else (IFD0, the Exif IFD) still matches.
    let kept = oracle::jpeg_names()
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
    let path =
        tools::temp_file_with_ext("mutated-jpeg-gps-pointer", "jpg", &mutated).expect("temp file");
    let live = tools::run_exiftool(&path, &oracle::jpeg_names());
    let _ = std::fs::remove_file(&path);
    let live = live.unwrap_or_else(|e| panic!("{test} (live variant): {e}"));
    for (n, v) in oracle::jpeg_names().iter().zip(&live) {
        if GPS_SELECTORS.contains(n) {
            assert!(v.is_none(), "live ExifTool still reports {n}: {v:?}");
        }
    }
    let live_cmp = compare_pinned(name, &ours, &live);
    assert!(
        live_cmp.unexplained.is_empty(),
        "ours and live ExifTool must agree on the mutated bytes: {}",
        describe(&live_cmp.unexplained)
    );
    println!("{test} (live variant): ExifTool agrees, GPS selectors are `-`");
}

// Layering (DEC-006): the same corruption leaves the JPEG layer clean and shows
// up as exactly one issue on the Exif block, whose link is the GPS pointer; the
// layered view names it and `is_clean` is false.
#[test]
fn jpeg_byte_mutation_keeps_the_jpeg_layer_clean_and_the_damage_on_the_block() {
    use exif_core::jpeg::AnyIssue;
    let name = "jpeg-exif-gps-full-le";
    let original = jpeg_fixture(name);
    let start = tiff_start(&original);
    let tiff = &original[start..];
    let order = mutate::order_of(tiff).expect("order");
    let ifd0 = usize::try_from(mutate::read_u32_at(tiff, 4, order).expect("IFD0")).expect("fits");
    let loc = mutate::locate_entry(tiff, ifd0, 0x8825).expect("GPS pointer");

    // Control: the original reads clean at every layer.
    let meta = exif_core::jpeg::read(&original).expect("a JPEG");
    assert!(meta.is_clean() && meta.jpeg().is_clean());
    assert_eq!(meta.issues().count(), 0);

    let mut mutated = original.clone();
    assert!(mutate::set_u32_at(
        &mut mutated,
        start + loc.value_at,
        0x00FF_0000,
        order
    ));
    let meta = exif_core::jpeg::read(&mutated).expect("a damaged Exif is not fatal");
    assert!(meta.jpeg().is_clean(), "{:?}", meta.jpeg().issues());
    assert!(meta.jpeg_issues().is_empty(), "the JPEG layer stays clean");
    assert!(!meta.is_clean());
    assert_eq!(meta.exif_blocks().len(), 1);
    let issues: Vec<AnyIssue<'_>> = meta.issues().collect();
    assert_eq!(issues.len(), 1, "{issues:?}");
    match issues[0] {
        AnyIssue::Exif { block, issue } => {
            assert_eq!(block, 0);
            assert_eq!(issue.link(), Link::Tag(0x8825));
            // The offset is relative to the TIFF blob (not to the file).
            assert_eq!(issue.offset(), Some(0x00FF_0000));
        }
        other => panic!("expected an Exif-layer issue, got {other:?}"),
    }
}

// Byte mutation J2 (design section 8, red-proof 2c). `jpeg-exif-gps-full-le` cut
// exactly where the scan starts (after the APP1, before the SOS marker): the
// outcome is typed (Ok; MissingSos then MissingEoi; the Exif block still read and
// clean), the values equal the original's, and nothing panics at any prefix of the
// file. ExifTool reads nothing from the cut-at-APP1 shape (the recorded
// divergence `jpeg-exif-last-segment-at-eof`); that is not asserted here.
#[test]
fn jpeg_truncated_between_app1_and_sos_is_a_typed_outcome() {
    use exif_core::jpeg::{self, SegmentKind};
    let name = "jpeg-exif-gps-full-le";
    let original = jpeg_fixture(name);
    let full = jpeg::read(&original).expect("a JPEG");
    let sos = full
        .jpeg()
        .segments()
        .iter()
        .find(|s| s.kind() == SegmentKind::Sos)
        .expect("an SOS");
    let cut = &original[..sos.offset()];

    let meta = jpeg::read(cut).expect("a truncated JPEG is not fatal");
    let kinds: Vec<&Error> = meta.jpeg_issues().iter().map(|i| i.error()).collect();
    assert_eq!(kinds, [&Error::MissingSos, &Error::MissingEoi]);
    assert_eq!(meta.exif_blocks().len(), 1);
    assert!(meta.exif_blocks()[0]
        .container()
        .is_ok_and(|c| c.is_clean()));
    assert_eq!(
        oracle::read_jpeg_all(&meta),
        oracle::read_jpeg_all(&full),
        "the cut loses no tag"
    );
    println!(
        "truncated at the SOS (file offset {}): JPEG-layer issues {:?}, {} Exif block, same tags",
        sos.offset(),
        kinds,
        meta.exif_blocks().len()
    );
    // Against the original's record: identical, nothing named.
    let cmp = compare_pinned(name, &oracle::read_jpeg_all(&meta), &jpeg_theirs(name));
    assert!(cmp.is_clean(), "{cmp:?}");

    // No prefix of the file panics, and every one is Ok or a typed fatal.
    for len in 0..=original.len() {
        match jpeg::read(&original[..len]) {
            Ok(m) => {
                let _ = oracle::read_jpeg_all(&m);
                let _ = m.issues().count();
            }
            Err(e) => assert!(len < 2 || e.to_string().len() > 1, "{len}: {e}"),
        }
    }
}

// ── The live variant skips loudly without ExifTool ─────────────────────────

/// Run the named test in a child copy of this binary with `PATH` pointing at
/// nothing, so ExifTool cannot be found. (The offline half of the GPS test
/// needs no cargo and no tool, so it runs to completion.)
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
fn the_gps_live_variant_skips_loudly_and_required_mode_fails() {
    let test = "corrupting_the_gps_pointer_is_reported_and_isolated";
    let out = run_self_without_exiftool(test, false);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{test}: should pass when skipped\n{stdout}"
    );
    assert!(
        stdout.contains(&format!("SKIP {test} (live variant)"))
            && stdout.contains("exiftool not on PATH"),
        "expected a SKIP line, got:\n{stdout}"
    );

    let out = run_self_without_exiftool(test, true);
    let both = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.status.success(),
        "{REQUIRE_VAR}=1 with no exiftool must fail\n{both}"
    );
    assert!(
        both.contains(&format!(
            "{test} (live variant): exiftool is not on PATH and {REQUIRE_VAR}=1"
        )),
        "{both}"
    );
}

// ── SPEC-013: the inventory probe and the library faults i1 to i6 ───────────
//
// The probe crate's `--inventory` mode prints the audit's inventory (the lines
// `inventory::ours` builds from `exif_core::audit::audit`), so a fault in the
// COPY of `src/audit/` or `src/tiff/` is compared with ExifTool's committed
// inventory through exactly the normalisation the live check uses. Same shape
// as the value mutants: a control on the unmutated copy first (every input
// agrees with its record, or has its pinned divergence), then the mutant,
// which must give a NAMED difference: the exact `(group, id)` that moved.

use inventory::{Inventory, InventoryDiff, Multiset, RegionKind};

fn inv_input(key: &str) -> inventory_corpus::Input {
    inventory_corpus::all_inputs(&root())
        .into_iter()
        .find(|i| i.key() == key)
        .unwrap_or_else(|| panic!("no inventory input {key}"))
}

/// ExifTool's committed inventory of an input (regions dropped for a bare TIFF,
/// Class H).
fn inv_theirs(key: &str) -> Inventory {
    let (corpus, name) = key.split_once('/').expect("corpus/name");
    let path = root()
        .join(inventory_corpus::RECORD_DIR)
        .join(inventory_corpus::record_file(corpus));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); run `just oracle-record`",
            path.display()
        )
    });
    let rec = inventory::parse_record(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let sec = rec
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section {name}"));
    let mut inv = inventory::inventory_of(&sec.lines).expect("classifiable");
    if !inv_input(key).is_jpeg() {
        inv.regions.clear();
    }
    inv
}

/// Stage + build a probe crate (optionally faulted), run its inventory mode on
/// each input key, and return what it printed. `Err` only for apparatus
/// failures (build, run).
fn inventory_probe(
    label: &str,
    keys: &[&str],
    inject: Option<&dyn Fn(&Path)>,
) -> Result<Vec<(String, Inventory)>, String> {
    let files = TempDir::new(&format!("{label}-files"));
    let crate_dir = TempDir::new(label);
    probe::stage_probe_crate(crate_dir.path(), inject);
    let bin = probe::build_probe(crate_dir.path())?;
    let mut out = Vec::new();
    for key in keys {
        let input = inv_input(key);
        let path = files
            .path()
            .join(format!("{}.{}", key.replace('/', "_"), input.ext()));
        std::fs::write(&path, &input.bytes).expect("write input");
        let text = probe::run_probe_inventory(&bin, &path)?;
        out.push((
            (*key).to_string(),
            inventory::parse_probe_text(&text).map_err(|e| format!("{key}: {e}"))?,
        ));
    }
    Ok(out)
}

/// The probe's reading of `key` against its committed record.
fn inv_diff(all: &[(String, Inventory)], key: &str) -> InventoryDiff {
    let mine = &all
        .iter()
        .find(|(k, _)| k == key)
        .unwrap_or_else(|| panic!("no result for {key}"))
        .1;
    let d = inventory::diff(mine, &inv_theirs(key));
    // Printed so the red result is visible in `--nocapture` output.
    println!("MUTANT RESULT {key}:\n{d}");
    d
}

fn set(rows: &[(&str, u16, usize)]) -> Multiset {
    rows.iter()
        .map(|&(g, id, n)| ((g.to_string(), id), n))
        .collect()
}

/// The control: on the unmutated copy every key agrees with its committed
/// record, or has exactly its pinned divergence.
fn assert_inventory_control(label: &str, keys: &[&str]) {
    let control = inventory_probe(&format!("{label}-control"), keys, None)
        .unwrap_or_else(|e| panic!("{label}: the CONTROL could not be built or run: {e}"));
    for (key, mine) in control {
        let bytes = inv_input(&key).bytes;
        let audit = exif_core::audit::audit(&bytes);
        // The probe's reading is the in-process one.
        assert_eq!(
            mine,
            inventory::ours(&audit),
            "{label}: the probe's inventory of {key} is not the in-process one"
        );
        // And clean unless pinned; `clean` is taken from the audit itself.
        let d = inventory::diff(&mine, &inv_theirs(&key));
        inventory::check_against(inventory::KNOWN_DIVERGENCES, &key, &d, audit.health().clean)
            .unwrap_or_else(|e| panic!("{label}: the unmutated control is not clean: {e}"));
    }
}

fn inventory_control_then_mutant(
    label: &str,
    keys: &[&str],
    inject: &dyn Fn(&Path),
) -> Vec<(String, Inventory)> {
    assert_inventory_control(label, keys);
    inventory_probe(&format!("{label}-mutant"), keys, Some(inject))
        .unwrap_or_else(|e| panic!("{label}: the MUTANT could not be built or run: {e}"))
}

/// Every input any inventory mutant uses.
const INVENTORY_MUTANT_INPUTS: &[&str] = &[
    "seed-tiff_ifd/exif-gps-interop",
    "fixtures/exif-basic-le",
    "fixtures/exif-gps-full-le",
    "fixtures/tiff-chain-four",
    "fixtures/jpeg-exif-after-scan",
    "fixtures/jpeg-exif-before-and-after-scan",
    "fixtures/jpeg-iptc",
    "fixtures/jpeg-xmp",
];

#[test]
fn inventory_probe_control_run_is_clean_before_every_mutant() {
    // Every distinct input of every corpus except the one ExifTool never
    // returns on (its answer is not recorded, so there is nothing to compare).
    let all = inventory_corpus::all_inputs(&root());
    let distinct = inventory_corpus::distinct(&all);
    assert_eq!(distinct.len(), 180);
    let keys: Vec<String> = distinct
        .iter()
        .filter(|d| !inventory_corpus::is_not_run(d))
        .map(|d| d.canonical.key())
        .collect();
    assert_eq!(keys.len(), 179);
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    assert_inventory_control("inventory-all", &refs);
    // The mutants' inputs are among them.
    for used in INVENTORY_MUTANT_INPUTS {
        assert!(keys.iter().any(|k| k == used), "{used} is not an input");
    }
}

// Mutant i1. Needle (src/audit/mod.rs, `Builder::add_container`):
//   let has_jpeg_pointer = ifd.has(TAG_JPEG_OFFSET);
// The Interop IFD's entries are skipped: the inventory loses them.
#[test]
fn inventory_mutant_i1_interop_skipped_is_named() {
    let key = "seed-tiff_ifd/exif-gps-interop";
    let bytes = inv_input(key).bytes;
    let want: Multiset = inventory::ours(&exif_core::audit::audit(&bytes))
        .tags
        .into_iter()
        .filter(|((g, _), _)| g == "InteropIFD")
        .collect();
    assert!(!want.is_empty(), "the fixture has Interop entries");
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("audit/mod.rs"),
            "let has_jpeg_pointer = ifd.has(TAG_JPEG_OFFSET);",
            &marked(
                "if ifd.kind() == IfdKind::Interop { continue; } \
                 let has_jpeg_pointer = ifd.has(TAG_JPEG_OFFSET);",
            ),
        );
    };
    let all = inventory_control_then_mutant("inventory-i1", &[key], &inject);
    let d = inv_diff(&all, key);
    assert_eq!(
        d.theirs_only, want,
        "exactly the Interop entries are missing"
    );
    assert!(d.ours_only.is_empty() && d.regions_ours_only.is_empty());
    assert!(d.to_string().contains("theirs only: InteropIFD"), "{d}");
}

// Mutant i2. Needle (src/audit/mod.rs, `Builder::add_container`):
//   let tag = entry.tag();
// The tag id is stored masked to its low byte.
#[test]
fn inventory_mutant_i2_masked_tag_id_is_named() {
    let key = "fixtures/exif-basic-le";
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("audit/mod.rs"),
            "let tag = entry.tag();",
            &marked("let tag = entry.tag() & 0x00FF;"),
        );
    };
    let all = inventory_control_then_mutant("inventory-i2", &[key], &inject);
    let d = inv_diff(&all, key);
    // Make (0x010f) is read as 0x000f: both sides name it.
    assert_eq!(d.theirs_only.get(&("IFD0".to_string(), 0x010f)), Some(&1));
    assert_eq!(d.ours_only.get(&("IFD0".to_string(), 0x000f)), Some(&1));
    // And every id we list that ExifTool does not is now a single byte.
    assert!(d.ours_only.keys().all(|(_, id)| *id <= 0x00FF), "{d}");
}

// Mutant i3. Needle (src/audit/mod.rs, `Builder::add_container`):
//   ifd: ifd.kind(),
// GPS items are labelled Exif.
#[test]
fn inventory_mutant_i3_gps_labelled_exif_is_named() {
    let key = "fixtures/exif-gps-full-le";
    let bytes = inv_input(key).bytes;
    let gps: Vec<(u16, usize)> = inventory::ours(&exif_core::audit::audit(&bytes))
        .tags
        .iter()
        .filter(|((g, _), _)| g == "GPS")
        .map(|((_, id), n)| (*id, *n))
        .collect();
    assert!(!gps.is_empty());
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("audit/mod.rs"),
            "ifd: ifd.kind(),",
            &marked("ifd: if ifd.kind() == IfdKind::Gps { IfdKind::Exif } else { ifd.kind() },"),
        );
    };
    let all = inventory_control_then_mutant("inventory-i3", &[key], &inject);
    let d = inv_diff(&all, key);
    let ours_only: Multiset = gps
        .iter()
        .map(|&(id, n)| (("ExifIFD".to_string(), id), n))
        .collect();
    let theirs_only: Multiset = gps
        .iter()
        .map(|&(id, n)| (("GPS".to_string(), id), n))
        .collect();
    assert_eq!(d.ours_only, ours_only);
    assert_eq!(d.theirs_only, theirs_only);
}

// Mutant i4. Needle (src/audit/mod.rs, `audit_jpeg`):
//   exif_issues = exif_issues.saturating_add(container.issues().len());
// Exif blocks after the scan are dropped from the inventory. ExifTool reads
// none of them either, so on `jpeg-exif-after-scan` the pinned divergence
// DISAPPEARS: the comparison goes red as a divergence that stopped being one.
#[test]
fn inventory_mutant_i4_after_scan_blocks_dropped_makes_the_divergence_vanish() {
    let keys = [
        "fixtures/jpeg-exif-after-scan",
        "fixtures/jpeg-exif-before-and-after-scan",
    ];
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("audit/mod.rs"),
            "exif_issues = exif_issues.saturating_add(container.issues().len());",
            &marked(
                "if after_scan(block.segment()) { continue; } \
                 exif_issues = exif_issues.saturating_add(container.issues().len());",
            ),
        );
    };
    let all = inventory_control_then_mutant("inventory-i4", &keys, &inject);
    for key in keys {
        let d = inv_diff(&all, key);
        // The pinned difference is gone: with the block dropped our inventory
        // equals ExifTool's.
        assert!(d.is_empty(), "{key}: {d}");
        let err = inventory::check_against(inventory::KNOWN_DIVERGENCES, key, &d, true)
            .expect_err("a divergence that stopped being one");
        assert_eq!(
            err,
            inventory::CheckError::StoppedDiverging {
                fixture: key.to_string()
            }
        );
        assert!(err.to_string().contains(key), "{err}");
    }
}

// Mutant i5. Needle (src/tiff/mod.rs, `chain_successor`):
//   IfdKind::Ifd0 => Some(IfdKind::Ifd1),
// The IFD after IFD0 is a plain chain IFD: every chain position is off by one.
#[test]
fn inventory_mutant_i5_chain_position_off_by_one_is_named() {
    let key = "fixtures/tiff-chain-four";
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("tiff/mod.rs"),
            "IfdKind::Ifd0 => Some(IfdKind::Ifd1),",
            &marked("IfdKind::Ifd0 => Some(IfdKind::Chain),"),
        );
    };
    let all = inventory_control_then_mutant("inventory-i5", &[key], &inject);
    let d = inv_diff(&all, key);
    assert_eq!(
        d.ours_only,
        set(&[
            ("IFD2", 0x0110, 1),
            ("IFD3", 0x0131, 1),
            ("IFD4", 0x013b, 1)
        ])
    );
    assert_eq!(
        d.theirs_only,
        set(&[
            ("IFD1", 0x0110, 1),
            ("IFD2", 0x0131, 1),
            ("IFD3", 0x013b, 1)
        ])
    );
}

// Mutant i6. Needle (src/audit/mod.rs, `audit_jpeg`):
//   Region::Iptc { segment, .. } => (RegionKind::Iptc, segment),
// An IPTC region is reported as XMP.
#[test]
fn inventory_mutant_i6_iptc_region_as_xmp_is_named() {
    let keys = ["fixtures/jpeg-iptc", "fixtures/jpeg-xmp"];
    let inject = |src: &Path| {
        probe::inject_replace(
            &src.join("audit/mod.rs"),
            "Region::Iptc { segment, .. } => (RegionKind::Iptc, segment),",
            &marked("Region::Iptc { segment, .. } => (RegionKind::Xmp, segment),"),
        );
    };
    let all = inventory_control_then_mutant("inventory-i6", &keys, &inject);
    let d = inv_diff(&all, "fixtures/jpeg-iptc");
    assert_eq!(d.regions_ours_only, [RegionKind::Xmp].into_iter().collect());
    assert_eq!(
        d.regions_theirs_only,
        [RegionKind::Iptc].into_iter().collect()
    );
    assert!(d.ours_only.is_empty() && d.theirs_only.is_empty());
    // The XMP fixture is unaffected: the fault is specific to IPTC.
    assert!(inv_diff(&all, "fixtures/jpeg-xmp").is_empty());
}
