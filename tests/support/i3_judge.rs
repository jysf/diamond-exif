//! Layer A of the PNG and WebP write parity (SPEC-039, DEC-023 rule 3): the
//! category judge. A tag ExifTool prints is put into a category by the table
//! below, typed from DEC-008, DEC-009, DEC-020 (with its amendment of
//! 2026-11-24) and DEC-022, never read from the library. For one fixture and
//! one policy the judge compares ExifTool's reading of the input with its
//! reading of our output, in both directions:
//!
//! * every tag of a category the policy removes is gone;
//! * every other tag of the input, in a category the policy keeps or in no
//!   category at all (an Exif tag such as `Orientation` or `ISO`), is still
//!   printed with the same value;
//! * nothing is printed that the input did not print.
//!
//! Three exceptions, each tied to the rule that causes it and allowed only
//! where that rule fires (DEC-023 rule 3); no other exists:
//!
//! * (a) the carrier rule (DEC-021 clause 4, DEC-022 clause 4): the tags a
//!   carrier chunk prints go under every policy that removes carriers, the
//!   kept-category ones among them. Allowed only on a fixture that holds a
//!   carrier, and then the carrier's tags must in fact be gone;
//! * (b) the whole-block rule: a policy that holds every category an Exif
//!   entry can have (`all`, `everything`) removes the whole `eXIf`/`EXIF`
//!   chunk, so the uncategorised Exif tags go with it;
//! * (c) the stated residuals: `Composite:GPS*` derived from an XMP packet
//!   that a policy keeping `Xmp` leaves, and `JUMBF`/`JSON` contents under a
//!   policy that keeps `Jumbf` (those are simply a kept category).
//!
//! Shared by `tests/strip_png_webp_oracle.rs`, `tests/strip_png_live.rs` and
//! `tests/strip_webp_live.rs`.

#![allow(dead_code)]

use std::collections::BTreeMap;

use crate::i3_records::Line;
use crate::webp_carriers;

/// The DEC-008 categories a PNG or a WebP can hold and this judge classifies.
/// `Iptc` has no carrier in either container; `EmbeddedImage` and
/// `OtherSegment` are not read as tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cat {
    Gps,
    DeviceId,
    Authorship,
    Rights,
    FreeText,
    Timestamps,
    CameraModel,
    Thumbnail,
    MakerNote,
    Software,
    Xmp,
    Icc,
    Jumbf,
}

pub const ALL_CATS: [Cat; 13] = [
    Cat::Gps,
    Cat::DeviceId,
    Cat::Authorship,
    Cat::Rights,
    Cat::FreeText,
    Cat::Timestamps,
    Cat::CameraModel,
    Cat::Thumbnail,
    Cat::MakerNote,
    Cat::Software,
    Cat::Xmp,
    Cat::Icc,
    Cat::Jumbf,
];

/// The presets by name, smallest first (index = the DEC-009 column).
pub const PRESET_NAMES: [&str; 5] = ["gps", "gps_and_device", "all_pii", "all", "everything"];

/// The Exif categories (every category an Exif entry can have).
const EXIF_CATS: [Cat; 10] = [
    Cat::Gps,
    Cat::DeviceId,
    Cat::Authorship,
    Cat::Rights,
    Cat::FreeText,
    Cat::Timestamps,
    Cat::CameraModel,
    Cat::Thumbnail,
    Cat::MakerNote,
    Cat::Software,
];

/// DEC-009 (as amended): does `policy` remove `cat`? Typed here. A preset
/// removes what its column says; a `Custom` row removes its one category.
pub fn removes(policy: &str, cat: Cat) -> bool {
    if let Some(col) = PRESET_NAMES.iter().position(|p| *p == policy) {
        let first = match cat {
            Cat::Gps => 0,
            Cat::DeviceId => 1,
            Cat::Authorship
            | Cat::FreeText
            | Cat::Timestamps
            | Cat::CameraModel
            | Cat::Thumbnail
            | Cat::MakerNote => 2,
            Cat::Software | Cat::Xmp | Cat::Rights => 3,
            Cat::Icc | Cat::Jumbf => 4,
        };
        return col >= first;
    }
    match policy {
        "xmp" => cat == Cat::Xmp,
        "icc" => cat == Cat::Icc,
        "jumbf" => cat == Cat::Jumbf,
        "rights" => cat == Cat::Rights,
        "timestamps" => cat == Cat::Timestamps,
        "free_text" => cat == Cat::FreeText,
        "other" => false,
        other => panic!("no such recorded policy: {other}"),
    }
}

/// DEC-021 clause 4 and its SPEC-037 clarifications: carriers go whenever the
/// policy holds a tag category, `Xmp`, `Iptc` or `OtherSegment`; a `Custom`
/// policy of `Icc` or `Jumbf` leaves them.
pub fn removes_carriers(policy: &str) -> bool {
    !matches!(policy, "icc" | "jumbf")
}

