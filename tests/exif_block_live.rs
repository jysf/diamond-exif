//! SPEC-025, AC9(a): ExifTool reads the placed block as it reads the source.
//!
//! For every fixture the builder does not refuse (the AC10 grid, a DNG and a
//! JPEG's Exif TIFF blob, in both byte orders), at bases 8 and 1000: the block
//! is placed by the test placer (`tests/support/exif_block_place.rs`), ExifTool
//! reads the source and the placed file for the three block IFDs
//! (`-a -G1 -s -n -H -Warning -ExifIFD:All -GPS:All -InteropIFD:All`), and
//! the two must agree, less the entries in `omissions()`. The placed file must
//! print no warning the source did not print.
//!
//! The judge is ExifTool's reading of the builder's OUTPUT (the SPEC-037
//! lesson), not the library's own self-check. Nothing here is recorded or
//! red-proofed; that is SPEC-026. Live only: without ExifTool the test prints
//! `SKIP` and passes, unless `DIAMOND_EXIF_REQUIRE_ORACLE=1`. ExifTool is run,
//! never read or linked.
//!
//! One shape's source is never given to ExifTool (a SubIFD cycle it recurses on
//! without bound); see [`SOURCE_NOT_RUN`].

#[path = "support/exif_block_place.rs"]
mod exif_block_place;
#[path = "support/exif_block_shapes.rs"]
mod exif_block_shapes;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/real_shapes.rs"]
mod real_shapes;
#[path = "support/strip_shapes.rs"]
mod strip_shapes;
#[path = "support/sweep.rs"]
mod sweep;
#[path = "support/tiff_shapes.rs"]
mod tiff_shapes;
#[path = "support/tools.rs"]
mod tools;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use exif_block_place::place;
use exif_block_shapes::{grid, order_shapes, Expect, Shape};
use exif_core::reader::ByteOrder;
use exif_core::write::{exif_block, BlockIfd, BlockOptions};
use sweep::{live_gate, Scratch};

type Pin = (&'static str, &'static str, &'static str, &'static str);

/// One shape: index, name, the files written for it, the omitted entries.
struct Case {
    n: usize,
    name: String,
    files: Vec<(String, Vec<u8>)>,
    omitted: BTreeSet<(String, String)>,
}

/// Shapes whose SOURCE ExifTool is never run on, with the entries the placed
/// file must print instead (`group`, `id`, `name`, `value`, typed from the
/// shape). `ok-subifd-loop` has IFD0's SubIFD lead to an IFD whose SubIFD leads
/// back to IFD0; ExifTool recurses without bound on a SubIFD cycle (the same
/// hazard as `two-hop-cycle`, see `tests/support/tools.rs`: it did not return
/// in 60 s here), so only the placed files, which hold no cycle, are read.
const SOURCE_NOT_RUN: [(&str, &[Pin]); 1] =
    [("ok-subifd-loop/", &[("ExifIFD", "0x8827", "ISO", "1")])];

fn source_not_run(name: &str) -> Option<Vec<Line>> {
    SOURCE_NOT_RUN
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
        .map(|(_, lines)| lines.iter().map(line_of).collect())
}

/// Where ExifTool's reading of the SOURCE and of the placed block differ for a
/// reason that is the point of the shape, not a fault: per shape (name prefix),
/// the lines the source prints from IFDs the block does not hold (the builder
/// selects the Exif and GPS IFDs whose parent is IFD0, DEC-014; ExifTool also
/// prints an Exif IFD found through a SubIFD or IFD1, an Interop IFD pointed to
/// from IFD0, a GPS IFD inside a SubIFD), and the lines only the placed file
/// prints (ExifTool drops a whole IFD when its entry 0 has a bad format, an
/// unknown type, so the source prints nothing from it; the block drops only the
/// unknown-type entries and keeps the rest, which are the source's own. An
/// unknown type in any later entry costs ExifTool just that entry, so the
/// `unknown-type-not-first/` shape has no pin: its parity is exact). Each
/// `source_only` line must be present; a line that is not is a failure.
struct Diff {
    shape: &'static str,
    source_only: &'static [Pin],
    placed_only: &'static [Pin],
}

