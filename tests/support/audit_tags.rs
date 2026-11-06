//! The live check of the DEC-008 tag table against ExifTool (SPEC-012).
//!
//! For each distinct tag id in the table, [`fixture`] builds a one-entry TIFF
//! (the type is chosen per tag), ExifTool is run on it as
//! `exiftool -G1 -a -s -u <file>`, and the name it gives the id is recorded in
//! `tests/oracle-fixtures/exiftool/audit-tag-names.txt`. The test carries the
//! name table ([`ROWS`]) and asserts it equals that record (replay and live),
//! and separately compares the recorded names with DEC-008's tag names; every
//! row where they differ is a recorded question, never an edit of the DEC.
//!
//! The builder here is independent of `src/`, of `oracle_tiff` and of
//! `examples/fuzz_seeds.rs`. ExifTool is run, never read or linked. std only.
//! Test support only: `#[path]`-included by `tests/audit_oracle.rs` and by
//! `examples/oracle_record.rs`; it needs the `tools` module at the crate root.

#![allow(dead_code)]

use crate::tools;
use std::path::Path;

/// The path of the committed record, relative to the repository root.
pub const RECORD: &str = "tests/oracle-fixtures/exiftool/audit-tag-names.txt";

/// The fixed arguments (the file path is added by `tools::run_tool`).
pub const ARGS: [&str; 4] = ["-G1", "-a", "-s", "-u"];

/// ExifTool groups that describe the file rather than a tag in it; their
/// lines are never part of a record.
pub const FILE_LEVEL_GROUPS: [&str; 4] = ["System", "File", "ExifTool", "Composite"];

