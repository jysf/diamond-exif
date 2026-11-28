//! SPEC-041, AC9(b): the cross-order completeness sweep of the `ByteOrder`
//! table against ExifTool's own list of tag ids. It reruns the sweep the design
//! measured (DEC-014's amendment of 2026-11-28, *Design measurements* of the
//! spec) and turns red when a new ExifTool reads one more tag by its own
//! format, or stops reading one the table leaves out.
//!
//! Candidates: every id of ExifTool's `Exif::Main` (from
//! `exiftool -listx -EXIF:All`) placed in the Exif IFD and, separately, in the
//! Interop IFD; every id of `GPS::Main` in the GPS IFD. Each is placed under
//! all 12 declared types. A one-byte type (1, 2, 6, 7) gets 7 payload shapes; a
//! multi-byte type gets one 16-byte payload. For each, a big-endian source is
//! written beside a little-endian copy whose value is swapped element by
//! element per its declared type (a one-byte value is the same bytes), and
//! `exiftool -j -n -b -a -G1 -u` reads both. A pair is *sensitive* when the two
//! readings differ; an undecoded blob (`base64:...`, which `-b` prints for data
//! ExifTool does not decode, ColorMap as SHORT for one) counts as equal when
//! its decoded length is equal, because `-n` without `-b` prints the same
//! `(Binary data N bytes)` for both.
//!
//! The assertions:
//!
//! * The sensitive set S (the `(IFD, tag, type)` triples with one differing
//!   pair), less the rule cases (CFAPattern and the UNICODE places), is a
//!   subset of the table, so a new ExifTool that reads one more tag by its own
//!   format fails here.
//! * The table, less S, is exactly OECF 0x8828, SpatialFrequencyResponse
//!   0xA20C and DeviceSettingDescription 0xA40B under every type in the Exif
//!   and the Interop IFD: ExifTool prints them as undecoded bytes, so it cannot
//!   see a difference. No other entry is left out needlessly.
//! * For the rule cases, the builder's own cross-order output, placed, reads
//!   the same as the source, in both directions.
//!
//! 0x014A is never given to ExifTool: it follows a SubIFD value that is not a
//! real IFD offset as an IFD, and hangs. Tags the block leaves out as
//! `OffsetCarrying` or `MakerNote` are read like any other here (the design did
//! the same) but sit outside the assertions: they never reach the converter.
//! Every ExifTool run carries an alarm (`perl -e 'alarm N; exec @ARGV'`; macOS
//! has no `timeout`), and no run is given `-m` (DEC-023).
//!
//! Live only, never replayed, never red-proofed (that is SPEC-026). Without
//! ExifTool the test prints `SKIP` and passes, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`.

#[path = "support/exif_block_place.rs"]
mod exif_block_place;
#[path = "support/exif_block_shapes.rs"]
mod exif_block_shapes;
#[path = "support/exif_block_sweep_tables.rs"]
mod exif_block_sweep_tables;
#[path = "support/sweep.rs"]
mod sweep;
#[path = "support/tools.rs"]
mod tools;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use exif_block_place::place;
use exif_block_shapes::{cfa_cases, elem_size, one_entry_source, unicode_cases, Enc};
use exif_block_sweep_tables::{byte_order_triples, MAKERNOTE, NOT_SEEN_BY_THE_SWEEP};
use exif_core::reader::ByteOrder;
use exif_core::write::{exif_block, BlockOptions};
use sweep::{exiftool_listx, live_gate, Scratch};

/// `(IFD, tag, type)`: IFD 0 is the Exif IFD, 1 the Interop IFD, 2 the GPS IFD.
type Triple = (u8, u16, u16);

/// ExifTool hangs on a 0x014A whose value is not a real IFD offset.
const NEVER_GIVEN: u16 = 0x014A;

/// Pairs per ExifTool run (two files each).
const CHUNK: usize = 500;

fn which_of(ifd: u8) -> &'static str {
    match ifd {
        0 => "exif",
        1 => "interop",
        _ => "gps",
    }
}

fn comment(code: &[u8; 8], text: &[u8]) -> Vec<u8> {
    let mut v = code.to_vec();
    v.extend_from_slice(text);
    v
}

