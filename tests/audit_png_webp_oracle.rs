//! SPEC-032 / DEC-020: the audit of a PNG and a WebP against ExifTool.
//!
//! Two checks, both with a replay (committed records, no tool) and a live run
//! (the installed ExifTool; `SKIP` without it, a failure with
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`):
//!
//! 1. **Category presence.** For every hand-built PNG and WebP oracle fixture
//!    (and, for GPS only, every committed walker seed) the audit has a `Gps`
//!    finding iff ExifTool printed a `Composite:GPS*` value, an `Xmp` finding
//!    iff it printed `XMP-xmp:CreatorTool`, and an `Icc` finding iff it printed
//!    an ICC selector (`PNG:ProfileName` or `ICC-header:ProfileFileSignature`
//!    for a PNG, the signature for a WebP). The audit LOCATES; ExifTool
//!    VALIDATES and reads one `eXIf` only; the cases where that makes them
//!    differ are listed below, each with its reason and a question id, and the
//!    test requires the list to be exactly the observed differences, so it can
//!    neither grow nor rot.
//! 2. **Text keywords.** `png-text-keywords` carries one `tEXt` chunk per
//!    keyword; its record (`-a -G1 -s -n`, the `[PNG]` group) maps each keyword
//!    to the tag name ExifTool printed. The audit's category for a keyword must
//!    agree with the printed name (`Artist`, `Author` to Authorship,
//!    `Copyright` to Rights (SPEC-033), `CreationTime`, `CreateDate`, `ModDate` to Timestamps,
//!    `Software` to Software, the rest FreeText),
//!    except for the keywords in [`KEYWORD_DIVERGENCES`].
//!
//! Red-proofs: a perturbed record makes each replay name the fixture or the
//! keyword. ExifTool is run, never read or linked.

#[path = "support/oracle.rs"]
mod oracle;
#[path = "support/oracle_png.rs"]
mod oracle_png;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/oracle_webp.rs"]
mod oracle_webp;
#[path = "support/tools.rs"]
mod tools;

use exif_core::audit::{audit, text_keyword_category, Category};
use oracle::SELECTORS;
use std::collections::BTreeSet;
use std::path::PathBuf;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_record(name: &str) -> tools::Record {
    let path = root()
        .join("tests/oracle-fixtures/exiftool")
        .join(format!("{name}.txt"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}; run `just oracle-record`)",
            path.display()
        )
    });
    tools::parse_record(&text).unwrap_or_else(|| panic!("{}: bad record header", path.display()))
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

// ── category presence ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Format {
    Png,
    Webp,
}

/// Which of the three categories a side has.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct Presence {
    gps: bool,
    xmp: bool,
    icc: bool,
}

/// What ExifTool printed, from the fields of a record (`png_names` or
/// `webp_names` order, or `names(SELECTORS)` for a seed, which has no region
/// selectors: those two slots are `None`, and the flag says they are unknown).
fn theirs(format: Format, fields: &[Option<String>], regions_known: bool) -> Presence {
    let names: Vec<&str> = match format {
        Format::Png => oracle::png_names(),
        Format::Webp => oracle::webp_names(),
    };
    let on = |wanted: &dyn Fn(&str) -> bool| {
        names
            .iter()
            .zip(fields)
            .any(|(n, f)| wanted(n) && f.is_some())
    };
    Presence {
        gps: on(&|n| n.starts_with("Composite:GPS")),
        xmp: regions_known && on(&|n| n == "XMP-xmp:CreatorTool"),
        icc: regions_known
            && on(&|n| n == "ICC-header:ProfileFileSignature" || n == "PNG:ProfileName"),
    }
}

/// What the audit has.
fn ours(bytes: &[u8]) -> Presence {
    let a = audit(bytes);
    let has = |c: Category| a.findings().iter().any(|f| f.category == c);
    Presence {
        gps: has(Category::Gps),
        xmp: has(Category::Xmp),
        icc: has(Category::Icc),
    }
}

/// A recorded difference between the audit and ExifTool for one input: the
/// categories on which they differ (`gps`, `xmp`, `icc`), why, and the question
/// that tracks it in `guidance/questions.yaml`. The ids of
/// `oracle::KNOWN_DIVERGENCES` are reused where one already covers the case.
struct Difference {
    input: &'static str,
    categories: &'static str,
    why: &'static str,
    question_id: &'static str,
}

/// Every observed difference, as measured on ExifTool 13.55. Kept by hand.
const DIFFERENCES: &[Difference] = &[
    Difference {
        input: "png-raw-xmp-itxt",
        categories: "xmp",
        why: "ExifTool decodes the ImageMagick raw profile and prints the XMP; the audit locates the chunk as a RawProfile (OtherSegment, may hold XMP) and has no Xmp finding, but the file is never clean and its status is `unreadable` (DEC-018, DEC-020 clause 5)",
        question_id: "png-raw-profile-not-decoded",
    },
    Difference {
        input: "png-raw-xmp-text",
        categories: "xmp",
        why: "ExifTool decodes the ImageMagick raw profile and prints the XMP; the audit locates the chunk as a RawProfile (OtherSegment, may hold XMP) and has no Xmp finding, but the file is never clean and its status is `unreadable` (DEC-018, DEC-020 clause 5)",
        question_id: "png-raw-profile-not-decoded",
    },
    Difference {
        input: "png-raw-xmp-ztxt",
        categories: "xmp",
        why: "ExifTool decodes the ImageMagick raw profile and prints the XMP; the audit locates the chunk as a RawProfile (OtherSegment, may hold XMP) and has no Xmp finding, but the file is never clean and its status is `unreadable` (DEC-018, DEC-020 clause 5)",
        question_id: "png-raw-profile-not-decoded",
    },
    Difference {
        input: "png-xmp-itxt-flag-2",
        categories: "xmp",
        why: "ExifTool fails to inflate an iTXt XMP with the invalid flag 2 and prints nothing; the audit locates it (located, not valid)",
        question_id: "png-xmp-flag-2-located-not-read",
    },
    Difference {
        input: "webp-exif-16-then-gps",
        categories: "gps",
        why: "ExifTool reads the 17th EXIF chunk, the library keeps 16 (MAX_WEBP_EXIF_BLOCKS) so the GPS is not listed; the audit is NOT clean for it (asserted by `the_seventeenth_exif_block_makes_the_audit_not_clean`)",
        question_id: "webp-exif-block-cap",
    },
    Difference {
        input: "webp-icc-garbage",
        categories: "icc",
        why: "ExifTool validates the ICC profile and prints no signature for it; the audit locates a non-empty ICCP chunk (located, not valid)",
        question_id: "webp-iccp-profile-not-validated",
    },
    Difference {
        input: "webp-icc-header-only",
        categories: "icc",
        why: "ExifTool validates the ICC profile and prints no signature for it; the audit locates a non-empty ICCP chunk (located, not valid)",
        question_id: "webp-iccp-profile-not-validated",
    },
    Difference {
        input: "webp-icc-stub-132",
        categories: "icc",
        why: "ExifTool validates the ICC profile and prints no signature for it; the audit locates a non-empty ICCP chunk (located, not valid)",
        question_id: "webp-iccp-profile-not-validated",
    },
    Difference {
        input: "webp-icc-zero-tags",
        categories: "icc",
        why: "ExifTool validates the ICC profile and prints no signature for it; the audit locates a non-empty ICCP chunk (located, not valid)",
        question_id: "webp-iccp-profile-not-validated",
    },
    Difference {
        input: "webp-xmp-garbage",
        categories: "xmp",
        why: "ExifTool parses the XMP packet and prints nothing for this one; the audit locates a non-empty XMP chunk (located, not valid)",
        question_id: "webp-xmp-content-not-parsed",
    },
    Difference {
        input: "webp-xmp-nul-id-1-bytes",
        categories: "xmp",
        why: "ExifTool parses the XMP packet and prints nothing for this one; the audit locates a non-empty XMP chunk (located, not valid)",
        question_id: "webp-xmp-content-not-parsed",
    },
    Difference {
        input: "webp-xmp-nul-id-2-bytes",
        categories: "xmp",
        why: "ExifTool parses the XMP packet and prints nothing for this one; the audit locates a non-empty XMP chunk (located, not valid)",
        question_id: "webp-xmp-content-not-parsed",
    },
    Difference {
        input: "webp-xmp-nul-id-3-bytes",
        categories: "xmp",
        why: "ExifTool parses the XMP packet and prints nothing for this one; the audit locates a non-empty XMP chunk (located, not valid)",
        question_id: "webp-xmp-content-not-parsed",
    },
    Difference {
        input: "webp-xmp-nul-id-4-bytes",
        categories: "xmp",
        why: "ExifTool parses the XMP packet and prints nothing for this one; the audit locates a non-empty XMP chunk (located, not valid)",
        question_id: "webp-xmp-content-not-parsed",
    },
    Difference {
        input: "webp-xmp-nul-id-5-bytes",
        categories: "xmp",
        why: "ExifTool parses the XMP packet and prints nothing for this one; the audit locates a non-empty XMP chunk (located, not valid)",
        question_id: "webp-xmp-content-not-parsed",
    },
    Difference {
        input: "webp-xmp-nul-id-6-bytes",
        categories: "xmp",
        why: "ExifTool parses the XMP packet and prints nothing for this one; the audit locates a non-empty XMP chunk (located, not valid)",
        question_id: "webp-xmp-content-not-parsed",
    },
    Difference {
        input: "webp-xmp-nul-id-7-bytes",
        categories: "xmp",
        why: "ExifTool parses the XMP packet and prints nothing for this one; the audit locates a non-empty XMP chunk (located, not valid)",
        question_id: "webp-xmp-content-not-parsed",
    },
];

fn differing(format: Format, name: &str, ours: Presence, theirs: Presence) -> String {
    let mut out = Vec::new();
    for (c, a, b) in [
        ("gps", ours.gps, theirs.gps),
        ("xmp", ours.xmp, theirs.xmp),
        ("icc", ours.icc, theirs.icc),
    ] {
        if a != b {
            out.push(c);
        }
    }
    let _ = (format, name);
    out.join("+")
}

/// One input of the check.
struct Case {
    name: String,
    format: Format,
    bytes: Vec<u8>,
    /// The committed record this input's ExifTool answer is read from.
    record: String,
    /// Whether the record has the XMP and ICC selectors (a hand-built fixture
    /// does, a seed does not).
    regions_known: bool,
}

fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    for (name, bytes) in oracle_png::fixtures() {
        v.push(Case {
            record: name.clone(),
            name,
            format: Format::Png,
            bytes,
            regions_known: true,
        });
    }
    for (name, bytes) in oracle_webp::fixtures() {
        v.push(Case {
            record: name.clone(),
            name,
            format: Format::Webp,
            bytes,
            regions_known: true,
        });
    }
    for (dir, prefix, format) in [
        ("png_chunks", "png", Format::Png),
        ("webp_riff", "webp", Format::Webp),
    ] {
        let seeds = root().join("fuzz/seeds").join(dir);
        let mut files: Vec<_> = std::fs::read_dir(&seeds)
            .expect("read seeds")
            .map(|e| e.expect("entry").path())
            .collect();
        files.sort();
        for path in files {
            let n = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            v.push(Case {
                name: format!("{prefix}-seed-{n}"),
                format,
                bytes: std::fs::read(&path).expect("read seed"),
                record: format!("{prefix}-seed-{n}"),
                regions_known: false,
            });
        }
    }
    v
}

/// ExifTool's fields for a case, from its committed record. `None` when the
/// record is a not-run record (ExifTool was not run on the input).
fn recorded_fields(c: &Case) -> Option<Vec<Option<String>>> {
    let rec = read_record(&c.record);
    if rec.outcome.is_some() {
        return None;
    }
    let names: Vec<&str> = if c.regions_known {
        match c.format {
            Format::Png => oracle::png_names(),
            Format::Webp => oracle::webp_names(),
        }
    } else {
        oracle::names(SELECTORS)
    };
    Some(tools::parse_fields(&names, &rec.stdout).unwrap_or_else(|e| panic!("{}: {e}", c.record)))
}

/// The presence ExifTool printed for a case, given its fields.
fn theirs_of(c: &Case, fields: &[Option<String>]) -> Presence {
    if c.regions_known {
        theirs(c.format, fields, true)
    } else {
        // A seed's record has the Exif selectors only: pad the two region slots.
        let mut padded = fields.to_vec();
        let extra = match c.format {
            Format::Png => oracle::PNG_REGION_SELECTORS.len(),
            Format::Webp => oracle::WEBP_REGION_SELECTORS.len(),
        };
        padded.extend(std::iter::repeat_n(None, extra));
        theirs(c.format, &padded, false)
    }
}

/// The inputs whose audit and ExifTool disagree on a compared category, as
/// `(input, categories)`, sorted. With `only` set, the categories of the other
/// slots are ignored (seeds compare GPS only: `regions_known` handles that).
fn observed(per_case: &[(String, Presence, Presence)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = per_case
        .iter()
        .filter_map(|(n, o, t)| {
            let d = differing(Format::Png, n, *o, *t);
            (!d.is_empty()).then(|| (n.clone(), d))
        })
        .collect();
    out.sort();
    out
}

fn replay_presence() -> Vec<(String, Presence, Presence)> {
    let mut out = Vec::new();
    for c in cases() {
        let Some(fields) = recorded_fields(&c) else {
            continue;
        };
        let t = theirs_of(&c, &fields);
        let mut o = ours(&c.bytes);
        if !c.regions_known {
            // Seeds are compared on GPS only.
            o.xmp = false;
            o.icc = false;
        }
        out.push((c.name.clone(), o, t));
    }
    out
}

#[test]
fn replay_category_presence_matches_the_records_except_the_recorded_differences() {
    let got = observed(&replay_presence());
    let want: Vec<(String, String)> = {
        let mut w: Vec<(String, String)> = DIFFERENCES
            .iter()
            .map(|d| (d.input.to_string(), d.categories.to_string()))
            .collect();
        w.sort();
        w
    };
    assert_eq!(
        got, want,
        "audit and ExifTool disagree on exactly these inputs (input, categories); a new \
         difference needs a reason and a question, a resolved one must leave the list"
    );
}

#[test]
fn replay_compares_a_useful_number_of_inputs() {
    let all = replay_presence();
    let n = |f: &dyn Fn(&Presence) -> bool| all.iter().filter(|(_, o, _)| f(o)).count();
    // Both formats, with and without each category.
    assert!(all.len() > 250, "{} inputs compared", all.len());
    assert!(n(&|p| p.gps) > 8, "{} inputs with GPS", n(&|p| p.gps));
    assert!(n(&|p| p.xmp) > 3, "{} inputs with XMP", n(&|p| p.xmp));
    assert!(n(&|p| p.icc) > 3, "{} inputs with ICC", n(&|p| p.icc));
    assert!(n(&|p| !p.gps && !p.xmp && !p.icc) > 20, "inputs with none");
    assert!(all.iter().any(|(n, _, _)| n.starts_with("png-")));
    assert!(all.iter().any(|(n, _, _)| n.starts_with("webp-")));
}

#[test]
fn a_perturbed_record_makes_the_replay_name_the_fixture() {
    // Flip each category's presence in the record of one named fixture, one at
    // a time, and require the comparison to name exactly that fixture.
    let cases = cases();
    let pick = |name: &str| cases.iter().find(|c| c.name == name).expect(name);
    for (fixture, category) in [
        ("png-exif-gps-full-le", "gps"),
        ("png-xmp-itxt", "xmp"),
        ("png-iccp-valid", "icc"),
        ("webp-exif-gps-full-le", "gps"),
        ("webp-all-three", "xmp"),
        ("webp-all-three", "icc"),
    ] {
        let c = pick(fixture);
        let fields = recorded_fields(c).expect("recorded");
        let t = theirs_of(c, &fields);
        let o = ours(&c.bytes);
        // Control: the unperturbed record agrees on this category.
        assert_eq!(differing(c.format, fixture, o, t), "", "control {fixture}");
        // Perturbation: the record claims the opposite.
        let mut bad = t;
        match category {
            "gps" => bad.gps = !bad.gps,
            "xmp" => bad.xmp = !bad.xmp,
            _ => bad.icc = !bad.icc,
        }
        let rows = [(fixture.to_string(), o, bad)];
        assert_eq!(
            observed(&rows),
            vec![(fixture.to_string(), category.to_string())],
            "{fixture}: perturbed {category}"
        );
    }
    // And through the real path: perturb the text of a record and re-read it.
    let c = pick("png-exif-gps-full-le");
    let rec = read_record(&c.record);
    let names = oracle::png_names();
    let mut fields = tools::parse_fields(&names, &rec.stdout).expect("fields");
    for (n, f) in names.iter().zip(fields.iter_mut()) {
        if n.starts_with("Composite:GPS") {
            *f = None;
        }
    }
    let t = theirs_of(c, &fields);
    assert!(!t.gps, "the perturbed record has no GPS");
    assert_eq!(
        observed(&[(c.name.clone(), ours(&c.bytes), t)]),
        vec![(c.name.clone(), "gps".to_string())]
    );
}

fn live_presence(c: &Case) -> Option<Presence> {
    let names: Vec<&str> = if c.regions_known {
        match c.format {
            Format::Png => oracle::png_names(),
            Format::Webp => oracle::webp_names(),
        }
    } else {
        oracle::names(SELECTORS)
    };
    if oracle::not_run_divergence(&c.record).is_some() {
        return None;
    }
    let ext = match c.format {
        Format::Png => "png",
        Format::Webp => "webp",
    };
    let path = tools::temp_file_with_ext(&c.name, ext, &c.bytes).expect("temp file");
    let run = tools::run_exiftool_full(&path, &names);
    let _ = std::fs::remove_file(&path);
    let (stdout, _) = run.unwrap_or_else(|e| panic!("{}: {e}", c.name));
    let fields = tools::parse_fields(&names, &stdout).unwrap_or_else(|e| panic!("{}: {e}", c.name));
    Some(theirs_of(c, &fields))
}

#[test]
fn live_category_presence_matches_exiftool_except_the_recorded_differences() {
    let test = "live_category_presence_matches_exiftool_except_the_recorded_differences";
    if !live_gate(test) {
        return;
    }
    let live = tools::exiftool_version().unwrap_or_default();
    println!("{test}: live exiftool {live}");
    let mut rows = Vec::new();
    for c in cases() {
        let Some(t) = live_presence(&c) else { continue };
        let mut o = ours(&c.bytes);
        if !c.regions_known {
            o.xmp = false;
            o.icc = false;
        }
        rows.push((c.name.clone(), o, t));
    }
    let got = observed(&rows);
    let mut want: Vec<(String, String)> = DIFFERENCES
        .iter()
        .map(|d| (d.input.to_string(), d.categories.to_string()))
        .collect();
    want.sort();
    assert_eq!(got, want, "live ExifTool {live}");
    // The committed replay data is what the live tool prints today.
    for c in cases() {
        let Some(fields) = recorded_fields(&c) else {
            continue;
        };
        let Some(t) = live_presence(&c) else { continue };
        assert_eq!(
            theirs_of(&c, &fields),
            t,
            "{}: the committed record is stale (run `just oracle-record`)",
            c.name
        );
    }
}

#[test]
fn every_difference_has_a_question_a_reason_and_is_an_input_of_the_check() {
    let questions = std::fs::read_to_string(root().join("guidance/questions.yaml"))
        .expect("read guidance/questions.yaml");
    let names: BTreeSet<String> = cases().into_iter().map(|c| c.name).collect();
    for d in DIFFERENCES {
        assert!(names.contains(d.input), "{}: not an input", d.input);
        assert!(!d.why.is_empty() && !d.categories.is_empty());
        assert!(
            questions.contains(&format!("- id: {}\n", d.question_id)),
            "{}: no question `{}` in guidance/questions.yaml",
            d.input,
            d.question_id
        );
    }
}

// ── text keywords ───────────────────────────────────────────────────────────

/// The category the ExifTool tag name implies, written out here (not taken
/// from the library): `Artist` and `Author` are authorship, `Copyright` is
/// rights (SPEC-033), `CreationTime` a timestamp, `Software` software, every other name free text.
fn category_of_printed_name(name: &str) -> Category {
    match name {
        "Artist" | "Author" => Category::Authorship,
        "Copyright" => Category::Rights,
        "CreationTime" | "CreateDate" | "ModDate" => Category::Timestamps,
        "Software" => Category::Software,
        _ => Category::FreeText,
    }
}

/// `(keyword, printed tag name)` for every keyword of the fixture, read from
/// the record text: each `[PNG] <name> : v:<keyword>` line.
fn keyword_names(record_text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in record_text.lines() {
        let Some(rest) = line.strip_prefix("[PNG] ") else {
            continue;
        };
        let Some((name, value)) = rest.split_once(" : ") else {
            continue;
        };
        if let Some(keyword) = value.strip_prefix("v:") {
            out.push((keyword.to_string(), name.trim().to_string()));
        }
    }
    out
}

/// A keyword whose printed name disagrees with the audit's table, by name.
struct KeywordDifference {
    keyword: &'static str,
    printed: &'static str,
    why: &'static str,
    question_id: &'static str,
}

/// Keywords ExifTool 13.55 prints under a standard tag name that the audit
/// keeps as free text. The audit's rule is MEASURED (first letter upper-cased,
/// then an exact match, see `text_keyword_category`): ExifTool prints
/// `CreationTime` for these two spellings too, but does not treat their value
/// as a date (`-d` leaves it alone), so the audit does not call them a
/// timestamp. Free text is the higher severity, so the audit errs toward
/// reporting more, never less.
const KEYWORD_DIVERGENCES: &[KeywordDifference] = &[
    KeywordDifference {
        keyword: "Creation time",
        printed: "CreationTime",
        why: "printed as the standard name, not reformatted by -d: an auto-named tag",
        question_id: "png-creation-time-keyword-case",
    },
    KeywordDifference {
        keyword: "creation time",
        printed: "CreationTime",
        why: "printed as the standard name, not reformatted by -d: an auto-named tag",
        question_id: "png-creation-time-keyword-case",
    },
];

fn keyword_mismatches(record_text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = keyword_names(record_text)
        .into_iter()
        .filter(|(k, name)| text_keyword_category(k.as_bytes()) != category_of_printed_name(name))
        .collect();
    out.sort();
    out
}

fn keyword_record_text() -> String {
    let path = root()
        .join("tests/oracle-fixtures/exiftool")
        .join(oracle_png::TEXT_KEYWORDS_RECORD);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}; run `just oracle-record`)",
            path.display()
        )
    })
}

#[test]
fn the_keyword_record_covers_every_keyword_of_the_fixture() {
    let text = keyword_record_text();
    assert!(text.contains("# exiftool-version: 13.55"));
    let named = keyword_names(&text);
    let keywords: Vec<&str> = named.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keywords, oracle_png::TEXT_KEYWORDS.to_vec());
    // The measured cases the rule rests on are in the fixture.
    for k in [
        "Author",
        "author",
        "AUTHOR",
        "aUTHOR",
        "Artist",
        "create-date",
    ] {
        assert!(keywords.contains(&k), "{k}");
    }
}

#[test]
fn the_audit_categorises_every_keyword_as_the_record_says_except_the_recorded_ones() {
    let got = keyword_mismatches(&keyword_record_text());
    let mut want: Vec<(String, String)> = KEYWORD_DIVERGENCES
        .iter()
        .map(|d| (d.keyword.to_string(), d.printed.to_string()))
        .collect();
    want.sort();
    assert_eq!(
        got, want,
        "keywords whose ExifTool name implies another category than the audit's"
    );
    let questions = std::fs::read_to_string(root().join("guidance/questions.yaml"))
        .expect("read guidance/questions.yaml");
    for d in KEYWORD_DIVERGENCES {
        assert!(!d.why.is_empty());
        assert!(
            questions.contains(&format!("- id: {}\n", d.question_id)),
            "{}: no question `{}`",
            d.keyword,
            d.question_id
        );
    }
}

#[test]
fn the_audit_of_the_keyword_fixture_applies_the_table_in_chunk_order() {
    let bytes = oracle_png::text_keywords_png();
    let a = audit(&bytes);
    let cats: Vec<Category> = a.findings().iter().map(|f| f.category).collect();
    let want: Vec<Category> = oracle_png::TEXT_KEYWORDS
        .iter()
        .map(|k| text_keyword_category(k.as_bytes()))
        .collect();
    assert_eq!(cats, want, "one finding per keyword, in chunk order");
    // The table, spot-checked by hand, not through the function under test.
    use Category::*;
    let by_hand = |k: &str| -> Category {
        let i = oracle_png::TEXT_KEYWORDS
            .iter()
            .position(|x| *x == k)
            .expect(k);
        cats[i]
    };
    assert_eq!(by_hand("Author"), Authorship);
    assert_eq!(by_hand("author"), Authorship);
    assert_eq!(by_hand("AUTHOR"), FreeText);
    assert_eq!(by_hand("aUTHOR"), FreeText);
    assert_eq!(by_hand("Copyright"), Rights);
    assert_eq!(by_hand("Creation Time"), Timestamps);
    assert_eq!(by_hand("Software"), Software);
    // L1 rows: ExifTool 13.55 prints `Artist`, `CreateDate`, `ModDate`.
    assert_eq!(by_hand("Artist"), Authorship);
    assert_eq!(by_hand("artist"), Authorship);
    assert_eq!(by_hand("ARTIST"), FreeText);
    assert_eq!(by_hand("create-date"), Timestamps);
    assert_eq!(by_hand("Create-date"), FreeText);
    assert_eq!(by_hand("CREATE-DATE"), FreeText);
    assert_eq!(by_hand("modify-date"), Timestamps);
    assert_eq!(by_hand("Modify-date"), FreeText);
    assert_eq!(by_hand("MODIFY-DATE"), FreeText);
    assert_eq!(by_hand("Title"), FreeText);
    assert_eq!(by_hand("Unknown Keyword"), FreeText);
}

#[test]
fn a_perturbed_keyword_record_makes_the_replay_name_the_keyword() {
    let text = keyword_record_text();
    // Control: the committed record mismatches exactly the recorded keywords.
    let control = keyword_mismatches(&text);
    assert_eq!(control.len(), KEYWORD_DIVERGENCES.len());
    // Perturb: the record claims ExifTool printed `Title` for the keyword
    // `Author`; the audit calls it Authorship, so the replay names `Author`.
    let perturbed = text.replacen(
        "Author                          : v:Author\n",
        "Title                           : v:Author\n",
        1,
    );
    assert_ne!(
        perturbed, text,
        "the record has the line the perturbation edits"
    );
    let got = keyword_mismatches(&perturbed);
    assert!(
        got.contains(&("Author".to_string(), "Title".to_string())),
        "{got:?}"
    );
    assert_eq!(got.len(), control.len() + 1);
    // And the other way: ExifTool printed `Author` for the keyword `Title`.
    let perturbed = text.replacen(
        "Title                           : v:Title\n",
        "Author                          : v:Title\n",
        1,
    );
    assert!(keyword_mismatches(&perturbed).contains(&("Title".to_string(), "Author".to_string())));
}

#[test]
fn live_keyword_names_match_the_committed_record() {
    let test = "live_keyword_names_match_the_committed_record";
    if !live_gate(test) {
        return;
    }
    let bytes = oracle_png::text_keywords_png();
    let path = tools::temp_file_with_ext(oracle_png::TEXT_KEYWORDS_NAME, "png", &bytes)
        .expect("temp file");
    let args: Vec<String> = ["-a", "-G1", "-s", "-n"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let run = tools::run_tool("exiftool", &args, &path);
    let _ = std::fs::remove_file(&path);
    let out = run.expect("run exiftool");
    let live = String::from_utf8_lossy(&out.stdout).into_owned();
    // The live names agree with the committed ones, keyword by keyword.
    assert_eq!(keyword_names(&live), keyword_names(&keyword_record_text()));
    // And the audit agrees with the live names, except the recorded keywords.
    let got = keyword_mismatches(&live);
    let mut want: Vec<(String, String)> = KEYWORD_DIVERGENCES
        .iter()
        .map(|d| (d.keyword.to_string(), d.printed.to_string()))
        .collect();
    want.sort();
    assert_eq!(got, want);
}

/// The one GPS difference is not a miss: the audit says it cannot vouch for the
/// file. A 17th `EXIF` chunk past the library's cap holds the only GPS; the
/// audit lists no GPS, is not clean, and the preview is not reliable.
#[test]
fn the_seventeenth_exif_block_makes_the_audit_not_clean() {
    let (_, bytes) = oracle_webp::fixtures()
        .into_iter()
        .find(|(n, _)| n == "webp-exif-16-then-gps")
        .expect("fixture");
    let a = audit(&bytes);
    assert!(!ours(&bytes).gps, "the 17th block is not listed");
    assert!(!a.health().clean, "but the audit is not clean");
    assert!(
        a.health().jpeg_issues >= 1,
        "the library's cap issue counts"
    );
    let p = exif_core::audit::preview(&a, exif_core::StripPolicy::Everything);
    assert!(!p.reliable());
}

/// DEC-020 clause 5, against ExifTool (live; `SKIP` without it). An ImageMagick
/// raw profile holding Exif with GPS, alone and beside a GPS-free `eXIf`, in a
/// `tEXt` and a `zTXt` chunk: ExifTool 13.55 decodes it and prints
/// `Composite:GPSLatitude` (measured, SPEC-032 fix round), the audit does not
/// decode it, so it has no `Gps` finding. That difference is not a silent miss:
/// the raw profile counts as a failed block, so the file is not clean and its
/// report status is `unreadable` (exit 3 in the CLI). The committed replay
/// fixtures carry no GPS raw profile, so this one is live only.
#[test]
fn a_raw_profile_with_gps_is_decoded_by_exiftool_and_unreadable_to_the_audit() {
    use exif_core::report::{status_of, FileResult, Status};
    let test = "a_raw_profile_with_gps_is_decoded_by_exiftool_and_unreadable_to_the_audit";
    if !live_gate(test) {
        return;
    }
    let tiff_named = |name: &str| -> Vec<u8> {
        oracle_tiff::fixtures()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, b)| b)
            .unwrap_or_else(|| panic!("no TIFF fixture named {name}"))
    };
    let gps_tiff = tiff_named("exif-gps-ne-le");
    let make_tiff = tiff_named("exif-basic-le");
    let body = oracle_png::raw_profile_body("exif", &[&b"Exif\0\0"[..], &gps_tiff].concat());
    let key: &[u8] = b"Raw profile type exif";
    let cases: Vec<(&str, Vec<Vec<u8>>)> = vec![
        ("text", vec![oracle_png::text(key, &body)]),
        ("ztxt", vec![oracle_png::ztxt(key, &body)]),
        (
            "text-beside-exif",
            vec![
                oracle_png::exif_chunk(&make_tiff),
                oracle_png::text(key, &body),
            ],
        ),
        (
            "ztxt-beside-exif",
            vec![
                oracle_png::exif_chunk(&make_tiff),
                oracle_png::ztxt(key, &body),
            ],
        ),
    ];
    for (name, chunks) in cases {
        let bytes = oracle_png::png(&chunks, &[], &[]);
        let path = tools::temp_file_with_ext(&format!("raw-gps-{name}"), "png", &bytes)
            .expect("temp file");
        let args: Vec<String> = ["-a", "-G1", "-s", "-n"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let run = tools::run_tool("exiftool", &args, &path);
        let _ = std::fs::remove_file(&path);
        let live = String::from_utf8_lossy(&run.expect("run exiftool").stdout).into_owned();
        assert!(
            live.contains("[Composite]") && live.contains("GPSLatitude"),
            "{name}: ExifTool printed no GPS (the oracle must see what the audit cannot):\n{live}"
        );
        assert!(
            !ours(&bytes).gps,
            "{name}: the audit decodes no raw profile"
        );
        let a = audit(&bytes);
        assert!(!a.health().clean, "{name}: never clean");
        assert_eq!(a.health().failed_blocks, 1, "{name}");
        assert_eq!(
            status_of(&FileResult::Audited(a)),
            Status::Unreadable,
            "{name}"
        );
    }
}