/// Exception (b): a policy that holds every Exif category.
pub fn holds_every_exif_category(policy: &str) -> bool {
    EXIF_CATS.iter().all(|c| removes(policy, *c))
}

/// The groups an Exif block's tags print in.
fn exif_group(g: &str) -> bool {
    matches!(
        g,
        "IFD0" | "IFD1" | "IFD2" | "ExifIFD" | "InteropIFD" | "SubIFD" | "GPS" | "Trailer"
    )
}

fn xml_group(g: &str) -> bool {
    g.starts_with("XMP") || g.starts_with("XML") || g.starts_with("SEAL")
}

/// `PNG`-group tags that are structure (an `IHDR` field, a rendering chunk),
/// not text. Every other `PNG` tag is text (DEC-020 clause 4).
const PNG_STRUCTURE: [&str; 20] = [
    "ImageWidth",
    "ImageHeight",
    "BitDepth",
    "ColorType",
    "Compression",
    "Filter",
    "Interlace",
    "Gamma",
    "SRGBRendering",
    "PixelsPerUnitX",
    "PixelsPerUnitY",
    "PixelUnits",
    "Palette",
    "Transparency",
    "BackgroundColor",
    "SignificantBits",
    "WhitePointX",
    "WhitePointY",
    "AnimationFrames",
    "AnimationPlays",
];

/// The category a printed tag stands for (DEC-023 rule 3), or `None` for a
/// tag in no category (structure, pixel tags, `Unknown_*`, derived tags).
pub fn category_of(group: &str, tag: &str) -> Option<Cat> {
    if group == "GPS" || (matches!(group, "Composite" | "Trailer") && tag.starts_with("GPS")) {
        return Some(Cat::Gps);
    }
    if group == "Composite" {
        return None;
    }
    if group.starts_with("XMP") {
        return Some(Cat::Xmp);
    }
    if group.starts_with("ICC") {
        return Some(Cat::Icc);
    }
    if group == "JUMBF" || group == "JSON" {
        return Some(Cat::Jumbf);
    }
    if tag.starts_with("MakerNote") {
        return Some(Cat::MakerNote);
    }
    if group == "PNG" {
        return match tag {
            "Artist" | "Author" => Some(Cat::Authorship),
            "Copyright" => Some(Cat::Rights),
            "CreationTime" | "CreateDate" | "ModDate" | "TimeStamp" | "ModifyDate" => {
                Some(Cat::Timestamps)
            }
            "Software" => Some(Cat::Software),
            "Make" | "Model" => Some(Cat::CameraModel),
            "ICC_Profile" | "ProfileName" => Some(Cat::Icc),
            // An XMP `iTXt` with an unusable flag is printed as this binary tag.
            "XMP" => Some(Cat::Xmp),
            t if PNG_STRUCTURE.contains(&t) => None,
            _ => Some(Cat::FreeText),
        };
    }
    // The tags of an `EXIF` chunk's TIFF, not those a `RIFF`-group carrier
    // prints under the same name (`RIFF:Artist` from a `LIST`).
    if group == "RIFF" {
        return None;
    }
    match tag {
        "SerialNumber"
        | "BodySerialNumber"
        | "InternalSerialNumber"
        | "LensSerialNumber"
        | "ImageUniqueID" => Some(Cat::DeviceId),
        "Author" | "Artist" | "OwnerName" | "CameraOwnerName" => Some(Cat::Authorship),
        "Copyright" => Some(Cat::Rights),
        "ImageDescription" | "UserComment" => Some(Cat::FreeText),
        "ModifyDate" | "DateTimeOriginal" | "CreateDate" => Some(Cat::Timestamps),
        "Make" | "Model" | "LensModel" => Some(Cat::CameraModel),
        "ThumbnailImage" | "ThumbnailOffset" | "ThumbnailLength" => Some(Cat::Thumbnail),
        "Software" => Some(Cat::Software),
        _ => None,
    }
}

/// A line the judge looks at: it has a category, or it is a tag of an Exif
/// block (uncategorised Exif tags are kept by every policy but a whole-block
/// one). Warnings and errors are not judged.
fn judged(l: &Line) -> bool {
    if l.tag == "Warning" || l.tag == "Error" {
        return false;
    }
    category_of(&l.group, &l.tag).is_some() || exif_group(&l.group)
}

// ── the carriers a fixture holds, found by a naive scan ─────────────────────

