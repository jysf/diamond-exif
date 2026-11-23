//! SPEC-038 fix round (verify H1 and L3): the completeness sweep of the WebP
//! carrier table, against ExifTool's own list of tag ids.
//!
//! The strip's table (`tests/support/webp_carriers.rs`, DEC-022) was measured
//! by the builders from their own list of chunk types, and a list typed by the
//! same people who wrote the strip can only be complete against itself: the
//! verify found a chunk coded `RIFF` that ExifTool walks into and the table
//! never named. This test takes the list from ExifTool instead.
//!
//! Candidates: every four-character tag id in `exiftool -listx`, every code of
//! the measured table, the codes with a rule of their own (`EXIF`, `XMP `,
//! `XMP\0`, `ICCP`), the not-metadata list, the structure codes and the
//! RIFF, AVI, WAV and WebP container codes. Each candidate is one chunk (before
//! the image data) in a bare WebP, in eight payload shapes: text, a TIFF with
//! an Artist, an XMP packet, an ID3 header, a JUMBF manifest, zeros, a
//! nested RIFF file and a `LIST/INFO` body. ExifTool runs once over all of them (`-a -u -G1 -s -r`).
//!
//! A file FAILS when it prints a tag that the bare file does not, other than an
//! `Unknown_*` dump or a tag of the `File`, `System` or `ExifTool` groups
//! (structure), and its code is neither in the measured table, nor one of the
//! four codes with a rule, nor in the pinned structure list with exactly the
//! tag names pinned there. So a code ExifTool decodes that the strip does not
//! know about turns this red, whoever forgot it.
//!
//! Live only, never replayed: the answer is about the installed ExifTool.
//! Without it the test prints `SKIP` and passes, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`. ExifTool is run, never read or linked.

#[path = "support/tools.rs"]
mod tools;
#[path = "support/webp_carriers.rs"]
mod webp_carriers;
#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;
use webp_carriers::Payload;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

/// Codes with a rule of their own in the strip (not table rows): `EXIF`, the
/// two XMP spellings and `ICCP`.
const RULE_CODES: [[u8; 4]; 4] = [*b"EXIF", *b"XMP ", *b"XMP\0", *b"ICCP"];

/// Chunk codes the strip never removes because they are image structure:
/// ExifTool prints these tag names for them (pinned; a name outside the list is
/// a failure). Measured on ExifTool 13.55 over the eight payload shapes.
const STRUCTURE: [([u8; 4], &[&str]); 6] = [
    (
        *b"VP8X",
        &["ImageHeight", "WebP_Flags", "ImageSize", "Megapixels"],
    ),
    (
        *b"ALPH",
        &["AlphaCompression", "AlphaFiltering", "AlphaPreprocessing"],
    ),
    (*b"ANIM", &["AnimationLoopCount", "BackgroundColor"]),
    (*b"ANMF", &["Duration"]),
    (*b"VP8 ", &[]),
    (*b"VP8L", &[]),
];

/// The RIFF, AVI, WAV and WebP container codes the sweep always tries (some are
/// not listed by `-listx` because ExifTool decodes them as sub-directories).
const CONTAINER_CODES: [&[u8; 4]; 21] = [
    b"RIFF", b"RF64", b"RIFX", b"LIST", b"AVI ", b"WAVE", b"WEBP", b"INFO", b"INF0", b"hdrl",
    b"movi", b"idx1", b"adtl", b"fmt ", b"data", b"JUNK", b"junk", b"PAD ", b"strl", b"odml",
    b"ISFT",
];