/// How a row's fixture is built.
#[derive(Clone, Copy)]
pub enum Shape {
    /// An ASCII entry in IFD0.
    Ascii(&'static [u8]),
    /// An UNDEFINED entry in IFD0.
    Undef(&'static [u8]),
    /// A BYTE entry in IFD0.
    Bytes(&'static [u8]),
    /// IFD0 (one SHORT `Orientation`) chained to an IFD1 holding a LONG entry
    /// with this tag, and a LONG `0x0202`.
    Ifd1Long,
}

/// One row of the name check.
pub struct Row {
    /// The tag id.
    pub id: u16,
    /// The name DEC-008 gives the tag.
    pub dec_name: &'static str,
    /// How the one-entry fixture is built.
    pub shape: Shape,
    /// The name ExifTool 13.55 was measured to give the id (the committed
    /// record is the evidence; the test asserts they agree).
    pub exiftool_name: &'static str,
    /// The open question that records a difference from `dec_name`, if any.
    pub question: Option<&'static str>,
}

const TEXT: &[u8] = b"test\0";
const UTF16: &[u8] = b"t\0e\0s\0t\0\0\0";

/// The recorded question for a row whose ExifTool name differs from DEC-008's:
/// `audit-table-exiftool-names` (the same tag under another name; the id
/// mismatches of `audit-table-exif3-tag-ids` were fixed by the DEC-008
/// amendment).
fn r(id: u16, dec: &'static str, shape: Shape, tool: &'static str) -> Row {
    let question = if dec == tool {
        None
    } else {
        Some("audit-table-exiftool-names")
    };
    Row {
        id,
        dec_name: dec,
        shape,
        exiftool_name: tool,
        question,
    }
}

/// The rows, in DEC-008 table order. `exiftool_name` is filled from the
/// committed record (`just oracle-record`), never from memory.
pub fn rows() -> Vec<Row> {
    use Shape::*;
    vec![
        r(0x010E, "ImageDescription", Ascii(TEXT), "ImageDescription"),
        r(0x010F, "Make", Ascii(TEXT), "Make"),
        r(0x0110, "Model", Ascii(TEXT), "Model"),
        r(0x0131, "Software", Ascii(TEXT), "Software"),
        r(
            0x0132,
            "DateTime",
            Ascii(b"2024:05:06 07:08:09\0"),
            "ModifyDate",
        ),
        r(0x013B, "Artist", Ascii(TEXT), "Artist"),
        r(0x8298, "Copyright", Ascii(TEXT), "Copyright"),
        r(
            0x9003,
            "DateTimeOriginal",
            Ascii(b"2024:05:06 07:08:09\0"),
            "DateTimeOriginal",
        ),
        r(
            0x9004,
            "DateTimeDigitized",
            Ascii(b"2024:05:06 07:08:09\0"),
            "CreateDate",
        ),
        r(0x9010, "OffsetTime", Ascii(b"+02:00\0"), "OffsetTime"),
        r(
            0x9011,
            "OffsetTimeOriginal",
            Ascii(b"+02:00\0"),
            "OffsetTimeOriginal",
        ),
        r(
            0x9012,
            "OffsetTimeDigitized",
            Ascii(b"+02:00\0"),
            "OffsetTimeDigitized",
        ),
        r(0x9290, "SubSecTime", Ascii(b"123\0"), "SubSecTime"),
        r(
            0x9291,
            "SubSecTimeOriginal",
            Ascii(b"123\0"),
            "SubSecTimeOriginal",
        ),
        r(
            0x9292,
            "SubSecTimeDigitized",
            Ascii(b"123\0"),
            "SubSecTimeDigitized",
        ),
        r(
            0x9286,
            "UserComment",
            Undef(b"ASCII\0\0\0test"),
            "UserComment",
        ),
        r(
            0x927C,
            "MakerNote",
            Undef(b"unparsed-maker-note"),
            "MakerNoteUnknownText",
        ),
        r(0xA420, "ImageUniqueID", Ascii(TEXT), "ImageUniqueID"),
        r(0xA430, "CameraOwnerName", Ascii(TEXT), "OwnerName"),
        r(0xA431, "BodySerialNumber", Ascii(TEXT), "SerialNumber"),
        r(0xA435, "LensSerialNumber", Ascii(TEXT), "LensSerialNumber"),
        r(0xA433, "LensMake", Ascii(TEXT), "LensMake"),
        r(0xA434, "LensModel", Ascii(TEXT), "LensModel"),
        r(
            0xC62F,
            "CameraSerialNumber",
            Ascii(TEXT),
            "CameraSerialNumber",
        ),
        r(0x9C9B, "XPTitle", Bytes(UTF16), "XPTitle"),
        r(0x9C9C, "XPComment", Bytes(UTF16), "XPComment"),
        r(0x9C9E, "XPKeywords", Bytes(UTF16), "XPKeywords"),
        r(0x9C9F, "XPSubject", Bytes(UTF16), "XPSubject"),
        r(0x9C9D, "XPAuthor", Bytes(UTF16), "XPAuthor"),
        r(0xA436, "ImageTitle", Ascii(TEXT), "ImageTitle"),
        r(0xA437, "Photographer", Ascii(TEXT), "Photographer"),
        r(0xA438, "ImageEditor", Ascii(TEXT), "ImageEditor"),
        r(0xA439, "CameraFirmware", Ascii(TEXT), "CameraFirmware"),
        r(
            0xA43A,
            "RAWDevelopingSoftware",
            Ascii(TEXT),
            "RAWDevelopingSoftware",
        ),
        r(
            0xA43B,
            "ImageEditingSoftware",
            Ascii(TEXT),
            "ImageEditingSoftware",
        ),
        r(
            0xA43C,
            "MetadataEditingSoftware",
            Ascii(TEXT),
            "MetadataEditingSoftware",
        ),
        r(
            0x02BC,
            "XMP",
            Bytes(
                b"<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?>\
                  <x:xmpmeta xmlns:x='adobe:ns:meta/'/><?xpacket end='w'?>",
            ),
            "ApplicationNotes",
        ),
        r(
            0x83BB,
            "IPTC-NAA",
            Undef(b"\x1C\x02\x78\x00\x04test"),
            "IPTC-NAA",
        ),
        r(
            0x8649,
            "Photoshop",
            Bytes(b"8BIM\x04\x04\x00\x00\x00\x00\x00\x04test"),
            "PhotoshopSettings",
        ),
        r(
            0x8773,
            "InterColorProfile",
            Undef(b"not-an-icc-profile"),
            "ICC_Profile",
        ),
        r(0x0201, "JPEGInterchangeFormat", Ifd1Long, "ThumbnailOffset"),
    ]
}

// ── a one-entry TIFF builder ────────────────────────────────────────────────

fn le16(v: u16) -> [u8; 2] {
    v.to_le_bytes()
}

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

/// One little-endian IFD of `(tag, type, count, payload)` entries, placed at
/// `at`, whose next-IFD word is `next`. Payloads over 4 bytes go right behind
/// the table. Returns the bytes (table, then payloads).
fn ifd_bytes(at: usize, entries: &[(u16, u16, u32, Vec<u8>)], next: u32) -> Vec<u8> {
    let table = 2 + 12 * entries.len() + 4;
    let mut out = Vec::new();
    let mut ext: Vec<u8> = Vec::new();
    out.extend_from_slice(&le16(entries.len() as u16));
    for (tag, typ, count, payload) in entries {
        out.extend_from_slice(&le16(*tag));
        out.extend_from_slice(&le16(*typ));
        out.extend_from_slice(&le32(*count));
        if payload.len() <= 4 {
            let mut field = payload.clone();
            field.resize(4, 0);
            out.extend_from_slice(&field);
        } else {
            out.extend_from_slice(&le32((at + table + ext.len()) as u32));
            ext.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                ext.push(0);
            }
        }
    }
    out.extend_from_slice(&le32(next));
    out.extend_from_slice(&ext);
    out
}

/// The one-entry TIFF for a row.
pub fn fixture(row: &Row) -> Vec<u8> {
    let mut out = b"II\x2A\x00".to_vec();
    out.extend_from_slice(&le32(8));
    let entry = |typ: u16, bytes: &[u8]| -> Vec<(u16, u16, u32, Vec<u8>)> {
        let count = bytes.len() as u32;
        vec![(row.id, typ, count, bytes.to_vec())]
    };
    match row.shape {
        Shape::Ascii(b) => out.extend(ifd_bytes(8, &entry(2, b), 0)),
        Shape::Undef(b) => out.extend(ifd_bytes(8, &entry(7, b), 0)),
        Shape::Bytes(b) => out.extend(ifd_bytes(8, &entry(1, b), 0)),
        Shape::Ifd1Long => {
            let ifd0: Vec<(u16, u16, u32, Vec<u8>)> = Vec::new();
            let ifd0_len = ifd_bytes(8, &ifd0, 0).len();
            let ifd1_at = 8 + ifd0_len;
            out.extend(ifd_bytes(8, &ifd0, ifd1_at as u32));
            // The thumbnail bytes go after IFD1: 2 + 2 * 12 + 4 = 30 bytes.
            let thumb_at = ifd1_at + 30;
            let ifd1 = vec![
                (row.id, 4u16, 1u32, le32(thumb_at as u32).to_vec()),
                (0x0202u16, 4u16, 1u32, le32(4).to_vec()),
            ];
            out.extend(ifd_bytes(ifd1_at, &ifd1, 0));
            out.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xD9]);
        }
    }
    out
}

