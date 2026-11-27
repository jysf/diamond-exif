//! SPEC-038: independence of facts for the WebP strip. The strip tests compare
//! the output with a model written from DEC-022; this file asks ExifTool
//! instead. It strips WebPs under all five presets, runs ExifTool on the OUTPUT
//! and fails if it still prints a tag the preset removes. It is the cheap
//! precursor of SPEC-039's formal ExifTool write parity and is kept small on
//! purpose (the PNG sibling is `strip_png_live.rs`).
//!
//! Live only, never replayed: a recorded answer about a file the strip produced
//! would be a record of this library's own output. Without ExifTool each test
//! prints `SKIP` and passes, unless `DIAMOND_EXIF_REQUIRE_ORACLE=1` (as its
//! siblings). ExifTool is run (`-a -u -G1 -H -s`, 8 s limit), never read or
//! linked.
//!
//! The pinned judge is the complete preset table of DEC-009 (SPEC-038 fix
//! round, verify M2): every printed tag is classified into a category by the
//! lists in [`category_of`] (typed here from DEC-008, not read from the
//! library), and per preset the judge asserts BOTH directions: every tag of a
//! category the preset removes is gone from the output, and every tag of a
//! category it keeps is still printed (a strip that over-removes is a failure;
//! the one documented exception is the carrier rule, which removes a `LIST`
//! and its `RIFF`-group tags under every preset). The categories a WebP can
//! hold, and the tags that stand for each:
//!
//! * `Gps`: any tag in the `GPS` group and any `Composite` tag named `GPS*`;
//! * `DeviceId`: `SerialNumber`, `BodySerialNumber`, `InternalSerialNumber`,
//!   `LensSerialNumber`, `ImageUniqueID`;
//! * `Authorship`: `Author`, `Artist`, `OwnerName`, `CameraOwnerName` (which
//!   also covers the `LIST` `INFO` personal tags);
//! * `Rights` (SPEC-033): `Copyright`;
//! * `FreeText`: `ImageDescription`, `UserComment`;
//! * `Timestamps`: `ModifyDate`, `DateTimeOriginal`, `CreateDate`;
//! * `CameraModel`: `Make`, `Model`, `LensModel`;
//! * `Thumbnail`: `ThumbnailImage`, `ThumbnailOffset`, `ThumbnailLength`;
//! * `MakerNote`: any tag named `MakerNote*`;
//! * `Software`: `Software`;
//! * `Xmp`: any tag in an `XMP*` group; `Icc`: any tag in an `ICC*` group;
//!   `Jumbf`: any tag in the `JUMBF` or `JSON` group (a `C2PA` chunk);
//! * `Iptc` has no WebP carrier and is not judged.
//!
//! Presets (DEC-009): `Gps` removes `Gps`; `GpsAndDevice` adds `DeviceId`;
//! `AllPii` adds `Authorship`, `FreeText`, `Timestamps`, `CameraModel`,
//! `Thumbnail`, `MakerNote`; `All` adds `Software`, `Xmp`, `Iptc`, `Rights`;
//! `Everything` adds `Icc`, `Jumbf`.
//!
//! The older lists, kept as the statement of the first build:
//!
//! * every preset: any tag in the `GPS` group and any `Composite` tag whose name
//!   starts with `GPS` (`GPSPosition`, `GPSLatitude`, ...);
//! * the carriers (the 20 measured carrier rows and the text `JUNK`): every tag
//!   a carrier file prints that the same file without the chunk does not print,
//!   under every preset; the opaque chunks (`CSET`, `fmt `, ...) and `C2PA`
//!   under `Everything` only (`StripPolicy` presets: only `Everything` has
//!   `OtherSegment` and `Jumbf`), and untouched byte for byte under the other
//!   four;
//! * every other spelling of a carrier code (the case grid): not decoded by
//!   ExifTool (only `Unknown_*` tags), and so not touched by any preset.
//!
//! Each carrier file must print a removable tag BEFORE the strip (never a
//! vacuous pass), the output must still carry every pixel chunk byte for byte
//! and ExifTool must print no `Error` for it that it did not print for the
//! input. The check is also shown red: [`the_check_goes_red_on_an_unstripped_file`].

#[path = "support/i3_judge.rs"]
mod i3_judge;
#[path = "support/i3_records.rs"]
mod i3_records;
#[path = "support/i3_shapes.rs"]
mod i3_shapes;
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
#[path = "support/webp_carriers.rs"]
mod webp_carriers;
#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use exif_core::policy::StripPolicy;
use exif_core::{strip, StripError};
use i3_judge::{Cat, ALL_CATS};
use i3_shapes::rich_tiff;
use oracle_tiff as t;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;
use webp_carriers::Row;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