/// The eight payload shapes of one candidate chunk.
const SHAPES: [&str; 8] = [
    "text", "tiff", "xmp", "id3", "jumbf", "zeros", "nested", "list",
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

/// A TIFF with one `Artist` entry (`Jane Doe`).
fn tiff_artist() -> Vec<u8> {
    let value = b"Jane Doe\0";
    let mut v = b"II*\0".to_vec();
    v.extend_from_slice(&8u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&0x013Bu16.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&(value.len() as u32).to_le_bytes());
    v.extend_from_slice(&26u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(value);
    v
}

fn shape_bytes(shape: &str) -> Vec<u8> {
    match shape {
        "text" => webp_carriers::payload(Payload::Text),
        "tiff" => tiff_artist(),
        "xmp" => webp_carriers::payload(Payload::Xmp),
        "id3" => webp_carriers::payload(Payload::Id3),
        "jumbf" => webp_carriers::payload(Payload::Jumbf),
        "zeros" => webp_carriers::payload(Payload::Zeros),
        "nested" => webp_carriers::riff_nested(1),
        "list" => webp_carriers::payload(Payload::ListInfo),
        other => panic!("unknown shape {other}"),
    }
}

/// The four-character ids of an `exiftool -listx` document (`<tag id='...'`),
/// unescaped, printable ASCII only.
fn listx_ids(xml: &str) -> BTreeSet<[u8; 4]> {
    let mut out = BTreeSet::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<tag id='") {
        rest = &rest[at + "<tag id='".len()..];
        let Some(end) = rest.find('\'') else {
            break;
        };
        let raw = &rest[..end];
        let id = raw
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'");
        let b = id.as_bytes();
        if let Ok(code) = <[u8; 4]>::try_from(b) {
            if code.iter().all(|c| (0x20..0x7F).contains(c)) {
                out.insert(code);
            }
        }
    }
    out
}

fn candidates(listx: &str) -> BTreeSet<[u8; 4]> {
    let mut all = listx_ids(listx);
    all.extend(webp_carriers::ROWS.iter().map(|r| r.code));
    all.extend(webp_carriers::NOT_METADATA.iter().copied());
    all.extend(RULE_CODES);
    all.extend(STRUCTURE.iter().map(|s| s.0));
    all.extend(CONTAINER_CODES.iter().map(|c| **c));
    all
}

/// One chunk before the image data in a minimal WebP.
fn file_of(code: &[u8; 4], shape: &str) -> Vec<u8> {
    webp_shapes::riff(&[
        webp_shapes::chunk(code, &shape_bytes(shape)),
        webp_shapes::vp8l(),
    ])
}

/// `Group:Name` keys of an `exiftool -a -u -G1 -s -r` listing, by file name
/// (the part after the last `/` of each `======== path` header).
fn parse_listing(stdout: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("======== ") {
            let name = path.rsplit('/').next().unwrap_or(path).to_string();
            out.entry(name.clone()).or_default();
            current = Some(name);
            continue;
        }
        let Some(name) = &current else {
            continue;
        };
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((group, rest)) = rest.split_once(']') else {
            continue;
        };
        let Some((left, _)) = rest.split_once(" : ").or_else(|| rest.split_once(':')) else {
            continue;
        };
        let Some(tag) = left.split_whitespace().last() else {
            continue;
        };
        out.entry(name.clone())
            .or_default()
            .insert(format!("{group}:{tag}"));
    }
    out
}

/// The tags a file prints that the bare file does not, less the structure ones
/// (`Unknown_*` dumps, the `File`, `System` and `ExifTool` groups).
fn new_tags(listing: &BTreeSet<String>, bare: &BTreeSet<String>) -> Vec<String> {
    listing
        .difference(bare)
        .filter(|k| {
            let (group, name) = k.split_once(':').unwrap_or(("", k));
            !name.starts_with("Unknown_") && !matches!(group, "File" | "System" | "ExifTool")
        })
        .cloned()
        .collect()
}

/// The measured table's codes plus the four codes with a rule of their own.
fn known_codes() -> BTreeSet<[u8; 4]> {
    let mut known: BTreeSet<[u8; 4]> = webp_carriers::ROWS.iter().map(|r| r.code).collect();
    known.extend(RULE_CODES);
    known
}

/// Why `tags` (printed from one `code` chunk) is not explained, or `None`.
fn unexplained(code: &[u8; 4], tags: &[String], known: &BTreeSet<[u8; 4]>) -> Option<String> {
    if tags.is_empty() || known.contains(code) {
        return None;
    }
    let pinned = STRUCTURE.iter().find(|s| &s.0 == code).map(|s| s.1);
    let extra: Vec<&String> = tags
        .iter()
        .filter(|k| {
            let name = k.split_once(':').map_or(k.as_str(), |(_, n)| n);
            !pinned.is_some_and(|p| p.contains(&name))
        })
        .collect();
    (!extra.is_empty()).then(|| format!("{extra:?}"))
}

