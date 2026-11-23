//! SPEC-037 verify M1: independence of facts. The strip tests compare the output
//! with a model written from DEC-021; this file asks ExifTool instead. It strips
//! a set of PNGs under all five presets, runs ExifTool on the OUTPUT and fails if
//! it still prints a tag the preset removes. It is the cheap precursor of
//! SPEC-039's formal I3 (ExifTool write parity) and is kept small on purpose.
//!
//! Live only, never replayed: a recorded answer about a file the strip produced
//! would be a record of this library's own output. Without ExifTool each test
//! prints `SKIP` and passes, unless `DIAMOND_EXIF_REQUIRE_ORACLE=1` (as its
//! siblings). ExifTool is run (`-a -u -G1 -H -s`, 8 s limit), never read or linked.
//!
//! The pinned tag list, per preset (DEC-009 columns, DEC-021 clause 2 and 4):
//!
//! * every preset: any tag in the `GPS` group and any `Composite` tag whose name
//!   starts with `GPS` (`GPSPosition`, `GPSLatitude`, ...);
//! * `GpsAndDevice` and up: `SerialNumber`, `BodySerialNumber`,
//!   `InternalSerialNumber`, `LensSerialNumber` (DeviceId);
//! * `AllPii`, `All`, `Everything` (Authorship): `Author`, `Artist`,
//!   `Copyright`, `OwnerName`, `CameraOwnerName`;
//! * the carriers (every preset): any tag from a decoded carrier chunk, that is
//!   any tag in the groups `IFD0`, `IFD1`, `ExifIFD`, `GPS`, `InteropIFD` or one
//!   starting `XMP`, `XML` or `SEAL` (the carrier files hold nothing else).
//!
//! Each check is also shown red: [`the_check_goes_red_on_an_unstripped_file`]
//! runs it over the input itself and requires violations, and every file here
//! must print at least one tag the preset removes before the strip (no vacuous
//! pass).

#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_png.rs"]
mod oracle_png;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/tools.rs"]
mod tools;

use exif_core::policy::StripPolicy;
use exif_core::strip;
use oracle_tiff as t;
use std::path::Path;
use std::time::Duration;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

/// The `-a -u -G1 -H -s` run, the same arguments as the audit oracle.
const ARGS: [&str; 5] = ["-a", "-u", "-G1", "-H", "-s"];

const DEVICE_TAGS: [&str; 4] = [
    "SerialNumber",
    "BodySerialNumber",
    "InternalSerialNumber",
    "LensSerialNumber",
];
// The library files the camera owner under Authorship (`category.rs`, 0xA430).
const AUTHORSHIP_TAGS: [&str; 5] = [
    "Author",
    "Artist",
    "Copyright",
    "OwnerName",
    "CameraOwnerName",
];
const CARRIER_GROUPS: [&str; 5] = ["IFD0", "IFD1", "ExifIFD", "GPS", "InteropIFD"];

/// The representative `oracle_png` fixtures (names are the committed ones).
const FIXTURES: [&str; 11] = [
    "png-exif-gps-ne-le",
    "png-exif-gps-sw-be",
    "png-exif-gps-full-le",
    "png-exif-basic-le",
    "png-exif-after-iend",
    "png-exif-prefix",
    "png-two-exif-same-tag",
    "png-xmp-itxt",
    "png-xmp-ztxt",
    "png-iccp-valid",
    "png-exif-bad-crc",
];

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Tag {
    group: String,
    name: String,
}

/// The tags of an `-a -u -G1 -H -s` listing: `[Group] 0xHHHH Name : value`.
fn parse(stdout: &str) -> Vec<Tag> {
    stdout
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix('[')?;
            let (group, rest) = rest.split_once(']')?;
            let (left, _value) = rest.split_once(" : ").or_else(|| rest.split_once(':'))?;
            let name = left.split_whitespace().last()?;
            Some(Tag {
                group: group.to_string(),
                name: name.to_string(),
            })
        })
        .collect()
}

