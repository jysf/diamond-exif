//! SPEC-020: ExifTool as the oracle for the strip writer's output.
//!
//! For every fixture of `tests/support/strip_shapes.rs` and each recorded
//! policy, ExifTool (`-a -u -G1 -s -t -n`) is run on the library's stripped
//! output. The records (`tests/oracle-fixtures/exiftool-strip/`) are replayed by
//! default and re-recorded by `just oracle-record`; the live tests run the same
//! comparator on fresh ExifTool output when it is installed (they print `SKIP`
//! when it is not; `DIAMOND_EXIF_REQUIRE_ORACLE=1` makes that a failure).
//! ExifTool is run, never read or linked.
//!
//! Group facts measured with ExifTool 13.55 (the table is typed in
//! `strip_records.rs`): `IFD0`, `ExifIFD`, `GPS`, `IFD1` for the Exif IFDs;
//! `XMP-xmp` (extended XMP merges into it) and `XMP-xmpNote`; `ICC-header` and
//! `ICC_Profile`; `IPTC` for the first Photoshop segment and `IPTC2` for the
//! second; `JUMBF` (read only if the segment precedes the Photoshop one);
//! `AROT`; `Adobe`; `JFIF`; `MPF0` and `MPImage1..n` (n = image count, the
//! primary included); the COM text is `File:Comment`.

#[path = "support/exif_layout.rs"]
mod exif_layout;
#[path = "support/exif_records.rs"]
mod exif_records;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/real_shapes.rs"]
mod real_shapes;
#[path = "support/strip_records.rs"]
mod strip_records;
#[path = "support/strip_shapes.rs"]
mod strip_shapes;
#[path = "support/tools.rs"]
mod tools;

use std::path::PathBuf;
use std::sync::OnceLock;

use exif_core::audit::Category;
use exif_core::policy::StripSpec;
use strip_records::{Line, Record, Section};
use strip_shapes::{model, spec_of, Expect, Shape};

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

fn record_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/oracle-fixtures/exiftool-strip")
        .join(format!("{name}.txt"))
}

fn shapes() -> Vec<Shape> {
    strip_shapes::all()
        .into_iter()
        .filter(|s| !s.refused)
        .collect()
}