/// The `-a -u -G1 -H -s` run, the same arguments as the audit oracle.
const ARGS: [&str; 5] = ["-a", "-u", "-G1", "-H", "-s"];

/// Representative `oracle_webp` fixtures (names are the committed ones): EXIF
/// with GPS in several orders, with a prefix, odd and with a lying size, XMP,
/// ICC, all three, and `EXIF` before the `VP8X`.
const FIXTURES: [&str; 13] = [
    "webp-exif-gps-full-le",
    "webp-exif-gps-ne-le",
    "webp-exif-gps-sw-be",
    "webp-exif-basic-le",
    "webp-exif-prefix",
    "webp-exif-odd-padded",
    "webp-exif-before-vp8x",
    "webp-xmp-basic",
    "webp-xmp-two",
    "webp-icc-valid-flag-set",
    "webp-all-three",
    "webp-exif-full-one-two",
    "webp-riff-size-small",
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Tag {
    group: String,
    name: String,
}

impl Tag {
    fn key(&self) -> String {
        format!("{}:{}", self.group, self.name)
    }
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
    let path = tools::temp_file_with_ext(label, "webp", bytes).expect("temp file");
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

fn keys(tags: &[Tag]) -> BTreeSet<String> {
    tags.iter().map(Tag::key).collect()
}

fn has_error(tags: &[Tag]) -> bool {
    tags.iter().any(|t| t.name == "Error")
}

/// DEC-009 (`support/i3_judge.rs`, typed there): does `preset` (index into
/// `StripPolicy::PRESETS`) remove `cat`?
fn preset_removes(preset: usize, cat: Cat) -> bool {
    i3_judge::removes(i3_judge::PRESET_NAMES[preset], cat)
}

/// The category a printed tag stands for, from the shared table.
fn category_of(tag: &Tag) -> Option<Cat> {
    i3_judge::category_of(&tag.group, &tag.name)
}

/// The classified tags of `listing`, by category.
fn by_category(listing: &[Tag]) -> std::collections::BTreeMap<Cat, BTreeSet<String>> {
    let mut out: std::collections::BTreeMap<Cat, BTreeSet<String>> = Default::default();
    for t in listing {
        if let Some(c) = category_of(t) {
            out.entry(c).or_default().insert(t.key());
        }
    }
    out
}

/// What is wrong with `after` (the listing of the output) under `preset`, given
/// `before` (the listing of the input): a tag of a removed category still
/// printed, or a tag of a kept category that is no longer printed.
/// A `RIFF`-group tag (what a `LIST` prints) is not classified, so the carrier
/// rule's over-removal of a `LIST` is not judged here.
fn judge(preset: usize, before: &[Tag], after: &[Tag]) -> Vec<String> {
    let (b, a) = (by_category(before), by_category(after));
    let mut bad = Vec::new();
    for cat in ALL_CATS {
        let (was, now) = (b.get(&cat), a.get(&cat));
        if preset_removes(preset, cat) {
            if let Some(now) = now {
                bad.extend(now.iter().map(|k| format!("{k} ({cat:?}) not removed")));
            }
        } else if let Some(was) = was {
            for k in was {
                if !now.is_some_and(|n| n.contains(k)) {
                    bad.push(format!("{k} ({cat:?}) over-removed"));
                }
            }
        }
    }
    bad
}

/// The tags of `listing` a preset must have removed (for the red proof and the
/// non-vacuity counts).
fn violations(preset: usize, listing: &[Tag]) -> Vec<String> {
    judge(preset, listing, listing)
}

// ── the pixel chunks ────────────────────────────────────────────────────────

/// The image chunks of a WebP (`VP8 `, `VP8L`, `ALPH`, `ANIM`, `ANMF`) with
/// their bytes, in order, found by a naive RIFF walk.
fn pixel_chunks(d: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut p = 12usize;
    while p + 8 <= d.len() {
        let size = u32::from_le_bytes([d[p + 4], d[p + 5], d[p + 6], d[p + 7]]) as usize;
        let end = (p + 8 + size).min(d.len());
        if matches!(
            &d[p..p + 4],
            b"VP8 " | b"VP8L" | b"ALPH" | b"ANIM" | b"ANMF"
        ) {
            out.push(d[p..end].to_vec());
        }
        p = end + (size & 1);
    }
    out
}

// ── the cases ───────────────────────────────────────────────────────────────

/// How a case is judged.
enum Judge {
    /// The pinned list per preset: GPS, device, authorship tags are gone.
    /// `xmp_gps`: the file holds an XMP packet with GPS properties, which a
    /// preset without the `Xmp` category (the first three) keeps by design
    /// (DEC-022 clause 2, the stated residual), so ExifTool's `Composite:GPS*`
    /// derived from it is expected there.
    Pinned { xmp_gps: bool },
    /// A carrier file: every tag it prints that `baseline` does not goes with
    /// the presets in `removed_by`; under the others the file is unchanged.
    Carrier {
        baseline: Vec<u8>,
        removed_by: [bool; 5],
        prints: String,
    },
    /// A file ExifTool must not decode beyond `Unknown_*` tags: no preset
    /// touches it.
    Untouched { baseline: Vec<u8> },
}

struct Case {
    name: String,
    input: Vec<u8>,
    judge: Judge,
    /// A refusal (`Uneditable`) is a fair answer under a partial policy.
    allow_refusal: bool,
}

#[derive(Default)]
struct Tally {
    failures: Vec<String>,
    checked: usize,
    refused: usize,
    stripped: usize,
}

/// Strip `case` under every preset, ExifTool the output, and judge it.
fn check(case: &Case, tally: &mut Tally) {
    let name = &case.name;
    let before = listing(&case.input, name);
    let before_keys = keys(&before);
    match &case.judge {
        Judge::Untouched { baseline } => {
            let base = keys(&listing(baseline, name));
            let new: Vec<&String> = before_keys.difference(&base).collect();
            if let Some(bad) = new.iter().find(|k| !k.contains("Unknown_")) {
                tally.failures.push(format!(
                    "{name}: ExifTool decodes this chunk type (prints {bad}); the strip leaves it"
                ));
            }
            for (i, policy) in StripPolicy::PRESETS.iter().enumerate() {
                match strip(&case.input, *policy) {
                    Ok(out) if out.bytes() == &case.input[..] => tally.checked += 1,
                    other => tally.failures.push(format!(
                        "{name} under preset {i}: expected an untouched file, got {:?}",
                        other.map(|o| o.bytes().len())
                    )),
                }
            }
            return;
        }
        Judge::Carrier {
            baseline, prints, ..
        } => {
            let base = keys(&listing(baseline, name));
            let new: Vec<&String> = before_keys.difference(&base).collect();
            if !new.iter().any(|k| k.as_str() == prints) {
                tally.failures.push(format!(
                    "{name}: ExifTool prints nothing from the carrier (wanted {prints}, new \
                     tags {new:?}), so the check would pass vacuously"
                ));
                return;
            }
        }
        Judge::Pinned { .. } => {}
    }
    let pixels = pixel_chunks(&case.input);
    for (i, policy) in StripPolicy::PRESETS.iter().enumerate() {
        let out = match strip(&case.input, *policy) {
            Ok(o) => o,
            Err(StripError::Uneditable { .. }) if case.allow_refusal => {
                tally.refused += 1;
                continue;
            }
            Err(e) => panic!("{name} {}: {e:?}", policy.name()),
        };
        tally.stripped += 1;
        if pixel_chunks(out.bytes()) != pixels {
            tally.failures.push(format!(
                "{name} under {}: a pixel chunk changed",
                policy.name()
            ));
        }
        match &case.judge {
            Judge::Pinned { xmp_gps } => {
                // The residual: GPS from a kept XMP packet (see `Judge`).
                let residual = |k: &String| {
                    *xmp_gps && i < 3 && (k.starts_with("Composite:GPS") || k.contains(":GPS"))
                };
                let after = listing(out.bytes(), name);
                let bad: Vec<String> = judge(i, &before, &after)
                    .into_iter()
                    .filter(|k| !residual(k))
                    .collect();
                // Every classified tag of the input is a check: removed if its
                // category goes, still there if it stays.
                tally.checked += by_category(&before)
                    .values()
                    .map(BTreeSet::len)
                    .sum::<usize>();
                if !bad.is_empty() {
                    tally
                        .failures
                        .push(format!("{name} under {}: {bad:?}", policy.name()));
                }
                if has_error(&after) && !has_error(&before) {
                    tally
                        .failures
                        .push(format!("{name} under {}: ExifTool Error", policy.name()));
                }
            }
            Judge::Carrier {
                baseline,
                removed_by,
                ..
            } => {
                if removed_by[i] {
                    let base = keys(&listing(baseline, name));
                    let after = listing(out.bytes(), name);
                    let leaked: Vec<String> = keys(&after).difference(&base).cloned().collect();
                    tally.checked += 1;
                    if out.bytes().len() >= case.input.len() {
                        tally
                            .failures
                            .push(format!("{name} under {}: nothing removed", policy.name()));
                    }
                    if !leaked.is_empty() {
                        tally.failures.push(format!(
                            "{name} under {}: still prints {leaked:?}",
                            policy.name()
                        ));
                    }
                    if has_error(&after) && !has_error(&before) {
                        tally
                            .failures
                            .push(format!("{name} under {}: ExifTool Error", policy.name()));
                    }
                } else if out.bytes() == &case.input[..] {
                    tally.checked += 1;
                } else {
                    tally.failures.push(format!(
                        "{name} under {}: expected the file untouched",
                        policy.name()
                    ));
                }
            }
            Judge::Untouched { .. } => unreachable!(),
        }
    }
}

/// Run `check` over `cases` on four threads.
fn run_all(cases: Vec<Case>) -> Tally {
    let cases = std::sync::Arc::new(cases);
    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let (cases, next) = (cases.clone(), next.clone());
        handles.push(std::thread::spawn(move || {
            let mut tally = Tally::default();
            loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some(case) = cases.get(i) else {
                    break;
                };
                check(case, &mut tally);
            }
            tally
        }));
    }
    let mut all = Tally::default();
    for h in handles {
        let t = h.join().expect("worker");
        all.failures.extend(t.failures);
        all.checked += t.checked;
        all.refused += t.refused;
        all.stripped += t.stripped;
    }
    all.failures.sort();
    all
}

