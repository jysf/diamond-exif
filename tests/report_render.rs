//! SPEC-015: the three renderers over real inputs.
//!
//! * the golden files (`tests/golden/report-full-house.{txt,json,csv}`) match
//!   byte for byte; regenerating one is a deliberate edit, never automatic;
//! * every committed seed of the four fuzz targets, under every policy and
//!   every `inventory`/`values` combination, renders output the independent
//!   strict parsers accept, with the facts guards green;
//! * a truncated audit previews conservatively end to end (carried F2);
//! * the largest output at the audit caps is measured.

#[path = "support/report_inputs.rs"]
mod report_inputs;

include!("../fuzz/shared/audit_invariants.rs");
include!("../fuzz/shared/render_check.rs");
include!("../fuzz/shared/render_invariants.rs");

use std::fs;
use std::path::PathBuf;

use exif_core::audit::{audit, Category, MAX_INVENTORY_ITEMS};
use exif_core::report::{
    render, render_csv, render_json, render_text, FailReason, FileEntry, FileResult, Format,
    RenderOptions, ReportBuilder,
};
use exif_core::StripPolicy;
use render_check::{parse_csv, parse_json, J};
use render_invariants::{build_case, CaseOpts};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ── golden files ────────────────────────────────────────────────────────────

/// The golden run: the full-house JPEG, an unsupported PNG signature and a
/// failed file, policy `all_pii`, inventory off, values on.
#[test]
fn golden_text_json_csv_match_byte_for_byte() {
    let jpeg = report_inputs::full_house_jpeg();
    let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let files = vec![
        FileEntry {
            label: b"photos/full-house.jpg",
            result: FileResult::Audited(audit(&jpeg)),
        },
        FileEntry {
            label: b"photos/screenshot.png",
            result: FileResult::Audited(audit(&png)),
        },
        FileEntry {
            label: b"photos/locked.jpg",
            result: FileResult::Failed {
                reason: FailReason::Unreadable,
                detail: b"permission denied (os error 13)",
            },
        },
    ];
    let opts = RenderOptions {
        policy: Some(StripPolicy::AllPii),
        inventory: false,
        values: true,
    };
    for (ext, got) in [
        ("txt", render_text(&files, &opts)),
        ("json", render_json(&files, &opts)),
        ("csv", render_csv(&files, &opts)),
    ] {
        let path = root().join(format!("tests/golden/report-full-house.{ext}"));
        let want = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            got.as_bytes(),
            &want[..],
            "{ext}: the output differs from {}; a golden file changes only by a deliberate edit",
            path.display()
        );
    }
}

/// SPEC-018: the builder reproduces every golden file, in all three formats,
/// through `render` and through `ReportBuilder` directly; the golden files
/// themselves are the committed bytes (the one-shot test above is unchanged).
#[test]
fn golden_files_are_unchanged_and_the_builder_reproduces_them() {
    let jpeg = report_inputs::full_house_jpeg();
    let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let files = vec![
        FileEntry {
            label: b"photos/full-house.jpg",
            result: FileResult::Audited(audit(&jpeg)),
        },
        FileEntry {
            label: b"photos/screenshot.png",
            result: FileResult::Audited(audit(&png)),
        },
        FileEntry {
            label: b"photos/locked.jpg",
            result: FileResult::Failed {
                reason: FailReason::Unreadable,
                detail: b"permission denied (os error 13)",
            },
        },
    ];
    let opts = RenderOptions {
        policy: Some(StripPolicy::AllPii),
        inventory: false,
        values: true,
    };
    for (ext, format) in [
        ("txt", Format::Text),
        ("json", Format::Json),
        ("csv", Format::Csv),
    ] {
        let path = root().join(format!("tests/golden/report-full-house.{ext}"));
        let want = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        // Through the builder, adding the files one at a time.
        let mut b = ReportBuilder::new(format, opts);
        for f in &files {
            b.add(f);
        }
        assert_eq!(b.len(), 3);
        assert_eq!(
            b.finish().as_bytes(),
            &want[..],
            "{ext}: the builder differs from {}",
            path.display()
        );
        // Through `render`.
        assert_eq!(
            render(format, &files, &opts).as_bytes(),
            &want[..],
            "{ext}: render() differs from {}",
            path.display()
        );
    }
}