fn listing(bytes: &[u8], label: &str) -> Vec<Tag> {
    let path = tools::temp_file_with_ext(label, "png", bytes).expect("temp file");
    let out = run(&path);
    let _ = std::fs::remove_file(&path);
    out
}

fn run(path: &Path) -> Vec<Tag> {
    let args: Vec<String> = ARGS.iter().map(|s| (*s).to_string()).collect();
    let out = tools::run_tool_with_limit("exiftool", &args, path, Duration::from_secs(8))
        .unwrap_or_else(|e| panic!("exiftool on {}: {e:?}", path.display()));
    parse(&String::from_utf8_lossy(&out.stdout))
}

/// The pinned list: does `preset` (index into `StripPolicy::PRESETS`) remove
/// this tag?
fn removed_by(preset: usize, tag: &Tag) -> bool {
    let gps = tag.group == "GPS" || (tag.group == "Composite" && tag.name.starts_with("GPS"));
    let device = preset >= 1 && DEVICE_TAGS.contains(&tag.name.as_str());
    let authorship = preset >= 2 && AUTHORSHIP_TAGS.contains(&tag.name.as_str());
    gps || device || authorship
}

fn is_carrier_tag(tag: &Tag) -> bool {
    CARRIER_GROUPS.contains(&tag.group.as_str())
        || tag.group.starts_with("XMP")
        || tag.group.starts_with("XML")
        || tag.group.starts_with("SEAL")
}

/// The tags of `listing` the preset must have removed.
fn violations(preset: usize, carrier: bool, listing: &[Tag]) -> Vec<String> {
    listing
        .iter()
        .filter(|t| removed_by(preset, t) || (carrier && is_carrier_tag(t)))
        .map(|t| format!("{}:{}", t.group, t.name))
        .collect()
}

/// Strip `input` under every preset, ExifTool the output, and return the
/// failures; also count the checks that were not vacuous.
fn check(
    name: &str,
    input: &[u8],
    carrier: bool,
    strict: bool,
    checked: &mut usize,
) -> Vec<String> {
    let before = listing(input, name);
    let mut failures = Vec::new();
    for (i, policy) in StripPolicy::PRESETS.iter().enumerate() {
        let had = violations(i, carrier, &before);
        assert!(
            !strict || !had.is_empty(),
            "{name}: ExifTool prints nothing the {} preset removes, so the check would pass \
             vacuously (listing: {before:?})",
            policy.name()
        );
        let out = strip(input, *policy).unwrap_or_else(|e| panic!("{name} {policy:?}: {e:?}"));
        let after = listing(out.bytes(), name);
        let left = violations(i, carrier, &after);
        *checked += had.len();
        if !left.is_empty() {
            failures.push(format!(
                "{name} under {}: still prints {left:?}",
                policy.name()
            ));
        }
    }
    failures
}

// ── the inputs ──────────────────────────────────────────────────────────────

/// A TIFF with a GPS position, a body serial, an owner name, an artist and a
/// copyright: every tag family the pinned list names, in the three IFDs.
fn rich_tiff() -> Vec<u8> {
    let big = false;
    let ifd0 = t::ifd(vec![
        t::ascii(0x010F, b"MakeCorp\0"),
        t::ascii(0x013B, b"Artist Zed\0"),
        t::ascii(0x8298, b"Copy Zed\0"),
        t::ptr(0x8769, 1),
        t::ptr(0x8825, 2),
    ]);
    let exif = t::ifd(vec![
        t::ascii(0xA430, b"Owner Zed\0"),
        t::ascii(0xA431, b"SN123456\0"),
    ]);
    let gps = t::ifd(vec![
        t::ascii(0x0001, b"N\0"),
        t::rational(big, 0x0002, &[(51, 1), (30, 1), (0, 1)]),
        t::ascii(0x0003, b"E\0"),
        t::rational(big, 0x0004, &[(0, 1), (7, 1), (0, 1)]),
    ]);
    t::assemble(big, &[ifd0, exif, gps])
}

fn zlib_wrapped(tiff: &[u8]) -> Vec<u8> {
    let mut d = vec![0];
    d.extend_from_slice(&(tiff.len() as u32).to_be_bytes());
    d.extend_from_slice(&oracle_png::zlib_stored(tiff));
    d
}