// ── the inputs ──────────────────────────────────────────────────────────────

/// The bare file every carrier file is compared with: a `VP8L` image only.
fn bare() -> Vec<u8> {
    webp_shapes::riff(&[webp_shapes::vp8l()])
}

/// The carrier chunk of `row`, spelt `code`.
fn carrier(code: &[u8; 4], row: &Row) -> Vec<u8> {
    webp_carriers::row_chunk(code, row.payload)
}

/// Who removes a row: a carrier goes with every preset; an opaque chunk and a
/// JUMBF manifest only with `Everything`.
fn row_removed_by(row: &Row) -> [bool; 5] {
    if row.region == "carrier" {
        [true; 5]
    } else {
        [false, false, false, false, true]
    }
}

/// The position grid of one chunk: before the image data, after it, beyond the
/// declared RIFF size and in a file with no `VP8X`. Each is paired with the
/// same file without the chunk.
fn positions(chunk: &[u8]) -> Vec<(&'static str, Vec<u8>, Vec<u8>)> {
    use webp_shapes::{riff, vp8l, vp8x};
    let mut beyond = riff(&[vp8x(0, 8, 8), vp8l()]);
    let beyond_base = beyond.clone();
    beyond.extend_from_slice(chunk);
    vec![
        (
            "before",
            riff(&[vp8x(0, 8, 8), chunk.to_vec(), vp8l()]),
            riff(&[vp8x(0, 8, 8), vp8l()]),
        ),
        (
            "after",
            riff(&[vp8x(0, 8, 8), vp8l(), chunk.to_vec()]),
            riff(&[vp8x(0, 8, 8), vp8l()]),
        ),
        ("beyond", beyond, beyond_base),
        ("novp8x", riff(&[vp8l(), chunk.to_vec()]), riff(&[vp8l()])),
    ]
}