// ── the cross product ───────────────────────────────────────────────────────

fn seed_files() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for target in ["tiff_ifd", "jpeg_segments", "audit", "audit_render"] {
        let dir = root().join("fuzz/seeds").join(target);
        let mut names: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|e| e.expect("entry").path())
            .collect();
        names.sort();
        for path in names {
            let data = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            out.push((
                format!("{target}/{}", path.file_name().unwrap().to_string_lossy()),
                data,
            ));
        }
    }
    out
}

/// Labels that would break a naive renderer.
const LABELS: [&[u8]; 8] = [
    b"photos/a.jpg",
    b"",
    b"=cmd|' /C calc'!A0",
    b"a,\"b\"\r\nc",
    b"\x1b[31m\xE2\x80\xAEgpj.exe\xC2\x85\xE2\x80\xA8\x7f",
    b"\xFF\xFE\xC3(",
    b"@SUM(1)+1",
    b"-1\t\r",
];

#[test]
fn every_seed_every_policy_every_option_renders_well_formed_output() {
    let seeds = seed_files();
    assert!(
        seeds.len() >= 32 + 37 + 78 + 14,
        "{} seed files",
        seeds.len()
    );
    let mut cases = 0usize;
    for (n, (name, data)) in seeds.iter().enumerate() {
        let label = LABELS[n % LABELS.len()];
        let detail = &data[..data.len().min(64)];
        // No policy, the five presets, and a Custom spec.
        for policy in [None, Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)] {
            for (inventory, values) in [(false, false), (true, false), (false, true), (true, true)]
            {
                let reason = (n + cases) % 4;
                let c = build_case(
                    data,
                    label,
                    detail,
                    CaseOpts {
                        policy,
                        custom_byte: n as u8,
                        inventory,
                        values,
                        reason,
                    },
                );
                // The checkers accept all three formats.
                parse_json(c.json.as_bytes()).unwrap_or_else(|e| panic!("{name}: {e}"));
                parse_csv(c.csv.as_bytes()).unwrap_or_else(|e| panic!("{name}: {e}"));
                // And every facts guard is green.
                let bad = render_invariants::violations(&c);
                assert!(
                    bad.is_empty(),
                    "{name} policy {policy:?} inventory {inventory} values {values}: {bad:?}"
                );
                cases += 1;
            }
        }
    }
    println!("{cases} cases over {} seed files", seeds.len());
}