/// The seven payload shapes of a one-byte type (the design's), as `(name,
/// bytes)`: CFA-like dimensions, OECF-like, UNICODE with Latin UCS-2, UNICODE
/// with CJK UCS-2, bare UTF-16 CJK, DSD-like, and 16 counting bytes. The CJK
/// text is U+4E2D U+6587: ExifTool guesses the order of Latin UCS-2, so only
/// CJK shows a wrong order.
fn one_byte_payloads() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("cfa", vec![0, 2, 0, 2, 0, 1, 1, 2]),
        (
            "oecf",
            vec![0, 2, 0, 2, b'A', 0, b'B', 0, 1, 2, 3, 4, 5, 6, 7, 8],
        ),
        ("unicode-latin", comment(b"UNICODE\0", &[0, b'A', 0, b'B'])),
        (
            "unicode-cjk",
            comment(b"UNICODE\0", &[0x4E, 0x2D, 0x65, 0x87]),
        ),
        ("utf16-cjk", vec![0x4E, 0x2D, 0x65, 0x87]),
        ("dsd", vec![0, 2, 0, 1, 0, b'X', 0, 0, 0, 1, 0, 2]),
        ("counting", (1..=16).collect()),
    ]
}

/// The multi-byte payload: the 16 counting bytes 0 to 15 as logical values,
/// each swap unit written in the given order. SPEC-041's design does not pin
/// these bytes (question `exif-block-order-sweep-payload`); this choice makes
/// the table less the sensitive set exactly OECF, SFR and DSD, and it shows
/// the entries listed in that question as sensitive although the table lacks
/// them. A printable payload reads some string tags (ColorMap, ThumbnailData,
/// GeoTiff params) as differing text where the design counted them equal.
fn counting_value(be: bool, ty: u16) -> Vec<u8> {
    let unit = match ty {
        3 | 8 => 2,
        12 => 8,
        _ => 4,
    };
    let logical: Vec<u8> = (0..16).collect();
    let mut out = Vec::new();
    for u in logical.chunks(unit) {
        if be {
            out.extend_from_slice(u);
        } else {
            out.extend(u.iter().rev());
        }
    }
    out
}

/// One swept source pair: the value as the big-endian source holds it and as
/// the little-endian copy holds it.
struct Pair {
    triple: Triple,
    payload: usize,
    count: u32,
    be_value: Vec<u8>,
    le_value: Vec<u8>,
}

fn pairs_of(ifd: u8, tag: u16) -> Vec<Pair> {
    let mut out = Vec::new();
    let shapes = one_byte_payloads();
    for ty in 1..=12u16 {
        if elem_size(ty) == 1 {
            for (p, (_, bytes)) in shapes.iter().enumerate() {
                out.push(Pair {
                    triple: (ifd, tag, ty),
                    payload: p,
                    count: bytes.len() as u32,
                    be_value: bytes.clone(),
                    le_value: bytes.clone(),
                });
            }
        } else {
            let count = (16 / elem_size(ty)) as u32;
            out.push(Pair {
                triple: (ifd, tag, ty),
                payload: 0,
                count,
                be_value: counting_value(true, ty),
                le_value: counting_value(false, ty),
            });
        }
    }
    out
}

fn stem(p: &Pair) -> String {
    let (ifd, tag, ty) = p.triple;
    format!("{ifd}_{tag:04x}_{ty}_{}", p.payload)
}

fn source_of(p: &Pair, be: bool) -> Vec<u8> {
    let (ifd, tag, ty) = p.triple;
    let value = if be { &p.be_value } else { &p.le_value };
    one_entry_source(
        be,
        which_of(ifd),
        Enc { be }.ent(tag, ty, p.count, value.clone()),
    )
}

/// The ids of one table of an `exiftool -listx` document, decimal or `0x` hex.
fn table_ids(xml: &str, table: &str) -> BTreeSet<u16> {
    let open = format!("<table name='{table}'");
    let Some(start) = xml.find(&open) else {
        return BTreeSet::new();
    };
    let body = &xml[start..];
    let body = &body[..body.find("</table>").unwrap_or(body.len())];
    let mut out = BTreeSet::new();
    let mut rest = body;
    while let Some(at) = rest.find("<tag id='") {
        rest = &rest[at + "<tag id='".len()..];
        let Some(end) = rest.find('\'') else { break };
        let raw = &rest[..end];
        let n = match raw.strip_prefix("0x") {
            Some(h) => u32::from_str_radix(h, 16).ok(),
            None => raw.parse::<u32>().ok(),
        };
        if let Some(id) = n.and_then(|n| u16::try_from(n).ok()) {
            out.insert(id);
        }
    }
    out
}

// ── reading `exiftool -j` ───────────────────────────────────────────────────