/// A scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Scratch {
        let dir = std::env::temp_dir().join(format!("diamond-exif-sweep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn exiftool_listx() -> String {
    let out = std::process::Command::new("exiftool")
        .arg("-listx")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("exiftool -listx");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The sweep: every candidate code in every shape, one ExifTool run.
#[test]
fn every_code_exiftool_knows_is_in_the_table_or_prints_nothing() {
    if !live_gate("every_code_exiftool_knows_is_in_the_table_or_prints_nothing") {
        return;
    }
    let codes: Vec<[u8; 4]> = candidates(&exiftool_listx()).into_iter().collect();
    assert!(
        codes.len() > 2000,
        "only {} candidate codes: did -listx change shape?",
        codes.len()
    );
    let scratch = Scratch::new();
    let name = |i: usize, s: usize| format!("{i:05}_{s}.webp");
    for (i, code) in codes.iter().enumerate() {
        for (s, shape) in SHAPES.iter().enumerate() {
            std::fs::write(scratch.0.join(name(i, s)), file_of(code, shape)).expect("write");
        }
    }
    std::fs::write(
        scratch.0.join("bare.webp"),
        webp_shapes::riff(&[webp_shapes::vp8l()]),
    )
    .expect("write bare");
    let args: Vec<String> = ["-a", "-u", "-G1", "-s", "-r"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let out = tools::run_tool_with_limit("exiftool", &args, &scratch.0, Duration::from_secs(240))
        .unwrap_or_else(|e| panic!("exiftool over the sweep: {e:?}"));
    let listing = parse_listing(&String::from_utf8_lossy(&out.stdout));
    let bare = listing.get("bare.webp").cloned().expect("the bare file");
    assert!(!bare.is_empty(), "the bare file printed nothing");

    let known = known_codes();
    let mut failures: Vec<String> = Vec::new();
    let mut printing: BTreeSet<[u8; 4]> = BTreeSet::new();
    for (i, code) in codes.iter().enumerate() {
        for (s, shape) in SHAPES.iter().enumerate() {
            let file = name(i, s);
            let tags = new_tags(
                listing
                    .get(&file)
                    .unwrap_or_else(|| panic!("no listing for {file}")),
                &bare,
            );
            if !tags.is_empty() {
                printing.insert(*code);
            }
            if let Some(why) = unexplained(code, &tags, &known) {
                failures.push(format!(
                    "{:?} ({shape}): ExifTool prints {why}, and the code is neither in the measured \
                     table nor pinned as structure",
                    String::from_utf8_lossy(code)
                ));
            }
        }
    }
    println!(
        "sweep: {} codes x {} shapes = {} files; {} codes print a tag the bare file does not",
        codes.len(),
        SHAPES.len(),
        codes.len() * SHAPES.len(),
        printing.len()
    );
    assert!(failures.is_empty(), "{:#?}", failures);
    // Not vacuous: every table row printed under at least one shape, and the
    // sweep saw a fair number of decoding codes.
    for row in webp_carriers::ROWS {
        assert!(
            printing.contains(&row.code),
            "{:?} printed nothing in any shape, so the sweep proves nothing for it",
            String::from_utf8_lossy(&row.code)
        );
    }
    assert!(printing.len() >= 30, "{} printing codes", printing.len());
}

/// The judge goes red: a chunk coded `RIFF` holding a nested file prints a tag
/// ExifTool decodes; with the `RIFF` row left out of the table (the state of
/// the strip before the SPEC-038 fix round) the judge reports it, and with the
/// row in it does not.
#[test]
fn the_sweep_goes_red_on_a_code_missing_from_the_table() {
    // The judge alone, on a made-up listing.
    let riff = *b"RIFF";
    let tags = vec!["RIFF:Artist".to_string()];
    let mut known = known_codes();
    assert!(unexplained(&riff, &tags, &known).is_none());
    known.remove(&riff);
    assert!(unexplained(&riff, &tags, &known).is_some());
    // A structure code prints only its pinned names.
    assert!(unexplained(b"ANMF", &["RIFF:Duration".to_string()], &known).is_none());
    assert!(unexplained(b"ANMF", &["RIFF:Artist".to_string()], &known).is_some());
    assert!(unexplained(b"PAD ", &["RIFF:Artist".to_string()], &known).is_some());
    assert!(unexplained(b"PAD ", &[], &known).is_none());
}

/// The same, against the real ExifTool: the nested-RIFF file prints `Artist`
/// and the bare file does not, so a table without the `RIFF` row is caught.
#[test]
fn the_nested_riff_file_prints_a_tag_the_bare_file_does_not() {
    if !live_gate("the_nested_riff_file_prints_a_tag_the_bare_file_does_not") {
        return;
    }
    let args: Vec<String> = ["-a", "-u", "-G1", "-s"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let run = |bytes: &[u8], label: &str| {
        let path = tools::temp_file_with_ext(label, "webp", bytes).expect("temp file");
        let out = tools::run_tool_with_limit("exiftool", &args, &path, Duration::from_secs(8))
            .unwrap_or_else(|e| panic!("exiftool: {e:?}"));
        let _ = std::fs::remove_file(&path);
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let named = format!("======== x.webp\n{text}");
        parse_listing(&named).remove("x.webp").unwrap_or_default()
    };
    let bare = run(&webp_shapes::riff(&[webp_shapes::vp8l()]), "sweep-bare");
    let nested = run(&file_of(b"RIFF", "nested"), "sweep-nested");
    let tags = new_tags(&nested, &bare);
    assert!(tags.iter().any(|k| k == "RIFF:Artist"), "{tags:?}");
    let mut known = known_codes();
    known.remove(b"RIFF");
    assert!(unexplained(b"RIFF", &tags, &known).is_some());
}