/// SPEC-018: over every committed seed of the four fuzz targets, the three
/// formats, no policy and the six policies, the four `inventory`/`values`
/// combinations and 0, 1, 2 and 5 entries, the builder's output equals the
/// one-shot renderer's and its summary equals `summarize`; the builder's
/// output is also accepted by the strict parsers.
#[test]
fn builder_equals_the_one_shot_on_the_seed_cross_product() {
    let seeds = seed_files();
    let all = exif_core::StripSpec::all();
    let policies = [
        None,
        Some(StripPolicy::Gps),
        Some(StripPolicy::GpsAndDevice),
        Some(StripPolicy::AllPii),
        Some(StripPolicy::All),
        Some(StripPolicy::Everything),
        Some(StripPolicy::Custom(all.exclude(Category::Icc))),
    ];
    let reasons = [
        FailReason::Unreadable,
        FailReason::TooLarge,
        FailReason::NotRegularFile,
        FailReason::Other,
    ];
    let mut cases = 0usize;
    for (n, (name, data)) in seeds.iter().enumerate() {
        let label = LABELS[n % LABELS.len()];
        let detail = &data[..data.len().min(64)];
        let (_, prev) = &seeds[(n + seeds.len() - 1) % seeds.len()];
        // Five entries: the seed, a failure, the seed again, a failure with
        // another reason, and the previous seed.
        let five = vec![
            FileEntry {
                label,
                result: FileResult::Audited(audit(data)),
            },
            FileEntry {
                label: LABELS[(n + 1) % LABELS.len()],
                result: FileResult::Failed {
                    reason: reasons[n % 4],
                    detail,
                },
            },
            FileEntry {
                label: LABELS[(n + 2) % LABELS.len()],
                result: FileResult::Audited(audit(data)),
            },
            FileEntry {
                label,
                result: FileResult::Failed {
                    reason: reasons[(n + 1) % 4],
                    detail: b"",
                },
            },
            FileEntry {
                label: LABELS[(n + 3) % LABELS.len()],
                result: FileResult::Audited(audit(prev)),
            },
        ];
        for policy in policies {
            for (inventory, values) in [(false, false), (true, false), (false, true), (true, true)]
            {
                let opts = RenderOptions {
                    policy,
                    inventory,
                    values,
                };
                for count in [0usize, 1, 2, 5] {
                    let files = &five[..count];
                    for (format, oneshot) in [
                        (Format::Text, render_text(files, &opts)),
                        (Format::Json, render_json(files, &opts)),
                        (Format::Csv, render_csv(files, &opts)),
                    ] {
                        let mut b = ReportBuilder::new(format, opts);
                        for f in files {
                            b.add(f);
                        }
                        assert_eq!(b.len(), count);
                        assert_eq!(
                            b.summary(),
                            exif_core::report::summarize(files, &opts),
                            "{name} {format:?} {policy:?} n={count}"
                        );
                        let got = b.finish();
                        assert_eq!(
                            got, oneshot,
                            "{name} {format:?} policy {policy:?} inventory {inventory} values {values} n={count}"
                        );
                        assert_eq!(got, render(format, files, &opts));
                        cases += 1;
                    }
                }
            }
        }
        // The builder's documents are well-formed (checked once per seed at
        // the richest option set).
        let rich = RenderOptions {
            policy: Some(StripPolicy::AllPii),
            inventory: true,
            values: true,
        };
        for format in [Format::Json, Format::Csv] {
            let mut b = ReportBuilder::new(format, rich);
            for f in &five {
                b.add(f);
            }
            let out = b.finish();
            match format {
                Format::Json => {
                    parse_json(out.as_bytes()).unwrap_or_else(|e| panic!("{name}: {e}"));
                }
                _ => {
                    parse_csv(out.as_bytes()).unwrap_or_else(|e| panic!("{name}: {e}"));
                }
            }
        }
    }
    println!("{cases} builder cases over {} seed files", seeds.len());
}

// ── carried F2: a truncated audit previews conservatively end to end ────────

/// A little-endian TIFF: IFD0 with `n0` unremarkable BYTE entries chained to a
/// second IFD holding `second` `(tag, type, count, inline value)` entries.
fn chained(n0: usize, second: &[(u16, u16, u32, u32)]) -> Vec<u8> {
    let ifd1_at = 8 + 2 + 12 * n0 + 4;
    let mut t = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    t.extend_from_slice(&u16::try_from(n0).expect("u16").to_le_bytes());
    for i in 0..n0 {
        t.extend_from_slice(&(0xF000 + (i % 0x0FFF) as u16).to_le_bytes());
        t.extend_from_slice(&1u16.to_le_bytes());
        t.extend_from_slice(&1u32.to_le_bytes());
        t.extend_from_slice(&[0, 0, 0, 0]);
    }
    t.extend_from_slice(&u32::try_from(ifd1_at).expect("u32").to_le_bytes());
    t.extend(report_inputs::le_ifd(second, 0));
    t
}

