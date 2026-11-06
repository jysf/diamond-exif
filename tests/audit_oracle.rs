//! SPEC-012: the live check of the DEC-008 tag table against ExifTool, and the
//! measurement of `Container`'s memory amplification.
//!
//! Names: `tests/support/audit_tags.rs` builds a one-entry TIFF per tag id; the
//! committed record `tests/oracle-fixtures/exiftool/audit-tag-names.txt` stores
//! the names ExifTool 13.55 gave them. The replay test needs no tool; the live
//! test runs ExifTool and skips loudly without it (`DIAMOND_EXIF_REQUIRE_ORACLE=1`
//! makes that a failure). Where ExifTool names an id differently from DEC-008,
//! the row carries a recorded question: DEC-008 and the table are not edited
//! here, and the test requires the list of differing rows to be exactly the
//! recorded one, so it can neither grow nor rot silently.
//!
//! Amplification: a hostile TIFF can name hundreds of overlapping IFDs of
//! 65,535 entries each inside a small file, and `Container` keeps every
//! entry of every IFD. The ignored test prints the numbers; the bounded test
//! checks the same shape at a size that finishes quickly.

#[path = "support/audit_tags.rs"]
mod audit_tags;
#[path = "support/tools.rs"]
mod tools;

use audit_tags::{rows, Row};
use exif_core::audit::{audit, categorize, Category, MAX_INVENTORY_ITEMS};
use exif_core::tiff::{Container, IfdKind};
use std::path::PathBuf;
use std::time::Instant;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn committed_record() -> (String, Vec<(u16, String, String)>) {
    let path = root().join(audit_tags::RECORD);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (run `just oracle-record`)",
            path.display()
        )
    });
    audit_tags::parse_record(&text)
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

/// The rows whose ExifTool name differs from DEC-008's, as measured on
/// ExifTool 13.55. Written out by hand: a new difference or a resolved one
/// fails `recorded_differences_are_exactly_these_rows`.
const DIFFERING: [(u16, &str, &str); 12] = [
    (0x0132, "DateTime", "ModifyDate"),
    (0x9004, "DateTimeDigitized", "CreateDate"),
    (0x927C, "MakerNote", "MakerNoteUnknownText"),
    (0xA430, "CameraOwnerName", "OwnerName"),
    (0xA431, "BodySerialNumber", "SerialNumber"),
    (0xA437, "ImageEditor", "Photographer"),
    (0xA438, "ImageEditingSoftware", "ImageEditor"),
    (0xA439, "MetadataEditingSoftware", "CameraFirmware"),
    (0x02BC, "XMP", "ApplicationNotes"),
    (0x8649, "Photoshop", "PhotoshopSettings"),
    (0x8773, "InterColorProfile", "ICC_Profile"),
    (0x0201, "JPEGInterchangeFormat", "ThumbnailOffset"),
];

#[test]
fn the_table_has_one_row_per_dec_008_id() {
    let rows = rows();
    assert_eq!(rows.len(), 38, "37 ids in the tag table plus 0x0201");
    let mut ids: Vec<u16> = rows.iter().map(|r| r.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), rows.len(), "ids are distinct");
    // Every row is categorised by the library (0x0201 only in an IFD1), so the
    // names checked here are the names of the ids the audit acts on.
    for row in &rows {
        let kind = if row.id == 0x0201 {
            IfdKind::Ifd1
        } else {
            IfdKind::Ifd0
        };
        assert!(
            categorize(kind, row.id).is_some(),
            "0x{:04X} ({}) is in the name table but not categorised",
            row.id,
            row.dec_name
        );
    }
    assert_eq!(categorize(IfdKind::Ifd1, 0x0201), Some(Category::Thumbnail));
}

#[test]
fn recorded_differences_are_exactly_these_rows() {
    let got: Vec<(u16, &str, &str)> = rows()
        .iter()
        .filter(|r| r.dec_name != r.exiftool_name)
        .map(|r| (r.id, r.dec_name, r.exiftool_name))
        .collect();
    assert_eq!(got, DIFFERING.to_vec());
    // And each differing row carries a question, and only those do.
    for row in rows() {
        assert_eq!(
            row.question.is_some(),
            row.dec_name != row.exiftool_name,
            "0x{:04X}",
            row.id
        );
    }
}

