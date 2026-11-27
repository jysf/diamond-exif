//! SPEC-025, AC9(b): the competeness sweep of the Exif block's omission table
//! against ExifTool's own list of tag ids. It reruns the two sweeps the design
//! measured (DEC-014's amendment of 2026-11-27) and turns red when a new
//! ExifTool relocates a tag the block still keeps.
//!
//! Candidates: every id in ExifTool's `Exif::Main` table (573 of them, from
//! `exiftool -listx -EXIF:All`) placed in the Exif IFD and, separately, in the
//! Interop IFD; every id in `GPS::Main` (32) placed in the GPS IFD. Each is a
//! LONG pointing at a mini-IFD 4 KiB away in a little-endian TIFF, so 1,178
//! sources. They are written from this file; ExifTool is run, never read or
//! linked.
//!
//! * Writer pass: `exiftool -m -Artist=sweep -o out/ in/` (`-m`, because
//!   ExifTool also refuses the swept 0xC612 and 0x927C with a `[minor]` error
//!   about the value 4096, which has nothing to do with an offset). A source
//!   ExifTool refuses with `No size tag for <IFD>:<Name>`, or one whose value comes back
//!   changed, holds a tag it relocates. A value that comes back unchanged does
//!   not.
//! * Reader pass: the pointed-at bytes zeroed, as a moved value would leave
//!   them; `exiftool -a -G1 -s -n -u` over both directories. A source whose
//!   listing changes holds a tag ExifTool reads through the offset.
//!
//! The assertions: the set ExifTool treats as offset-carrying in each IFD (the
//! union of the two passes) is a subset of what the block omits as
//! `OffsetCarrying` there, and the block omits nothing else beyond the
//! documented extras (`tests/support/exif_block_sweep_tables.rs`). A new
//! ExifTool that adds an offset tag fails this test.
//!
//! Live only, never replayed, never red-proofed (that is SPEC-026). Without
//! ExifTool the test prints `SKIP` and passes, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`.

#[path = "support/exif_block_sweep_tables.rs"]
mod exif_block_sweep_tables;
#[path = "support/sweep.rs"]
mod sweep;
#[path = "support/tools.rs"]
mod tools;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use exif_block_sweep_tables::{
    CONSERVATIVE_EXTRAS, MAKERNOTE, POINTER_EXTRAS, SUBIFD_TAG, WRITER_NO_SIZE_TAG,
};
use exif_core::write::{exif_block, BlockIfd, BlockOptions, OmitReason};
use sweep::{exiftool_listx, live_gate, Scratch};

/// Where the mini-IFD sits: 4 KiB into the file.
const TARGET: u32 = 4096;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    Exif,
    Interop,
    Gps,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::Exif => "exif",
            Kind::Interop => "interop",
            Kind::Gps => "gps",
        }
    }
    fn block_ifd(self) -> BlockIfd {
        match self {
            Kind::Exif => BlockIfd::Exif,
            Kind::Interop => BlockIfd::Interop,
            Kind::Gps => BlockIfd::Gps,
        }
    }
}

type Key = (Kind, u16);
type Ent = (u16, u16, u32, u32);

fn file_name(k: Key) -> String {
    format!("{}_{:04x}.tif", k.0.label(), k.1)
}

/// One IFD: count, entries `(tag, type, count, value)`, next-word 0.
fn ifd(ents: &[Ent]) -> Vec<u8> {
    let mut b = (ents.len() as u16).to_le_bytes().to_vec();
    for (t, ty, c, v) in ents {
        b.extend_from_slice(&t.to_le_bytes());
        b.extend_from_slice(&ty.to_le_bytes());
        b.extend_from_slice(&c.to_le_bytes());
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0u32.to_le_bytes());
    b
}

