//! SPEC-017: the total entry budget against ExifTool, and the proof that the
//! budget moved nothing else.
//!
//! ExifTool 13.55 is not exposed to the overlapping-SubIFD shape: it keeps the
//! first 10 SubIFD directories, prints `Warning: Ignoring 501 SubIFD
//! directories` and returns in about 0.04 s at about 22 MB. This library does
//! not copy that policy (its SubIFD cap stays `MAX_IFDS`, DEC-005); it bounds
//! the retained entries instead (`len / 12`, DEC-001). The committed record
//! `tests/oracle-fixtures/exiftool/hostile-subifds-overlap.txt` holds
//! ExifTool's answer (dashes and the warning); the replay test needs no tool,
//! the live test runs ExifTool under the tool time limit and skips loudly
//! without it (`DIAMOND_EXIF_REQUIRE_ORACLE=1` makes that a failure).
//!
//! `existing_records_and_seeds_are_byte_identical` pins the FNV-1a 64 hash of
//! every record and every seed that existed before SPEC-017, so the budget
//! cannot silently change a recorded answer or a seed.

#[path = "support/hostile.rs"]
mod hostile;
#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/tools.rs"]
mod tools;

use exif_core::tiff::{entry_budget, Container};
use exif_core::Error;
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

/// Our side of the shape: bounded, one `TooManyEntries`, nothing else.
fn assert_we_report_the_budget(bytes: &[u8]) {
    let c = Container::parse(bytes).expect("parses");
    let budget = entry_budget(bytes.len());
    assert!(c.entry_count() <= budget);
    assert_eq!(c.issues().len(), 1, "{:?}", c.issues());
    assert_eq!(
        c.issues()[0].error(),
        &Error::TooManyEntries { limit: budget }
    );
}

#[test]
fn replay_hostile_subifds_record_header_has_the_warning() {
    let path = root().join(hostile::RECORD);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (run `just oracle-record`)",
            path.display()
        )
    });
    let rec = tools::parse_record(&text).expect("record header");
    assert_eq!(rec.version, "13.55");
    assert_eq!(rec.source, "hostile hostile-subifds-overlap");
    assert!(rec.outcome.is_none());
    // ExifTool kept the first 10 SubIFDs and said so; it read none of the
    // three IFD0 selectors.
    assert_eq!(
        rec.stderr,
        vec!["Warning: Ignoring 501 SubIFD directories - <file>".to_string()]
    );
    let names = oracle::names(oracle::SEED_SELECTORS);
    let fields = tools::parse_fields(&names, &rec.stdout).expect("fields");
    assert!(fields.iter().all(Option::is_none), "{fields:?}");
    assert_eq!(rec.command, tools::exiftool_command_line(&names));
    // And ours, on the same bytes, with no tool.
    assert_we_report_the_budget(&hostile::hostile(hostile::N_SUB));
}

#[test]
fn live_hostile_subifds_exiftool_survives_and_we_report_the_budget() {
    if !live_gate("live_hostile_subifds_exiftool_survives_and_we_report_the_budget") {
        return;
    }
    let bytes = hostile::hostile(hostile::N_SUB);
    let path = tools::temp_file_for("hostile-subifds-overlap-live", &bytes).expect("temp file");
    let names = oracle::names(oracle::SEED_SELECTORS);
    // `run_tool` kills ExifTool after the tool time limit (5 s by default), so
    // a runaway fails this test instead of hanging it.
    let run = tools::run_exiftool_full(&path, &names);
    let _ = std::fs::remove_file(&path);
    let (stdout, stderr) = run.expect("ExifTool returned");
    let warnings = tools::warning_lines(&stderr, &path);
    assert_eq!(
        warnings,
        vec!["Warning: Ignoring 501 SubIFD directories - <file>".to_string()]
    );
    let fields = tools::parse_fields(&names, &stdout).expect("fields");
    assert!(fields.iter().all(Option::is_none), "{fields:?}");
    // The live answer is the committed one.
    let rec_text = std::fs::read_to_string(root().join(hostile::RECORD)).expect("record");
    let rec = tools::parse_record(&rec_text).expect("record header");
    assert_eq!(rec.stdout, stdout);
    assert_eq!(rec.stderr, warnings);
    assert_we_report_the_budget(&bytes);
}