#[test]
fn table_names_replay_matches_the_committed_record() {
    let (version, recorded) = committed_record();
    assert!(!version.is_empty(), "record has no exiftool-version line");
    let bad = audit_tags::compare(&rows(), &recorded);
    assert!(
        bad.is_empty(),
        "the name table disagrees with the committed record:\n{}",
        bad.join("\n")
    );
    // The record names exactly the table's ids, nothing else.
    let mut rec_ids: Vec<u16> = recorded.iter().map(|r| r.0).collect();
    rec_ids.sort_unstable();
    rec_ids.dedup();
    let mut table_ids: Vec<u16> = rows().iter().map(|r| r.id).collect();
    table_ids.sort_unstable();
    assert_eq!(rec_ids, table_ids);
}

#[test]
fn live_table_names_match_exiftool() {
    if !live_gate("live_table_names_match_exiftool") {
        return;
    }
    let mut live: Vec<(u16, String, String)> = Vec::new();
    for row in rows() {
        let names = audit_tags::measure(&row).expect("run exiftool");
        for (group, name) in names {
            live.push((row.id, group, name));
        }
    }
    let bad = audit_tags::compare(&rows(), &live);
    assert!(
        bad.is_empty(),
        "live ExifTool disagrees with the name table:\n{}",
        bad.join("\n")
    );
    // And the live output reproduces the committed record line for line.
    let (_, recorded) = committed_record();
    assert_eq!(
        live, recorded,
        "live output differs from the committed record"
    );
}

#[test]
fn table_name_check_is_red_when_a_name_is_perturbed() {
    let (_, recorded) = committed_record();
    let table = rows();
    assert!(audit_tags::compare(&table, &recorded).is_empty(), "control");

    // Perturb the recorded name of one row (every line for that id): exactly
    // that row is named, for several different rows.
    for target in [0x010F_u16, 0x9C9D, 0x0201, 0x02BC] {
        let perturbed: Vec<(u16, String, String)> = recorded
            .iter()
            .map(|(id, g, n)| {
                if *id == target {
                    (*id, g.clone(), format!("{n}X"))
                } else {
                    (*id, g.clone(), n.clone())
                }
            })
            .collect();
        let bad = audit_tags::compare(&table, &perturbed);
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].starts_with(&format!("0x{target:04X} ")), "{bad:?}");
    }
    // A table name perturbed instead of the record: same.
    let mut t2: Vec<Row> = rows();
    t2[1].exiftool_name = "Maker";
    let bad = audit_tags::compare(&t2, &recorded);
    assert_eq!(bad.len(), 1);
    assert!(bad[0].starts_with("0x010F "));
    // An id with no recorded name at all is a mismatch too.
    let missing: Vec<(u16, String, String)> =
        recorded.iter().filter(|r| r.0 != 0x8773).cloned().collect();
    let bad = audit_tags::compare(&table, &missing);
    assert_eq!(bad.len(), 1);
    assert!(bad[0].starts_with("0x8773 "));
}

#[test]
fn name_parsers_read_exiftool_output_shapes() {
    let out = "[System]        FileName                        : x.tif\n\
               [IFD0]          Make                            : test\n\
               [IPTC]          Caption-Abstract                : test\n\
               [ExifTool]      Warning                         : none\n";
    assert_eq!(
        audit_tags::parse_names(out),
        vec![
            ("IFD0".to_string(), "Make".to_string()),
            ("IPTC".to_string(), "Caption-Abstract".to_string())
        ]
    );
    let v = "  + [IFD0 directory with 1 entries, Little-endian]\n\
             | 0)  ApplicationNotes (SubDirectory) -->\n\
             | | 0)  Nested = 1\n\
             | 1)  Make = test\n  | 2)  PhotoshopSettings (SubDirectory) -->\n";
    // Only single-bar lines count; the leading spaces of the first level are
    // part of ExifTool's layout.
    let got = audit_tags::parse_verbose_names(v);
    let names: Vec<&str> = got.iter().map(|g| g.1.as_str()).collect();
    assert_eq!(names, vec!["ApplicationNotes", "Make", "PhotoshopSettings"]);
}

// ── Container amplification ─────────────────────────────────────────────────