fn committed(shape: &Shape) -> Record {
    let path = record_path(shape.name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (run `just oracle-record`)",
            path.display()
        )
    });
    strip_records::parse_record(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
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

/// Fresh ExifTool records of every fixture, run once per process (in parallel,
/// one thread per fixture).
fn live_records() -> &'static Vec<(&'static str, String)> {
    static LIVE: OnceLock<Vec<(&'static str, String)>> = OnceLock::new();
    LIVE.get_or_init(|| {
        let version = tools::exiftool_version().expect("exiftool -ver");
        std::thread::scope(|scope| {
            let handles: Vec<_> = shapes()
                .into_iter()
                .map(|s| {
                    let version = version.clone();
                    scope.spawn(move || {
                        let text = strip_records::record_text(&s, &version)
                            .unwrap_or_else(|e| panic!("{}: {e}", s.name));
                        (s.name, text)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("thread"))
                .collect()
        })
    })
}

fn live(shape: &Shape) -> Record {
    let text = &live_records()
        .iter()
        .find(|(n, _)| *n == shape.name)
        .unwrap()
        .1;
    strip_records::parse_record(text).unwrap()
}

/// `(policy name, spec)` of the recorded policies for which the model says the
/// library strips.
fn stripped_policies(shape: &Shape) -> Vec<(&'static str, StripSpec)> {
    strip_records::policies()
        .into_iter()
        .map(|(n, p)| (n, p.spec()))
        .filter(|(_, spec)| matches!(model(shape, *spec), Expect::Ok { .. } | Expect::EntryEdit))
        .collect()
}

/// A comparison of one output section with the table.
type OutputCheck = dyn Fn(&Shape, &str, StripSpec, &Section, &Section) -> Result<(), String>;

/// Run `check` over every (fixture, policy) of a record source.
fn each_output(source: &dyn Fn(&Shape) -> Record, check: &OutputCheck) -> usize {
    let mut n = 0;
    for shape in shapes() {
        let record = source(&shape);
        let input = record.section("input").expect("input section");
        for (name, spec) in stripped_policies(&shape) {
            let section = record
                .section(name)
                .unwrap_or_else(|| panic!("{} has no `{name}` section", shape.name));
            check(&shape, name, spec, section, input)
                .unwrap_or_else(|e| panic!("{} under {name}: {e}", shape.name));
            n += 1;
        }
    }
    n
}

fn groups_check(
    shape: &Shape,
    _: &str,
    spec: StripSpec,
    section: &Section,
    input: &Section,
) -> Result<(), String> {
    strip_records::check_groups(shape, spec, section, input)
}

#[test]
fn exiftool_group_table_equals_the_output_records_per_policy_and_fixture() {
    let n = each_output(&committed, &groups_check);
    assert!(n > 100, "{n} (fixture, policy) pairs replayed");
    println!("replayed {n} outputs");
    if live_gate("exiftool_group_table_equals_the_output_records_per_policy_and_fixture") {
        let n = each_output(&live, &groups_check);
        println!("live: {n} outputs");
    }
}

fn no_new_warnings(
    shape: &Shape,
    _: &str,
    _: StripSpec,
    section: &Section,
    input: &Section,
) -> Result<(), String> {
    let old = strip_records::warnings(&input.lines, &input.stderr);
    let new: Vec<String> = strip_records::warnings(&section.lines, &section.stderr)
        .into_iter()
        .filter(|x| !old.contains(x))
        .collect();
    if new.is_empty() {
        Ok(())
    } else {
        Err(format!("{}: new warnings {new:?}", shape.name))
    }
}

#[test]
fn exiftool_reads_every_output_without_new_warnings() {
    each_output(&committed, &no_new_warnings);
    if live_gate("exiftool_reads_every_output_without_new_warnings") {
        each_output(&live, &no_new_warnings);
    }
}

fn mpf_check(
    shape: &Shape,
    name: &str,
    spec: StripSpec,
    section: &Section,
    input: &Section,
) -> Result<(), String> {
    if name != "all"
        || shape.mpf.is_none()
        || !matches!(
            shape.name,
            "mpo-late-xmp-le" | "mpo-late-xmp-be" | "mpo-trailer-gaps"
        )
    {
        return Ok(());
    }
    strip_records::check_mpf_fields(shape, spec, section, input)
}

#[test]
fn mpf_fields_read_back_as_expected_after_all() {
    // The three fixtures where `All` removes something that precedes an image.
    each_output(&committed, &mpf_check);
    if live_gate("mpf_fields_read_back_as_expected_after_all") {
        each_output(&live, &mpf_check);
    }
    // The bytes at the new positions start FF D8 (checked on the library's
    // output, which is deterministic).
    for s in [
        strip_shapes::mpo_late_xmp(false),
        strip_shapes::mpo_late_xmp(true),
        strip_shapes::mpo_trailer_gaps(),
    ] {
        let out = exif_core::strip(&s.bytes, exif_core::StripPolicy::All).unwrap();
        let a = exif_core::audit::audit(out.bytes());
        let starts: Vec<_> = a
            .findings_in(Category::EmbeddedImage)
            .filter_map(|f| f.embedded)
            .map(|e| e.offset)
            .collect();
        assert_eq!(starts.len(), 2, "{}", s.name);
        for at in starts {
            assert_eq!(&out.bytes()[at..at + 2], &[0xFF, 0xD8], "{}", s.name);
        }
    }
}

fn no_mpf(
    shape: &Shape,
    name: &str,
    _: StripSpec,
    section: &Section,
    _: &Section,
) -> Result<(), String> {
    if name != "everything" || shape.mpf.is_none() {
        return Ok(());
    }
    match section
        .lines
        .iter()
        .find(|l| l.group == "MPF0" || l.group.starts_with("MPImage"))
    {
        None => Ok(()),
        Some(l) => Err(format!("{} survived Everything", l.group)),
    }
}

#[test]
fn no_mpf_groups_after_everything() {
    let n = each_output(&committed, &no_mpf);
    assert!(n > 0);
    if live_gate("no_mpf_groups_after_everything") {
        each_output(&live, &no_mpf);
    }
}

#[test]
fn strip_records_replay_and_reproduce() {
    // Every fixture has a record, whose sections are exactly the input and the
    // policies that strip.
    for shape in shapes() {
        let record = committed(&shape);
        let mut want: Vec<&str> = vec!["input"];
        want.extend(stripped_policies(&shape).iter().map(|(n, _)| *n));
        let got: Vec<&str> = record.sections.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(got, want, "{}", shape.name);
    }
    // The Exif entry records (SPEC-021): the input and every policy of the
    // grid that strips.
    for (name, _) in exif_records::exif_fixtures() {
        let record = exif_committed(name);
        let mut want: Vec<&str> = vec!["input"];
        want.extend(strip_records::exif_stripped_policies(name));
        let got: Vec<&str> = record.sections.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(got, want, "exif-{name}");
    }
    // Live: ExifTool's answer today is the committed text, byte for byte
    // (same version).
    if live_gate("strip_records_replay_and_reproduce") {
        for (name, text) in live_records() {
            let path = record_path(name);
            let have = std::fs::read_to_string(&path).unwrap();
            assert_eq!(&have, text, "{name}: re-record with `just oracle-record`");
        }
        for (name, text) in live_exif_records() {
            let have = std::fs::read_to_string(exif_record_path(name)).unwrap();
            assert_eq!(
                &have, text,
                "exif-{name}: re-record with `just oracle-record`"
            );
        }
    }
}

// ── SPEC-021: Exif entry edits at tag level ─────────────────────────────────

fn exif_record_path(name: &str) -> PathBuf {
    record_path(&format!("exif-{name}"))
}

fn exif_committed(name: &str) -> Record {
    let path = exif_record_path(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (run `just oracle-record`)",
            path.display()
        )
    });
    strip_records::parse_record(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Fresh ExifTool records of every Exif fixture, once per process.
fn live_exif_records() -> &'static Vec<(&'static str, String)> {
    static LIVE: OnceLock<Vec<(&'static str, String)>> = OnceLock::new();
    LIVE.get_or_init(|| {
        let version = tools::exiftool_version().expect("exiftool -ver");
        std::thread::scope(|scope| {
            let handles: Vec<_> = exif_records::exif_fixtures()
                .into_iter()
                .map(|(name, layouts)| {
                    let version = version.clone();
                    scope.spawn(move || {
                        let jpg = exif_records::jpeg_of(&layouts, false);
                        let text = strip_records::exif_record_text(name, &jpg, &version)
                            .unwrap_or_else(|e| panic!("{name}: {e}"));
                        (name, text)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("thread"))
                .collect()
        })
    })
}

fn exif_live(name: &str) -> Record {
    let text = &live_exif_records()
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap()
        .1;
    strip_records::parse_record(text).unwrap()
}

type ExifCheck =
    dyn Fn(&[exif_layout::Layout], &[exif_layout::Cat], &Section, &Section) -> Result<(), String>;

/// Run `check` over every (fixture, policy) of the Exif grid that strips.
fn each_exif_output(source: &dyn Fn(&str) -> Record, check: &ExifCheck) -> usize {
    let mut n = 0;
    for (name, layouts) in exif_records::exif_fixtures() {
        let record = source(name);
        let input = record.section("input").expect("input section");
        for (pname, cats, _) in exif_records::exif_policies() {
            if exif_records::model_refusal(name, cats).is_some() {
                continue;
            }
            let section = record
                .section(pname)
                .unwrap_or_else(|| panic!("exif-{name} has no `{pname}` section"));
            check(&layouts, cats, section, input)
                .unwrap_or_else(|e| panic!("exif-{name} under {pname}: {e}"));
            n += 1;
        }
    }
    n
}

#[test]
fn exiftool_tag_sets_equal_the_declared_tables_per_policy_and_fixture() {
    // Control: the hand table reproduces ExifTool's own reading of every input.
    for (name, layouts) in exif_records::exif_fixtures() {
        let record = exif_committed(name);
        let input = record.section("input").unwrap();
        let none: Vec<_> = layouts
            .iter()
            .map(|_| exif_layout::Del::default())
            .collect();
        let want = strip_records::exif_declared(&layouts, &none);
        let got: Vec<(String, String)> = input
            .lines
            .iter()
            .filter(|l| {
                [
                    "IFD0",
                    "IFD1",
                    "IFD2",
                    "ExifIFD",
                    "InteropIFD",
                    "GPS",
                    "SubIFD",
                ]
                .contains(&l.group.as_str())
            })
            .map(|l| (l.group.clone(), l.tag.clone()))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let want_set: Vec<(String, String)> = want
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(got, want_set, "exif-{name}: the table against the input");
    }
    let n = each_exif_output(&exif_committed, &|l, c, s, i| {
        strip_records::check_exif_tags(l, c, s, i)
    });
    assert!(n > 100, "{n} (fixture, policy) pairs replayed");
    if live_gate("exiftool_tag_sets_equal_the_declared_tables_per_policy_and_fixture") {
        each_exif_output(&exif_live, &|l, c, s, i| {
            strip_records::check_exif_tags(l, c, s, i)
        });
    }
}

#[test]
fn exiftool_values_of_kept_tags_equal_the_inputs() {
    each_exif_output(&exif_committed, &|l, c, s, i| {
        strip_records::check_exif_values(l, c, s, i)
    });
    if live_gate("exiftool_values_of_kept_tags_equal_the_inputs") {
        each_exif_output(&exif_live, &|l, c, s, i| {
            strip_records::check_exif_values(l, c, s, i)
        });
    }
}

#[test]
fn exiftool_reads_every_output_without_new_warning_kinds() {
    each_exif_output(&exif_committed, &|_, _, s, i| {
        strip_records::check_exif_warnings(s, i)
    });
    if live_gate("exiftool_reads_every_output_without_new_warning_kinds_exif") {
        each_exif_output(&exif_live, &|_, _, s, i| {
            strip_records::check_exif_warnings(s, i)
        });
    }
    // The input carries the warnings the MakerNote and the XMP tag cause; the
    // outputs that keep them keep the same kinds, those that drop them lose
    // them (never a new one).
    let full = exif_committed("full-le");
    let input = full.section("input").unwrap();
    assert!(
        strip_records::warnings(&input.lines, &input.stderr)
            .iter()
            .any(|w| w.contains("MakerNotes")),
        "the fixture's own warning"
    );
    let all_pii = full.section("all_pii").unwrap();
    assert!(strip_records::warnings(&all_pii.lines, &all_pii.stderr).is_empty());
}

/// Rule 9, measured with ExifTool 13.55: an IFD left with no entry (count 0)
/// reads without a warning, prints no tag in its group, and the byte order
/// survives; the library's walk calls the output clean and the writer allows it.
fn emptied_ifd_check(record: &Record) {
    let input = record.section("input").unwrap();
    let out = record.section("all_pii").unwrap();
    // The input has the three IFD0 tags; the output none, in any Exif group.
    assert_eq!(input.lines.iter().filter(|l| l.group == "IFD0").count(), 3);
    assert!(out.lines.iter().all(|l| !exif_group_line(&l.group)));
    // The byte order is still read, no warning of any kind appears.
    assert!(out
        .lines
        .iter()
        .any(|l| l.group == "File" && l.tag == "ExifByteOrder" && l.value == "II"));
    assert!(strip_records::warnings(&out.lines, &out.stderr).is_empty());
    assert!(!out
        .lines
        .iter()
        .any(|l| l.tag == "Warning" || l.tag == "Error"));
}

fn exif_group_line(g: &str) -> bool {
    [
        "IFD0",
        "IFD1",
        "IFD2",
        "ExifIFD",
        "InteropIFD",
        "GPS",
        "SubIFD",
    ]
    .contains(&g)
}

#[test]
fn emptied_ifd_behaviour_is_pinned() {
    emptied_ifd_check(&exif_committed("every-entry-target"));
    if live_gate("emptied_ifd_behaviour_is_pinned") {
        emptied_ifd_check(&exif_live("every-entry-target"));
    }
    // The library: the walk of the output is clean (an empty IFD0 is no issue),
    // the policy's categories are gone and stripping again is a no-op.
    let j = exif_records::jpeg_of(&[exif_layout::every_entry_target().layout()], false);
    let out = exif_core::strip(&j.bytes, exif_core::StripPolicy::AllPii).unwrap();
    let m = exif_core::jpeg::read(out.bytes()).unwrap();
    assert!(m.is_clean());
    assert_eq!(m.exif().unwrap().entry_count(), 0);
    assert_eq!(out.entries().len(), 3);
    let again = exif_core::strip(out.bytes(), exif_core::StripPolicy::AllPii).unwrap();
    assert_eq!(again.bytes(), out.bytes());
}

// ── red-proofs of the tag-level comparator ──────────────────────────────────

fn full_record() -> (Vec<exif_layout::Layout>, Record) {
    (
        vec![exif_layout::full_le().layout()],
        exif_committed("full-le"),
    )
}

fn red_proof(
    policy: &str,
    cats: &[exif_layout::Cat],
    plant: &dyn Fn(&mut Section, &Section),
) -> String {
    let (layouts, record) = full_record();
    let input = record.section("input").unwrap();
    let mut out = record.section(policy).unwrap().clone();
    // Control: the genuine record passes every comparison.
    strip_records::check_exif_section(&layouts, cats, &out, input).expect("control");
    plant(&mut out, input);
    strip_records::check_exif_section(&layouts, cats, &out, input).unwrap_err()
}

fn line(group: &str, tag: &str, value: &str) -> Line {
    Line {
        group: group.into(),
        tag: tag.into(),
        value: value.into(),
    }
}

#[test]
fn comparator_is_red_when_a_deleted_tag_is_left() {
    let e = red_proof("all_pii", exif_records::ALL_PII, &|out, _| {
        out.lines.push(line("IFD0", "Make", "CANARY-MAKE-0001"));
    });
    assert!(e.contains("extra") && e.contains("Make"), "{e}");
}

#[test]
fn comparator_is_red_when_a_kept_tag_is_missing() {
    let e = red_proof("gps", &["gps"], &|out, _| {
        out.lines
            .retain(|l| !(l.group == "IFD0" && l.tag == "Orientation"));
    });
    assert!(e.contains("missing") && e.contains("Orientation"), "{e}");
}

#[test]
fn comparator_is_red_when_the_gps_group_is_left() {
    let e = red_proof("gps", &["gps"], &|out, input| {
        out.lines
            .extend(input.lines.iter().filter(|l| l.group == "GPS").cloned());
    });
    assert!(e.contains("GPS"), "{e}");
}

#[test]
fn comparator_is_red_when_a_kept_value_changed() {
    let e = red_proof("gps", &["gps"], &|out, _| {
        for l in &mut out.lines {
            if l.group == "IFD0" && l.tag == "Make" {
                l.value = "CANARY-MAKE-0042".into();
            }
        }
    });
    assert!(e.contains("values differ") && e.contains("Make"), "{e}");
}

#[test]
fn comparator_is_red_when_the_thumbnail_image_is_left() {
    let e = red_proof("thumbnail", &["thumbnail"], &|out, input| {
        out.lines.extend(
            input
                .lines
                .iter()
                .filter(|l| l.group == "IFD1" && l.tag == "ThumbnailImage")
                .cloned(),
        );
    });
    assert!(e.contains("ThumbnailImage"), "{e}");
}

#[test]
fn comparator_is_red_when_a_phantom_tag_is_added() {
    let e = red_proof("gps", &["gps"], &|out, _| {
        out.lines.push(line("IFD0", "Phantom", "1"));
    });
    assert!(e.contains("Phantom"), "{e}");
}

#[test]
fn comparator_is_red_when_a_declared_loss_is_not_declared() {
    // The Composite GPS tags go with the GPS group; one left behind is caught.
    let e = red_proof("gps", &["gps"], &|out, input| {
        out.lines.extend(
            input
                .lines
                .iter()
                .filter(|l| l.group == "Composite" && l.tag == "GPSPosition")
                .cloned(),
        );
    });
    assert!(e.contains("Composite"), "{e}");
}

// ── red-proofs: the comparator must fail on its own ─────────────────────────

fn sink() -> (Shape, Record) {
    let s = strip_shapes::kitchen_sink();
    let r = committed(&s);
    (s, r)
}

fn all_spec() -> StripSpec {
    exif_core::StripPolicy::All.spec()
}

#[test]
fn comparator_is_red_when_an_xmp_group_is_left_in() {
    let (s, r) = sink();
    let input = r.section("input").unwrap();
    let mut out = r.section("all").unwrap().clone();
    // Control: the genuine record passes.
    strip_records::check_groups(&s, all_spec(), &out, input).expect("control");
    out.lines.push(Line {
        group: "XMP-xmp".into(),
        tag: "CreatorTool".into(),
        value: "left behind".into(),
    });
    let e = strip_records::check_groups(&s, all_spec(), &out, input).unwrap_err();
    assert!(e.contains("XMP-xmp"), "{e}");
}

#[test]
fn comparator_is_red_when_icc_is_missing() {
    let (s, r) = sink();
    let input = r.section("input").unwrap();
    let mut out = r.section("all").unwrap().clone();
    strip_records::check_groups(&s, all_spec(), &out, input).expect("control");
    out.lines.retain(|l| !l.group.starts_with("ICC"));
    let e = strip_records::check_groups(&s, all_spec(), &out, input).unwrap_err();
    assert!(e.contains("ICC_Profile") && e.contains("missing"), "{e}");
}

#[test]
fn comparator_is_red_when_an_mp_image_start_is_off_by_the_removed_length() {
    let s = strip_shapes::mpo_late_xmp(false);
    let r = committed(&s);
    let input = r.section("input").unwrap();
    let mut out = r.section("all").unwrap().clone();
    strip_records::check_mpf_fields(&s, all_spec(), &out, input).expect("control");
    // The index of the unpatched file: image 2 still says where it was before
    // the XMP and the COM were removed.
    let late = (s.parts_of(strip_shapes::K::Xmp)[0].len()
        + s.parts_of(strip_shapes::K::Com)[0].len()) as i64;
    for l in &mut out.lines {
        if l.group == "MPImage2" && l.tag == "MPImageStart" {
            let v: i64 = l.value.parse().unwrap();
            l.value = (v + late).to_string();
        }
    }
    let e = strip_records::check_mpf_fields(&s, all_spec(), &out, input).unwrap_err();
    assert!(e.contains("MPImage2 MPImageStart"), "{e}");
}

#[test]
fn comparator_is_red_when_a_phantom_group_is_added() {
    let (s, r) = sink();
    let input = r.section("input").unwrap();
    let mut out = r.section("everything").unwrap().clone();
    let spec = exif_core::StripPolicy::Everything.spec();
    strip_records::check_groups(&s, spec, &out, input).expect("control");
    out.lines.push(Line {
        group: "Phantom".into(),
        tag: "Tag".into(),
        value: "1".into(),
    });
    let e = strip_records::check_groups(&s, spec, &out, input).unwrap_err();
    assert!(e.contains("Phantom"), "{e}");
}

#[test]
fn comparator_is_red_when_the_exif_segment_was_kept() {
    let (s, r) = sink();
    let input = r.section("input").unwrap();
    // A stripped file whose Exif segment was kept reads, in ExifTool, like the
    // input in the Exif groups: splice them into the genuine `All` record.
    let mut out = r.section("all").unwrap().clone();
    strip_records::check_groups(&s, all_spec(), &out, input).expect("control");
    out.lines.extend(
        input
            .lines
            .iter()
            .filter(|l| matches!(l.group.as_str(), "IFD0" | "ExifIFD" | "GPS" | "IFD1"))
            .cloned(),
    );
    let e = strip_records::check_groups(&s, all_spec(), &out, input).unwrap_err();
    assert!(e.contains("IFD0") || e.contains("GPS"), "{e}");
    // Also: the whole input read as if it were the output.
    let e = strip_records::check_groups(&s, all_spec(), input, input).unwrap_err();
    assert!(e.contains("extra"), "{e}");
    let _ = spec_of(&[]);
}
