//! Unit tests of the three renderers (SPEC-015). They read their output back
//! with the independent strict parsers of `fuzz/shared/render_check.rs`, which
//! call nothing in this module.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use super::*;
use crate::audit::{audit, RegionKind, Sniffed};
use crate::policy::StripSpec;

#[path = "../../tests/support/report_inputs.rs"]
mod inputs;

include!("../../fuzz/shared/render_check.rs");
use render_check::{parse_csv, parse_json, J};

const PNG: [u8; 10] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];

/// The full-house JPEG, an unsupported PNG and a failed file.
fn three<'a>(jpeg: &'a [u8], label: &'a [u8]) -> Vec<FileEntry<'a>> {
    vec![
        FileEntry {
            label,
            result: FileResult::Audited(audit(jpeg)),
        },
        FileEntry {
            label: b"image.png",
            result: FileResult::Audited(audit(&PNG)),
        },
        FileEntry {
            label: b"bad.jpg",
            result: FileResult::Failed {
                reason: FailReason::Unreadable,
                detail: b"permission denied",
            },
        },
    ]
}

fn opts(policy: Option<StripPolicy>, inventory: bool, values: bool) -> RenderOptions {
    RenderOptions {
        policy,
        inventory,
        values,
    }
}

fn json(files: &[FileEntry<'_>], o: &RenderOptions) -> J {
    let text = render_json(files, o);
    parse_json(text.as_bytes()).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

fn csv(files: &[FileEntry<'_>], o: &RenderOptions) -> Vec<Vec<String>> {
    let text = render_csv(files, o);
    parse_csv(text.as_bytes()).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

#[test]
fn values_are_absent_unless_asked() {
    let tiff = inputs::secret_tiff();
    let a = audit(&tiff);
    assert_eq!(a.findings().len(), 3, "Make, Artist and BodySerialNumber");
    let files = vec![FileEntry {
        label: b"secret.tif",
        result: FileResult::Audited(a),
    }];
    let secret = std::str::from_utf8(inputs::SECRET).unwrap();
    for inventory in [false, true] {
        for policy in [None, Some(StripPolicy::AllPii)] {
            let off = opts(policy, inventory, false);
            for (name, out) in [
                ("text", render_text(&files, &off)),
                ("json", render_json(&files, &off)),
                ("csv", render_csv(&files, &off)),
            ] {
                assert!(!out.contains(secret), "{name} leaks the payload: {out}");
                assert!(!out.contains("SECRET"), "{name} leaks part of it");
            }
            // The JSON `value` of every finding and item is null, the CSV
            // value columns are empty.
            let j = json(&files, &off);
            for f in j.get("files").unwrap().as_arr().unwrap() {
                for finding in f.get("findings").unwrap().as_arr().unwrap() {
                    assert!(finding.get("value").unwrap().is_null());
                }
                if let Some(inv) = f.get("inventory").and_then(J::as_arr) {
                    assert!(inv.iter().all(|i| i.get("value").unwrap().is_null()));
                }
            }
            for row in csv(&files, &off).iter().skip(1) {
                assert_eq!((row[17].as_str(), row[18].as_str()), ("", ""));
            }
            // And with values on, all three carry it.
            let on = opts(policy, inventory, true);
            for (name, out) in [
                ("text", render_text(&files, &on)),
                ("json", render_json(&files, &on)),
                ("csv", render_csv(&files, &on)),
            ] {
                assert!(out.contains(secret), "{name} lacks the excerpt: {out}");
            }
        }
    }
    // Escaped: a payload with a quote, a backslash and a control byte.
    let hostile = inputs::tiff_with_payloads(&[(0x010F, 2, b"a\"b\\c\x01\0")]);
    let files = vec![FileEntry {
        label: b"x",
        result: FileResult::Audited(audit(&hostile)),
    }];
    let on = opts(None, false, true);
    assert!(render_text(&files, &on).contains("value=\"a\\\"b\\\\c\\x01\""));
    let j = json(&files, &on);
    let f = &j.get("files").unwrap().as_arr().unwrap()[0];
    let v = f.get("findings").unwrap().as_arr().unwrap()[0]
        .get("value")
        .unwrap()
        .get("text")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(v, "a\\\"b\\\\c\\x01");
    assert_eq!(csv(&files, &on)[1][17], "a\\\"b\\\\c\\x01");
}

// ── JSON ────────────────────────────────────────────────────────────────────

fn keys_are(j: &J, want: &[&str], what: &str) {
    assert_eq!(j.keys(), want, "{what}");
}

#[test]
fn json_has_the_pinned_key_order_at_every_level() {
    let jpeg = inputs::full_house_jpeg();
    let files = three(&jpeg, b"photos/full-house.jpg");
    let j = json(&files, &opts(Some(StripPolicy::AllPii), true, true));
    keys_are(
        &j,
        &["schema_version", "policy", "options", "summary", "files"],
        "top",
    );
    assert_eq!(j.get("schema_version").unwrap().as_uint(), Some(1));
    assert_eq!(j.get("policy").unwrap().as_str(), Some("all_pii"));
    keys_are(
        j.get("options").unwrap(),
        &["inventory", "values"],
        "options",
    );
    let s = j.get("summary").unwrap();
    keys_are(
        s,
        &[
            "files",
            "failed",
            "unsupported",
            "unreadable",
            "issues",
            "ok",
            "findings",
            "by_category",
            "max_severity",
            "residual_findings",
            "reliable_files",
            "unreliable_files",
        ],
        "summary",
    );
    keys_are(
        s.get("by_category").unwrap(),
        &[
            "gps",
            "device_id",
            "authorship",
            "free_text",
            "thumbnail",
            "maker_note",
            "xmp",
            "iptc",
            "jumbf",
            "timestamps",
            "camera_model",
            "software",
            "icc",
        ],
        "by_category",
    );
    let file_keys = [
        "label",
        "status",
        "container",
        "failure",
        "health",
        "max_severity",
        "inventory_items",
        "findings",
        "inventory",
        "preview",
    ];
    let list = j.get("files").unwrap().as_arr().unwrap();
    assert_eq!(list.len(), 3);
    for f in list {
        keys_are(f, &file_keys, "file");
    }
    // The audited full-house file.
    let f = &list[0];
    keys_are(
        f.get("health").unwrap(),
        &[
            "clean",
            "jpeg_issues",
            "exif_issues",
            "failed_blocks",
            "inventory_truncated",
        ],
        "health",
    );
    keys_are(
        f.get("preview").unwrap(),
        &[
            "policy",
            "counts",
            "reliable",
            "residual_findings",
            "residual_severity",
        ],
        "preview",
    );
    keys_are(
        f.get("preview").unwrap().get("counts").unwrap(),
        &["removed", "kept", "kept_unread"],
        "counts",
    );
    let finding_keys = [
        "category",
        "severity",
        "at",
        "block",
        "segment",
        "after_scan",
        "disposition",
        "thumbnail",
        "value",
    ];
    let findings = f.get("findings").unwrap().as_arr().unwrap();
    assert_eq!(findings.len(), 15);
    let mut saw = (false, false, false);
    for fi in findings {
        keys_are(fi, &finding_keys, "finding");
        let at = fi.get("at").unwrap();
        match at.get("kind").unwrap().as_str().unwrap() {
            "tag" => {
                keys_are(
                    at,
                    &["kind", "ifd", "ifd_index", "tag", "field_type", "count"],
                    "tag location",
                );
                saw.0 = true;
            }
            "region" => {
                keys_are(at, &["kind", "region", "segment", "len"], "region location");
                saw.1 = true;
            }
            other => panic!("kind {other}"),
        }
        if let Some(t) = Some(fi.get("thumbnail").unwrap()).filter(|t| !t.is_null()) {
            keys_are(t, &["offset", "length", "starts_with_ffd8"], "thumbnail");
            saw.2 = true;
        }
        let v = fi.get("value").unwrap();
        if !v.is_null() {
            keys_are(v, &["text", "truncated"], "value");
        }
    }
    assert_eq!(saw, (true, true, true));
    let inv = f.get("inventory").unwrap().as_arr().unwrap();
    assert!(inv.len() >= 15);
    for item in inv {
        keys_are(
            item,
            &["at", "block", "segment", "after_scan", "category", "value"],
            "inventory item",
        );
    }
    // The unsupported and the failed file.
    let png = &list[1];
    assert!(png.get("failure").unwrap().is_null());
    let failed = &list[2];
    keys_are(
        failed.get("failure").unwrap(),
        &["reason", "detail"],
        "failure",
    );
    assert!(failed.get("health").unwrap().is_null());
    assert!(failed.get("container").unwrap().is_null());
    assert_eq!(failed.get("status").unwrap().as_str(), Some("failed"));
}

#[test]
fn json_enumerations_are_pinned() {
    // Status, failure reasons, containers, IFD and region names, dispositions
    // and severities are API.
    let names: Vec<&str> = [
        Status::Failed,
        Status::Unsupported,
        Status::Unreadable,
        Status::Issues,
        Status::Ok,
    ]
    .iter()
    .map(|s| s.name())
    .collect();
    assert_eq!(
        names,
        ["failed", "unsupported", "unreadable", "issues", "ok"]
    );
    let reasons: Vec<&str> = [
        FailReason::Unreadable,
        FailReason::TooLarge,
        FailReason::NotRegularFile,
        FailReason::Other,
    ]
    .iter()
    .map(|r| r.name())
    .collect();
    assert_eq!(
        reasons,
        ["unreadable", "too_large", "not_regular_file", "other"]
    );
    let containers: Vec<&str> = [
        ContainerKind::Tiff,
        ContainerKind::Jpeg,
        ContainerKind::Unsupported(Sniffed::Png),
        ContainerKind::Unsupported(Sniffed::Webp),
        ContainerKind::Unsupported(Sniffed::Heic),
        ContainerKind::Unsupported(Sniffed::Gif),
        ContainerKind::Unsupported(Sniffed::BigTiff),
        ContainerKind::Unsupported(Sniffed::Unknown),
        ContainerKind::Unsupported(Sniffed::Empty),
    ]
    .into_iter()
    .map(container_name)
    .collect();
    assert_eq!(
        containers,
        ["tiff", "jpeg", "png", "webp", "heic", "gif", "bigtiff", "unknown", "empty"]
    );
    let ifds: Vec<&str> = [
        IfdKind::Ifd0,
        IfdKind::Ifd1,
        IfdKind::Chain,
        IfdKind::Sub,
        IfdKind::Exif,
        IfdKind::Gps,
        IfdKind::Interop,
    ]
    .into_iter()
    .map(ifd_name)
    .collect();
    assert_eq!(
        ifds,
        ["ifd0", "ifd1", "chain", "sub", "exif", "gps", "interop"]
    );
    let regions: Vec<&str> = [
        RegionKind::Xmp,
        RegionKind::ExtendedXmp,
        RegionKind::Iptc,
        RegionKind::Icc,
        RegionKind::Jumbf,
    ]
    .into_iter()
    .map(region_name)
    .collect();
    assert_eq!(regions, ["xmp", "extended_xmp", "iptc", "icc", "jumbf"]);
    // On a real report: every statistic uses only the allowed values.
    let jpeg = inputs::full_house_jpeg();
    let files = three(&jpeg, b"f.jpg");
    let j = json(&files, &opts(Some(StripPolicy::GpsAndDevice), true, false));
    let allowed_disp = ["removed", "kept", "kept_unread"];
    let mut seen_disp = std::collections::BTreeSet::new();
    for f in j.get("files").unwrap().as_arr().unwrap() {
        for fi in f.get("findings").unwrap().as_arr().unwrap() {
            let d = fi.get("disposition").unwrap().as_str().unwrap();
            assert!(allowed_disp.contains(&d), "{d}");
            seen_disp.insert(d.to_string());
            assert!(
                ["low", "medium", "high"].contains(&fi.get("severity").unwrap().as_str().unwrap())
            );
        }
    }
    assert_eq!(
        seen_disp.len(),
        3,
        "all three dispositions occur: {seen_disp:?}"
    );
    // Statuses on the three files.
    let st: Vec<&str> = j
        .get("files")
        .unwrap()
        .as_arr()
        .unwrap()
        .iter()
        .map(|f| f.get("status").unwrap().as_str().unwrap())
        .collect();
    assert_eq!(st, ["ok", "unsupported", "failed"]);
    // No policy: disposition is null, preview is null.
    let j = json(&files, &RenderOptions::default());
    assert!(j.get("policy").unwrap().is_null());
    let f = &j.get("files").unwrap().as_arr().unwrap()[0];
    assert!(f.get("preview").unwrap().is_null());
    assert!(f.get("inventory").unwrap().is_null());
    for fi in f.get("findings").unwrap().as_arr().unwrap() {
        assert!(fi.get("disposition").unwrap().is_null());
    }
}

#[test]
fn json_escapes_quote_backslash_and_every_control() {
    // The JSON layer alone: every U+0000..U+001F, the quote and the backslash.
    for n in 0u32..0x20 {
        let c = char::from_u32(n).unwrap();
        let mut out = String::new();
        json::string(&mut out, &c.to_string());
        assert!(
            out.bytes().all(|b| b >= 0x20),
            "a raw control in {out:?} for {n:#x}"
        );
        let back = parse_json(out.as_bytes()).unwrap();
        assert_eq!(back.as_str(), Some(c.to_string().as_str()), "{n:#x}");
    }
    let mut out = String::new();
    json::string(&mut out, "a\"b\\c\u{1}\u{1f}\n");
    assert_eq!(out, "\"a\\\"b\\\\c\\u0001\\u001f\\n\"");
    // Through the whole pipeline: a label with every hostile class parses back
    // to exactly the display-safe text.
    let label: Vec<u8> = (0u8..=255).collect();
    let files = vec![FileEntry {
        label: &label,
        result: FileResult::Failed {
            reason: FailReason::Other,
            detail: &label,
        },
    }];
    let text = render_json(&files, &RenderOptions::default());
    assert!(
        text.bytes().all(|b| b >= 0x20 || b == b'\n'),
        "a raw control character in the JSON"
    );
    let j = parse_json(text.as_bytes()).unwrap();
    let f = &j.get("files").unwrap().as_arr().unwrap()[0];
    assert_eq!(
        f.get("label").unwrap().as_str(),
        Some(safe_text(&label).as_str())
    );
    assert_eq!(
        f.get("failure").unwrap().get("detail").unwrap().as_str(),
        Some(safe_text(&label).as_str())
    );
}

#[test]
fn json_preview_residuals_are_null_unless_reliable() {
    let jpeg = inputs::full_house_jpeg();
    let truncated_jpeg = [0xFF, 0xD8];
    let files = vec![
        FileEntry {
            label: b"clean.jpg",
            result: FileResult::Audited(audit(&jpeg)),
        },
        FileEntry {
            label: b"png",
            result: FileResult::Audited(audit(&PNG)),
        },
        FileEntry {
            label: b"cut.jpg",
            result: FileResult::Audited(audit(&truncated_jpeg)),
        },
        FileEntry {
            label: b"bad.tif",
            result: FileResult::Audited(audit(b"II\x2A\x00\x40\x00\x00\x00")),
        },
    ];
    let j = json(&files, &opts(Some(StripPolicy::Gps), false, false));
    let list = j.get("files").unwrap().as_arr().unwrap();
    let reliable: Vec<bool> = list
        .iter()
        .map(
            |f| match f.get("preview").unwrap().get("reliable").unwrap() {
                J::Bool(b) => *b,
                other => panic!("{other:?}"),
            },
        )
        .collect();
    assert_eq!(reliable, [true, false, false, false]);
    for (f, ok) in list.iter().zip(&reliable) {
        let p = f.get("preview").unwrap();
        let (n, s) = (
            p.get("residual_findings").unwrap(),
            p.get("residual_severity").unwrap(),
        );
        if *ok {
            assert!(n.as_uint().is_some(), "reliable: residual findings");
            assert!(s.as_str().is_some(), "reliable: residual severity");
        } else {
            assert!(n.is_null() && s.is_null(), "unreliable: both null");
        }
    }
    // A clean audit with nothing left reports 0 and a null severity: the
    // number is meaningful, the severity has nothing to name.
    let j = json(&files, &opts(Some(StripPolicy::Everything), false, false));
    let p = j.get("files").unwrap().as_arr().unwrap()[0]
        .get("preview")
        .unwrap()
        .clone();
    assert_eq!(p.get("residual_findings").unwrap().as_uint(), Some(0));
    assert!(p.get("residual_severity").unwrap().is_null());
}

#[test]
fn json_summary_matches_summarize() {
    let jpeg = inputs::full_house_jpeg();
    let files = three(&jpeg, b"f.jpg");
    for policy in [None, Some(StripPolicy::AllPii)] {
        let o = opts(policy, false, false);
        let s = summarize(&files, &o);
        let j = json(&files, &o);
        let js = j.get("summary").unwrap();
        let n = |k: &str| js.get(k).unwrap().as_uint().unwrap() as usize;
        assert_eq!(n("files"), s.files);
        assert_eq!(n("failed"), s.failed);
        assert_eq!(n("unsupported"), s.unsupported);
        assert_eq!(n("unreadable"), s.unreadable);
        assert_eq!(n("issues"), s.issues);
        assert_eq!(n("ok"), s.ok);
        assert_eq!(n("findings"), s.findings);
        assert_eq!(n("reliable_files"), s.reliable_files);
        assert_eq!(n("unreliable_files"), s.unreliable_files);
        for (c, count) in s.by_category {
            let got = js.get("by_category").unwrap().get(c.name()).unwrap();
            assert_eq!(got.as_uint().unwrap() as usize, count, "{c:?}");
        }
        assert_eq!(
            js.get("max_severity").unwrap().as_str(),
            s.max_severity.map(|m| m.name())
        );
        assert_eq!(
            js.get("residual_findings")
                .unwrap()
                .as_uint()
                .map(|v| v as usize),
            s.residual_findings
        );
        // The run counts the three files.
        assert_eq!((s.files, s.failed, s.unsupported, s.ok), (3, 1, 1, 1));
    }
}

// ── CSV ─────────────────────────────────────────────────────────────────────

#[test]
fn csv_has_the_pinned_header_and_19_fields_per_row() {
    let jpeg = inputs::full_house_jpeg();
    let files = three(&jpeg, b"f.jpg");
    let want: Vec<&str> = "label,status,container,category,severity,kind,ifd,ifd_index,tag,\
field_type,count,region,region_len,block,segment,after_scan,disposition,value,value_truncated"
        .split(',')
        .collect();
    assert_eq!(want.len(), 19);
    for o in [
        RenderOptions::default(),
        opts(Some(StripPolicy::All), true, true),
    ] {
        let rows = csv(&files, &o);
        assert_eq!(rows[0], want);
        for r in &rows {
            assert_eq!(r.len(), 19);
        }
        assert!(rows.len() > 3);
    }
    assert!(render_csv(&files, &RenderOptions::default()).starts_with(
        "label,status,container,category,severity,kind,ifd,ifd_index,tag,field_type,count,\
region,region_len,block,segment,after_scan,disposition,value,value_truncated\r\n"
    ));
}

#[test]
fn csv_quotes_comma_quote_cr_lf_and_doubles_quotes() {
    let field = |t: &str| {
        let mut o = String::new();
        csv::push_field(&mut o, t);
        o
    };
    assert_eq!(field("plain"), "plain");
    assert_eq!(field(""), "");
    assert_eq!(field("a,b"), "\"a,b\"");
    assert_eq!(field("say \"hi\""), "\"say \"\"hi\"\"\"");
    assert_eq!(field("a\rb"), "\"a\rb\"");
    assert_eq!(field("a\nb"), "\"a\nb\"");
    assert_eq!(field("a\r\nb"), "\"a\r\nb\"");
    assert_eq!(field("\""), "\"\"\"\"");
    // And the layer alone round-trips through the strict parser.
    for t in ["a,b", "q\"q", "cr\rcr", "lf\nlf", "", "plain", "\"\"", ",,"] {
        let mut o = String::new();
        csv::push_field(&mut o, t);
        o.push_str("\r\n");
        let rows = parse_csv(o.as_bytes()).unwrap();
        assert_eq!(rows, vec![vec![t.to_string()]], "{t:?}");
    }
    // A label with a comma reaches the output quoted.
    let files = vec![FileEntry {
        label: b"a,b.jpg",
        result: FileResult::Audited(audit(&PNG)),
    }];
    let text = render_csv(&files, &RenderOptions::default());
    assert!(text.contains("\"a,b.jpg\",unsupported,png"), "{text}");
}

#[test]
fn csv_uses_crlf() {
    let jpeg = inputs::full_house_jpeg();
    let files = three(&jpeg, b"f.jpg");
    for o in [
        RenderOptions::default(),
        opts(Some(StripPolicy::AllPii), true, true),
    ] {
        let text = render_csv(&files, &o);
        assert!(text.ends_with("\r\n"));
        let bytes = text.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            if *b == b'\n' {
                assert!(i > 0 && bytes[i - 1] == b'\r', "a bare LF at {i}");
            }
        }
        assert!(!text.contains("\r\r"));
    }
    // A hostile label cannot add a line end: CR and LF are escaped before the
    // CSV layer sees them.
    let files = vec![FileEntry {
        label: b"a\r\nb",
        result: FileResult::Failed {
            reason: FailReason::Other,
            detail: b"x\ny",
        },
    }];
    let text = render_csv(&files, &RenderOptions::default());
    assert_eq!(text.matches("\r\n").count(), 2, "{text:?}");
}

#[test]
fn csv_formula_guard_prefixes_each_trigger_character_on_hostile_fields_only() {
    // Labels that start with a trigger character, through the real pipeline.
    for (label, want) in [
        ("=cmd|' /C calc'!A0", "'=cmd|' /C calc'!A0"),
        ("+1", "'+1"),
        ("-1", "'-1"),
        ("@SUM(1)", "'@SUM(1)"),
        ("plain", "plain"),
        ("a=b", "a=b"),
        ("", ""),
        // TAB and CR are escaped by `safe_text` first, so what reaches the CSV
        // layer starts with a backslash and needs no guard.
        ("\tx", "\\tx"),
        ("\rx", "\\rx"),
    ] {
        let files = vec![FileEntry {
            label: label.as_bytes(),
            result: FileResult::Audited(audit(&PNG)),
        }];
        let rows = csv(&files, &RenderOptions::default());
        assert_eq!(rows[1][0], want, "{label:?}");
    }
    // A failure detail is guarded too (it is carried in `value`).
    let files = vec![FileEntry {
        label: b"x",
        result: FileResult::Failed {
            reason: FailReason::Other,
            detail: b"=HYPERLINK(1)",
        },
    }];
    assert_eq!(
        csv(&files, &RenderOptions::default())[1][17],
        "'=HYPERLINK(1)"
    );
    // An excerpt is guarded.
    let t = inputs::tiff_with_payloads(&[(0x010F, 2, b"-1+1\0")]);
    let files = vec![FileEntry {
        label: b"x",
        result: FileResult::Audited(audit(&t)),
    }];
    assert_eq!(csv(&files, &opts(None, false, true))[1][17], "'-1+1");
    // The layer alone, on a row of raw characters: each trigger is guarded in
    // the label and value columns, and in no other column.
    for trigger in ['=', '+', '-', '@', '\t', '\r'] {
        assert!(csv::starts_like_formula(&format!("{trigger}x")));
        let mut row = csv::Row::default();
        for cell in &mut row {
            *cell = format!("{trigger}x");
        }
        let mut out = String::new();
        csv::push_row(&mut out, &row);
        let fields = parse_csv(out.as_bytes()).unwrap();
        for (i, cell) in fields[0].iter().enumerate() {
            let guarded = cell.starts_with('\'');
            let hostile = i == csv::LABEL || i == csv::VALUE;
            assert_eq!(guarded, hostile, "column {i} for {trigger:?}: {cell:?}");
        }
    }
    assert!(!csv::starts_like_formula("x="));
    assert!(!csv::starts_like_formula("'="));
    assert!(!csv::starts_like_formula(""));
}

#[test]
fn csv_has_a_row_for_every_file_without_findings() {
    let clean_empty = inputs::jpeg(&[]);
    let files = vec![
        FileEntry {
            label: b"png",
            result: FileResult::Audited(audit(&PNG)),
        },
        FileEntry {
            label: b"clean-empty.jpg",
            result: FileResult::Audited(audit(&clean_empty)),
        },
        FileEntry {
            label: b"gone",
            result: FileResult::Failed {
                reason: FailReason::TooLarge,
                detail: b"too big",
            },
        },
        FileEntry {
            label: b"bad.tif",
            result: FileResult::Audited(audit(b"II\x2A\x00\x40\x00\x00\x00")),
        },
    ];
    for o in [
        RenderOptions::default(),
        opts(None, true, true),
        opts(Some(StripPolicy::Everything), true, false),
    ] {
        let rows = csv(&files, &o);
        assert_eq!(rows.len(), 1 + 4, "one row per file, none is absent");
        let by_label = |l: &str| rows.iter().find(|r| r[0] == l).unwrap().clone();
        let png = by_label("png");
        assert_eq!((png[1].as_str(), png[2].as_str()), ("unsupported", "png"));
        let clean = by_label("clean-empty.jpg");
        assert_eq!((clean[1].as_str(), clean[2].as_str()), ("ok", "jpeg"));
        let failed = by_label("gone");
        assert_eq!(failed[1], "failed");
        assert_eq!(failed[2], "");
        assert_eq!(failed[17], "too big", "the failure detail is in value");
        let bad = by_label("bad.tif");
        assert_eq!((bad[1].as_str(), bad[2].as_str()), ("unreadable", "tiff"));
        // The rest is empty.
        for r in [&png, &clean, &failed, &bad] {
            for (i, cell) in r.iter().enumerate().skip(3) {
                if i != 17 {
                    assert_eq!(cell, "", "column {i} of {:?}", r[0]);
                }
            }
        }
    }
}

#[test]
fn csv_inventory_mode_rows() {
    let jpeg = inputs::full_house_jpeg();
    let a = audit(&jpeg);
    let items = a.items().len();
    let findings = a.findings().len();
    assert!(items > findings, "the fixture has inventory-only items");
    let files = vec![FileEntry {
        label: b"f.jpg",
        result: FileResult::Audited(a),
    }];
    let plain = csv(&files, &RenderOptions::default());
    assert_eq!(plain.len(), 1 + findings);
    let inv = csv(&files, &opts(Some(StripPolicy::AllPii), true, false));
    assert_eq!(inv.len(), 1 + items, "one row per item, none dropped");
    let empty_cat = inv.iter().skip(1).filter(|r| r[3].is_empty()).count();
    assert_eq!(empty_cat, items - findings, "items without a finding");
    for r in inv.iter().skip(1) {
        if r[3].is_empty() {
            assert!(
                r[4].is_empty() && r[16].is_empty(),
                "no severity or disposition"
            );
        } else {
            assert!(!r[4].is_empty() && !r[16].is_empty());
        }
        assert!(r[5] == "tag" || r[5] == "region");
        assert!(!r[15].is_empty(), "after_scan is always filled");
    }
    // The pointer tags (0x8769, 0x8825) are listed with no category.
    assert!(inv.iter().any(|r| r[8] == "0x8769" && r[3].is_empty()));
    // A region row names its region and length.
    let xmp = inv.iter().find(|r| r[11] == "xmp").unwrap();
    assert_eq!(
        (xmp[5].as_str(), xmp[3].as_str(), xmp[12].as_str()),
        ("region", "xmp", "4")
    );
}

// ── text ────────────────────────────────────────────────────────────────────

fn forbidden_char(c: char) -> bool {
    (c < ' ' && c != '\n')
        || c == '\u{7f}'
        || ('\u{80}'..='\u{9f}').contains(&c)
        || matches!(
            c,
            '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{2028}' | '\u{2029}'
        )
        || ('\u{202A}'..='\u{202E}').contains(&c)
        || ('\u{2066}'..='\u{2069}').contains(&c)
}

#[test]
fn text_has_no_forbidden_character_or_phrase() {
    let jpeg = inputs::full_house_jpeg();
    let hostile: Vec<u8> = {
        let mut v = b"a\x1b[31mred\x07\xE2\x80\xAEgpj.exe\xC2\x85\xE2\x80\xA8\xFF\"\\".to_vec();
        v.extend((0u8..32).collect::<Vec<_>>());
        v
    };
    let files = three(&jpeg, &hostile);
    let many = [
        RenderOptions::default(),
        opts(Some(StripPolicy::All), false, true),
        opts(Some(StripPolicy::Gps), true, true),
        opts(None, true, false),
    ];
    for o in many {
        let text = render_text(&files, &o);
        for c in text.chars() {
            assert!(!forbidden_char(c), "{c:?} in the text report");
        }
        let lower = text.to_lowercase();
        for phrase in ["no personal data", "safe", "clean file"] {
            assert!(!lower.contains(phrase), "{phrase:?} in the text report");
        }
        assert!(!text.contains('\r'));
        // The label is there, escaped.
        assert!(text.contains(&safe_text(&hostile)));
    }
}

#[test]
fn text_words_kept_and_kept_unread_as_dec_009_requires() {
    let jpeg = inputs::full_house_jpeg();
    let files = three(&jpeg, b"f.jpg");
    let text = render_text(&files, &opts(Some(StripPolicy::GpsAndDevice), false, false));
    assert!(text.contains(": removed"));
    assert!(text.contains(": kept (not targeted by this policy)"));
    assert!(text.contains(": kept_unread (may hold what this policy removes)"));
    assert!(!text.contains("contains none of it"));
    // Line by line: the ICC region is kept (nothing hides in it), the MakerNote
    // is kept_unread under GpsAndDevice, GPS is removed.
    let line = |needle: &str| {
        text.lines()
            .find(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no line with {needle}: {text}"))
            .to_string()
    };
    assert!(line("- icc ").ends_with("kept (not targeted by this policy)"));
    assert!(line("- maker_note ").ends_with("kept_unread (may hold what this policy removes)"));
    assert!(line("- gps ").ends_with(": removed"));
    // Without a policy there is no disposition wording at all.
    let plain = render_text(&files, &RenderOptions::default());
    assert!(!plain.contains("kept") && !plain.contains("removed"));
}

#[test]
fn text_prints_zero_findings_not_silence() {
    let clean_empty = inputs::jpeg(&[]);
    let files = vec![
        FileEntry {
            label: b"clean-empty.jpg",
            result: FileResult::Audited(audit(&clean_empty)),
        },
        FileEntry {
            label: b"image.png",
            result: FileResult::Audited(audit(&PNG)),
        },
        FileEntry {
            label: b"gone",
            result: FileResult::Failed {
                reason: FailReason::Unreadable,
                detail: b"denied",
            },
        },
    ];
    let text = render_text(&files, &RenderOptions::default());
    assert_eq!(text.matches("  0 findings\n").count(), 3, "{text}");
    assert!(text.contains("status: ok"));
    assert!(text.contains("status: unsupported"));
    assert!(text.contains("status: failed"));
    assert!(text.contains("failure: unreadable: denied"));
    assert!(text.contains("  findings: 0\n"));
}

#[test]
fn text_preview_not_reliable_line() {
    let jpeg = inputs::full_house_jpeg();
    let cut = [0xFF, 0xD8];
    let files = vec![
        FileEntry {
            label: b"good.jpg",
            result: FileResult::Audited(audit(&jpeg)),
        },
        FileEntry {
            label: b"image.png",
            result: FileResult::Audited(audit(&PNG)),
        },
        FileEntry {
            label: b"cut.jpg",
            result: FileResult::Audited(audit(&cut)),
        },
    ];
    let text = render_text(&files, &opts(Some(StripPolicy::AllPii), false, false));
    let line = "preview not reliable: the file was not read cleanly";
    assert_eq!(
        text.matches(line).count(),
        2,
        "png and the cut jpeg: {text}"
    );
    // The residual numbers appear once, for the reliable file.
    assert_eq!(text.matches("residual severity").count(), 1);
    let blocks: Vec<&str> = text.split("\n\n").collect();
    for b in blocks {
        if b.contains("image.png") || b.contains("cut.jpg") {
            assert!(!b.contains("residual"), "{b}");
        }
        if b.contains("good.jpg") {
            assert!(b.contains("residual findings"), "{b}");
        }
    }
    // No policy: no preview line at all.
    let plain = render_text(&files, &RenderOptions::default());
    assert!(!plain.contains("preview"));
}

// ── determinism ─────────────────────────────────────────────────────────────

#[test]
fn render_is_deterministic() {
    let jpeg = inputs::full_house_jpeg();
    let files = three(&jpeg, b"f.jpg");
    let o = opts(Some(StripPolicy::Custom(StripSpec::all())), true, true);
    for _ in 0..3 {
        assert_eq!(render_text(&files, &o), render_text(&files, &o));
        assert_eq!(render_json(&files, &o), render_json(&files, &o));
        assert_eq!(render_csv(&files, &o), render_csv(&files, &o));
    }
    // A second audit of the same bytes renders the same.
    let again = three(&jpeg, b"f.jpg");
    assert_eq!(render_json(&files, &o), render_json(&again, &o));
    assert_eq!(render_text(&files, &o), render_text(&again, &o));
    assert_eq!(render_csv(&files, &o), render_csv(&again, &o));
    // And a different option changes the output.
    assert_ne!(
        render_json(&files, &o),
        render_json(&files, &RenderOptions::default())
    );
}
