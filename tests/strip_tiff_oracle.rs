//! SPEC-022: ExifTool as the oracle for the TIFF and DNG strip's output.
//!
//! Records in `tests/oracle-fixtures/exiftool-strip-tiff/` (one file per
//! fixture: an `## input` section and one per recorded policy that strips) are
//! replayed by default and re-recorded by `just oracle-record`; the live test
//! runs the same comparator on fresh ExifTool output and SKIPs loudly without
//! ExifTool (`DIAMOND_EXIF_REQUIRE_ORACLE=1` makes the skip a failure).
//! ExifTool is run, never read or linked.

#[path = "support/tiff_records.rs"]
mod tiff_records;
#[path = "support/tiff_shapes.rs"]
mod tiff_shapes;
#[path = "support/tools.rs"]
mod tools;

use std::path::PathBuf;

use exif_core::policy::StripSpec;
use tiff_records::{check, parse_record, policies, Section};

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

fn record_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/oracle-fixtures/exiftool-strip-tiff")
        .join(format!("{name}.txt"))
}

fn section<'a>(r: &'a [Section], name: &str) -> &'a Section {
    r.iter().find(|s| s.name == name).unwrap()
}

fn each(records: &dyn Fn(&tiff_shapes::Shape) -> Vec<Section>) -> usize {
    let mut n = 0;
    for shape in tiff_shapes::all() {
        let rec = records(&shape);
        let input = section(&rec, "input");
        for (name, policy) in policies() {
            if exif_core::strip(&shape.bytes, policy).is_err() {
                assert!(rec.iter().all(|s| s.name != name), "{} {name}", shape.name);
                continue;
            }
            check(policy.spec(), section(&rec, name), input)
                .unwrap_or_else(|e| panic!("{} under {name}: {e}", shape.name));
            n += 1;
        }
    }
    n
}

fn replay(shape: &tiff_shapes::Shape) -> Vec<Section> {
    let path = record_path(&shape.name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (run `just oracle-record`)",
            path.display()
        )
    });
    parse_record(&text)
}

#[test]
fn recorded_exiftool_agrees_with_every_stripped_output() {
    assert!(each(&replay) > 20);
}

#[test]
fn live_exiftool_agrees_with_every_stripped_output() {
    if !tools::exiftool_available() {
        if std::env::var(REQUIRE_VAR).as_deref() == Ok("1") {
            panic!("exiftool is not on PATH and {REQUIRE_VAR}=1 makes that a failure");
        }
        println!("SKIP live_exiftool_agrees_with_every_stripped_output - exiftool not on PATH");
        return;
    }
    let version = tools::exiftool_version().unwrap();
    let n = each(&|s| parse_record(&tiff_records::record_text(s, &version).unwrap()));
    assert!(n > 20);
}

/// Red-proof of the comparator: each planted fault must make it fail.
#[test]
fn the_comparator_goes_red_on_planted_faults() {
    let shape = tiff_shapes::stripped(false);
    let rec = replay(&shape);
    let input = section(&rec, "input");
    let gps = section(&rec, "gps");
    let spec = exif_core::StripPolicy::Gps.spec();
    check(spec, gps, input).unwrap();
    // 1. A GPS line left in the output.
    let mut leaked = gps.clone();
    leaked.lines.push(tiff_records::Line {
        group: "GPS".into(),
        tag: "GPSLatitude".into(),
        value: "52".into(),
    });
    assert!(check(spec, &leaked, input).is_err());
    // 2. A StripOffsets value changed.
    let mut moved = gps.clone();
    for l in moved.lines.iter_mut().filter(|l| l.tag == "StripOffsets") {
        l.value.push_str(" 1");
    }
    assert!(check(spec, &moved, input).is_err());
    // 3. A kept tag lost (Make under a GPS-only policy).
    let mut lost = gps.clone();
    lost.lines.retain(|l| l.tag != "Make");
    assert!(check(spec, &lost, input).is_err());
    // 4. The input itself is not a stripped output.
    assert!(check(spec, input, input).is_err());
    let _ = StripSpec::none();
}