const DIFFS: [Diff; 7] = [
    Diff {
        shape: "exif-only-in-sub/",
        source_only: &[("ExifIFD", "0x8827", "ISO", "1")],
        placed_only: &[],
    },
    Diff {
        shape: "exif-only-under-ifd1/",
        source_only: &[("ExifIFD", "0x8827", "ISO", "1")],
        placed_only: &[],
    },
    Diff {
        shape: "ifd0-and-ifd1-exif/",
        source_only: &[("ExifIFD", "0x8827", "ISO", "99")],
        placed_only: &[],
    },
    Diff {
        shape: "a005-in-ifd0/",
        source_only: &[("InteropIFD", "0x0001", "InteropIndex", "R98")],
        placed_only: &[],
    },
    Diff {
        shape: "subifds-inside-exif/",
        source_only: &[("ExifIFD", "0x8827", "ISO", "2")],
        placed_only: &[],
    },
    Diff {
        shape: "exif-in-subifd-walked-first/",
        source_only: &[
            ("ExifIFD", "0x9003", "DateTimeOriginal", "SUB-EXIF"),
            ("GPS", "0x0001", "GPSLatitudeRef", "S"),
        ],
        placed_only: &[],
    },
    Diff {
        shape: "unknown-types/",
        source_only: &[],
        placed_only: &[
            ("ExifIFD", "0x8827", "ISO", "1"),
            ("GPS", "0x0001", "GPSLatitudeRef", "N"),
            ("InteropIFD", "0x0001", "InteropIndex", "R98"),
        ],
    },
];

/// The DNG's GPS IFD sits inside the raw SubIFD, so the block does not hold it
/// (AC10 shape 2): ExifTool prints these three lines for the source only.
const DNG_SUBIFD_GPS: [Pin; 3] = [
    ("GPS", "0x0001", "GPSLatitudeRef", "N"),
    ("GPS", "0x0002", "GPSLatitude", "52.5041666666667"),
    (
        "GPS",
        "0x001c",
        "GPSAreaInformation",
        "CANARY-SUBIFD-GPS-AREA...",
    ),
];

fn line_of(p: &Pin) -> Line {
    (
        p.0.to_string(),
        p.1.to_string(),
        p.2.to_string(),
        p.3.to_string(),
    )
}

/// `kept` (the source's lines less the omissions) adjusted by the shape's
/// pinned difference.
fn apply_diffs(name: &str, mut kept: Vec<Line>) -> Result<Vec<Line>, String> {
    let (source_only, placed_only): (&[Pin], &[Pin]) = if name.starts_with("dng/") {
        (&DNG_SUBIFD_GPS, &[])
    } else if let Some(d) = DIFFS.iter().find(|d| name.starts_with(d.shape)) {
        (d.source_only, d.placed_only)
    } else {
        (&[], &[])
    };
    if !placed_only.is_empty() && !kept.is_empty() {
        return Err(format!("the source prints {kept:?}, expected nothing"));
    }
    for pin in source_only {
        let line = line_of(pin);
        let at = kept
            .iter()
            .position(|l| *l == line)
            .ok_or_else(|| format!("the source does not print the pinned line {line:?}"))?;
        kept.remove(at);
    }
    kept.extend(placed_only.iter().map(line_of));
    Ok(kept)
}

fn all_shapes() -> Vec<Shape> {
    let mut v = grid();
    for be in [false, true] {
        let tag = if be { "be" } else { "le" };
        v.push(Shape {
            name: format!("dng/{tag}"),
            bytes: tiff_shapes::dng(be).bytes,
            expect: Expect::Block,
        });
        v.push(Shape {
            name: format!("jpeg-exif-blob/{tag}"),
            bytes: strip_shapes::full_tiff(be),
            expect: Expect::Block,
        });
    }
    v
}

/// One printed entry: group, tag id (`0x....`, or `-` for a warning), name,
/// value.
type Line = (String, String, String, String);