/// What the bytes of a fixture hold that exception (a) is about.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Carriers {
    /// A PNG chunk that prints Exif tags (`zxIf` in any case, a mixed-case
    /// `eXIf`, a raw-profile `exif`/`APP1` text chunk).
    pub exif: bool,
    /// A PNG chunk that prints XMP/XML tags (`tXMP`, `meTa`, `seAl`, a
    /// raw-profile `xmp` chunk).
    pub xml: bool,
    /// The file also holds a plain Exif block (`eXIf`; WebP `EXIF`).
    pub exif_plain: bool,
    /// The file also holds a plain XMP packet (`XML:com.adobe.xmp`; WebP `XMP `).
    pub xml_plain: bool,
    /// WebP carrier rows present: the tag each prints (`Group:Tag`).
    pub row_tags: Vec<&'static str>,
}

impl Carriers {
    pub fn any(&self) -> bool {
        self.exif || self.xml || !self.row_tags.is_empty()
    }
}

fn be32(b: &[u8], at: usize) -> Option<usize> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize)
}

fn le32(b: &[u8], at: usize) -> Option<usize> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize)
}

/// A naive scan, written for the judge and sharing nothing with `src/`: every
/// chunk to the end of the file (a trailer after `IEND` too), the declared
/// lengths bounded by the file.
pub fn scan(ext: &str, bytes: &[u8]) -> Carriers {
    let mut c = Carriers::default();
    if ext == "png" {
        let mut at = 8usize;
        while let (Some(len), Some(ty)) = (be32(bytes, at), bytes.get(at + 4..at + 8)) {
            let data_at = at + 8;
            let data = bytes
                .get(data_at..data_at.saturating_add(len))
                .unwrap_or(&[]);
            let lower: Vec<u8> = ty.iter().map(u8::to_ascii_lowercase).collect();
            match ty {
                b"eXIf" => c.exif_plain = true,
                b"tXMP" | b"meTa" | b"seAl" => c.xml = true,
                _ if lower == b"zxif" || lower == b"exif" => c.exif = true,
                b"tEXt" | b"zTXt" | b"iTXt" => {
                    let key = data.split(|b| *b == 0).next().unwrap_or(&[]);
                    if key == b"XML:com.adobe.xmp" {
                        c.xml_plain = true;
                    } else if let Some(kind) = key.strip_prefix(b"Raw profile type ") {
                        match kind {
                            b"exif" | b"APP1" => c.exif = true,
                            b"xmp" => c.xml = true,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            at = match data_at.checked_add(len).and_then(|n| n.checked_add(4)) {
                Some(n) if n <= bytes.len() => n,
                _ => break,
            };
        }
    } else {
        let mut at = 12usize;
        while let (Some(code), Some(size)) = (bytes.get(at..at + 4), le32(bytes, at + 4)) {
            let data = bytes
                .get(at + 8..(at + 8).saturating_add(size))
                .unwrap_or(&[]);
            let code4 = [code[0], code[1], code[2], code[3]];
            match &code4 {
                b"EXIF" => c.exif_plain = true,
                b"XMP " | b"XMP\0" => c.xml_plain = true,
                _ => {}
            }
            for row in webp_carriers::ROWS.iter().filter(|r| r.region == "carrier") {
                if row.code != code4 {
                    continue;
                }
                // A `JUNK` chunk is a carrier by the text rule only (DEC-022).
                if &code4 == b"JUNK" && !is_text_junk(data) {
                    continue;
                }
                c.row_tags.push(row.prints);
                // A row that prints XMP/XML tags is an XML carrier: every tag of
                // those groups goes with it.
                if row
                    .prints
                    .split_once(':')
                    .is_some_and(|(g, _)| xml_group(g))
                {
                    c.xml = true;
                }
            }
            at = match (at + 8)
                .checked_add(size)
                .and_then(|n| n.checked_add(size & 1))
            {
                Some(n) if n < bytes.len() => n,
                _ => break,
            };
        }
    }
    c
}

/// DEC-022: printable ASCII, then NULs, with at least one text byte.
fn is_text_junk(data: &[u8]) -> bool {
    let end = data.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    end > 0 && data[..end].iter().all(|b| (0x20..=0x7E).contains(b))
}

// ── the judge ───────────────────────────────────────────────────────────────

/// How often each exception was used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Uses {
    pub carrier: usize,
    pub whole_block: usize,
    pub residual: usize,
}

impl std::ops::AddAssign for Uses {
    fn add_assign(&mut self, o: Uses) {
        self.carrier += o.carrier;
        self.whole_block += o.whole_block;
        self.residual += o.residual;
    }
}

#[derive(Clone, Debug, Default)]
pub struct Verdict {
    /// Each names the fixture, the policy and the tag.
    pub failures: Vec<String>,
    pub uses: Uses,
    /// Tags of the input in a category the policy removes (0: the fixture
    /// prints nothing removable, and is not counted as a removal).
    pub removable_before: usize,
}

fn multiset(lines: &[&Line]) -> BTreeMap<(String, String, String), usize> {
    let mut m = BTreeMap::new();
    for l in lines {
        *m.entry((l.group.clone(), l.tag.clone(), l.value.clone()))
            .or_insert(0) += 1;
    }
    m
}

/// Judge one output reading against the input reading.
pub fn judge(
    fixture: &str,
    ext: &str,
    bytes: &[u8],
    policy: &str,
    input: &[Line],
    output: &[Line],
) -> Verdict {
    let mut v = Verdict::default();
    let at = |what: &str, l: &(String, String, String)| {
        format!("{fixture} under {policy}: {what} {}:{}={}", l.0, l.1, l.2)
    };
    let carriers = scan(ext, bytes);
    let in_j: Vec<&Line> = input.iter().filter(|l| judged(l)).collect();
    let out_j: Vec<&Line> = output.iter().filter(|l| judged(l)).collect();
    let (in_m, out_m) = (multiset(&in_j), multiset(&out_j));

    let removed_cat =
        |l: &(String, String, String)| category_of(&l.0, &l.1).is_some_and(|c| removes(policy, c));
    v.removable_before = in_j
        .iter()
        .filter(|l| category_of(&l.group, &l.tag).is_some_and(|c| removes(policy, c)))
        .count();

    // The residual (c): a Composite GPS tag derived from an XMP packet the
    // output still prints, under a policy that keeps `Xmp`.
    let xmp_gps_left = !removes(policy, Cat::Xmp)
        && output
            .iter()
            .any(|l| l.group.starts_with("XMP") && l.tag.starts_with("GPS"));

    // Output lines the input did not have.
    let mut phantom: Vec<(String, String, String)> = Vec::new();
    for (k, n) in &out_m {
        let had = in_m.get(k).copied().unwrap_or(0);
        for _ in had..*n {
            phantom.push(k.clone());
        }
    }
    // Input lines the output lacks.
    let mut missing: Vec<(String, String, String)> = Vec::new();
    for (k, n) in &in_m {
        let now = out_m.get(k).copied().unwrap_or(0);
        for _ in now..*n {
            missing.push(k.clone());
        }
    }

    // A removed category must be gone: any output line of one that was there.
    for (k, n) in &out_m {
        if !removed_cat(k) {
            continue;
        }
        for _ in 0..*n {
            if k.0 == "Composite" && xmp_gps_left {
                v.uses.residual += 1;
            } else {
                v.failures.push(at("removed tag left in", k));
            }
        }
    }
    // Phantoms (a kept tag whose value changed shows as one missing and one
    // phantom line of the same name).
    for k in &phantom {
        if removed_cat(k) {
            continue; // already reported as left in
        }
        let name = (&k.0, &k.1);
        if missing.iter().any(|m| (&m.0, &m.1) == name) {
            v.failures.push(at("kept value changed", k));
        } else {
            v.failures.push(at("phantom tag", k));
        }
    }
    // Missing lines.
    for k in &missing {
        if removed_cat(k) {
            continue;
        }
        let in_exif = exif_group(&k.0);
        // The Composite GPS tags are derived from the carrier's own GPS tags.
        let row_gps = carriers
            .row_tags
            .iter()
            .any(|t| t.split_once(':').is_some_and(|(_, n)| n.starts_with("GPS")));
        let derived = k.0 == "Composite"
            && k.1.starts_with("GPS")
            && (row_gps || carriers.exif || carriers.xml);
        let carrier_line = derived
            || (carriers.exif && in_exif)
            || (carriers.xml && xml_group(&k.0))
            || carriers
                .row_tags
                .iter()
                .any(|t| t.split_once(':').is_some_and(|(g, _)| g == k.0));
        if carrier_line && removes_carriers(policy) {
            v.uses.carrier += 1;
        } else if in_exif && holds_every_exif_category(policy) {
            v.uses.whole_block += 1;
        } else {
            v.failures.push(at("kept tag missing", k));
        }
    }
    // The carrier rule fires: what the carriers print is gone. Only claimed
    // where no plain block of the same family prints the same groups.
    if removes_carriers(policy) {
        if carriers.exif && !carriers.exif_plain {
            for l in output
                .iter()
                .filter(|l| exif_group(&l.group) && l.tag != "Warning")
            {
                v.failures.push(format!(
                    "{fixture} under {policy}: carrier tag left in {}:{}={}",
                    l.group, l.tag, l.value
                ));
            }
        }
        if carriers.xml && !carriers.xml_plain {
            for l in output.iter().filter(|l| xml_group(&l.group)) {
                v.failures.push(format!(
                    "{fixture} under {policy}: carrier tag left in {}:{}={}",
                    l.group, l.tag, l.value
                ));
            }
        }
        for t in &carriers.row_tags {
            if output.iter().any(|l| l.key() == *t) {
                v.failures
                    .push(format!("{fixture} under {policy}: carrier tag left in {t}"));
            }
        }
    }
    v.failures.sort();
    v.failures.dedup();
    v
}