/// One carrier case per row and position (the `JUNK` row included: a text
/// `JUNK` is a carrier).
fn carrier_cases() -> Vec<Case> {
    let mut out = Vec::new();
    for row in webp_carriers::ROWS {
        let code = String::from_utf8_lossy(&row.code).into_owned();
        for (pos, input, baseline) in positions(&carrier(&row.code, row)) {
            out.push(Case {
                name: format!("row-{code}-{pos}"),
                input,
                judge: Judge::Carrier {
                    baseline,
                    removed_by: row_removed_by(row),
                    prints: row.prints.to_string(),
                },
                allow_refusal: false,
            });
        }
    }
    // SPEC-038 fix round: a chunk coded `RIFF` is walked into at any depth and
    // a second whole file appended after the first is one (measured); each
    // must print `RIFF:Artist` before the strip and nothing after it.
    let artist = webp_carriers::ROWS
        .iter()
        .find(|r| &r.code == b"RIFF")
        .map(|r| r.prints.to_string())
        .expect("the RIFF row");
    let deep = webp_shapes::chunk(b"RIFF", &webp_carriers::riff_nested(3));
    let mut appended = bare();
    appended.extend_from_slice(&webp_shapes::riff(&[webp_shapes::chunk(
        b"LIST",
        &webp_carriers::payload(webp_carriers::Payload::ListInfo),
    )]));
    for (name, input) in [
        (
            "riff-depth-three",
            webp_shapes::riff(&[webp_shapes::vp8l(), deep]),
        ),
        ("riff-appended-second-file", appended),
    ] {
        out.push(Case {
            name: name.to_string(),
            input,
            judge: Judge::Carrier {
                baseline: bare(),
                removed_by: [true; 5],
                prints: artist.clone(),
            },
            allow_refusal: false,
        });
    }
    out
}