/// `-a -G1 -s -n -H` lines by file name: `[Group] 0xID Name : value`.
fn listing(stdout: &str) -> BTreeMap<String, Vec<Line>> {
    let mut out: BTreeMap<String, Vec<Line>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("======== ") {
            let name = path.rsplit('/').next().unwrap_or(path).to_string();
            out.entry(name.clone()).or_default();
            current = Some(name);
            continue;
        }
        let (Some(name), Some(rest)) = (&current, line.strip_prefix('[')) else {
            continue;
        };
        let Some((group, rest)) = rest.split_once(']') else {
            continue;
        };
        let Some((left, value)) = rest.split_once(" : ") else {
            continue;
        };
        let mut words = left.split_whitespace();
        let (Some(id), Some(tag)) = (words.next(), words.next()) else {
            continue;
        };
        out.entry(name.clone()).or_default().push((
            group.to_string(),
            id.to_string(),
            tag.to_string(),
            value.to_string(),
        ));
    }
    out
}

fn group_of(i: BlockIfd) -> &'static str {
    match i {
        BlockIfd::Exif => "ExifIFD",
        BlockIfd::Interop => "InteropIFD",
        BlockIfd::Gps => "GPS",
        _ => "?",
    }
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

fn sorted(mut v: Vec<Line>) -> Vec<Line> {
    v.sort();
    v
}

#[test]
fn placed_block_reads_as_source_in_exiftool() {
    live("placed_block_reads_as_source_in_exiftool", false);
}

/// SPEC-041, AC9(a): the same judge over the block built in the OTHER byte
/// order, both directions (every shape is built in both source orders, so
/// asking for the opposite of each source's own order covers a big-endian
/// source to little-endian and the reverse). The omissions now include the
/// `ByteOrder` ones; a value that converted must read as the source's.
#[test]
fn placed_cross_order_block_reads_as_source_in_exiftool() {
    live("placed_cross_order_block_reads_as_source_in_exiftool", true);
}