#[test]
fn a_truncated_audit_previews_conservatively_end_to_end() {
    use exif_core::audit::{preview, Severity};
    // The `over` input of the cap-boundary test: 65,535 + 3 entries; Make
    // (CameraModel, low) and BodySerialNumber (DeviceId, high) fall past the
    // cap, so their findings are kept without an item index.
    const A: u32 = 0x0000_0061;
    let bytes = chained(
        MAX_INVENTORY_ITEMS - 1,
        &[(0xF001, 3, 1, 0), (0x010F, 2, 2, A), (0xA431, 2, 2, A)],
    );
    let a = audit(&bytes);
    assert!(a.health().inventory_truncated && !a.health().clean);
    assert_eq!(a.items().len(), MAX_INVENTORY_ITEMS);
    assert_eq!(a.findings().len(), 2);
    assert!(a.findings().iter().all(|f| f.item.is_none()));
    assert_eq!(a.max_severity(), Some(Severity::High));

    let all = exif_core::StripSpec::all();
    let policies = [
        StripPolicy::Gps,
        StripPolicy::GpsAndDevice,
        StripPolicy::AllPii,
        StripPolicy::All,
        StripPolicy::Everything,
        StripPolicy::Custom(all),
        StripPolicy::Custom(all.exclude(Category::Icc)),
        StripPolicy::Custom(exif_core::StripSpec::none()),
    ];
    for p in policies {
        let pv = preview(&a, p);
        assert!(
            !pv.reliable(),
            "{}: a truncated audit is never reliable",
            p.name()
        );
        let counts = pv.counts();
        assert_eq!(
            counts.removed + counts.kept + counts.kept_unread,
            a.findings().len(),
            "{}: the counts are over the stored findings",
            p.name()
        );
        let removes_everything = p.spec() == all;
        if removes_everything {
            assert_eq!(pv.residual_severity(), None, "{}", p.name());
        } else {
            // The dropped findings' categories are unknown: nothing is assumed
            // removed, so the residual is the audit's own maximum.
            assert_eq!(pv.residual_severity(), a.max_severity(), "{}", p.name());
        }
    }

    // The harness's preview guards agree with the library on this audit: they
    // re-derive the truncated branch (including the `all()` exception) from the
    // flattened audit, and this is an audit that really takes that branch with
    // stored findings (carried F2: the old mutants 15, 16 and vM).
    let facts = audit_invariants::facts(&a);
    let pin = audit_invariants::preview_input(&bytes, &a, &facts);
    assert!(pin.truncated && !pin.clean && pin.findings.len() == 2);
    let bad = audit_invariants::preview_violations(&pin);
    assert!(bad.is_empty(), "{bad:?}");
    let everything = &pin.cases[4];
    assert_eq!(everything.label, "everything");
    assert_eq!(everything.got.residual_sev, None);
    assert_eq!(everything.want.residual_sev, None);
    assert_eq!(pin.cases[0].want.residual_sev.as_deref(), Some("High"));

    // End to end: the three renderers say the same thing.
    let files = vec![FileEntry {
        label: b"huge.tif",
        result: FileResult::Audited(a),
    }];
    let opts = RenderOptions {
        policy: Some(StripPolicy::GpsAndDevice),
        inventory: true,
        values: false,
    };
    let json = parse_json(render_json(&files, &opts).as_bytes()).expect("json");
    let f = &json.get("files").and_then(J::as_arr).expect("files")[0];
    assert_eq!(f.get("status").and_then(J::as_str), Some("issues"));
    let p = f.get("preview").expect("preview");
    assert_eq!(p.get("reliable").and_then(J::as_bool), Some(false));
    assert!(p.get("residual_findings").expect("rf").is_null());
    assert!(p.get("residual_severity").expect("rs").is_null());
    assert_eq!(
        p.get("counts")
            .and_then(|c| c.get("removed"))
            .and_then(J::as_uint),
        Some(1),
        "DeviceId is removed by gps_and_device, CameraModel is kept"
    );
    let health = f.get("health").expect("health");
    assert_eq!(
        health.get("inventory_truncated").and_then(J::as_bool),
        Some(true)
    );
    let text = render_text(&files, &opts);
    assert!(text.contains("preview not reliable: the file was not read cleanly"));
    let file_block = text.split("summary\n").next().expect("file block");
    assert!(!file_block.contains("residual"), "{file_block}");
    // The CSV lists every item and then the two findings that have no item.
    let rows = parse_csv(render_csv(&files, &opts).as_bytes()).expect("csv");
    assert_eq!(rows.len(), 1 + MAX_INVENTORY_ITEMS + 2);
    let tail: Vec<&str> = rows[rows.len() - 2..]
        .iter()
        .map(|r| r[3].as_str())
        .collect();
    assert_eq!(tail, ["camera_model", "device_id"]);
    // The facts guards agree.
    let c = build_case(
        &bytes,
        b"huge.tif",
        b"x",
        CaseOpts {
            policy: Some(1),
            custom_byte: 0,
            inventory: true,
            values: false,
            reason: 0,
        },
    );
    assert!(render_invariants::violations(&c).is_empty());
}