/// The case grid: every ASCII-case spelling of every row's code other than the
/// exact code and the other measured rows. ExifTool decodes none of them, and
/// no preset touches them.
fn case_grid_cases() -> Vec<Case> {
    let mut out = Vec::new();
    for row in webp_carriers::ROWS {
        for s in webp_carriers::spellings(row.code) {
            if s == row.code || webp_carriers::ROWS.iter().any(|r| r.code == s) {
                continue;
            }
            let input = webp_shapes::riff(&[webp_shapes::vp8l(), carrier(&s, row)]);
            out.push(Case {
                name: format!("spelling-{}-of-{}", String::from_utf8_lossy(&s), {
                    String::from_utf8_lossy(&row.code)
                }),
                input,
                judge: Judge::Untouched { baseline: bare() },
                allow_refusal: false,
            });
        }
    }
    out
}

/// A TIFF holding one tag of every Exif-set category in its three IFDs plus an
/// IFD1 thumbnail (a real JPEG after the IFDs): free text, camera model,
/// software, timestamps, authorship, device identity, a maker note, GPS.
fn all_categories_tiff() -> Vec<u8> {
    let big = false;
    let thumb = [0xFFu8, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xD9];
    let build = |thumb_at: u32| {
        let ifd0 = t::ifd_next(
            vec![
                t::ascii(0x010E, b"A free description\0"),
                t::ascii(0x010F, b"MakeCorp\0"),
                t::ascii(0x0110, b"ModelX\0"),
                t::ascii(0x0131, b"SoftTool 1.0\0"),
                t::ascii(0x0132, b"2024:01:02 03:04:05\0"),
                t::ascii(0x013B, b"Artist Zed\0"),
                t::ascii(0x8298, b"Copy Zed\0"),
                t::ptr(0x8769, 1),
                t::ptr(0x8825, 2),
            ],
            3,
        );
        let exif = t::ifd(vec![
            t::ascii(0x9003, b"2024:01:02 03:04:05\0"),
            t::ascii(0x9004, b"2024:01:02 03:04:06\0"),
            t::undef(0x9286, b"ASCII\0\0\0A user comment"),
            t::ascii(0xA420, b"UNIQUEID123456\0"),
            t::ascii(0xA430, b"Owner Zed\0"),
            t::ascii(0xA431, b"SN123456\0"),
            t::ascii(0xA434, b"LensModelZ\0"),
            t::undef(0x927C, b"Unknown maker note bytes 0123456789"),
        ]);
        let gps = t::ifd(vec![
            t::ascii(0x0001, b"N\0"),
            t::rational(big, 0x0002, &[(51, 1), (30, 1), (0, 1)]),
            t::ascii(0x0003, b"E\0"),
            t::rational(big, 0x0004, &[(0, 1), (7, 1), (0, 1)]),
        ]);
        let ifd1 = t::ifd(vec![
            t::short(big, 0x0103, 6),
            t::ptr_raw(0x0201, thumb_at),
            t::long(big, 0x0202, thumb.len() as u32),
        ]);
        t::assemble(big, &[ifd0, exif, gps, ifd1])
    };
    let len = build(0).len() as u32;
    let mut out = build(len);
    out.extend_from_slice(&thumb);
    out
}

/// A `C2PA` chunk: a JUMBF manifest with a GPS and a creator assertion.
fn c2pa_chunk() -> Vec<u8> {
    webp_shapes::chunk(
        b"C2PA",
        &webp_carriers::payload(webp_carriers::Payload::Jumbf),
    )
}

/// A WebP holding only the rich TIFF in an `EXIF` chunk (the main type).
fn exif_file(n: usize, flags: u8) -> Vec<u8> {
    let mut parts = vec![webp_shapes::vp8x(flags, 8, 8), webp_shapes::vp8l()];
    for _ in 0..n {
        parts.push(webp_shapes::exif_chunk(&rich_tiff()));
    }
    webp_shapes::riff(&parts)
}

