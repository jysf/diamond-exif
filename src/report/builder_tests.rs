//! SPEC-018: the incremental builder equals the one-shot renderers.
//!
//! Like `render_tests.rs`, nothing here reads a file outside `src/` (`just
//! lint-red-proof` copies only `Cargo.toml` and `src/`); the cross product over
//! the committed seeds is in `tests/report_render.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use super::*;
use crate::audit::audit;
use crate::policy::StripPolicy;

fn tiff_make(make: &[u8]) -> Vec<u8> {
    let mut payload = make.to_vec();
    payload.push(0);
    let mut v = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&0x010Fu16.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(&26u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&payload);
    v
}

/// Inputs that reach every status: ok with findings, unsupported, unreadable,
/// issues.
struct Inputs {
    clean: Vec<u8>,
    png: Vec<u8>,
    bad: Vec<u8>,
    issues: Vec<u8>,
}

fn inputs() -> Inputs {
    Inputs {
        clean: tiff_make(b"Cam\"era,=1"),
        png: vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
        bad: b"II\x2A\x00\x40\x00\x00\x00".to_vec(),
        issues: vec![0xFF, 0xD8],
    }
}

/// Six entries covering Audited (every status) and Failed.
fn entries(i: &Inputs) -> Vec<FileEntry<'_>> {
    vec![
        FileEntry {
            label: b"a,\"x\"",
            result: FileResult::Audited(audit(&i.clean)),
        },
        FileEntry {
            label: b"=b",
            result: FileResult::Audited(audit(&i.png)),
        },
        FileEntry {
            label: b"c",
            result: FileResult::Audited(audit(&i.bad)),
        },
        FileEntry {
            label: b"d\n",
            result: FileResult::Audited(audit(&i.issues)),
        },
        FileEntry {
            label: b"e",
            result: FileResult::Failed {
                reason: FailReason::TooLarge,
                detail: b"big\r\n",
            },
        },
        FileEntry {
            label: b"f",
            result: FileResult::Audited(audit(&i.clean)),
        },
    ]
}

const FORMATS: [Format; 3] = [Format::Text, Format::Json, Format::Csv];

fn option_set() -> Vec<RenderOptions> {
    let mut out = Vec::new();
    for policy in [None, Some(StripPolicy::Gps), Some(StripPolicy::AllPii)] {
        for inventory in [false, true] {
            for values in [false, true] {
                out.push(RenderOptions {
                    policy,
                    inventory,
                    values,
                });
            }
        }
    }
    out
}