/// The source for `tag` in the IFD of `kind`: IFD0 holds the pointer (0x8769 or
/// 0x8825), the Exif IFD holds the Interop pointer for the Interop kind, the
/// swept IFD holds `tag` as a LONG, count 1, pointing at the mini-IFD (one
/// `Software` entry) at [`TARGET`].
fn source(key: Key) -> Vec<u8> {
    let (k, tag) = key;
    let probe: Ent = (tag, 4, 1, TARGET);
    let body: Vec<u8> = match k {
        Kind::Gps => [ifd(&[(0x8825, 4, 1, 26)]), ifd(&[probe])].concat(),
        Kind::Exif => [ifd(&[(0x8769, 4, 1, 26)]), ifd(&[probe])].concat(),
        Kind::Interop => [
            ifd(&[(0x8769, 4, 1, 26)]),
            ifd(&[(0xA005, 4, 1, 44)]),
            ifd(&[probe]),
        ]
        .concat(),
    };
    let mut f = b"II*\0".to_vec();
    f.extend_from_slice(&8u32.to_le_bytes());
    f.extend_from_slice(&body);
    f.resize(TARGET as usize, 0);
    let software = u32::from_le_bytes(*b"abc\0");
    f.extend_from_slice(&ifd(&[(0x0131, 2, 4, software)]));
    f
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

// ── reading a TIFF back (little- or big-endian), for the writer pass ────────

fn rd16(b: &[u8], at: usize, be: bool) -> Option<u16> {
    let v: [u8; 2] = b.get(at..at + 2)?.try_into().ok()?;
    Some(if be {
        u16::from_be_bytes(v)
    } else {
        u16::from_le_bytes(v)
    })
}

fn rd32(b: &[u8], at: usize, be: bool) -> Option<u32> {
    let v: [u8; 4] = b.get(at..at + 4)?.try_into().ok()?;
    Some(if be {
        u32::from_be_bytes(v)
    } else {
        u32::from_le_bytes(v)
    })
}

/// Every entry of the IFD at `at`.
fn entries(b: &[u8], at: u32, be: bool) -> Option<Vec<Ent>> {
    let at = at as usize;
    let n = rd16(b, at, be)? as usize;
    (0..n)
        .map(|i| {
            let e = at + 2 + 12 * i;
            Some((
                rd16(b, e, be)?,
                rd16(b, e + 2, be)?,
                rd32(b, e + 4, be)?,
                rd32(b, e + 8, be)?,
            ))
        })
        .collect()
}

fn find(ents: &[Ent], tag: u16) -> Option<Ent> {
    ents.iter().find(|e| e.0 == tag).copied()
}

/// The `(type, count, value)` of `key`'s entry in the output file, or `None`
/// when the entry, or an IFD on the way to it, is gone.
fn field_of(b: &[u8], key: Key) -> Option<(u16, u32, u32)> {
    let be = match b.get(..2)? {
        b"MM" => true,
        b"II" => false,
        _ => return None,
    };
    let ifd0 = entries(b, rd32(b, 4, be)?, be)?;
    let swept = match key.0 {
        Kind::Gps => entries(b, find(&ifd0, 0x8825)?.3, be)?,
        Kind::Exif => entries(b, find(&ifd0, 0x8769)?.3, be)?,
        Kind::Interop => {
            let exif = entries(b, find(&ifd0, 0x8769)?.3, be)?;
            entries(b, find(&exif, 0xA005)?.3, be)?
        }
    };
    find(&swept, key.1).map(|e| (e.1, e.2, e.3))
}

// ── ExifTool listings ───────────────────────────────────────────────────────

/// The lines of an `exiftool -a -G1 -s -n -u <dir>` listing by file name, less
/// the `System` group (directory and file times).
fn listing(stdout: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("======== ") {
            let name = path.rsplit('/').next().unwrap_or(path).to_string();
            out.entry(name.clone()).or_default();
            current = Some(name);
        } else if let (Some(name), true) = (&current, line.starts_with('[')) {
            if !line.starts_with("[System]") {
                out.entry(name.clone())
                    .or_default()
                    .insert(line.trim_end().to_string());
            }
        }
    }
    out
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

/// What the block builder does with a sweep source: the omission reasons of the
/// swept tag in the swept IFD, or `None` when it refuses the source.
fn block_verdict(bytes: &[u8], key: Key) -> Option<Vec<OmitReason>> {
    let plan = exif_block(bytes, &BlockOptions::new()).ok()?;
    Some(
        plan.omissions()
            .iter()
            .filter(|o| o.ifd() == key.0.block_ifd() && o.tag() == key.1)
            .map(|o| o.reason())
            .collect(),
    )
}

#[test]
fn exiftool_offset_tags_are_all_omitted() {
    let test = "exiftool_offset_tags_are_all_omitted";
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
    let keys: Vec<Key> = exif_ids
        .iter()
        .map(|t| (Kind::Exif, *t))
        .chain(exif_ids.iter().map(|t| (Kind::Interop, *t)))
        .chain(gps_ids.iter().map(|t| (Kind::Gps, *t)))
        .collect();

    // The sources, and the same with the pointed-at bytes zeroed.
    let scratch = Scratch::new("block");
    let (input, zeroed, output) = (
        scratch.0.join("in"),
        scratch.0.join("zero"),
        scratch.0.join("out"),
    );
    for dir in [&input, &zeroed, &output] {
        std::fs::create_dir_all(dir).expect("scratch subdir");
    }
    let mut sources: BTreeMap<Key, Vec<u8>> = BTreeMap::new();
    for key in &keys {
        let bytes = source(*key);
        std::fs::write(input.join(file_name(*key)), &bytes).expect("write source");
        let mut z = bytes.clone();
        z.get_mut(TARGET as usize..)
            .expect("target inside the file")
            .fill(0);
        std::fs::write(zeroed.join(file_name(*key)), &z).expect("write zeroed source");
        sources.insert(*key, bytes);
    }

    // Writer pass.
    let out_arg = format!("{}/", output.display());
    let limit = Duration::from_secs(300);
    let wrote = tools::run_tool_with_limit(
        "exiftool",
        &args(&["-m", "-Artist=sweep", "-o", &out_arg]),
        &input,
        limit,
    )
    .unwrap_or_else(|e| panic!("exiftool writer pass: {e:?}"));
    let stderr = String::from_utf8_lossy(&wrote.stderr).into_owned();
    let mut no_size_tag: BTreeSet<Key> = BTreeSet::new();
    let by_name: BTreeMap<String, Key> = keys.iter().map(|k| (file_name(*k), *k)).collect();
    for line in stderr.lines().filter(|l| l.contains("No size tag for")) {
        let file = line.rsplit(" - ").next().unwrap_or("");
        let file = file.rsplit('/').next().unwrap_or(file);
        if let Some(k) = by_name.get(file) {
            no_size_tag.insert(*k);
        }
    }
    let mut refused: BTreeSet<Key> = BTreeSet::new();
    let mut changed: BTreeSet<Key> = BTreeSet::new();
    let mut lost: BTreeSet<Key> = BTreeSet::new();
    let mut unchanged = 0usize;
    for key in &keys {
        let Ok(out) = std::fs::read(output.join(file_name(*key))) else {
            refused.insert(*key);
            continue;
        };
        match field_of(&out, *key) {
            Some((4, 1, v)) if v == TARGET => unchanged += 1,
            Some(_) => {
                changed.insert(*key);
            }
            None => {
                lost.insert(*key);
            }
        }
    }
    println!(
        "writer pass: {} sources; {} refused ({} with `No size tag`), {} changed, {} lost, {} unchanged",
        keys.len(),
        refused.len(),
        no_size_tag.len(),
        changed.len(),
        lost.len(),
        unchanged
    );

    // Reader pass.
    let read_args = args(&["-a", "-G1", "-s", "-n", "-u"]);
    let read = |dir: &std::path::Path| {
        let out = tools::run_tool_with_limit("exiftool", &read_args, dir, limit)
            .unwrap_or_else(|e| panic!("exiftool reader pass: {e:?}"));
        listing(&String::from_utf8_lossy(&out.stdout))
    };
    let (before, after) = (read(&input), read(&zeroed));
    let mut moved: BTreeSet<Key> = BTreeSet::new();
    let mut moved_lines: BTreeMap<Key, Vec<String>> = BTreeMap::new();
    for key in &keys {
        let name = file_name(*key);
        let a = before
            .get(&name)
            .unwrap_or_else(|| panic!("no listing {name}"));
        let b = after
            .get(&name)
            .unwrap_or_else(|| panic!("no listing {name}"));
        assert!(!a.is_empty(), "{name} printed nothing");
        if a != b {
            moved.insert(*key);
            moved_lines.insert(*key, a.symmetric_difference(b).cloned().collect());
        }
    }
    println!("reader pass: {} sources whose listing changes", moved.len());

    // The measured numbers of the design (ExifTool 13.55).
    let expected_refused: BTreeSet<Key> = [Kind::Exif, Kind::Interop]
        .iter()
        .flat_map(|k| WRITER_NO_SIZE_TAG.iter().map(move |t| (*k, *t)))
        .collect();
    let subifd: BTreeSet<Key> = [(Kind::Exif, SUBIFD_TAG), (Kind::Interop, SUBIFD_TAG)].into();
    assert_eq!(refused, expected_refused, "writer pass: refused sources");
    assert_eq!(refused.len(), 22);
    assert_eq!(no_size_tag, refused, "every refusal says `No size tag`");
    assert_eq!(changed, subifd, "writer pass: changed values");
    assert_eq!(
        lost,
        BTreeSet::from([(Kind::Exif, 0x013B)]),
        "writer pass: the one source that lost its Exif IFD (-Artist moves the tag)"
    );
    assert_eq!(moved, subifd, "reader pass: changed listings");
    for key in &subifd {
        let lines = &moved_lines[key];
        assert!(
            lines
                .iter()
                .any(|l| l.contains("SubIFD") && l.contains("Software")),
            "{key:?}: {lines:?}"
        );
    }

    // The assertion: ExifTool's offset set is inside the block's, and the block
    // omits nothing else beyond the documented extras.
    let exiftool_offset: BTreeSet<Key> = refused
        .union(&changed)
        .chain(moved.iter())
        .copied()
        .collect();
    let extras: BTreeSet<u16> = CONSERVATIVE_EXTRAS
        .iter()
        .chain(POINTER_EXTRAS.iter())
        .copied()
        .collect();
    let mut failures: Vec<String> = Vec::new();
    let mut omitted_extras = 0usize;
    for (key, bytes) in &sources {
        let verdict = block_verdict(bytes, *key);
        if exiftool_offset.contains(key) {
            if !verdict
                .as_ref()
                .is_some_and(|v| v.contains(&OmitReason::OffsetCarrying))
            {
                failures.push(format!(
                    "{key:?}: ExifTool relocates it, the block keeps it ({verdict:?})"
                ));
            }
        } else {
            match verdict {
                // A refusal is allowed only where the swept tag is a pointer.
                None if !extras.contains(&key.1) => {
                    failures.push(format!("{key:?}: the block refuses this source"));
                }
                Some(v) if !v.is_empty() => {
                    omitted_extras += 1;
                    // A MakerNote is omitted for being one, not for an offset.
                    let maker = v == [OmitReason::MakerNote] && MAKERNOTE.contains(&key.1);
                    if !maker && !extras.contains(&key.1) {
                        failures.push(format!(
                            "{key:?}: ExifTool does not relocate it, and the block omits it ({v:?}) beyond the documented extras"
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    println!(
        "sweep: {} sources; ExifTool offset-carrying {}; {} further omissions, all MakerNotes or documented extras",
        sources.len(),
        exiftool_offset.len(),
        omitted_extras
    );
    assert!(failures.is_empty(), "{failures:#?}");
}