// ── ExifTool output and the record ──────────────────────────────────────────

/// `(group, name)` for each tag line of `exiftool -G1 -a -s -u` output, with
/// the file-level groups dropped. A line is `[Group] Name   : value`.
pub fn parse_names(stdout: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((group, after)) = rest.split_once(']') else {
            continue;
        };
        if FILE_LEVEL_GROUPS.contains(&group) {
            continue;
        }
        let name: String = after
            .trim_start()
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        if !name.is_empty() {
            out.push((group.to_string(), name));
        }
    }
    out
}

/// The group recorded for a name read from the verbose listing.
pub const VERBOSE_GROUP: &str = "v3";

/// The top-level entry names of `exiftool -v3` output: lines of the form
/// `  | 0)  Name = value` or `  | 0)  Name (SubDirectory) -->` (one bar only;
/// nested directories have two). This is how ExifTool shows the name it gives
/// a tag that is a directory (XMP, Photoshop, ICC) and so prints no value line
/// of its own.
pub fn parse_verbose_names(stdout: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let Some(rest) = line.trim_start().strip_prefix("| ") else {
            continue;
        };
        let Some((index, after)) = rest.split_once(')') else {
            continue;
        };
        if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let name: String = after
            .trim_start()
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        if !name.is_empty() {
            out.push((VERBOSE_GROUP.to_string(), name));
        }
    }
    out
}