fn built(format: Format, files: &[&FileEntry<'_>], opts: RenderOptions) -> String {
    let mut b = ReportBuilder::new(format, opts);
    for f in files {
        b.add(f);
    }
    b.finish()
}

/// The one-shot renderer of a format, called by name (not through `render`).
fn oneshot(format: Format, files: &[FileEntry<'_>], opts: &RenderOptions) -> String {
    match format {
        Format::Text => render_text(files, opts),
        Format::Json => render_json(files, opts),
        Format::Csv => render_csv(files, opts),
    }
}

#[test]
fn builder_with_no_entries_equals_the_empty_one_shot_in_every_format() {
    for opts in option_set() {
        for format in FORMATS {
            let b = ReportBuilder::new(format, opts);
            assert!(b.is_empty());
            assert_eq!(b.len(), 0);
            assert_eq!(b.summary(), summarize(&[], &opts));
            let got = b.finish();
            assert_eq!(got, oneshot(format, &[], &opts), "{format:?} {opts:?}");
            assert!(!got.is_empty(), "{format:?}: the frame is not empty");
        }
    }
    // The empty JSON document is exactly the frame with `"files":[]`.
    let json = ReportBuilder::new(Format::Json, RenderOptions::default()).finish();
    assert!(json.ends_with("\"files\":[]}\n"), "{json}");
}

#[test]
fn builder_equals_the_one_shot_for_one_and_for_many_entries_in_every_format() {
    let i = inputs();
    let all = entries(&i);
    for opts in option_set() {
        for format in FORMATS {
            for n in [1usize, 2, 6] {
                let files = &all[..n];
                let refs: Vec<&FileEntry<'_>> = files.iter().collect();
                assert_eq!(
                    built(format, &refs, opts),
                    oneshot(format, files, &opts),
                    "{format:?} n={n} {opts:?}"
                );
            }
        }
    }
}

#[test]
fn builder_fragments_appear_in_add_order_and_are_independent_of_neighbours() {
    let i = inputs();
    let all = entries(&i);
    let opts = RenderOptions {
        policy: Some(StripPolicy::AllPii),
        inventory: true,
        values: true,
    };
    let a = all[0].clone();
    let b = all[4].clone();
    // CSV: header + frag(a) + frag(b), in add order.
    let header = render_csv(&[], &opts);
    let fa = render_csv(std::slice::from_ref(&a), &opts)
        .strip_prefix(&header)
        .unwrap()
        .to_string();
    let fb = render_csv(std::slice::from_ref(&b), &opts)
        .strip_prefix(&header)
        .unwrap()
        .to_string();
    assert!(!fa.is_empty() && !fb.is_empty() && fa != fb);
    assert_eq!(
        render_csv(&[a.clone(), b.clone()], &opts),
        format!("{header}{fa}{fb}")
    );
    assert_eq!(
        render_csv(&[b.clone(), a.clone()], &opts),
        format!("{header}{fb}{fa}")
    );
    // Text: frag(a) frag(b), then the summary block.
    let block = |e: &FileEntry<'_>| {
        let one = render_text(std::slice::from_ref(e), &opts);
        one.split("summary\n").next().unwrap().to_string()
    };
    let (ba, bb) = (block(&a), block(&b));
    // The blocks are the files' own (a summary-first text would make both
    // empty, and "empty then summary" would pass the prefix checks below).
    assert!(
        ba.starts_with("file: ") && bb.starts_with("file: "),
        "{ba:?} {bb:?}"
    );
    assert!(ba != bb);
    assert!(render_text(&[a.clone(), b.clone()], &opts).starts_with(&format!("{ba}{bb}summary\n")));
    assert!(render_text(&[b.clone(), a.clone()], &opts).starts_with(&format!("{bb}{ba}summary\n")));
    // JSON: the files array is frag(a),frag(b).
    let files_of = |files: &[FileEntry<'_>]| {
        let j = render_json(files, &opts);
        let at = j.find("\"files\":[{").unwrap() + "\"files\":[".len();
        j[at..].strip_suffix("]}\n").unwrap().to_string()
    };
    let (ja, jb) = (
        files_of(std::slice::from_ref(&a)),
        files_of(std::slice::from_ref(&b)),
    );
    assert_eq!(files_of(&[a.clone(), b.clone()]), format!("{ja},{jb}"));
    assert_eq!(files_of(&[b.clone(), a.clone()]), format!("{jb},{ja}"));
    // And the builder agrees in both orders.
    for format in FORMATS {
        assert_eq!(
            built(format, &[&a, &b], opts),
            oneshot(format, &[a.clone(), b.clone()], &opts)
        );
        assert_eq!(
            built(format, &[&b, &a], opts),
            oneshot(format, &[b.clone(), a.clone()], &opts)
        );
    }
}

#[test]
fn builder_adding_the_same_entry_twice_gives_two_fragments() {
    let i = inputs();
    let all = entries(&i);
    let opts = RenderOptions {
        policy: Some(StripPolicy::Gps),
        inventory: false,
        values: true,
    };
    let e = &all[0];
    for format in FORMATS {
        let mut b = ReportBuilder::new(format, opts);
        b.add(e);
        b.add(e);
        assert_eq!(b.len(), 2);
        let twice = b.finish();
        assert_eq!(twice, oneshot(format, &[e.clone(), e.clone()], &opts));
        let once = built(format, &[e], opts);
        assert!(twice.len() > once.len(), "{format:?}");
    }
    let csv = built(Format::Csv, &[e, e], opts);
    let header = render_csv(&[], &opts);
    let body = csv.strip_prefix(&header).unwrap();
    let (x, y) = body.split_at(body.len() / 2);
    assert_eq!(x, y, "two identical fragments");
    assert!(!x.is_empty());
}

#[test]
fn builder_summary_equals_summarize_at_every_step() {
    let i = inputs();
    let all = entries(&i);
    for opts in option_set() {
        for format in FORMATS {
            let mut b = ReportBuilder::new(format, opts);
            assert_eq!(b.summary(), summarize(&[], &opts));
            for (n, e) in all.iter().enumerate() {
                b.add(e);
                assert_eq!(b.len(), n + 1);
                assert!(!b.is_empty());
                assert_eq!(
                    b.summary(),
                    summarize(&all[..=n], &opts),
                    "{format:?} after {} entries {opts:?}",
                    n + 1
                );
            }
        }
    }
    // A residual is None over no reliable file, at every step (a failed file
    // first, then a reliable one).
    let opts = RenderOptions {
        policy: Some(StripPolicy::Gps),
        ..RenderOptions::default()
    };
    let mut b = ReportBuilder::new(Format::Json, opts);
    b.add(&all[4]);
    assert_eq!(b.summary().residual_findings, None);
    b.add(&all[0]);
    assert_eq!(b.summary().residual_findings, Some(1));
}

#[test]
fn builder_keeps_no_borrow_of_the_added_entry() {
    let opts = RenderOptions {
        policy: Some(StripPolicy::AllPii),
        inventory: true,
        values: true,
    };
    for format in FORMATS {
        let mut b = ReportBuilder::new(format, opts);
        let want;
        {
            // The buffer, the label and the detail all die before `finish`.
            let mut buffer = tiff_make(b"Secret-Make-Value");
            let label = b"short-lived label".to_vec();
            let detail = b"short-lived detail".to_vec();
            {
                let audited = FileEntry {
                    label: &label,
                    result: FileResult::Audited(audit(&buffer)),
                };
                let failed = FileEntry {
                    label: &label,
                    result: FileResult::Failed {
                        reason: FailReason::Other,
                        detail: &detail,
                    },
                };
                want = oneshot(format, &[audited.clone(), failed.clone()], &opts);
                b.add(&audited);
                b.add(&failed);
            }
            // Overwrite the buffer: the fragments must already be made.
            buffer.fill(b'Z');
            assert!(buffer.iter().all(|x| *x == b'Z'));
        }
        let summary = b.summary();
        assert_eq!(summary.files, 2);
        let got = b.finish();
        assert_eq!(got, want, "{format:?}");
        assert!(got.contains("Secret-Make-Value"), "{format:?}");
    }
}

#[test]
fn builder_retains_only_fragments_not_input_bytes() {
    // A large synthetic entry (a 3 MiB failure detail, which `safe_text` keeps
    // whole): after the buffer is dropped, the output grows by the fragment of
    // each added file and by nothing else.
    let opts = RenderOptions::default();
    let big_len = 3 * 1024 * 1024;
    for format in FORMATS {
        let mut lens = Vec::new();
        for copies in 0..=3usize {
            let mut b = ReportBuilder::new(format, opts);
            {
                let big = vec![b'a'; big_len];
                let e = FileEntry {
                    label: b"big",
                    result: FileResult::Failed {
                        reason: FailReason::Other,
                        detail: &big,
                    },
                };
                for _ in 0..copies {
                    b.add(&e);
                }
            }
            lens.push(b.finish().len());
        }
        // JSON separates fragments with one comma.
        let sep = usize::from(format == Format::Json);
        let frag = lens[2] - lens[1] - sep;
        assert!(frag >= big_len, "{format:?}: fragment {frag}");
        assert!(frag < big_len + 1024, "{format:?}: fragment {frag}");
        // Constant growth per file after the first.
        assert_eq!(lens[3] - lens[2], lens[2] - lens[1], "{format:?}");
        // The first file adds its fragment and at most the digits of the
        // summary: never a second copy of the input.
        assert!(lens[1] - lens[0] >= frag, "{format:?} {lens:?}");
        assert!(lens[1] - lens[0] < frag + 64, "{format:?} {lens:?}");
    }
}

#[test]
fn builder_json_assembly_puts_the_running_summary_before_the_files() {
    let i = inputs();
    let all = entries(&i);
    let opts = RenderOptions {
        policy: Some(StripPolicy::Gps),
        inventory: false,
        values: false,
    };
    let mut b = ReportBuilder::new(Format::Json, opts);
    for e in &all {
        b.add(e);
    }
    let s = b.summary();
    let json = b.finish();
    let summary_at = json.find("\"summary\":{\"files\":").unwrap();
    let files_at = json.find("\"files\":[{").unwrap();
    assert!(summary_at < files_at);
    assert!(json.starts_with("{\"schema_version\":1,\"policy\":\"gps\",\"options\":{"));
    assert!(json.contains(&format!("\"summary\":{{\"files\":{},", all.len())));
    assert_eq!(s.files, all.len());
    // The summary is the one of ALL entries, including the last.
    let head = &json[summary_at..files_at];
    assert!(
        head.contains(&format!("\"unreliable_files\":{}", s.unreliable_files)),
        "{head}"
    );
    assert!(head.contains(&format!("\"findings\":{},", s.findings)));
    assert!(head.contains(&format!("\"ok\":{},", s.ok)));
    assert!(json.ends_with("]}\n"));
}

#[test]
fn render_dispatches_to_the_matching_one_shot() {
    let i = inputs();
    let all = entries(&i);
    for opts in option_set() {
        assert_eq!(render(Format::Text, &all, &opts), render_text(&all, &opts));
        assert_eq!(render(Format::Json, &all, &opts), render_json(&all, &opts));
        assert_eq!(render(Format::Csv, &all, &opts), render_csv(&all, &opts));
    }
    // The three outputs differ from each other, so a swapped dispatch shows.
    let o = RenderOptions::default();
    let (t, j, c) = (
        render(Format::Text, &all, &o),
        render(Format::Json, &all, &o),
        render(Format::Csv, &all, &o),
    );
    assert!(t != j && j != c && t != c);
    assert!(j.starts_with('{') && c.starts_with("label,") && t.starts_with("file: "));
}

/// A little-endian TIFF whose IFD0 holds one inline ASCII entry `tag` = "a".
fn tiff_inline_ascii(tag: u16) -> Vec<u8> {
    let mut v = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&tag.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&2u32.to_le_bytes());
    v.extend_from_slice(&[b'a', 0, 0, 0]);
    v.extend_from_slice(&0u32.to_le_bytes());
    v
}

