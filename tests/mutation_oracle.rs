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
//! The probe builds are slower than ordinary tests (each one compiles a small
//! crate); `just mutation-oracle` runs just this file.

#[path = "support/mutate.rs"]
mod mutate;
#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/probe.rs"]
mod probe;
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

/// No `RED-PROOF INJECTION` marker in the library or any other non-test source:
/// an injection can only live in a staged copy.
#[test]
fn no_injection_marker_outside_the_tests() {
    fn walk(dir: &Path, hits: &mut Vec<PathBuf>) {
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
                walk(&path, hits);
            } else if let Ok(text) = std::fs::read_to_string(&path) {
                if text.contains(MARKER) {
                    hits.push(path);
                }
            }
        }
    }
    let mut hits = Vec::new();
    for dir in ["src", "examples", "fuzz", "scripts"] {
        walk(&root().join(dir), &mut hits);
    }
    assert!(hits.is_empty(), "injection marker leaked into: {hits:?}");
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
