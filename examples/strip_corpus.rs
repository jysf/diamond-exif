//! A dev example (SPEC-020): run the strip writer over a directory of real
//! photos, in place and in memory, and print COUNTS only.
//!
//! `cargo run --release --example strip_corpus -- DIR` walks `DIR` (recursively),
//! takes every file that starts `FF D8` for a JPEG, strips it with `All` and with
//! `Everything` (the library's `strip`, which runs the self-check), checks
//! independently that the scan region is byte-identical and that the output's
//! audit has none of the removed categories, and prints:
//!
//! * the number of files and of JPEGs;
//! * per policy: `Ok` and each error kind, the bytes removed, the patches
//!   written, the files with a Multi-Picture index, how many outputs have an
//!   identical scan region and an audit clean of the policy's categories;
//! * for every file that is not a plain `Ok` and for every Multi-Picture file,
//!   one line with the file's SHORT HASH (the first 8 hex digits of its FNV-1a
//!   64 over the content) and the outcome. Never a path, a value or a byte of
//!   content.
//!
//! `--emit HASH POLICY` (POLICY is `all` or `everything`) writes the one
//! stripped file whose short hash is HASH to standard output, to be piped to
//! `exiftool -`; nothing is saved. Nothing is ever written to disk. The library
//! does no I/O: the reading is this example's. std only.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use exif_core::audit::audit;
use exif_core::jpeg::{self, Jpeg};
use exif_core::{strip, StripError, StripPolicy};

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn short(bytes: &[u8]) -> String {
    format!("{:08x}", fnv1a_64(bytes) >> 32)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn error_kind(e: &StripError) -> String {
    match e {
        StripError::Unsupported { .. } => "Unsupported".into(),
        StripError::NotJpeg => "NotJpeg".into(),
        StripError::Unstrippable { reason, .. } => format!("Unstrippable({reason:?})"),
        StripError::NeedsEntryEdit { .. } => "NeedsEntryEdit".into(),
        StripError::Overlap { .. } => "Overlap".into(),
        StripError::MpfNotIndexed { .. } => "MpfNotIndexed".into(),
        StripError::VerifyFailed { check } => format!("VerifyFailed({check:?})"),
        other => format!("{other:?}"),
    }
}

#[derive(Default)]
struct Tally {
    ok: usize,
    errors: BTreeMap<String, usize>,
    removed_bytes: u64,
    patches: usize,
    scan_identical: usize,
    audit_clean: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(dir) = args.first() else {
        eprintln!("usage: strip_corpus DIR [--emit HASH all|everything]");
        std::process::exit(2);
    };
    let emit = match args.get(1).map(String::as_str) {
        Some("--emit") => Some((
            args.get(2).cloned().unwrap_or_default(),
            args.get(3).cloned().unwrap_or_default(),
        )),
        _ => None,
    };
    let policies = [
        ("all", StripPolicy::All),
        ("everything", StripPolicy::Everything),
    ];

    let mut files = Vec::new();
    walk(Path::new(dir), &mut files);

    if let Some((hash, which)) = emit {
        // The file is named by the short hash the counts print, never by path.
        // POLICY `none` writes the input as it is (for a before/after read).
        for path in &files {
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            if bytes.starts_with(&[0xFF, 0xD8]) && short(&bytes).starts_with(&hash) {
                let out = if which == "none" {
                    bytes.clone()
                } else {
                    let policy = policies
                        .iter()
                        .find(|(n, _)| *n == which)
                        .map(|(_, p)| *p)
                        .expect("policy all, everything or none");
                    strip(&bytes, policy).expect("the file strips").into_bytes()
                };
                // A reader that stops early (`head -c2`) is not an error.
                if let Err(e) = std::io::stdout().write_all(&out) {
                    if e.kind() != std::io::ErrorKind::BrokenPipe {
                        eprintln!("write stdout: {e}");
                        std::process::exit(1);
                    }
                }
                return;
            }
        }
        eprintln!("no JPEG with that hash");
        std::process::exit(1);
    }

    let mut jpegs = 0usize;
    let mut with_mpf = 0usize;
    let mut tallies: Vec<Tally> = policies.iter().map(|_| Tally::default()).collect();
    let mut lines: Vec<String> = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if !bytes.starts_with(&[0xFF, 0xD8]) {
            continue;
        }
        jpegs += 1;
        let has_mpf = jpeg::read(&bytes).is_ok_and(|m| m.mpf().is_some());
        if has_mpf {
            with_mpf += 1;
        }
        let id = short(&bytes);
        for (i, (name, policy)) in policies.iter().enumerate() {
            let t = &mut tallies[i];
            match strip(&bytes, *policy) {
                Ok(out) => {
                    t.ok += 1;
                    t.removed_bytes += (bytes.len() - out.bytes().len()) as u64;
                    t.patches += out.patches().len();
                    let same_scan = match (Jpeg::parse(&bytes), Jpeg::parse(out.bytes())) {
                        (Ok(a), Ok(b)) => a.scan_region() == b.scan_region(),
                        _ => false,
                    };
                    let spec = policy.spec();
                    let clean = audit(out.bytes())
                        .findings()
                        .iter()
                        .all(|f| !spec.contains(f.category));
                    t.scan_identical += usize::from(same_scan);
                    t.audit_clean += usize::from(clean);
                    if has_mpf || !same_scan || !clean {
                        lines.push(format!(
                            "{id} {name} Ok mpf={has_mpf} patches={} scan_identical={same_scan} audit_clean={clean}",
                            out.patches().len()
                        ));
                    }
                }
                Err(e) => {
                    *t.errors.entry(error_kind(&e)).or_default() += 1;
                    lines.push(format!("{id} {name} {} mpf={has_mpf}", error_kind(&e)));
                }
            }
        }
    }

    println!("files {}  jpegs {jpegs}  with_mpf {with_mpf}", files.len());
    for ((name, _), t) in policies.iter().zip(&tallies) {
        println!(
            "{name}: ok {}  errors {:?}  bytes_removed {}  patches {}  scan_identical {}  audit_clean {}",
            t.ok, t.errors, t.removed_bytes, t.patches, t.scan_identical, t.audit_clean
        );
    }
    for l in lines {
        println!("{l}");
    }
}