#[test]
fn builder_summary_max_severity_is_the_maximum_in_both_add_orders() {
    use crate::audit::Severity;
    // BodySerialNumber is high (device_id); Make is low (camera_model).
    let high_bytes = tiff_inline_ascii(0xA431);
    let low_bytes = tiff_make(b"Cam");
    let high = FileEntry {
        label: b"high",
        result: FileResult::Audited(audit(&high_bytes)),
    };
    let low = FileEntry {
        label: b"low",
        result: FileResult::Audited(audit(&low_bytes)),
    };
    let sev = |e: &FileEntry<'_>| match &e.result {
        FileResult::Audited(a) => a.max_severity(),
        FileResult::Failed { .. } => None,
    };
    // Artist is medium (authorship): the middle rank, so a swapped Low/Medium
    // ranking (Medium counted below Low) changes a maximum (SPEC-018 verify D2).
    let medium_bytes = tiff_inline_ascii(0x013B);
    let medium = FileEntry {
        label: b"medium",
        result: FileResult::Audited(audit(&medium_bytes)),
    };
    assert_eq!(sev(&high), Some(Severity::High));
    assert_eq!(sev(&medium), Some(Severity::Medium));
    assert_eq!(sev(&low), Some(Severity::Low));
    // Medium with Low only, both orders: the maximum is Medium.
    for opts in option_set() {
        for order in [[&medium, &low], [&low, &medium]] {
            for format in FORMATS {
                let mut b = ReportBuilder::new(format, opts);
                for e in order {
                    b.add(e);
                }
                assert_eq!(b.summary().max_severity, Some(Severity::Medium));
            }
        }
        // All three, in every order: High; without High, Medium.
        let all: [[&FileEntry<'_>; 3]; 6] = [
            [&high, &medium, &low],
            [&high, &low, &medium],
            [&medium, &high, &low],
            [&medium, &low, &high],
            [&low, &high, &medium],
            [&low, &medium, &high],
        ];
        for order in all {
            let mut b = ReportBuilder::new(Format::Json, opts);
            for e in order {
                b.add(e);
            }
            assert_eq!(b.summary().max_severity, Some(Severity::High));
        }
    }
    for opts in option_set() {
        for order in [[&high, &low], [&low, &high]] {
            for format in FORMATS {
                let mut b = ReportBuilder::new(format, opts);
                let mut so_far: Vec<FileEntry<'_>> = Vec::new();
                for e in order {
                    b.add(e);
                    so_far.push(e.clone());
                    assert_eq!(b.summary(), summarize(&so_far, &opts));
                }
                // After both, the maximum, whichever came first.
                assert_eq!(b.summary().max_severity, Some(Severity::High));
            }
        }
    }
}

#[test]
fn format_names_are_pinned() {
    assert_eq!(Format::Text.name(), "text");
    assert_eq!(Format::Json.name(), "json");
    assert_eq!(Format::Csv.name(), "csv");
}
