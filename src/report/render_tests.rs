//! Unit tests of the private layers of the renderers (SPEC-015): the report
//! name tables, the JSON string writer and the CSV field and row writers.
//!
//! The tests that read a whole report back with the independent strict parsers
//! are in `tests/report_formats.rs`: `just lint-red-proof` copies only
//! `Cargo.toml` and `src/` to a temporary crate, so nothing under `src/` may
//! read a file outside it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use super::*;
use crate::audit::{RegionKind, Sniffed};

#[test]
fn report_names_are_pinned() {
    // Field names and enumerated values are API (a change bumps
    // `schema_version`): these are the tables the renderers draw on.
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
    assert_eq!(tag_hex(0x010F), "0x010f");
    assert_eq!(tag_hex(0), "0x0000");
}

#[test]
fn json_escapes_quote_backslash_and_every_control() {
    // The JSON string writer alone: every U+0000..U+001F, the quote and the
    // backslash.
    for n in 0u32..0x20 {
        let c = char::from_u32(n).unwrap();
        let mut out = String::new();
        json::string(&mut out, &c.to_string());
        assert!(
            out.bytes().all(|b| b >= 0x20),
            "a raw control in {out:?} for {n:#x}"
        );
        assert!(out.starts_with('"') && out.ends_with('"') && out.len() >= 4);
    }
    let expect = |input: &str, want: &str| {
        let mut out = String::new();
        json::string(&mut out, input);
        assert_eq!(out, want, "{input:?}");
    };
    expect("plain", "\"plain\"");
    expect("", "\"\"");
    expect("a\"b", "\"a\\\"b\"");
    expect("a\\b", "\"a\\\\b\"");
    expect("\n\r\t", "\"\\n\\r\\t\"");
    expect("\u{8}\u{c}", "\"\\b\\f\"");
    expect("\u{0}", "\"\\u0000\"");
    expect("\u{1}\u{1f}", "\"\\u0001\\u001f\"");
    expect("\u{b}", "\"\\u000b\"");
    // Non-ASCII passes through (JSON is UTF-8).
    expect("caf\u{e9}\u{1F600}", "\"caf\u{e9}\u{1F600}\"");
    // DEL is not a JSON control; `safe_text` has already escaped it.
    expect("\u{7f}", "\"\u{7f}\"");
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
    assert_eq!(field("\"\""), "\"\"\"\"\"\"");
    assert_eq!(field(",,"), "\",,\"");
    assert_eq!(field("it's"), "it's");
    // The semicolon quotes too (a semicolon-locale spreadsheet splits on it).
    assert_eq!(field("a;b"), "\"a;b\"");
    assert_eq!(field(";"), "\";\"");
    assert_eq!(field("a;=1+cmd"), "\"a;=1+cmd\"");
    // A row ends with CRLF and has the 19 fields.
    let mut row = csv::Row::default();
    row[csv::LABEL] = "a,b".to_string();
    let mut out = String::new();
    csv::push_row(&mut out, &row);
    assert!(out.starts_with("\"a,b\","));
    assert!(out.ends_with("\r\n"));
    assert_eq!(out.matches(',').count(), 1 + 18);
}

#[test]
fn csv_formula_guard_prefixes_each_trigger_character_on_hostile_fields_only() {
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
        // Split the single row into its fields by hand: the cells hold no
        // comma or quote, but a CR cell is quoted and so holds a CR.
        let line = out.strip_suffix("\r\n").unwrap();
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 19, "{trigger:?}");
        for (i, cell) in fields.iter().enumerate() {
            let cell = cell.trim_matches('"');
            let guarded = cell.starts_with('\'');
            let hostile = i == csv::LABEL || i == csv::VALUE;
            assert_eq!(guarded, hostile, "column {i} for {trigger:?}: {cell:?}");
            if hostile {
                assert_eq!(cell, format!("'{trigger}x"));
            }
        }
    }
    // Not a trigger: later in the field, a quote, empty.
    assert!(!csv::starts_like_formula("x="));
    assert!(!csv::starts_like_formula("'="));
    assert!(!csv::starts_like_formula(""));
    assert!(!csv::starts_like_formula("a"));
    // The category column is never guarded: it is this library's own constant.
    let mut row = csv::Row::default();
    row[csv::CATEGORY] = "-gps".to_string();
    row[csv::SEVERITY] = "=high".to_string();
    let mut out = String::new();
    csv::push_row(&mut out, &row);
    assert!(!out.contains('\''), "{out:?}");
}