// ── the largest output at the caps ──────────────────────────────────────────

/// 65,536 findings (items and findings both at the cap), every one a Make
/// whose payload is 70 control bytes shared by all entries (the entries
/// overlap the one payload at the end of the file), which `values` escapes to
/// the longest excerpt: 64 bytes of `\x01`.
fn worst_case_input() -> Vec<u8> {
    let n0 = MAX_INVENTORY_ITEMS - 1;
    let payload_len = 70usize;
    let ifd1_len = 2 + 12 + 4;
    let payload_at = 8 + 2 + 12 * n0 + 4 + ifd1_len;
    let at = u32::try_from(payload_at).expect("u32");
    let entry = |t: &mut Vec<u8>| {
        t.extend_from_slice(&0x010Fu16.to_le_bytes());
        t.extend_from_slice(&2u16.to_le_bytes());
        t.extend_from_slice(&u32::try_from(payload_len).expect("u32").to_le_bytes());
        t.extend_from_slice(&at.to_le_bytes());
    };
    let mut t = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    t.extend_from_slice(&u16::try_from(n0).expect("u16").to_le_bytes());
    for _ in 0..n0 {
        entry(&mut t);
    }
    t.extend_from_slice(
        &u32::try_from(8 + 2 + 12 * n0 + 4)
            .expect("u32")
            .to_le_bytes(),
    );
    t.extend_from_slice(&1u16.to_le_bytes());
    entry(&mut t);
    t.extend_from_slice(&0u32.to_le_bytes());
    t.extend_from_slice(&[0x01; 70]);
    t
}

fn measure() -> (usize, usize, usize, usize, usize, usize) {
    let bytes = worst_case_input();
    let a = audit(&bytes);
    assert_eq!(a.items().len(), MAX_INVENTORY_ITEMS);
    assert_eq!(a.findings().len(), MAX_INVENTORY_ITEMS);
    assert!(a.health().clean, "{:?}", a.health());
    let files = vec![FileEntry {
        label: b"huge.tif",
        result: FileResult::Audited(a),
    }];
    let opts = RenderOptions {
        policy: Some(StripPolicy::AllPii),
        inventory: true,
        values: true,
    };
    let (t, j, c) = (
        render_text(&files, &opts),
        render_json(&files, &opts),
        render_csv(&files, &opts),
    );
    let no_inv = RenderOptions {
        inventory: false,
        ..opts
    };
    let (t2, j2, c2) = (
        render_text(&files, &no_inv),
        render_json(&files, &no_inv),
        render_csv(&files, &no_inv),
    );
    // Still well-formed at the caps.
    parse_json(j.as_bytes()).expect("json at the caps");
    parse_csv(c.as_bytes()).expect("csv at the caps");
    (t.len(), j.len(), c.len(), t2.len(), j2.len(), c2.len())
}

/// The ceiling per item and per finding (both at the cap) with inventory and
/// values on: the measured sizes are in the handback; this bound has headroom.
const CEILING_PER_ENTRY: usize = 1_400;

#[test]
fn largest_output_at_the_caps_stays_under_the_ceiling() {
    let (t, j, c, t2, j2, c2) = measure();
    let n = MAX_INVENTORY_ITEMS;
    for (name, len) in [("text", t), ("json", j), ("csv", c)] {
        assert!(
            len <= CEILING_PER_ENTRY * n,
            "{name}: {len} bytes at the caps, ceiling {} bytes",
            CEILING_PER_ENTRY * n
        );
    }
    for (name, len) in [("text", t2), ("json", j2), ("csv", c2)] {
        assert!(
            len <= CEILING_PER_ENTRY * n,
            "{name} without inventory: {len}"
        );
    }
}

#[test]
#[ignore = "prints the sizes; run with --ignored --nocapture"]
fn largest_output_at_the_caps_is_measured() {
    let (t, j, c, t2, j2, c2) = measure();
    println!("inventory+values: text {t} json {j} csv {c} bytes");
    println!("values only:      text {t2} json {j2} csv {c2} bytes");
}