/// Run ExifTool on the row's fixture and return the names it gives the id: the
/// tag lines of `-G1 -a -s -u`, then the entry names of a `-v3` listing.
pub fn measure(row: &Row) -> Result<Vec<(String, String)>, tools::ToolError> {
    let bytes = fixture(row);
    let name = format!("audit-tag-{:04x}", row.id);
    let path = tools::temp_file_for(&name, &bytes).map_err(|e| tools::ToolError::Exec {
        tool: "exiftool",
        message: e.to_string(),
    })?;
    let run = run_for_names(&path);
    let _ = std::fs::remove_file(&path);
    run
}

fn run_for_names(path: &Path) -> Result<Vec<(String, String)>, tools::ToolError> {
    let args: Vec<String> = ARGS.iter().map(|s| s.to_string()).collect();
    let out = tools::run_tool("exiftool", &args, path)?;
    let mut names = parse_names(&String::from_utf8_lossy(&out.stdout));
    // Always also the verbose listing: it is the only place a directory tag
    // (XMP, IPTC, Photoshop, ICC) shows the name ExifTool gives the id itself,
    // and for the rest it names the same entry again.
    let verbose: Vec<String> = ["-v3", "-G1", "-a", "-s", "-u"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let out = tools::run_tool("exiftool", &verbose, path)?;
    names.extend(parse_verbose_names(&String::from_utf8_lossy(&out.stdout)));
    Ok(names)
}

/// The record text: a `#` header, then one line per named tag,
/// `0xID<TAB>group<TAB>name`, or `0xID<TAB>-<TAB>-` for an id ExifTool
/// reported nothing for.
pub fn record_text(version: &str, measured: &[(u16, Vec<(String, String)>)]) -> String {
    let mut s = format!(
        "# command: exiftool {} <file>\n# command: exiftool -v3 {} <file>\n\
         # exiftool-version: {version}\n\
         # source: one-entry TIFF per DEC-008 tag id (tests/support/audit_tags.rs)\n",
        ARGS.join(" "),
        ARGS.join(" ")
    );
    for (id, names) in measured {
        if names.is_empty() {
            s.push_str(&format!("0x{id:04X}\t-\t-\n"));
        }
        for (group, name) in names {
            s.push_str(&format!("0x{id:04X}\t{group}\t{name}\n"));
        }
    }
    s
}

/// The recorded `(id, group, name)` lines of a record text, and its version.
pub fn parse_record(text: &str) -> (String, Vec<(u16, String, String)>) {
    let mut version = String::new();
    let mut lines = Vec::new();
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("# exiftool-version: ") {
            version = v.to_string();
        } else if line.starts_with('#') || line.is_empty() {
            continue;
        } else {
            let mut f = line.split('\t');
            let id = f
                .next()
                .and_then(|h| h.strip_prefix("0x"))
                .and_then(|h| u16::from_str_radix(h, 16).ok())
                .unwrap_or_else(|| panic!("bad record line {line:?}"));
            let group = f.next().unwrap_or("").to_string();
            let name = f.next().unwrap_or("").to_string();
            lines.push((id, group, name));
        }
    }
    (version, lines)
}

/// Compare the table's carried ExifTool names with measured lines: one string
/// per row that disagrees, naming the id. A row agrees when the measured lines
/// for its id contain its `exiftool_name`.
pub fn compare(rows: &[Row], measured: &[(u16, String, String)]) -> Vec<String> {
    let mut bad = Vec::new();
    for row in rows {
        let names: Vec<&str> = measured
            .iter()
            .filter(|m| m.0 == row.id)
            .map(|m| m.2.as_str())
            .collect();
        if !names.contains(&row.exiftool_name) {
            bad.push(format!(
                "0x{:04X} ({}): table says {:?}, measured {:?}",
                row.id, row.dec_name, row.exiftool_name, names
            ));
        }
    }
    bad
}