fn xmp_with_gps() -> Vec<u8> {
    br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:exif="http://ns.adobe.com/exif/1.0/" exif:GPSLatitude="51,30.0N" exif:GPSLongitude="0,7.0E"><dc:creator><rdf:Seq><rdf:li>Zed</rdf:li></rdf:Seq></dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>"#.to_vec()
}

/// An XMP packet with a creator and no GPS.
fn xmp_plain() -> Vec<u8> {
    br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator><rdf:Seq><rdf:li>Zed</rdf:li></rdf:Seq></dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>"#.to_vec()
}

/// The three main types, the 16- and 17-`EXIF` files, the size variants and the
/// `oracle_webp` fixtures: judged by the pinned list.
fn pinned_cases() -> Vec<Case> {
    use webp_shapes::{chunk, riff, riff_with_size, vp8l, vp8x, FLAG_EXIF, FLAG_ICC, FLAG_XMP};
    let mut out: Vec<Case> = Vec::new();
    let mut add = |name: &str, input: Vec<u8>, allow_refusal: bool, xmp_gps: bool| {
        out.push(Case {
            name: name.to_string(),
            input,
            judge: Judge::Pinned { xmp_gps },
            allow_refusal,
        });
    };
    add("main-exif", exif_file(1, FLAG_EXIF), false, false);
    add(
        "main-xmp",
        riff(&[
            vp8x(FLAG_XMP, 8, 8),
            vp8l(),
            chunk(b"XMP ", &xmp_with_gps()),
        ]),
        false,
        true,
    );
    add(
        "main-icc",
        riff(&[
            vp8x(FLAG_ICC, 8, 8),
            chunk(b"ICCP", &oracle_png::icc_profile()),
            vp8l(),
        ]),
        false,
        false,
    );
    add(
        "main-all-three-and-list",
        riff(&[
            vp8x(FLAG_ICC | FLAG_EXIF | FLAG_XMP, 8, 8),
            chunk(b"ICCP", &oracle_png::icc_profile()),
            vp8l(),
            webp_shapes::exif_chunk(&rich_tiff()),
            chunk(b"XMP ", &xmp_plain()),
            webp_carriers::row_chunk(b"LIST", webp_carriers::Payload::ListInfo),
        ]),
        false,
        false,
    );
    // Every category a WebP can hold, at once (the complete DEC-009 table):
    // Exif of all ten Exif-set categories, plain XMP, ICC and a C2PA manifest.
    add(
        "all-categories",
        riff(&[
            vp8x(FLAG_ICC | FLAG_EXIF | FLAG_XMP, 8, 8),
            chunk(b"ICCP", &oracle_png::icc_profile()),
            vp8l(),
            webp_shapes::exif_chunk(&all_categories_tiff()),
            chunk(b"XMP ", &xmp_plain()),
            c2pa_chunk(),
        ]),
        false,
        false,
    );
    add("exif-sixteen", exif_file(16, FLAG_EXIF), true, false);
    add("exif-seventeen", exif_file(17, FLAG_EXIF), true, false);
    add("exif-twenty", exif_file(20, FLAG_EXIF), true, false);
    // Size variants: the declared RIFF size lies, small, large, zero, odd,
    // exact (the strip keeps the lie and takes the bytes it removed from it).
    for (label, declared) in [
        ("small", Some(4u32)),
        ("large", Some(0xFFFF_FFF0)),
        ("zero", Some(0)),
        ("odd", Some(7)),
        ("exact", None),
    ] {
        let parts = [
            vp8x(FLAG_EXIF | FLAG_XMP, 8, 8),
            vp8l(),
            webp_shapes::exif_chunk(&rich_tiff()),
            chunk(b"XMP ", &xmp_plain()),
            webp_carriers::row_chunk(b"LIST", webp_carriers::Payload::ListInfo),
        ];
        let input = match declared {
            Some(d) => riff_with_size(d, &parts),
            None => riff(&parts),
        };
        add(&format!("size-{label}"), input, false, false);
    }
    // The `oracle_webp` fixtures.
    for name in FIXTURES {
        let bytes = oracle_webp::fixtures()
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b)
            .unwrap_or_else(|| panic!("no WebP fixture named {name}"));
        add(name, bytes, false, false);
    }
    out
}

// ── the tests ───────────────────────────────────────────────────────────────

