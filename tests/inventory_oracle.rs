//! SPEC-013: inventory parity against ExifTool.
//!
//! The audit model lists every IFD entry it found and every located region. This
//! file checks that listing, as a multiset of `(group, tag id)` plus the set of
//! region kinds, against `exiftool -a -u -G1 -H -s` over five corpora: the
//! hand-built fixtures, the DEC-008 one-entry TIFFs and the three fuzz-seed
//! directories (247 inputs, 178 distinct by content). The model and the pinned
//! divergences are in `tests/support/inventory.rs`; the committed ExifTool
//! listings are in `tests/oracle-fixtures/exiftool-inventory/`.
//!
//! Live tests run ExifTool (they skip loudly without it;
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1` turns a missing tool into a failure); replay
//! tests run the same comparator against the committed records with no tool.
//! Both go through `inventory::{ours, theirs, diff}` and `KNOWN_DIVERGENCES`:
//! an input either agrees exactly or has a pinned divergence that names the
//! exact difference. ExifTool is run, never read, never linked.

#[path = "support/audit_tags.rs"]
mod audit_tags;
#[path = "support/inventory.rs"]
mod inventory;
#[path = "support/inventory_corpus.rs"]
mod inventory_corpus;
#[path = "support/mutate.rs"]
mod mutate;
#[path = "support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/tools.rs"]
mod tools;

use exif_core::audit::audit;
use exif_core::tiff::{entry_budget, Container};
use exif_core::Error;
use inventory::{
    check_against, diff, inventory_of, ours, parse_stdout, theirs, CheckError, Class, Inventory,
    Line, ParseError, RegionKind, Side, KNOWN_DIVERGENCES,
};
use inventory_corpus::{Distinct, Input};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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

// ── helpers ─────────────────────────────────────────────────────────────────

fn all_inputs() -> Vec<Input> {
    inventory_corpus::all_inputs(&root())
}

fn distinct_inputs() -> Vec<Distinct> {
    inventory_corpus::distinct(&all_inputs())
}

fn records() -> BTreeMap<&'static str, inventory::RecordFile> {
    inventory_corpus::CORPORA
        .iter()
        .map(|&corpus| {
            let path = root()
                .join(inventory_corpus::RECORD_DIR)
                .join(inventory_corpus::record_file(corpus));
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!(
                    "cannot read {}: {e} (run `just oracle-record`)",
                    path.display()
                )
            });
            let rec = inventory::parse_record(&text)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            (corpus, rec)
        })
        .collect()
}

/// The committed section of an input key (`<corpus>/<name>`).
fn section_of(
    recs: &BTreeMap<&'static str, inventory::RecordFile>,
    key: &str,
) -> inventory::Section {
    let (corpus, name) = key.split_once('/').expect("a corpus/name key");
    recs.get(corpus)
        .unwrap_or_else(|| panic!("no record for corpus {corpus}"))
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section {name} in the {corpus} record"))
        .clone()
}

fn bytes_of(key: &str) -> Vec<u8> {
    all_inputs()
        .into_iter()
        .find(|i| i.key() == key)
        .unwrap_or_else(|| panic!("no input {key}"))
        .bytes
}

/// Their inventory of a committed section, for an input of the given container.
/// For a bare TIFF the region-group lines are the decoded contents of a
/// directory-valued tag (Class H) and are dropped.
fn their_inventory(lines: &[Line], is_jpeg: bool) -> Inventory {
    let mut inv = inventory_of(lines).expect("classifiable groups");
    if !is_jpeg {
        inv.regions.clear();
    }
    inv
}

/// Compare one distinct input against `their` inventory and the table.
fn check_input(d: &Distinct, their: &Inventory) -> Result<(), CheckError> {
    let a = audit(&d.canonical.bytes);
    let mine = ours(&a);
    let d_ = diff(&mine, their);
    check_against(KNOWN_DIVERGENCES, &d.canonical.key(), &d_, a.health().clean)
}

fn replay_all() -> Vec<CheckError> {
    let recs = records();
    let mut errors = Vec::new();
    for d in distinct_inputs() {
        if inventory_corpus::is_not_run(&d) {
            continue;
        }
        let sec = section_of(&recs, &d.canonical.key());
        let their = their_inventory(&sec.lines, d.canonical.is_jpeg());
        if let Err(e) = check_input(&d, &their) {
            errors.push(e);
        }
    }
    errors
}

/// Run ExifTool alone on one input, under the tool time limit.
fn live_lines(input: &Input) -> (Vec<String>, Vec<Line>) {
    let path = tools::temp_file_with_ext("inventory-live", input.ext(), &input.bytes)
        .expect("write temp file");
    let args: Vec<String> = inventory::COMMAND_ARGS
        .iter()
        .map(|s| s.to_string())
        .collect();
    let run = tools::run_tool("exiftool", &args, &path);
    let _ = std::fs::remove_file(&path);
    let out = run.unwrap_or_else(|e| panic!("exiftool on {}: {e}", input.key()));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let lines = parse_stdout(&stdout).unwrap_or_else(|e| panic!("{}: {e}", input.key()));
    (tools::warning_lines(&stderr, &path), lines)
}

/// Live parity over the distinct inputs selected by `pick`: ExifTool's answer
/// is the committed one and the comparison against the table is clean.
fn live_parity(pick: impl Fn(&Distinct) -> bool) -> usize {
    let recs = records();
    let mut checked = 0usize;
    let mut errors: Vec<String> = Vec::new();
    for d in distinct_inputs().into_iter().filter(|d| pick(d)) {
        if inventory_corpus::is_not_run(&d) {
            // Never run: ExifTool does not return on it.
            continue;
        }
        let (stderr, lines) = live_lines(&d.canonical);
        let sec = section_of(&recs, &d.canonical.key());
        if lines != sec.lines || stderr != sec.stderr {
            errors.push(format!(
                "{}: the live listing differs from the committed record (re-record and review)",
                d.canonical.key()
            ));
        }
        match inventory_of(&lines) {
            Ok(mut their) => {
                if !d.canonical.is_jpeg() {
                    their.regions.clear();
                }
                if let Err(e) = check_input(&d, &their) {
                    errors.push(e.to_string());
                }
            }
            Err(e) => errors.push(format!("{}: {e}", d.canonical.key())),
        }
        checked += 1;
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    checked
}

fn divergences_of(class: Class) -> Vec<&'static inventory::Divergence> {
    KNOWN_DIVERGENCES
        .iter()
        .filter(|k| k.class == class)
        .collect()
}

fn keys_of(class: Class) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = divergences_of(class).iter().map(|k| k.fixture).collect();
    v.sort_unstable();
    v
}

fn count_of(inv: &Inventory, group: &str, id: u16) -> usize {
    inv.tags.get(&(group.to_string(), id)).copied().unwrap_or(0)
}

fn ours_of_key(key: &str) -> Inventory {
    ours(&audit(&bytes_of(key)))
}

fn theirs_of_key(key: &str) -> Inventory {
    let recs = records();
    let is_jpeg = bytes_of(key).starts_with(&[0xFF, 0xD8]);
    their_inventory(&section_of(&recs, key).lines, is_jpeg)
}

// ── the parser and the model ────────────────────────────────────────────────

#[test]
fn directory_valued_ids_are_exactly_the_eight_and_each_has_a_fixture() {
    let mut ids = inventory::DIRECTORY_VALUED.to_vec();
    ids.sort_unstable();
    assert_eq!(
        ids,
        vec![0x014A, 0x02BC, 0x83BB, 0x8649, 0x8769, 0x8773, 0x8825, 0xA005]
    );
    // Each id is an entry of at least one input, read by the audit itself.
    let mut seen: BTreeSet<u16> = BTreeSet::new();
    for input in all_inputs() {
        for item in audit(&input.bytes).items() {
            if let exif_core::audit::ItemKind::Tag { tag, .. } = item.kind() {
                if inventory::DIRECTORY_VALUED.contains(&tag) {
                    seen.insert(tag);
                }
            }
        }
    }
    let want: BTreeSet<u16> = inventory::DIRECTORY_VALUED.iter().copied().collect();
    assert_eq!(seen, want, "every directory-valued id has an input");
    // And the one-entry fixture of each of the four decoded ids is in the
    // audit-tags corpus, which is where ExifTool's non-listing of them is
    // measured (see the next test).
    for id in [0x02BC_u16, 0x83BB, 0x8649, 0x8773] {
        let key = format!("audit-tags/tag-{id:04x}");
        assert!(
            all_inputs().iter().any(|i| i.key() == key),
            "no fixture {key}"
        );
    }
}

/// Class H, the other half: a bare TIFF's region-group lines come only from the
/// four ids whose content ExifTool decodes elsewhere.
#[test]
fn tiff_region_lines_come_only_from_directory_valued_ids() {
    let recs = records();
    let mut with_regions = 0usize;
    for d in distinct_inputs() {
        if d.canonical.is_jpeg() || inventory_corpus::is_not_run(&d) {
            continue;
        }
        let sec = section_of(&recs, &d.canonical.key());
        let raw = inventory_of(&sec.lines).expect("classifiable");
        if raw.regions.is_empty() {
            continue;
        }
        with_regions += 1;
        let a = audit(&d.canonical.bytes);
        let carries = a.items().iter().any(|i| {
            matches!(i.kind(), exif_core::audit::ItemKind::Tag { tag, .. }
                if [0x02BC_u16, 0x83BB, 0x8649, 0x8773].contains(&tag))
        });
        assert!(
            carries,
            "{}: ExifTool lists region groups {:?} but the TIFF has none of the four decoded ids",
            d.canonical.key(),
            raw.regions
        );
    }
    println!("TIFF inputs with region-group lines: {with_regions}");
}

#[test]
fn inventory_groups_map_ifd_kinds_and_number_the_chain() {
    use exif_core::tiff::IfdKind;
    assert_eq!(inventory::group_of(IfdKind::Ifd0, 0), "IFD0");
    assert_eq!(inventory::group_of(IfdKind::Ifd1, 0), "IFD1");
    assert_eq!(inventory::group_of(IfdKind::Exif, 0), "ExifIFD");
    assert_eq!(inventory::group_of(IfdKind::Gps, 0), "GPS");
    assert_eq!(inventory::group_of(IfdKind::Interop, 0), "InteropIFD");
    assert_eq!(inventory::group_of(IfdKind::Sub, 7), "SubIFD*");
    assert_eq!(inventory::group_of(IfdKind::Chain, 0), "IFD2");
    assert_eq!(inventory::group_of(IfdKind::Chain, 1), "IFD3");

    // The chain fixture, read by the audit: IFD0, IFD1, IFD2, IFD3.
    let chain = ours_of_key("fixtures/tiff-chain-four");
    let want: BTreeMap<(String, u16), usize> = [
        (("IFD0".to_string(), 0x010f), 1),
        (("IFD1".to_string(), 0x0110), 1),
        (("IFD2".to_string(), 0x0131), 1),
        (("IFD3".to_string(), 0x013b), 1),
    ]
    .into_iter()
    .collect();
    assert_eq!(chain.tags, want);

    // Numbered per Exif block: two blocks that each carry a four-IFD chain give
    // IFD2 and IFD3 twice, never IFD4 and IFD5.
    let tiff = oracle_tiff::inventory_fixtures()
        .into_iter()
        .find(|(n, _)| *n == "tiff-chain-four")
        .expect("fixture")
        .1;
    let mut jpeg = oracle_jpeg::SOI.to_vec();
    jpeg.extend(oracle_jpeg::exif_app1(&tiff));
    jpeg.extend(oracle_jpeg::exif_app1(&tiff));
    jpeg.extend(oracle_jpeg::sos_header());
    jpeg.extend(oracle_jpeg::SCAN);
    jpeg.extend(oracle_jpeg::EOI);
    let two = ours(&audit(&jpeg));
    assert_eq!(count_of(&two, "IFD2", 0x0131), 2);
    assert_eq!(count_of(&two, "IFD3", 0x013b), 2);
    assert_eq!(count_of(&two, "IFD0", 0x010f), 2);
    assert!(two.tags.keys().all(|(g, _)| g != "IFD4" && g != "IFD5"));

    // The Exif, GPS and Interop groups, and the pointer tags left out.
    let gi = ours_of_key("seed-tiff_ifd/exif-gps-interop");
    for g in ["ExifIFD", "GPS", "InteropIFD"] {
        assert!(gi.tags.keys().any(|(grp, _)| grp == g), "{g}");
    }
    assert!(gi
        .tags
        .keys()
        .all(|(_, id)| !inventory::DIRECTORY_VALUED.contains(id)));
}

#[test]
fn sub_ifd_names_are_pinned_on_the_three_subifd_fixture() {
    // ExifTool names the three leaf IFDs SubIFD, SubIFD1 and SubIFD2; the
    // inventory normalises the numbering to SubIFD*.
    let recs = records();
    let sec = section_of(&recs, "fixtures/tiff-subifds-three");
    let groups: Vec<&str> = sec.lines.iter().map(|l| l.group.as_str()).collect();
    assert_eq!(groups, vec!["IFD0", "SubIFD", "SubIFD1", "SubIFD2"]);
    let ids: Vec<Option<u32>> = sec.lines.iter().map(|l| l.id).collect();
    assert_eq!(
        ids,
        vec![Some(0x010f), Some(0x0100), Some(0x0101), Some(0x0112)]
    );
    let their = inventory_of(&sec.lines).expect("classifiable");
    assert_eq!(count_of(&their, "SubIFD*", 0x0100), 1);
    assert_eq!(count_of(&their, "SubIFD*", 0x0101), 1);
    assert_eq!(count_of(&their, "SubIFD*", 0x0112), 1);
    // Ours lists the same three, in the same normalised group.
    let mine = ours_of_key("fixtures/tiff-subifds-three");
    assert_eq!(mine.tags, their.tags);
}

#[test]
fn theirs_parser_drops_file_level_groups_and_synthetic_ids() {
    let stdout = "\
[ExifTool]           - ExifToolVersion                 : 13.55
[System]             - FileName                        : x
[File]               - FileType                        : TIFF
[IFD0]          0x010f Make                            : Canon
[IFD0]               - ThumbnailImage                  : (Binary data 3 bytes)
[ExifIFD]       0x9003 DateTimeOriginal                : 2024:01:01 00:00:00
a continuation line of a multi-line value
[GPS]           0x0001 GPSLatitudeRef                  : North
[Composite]          - ImageSize                       : 4x2
";
    let lines = parse_stdout(stdout).expect("parses");
    // File-level groups are gone; the synthetic IFD0 line is still a line (it
    // is kept in the record) but is not an inventory entry.
    let kept: Vec<(&str, Option<u32>)> = lines.iter().map(|l| (l.group.as_str(), l.id)).collect();
    assert_eq!(
        kept,
        vec![
            ("IFD0", Some(0x010f)),
            ("IFD0", None),
            ("ExifIFD", Some(0x9003)),
            ("GPS", Some(0x0001)),
        ]
    );
    let inv = theirs(stdout).expect("classifies");
    assert_eq!(inv.tags.len(), 3, "{:?}", inv.tags);
    assert_eq!(count_of(&inv, "IFD0", 0x010f), 1);
    assert!(inv.regions.is_empty());
}

#[test]
fn theirs_parser_rejects_an_unknown_group() {
    let err = theirs("[MakerNotes]    0x0001 Quality : 1\n").expect_err("unknown group");
    assert!(
        matches!(&err, ParseError::UnknownGroup { group, .. } if group == "MakerNotes"),
        "{err:?}"
    );
    assert!(err.to_string().contains("UNMODELLED_GROUPS"), "{err}");
    // Not a silent drop even when it is the only line of a file.
    assert!(theirs("[Canon] 0x0001 Foo : 1").is_err());
    // A bad id is its own error.
    assert!(matches!(
        theirs("[IFD0] zzz Make : x"),
        Err(ParseError::BadId(_))
    ));
    // An EXIF-family id wider than 16 bits cannot be a tag number.
    assert!(matches!(
        theirs("[IFD0] 0x10000 Make : x"),
        Err(ParseError::BadId(_))
    ));
    // The two named groups are accepted, and nothing else is.
    assert!(theirs("[JFIF] 0x0000 JFIFVersion : 1.01").is_ok());
    assert!(theirs("[Photoshop] 0x0408 GridGuidesInfo : x").is_ok());
}

#[test]
fn theirs_parser_maps_region_groups_by_prefix() {
    let cases = [
        ("[XMP-xmp]            - CreatorTool : T", RegionKind::Xmp),
        ("[XMP-dc]             - Title : T", RegionKind::Xmp),
        ("[IPTC]          0x0005 ObjectName : T", RegionKind::Iptc),
        ("[ICC-header]    0x0004 ProfileCMMType : ", RegionKind::Icc),
        (
            "[ICC_Profile]        - ProfileDescription : x",
            RegionKind::Icc,
        ),
        ("[JUMBF]              - JUMDType : 0", RegionKind::Jumbf),
    ];
    for (line, kind) in cases {
        let inv = theirs(line).unwrap_or_else(|e| panic!("{line}: {e}"));
        assert_eq!(
            inv.regions.iter().copied().collect::<Vec<_>>(),
            vec![kind],
            "{line}"
        );
        assert!(inv.tags.is_empty(), "{line}: a region line is not a tag");
    }
    // Unmodelled groups are dropped without a region.
    let inv = theirs("[JFIF] 0x0000 JFIFVersion : 1.01\n[Photoshop] 0x0408 X : y").expect("ok");
    assert!(inv.regions.is_empty() && inv.tags.is_empty());
    // Several region lines give one region.
    let inv = theirs("[ICC-header] 0x0004 A : \n[ICC-header] 0x0008 B : \n[ICC_Profile] - C : x")
        .expect("ok");
    assert_eq!(inv.regions.len(), 1);
}

#[test]
fn distinct_inputs_are_counted_once_across_seed_directories() {
    let all = all_inputs();
    assert_eq!(all.len(), 247, "inputs over the five corpora");
    let per_corpus: Vec<usize> = inventory_corpus::CORPORA
        .iter()
        .map(|c| all.iter().filter(|i| i.corpus == *c).count())
        .collect();
    // 61 hand-built (12 + 2 TIFF, 42 + 5 JPEG), 41 DEC-008 ids, and the seeds
    // (32 TIFF, 37 JPEG, 76 audit).
    assert_eq!(per_corpus, vec![61, 41, 32, 37, 76]);
    let keys: BTreeSet<String> = all.iter().map(Input::key).collect();
    assert_eq!(keys.len(), all.len(), "keys are unique");

    let distinct = inventory_corpus::distinct(&all);
    assert_eq!(distinct.len(), 178, "distinct inputs by content");
    let members: usize = distinct.iter().map(|d| d.keys.len()).sum();
    assert_eq!(members, all.len(), "every input belongs to one group");

    // The audit directory copies the other two seed directories under a prefix:
    // 32 + 37 of its 76 seeds are aliases of an earlier seed, none is a
    // canonical name.
    let aliases = |prefix: &str, target: &str| -> usize {
        distinct
            .iter()
            .filter(|d| {
                d.canonical.corpus == target
                    && d.keys
                        .iter()
                        .any(|k| k.starts_with(&format!("seed-audit/{prefix}")))
            })
            .count()
    };
    assert_eq!(aliases("tiff-", "seed-tiff_ifd"), 32);
    assert_eq!(aliases("jpeg-", "seed-jpeg_segments"), 37);
    let audit_canonical = distinct
        .iter()
        .filter(|d| d.canonical.corpus == "seed-audit")
        .count();
    assert_eq!(audit_canonical, 7, "the seven audit-only shapes");
    // A canonical key is the first corpus's: fixtures before audit-tags before
    // the seed directories.
    for d in &distinct {
        assert_eq!(d.keys[0], d.canonical.key());
    }
}

// ── replay ──────────────────────────────────────────────────────────────────

#[test]
fn replay_every_corpus_is_clean_except_pinned_divergences() {
    let recs = records();
    // The record of each corpus has exactly the corpus's inputs, in order, and
    // byte-identical inputs have identical listings.
    let all = all_inputs();
    let mut by_bytes: BTreeMap<&[u8], Vec<Line>> = BTreeMap::new();
    for corpus in inventory_corpus::CORPORA {
        let rec = &recs[corpus];
        let names: Vec<&str> = rec.sections.iter().map(|s| s.name.as_str()).collect();
        let want: Vec<String> = all
            .iter()
            .filter(|i| i.corpus == corpus)
            .map(|i| i.name.clone())
            .collect();
        assert_eq!(names, want, "{corpus}: sections are the corpus's inputs");
    }
    for input in &all {
        let sec = section_of(&recs, &input.key());
        match by_bytes.get(input.bytes.as_slice()) {
            Some(lines) => assert_eq!(lines, &sec.lines, "{}: twin listings differ", input.key()),
            None => {
                by_bytes.insert(input.bytes.as_slice(), sec.lines.clone());
            }
        }
    }

    let errors = replay_all();
    assert!(
        errors.is_empty(),
        "{} unexplained or changed differences:\n{}",
        errors.len(),
        errors
            .iter()
            .map(CheckError::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );

    // The table is exactly what was compared: every row names a distinct input,
    // no row twice, and the count of rows is the measured set.
    let keys: BTreeSet<&str> = KNOWN_DIVERGENCES.iter().map(|k| k.fixture).collect();
    assert_eq!(keys.len(), KNOWN_DIVERGENCES.len(), "a row twice");
    let canonical: BTreeSet<String> = distinct_inputs()
        .iter()
        .map(|d| d.canonical.key())
        .collect();
    for k in &keys {
        assert!(canonical.contains(*k), "{k} is not a canonical input key");
    }
    assert_eq!(
        KNOWN_DIVERGENCES.len(),
        32,
        "5 A, 2 B, 1 C, 4 D, 1 E, 6 F, 13 R"
    );
}

#[test]
fn replay_inventory_headers_record_command_version_and_stderr() {
    let recs = records();
    for corpus in inventory_corpus::CORPORA {
        let rec = &recs[corpus];
        assert_eq!(rec.command, "exiftool -a -u -G1 -H -s <file>", "{corpus}");
        assert_eq!(rec.version, "13.55", "{corpus}");
        assert_eq!(rec.corpus, corpus);
        // No file-level group ever reaches a record, and every group is one
        // the comparator classifies.
        for s in &rec.sections {
            for l in &s.lines {
                assert!(
                    !inventory::FILE_LEVEL_GROUPS.contains(&l.group.as_str()),
                    "{corpus}/{}: file-level group {}",
                    s.name,
                    l.group
                );
                assert!(
                    inventory::classify_group(&l.group).is_some(),
                    "{corpus}/{}: unclassified group {}",
                    s.name,
                    l.group
                );
            }
            // Measured: ExifTool wrote nothing to stderr for any input.
            assert!(s.stderr.is_empty(), "{corpus}/{}: {:?}", s.name, s.stderr);
        }
    }
    // The record format carries stderr lines and outcomes (round trip).
    let sec = inventory::section_text(
        "x",
        Some("not run: because"),
        &["Warning: a - <file>".to_string(), "Warning: b".to_string()],
        &[Line {
            group: "IFD0".into(),
            id: Some(0x010f),
            name: "Make".into(),
        }],
    );
    let text = format!("{}{sec}", inventory::record_header("13.55", "c"));
    let parsed = inventory::parse_record(&text).expect("round trip");
    assert_eq!(parsed.sections.len(), 1);
    let s = &parsed.sections[0];
    assert_eq!(s.outcome.as_deref(), Some("not run: because"));
    assert_eq!(s.stderr, vec!["Warning: a - <file>", "Warning: b"]);
    assert_eq!(s.lines.len(), 1);
}

#[test]
fn replay_two_hop_cycle_is_recorded_not_run() {
    let recs = records();
    for key in [
        "seed-tiff_ifd/two-hop-cycle",
        "seed-audit/tiff-two-hop-cycle",
    ] {
        let sec = section_of(&recs, key);
        let outcome = sec.outcome.as_deref().unwrap_or("");
        assert!(outcome.starts_with("not run:"), "{key}: {outcome:?}");
        assert!(outcome.contains("exiftool-two-hop-cycle-does-not-return"));
        assert!(sec.lines.is_empty() && sec.stderr.is_empty(), "{key}");
    }
    // It is the only one.
    let not_run = recs
        .values()
        .flat_map(|r| r.sections.iter())
        .filter(|s| s.outcome.is_some())
        .count();
    assert_eq!(not_run, 2, "the seed and its audit-directory copy");
    // And the table says so: a Class E row, never compared.
    let e = divergences_of(Class::E);
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].fixture, "seed-tiff_ifd/two-hop-cycle");
    assert_eq!(e[0].right_side, Side::Neither);
    let d = inventory_corpus::distinct(&all_inputs());
    let hop = d
        .iter()
        .find(|d| d.canonical.key() == "seed-tiff_ifd/two-hop-cycle")
        .expect("input");
    assert!(inventory_corpus::is_not_run(hop));
    assert_eq!(hop.keys.len(), 2);
}

// ── live ────────────────────────────────────────────────────────────────────

#[test]
fn live_inventory_parity_on_hand_built_fixtures() {
    if !live_gate("live_inventory_parity_on_hand_built_fixtures") {
        return;
    }
    let n = live_parity(|d| d.canonical.corpus == "fixtures");
    assert_eq!(n, 61, "all 61 hand-built fixtures are distinct and run");
}

#[test]
fn live_inventory_parity_on_the_audit_tag_fixtures() {
    if !live_gate("live_inventory_parity_on_the_audit_tag_fixtures") {
        return;
    }
    let n = live_parity(|d| d.canonical.corpus == "audit-tags");
    assert_eq!(n, 41);
}

#[test]
fn live_inventory_parity_on_the_seed_corpora() {
    if !live_gate("live_inventory_parity_on_the_seed_corpora") {
        return;
    }
    let n = live_parity(|d| d.canonical.corpus.starts_with("seed-"));
    // 76 distinct seeds, minus the one ExifTool never returns on.
    assert_eq!(n, 75);
}

// ── the classes ─────────────────────────────────────────────────────────────

#[test]
fn class_a_unreadable_values_are_ours_only_and_still_present() {
    assert_eq!(
        keys_of(Class::A),
        vec![
            "seed-tiff_ifd/ascii-count-past-eof",
            "seed-tiff_ifd/count-overflow",
            "seed-tiff_ifd/count-overflow-be",
            "seed-tiff_ifd/payload-past-eof",
            "seed-tiff_ifd/payload-past-eof-be",
        ]
    );
    for k in divergences_of(Class::A) {
        assert!(k.theirs_only.is_empty() && !k.ours_hides && k.right_side == Side::Ours);
        let mine = ours_of_key(k.fixture);
        let their = theirs_of_key(k.fixture);
        for &(g, id, n) in k.ours_only {
            // The entry is in the audit, and ExifTool lists none of it.
            assert!(count_of(&mine, g, id) >= n, "{}: {g} {id:#06x}", k.fixture);
            assert_eq!(count_of(&their, g, id), 0, "{}: {g} {id:#06x}", k.fixture);
        }
    }
}

#[test]
fn class_b_revisited_ifd_is_theirs_only_and_still_present() {
    assert_eq!(
        keys_of(Class::B),
        vec![
            "seed-tiff_ifd/exif-points-at-ifd0",
            "seed-tiff_ifd/self-cycle"
        ]
    );
    for k in divergences_of(Class::B) {
        assert!(k.ours_only.is_empty() && k.ours_hides && k.right_side == Side::Ours);
        let bytes = bytes_of(k.fixture);
        let a = audit(&bytes);
        // The same bytes: the entry ExifTool lists a second time is already in
        // our IFD0 listing, once; we visit an offset once (DEC-005) and say so.
        let mine = ours(&a);
        let their = theirs_of_key(k.fixture);
        assert_eq!(count_of(&mine, "IFD0", 0x0100), 1, "{}", k.fixture);
        assert_eq!(count_of(&their, "IFD0", 0x0100), 1, "{}", k.fixture);
        assert!(
            !a.health().clean && a.health().exif_issues > 0,
            "{}",
            k.fixture
        );
        let c = Container::parse(&bytes).expect("parses");
        assert!(
            c.issues()
                .iter()
                .any(|i| matches!(i.error(), Error::CyclicIfd { .. })),
            "{}: {:?}",
            k.fixture,
            c.issues()
        );
    }
}

#[test]
fn class_c_depth_limit_is_theirs_only_and_the_file_is_not_clean() {
    assert_eq!(keys_of(Class::C), vec!["seed-tiff_ifd/deep-nesting"]);
    let key = "seed-tiff_ifd/deep-nesting";
    let bytes = bytes_of(key);
    let a = audit(&bytes);
    // 8 SubIFD entries here, 11 there: the three extra levels.
    assert_eq!(count_of(&ours(&a), "SubIFD*", 0x0100), 8);
    assert_eq!(count_of(&theirs_of_key(key), "SubIFD*", 0x0100), 11);
    // Not silent: the depth limit is an issue and the file is not clean.
    assert!(!a.health().clean && a.health().exif_issues > 0);
    let c = Container::parse(&bytes).expect("parses");
    assert!(c
        .issues()
        .iter()
        .any(|i| matches!(i.error(), Error::IfdDepthExceeded { .. })));
}

#[test]
fn class_d_caps_and_budget_are_ours_only_and_still_present() {
    assert_eq!(
        keys_of(Class::D),
        vec![
            "seed-audit/audit-many-overlapping-ifds",
            "seed-tiff_ifd/entry-budget-exact",
            "seed-tiff_ifd/entry-budget-over",
            "seed-tiff_ifd/overlapping-subifds",
        ]
    );
    for k in divergences_of(Class::D) {
        assert!(k.theirs_only.is_empty() && !k.ours_hides && k.right_side == Side::Neither);
        let bytes = bytes_of(k.fixture);
        let mine = ours(&audit(&bytes));
        for &(g, id, n) in k.ours_only {
            assert!(count_of(&mine, g, id) >= n, "{}: {g} {id:#06x}", k.fixture);
        }
        // Ours is within the documented len/12 entry budget (DEC-001).
        let c = Container::parse(&bytes).expect("parses");
        assert!(
            c.entry_count() <= entry_budget(bytes.len()),
            "{}: {} entries, budget {}",
            k.fixture,
            c.entry_count(),
            entry_budget(bytes.len())
        );
    }
    // ExifTool keeps the first 10 SubIFDs of an array: it lists no SubIFD entry
    // at all for the 255 and 767 overlapping ones.
    for key in [
        "seed-tiff_ifd/overlapping-subifds",
        "seed-audit/audit-many-overlapping-ifds",
    ] {
        assert!(
            theirs_of_key(key).tags.keys().all(|(g, _)| g != "SubIFD*"),
            "{key}"
        );
    }
}

#[test]
fn class_f_jpeg_shapes_match_the_recorded_divergences() {
    assert_eq!(
        keys_of(Class::F),
        vec![
            "fixtures/jpeg-exif-after-scan",
            "fixtures/jpeg-exif-before-and-after-scan",
            "fixtures/jpeg-exif-last-segment-at-eof",
            "fixtures/jpeg-seventeen-exif-last-differs",
            "seed-jpeg_segments/exif-after-scan",
            "seed-jpeg_segments/seventeen-exif",
        ]
    );
    // After the scan: ExifTool lists nothing; every entry we list is flagged as
    // lying after the scan (the audit says where it came from).
    for key in [
        "fixtures/jpeg-exif-after-scan",
        "seed-jpeg_segments/exif-after-scan",
    ] {
        let bytes_tmp = bytes_of(key);
        let a = audit(&bytes_tmp);
        assert!(!a.items().is_empty(), "{key}");
        assert!(a.items().iter().all(|i| i.origin().after_scan), "{key}");
        assert!(theirs_of_key(key).tags.is_empty(), "{key}");
    }
    // Before and after: the first block is shared, the second adds one copy of
    // the tag the block carries.
    let key = "fixtures/jpeg-exif-before-and-after-scan";
    let bytes_tmp = bytes_of(key);
    let a = audit(&bytes_tmp);
    assert!(a.items().iter().any(|i| i.origin().after_scan));
    assert_eq!(count_of(&ours(&a), "IFD0", 0x010f), 2);
    assert_eq!(count_of(&theirs_of_key(key), "IFD0", 0x010f), 1);
    // The last segment ends the file: ExifTool lists nothing, and the audit
    // reports the JPEG-layer damage (no scan, no EOI).
    let key = "fixtures/jpeg-exif-last-segment-at-eof";
    let bytes_tmp = bytes_of(key);
    let a = audit(&bytes_tmp);
    assert!(a.health().jpeg_issues > 0 && !a.health().clean);
    assert!(theirs_of_key(key).tags.is_empty());
    // And the divergence rows say which side is right.
    for k in divergences_of(Class::F) {
        let hides = k.fixture.contains("seventeen");
        assert_eq!(k.ours_hides, hides, "{}", k.fixture);
        assert_eq!(
            k.right_side,
            if hides { Side::Neither } else { Side::Ours },
            "{}",
            k.fixture
        );
    }
}

#[test]
fn seventeenth_block_is_theirs_only_and_the_file_is_not_clean() {
    for (key, tag_group, tag) in [
        (
            "fixtures/jpeg-seventeen-exif-last-differs",
            "IFD0",
            0x010f_u16,
        ),
        ("seed-jpeg_segments/seventeen-exif", "IFD0", 0x010f),
    ] {
        let bytes = bytes_of(key);
        let a = audit(&bytes);
        // Sixteen blocks parsed (the cap), seventeen read by ExifTool.
        assert_eq!(count_of(&ours(&a), tag_group, tag), 16, "{key}");
        assert_eq!(count_of(&theirs_of_key(key), tag_group, tag), 17, "{key}");
        // Not silent: the extra block is an ExtraExif issue and the file is not
        // clean.
        assert!(!a.health().clean && a.health().jpeg_issues > 0, "{key}");
        let meta = exif_core::jpeg::read(&bytes).expect("a JPEG");
        let extra = meta
            .jpeg_issues()
            .iter()
            .filter(|i| matches!(i.error(), Error::ExtraExif { .. }))
            .count();
        assert!(extra >= 1, "{key}: no ExtraExif issue");
        let k = KNOWN_DIVERGENCES
            .iter()
            .find(|k| k.fixture == key)
            .expect("a row");
        assert!(k.ours_hides && k.ours_only.is_empty() && !k.theirs_only.is_empty());
    }
}

/// Process rule 2: every divergence has a question in `guidance/questions.yaml`.
#[test]
fn every_divergence_names_an_existing_question() {
    let text = std::fs::read_to_string(root().join("guidance/questions.yaml"))
        .expect("read guidance/questions.yaml");
    for k in KNOWN_DIVERGENCES {
        assert!(
            text.contains(&format!("  - id: {}\n", k.question)),
            "{}: no question {} in guidance/questions.yaml",
            k.fixture,
            k.question
        );
        assert!(!k.measured.is_empty(), "{}: no measured output", k.fixture);
    }
}

#[test]
fn ours_hides_implies_not_clean_for_every_divergence() {
    let mut hiding = 0usize;
    for k in KNOWN_DIVERGENCES {
        assert!(
            k.hides_is_consistent(),
            "{}: ours_hides disagrees with its sets",
            k.fixture
        );
        if k.class == Class::E {
            continue;
        }
        if k.ours_hides {
            hiding += 1;
            let bytes_tmp = bytes_of(k.fixture);
            let a = audit(&bytes_tmp);
            assert!(
                !a.health().clean,
                "{}: ExifTool lists entries we do not, yet the audit says clean",
                k.fixture
            );
        }
    }
    // B (2), C (1) and F (2: the 17th block).
    assert_eq!(hiding, 5);
}

#[test]
fn the_hides_invariant_goes_red_on_a_clean_file() {
    // Control: the deep-nesting divergence with its real health (not clean)
    // passes the comparison.
    let key = "seed-tiff_ifd/deep-nesting";
    let bytes = bytes_of(key);
    let a = audit(&bytes);
    let their = theirs_of_key(key);
    let d = diff(&ours(&a), &their);
    assert!(!a.health().clean);
    assert_eq!(
        check_against(KNOWN_DIVERGENCES, key, &d, a.health().clean),
        Ok(())
    );
    // Red: the same divergence on a hand-patched clean `Health` is refused, by
    // name.
    let mut patched = *a.health();
    patched.clean = true;
    match check_against(KNOWN_DIVERGENCES, key, &d, patched.clean) {
        Err(CheckError::HidesButClean { fixture, diff }) => {
            assert_eq!(fixture, key);
            assert!(diff.contains("theirs only: SubIFD* 0x0100 x3"), "{diff}");
        }
        other => panic!("expected HidesButClean, got {other:?}"),
    }
    // A divergence that hides nothing does not need `clean == false`.
    let key = "seed-tiff_ifd/payload-past-eof";
    let bytes_tmp = bytes_of(key);
    let a = audit(&bytes_tmp);
    assert!(a.health().clean);
    let d = diff(&ours(&a), &theirs_of_key(key));
    assert_eq!(check_against(KNOWN_DIVERGENCES, key, &d, true), Ok(()));
}

#[test]
fn a_divergence_that_stops_being_one_fails() {
    // The input agrees with ExifTool (we feed our own inventory as theirs), yet
    // the table pins a difference for it.
    let key = "seed-tiff_ifd/self-cycle";
    let bytes_tmp = bytes_of(key);
    let a = audit(&bytes_tmp);
    let mine = ours(&a);
    let d = diff(&mine, &mine);
    assert!(d.is_empty());
    assert_eq!(
        check_against(KNOWN_DIVERGENCES, key, &d, false),
        Err(CheckError::StoppedDiverging {
            fixture: key.to_string()
        })
    );
    // And an input with no row and no difference is fine.
    assert_eq!(
        check_against(KNOWN_DIVERGENCES, "fixtures/exif-basic-le", &d, true),
        Ok(())
    );
}

#[test]
fn a_divergence_that_changes_its_diff_fails() {
    let key = "seed-tiff_ifd/deep-nesting";
    let bytes_tmp = bytes_of(key);
    let a = audit(&bytes_tmp);
    let their = theirs_of_key(key);
    let real = diff(&ours(&a), &their);
    assert_eq!(check_against(KNOWN_DIVERGENCES, key, &real, false), Ok(()));

    let changed = |d: &inventory::InventoryDiff| -> bool {
        matches!(
            check_against(KNOWN_DIVERGENCES, key, d, false),
            Err(CheckError::ChangedDiff { .. })
        )
    };
    // Shrinks: one fewer entry.
    let mut shrunk = real.clone();
    *shrunk
        .theirs_only
        .get_mut(&("SubIFD*".to_string(), 0x0100))
        .expect("pinned entry") = 2;
    assert!(changed(&shrunk));
    // Grows: a second entry.
    let mut grown = real.clone();
    grown.theirs_only.insert(("SubIFD*".to_string(), 0x0101), 1);
    assert!(changed(&grown));
    // Grows on the other side.
    let mut other = real.clone();
    other.ours_only.insert(("IFD0".to_string(), 0x010f), 1);
    assert!(changed(&other));
    // A different region difference.
    let mut region = real.clone();
    region.regions_ours_only.insert(RegionKind::Xmp);
    assert!(changed(&region));
    // A difference on an input with no row is unexplained.
    assert!(matches!(
        check_against(KNOWN_DIVERGENCES, "fixtures/exif-basic-le", &real, true),
        Err(CheckError::Unexplained { .. })
    ));
    // The failure message names the pinned and the actual difference.
    let msg = check_against(KNOWN_DIVERGENCES, key, &grown, false)
        .expect_err("changed")
        .to_string();
    assert!(msg.contains("pinned") && msg.contains("0x0101"), "{msg}");
}

// ── regions ─────────────────────────────────────────────────────────────────

#[test]
fn regions_agree_on_xmp_iptc_icc_and_jumbf_fixtures() {
    for (key, kind) in [
        ("fixtures/jpeg-xmp", Some(RegionKind::Xmp)),
        ("fixtures/jpeg-iptc", Some(RegionKind::Iptc)),
        ("fixtures/jpeg-icc-valid", Some(RegionKind::Icc)),
        ("fixtures/jpeg-jumbf-valid", Some(RegionKind::Jumbf)),
        ("fixtures/jpeg-extended-xmp-pair", Some(RegionKind::Xmp)),
        // The absent cases: XMP in the wrong marker, and Photoshop resources
        // without the 0x0404 IPTC one (ExifTool lists the Photoshop group
        // only; we list no region).
        ("fixtures/jpeg-xmp-in-wrong-marker", None),
        ("fixtures/jpeg-iptc-without-0404", None),
    ] {
        let mine = ours_of_key(key).regions;
        let their = theirs_of_key(key).regions;
        let want: BTreeSet<RegionKind> = kind.into_iter().collect();
        assert_eq!(mine, want, "{key}: ours");
        assert_eq!(their, want, "{key}: theirs");
    }
    // The ICC fixture's ExifTool listing: sixteen header lines and the
    // description (measured; the SPEC-013 design predicted fifteen).
    let recs = records();
    let sec = section_of(&recs, "fixtures/jpeg-icc-valid");
    let header = sec.lines.iter().filter(|l| l.group == "ICC-header").count();
    assert_eq!(header, 16);
    assert!(sec
        .lines
        .iter()
        .any(|l| l.group == "ICC_Profile" && l.id.is_none() && l.name == "ProfileDescription"));
    let sec = section_of(&recs, "fixtures/jpeg-jumbf-valid");
    let names: Vec<&str> = sec.lines.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, vec!["JUMDType", "JUMDToggles", "JUMDLabel"]);
}

#[test]
fn stub_icc_and_jumbf_are_listed_by_us_and_not_by_exiftool() {
    let recs = records();
    for (key, kind) in [
        ("fixtures/jpeg-icc-stub", RegionKind::Icc),
        ("fixtures/jpeg-jumbf-stub", RegionKind::Jumbf),
    ] {
        // ExifTool lists nothing at all.
        let sec = section_of(&recs, key);
        assert!(sec.lines.is_empty(), "{key}: {:?}", sec.lines);
        // We locate the region.
        let mine = ours_of_key(key);
        assert_eq!(mine.regions, [kind].into_iter().collect(), "{key}");
        // And the table pins exactly that.
        let k = KNOWN_DIVERGENCES
            .iter()
            .find(|k| k.fixture == key)
            .expect("a row");
        assert_eq!(k.class, Class::R);
        assert_eq!(k.regions_ours_only, &[kind]);
        assert!(k.theirs_only.is_empty() && k.regions_theirs_only.is_empty());
    }
    // The eleven seed payloads of the same kind: every Class R row is a region
    // only ours lists, and ours is the side that locates the segment.
    let r = divergences_of(Class::R);
    assert_eq!(r.len(), 13);
    for k in r {
        assert!(k.ours_only.is_empty() && k.theirs_only.is_empty());
        assert!(!k.regions_ours_only.is_empty() && k.regions_theirs_only.is_empty());
        assert_eq!(k.right_side, Side::Ours);
        let their = theirs_of_key(k.fixture);
        assert!(
            k.regions_ours_only
                .iter()
                .all(|r| !their.regions.contains(r)),
            "{}",
            k.fixture
        );
    }
}

#[test]
fn unmodelled_groups_are_exactly_jfif_and_photoshop() {
    assert_eq!(inventory::UNMODELLED_GROUPS, ["JFIF", "Photoshop"]);
    // Both occur in the committed records, and no other group is dropped for
    // being unmodelled: every group in every record classifies, and the ones
    // that classify as unmodelled are exactly these two.
    let recs = records();
    let mut unmodelled: BTreeSet<String> = BTreeSet::new();
    let mut all_groups: BTreeSet<String> = BTreeSet::new();
    for rec in recs.values() {
        for s in &rec.sections {
            for l in &s.lines {
                all_groups.insert(l.group.clone());
                match inventory::classify_group(&l.group) {
                    Some(inventory::GroupClass::Unmodelled) => {
                        unmodelled.insert(l.group.clone());
                    }
                    Some(_) => {}
                    None => panic!("{}: unclassified group {}", s.name, l.group),
                }
            }
        }
    }
    let want: BTreeSet<String> = ["JFIF", "Photoshop"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(unmodelled, want);
    println!("groups seen in the records: {all_groups:?}");
}

// ── the existing records ────────────────────────────────────────────────────

/// FNV-1a 64, written out (std only).
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[test]
fn existing_oracle_records_are_byte_identical_to_the_committed_set() {
    assert_eq!(EXISTING_RECORDS.len(), 125);
    let dir = root().join("tests/oracle-fixtures/exiftool");
    for (name, hash) in EXISTING_RECORDS {
        let bytes = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(fnv1a_64(&bytes), hash, "{name}: the record changed");
    }
    // And there is nothing else in the directory: the inventory records live in
    // their own directory, so adding them moved none of these.
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("read records")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let want: Vec<String> = EXISTING_RECORDS
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    assert_eq!(on_disk, want);
}

// Generated from the 125 records committed before SPEC-013 (FNV-1a 64, sorted).
const EXISTING_RECORDS: [(&str, u64); 125] = [
    ("ascii-count-past-eof.txt", 0x79e5bf42df00b8ac),
    ("ascii-edges.txt", 0xf65f8c527883ff8e),
    ("ascii-shapes.txt", 0x848c6dbffc48ee87),
    ("audit-tag-names.txt", 0xa6ea70fd5020a6e3),
    ("chain-be.txt", 0xacf0cb4535868af2),
    ("chain.txt", 0x69389d8e521587ae),
    ("corrupt-subifd.txt", 0xc2bdaef8cfc2c2a4),
    ("count-overflow-be.txt", 0xc0f070639491719f),
    ("count-overflow.txt", 0xa30617cde6c10ae5),
    ("deep-nesting.txt", 0x93e85e0840d5ba7e),
    ("depth-limit-exact.txt", 0x698437094860321a),
    ("entry-budget-exact.txt", 0x01745a4cd47b58c9),
    ("entry-budget-over.txt", 0xec0548c9a8f60be0),
    ("entry-count-past-eof.txt", 0x51cf96191aa10fef),
    ("exif-basic-be.txt", 0x4f5dfcbdc9ffd5b2),
    ("exif-basic-le.txt", 0xf15687e6310fb37c),
    ("exif-gps-full-le.txt", 0x5d005bbc9c3bd39d),
    ("exif-gps-interop.txt", 0x9543dd3371fd046c),
    ("exif-gps-ne-le.txt", 0x200213b7db4f4b42),
    ("exif-gps-nw-le.txt", 0xfbc77992ab83f9b3),
    ("exif-gps-se-be.txt", 0xbe7cac24c2a4ec26),
    ("exif-gps-sw-be.txt", 0xd2d4550bb2975ee3),
    ("exif-points-at-ifd0.txt", 0x40eeeeddae8f2b7d),
    ("gps-corrupt-exif-ok.txt", 0xd0a4cdc254b677dd),
    ("gps-past-eof-exif-ok.txt", 0x9a122437ab1596ff),
    ("hostile-subifds-overlap.txt", 0x7c6146948b8cdf1c),
    ("ifd0-offset-zero.txt", 0x719852073e2e32af),
    ("ifd1-corrupt.txt", 0xbb91d0ce1818840e),
    ("jpeg-ascii-edges.txt", 0x70e8a5d1d231d398),
    ("jpeg-bad-length.txt", 0xd7e2eaf698e5143d),
    ("jpeg-bad-tiff-in-exif.txt", 0x8627f1e82ffaa974),
    ("jpeg-exif-after-eoi.txt", 0x013f1468e62f0d92),
    ("jpeg-exif-after-scan.txt", 0xa0afaf869cbcb677),
    ("jpeg-exif-basic-be.txt", 0xc93f1b138aa91330),
    ("jpeg-exif-basic-le.txt", 0x0b79dc57b7fff27e),
    ("jpeg-exif-before-and-after-scan.txt", 0x70a9c3f019500cec),
    ("jpeg-exif-eoi-no-sos.txt", 0x82cc3101bd7f2863),
    ("jpeg-exif-fatal-then-good.txt", 0xa0d2f3539f742ba2),
    ("jpeg-exif-fifth-byte-nonzero.txt", 0x32a8d3ca7523a88d),
    ("jpeg-exif-five-byte-payload.txt", 0x750038f6d1be61a9),
    ("jpeg-exif-good-then-fatal.txt", 0xf104dd6a10bdd8c6),
    ("jpeg-exif-gps-full-le.txt", 0xb71b115b84728ee7),
    ("jpeg-exif-gps-ne-le.txt", 0x859454d665a3460c),
    ("jpeg-exif-gps-nw-le.txt", 0x7edf18d11916fe7d),
    ("jpeg-exif-gps-se-be.txt", 0x0ca40f58360de708),
    ("jpeg-exif-gps-sw-be.txt", 0xea4b80c677c13c0d),
    ("jpeg-exif-in-app2.txt", 0xdfa3d20ddb409d28),
    ("jpeg-exif-last-segment-at-eof.txt", 0x558d65dd4945cb59),
    ("jpeg-exif-sixth-byte-nonzero.txt", 0xdc2c946ee92f2501),
    ("jpeg-exif-then-com-at-eof.txt", 0x7426b75e945103b4),
    ("jpeg-exif-then-jfif.txt", 0x817c933dcdddcc41),
    ("jpeg-extended-xmp-pair.txt", 0xe019f45220072967),
    ("jpeg-gps-corrupt-exif-ok.txt", 0xe152dd18c2c84457),
    ("jpeg-gps-split-across-two-blocks.txt", 0xe0f5887d8fc831bc),
    ("jpeg-ifd1-corrupt.txt", 0xb52f835ff53bca30),
    ("jpeg-iptc-without-0404.txt", 0xdcb1b4f0ffbadc9c),
    ("jpeg-iptc.txt", 0xf8a480a996f61117),
    ("jpeg-jfif-then-exif.txt", 0x196708ffdc51f939),
    ("jpeg-seed-app-after-scan.txt", 0x47dde5d69188ccdc),
    ("jpeg-seed-app1-len-0.txt", 0xbe004aff6ee1d758),
    ("jpeg-seed-app1-len-1.txt", 0xe2ad2fc9e0b97dc5),
    ("jpeg-seed-app1-len-2.txt", 0xef45e34cfa211ecb),
    ("jpeg-seed-app1-len-ffff.txt", 0x1b97889916c91b39),
    ("jpeg-seed-app1-past-eof.txt", 0x0739ff1a79e2aea3),
    ("jpeg-seed-exif-after-scan.txt", 0x3496649c76aee6ab),
    ("jpeg-seed-exif-and-xmp.txt", 0x008d077d068e0e7b),
    ("jpeg-seed-exif-bad-tiff.txt", 0x2c7e7aff7e1f0be6),
    ("jpeg-seed-exif-five-byte-payload.txt", 0x83113275eb9942f5),
    ("jpeg-seed-exif-in-jpeg.txt", 0xcddea91ebf189f4f),
    ("jpeg-seed-exif-sixth-byte-nonzero.txt", 0x4686a4213a8a5406),
    ("jpeg-seed-extended-xmp-pair.txt", 0xf51ab685b1707678),
    ("jpeg-seed-extended-xmp-short.txt", 0xf6c6c7858faefddc),
    ("jpeg-seed-ff00-stray.txt", 0x5c4f003fe5d1c23e),
    ("jpeg-seed-fill-bytes.txt", 0x8ea669115f2856a3),
    ("jpeg-seed-icc-chunks-mismatch.txt", 0x37643c067051c5cd),
    ("jpeg-seed-icc-chunks-ok.txt", 0xe545397db2c64af2),
    ("jpeg-seed-icc-iptc-jumbf.txt", 0xcc51b1d0903502e0),
    ("jpeg-seed-icc-seq-zero.txt", 0x1b579e270636d755),
    ("jpeg-seed-iptc-8bim.txt", 0xb090def53ad335f8),
    ("jpeg-seed-iptc-truncated-resource.txt", 0x4a5c7e84de200581),
    ("jpeg-seed-jumbf-header.txt", 0xd57ab7a2104563dc),
    ("jpeg-seed-jumbf-short.txt", 0xd7b03bc35ec0eac9),
    ("jpeg-seed-many-ff00.txt", 0xd301cb5ea70c5753),
    ("jpeg-seed-many-segments.txt", 0x440cf02b981ed010),
    ("jpeg-seed-minimal.txt", 0xab66a8c5b0ec8439),
    ("jpeg-seed-no-sos.txt", 0xd48ff6ad1c0f2870),
    ("jpeg-seed-progressive-two-scans.txt", 0x5c8181ae649fca4d),
    ("jpeg-seed-seventeen-exif.txt", 0x573d9e45ac34c616),
    (
        "jpeg-seed-signatures-under-wrong-marker.txt",
        0x28e3e717a6b27a07,
    ),
    ("jpeg-seed-soi-only.txt", 0x1dbe03f85a5f7f48),
    ("jpeg-seed-sos-no-eoi.txt", 0x717fa4dc2c30c69b),
    ("jpeg-seed-stray-bytes.txt", 0x479cd74316cda12e),
    ("jpeg-seed-trailer-after-eoi.txt", 0x260f85812722b476),
    ("jpeg-seed-two-exif.txt", 0x7f099eccd905d1be),
    ("jpeg-seed-xmp-and-extended-xmp.txt", 0xacbb500ed1e6c01d),
    ("jpeg-seventeen-exif-last-differs.txt", 0xdbe7f27734f3491c),
    ("jpeg-signed-float-be.txt", 0xa40158e4c2cb3d76),
    ("jpeg-signed-float-le.txt", 0x9b5e2d76bd453bcc),
    ("jpeg-stray-bytes.txt", 0x380e36e68de3ab4e),
    (
        "jpeg-truncated-between-app1-and-sos.txt",
        0x015ff76d2d5ce7d1,
    ),
    ("jpeg-truncated-mid-app1.txt", 0xeaadc9bf1d09dda9),
    ("jpeg-truncated-mid-scan.txt", 0x9e40825943c0998e),
    ("jpeg-two-exif-disjoint-tags.txt", 0x21144da5f036c0d4),
    ("jpeg-two-exif-same-tag-reversed.txt", 0xd0ddaeff8f4ea86f),
    ("jpeg-two-exif-same-tag.txt", 0xfa105de6e3f837f2),
    ("jpeg-xmp-in-wrong-marker.txt", 0xb1d91dd69e4dc1c4),
    ("jpeg-xmp.txt", 0xe430011a5cea4fd8),
    ("leaf-next-word.txt", 0x78b80209148a3c2e),
    ("malformed-subifds-tag.txt", 0xad15d98b40e631b2),
    ("next-past-eof.txt", 0x414e768fd731d370),
    ("overlapping-subifds.txt", 0x178d1a4554d807f7),
    ("payload-past-eof-be.txt", 0x488e10efe866a5e0),
    ("payload-past-eof.txt", 0x97e2d3649c26c27e),
    ("pointer-bad-count.txt", 0x2edc1519d5238d1a),
    ("self-cycle.txt", 0x08bc8d1b106ce626),
    ("signed-and-float.txt", 0xfea308cb93380378),
    ("signed-float-be.txt", 0x4eaa0e5baf1cc8c8),
    ("signed-float-le.txt", 0x13f0c74d9b84a30a),
    ("subifd-be.txt", 0x7907d79c74cf0b68),
    ("subifd.txt", 0xaef48c028e0b4d14),
    ("subifds-count-over-cap.txt", 0xe770cce2034c9dd3),
    ("truncated-header.txt", 0x02bac1febad20af1),
    ("two-hop-cycle.txt", 0x85d904043923ed71),
    ("valid-be.txt", 0x2a8146e15f893d30),
    ("valid-le.txt", 0xe015beb701611dbe),
];

// ── red-proofs: ours-side perturbation ──────────────────────────────────────

/// The inventory pair of an input: ours (from the bytes) and theirs (from the
/// committed record).
fn pair(key: &str) -> (Inventory, Inventory) {
    (ours_of_key(key), theirs_of_key(key))
}

/// Control first: the unperturbed pair has no difference and no table row.
fn control(key: &str) -> (Inventory, Inventory) {
    let (mine, their) = pair(key);
    let d = diff(&mine, &their);
    assert!(d.is_empty(), "{key}: control run is not clean:\n{d}");
    assert!(KNOWN_DIVERGENCES.iter().all(|k| k.fixture != key));
    (mine, their)
}

#[test]
fn oracle_names_a_dropped_entry() {
    let key = "fixtures/exif-gps-full-le";
    let (mut mine, their) = control(key);
    assert_eq!(mine.tags.remove(&("IFD0".to_string(), 0x010f)), Some(1));
    let d = diff(&mine, &their);
    let want: Multiset = [(("IFD0".to_string(), 0x010f), 1)].into_iter().collect();
    assert_eq!(d.theirs_only, want);
    assert!(d.ours_only.is_empty());
    assert!(d.to_string().contains("theirs only: IFD0 0x010f x1"), "{d}");
    assert!(matches!(
        check_against(&[], key, &d, true),
        Err(CheckError::Unexplained { .. })
    ));
}

type Multiset = inventory::Multiset;

#[test]
fn oracle_names_an_invented_entry() {
    let key = "fixtures/exif-gps-full-le";
    let (mut mine, their) = control(key);
    mine.tags.insert(("ExifIFD".to_string(), 0x9999), 1);
    let d = diff(&mine, &their);
    let want: Multiset = [(("ExifIFD".to_string(), 0x9999), 1)].into_iter().collect();
    assert_eq!(d.ours_only, want);
    assert!(d.theirs_only.is_empty());
    assert!(
        d.to_string().contains("ours only:   ExifIFD 0x9999 x1"),
        "{d}"
    );
    // One more copy of an entry that is there is named by its count.
    let (mut mine, their) = control(key);
    *mine
        .tags
        .get_mut(&("IFD0".to_string(), 0x010f))
        .expect("entry") += 1;
    let d = diff(&mine, &their);
    assert!(d.to_string().contains("ours only:   IFD0 0x010f x1"), "{d}");
}

#[test]
fn oracle_names_a_regrouped_entry() {
    let key = "fixtures/exif-gps-full-le";
    let (mut mine, their) = control(key);
    // A GPS entry labelled as an Exif IFD entry.
    assert_eq!(mine.tags.remove(&("GPS".to_string(), 0x0001)), Some(1));
    mine.tags.insert(("ExifIFD".to_string(), 0x0001), 1);
    let d = diff(&mine, &their);
    let ours_only: Multiset = [(("ExifIFD".to_string(), 0x0001), 1)].into_iter().collect();
    let theirs_only: Multiset = [(("GPS".to_string(), 0x0001), 1)].into_iter().collect();
    assert_eq!(
        (d.ours_only.clone(), d.theirs_only.clone()),
        (ours_only, theirs_only)
    );
    let text = d.to_string();
    assert!(text.contains("ours only:   ExifIFD 0x0001 x1"), "{text}");
    assert!(text.contains("theirs only: GPS 0x0001 x1"), "{text}");
}

#[test]
fn oracle_names_a_renumbered_chain_group() {
    let key = "fixtures/tiff-chain-four";
    let (mine, their) = control(key);
    // IFD2 renumbered as IFD3 (and IFD3 as IFD4, the off-by-one of a chain
    // position).
    let mut moved = Inventory::default();
    for ((g, id), n) in &mine.tags {
        let g2 = match g.as_str() {
            "IFD2" => "IFD3",
            "IFD3" => "IFD4",
            other => other,
        };
        moved.tags.insert((g2.to_string(), *id), *n);
    }
    let d = diff(&moved, &their);
    let ours_only: Multiset = [
        (("IFD3".to_string(), 0x0131), 1),
        (("IFD4".to_string(), 0x013b), 1),
    ]
    .into_iter()
    .collect();
    let theirs_only: Multiset = [
        (("IFD2".to_string(), 0x0131), 1),
        (("IFD3".to_string(), 0x013b), 1),
    ]
    .into_iter()
    .collect();
    assert_eq!(d.ours_only, ours_only);
    assert_eq!(d.theirs_only, theirs_only);
}

#[test]
fn oracle_names_an_extra_region() {
    let key = "fixtures/jpeg-xmp";
    let (mut mine, their) = control(key);
    assert_eq!(mine.regions, [RegionKind::Xmp].into_iter().collect());
    mine.regions.insert(RegionKind::Iptc);
    let d = diff(&mine, &their);
    assert_eq!(
        d.regions_ours_only,
        [RegionKind::Iptc].into_iter().collect()
    );
    assert!(d.to_string().contains("ours only:   region Iptc"), "{d}");
    // And a region we fail to list is named on the other side.
    let (mut mine, their) = control(key);
    mine.regions.clear();
    let d = diff(&mine, &their);
    assert_eq!(
        d.regions_theirs_only,
        [RegionKind::Xmp].into_iter().collect()
    );
    assert!(d.to_string().contains("theirs only: region Xmp"), "{d}");
}

// ── red-proofs: their-side perturbation of a committed record ───────────────

fn lines_of(key: &str) -> Vec<Line> {
    section_of(&records(), key).lines
}

#[test]
fn oracle_names_a_deleted_line_in_a_record() {
    let key = "fixtures/exif-gps-full-le";
    let (mine, _) = control(key);
    let mut lines = lines_of(key);
    let at = lines
        .iter()
        .position(|l| l.group == "GPS" && l.id == Some(0x0002))
        .expect("the GPS latitude line");
    lines.remove(at);
    let d = diff(&mine, &their_inventory(&lines, false));
    let want: Multiset = [(("GPS".to_string(), 0x0002), 1)].into_iter().collect();
    assert_eq!(d.ours_only, want);
    assert!(d.theirs_only.is_empty());
}

#[test]
fn oracle_names_a_changed_id_in_a_record() {
    let key = "fixtures/exif-gps-full-le";
    let (mine, _) = control(key);
    let mut lines = lines_of(key);
    let l = lines
        .iter_mut()
        .find(|l| l.group == "IFD0" && l.id == Some(0x010f))
        .expect("the Make line");
    l.id = Some(0x0123);
    let d = diff(&mine, &their_inventory(&lines, false));
    let ours_only: Multiset = [(("IFD0".to_string(), 0x010f), 1)].into_iter().collect();
    let theirs_only: Multiset = [(("IFD0".to_string(), 0x0123), 1)].into_iter().collect();
    assert_eq!(d.ours_only, ours_only);
    assert_eq!(d.theirs_only, theirs_only);
}

#[test]
fn oracle_names_a_changed_group_in_a_record() {
    let key = "fixtures/exif-gps-full-le";
    let (mine, _) = control(key);
    let mut lines = lines_of(key);
    let l = lines
        .iter_mut()
        .find(|l| l.group == "GPS" && l.id == Some(0x0001))
        .expect("the GPS ref line");
    l.group = "ExifIFD".to_string();
    let d = diff(&mine, &their_inventory(&lines, false));
    let ours_only: Multiset = [(("GPS".to_string(), 0x0001), 1)].into_iter().collect();
    let theirs_only: Multiset = [(("ExifIFD".to_string(), 0x0001), 1)].into_iter().collect();
    assert_eq!(d.ours_only, ours_only);
    assert_eq!(d.theirs_only, theirs_only);
}

// ── red-proofs: byte mutation of a fixture ──────────────────────────────────

#[test]
fn byte_mutation_changes_one_tag_id_in_a_tiff_and_is_named() {
    let key = "fixtures/exif-basic-le";
    // Control: the committed record is the unmutated file's answer.
    let (_, their) = control(key);
    let mut bytes = bytes_of(key);
    let order = mutate::order_of(&bytes).expect("TIFF header");
    let loc = mutate::locate_entry(&bytes, 8, 0x010f).expect("the Make entry in IFD0");
    assert!(mutate::set_u16_at(&mut bytes, loc.entry_at, 0x0123, order));
    let d = diff(&ours(&audit(&bytes)), &their);
    let ours_only: Multiset = [(("IFD0".to_string(), 0x0123), 1)].into_iter().collect();
    let theirs_only: Multiset = [(("IFD0".to_string(), 0x010f), 1)].into_iter().collect();
    assert_eq!(d.ours_only, ours_only, "{d}");
    assert_eq!(d.theirs_only, theirs_only, "{d}");
}

/// Where the TIFF blob of a JPEG's first Exif APP1 starts.
fn tiff_start_in_jpeg(bytes: &[u8]) -> usize {
    bytes
        .windows(6)
        .position(|w| w == b"Exif\0\0")
        .expect("an Exif signature")
        + 6
}

#[test]
fn byte_mutation_changes_one_tag_id_in_a_jpeg_app1_and_is_named() {
    let key = "fixtures/jpeg-exif-basic-le";
    let (_, their) = control(key);
    let mut bytes = bytes_of(key);
    let start = tiff_start_in_jpeg(&bytes);
    let tiff = bytes.get(start..).expect("TIFF bytes").to_vec();
    let order = mutate::order_of(&tiff).expect("TIFF header");
    let loc = mutate::locate_entry(&tiff, 8, 0x010f).expect("the Make entry in IFD0");
    // The tag field sits at (TIFF offset + 10 + 12 * index) inside the APP1.
    assert!(loc.entry_at >= 10 && (loc.entry_at - 10).is_multiple_of(12));
    assert!(mutate::set_u16_at(
        &mut bytes,
        start + loc.entry_at,
        0x0123,
        order
    ));
    let d = diff(&ours(&audit(&bytes)), &their);
    let ours_only: Multiset = [(("IFD0".to_string(), 0x0123), 1)].into_iter().collect();
    let theirs_only: Multiset = [(("IFD0".to_string(), 0x010f), 1)].into_iter().collect();
    assert_eq!(d.ours_only, ours_only, "{d}");
    assert_eq!(d.theirs_only, theirs_only, "{d}");
}

#[test]
fn truncating_between_app1_and_sos_leaves_the_inventory_unchanged() {
    let key = "fixtures/jpeg-exif-basic-le";
    let (_, their) = control(key);
    let mut bytes = bytes_of(key);
    // SOI (2) + APP1 marker (2) + the APP1 length (which counts its own two
    // bytes): the end of the Exif segment, where the SOS header begins.
    let len = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let cut = 2 + 2 + len;
    assert_eq!(
        &bytes[cut..cut + 2],
        &[0xFF, 0xDA],
        "the SOS follows the APP1"
    );
    mutate::truncate_at(&mut bytes, cut);
    let a = audit(&bytes);
    // The typed outcome of SPEC-011: the inventory is the same; what changed is
    // the JPEG layer (no scan, no EOI), which the health reports.
    assert!(diff(&ours(&a), &their).is_empty());
    assert!(a.health().jpeg_issues > 0 && !a.health().clean);
    // And the committed record of the hand-built truncated shape agrees.
    let (mine, their) = control("fixtures/jpeg-truncated-between-app1-and-sos");
    assert!(diff(&mine, &their).is_empty());
}