/// FNV-1a 64, written out (std only).
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Every file of `dir` named in `old` must have exactly its recorded hash;
/// `extra` lists the only files allowed besides them.
fn check_dir(dir: &str, old: &[(&str, u64)], extra: &[&str]) {
    let base = root().join(dir);
    for (name, hash) in old {
        let path = base.join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(fnv1a_64(&bytes), *hash, "{dir}/{name}: contents changed");
    }
    let mut on_disk: Vec<String> = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("{}: {e}", base.display()))
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let mut want: Vec<String> = old.iter().map(|(n, _)| n.to_string()).collect();
    want.extend(extra.iter().map(|n| n.to_string()));
    want.sort();
    assert_eq!(on_disk, want, "{dir}: the set of files");
}

#[test]
fn existing_records_and_seeds_are_byte_identical() {
    assert_eq!(OLD_RECORDS.len(), 121);
    assert_eq!(OLD_TIFF_SEEDS.len(), 29);
    assert_eq!(OLD_JPEG_SEEDS.len(), 37);
    assert_eq!(OLD_AUDIT_SEEDS.len(), 73);
    // Records: the 121 old ones plus the hostile record and the three records
    // of the new `tiff_ifd` seeds (the recorder writes one per seed).
    check_dir(
        "tests/oracle-fixtures/exiftool",
        &OLD_RECORDS,
        &[
            "hostile-subifds-overlap.txt",
            "entry-budget-exact.txt",
            "entry-budget-over.txt",
            "overlapping-subifds.txt",
        ],
    );
    let new_tiff = [
        "entry-budget-exact",
        "entry-budget-over",
        "overlapping-subifds",
    ];
    check_dir("fuzz/seeds/tiff_ifd", &OLD_TIFF_SEEDS, &new_tiff);
    check_dir("fuzz/seeds/jpeg_segments", &OLD_JPEG_SEEDS, &[]);
    // The audit directory copies each `tiff_ifd` seed under a `tiff-` prefix.
    let new_audit = [
        "tiff-entry-budget-exact",
        "tiff-entry-budget-over",
        "tiff-overlapping-subifds",
    ];
    check_dir("fuzz/seeds/audit", &OLD_AUDIT_SEEDS, &new_audit);
}