/// A TIFF whose IFD0 holds one `SubIFDs` entry naming `n_sub` IFDs at offsets
/// two bytes apart. Each of them is "65,535 entries" because the bytes there
/// are all `0xFF`, so the entry tables overlap and the file is only a little
/// over 786 KB however many IFDs it names.
fn hostile(n_sub: usize) -> Vec<u8> {
    const ENTRIES: usize = 0xFFFF;
    let array_at = 8 + 2 + 12 + 4;
    let base = array_at + 4 * n_sub;
    let base = base + (base % 2);
    let mut out = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&330u16.to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&(n_sub as u32).to_le_bytes());
    out.extend_from_slice(&(array_at as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for k in 0..n_sub {
        out.extend_from_slice(&((base + 2 * k) as u32).to_le_bytes());
    }
    while out.len() < base {
        out.push(0);
    }
    let region = 2 * n_sub.saturating_sub(1) + 2 + 12 * ENTRIES + 4;
    out.resize(base + region, 0xFF);
    // The next-IFD words sit in the last `2 * (n_sub - 1) + 4` bytes of the
    // region; zero them so no IFD chains on (a chained attempt would spend one
    // of the 512 IFD visits on a failure and halve the amplification).
    let next_words = base + 2 + 12 * ENTRIES;
    for b in out.iter_mut().skip(next_words) {
        *b = 0;
    }
    out
}

fn rss_kb() -> Option<u64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn total_entries(c: &Container<'_>) -> usize {
    c.ifds().iter().map(|i| i.entries().len()).sum()
}

#[test]
fn container_amplification_is_bounded_in_test() {
    // The same shape at 8 IFDs: 8 x 65,535 entries from a file of about 790 KB.
    let bytes = hostile(8);
    let start = Instant::now();
    let c = Container::parse(&bytes).expect("parses");
    let parse = start.elapsed();
    assert_eq!(c.ifds().len(), 9, "IFD0 plus 8 overlapping SubIFDs");
    assert_eq!(total_entries(&c), 1 + 8 * 65_535);
    // The audit on top keeps its own inventory bounded.
    let start = Instant::now();
    let a = audit(&bytes);
    let audited = start.elapsed();
    assert_eq!(a.items().len(), MAX_INVENTORY_ITEMS);
    assert!(a.health().inventory_truncated);
    assert!(!a.health().clean);
    assert!(
        parse.as_secs() < 30 && audited.as_secs() < 60,
        "parse {parse:?}, audit {audited:?}"
    );
    println!(
        "bounded amplification: input {} bytes, {} entries kept ({} bytes of entries at 24 each), \
         parse {parse:?}, audit {audited:?}",
        bytes.len(),
        total_entries(&c),
        total_entries(&c) * 24
    );
}

/// SPEC-012: measure the worst case and print the numbers. Not part of the
/// normal run (about 800 MB). Run it with
/// `cargo test --release --test audit_oracle container_amplification_is_measured_and_reported -- --ignored --nocapture`
/// under `/usr/bin/time -l` for the process's peak memory.
#[test]
#[ignore = "measurement: allocates close to a gigabyte; run with --ignored --nocapture"]
fn container_amplification_is_measured_and_reported() {
    let n_sub: usize = 511; // MAX_IFDS (512) minus IFD0
    let bytes = hostile(n_sub);
    println!("input: {} bytes ({n_sub} overlapping SubIFDs)", bytes.len());
    println!("rss before parse: {:?} KB", rss_kb());
    let start = Instant::now();
    let c = Container::parse(&bytes).expect("parses");
    let parse = start.elapsed();
    let entries = total_entries(&c);
    println!(
        "Container::parse: {parse:?}, {} IFDs, {entries} entries, {} issues, rss after: {:?} KB",
        c.ifds().len(),
        c.issues().len(),
        rss_kb()
    );
    println!(
        "entries x size_of::<Entry>() = {} bytes; amplification {:.0}x of the input",
        entries * std::mem::size_of::<exif_core::tiff::Entry>(),
        (entries * std::mem::size_of::<exif_core::tiff::Entry>()) as f64 / bytes.len() as f64
    );
    drop(c);
    let start = Instant::now();
    let a = audit(&bytes);
    let audited = start.elapsed();
    println!(
        "audit: {audited:?} (includes its own Container::parse), items {}, findings {}, truncated {}, \
         rss after: {:?} KB",
        a.items().len(),
        a.findings().len(),
        a.health().inventory_truncated,
        rss_kb()
    );
}