fn xmp_with_gps() -> Vec<u8> {
    br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:exif="http://ns.adobe.com/exif/1.0/" exif:GPSLatitude="51,30.0N" exif:GPSLongitude="0,7.0E"><dc:creator><rdf:Seq><rdf:li>Zed</rdf:li></rdf:Seq></dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>"#.to_vec()
}

/// A minimal PNG around the given chunks (before `IDAT`).
fn with(chunks: &[Vec<u8>]) -> Vec<u8> {
    oracle_png::png(chunks, &[], &[])
}

/// The 16 ASCII-case spellings of a four-letter type.
fn spellings(base: [u8; 4]) -> Vec<[u8; 4]> {
    (0..16u8)
        .map(|mask| {
            let mut ty = base;
            for (i, b) in ty.iter_mut().enumerate() {
                if (mask >> i) & 1 == 1 {
                    *b ^= 0x20;
                }
            }
            ty
        })
        .collect()
}

/// `eXIf` and `zxIf` in every spelling but the exact `eXIf`, each as a bare TIFF
/// and as `\0`, a length and a zlib stream (the case grid).
fn case_grid() -> Vec<(String, Vec<u8>)> {
    let tiff = rich_tiff();
    let mut out = Vec::new();
    for base in [*b"eXIf", *b"zxIf"] {
        for ty in spellings(base) {
            if &ty == b"eXIf" {
                continue;
            }
            for (enc, data) in [("bare", tiff.clone()), ("zlib", zlib_wrapped(&tiff))] {
                let name = format!("grid-{}-{enc}", String::from_utf8_lossy(&ty));
                out.push((name, with(&[oracle_png::chunk(&ty, &data)])));
            }
        }
    }
    out
}

/// The opaque carriers ExifTool decodes: a file that holds only the carrier.
fn carrier_shapes() -> Vec<(String, Vec<u8>)> {
    let tiff = rich_tiff();
    let xmp = xmp_with_gps();
    let xml = b"<?xml version=\"1.0\"?><a><b>Zed</b></a>".to_vec();
    let raw = oracle_png::raw_profile_body("exif", &tiff);
    vec![
        (
            "carrier-zxIf".to_string(),
            with(&[oracle_png::chunk(b"zxIf", &zlib_wrapped(&tiff))]),
        ),
        (
            "carrier-tXMP".to_string(),
            with(&[oracle_png::chunk(b"tXMP", &xmp)]),
        ),
        (
            "carrier-meTa".to_string(),
            with(&[oracle_png::chunk(b"meTa", &xml)]),
        ),
        (
            "carrier-seAl".to_string(),
            with(&[oracle_png::chunk(b"seAl", &xml)]),
        ),
        (
            "carrier-raw-profile-text".to_string(),
            with(&[oracle_png::text(b"Raw profile type exif", &raw)]),
        ),
        (
            "carrier-after-iend".to_string(),
            oracle_png::png(&[], &[], &oracle_png::chunk(b"zxIf", &tiff)),
        ),
    ]
}

/// The `oracle_png` fixtures, plus one with every pinned tag in an `eXIf`.
fn fixtures() -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = FIXTURES
        .iter()
        .map(|name| {
            let bytes = oracle_png::fixtures()
                .into_iter()
                .find(|(n, _)| n == name)
                .map(|(_, b)| b)
                .unwrap_or_else(|| panic!("no PNG fixture named {name}"));
            ((*name).to_string(), bytes)
        })
        .collect();
    out.push((
        "rich-exif".to_string(),
        with(&[oracle_png::exif_chunk(&rich_tiff())]),
    ));
    out.push((
        "rich-text-and-exif".to_string(),
        with(&[
            oracle_png::text(b"Author", b"Zed"),
            oracle_png::text(b"Copyright", b"Zed"),
            oracle_png::exif_chunk(&rich_tiff()),
        ]),
    ));
    out
}