/// The end of the JSON value that starts at `i` (after whitespace).
fn skip_value(b: &[u8], mut i: usize) -> usize {
    while b.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
        i += 1;
    }
    match b.get(i) {
        Some(b'"') => {
            i += 1;
            while let Some(c) = b.get(i) {
                match c {
                    b'\\' => i += 2,
                    b'"' => return i + 1,
                    _ => i += 1,
                }
            }
            i
        }
        Some(b'[' | b'{') => {
            let mut depth = 0usize;
            while let Some(c) = b.get(i) {
                match c {
                    b'"' => {
                        i = skip_value(b, i);
                        continue;
                    }
                    b'[' | b'{' => depth += 1,
                    b']' | b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return i + 1;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            i
        }
        _ => {
            while b
                .get(i)
                .is_some_and(|c| !matches!(c, b',' | b'}' | b']') && !c.is_ascii_whitespace())
            {
                i += 1;
            }
            i
        }
    }
}

/// `exiftool -j` output as one map per file: its name (the part of
/// `SourceFile` after the last `/`) and its `Group:Tag` keys with each value's
/// raw JSON text. `System`, `File` and `SourceFile` are dropped: directory
/// times, sizes and the file's own name.
fn parse_json(text: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let b = text.as_bytes();
    let mut files = BTreeMap::new();
    let mut i = 0usize;
    while let Some(c) = b.get(i) {
        if *c != b'{' {
            i += 1;
            continue;
        }
        i += 1;
        let mut obj: BTreeMap<String, String> = BTreeMap::new();
        loop {
            while b
                .get(i)
                .is_some_and(|c| c.is_ascii_whitespace() || *c == b',')
            {
                i += 1;
            }
            if b.get(i) != Some(&b'"') {
                i += 1; // the closing brace
                break;
            }
            let key_end = skip_value(b, i);
            let key = text.get(i + 1..key_end - 1).unwrap_or("").to_string();
            i = key_end;
            while b
                .get(i)
                .is_some_and(|c| c.is_ascii_whitespace() || *c == b':')
            {
                i += 1;
            }
            let end = skip_value(b, i);
            let value = text.get(i..end).unwrap_or("").to_string();
            i = end;
            obj.insert(key, value);
        }
        let name = obj
            .remove("SourceFile")
            .map(|s| {
                let s = s.trim_matches('"').to_string();
                s.rsplit('/').next().unwrap_or("").to_string()
            })
            .unwrap_or_default();
        obj.retain(|k, _| !k.starts_with("System:") && !k.starts_with("File:"));
        files.insert(name, obj);
    }
    files
}

/// The decoded length of a `"base64:..."` JSON string, or `None` for any other
/// value.
fn blob_len(raw: &str) -> Option<usize> {
    let body = raw.strip_prefix("\"base64:")?.strip_suffix('"')?;
    let pad = body.bytes().rev().take_while(|c| *c == b'=').count();
    Some((body.len() / 4 * 3).saturating_sub(pad))
}

/// Whether two readings (the keys of one file each) are the same, judged as in
/// the design; the differing keys when they are not.
fn differences(a: &BTreeMap<String, String>, b: &BTreeMap<String, String>) -> Vec<String> {
    let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    keys.into_iter()
        .filter_map(|k| {
            let (x, y) = (a.get(k), b.get(k));
            let same = match (x, y) {
                (Some(x), Some(y)) => {
                    x == y || matches!((blob_len(x), blob_len(y)), (Some(m), Some(n)) if m == n)
                }
                _ => false,
            };
            (!same).then(|| format!("{k}: {x:?} vs {y:?}"))
        })
        .collect()
}

// ── running ExifTool ────────────────────────────────────────────────────────

/// `exiftool -j -n -b -a -G1 -u` over `files` under an `alarm` (seconds), as
/// `perl -e 'alarm N; exec @ARGV' -- exiftool ...`. No `-m`.
fn exiftool_json(files: &[PathBuf], alarm: u64) -> Result<String, String> {
    let Some((last, rest)) = files.split_last() else {
        return Ok(String::new());
    };
    let mut args: Vec<String> = [
        "-e",
        &format!("alarm {alarm}; exec @ARGV"),
        "--",
        "exiftool",
        "-j",
        "-n",
        "-b",
        "-a",
        "-G1",
        "-u",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    args.extend(rest.iter().map(|p| p.display().to_string()));
    let out = tools::run_tool_with_limit("perl", &args, last, Duration::from_secs(alarm + 5))
        .map_err(|e| format!("{e:?}"))?;
    // A run killed by its alarm leaves no complete array.
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    if text.trim_end().ends_with(']') {
        Ok(text)
    } else {
        Err(format!(
            "no complete listing (alarm?): {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

fn workers() -> usize {
    std::thread::available_parallelism().map_or(2, |n| n.get().min(8))
}

/// Spread `jobs` over a few threads.
fn par<T: Sync, R: Send>(jobs: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let w = workers();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..w)
            .map(|k| {
                let f = &f;
                scope.spawn(move || {
                    jobs.iter()
                        .skip(k)
                        .step_by(w)
                        .map(|j| f(j))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("worker"))
            .collect()
    })
}

/// A listing of `paths` (a batch under a long alarm; on a failure, one file
/// at a time under the 8 s alarm, so one hung file is named and does not take
/// the batch with it). Failures are returned as `(file, why)`.
fn read_all(paths: &[PathBuf]) -> (BTreeMap<String, BTreeMap<String, String>>, Vec<String>) {
    match exiftool_json(paths, 120) {
        Ok(text) => (parse_json(&text), Vec::new()),
        Err(_) => {
            let mut all = BTreeMap::new();
            let mut failed = Vec::new();
            for p in paths {
                match exiftool_json(std::slice::from_ref(p), 8) {
                    Ok(text) => all.extend(parse_json(&text)),
                    Err(e) => failed.push(format!("{}: {e}", p.display())),
                }
            }
            (all, failed)
        }
    }
}

// ── the sweep ───────────────────────────────────────────────────────────────

/// The rule cases (DEC-014 amendment items 4 and 5): where the builder
/// converts by a rule rather than leaving the entry out.
fn is_rule_case((ifd, tag, ty): Triple) -> bool {
    match ifd {
        2 => matches!(tag, 0x001B | 0x001C) && ty == 7,
        _ => (tag == 0xA302 && ty == 7) || (tag == 0x9286 && matches!(ty, 1 | 2 | 6 | 7)),
    }
}

/// What the block leaves out before any `ByteOrder` check (offset-carrying,
/// MakerNote): typed by hand from the measurement, for the report only.
fn outside_the_converter(tag: u16) -> bool {
    MAKERNOTE.contains(&tag)
        || [
            0x0111u16, 0x0117, 0x0120, 0x0121, 0x0144, 0x0145, 0x014A, 0x0190, 0x0201, 0x0202,
            0x0207, 0x0208, 0x0209, 0x8290, 0x8568, 0x8769, 0x8781, 0x8825, 0x888A, 0xA005, 0xA010,
            0xA011, 0xBCC0, 0xBCC1, 0xBCC2, 0xBCC3, 0xC51B, 0xC6F5, 0xCD41, 0xFE00,
        ]
        .contains(&tag)
}

#[test]
fn cross_order_sweep_matches_byte_order_table() {
    let test = "cross_order_sweep_matches_byte_order_table";
    if !live_gate(test) {
        return;
    }
    let listx = exiftool_listx();
    let exif_ids = table_ids(&listx, "Exif::Main");
    let gps_ids = table_ids(&listx, "GPS::Main");
    assert!(
        exif_ids.len() > 500 && gps_ids.len() > 25,
        "{} Exif ids, {} GPS ids: did -listx change shape?",
        exif_ids.len(),
        gps_ids.len()
    );

    let mut pairs: Vec<Pair> = Vec::new();
    let mut keys = 0usize;
    for (ifd, ids) in [(0u8, &exif_ids), (1, &exif_ids), (2, &gps_ids)] {
        for tag in ids.iter().filter(|t| **t != NEVER_GIVEN) {
            keys += 1;
            pairs.extend(pairs_of(ifd, *tag));
        }
    }

    let scratch = Scratch::new("block-order");
    for p in &pairs {
        for (be, suffix) in [(true, "be"), (false, "le")] {
            std::fs::write(
                scratch.0.join(format!("{}_{suffix}.tif", stem(p))),
                source_of(p, be),
            )
            .expect("write source");
        }
    }

    let chunks: Vec<&[Pair]> = pairs.chunks(CHUNK).collect();
    let results = par(&chunks, |chunk| {
        let paths: Vec<PathBuf> = chunk
            .iter()
            .flat_map(|p| ["be", "le"].map(|s| scratch.0.join(format!("{}_{s}.tif", stem(p)))))
            .collect();
        let (listed, failed) = read_all(&paths);
        let mut sensitive: Vec<(Triple, String)> = Vec::new();
        let mut missing: Vec<String> = Vec::new();
        for p in chunk.iter() {
            let (be, le) = (
                listed.get(&format!("{}_be.tif", stem(p))),
                listed.get(&format!("{}_le.tif", stem(p))),
            );
            let (Some(be), Some(le)) = (be, le) else {
                missing.push(stem(p));
                continue;
            };
            let d = differences(be, le);
            if !d.is_empty() {
                sensitive.push((p.triple, format!("{} {}", stem(p), d.join("; "))));
            }
        }
        (sensitive, missing, failed)
    });
    let mut differing: Vec<(Triple, String)> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for (s, m, f) in results {
        differing.extend(s);
        problems.extend(m.into_iter().map(|m| format!("no listing for {m}")));
        problems.extend(f);
    }
    assert!(
        problems.is_empty(),
        "{} problems: {problems:#?}",
        problems.len()
    );
    assert_eq!(
        keys * 36,
        pairs.len(),
        "12 types, 7 shapes for each one-byte type and 1 for each other"
    );

    let mut sensitive: BTreeMap<Triple, usize> = BTreeMap::new();
    for (t, _) in &differing {
        *sensitive.entry(*t).or_default() += 1;
    }
    let s: BTreeSet<Triple> = sensitive.keys().copied().collect();
    let per_ifd = |set: &BTreeSet<Triple>, ifd: u8| set.iter().filter(|t| t.0 == ifd).count();
    let table: BTreeSet<Triple> = byte_order_triples().into_iter().collect();

    // S less the rule cases, and less what never reaches the converter, must
    // be inside the table.
    let s_less_rules: BTreeSet<Triple> = s.iter().copied().filter(|t| !is_rule_case(*t)).collect();
    let mut failures: Vec<String> = Vec::new();
    let mut tolerated: Vec<Triple> = Vec::new();
    for t in &s_less_rules {
        if !table.contains(t) && outside_the_converter(t.1) {
            tolerated.push(*t);
        }
        if !table.contains(t) && !outside_the_converter(t.1) {
            let sample: Vec<&String> = differing
                .iter()
                .filter(|(x, _)| x == t)
                .map(|(_, d)| d)
                .take(2)
                .collect();
            failures.push(format!(
                "ExifTool reads {t:?} by its own format and the table lacks it: {sample:?}"
            ));
        }
    }
    // The table, less S, is exactly the three ExifTool cannot see.
    let unseen: BTreeSet<Triple> = table.difference(&s).copied().collect();
    let expected_unseen: BTreeSet<Triple> = [0u8, 1]
        .iter()
        .flat_map(|ifd| {
            NOT_SEEN_BY_THE_SWEEP
                .iter()
                .flat_map(move |tag| (1..=12u16).map(move |ty| (*ifd, *tag, ty)))
        })
        .collect();
    if unseen != expected_unseen {
        failures.push(format!(
            "table less sensitive set: extra {:?}, missing {:?}",
            unseen.difference(&expected_unseen).collect::<Vec<_>>(),
            expected_unseen.difference(&unseen).collect::<Vec<_>>()
        ));
    }
    println!(
        "sweep: {keys} keys, {} pairs ({} files); {} pairs differ; sensitive set {} triples \
         (Exif {}, Interop {}, GPS {}); less rule cases {} (Exif {}, Interop {}, GPS {}); \
         table {} triples, of which {} not seen by the sweep",
        pairs.len(),
        pairs.len() * 2,
        differing.len(),
        s.len(),
        per_ifd(&s, 0),
        per_ifd(&s, 1),
        per_ifd(&s, 2),
        s_less_rules.len(),
        per_ifd(&s_less_rules, 0),
        per_ifd(&s_less_rules, 1),
        per_ifd(&s_less_rules, 2),
        table.len(),
        unseen.len(),
    );
    println!("sensitive but never converted (offset-carrying or MakerNote): {tolerated:?}");
    // The rule cases do not depend on the sweep's verdict: run them first, so
    // a red sweep still reports them.
    rule_cases(&scratch.0);
    assert!(
        failures.is_empty(),
        "{} failures: {failures:#?}",
        failures.len()
    );
}

/// The rule cases: the builder's own cross-order output, placed, reads as the
/// source, in both directions (AC9(b), last bullet).
fn rule_cases(dir: &Path) {
    let dir = dir.join("rules");
    std::fs::create_dir_all(&dir).expect("rules dir");
    // Every place a rule applies, with the shapes that probe it.
    let mut combos: Vec<(u8, u16, u16)> = Vec::new();
    for ifd in [0u8, 1] {
        combos.push((ifd, 0xA302, 7));
        for ty in [1u16, 2, 6, 7] {
            combos.push((ifd, 0x9286, ty));
        }
    }
    combos.push((2, 0x001B, 7));
    combos.push((2, 0x001C, 7));
    let shapes = one_byte_payloads();

    struct Case {
        name: String,
        be: bool,
        triple: Triple,
        value: Vec<u8>,
    }
    let mut cases: Vec<Case> = Vec::new();
    for (ifd, tag, ty) in combos.iter().copied() {
        let mut values: Vec<(String, Vec<u8>)> = shapes
            .iter()
            .map(|(n, v)| ((*n).to_string(), v.clone()))
            .collect();
        if tag == 0xA302 {
            values.extend(cfa_cases().into_iter().map(|(n, v, _)| (n.to_string(), v)));
        } else {
            values.extend(
                unicode_cases()
                    .into_iter()
                    .map(|(n, v, _)| (n.to_string(), v)),
            );
        }
        for (n, value) in values {
            for be in [true, false] {
                cases.push(Case {
                    name: format!("{ifd}_{tag:04x}_{ty}_{n}_{}", if be { "be" } else { "le" }),
                    be,
                    triple: (ifd, tag, ty),
                    value: value.clone(),
                });
            }
        }
    }
    let mut paths: Vec<(PathBuf, PathBuf)> = Vec::new();
    for c in &cases {
        let (ifd, tag, ty) = c.triple;
        let bytes = one_entry_source(
            c.be,
            which_of(ifd),
            Enc { be: c.be }.ent(tag, ty, c.value.len() as u32, c.value.clone()),
        );
        let other = if c.be {
            ByteOrder::Little
        } else {
            ByteOrder::Big
        };
        let plan = exif_block(&bytes, &BlockOptions::new().with_byte_order(other))
            .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert!(
            plan.omissions().is_empty(),
            "{}: a rule case is converted, not left out: {:?}",
            c.name,
            plan.omissions()
        );
        let block = plan.write(8).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        let placed = place(!c.be, 8, &block, plan.exif_position(), plan.gps_position());
        let (src, out) = (
            dir.join(format!("{}_src.tif", c.name)),
            dir.join(format!("{}_out.tif", c.name)),
        );
        std::fs::write(&src, &bytes).expect("write source");
        std::fs::write(&out, &placed).expect("write placed");
        paths.push((src, out));
    }
    let chunks: Vec<&[(PathBuf, PathBuf)]> = paths.chunks(CHUNK).collect();
    let results = par(&chunks, |chunk| {
        let files: Vec<PathBuf> = chunk
            .iter()
            .flat_map(|(a, b)| [a.clone(), b.clone()])
            .collect();
        let (listed, failed) = read_all(&files);
        let mut bad: Vec<String> = failed;
        for (src, out) in chunk.iter() {
            let name = |p: &PathBuf| {
                p.file_name()
                    .map_or(String::new(), |n| n.to_string_lossy().into_owned())
            };
            let (Some(a), Some(b)) = (listed.get(&name(src)), listed.get(&name(out))) else {
                bad.push(format!("no listing for {}", name(src)));
                continue;
            };
            // The three block IFDs, exactly; a warning only in the placed file.
            let block_keys = |m: &BTreeMap<String, String>| -> BTreeMap<String, String> {
                m.iter()
                    .filter(|(k, _)| {
                        ["ExifIFD:", "InteropIFD:", "GPS:"]
                            .iter()
                            .any(|g| k.starts_with(g))
                    })
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            };
            let d = differences(&block_keys(a), &block_keys(b));
            if !d.is_empty() {
                bad.push(format!("{}: {}", name(src), d.join("; ")));
            }
            for (k, v) in b.iter().filter(|(k, _)| k.ends_with(":Warning")) {
                if a.get(k) != Some(v) {
                    bad.push(format!("{}: new warning {v}", name(src)));
                }
            }
        }
        bad
    });
    let bad: Vec<String> = results.into_iter().flatten().collect();
    println!(
        "rule cases: {} places, {} cases ({} pairs: source and placed output); {} differ",
        combos.len(),
        cases.len(),
        cases.len(),
        bad.len()
    );
    assert!(
        bad.is_empty(),
        "{} rule-case differences: {bad:#?}",
        bad.len()
    );
}