// Generated from the files committed before SPEC-017 (FNV-1a 64, sorted by name).
const OLD_RECORDS: [(&str, u64); 121] = [
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

const OLD_TIFF_SEEDS: [(&str, u64); 29] = [
    ("ascii-count-past-eof", 0xf9fb168ecf63c873),
    ("ascii-shapes", 0x40686d2b31db9f20),
    ("chain", 0x5103cafa5ffe6da1),
    ("chain-be", 0x0e7a1e63dbe9517d),
    ("corrupt-subifd", 0x8dad61eefdeaa933),
    ("count-overflow", 0x0ca614f0bde8ce1c),
    ("count-overflow-be", 0x46a5d940e364b10e),
    ("deep-nesting", 0x86bc5a943f3da84d),
    ("depth-limit-exact", 0x2d1ea0dcff4e5a83),
    ("entry-count-past-eof", 0x78590c388acc9606),
    ("exif-gps-interop", 0x662b025c31b0e761),
    ("exif-points-at-ifd0", 0xbd387c33c1a06741),
    ("gps-past-eof-exif-ok", 0xc46df5c8bc378c6b),
    ("ifd0-offset-zero", 0x432d45b4d7c255d5),
    ("leaf-next-word", 0x85063aa66064a89b),
    ("malformed-subifds-tag", 0xb547aa14c20643ba),
    ("next-past-eof", 0x802bbf014b971d79),
    ("payload-past-eof", 0x24228b3502290f30),
    ("payload-past-eof-be", 0xb936734a88d49c7a),
    ("pointer-bad-count", 0x41444d715968b875),
    ("self-cycle", 0x7e9d0243fc17108b),
    ("signed-and-float", 0xc64c538111f83ebb),
    ("subifd", 0x6f3e5be266b40257),
    ("subifd-be", 0xb367930d24976e8b),
    ("subifds-count-over-cap", 0x29ca58ae79ebcb95),
    ("truncated-header", 0x50e0a90335f50b17),
    ("two-hop-cycle", 0xba54c41e6b2198b3),
    ("valid-be", 0xd14e46fc095cffa9),
    ("valid-le", 0x80fe123345fe5215),
];

const OLD_JPEG_SEEDS: [(&str, u64); 37] = [
    ("app-after-scan", 0x3c0b74c4ea41965e),
    ("app1-len-0", 0xa71f9b333b778368),
    ("app1-len-1", 0xd3cb0a5e7ebf3f1f),
    ("app1-len-2", 0x03f1c23d02e437f0),
    ("app1-len-ffff", 0xc760bd59985fc432),
    ("app1-past-eof", 0xf7f17c5736b2e083),
    ("exif-after-scan", 0x10ad14693c9e9c6b),
    ("exif-and-xmp", 0xadb9ce26dd6e2b38),
    ("exif-bad-tiff", 0xb11d03c3f7ca9288),
    ("exif-five-byte-payload", 0xe5ae7fec035eef93),
    ("exif-in-jpeg", 0x7c21afba5dea8c89),
    ("exif-sixth-byte-nonzero", 0x929d86c9b12852c6),
    ("extended-xmp-pair", 0x8768c0104678fa1d),
    ("extended-xmp-short", 0xc2fb6f7d331f2226),
    ("ff00-stray", 0x329ecc00b7b39d86),
    ("fill-bytes", 0x0c6af3c3636b50db),
    ("icc-chunks-mismatch", 0xcf9d799ac0d0f1e6),
    ("icc-chunks-ok", 0xd70da14137a10c3a),
    ("icc-iptc-jumbf", 0xbe84bb0a5f3b3ec3),
    ("icc-seq-zero", 0x49e811923270b17b),
    ("iptc-8bim", 0xc12b2f98a1b99274),
    ("iptc-truncated-resource", 0xd533a41f8b100bb4),
    ("jumbf-header", 0xbca67c1eeca4ff73),
    ("jumbf-short", 0xb3cae112b487977c),
    ("many-ff00", 0x1a0b709fa81c45f2),
    ("many-segments", 0xc675c75e5ea8ad5e),
    ("minimal", 0x47af3ff740559a4a),
    ("no-sos", 0x1bdded91eb609b96),
    ("progressive-two-scans", 0xd126899024ddea18),
    ("seventeen-exif", 0x6e50da0b2ab0c175),
    ("signatures-under-wrong-marker", 0xe0544e0816d47269),
    ("soi-only", 0x0a99ee07b6f68642),
    ("sos-no-eoi", 0x0e30d944ac162394),
    ("stray-bytes", 0xcaad01d80bb242c1),
    ("trailer-after-eoi", 0x10a1ad72a76b0149),
    ("two-exif", 0x546b67de32768525),
    ("xmp-and-extended-xmp", 0x7cd5037ca5f8982e),
];

const OLD_AUDIT_SEEDS: [(&str, u64); 73] = [
    ("audit-bigtiff-magic", 0xb694d19012a33384),
    ("audit-empty-marker", 0xcbf29ce484222325),
    ("audit-exif-gps-and-thumbnail", 0x0d3cbd1d12082342),
    ("audit-ifd1-strips", 0xa548004e68874563),
    ("audit-many-overlapping-ifds", 0x320552cbf7555441),
    ("audit-png-magic", 0xba9d4d103109abc8),
    ("audit-two-blocks-same-tag", 0x950fbb905aba86fe),
    ("jpeg-app-after-scan", 0x3c0b74c4ea41965e),
    ("jpeg-app1-len-0", 0xa71f9b333b778368),
    ("jpeg-app1-len-1", 0xd3cb0a5e7ebf3f1f),
    ("jpeg-app1-len-2", 0x03f1c23d02e437f0),
    ("jpeg-app1-len-ffff", 0xc760bd59985fc432),
    ("jpeg-app1-past-eof", 0xf7f17c5736b2e083),
    ("jpeg-exif-after-scan", 0x10ad14693c9e9c6b),
    ("jpeg-exif-and-xmp", 0xadb9ce26dd6e2b38),
    ("jpeg-exif-bad-tiff", 0xb11d03c3f7ca9288),
    ("jpeg-exif-five-byte-payload", 0xe5ae7fec035eef93),
    ("jpeg-exif-in-jpeg", 0x7c21afba5dea8c89),
    ("jpeg-exif-sixth-byte-nonzero", 0x929d86c9b12852c6),
    ("jpeg-extended-xmp-pair", 0x8768c0104678fa1d),
    ("jpeg-extended-xmp-short", 0xc2fb6f7d331f2226),
    ("jpeg-ff00-stray", 0x329ecc00b7b39d86),
    ("jpeg-fill-bytes", 0x0c6af3c3636b50db),
    ("jpeg-icc-chunks-mismatch", 0xcf9d799ac0d0f1e6),
    ("jpeg-icc-chunks-ok", 0xd70da14137a10c3a),
    ("jpeg-icc-iptc-jumbf", 0xbe84bb0a5f3b3ec3),
    ("jpeg-icc-seq-zero", 0x49e811923270b17b),
    ("jpeg-iptc-8bim", 0xc12b2f98a1b99274),
    ("jpeg-iptc-truncated-resource", 0xd533a41f8b100bb4),
    ("jpeg-jumbf-header", 0xbca67c1eeca4ff73),
    ("jpeg-jumbf-short", 0xb3cae112b487977c),
    ("jpeg-many-ff00", 0x1a0b709fa81c45f2),
    ("jpeg-many-segments", 0xc675c75e5ea8ad5e),
    ("jpeg-minimal", 0x47af3ff740559a4a),
    ("jpeg-no-sos", 0x1bdded91eb609b96),
    ("jpeg-progressive-two-scans", 0xd126899024ddea18),
    ("jpeg-seventeen-exif", 0x6e50da0b2ab0c175),
    ("jpeg-signatures-under-wrong-marker", 0xe0544e0816d47269),
    ("jpeg-soi-only", 0x0a99ee07b6f68642),
    ("jpeg-sos-no-eoi", 0x0e30d944ac162394),
    ("jpeg-stray-bytes", 0xcaad01d80bb242c1),
    ("jpeg-trailer-after-eoi", 0x10a1ad72a76b0149),
    ("jpeg-two-exif", 0x546b67de32768525),
    ("jpeg-xmp-and-extended-xmp", 0x7cd5037ca5f8982e),
    ("tiff-ascii-count-past-eof", 0xf9fb168ecf63c873),
    ("tiff-ascii-shapes", 0x40686d2b31db9f20),
    ("tiff-chain", 0x5103cafa5ffe6da1),
    ("tiff-chain-be", 0x0e7a1e63dbe9517d),
    ("tiff-corrupt-subifd", 0x8dad61eefdeaa933),
    ("tiff-count-overflow", 0x0ca614f0bde8ce1c),
    ("tiff-count-overflow-be", 0x46a5d940e364b10e),
    ("tiff-deep-nesting", 0x86bc5a943f3da84d),
    ("tiff-depth-limit-exact", 0x2d1ea0dcff4e5a83),
    ("tiff-entry-count-past-eof", 0x78590c388acc9606),
    ("tiff-exif-gps-interop", 0x662b025c31b0e761),
    ("tiff-exif-points-at-ifd0", 0xbd387c33c1a06741),
    ("tiff-gps-past-eof-exif-ok", 0xc46df5c8bc378c6b),
    ("tiff-ifd0-offset-zero", 0x432d45b4d7c255d5),
    ("tiff-leaf-next-word", 0x85063aa66064a89b),
    ("tiff-malformed-subifds-tag", 0xb547aa14c20643ba),
    ("tiff-next-past-eof", 0x802bbf014b971d79),
    ("tiff-payload-past-eof", 0x24228b3502290f30),
    ("tiff-payload-past-eof-be", 0xb936734a88d49c7a),
    ("tiff-pointer-bad-count", 0x41444d715968b875),
    ("tiff-self-cycle", 0x7e9d0243fc17108b),
    ("tiff-signed-and-float", 0xc64c538111f83ebb),
    ("tiff-subifd", 0x6f3e5be266b40257),
    ("tiff-subifd-be", 0xb367930d24976e8b),
    ("tiff-subifds-count-over-cap", 0x29ca58ae79ebcb95),
    ("tiff-truncated-header", 0x50e0a90335f50b17),
    ("tiff-two-hop-cycle", 0xba54c41e6b2198b3),
    ("tiff-valid-be", 0xd14e46fc095cffa9),
    ("tiff-valid-le", 0x80fe123345fe5215),
];