fn live(test: &str, cross: bool) {
    if !live_gate(test) {
        return;
    }
    let scratch = Scratch::new(if cross {
        "block-live-cross"
    } else {
        "block-live"
    });
    // One directory, one ExifTool run: `<n>_src`, `<n>_b8`, `<n>_b1000`.
    let mut cases: Vec<Case> = Vec::new();
    let mut shapes = all_shapes();
    if cross {
        shapes.extend(order_shapes());
    }
    let mut converted = 0usize;
    for s in shapes {
        if s.expect != Expect::Block {
            continue;
        }
        let own = exif_block(&s.bytes, &BlockOptions::new())
            .unwrap_or_else(|e| panic!("{}: expected a block, got {e}", s.name));
        let plan = if cross {
            let other = match own.byte_order() {
                ByteOrder::Big => ByteOrder::Little,
                _ => ByteOrder::Big,
            };
            let p = exif_block(&s.bytes, &BlockOptions::new().with_byte_order(other))
                .unwrap_or_else(|e| panic!("{} in {other:?}: {e}", s.name));
            assert_ne!(p.byte_order(), p.source_byte_order(), "{}", s.name);
            converted += usize::from(!p.is_empty());
            p
        } else {
            own
        };
        let be = plan.byte_order() == ByteOrder::Big;
        let n = cases.len();
        let mut files = Vec::new();
        if source_not_run(&s.name).is_none() {
            files.push((format!("{n:04}_src.tif"), s.bytes.clone()));
        }
        for base in [8u32, 1000] {
            let block = plan
                .write(base)
                .unwrap_or_else(|e| panic!("{} at {base}: {e}", s.name));
            files.push((
                format!("{n:04}_b{base}.tif"),
                place(be, base, &block, plan.exif_position(), plan.gps_position()),
            ));
        }
        // The omitted entries, as ExifTool names them: (group, id).
        let omitted: BTreeSet<(String, String)> = plan
            .omissions()
            .iter()
            .map(|o| (group_of(o.ifd()).to_string(), format!("0x{:04x}", o.tag())))
            .collect();
        for (name, bytes) in &files {
            std::fs::write(scratch.0.join(name), bytes).expect("write file");
        }
        cases.push(Case {
            n,
            name: s.name.clone(),
            files,
            omitted,
        });
    }
    assert!(cases.len() > 200, "{} shapes", cases.len());

    // One ExifTool run per shape (its three files), spread over a few threads:
    // a hostile source must not be able to take the rest of the batch with it.
    let selectors = args(&[
        "-a",
        "-G1",
        "-s",
        "-n",
        "-H",
        "-Warning",
        "-ExifIFD:All",
        "-GPS:All",
        "-InteropIFD:All",
    ]);
    let workers = std::thread::available_parallelism().map_or(2, |n| n.get().min(8));
    let results: Vec<(usize, Result<String, String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|w| {
                let (cases, selectors, dir) = (&cases, &selectors, &scratch.0);
                scope.spawn(move || {
                    cases
                        .iter()
                        .skip(w)
                        .step_by(workers)
                        .map(|Case { n, files, .. }| {
                            let mut a = selectors.clone();
                            let paths: Vec<_> =
                                files.iter().map(|(name, _)| dir.join(name)).collect();
                            let Some((last, rest)) = paths.split_last() else {
                                return (*n, Err("no files".to_string()));
                            };
                            a.extend(rest.iter().map(|p| p.display().to_string()));
                            let r = tools::run_tool_with_limit(
                                "exiftool",
                                &a,
                                last,
                                Duration::from_secs(60),
                            )
                            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                            .map_err(|e| format!("{e:?}"));
                            (*n, r)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("worker"))
            .collect()
    });
    let mut listed: BTreeMap<String, Vec<Line>> = BTreeMap::new();
    let mut run_failures: Vec<String> = Vec::new();
    for (n, r) in results {
        match r {
            Ok(text) => listed.extend(listing(&text)),
            Err(e) => run_failures.push(format!("shape {n} {}: exiftool: {e}", cases[n].name)),
        }
    }
    assert!(run_failures.is_empty(), "{run_failures:#?}");

    let mut failures: Vec<String> = Vec::new();
    let (mut compared, mut with_entries) = (0usize, 0usize);
    for Case {
        n, name, omitted, ..
    } in &cases
    {
        let read = |suffix: &str| -> Vec<Line> {
            listed
                .get(&format!("{n:04}_{suffix}.tif"))
                .cloned()
                .unwrap_or_else(|| panic!("no listing for {n:04}_{suffix}.tif"))
        };
        let is_warning = |l: &Line| l.2 == "Warning";
        let pinned = source_not_run(name);
        let source = if pinned.is_some() {
            Vec::new()
        } else {
            read("src")
        };
        let want: Vec<Line> = match pinned {
            Some(lines) => sorted(lines),
            None => {
                let kept: Vec<Line> = source
                    .iter()
                    .filter(|l| !is_warning(l))
                    .filter(|l| !omitted.contains(&(l.0.clone(), l.1.clone())))
                    .cloned()
                    .collect();
                match apply_diffs(name, kept) {
                    Ok(v) => sorted(v),
                    Err(e) => {
                        failures.push(format!("{name}: {e}"));
                        continue;
                    }
                }
            }
        };
        let source_warnings: BTreeSet<String> = source
            .iter()
            .filter(|l| is_warning(l))
            .map(|l| l.3.clone())
            .collect();
        for base in ["b8", "b1000"] {
            let placed = read(base);
            let got: Vec<Line> =
                sorted(placed.iter().filter(|l| !is_warning(l)).cloned().collect());
            compared += 1;
            with_entries += usize::from(!got.is_empty());
            if got != want {
                failures.push(format!("{name} {base}: entries differ\n  source less omissions {want:?}\n  placed {got:?}"));
            }
            for w in placed.iter().filter(|l| is_warning(l)) {
                if !source_warnings.contains(&w.3) {
                    failures.push(format!("{name} {base}: new warning {:?}", w.3));
                }
            }
        }
    }
    println!(
        "{} shapes, {} placed files compared with ExifTool, {} with at least one entry",
        cases.len(),
        compared,
        with_entries
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{:#?}",
        failures.len(),
        failures
    );
    assert!(with_entries > 100, "{with_entries} files printed an entry");
    if cross {
        println!("{converted} non-empty cross-order blocks");
        assert!(converted > 200, "{converted} non-empty cross-order blocks");
    }
}