/// Run `check` over `items` on four threads.
fn run_all(items: Vec<(String, Vec<u8>)>, carrier: bool) -> (Vec<String>, usize) {
    // A carrier file holds nothing but the carrier, so every preset removes
    // something from it; a fixture may hold nothing the pinned list names.
    let strict = carrier;
    let items = std::sync::Arc::new(items);
    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let (items, next) = (items.clone(), next.clone());
        handles.push(std::thread::spawn(move || {
            let (mut failures, mut checked) = (Vec::new(), 0usize);
            loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some((name, bytes)) = items.get(i) else {
                    break;
                };
                failures.extend(check(name, bytes, carrier, strict, &mut checked));
            }
            (failures, checked)
        }));
    }
    let (mut failures, mut checked) = (Vec::new(), 0);
    for h in handles {
        let (f, c) = h.join().expect("worker");
        failures.extend(f);
        checked += c;
    }
    failures.sort();
    (failures, checked)
}

// ── the tests ───────────────────────────────────────────────────────────────

/// The case grid: ExifTool still decodes every spelling of `eXIf` and `zxIf`,
/// in both encodings, from the input; after a strip under any preset it prints
/// no tag from them.
#[test]
fn the_case_grid_leaves_no_decoded_tag_for_exiftool() {
    if !live_gate("the_case_grid_leaves_no_decoded_tag_for_exiftool") {
        return;
    }
    let grid = case_grid();
    assert_eq!(grid.len(), 31 * 2);
    let (failures, checked) = run_all(grid, true);
    println!("case grid: {checked} non-vacuous tag checks");
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(checked >= 31 * 2 * 5);
}

/// The carrier shapes (`zxIf`, `tXMP`, `meTa`, `seAl`, a raw profile, a carrier
/// after `IEND`).
#[test]
fn the_carrier_shapes_leave_no_decoded_tag_for_exiftool() {
    if !live_gate("the_carrier_shapes_leave_no_decoded_tag_for_exiftool") {
        return;
    }
    let (failures, checked) = run_all(carrier_shapes(), true);
    println!("carrier shapes: {checked} non-vacuous tag checks");
    assert!(failures.is_empty(), "{failures:#?}");
}

/// A dozen representative `oracle_png` fixtures plus one with every pinned tag:
/// the output prints none of the tags the preset removes (an `eXIf` keeps its
/// other tags, so only the pinned list is checked).
#[test]
fn representative_fixtures_print_no_tag_the_preset_removes() {
    if !live_gate("representative_fixtures_print_no_tag_the_preset_removes") {
        return;
    }
    let (failures, checked) = run_all(fixtures(), false);
    println!("fixtures: {checked} non-vacuous tag checks");
    assert!(failures.is_empty(), "{failures:#?}");
    // Not vacuous overall: the fixtures do print removable tags before the strip.
    assert!(checked >= 5 * 4, "only {checked} tags were ever checked");
}

/// The check goes red: on the unstripped input it reports the tags (so a strip
/// that did nothing would fail the three tests above), and on the case grid
/// input too.
#[test]
fn the_check_goes_red_on_an_unstripped_file() {
    if !live_gate("the_check_goes_red_on_an_unstripped_file") {
        return;
    }
    let rich = with(&[oracle_png::exif_chunk(&rich_tiff())]);
    let tags = listing(&rich, "red");
    for preset in 0..5 {
        let v = violations(preset, false, &tags);
        assert!(
            v.iter()
                .any(|t| t.starts_with("GPS:") || t.starts_with("Composite:GPS")),
            "preset {preset}: {v:?}"
        );
    }
    assert!(violations(1, false, &tags)
        .iter()
        .any(|t| t.ends_with("SerialNumber")));
    assert!(violations(2, false, &tags)
        .iter()
        .any(|t| t.ends_with("Artist")));
    assert!(!violations(1, false, &tags)
        .iter()
        .any(|t| t.ends_with("Artist")));
    // A "strip" that returns its input unchanged is caught by `check`.
    let grid = case_grid();
    let (name, bytes) = &grid[0];
    let before = listing(bytes, name);
    assert!(!violations(0, true, &before).is_empty(), "{name}");
}