/// Every carrier row, in every position: ExifTool prints the carrier's tags
/// before the strip (never vacuous), and after it none that the bare file does
/// not print, under every preset that removes the row; under the others the file
/// is untouched.
#[test]
fn every_measured_carrier_row_leaves_no_decoded_tag_for_exiftool() {
    if !live_gate("every_measured_carrier_row_leaves_no_decoded_tag_for_exiftool") {
        return;
    }
    let cases = carrier_cases();
    // Four positions per row, plus the two RIFF-chunk shapes.
    let n = webp_carriers::ROWS.len() * 4 + 2;
    assert_eq!(cases.len(), n);
    let tally = run_all(cases);
    println!(
        "carrier rows: {n} files, {} strips, {} non-vacuous checks",
        tally.stripped, tally.checked
    );
    assert!(tally.failures.is_empty(), "{:#?}", tally.failures);
    assert!(tally.checked >= n * 5);
}

/// The case grid: no other spelling of a carrier code is decoded by ExifTool
/// (only `Unknown_*`), and no preset touches one.
#[test]
fn the_case_grid_is_neither_decoded_nor_touched() {
    if !live_gate("the_case_grid_is_neither_decoded_nor_touched") {
        return;
    }
    let cases = case_grid_cases();
    // 33 rows (the `RIFF` row since the fix round), 16 spellings each, less the
    // exact code and the other rows.
    assert_eq!(cases.len(), 415, "spellings");
    let n = cases.len();
    let tally = run_all(cases);
    println!("case grid: {n} files, {} untouched checks", tally.checked);
    assert!(tally.failures.is_empty(), "{:#?}", tally.failures);
    assert_eq!(tally.checked, n * 5);
}

/// The three main types, 16 and 17 `EXIF` chunks, the size variants and a
/// dozen `oracle_webp` fixtures: the output prints none of the tags the preset
/// removes (an `EXIF` keeps its other tags, so only the pinned list is
/// checked), keeps its pixel chunks and gains no ExifTool error.
#[test]
fn representative_files_print_no_tag_the_preset_removes() {
    if !live_gate("representative_files_print_no_tag_the_preset_removes") {
        return;
    }
    let cases = pinned_cases();
    let n = cases.len();
    let tally = run_all(cases);
    println!(
        "pinned: {n} files, {} strips, {} refusals, {} non-vacuous tag checks",
        tally.stripped, tally.refused, tally.checked
    );
    assert!(tally.failures.is_empty(), "{:#?}", tally.failures);
    // Not vacuous overall: the files print removable tags before the strip.
    assert!(
        tally.checked >= 5 * 12,
        "only {} tags checked",
        tally.checked
    );
    // A refusal is only the partial-policy case with more than 16 `EXIF`
    // chunks; `Everything` strips them all.
    assert!(tally.refused <= 2 * 4 + 4);
}

/// The check goes red: on the unstripped input it reports the tags (so a strip
/// that did nothing would fail the tests above).
#[test]
fn the_check_goes_red_on_an_unstripped_file() {
    if !live_gate("the_check_goes_red_on_an_unstripped_file") {
        return;
    }
    let rich = exif_file(1, webp_shapes::FLAG_EXIF);
    let tags = listing(&rich, "red");
    for preset in 0..5 {
        let v = violations(preset, &tags);
        assert!(
            v.iter()
                .any(|t| t.starts_with("GPS:") || t.starts_with("Composite:GPS")),
            "preset {preset}: {v:?}"
        );
    }
    assert!(violations(1, &tags)
        .iter()
        .any(|t| t.contains("SerialNumber")));
    assert!(violations(2, &tags).iter().any(|t| t.contains("Artist")));
    assert!(!violations(1, &tags).iter().any(|t| t.contains("Artist")));
    // A carrier file unstripped prints tags the bare file does not (the leak
    // check of `check`), for a carrier and an opaque chunk.
    let base = keys(&listing(&bare(), "red-base"));
    for code in [b"LIST", b"CSET"] {
        let row = webp_carriers::ROWS
            .iter()
            .find(|r| &r.code == code)
            .expect("a row");
        let (_, input, _) = positions(&carrier(code, row))
            .into_iter()
            .next()
            .expect("a position");
        let leaked: Vec<String> = keys(&listing(&input, "red-carrier"))
            .difference(&base)
            .cloned()
            .collect();
        assert!(leaked.iter().any(|k| k == row.prints), "{leaked:?}");
    }
    // And a strip that returned its input is caught: judge the unstripped file
    // as if it were an output.
    let case = Case {
        name: "red-noop".into(),
        input: rich.clone(),
        judge: Judge::Pinned { xmp_gps: false },
        allow_refusal: false,
    };
    let before = listing(&case.input, "red-noop");
    assert!(!violations(0, &before).is_empty());
}

/// The complete judge, shown red both ways on the all-categories file: it holds
/// a tag of every one of the twelve categories (never vacuous), a strip that
/// removed nothing fails every preset, and a strip that removed too much
/// (`Everything`'s output judged as a `Gps` strip) is an over-removal failure.
#[test]
fn the_complete_judge_goes_red_when_a_strip_removes_too_little_or_too_much() {
    if !live_gate("the_complete_judge_goes_red_when_a_strip_removes_too_little_or_too_much") {
        return;
    }
    use webp_shapes::{chunk, riff, vp8l, vp8x, FLAG_EXIF, FLAG_ICC, FLAG_XMP};
    let input = riff(&[
        vp8x(FLAG_ICC | FLAG_EXIF | FLAG_XMP, 8, 8),
        chunk(b"ICCP", &oracle_png::icc_profile()),
        vp8l(),
        webp_shapes::exif_chunk(&all_categories_tiff()),
        chunk(b"XMP ", &xmp_plain()),
        c2pa_chunk(),
    ]);
    let before = listing(&input, "judge-before");
    let present = by_category(&before);
    for cat in ALL_CATS {
        assert!(present.contains_key(&cat), "{cat:?} is not in the file");
    }
    // Nothing removed: every preset has something left to complain about, and
    // each removes at least the category it adds.
    for preset in 0..5 {
        assert!(!judge(preset, &before, &before).is_empty(), "{preset}");
    }
    // Everything removed, judged as `Gps`: the kept categories are gone.
    let everything = strip(&input, StripPolicy::Everything).unwrap();
    let after = listing(everything.bytes(), "judge-everything");
    assert!(judge(4, &before, &after).is_empty());
    let over = judge(0, &before, &after);
    assert!(over.iter().any(|m| m.contains("over-removed")), "{over:?}");
    // And the honest `Gps` strip passes its own preset but not a higher one.
    let gps = strip(&input, StripPolicy::Gps).unwrap();
    let after = listing(gps.bytes(), "judge-gps");
    assert!(judge(0, &before, &after).is_empty());
    assert!(!judge(1, &before, &after).is_empty());
}

/// SPEC-038 fix round (verify L1): ExifTool prints the flags of every `VP8X`
/// with 4 or more data bytes, wherever it lies, and the last one wins. After
/// `Everything` none of them may still announce ICC, Exif or XMP; before the
/// strip each did (never vacuous).
#[test]
fn no_vp8x_prints_a_flag_for_a_chunk_the_strip_removed() {
    if !live_gate("no_vp8x_prints_a_flag_for_a_chunk_the_strip_removed") {
        return;
    }
    use webp_shapes::{chunk, riff, vp8l, vp8x, FLAG_EXIF, FLAG_ICC, FLAG_XMP};
    let all = FLAG_ICC | FLAG_EXIF | FLAG_XMP;
    let input = riff(&[
        vp8x(all | 0x10, 1, 1),
        chunk(b"VP8X", &[all, 0, 0, 0, 0, 0, 0, 0]),
        chunk(b"VP8X", &[all, 0, 0, 0]),
        chunk(b"ICCP", &oracle_png::icc_profile()),
        vp8l(),
        webp_shapes::exif_chunk(&rich_tiff()),
        chunk(b"XMP ", &xmp_plain()),
        vp8x(all, 1, 1),
    ]);
    let flags = |bytes: &[u8], label: &str| -> Vec<u32> {
        let path = tools::temp_file_with_ext(label, "webp", bytes).expect("temp file");
        let args: Vec<String> = ["-a", "-n", "-s3", "-RIFF:WebP_Flags"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let out = tools::run_tool_with_limit("exiftool", &args, &path, Duration::from_secs(8))
            .expect("exiftool");
        let _ = std::fs::remove_file(&path);
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.trim().parse().ok())
            .collect()
    };
    let before = flags(&input, "flags-before");
    assert_eq!(before.len(), 4, "{before:?}");
    assert!(before.iter().all(|f| f & u32::from(all) == u32::from(all)));
    let out = strip(&input, StripPolicy::Everything).unwrap();
    let after = flags(out.bytes(), "flags-after");
    assert_eq!(after.len(), 4, "{after:?}");
    assert!(
        after.iter().all(|f| f & u32::from(all) == 0),
        "a flag survives: {after:?}"
    );
}
