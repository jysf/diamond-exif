//! Records live ExifTool output for the parity oracle (SPEC-005).
//!
//! `just oracle-record` (or `cargo run --example oracle_record`) runs the
//! installed ExifTool over `fixtures()` and the committed `tiff_ifd` seeds and
//! writes `tests/oracle-fixtures/exiftool/<name>.txt`: a `#` header with the
//! exact command line and ExifTool's version, then its stdout verbatim. This is
//! a recorder, not a test; it is never run in CI. ExifTool is only run, never
//! read. std only; examples are outside the library lint set.
//!
//! SPEC-011 adds the JPEG records: `jpeg-<fixture>.txt` for every
//! `oracle_jpeg::fixtures()` entry (the 20 Exif selectors and the two region
//! presence selectors) and `jpeg-seed-<seed>.txt` for every committed
//! `jpeg_segments` seed (the 20 Exif selectors). JPEG records also carry one
//! `# stderr:` line per ExifTool warning, because for JPEG the warning is part
//! of the signal; the TIFF records are written exactly as before (the CI job
//! `oracle-records` re-records and diffs them, and the TIFF records must not
//! move).
//!
//! SPEC-031 adds the PNG records: `png-<fixture>.txt` for every
//! `oracle_png::fixtures()` entry (the 20 Exif selectors and the two PNG region
//! selectors) and `png-seed-<seed>.txt` for every committed `png_chunks` seed
//! (the 20 Exif selectors). Like the JPEG records they carry `# stderr:` lines,
//! restricted to ExifTool's `Warning:`/`Error:` lines.
//!
//! SPEC-013 adds the inventory records, one per corpus, in
//! `tests/oracle-fixtures/exiftool-inventory/`: `exiftool -a -u -G1 -H -s` over
//! every input of the hand-built fixtures, the DEC-008 one-entry TIFFs and the
//! three seed directories, as `group<TAB>id<TAB>name` lines (file-level groups
//! removed). ExifTool is run once per distinct input (by content); an input it
//! does not return on is recorded as not run.
//!
//! SPEC-036 adds the `png-webp` inventory corpus (every PNG and WebP oracle
//! fixture, shape and committed seed, `png-webp.txt`), recorded like
//! `real-shapes`: its own record, each input run once, alone, as a `.png` or
//! `.webp` file, so the five older records do not move.

#[path = "../tests/support/audit_tags.rs"]
mod audit_tags;
#[path = "../tests/support/exif_layout.rs"]
mod exif_layout;
#[path = "../tests/support/exif_records.rs"]
mod exif_records;
#[path = "../tests/support/hostile.rs"]
mod hostile;
#[path = "../tests/support/i3_records.rs"]
mod i3_records;
#[path = "../tests/support/i3_shapes.rs"]
mod i3_shapes;
#[path = "../tests/support/inventory.rs"]
mod inventory;
#[path = "../tests/support/inventory_corpus.rs"]
mod inventory_corpus;
#[path = "../tests/support/oracle.rs"]
mod oracle;
#[path = "../tests/support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "../tests/support/oracle_png.rs"]
mod oracle_png;
#[path = "../tests/support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "../tests/support/oracle_webp.rs"]
mod oracle_webp;
#[path = "../tests/support/png_shapes.rs"]
mod png_shapes;
#[path = "../tests/support/real_shapes.rs"]
mod real_shapes;
#[path = "../tests/support/strip_records.rs"]
mod strip_records;
#[path = "../tests/support/strip_shapes.rs"]
mod strip_shapes;
#[path = "../tests/support/tiff_records.rs"]
mod tiff_records;
#[path = "../tests/support/tiff_shapes.rs"]
mod tiff_shapes;
#[path = "../tests/support/tools.rs"]
mod tools;
#[path = "../tests/support/webp_carriers.rs"]
mod webp_carriers;
#[path = "../tests/support/webp_shapes.rs"]
mod webp_shapes;

use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("tests/oracle-fixtures/exiftool");
    fs::create_dir_all(&out_dir).expect("create record dir");

    let version = match tools::exiftool_version() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("oracle-record: cannot run exiftool: {e}");
            std::process::exit(1);
        }
    };
    println!("oracle-record: exiftool {version}");

    let mut written = 0usize;
    for (name, bytes) in oracle_tiff::fixtures() {
        let path = tools::temp_file_for(name, &bytes).expect("write temp fixture");
        let names = oracle::names(oracle::SELECTORS);
        let stdout = tools::run_exiftool_raw(&path, &names).expect("run exiftool");
        let _ = fs::remove_file(&path);
        let text = tools::record_text(&names, &version, &format!("fixture {name}"), &stdout);
        fs::write(out_dir.join(format!("{name}.txt")), text).expect("write record");
        written += 1;
    }

    let seed_dir = root.join("fuzz/seeds/tiff_ifd");
    let mut seeds: Vec<PathBuf> = fs::read_dir(&seed_dir)
        .expect("read seeds")
        .map(|e| e.expect("dir entry").path())
        .collect();
    seeds.sort();
    for path in seeds {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let names = oracle::names(oracle::SEED_SELECTORS);
        let source = format!("seed {name}");
        let text = if let Some(d) = oracle::not_run_divergence(&name) {
            // ExifTool does not return on this input (about 9 GB in 5 s): not run.
            println!("oracle-record: NOT RUN on {name}: {}", d.question_id);
            tools::record_text_not_run(&names, &version, &source, &oracle::not_run_outcome(d))
        } else {
            let stdout = tools::run_exiftool_raw(&path, &names).expect("run exiftool");
            tools::record_text(&names, &version, &source, &stdout)
        };
        fs::write(out_dir.join(format!("{name}.txt")), text).expect("write record");
        written += 1;
    }
    println!("oracle-record: wrote {written} TIFF records");

    let mut jpeg_written = 0usize;
    for (name, bytes) in oracle_jpeg::fixtures() {
        let names = oracle::jpeg_names();
        let source = format!("fixture {name}");
        let text = if let Some(d) = oracle::not_run_divergence(&name) {
            println!("oracle-record: NOT RUN on {name}: {}", d.question_id);
            tools::record_text_jpeg_not_run(&names, &version, &source, &oracle::not_run_outcome(d))
        } else {
            let path = tools::temp_file_with_ext(&name, "jpg", &bytes).expect("write temp fixture");
            let run = tools::run_exiftool_full(&path, &names);
            let _ = fs::remove_file(&path);
            let (stdout, stderr) = run.expect("run exiftool");
            let warnings = tools::warning_lines(&stderr, &path);
            tools::record_text_jpeg(&names, &version, &source, &warnings, &stdout)
        };
        fs::write(out_dir.join(format!("{name}.txt")), text).expect("write record");
        jpeg_written += 1;
    }

    let jseed_dir = root.join("fuzz/seeds/jpeg_segments");
    let mut jseeds: Vec<PathBuf> = fs::read_dir(&jseed_dir)
        .expect("read jpeg seeds")
        .map(|e| e.expect("dir entry").path())
        .collect();
    jseeds.sort();
    for path in jseeds {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        // SPEC-019: the `shape-*` seeds are recorded in the real-shapes corpus.
        if name.starts_with(inventory_corpus::SHAPE_SEED_PREFIX) {
            continue;
        }
        let record = format!("jpeg-seed-{name}");
        let names = oracle::names(oracle::SELECTORS);
        let source = format!("seed jpeg_segments/{name}");
        let text = if let Some(d) = oracle::not_run_divergence(&record) {
            println!("oracle-record: NOT RUN on {record}: {}", d.question_id);
            tools::record_text_jpeg_not_run(&names, &version, &source, &oracle::not_run_outcome(d))
        } else {
            // The seed is read from the committed file itself, not a copy.
            let (stdout, stderr) = tools::run_exiftool_full(&path, &names).expect("run exiftool");
            let warnings = tools::warning_lines(&stderr, &path);
            tools::record_text_jpeg(&names, &version, &source, &warnings, &stdout)
        };
        fs::write(out_dir.join(format!("{record}.txt")), text).expect("write record");
        jpeg_written += 1;
    }
    println!(
        "oracle-record: wrote {jpeg_written} JPEG records; {} files in {}",
        written + jpeg_written,
        out_dir.display()
    );

    record_png(&root, &out_dir, &version);
    record_webp(&root, &out_dir, &version);

    // SPEC-012: the names ExifTool gives the DEC-008 tag ids (one record, one
    // line per named tag). Run on tiny valid one-entry TIFFs, each alone under
    // the time limit.
    let mut measured = Vec::new();
    for row in audit_tags::rows() {
        let names = audit_tags::measure(&row).expect("run exiftool");
        measured.push((row.id, names));
    }
    let text = audit_tags::record_text(&version, &measured);
    fs::write(root.join(audit_tags::RECORD), text).expect("write audit tag-name record");
    println!(
        "oracle-record: wrote the audit tag-name record ({} ids)",
        measured.len()
    );

    // SPEC-017: the 511-SubIFD overlapping shape. ExifTool 13.55 ignores all
    // but the first 10 SubIFD directories and prints a warning; the record
    // keeps that warning (`# stderr:`) and the dashes. Probed alone under the
    // time limit before it joined this recorder (0.04 s, about 22 MB).
    let bytes = hostile::hostile(hostile::N_SUB);
    let path = tools::temp_file_for("hostile-subifds-overlap", &bytes).expect("write temp file");
    let names = oracle::names(oracle::SEED_SELECTORS);
    let run = tools::run_exiftool_full(&path, &names);
    let _ = fs::remove_file(&path);
    let (stdout, stderr) = run.expect("run exiftool");
    let warnings = tools::warning_lines(&stderr, &path);
    let text = tools::record_text_jpeg(
        &names,
        &version,
        "hostile hostile-subifds-overlap",
        &warnings,
        &stdout,
    );
    fs::write(root.join(hostile::RECORD), text).expect("write hostile record");
    println!("oracle-record: wrote the hostile-subifds-overlap record");

    record_inventory(&root, &version);
    record_strip(&root, &version);
    record_strip_tiff(&root, &version);
    record_strip_png_webp(&root, &version);
}

/// SPEC-039 (DEC-023): the PNG and WebP write-parity records. For every I3
/// fixture, ExifTool (`-a -u -G1 -s -t -n`) on the input, on the library's
/// output under each recorded policy that strips (with the output's length
/// and digest), and on ExifTool's own `-all=` and `-gps:all=` outputs, into
/// `tests/oracle-fixtures/exiftool-strip-{png,webp}/<fixture>.txt`.
fn record_strip_png_webp(root: &std::path::Path, version: &str) {
    let fixtures = i3_shapes::fixtures();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let records = i3_records::record_all(&fixtures, version, threads);
    for ext in ["png", "webp"] {
        fs::create_dir_all(root.join(i3_records::record_dir(ext)))
            .expect("create strip png/webp record dir");
    }
    for (name, ext, text) in &records {
        let path = root
            .join(i3_records::record_dir(ext))
            .join(format!("{name}.txt"));
        fs::write(path, text).expect("write strip png/webp record");
    }
    println!(
        "oracle-record: wrote {} PNG and WebP strip records",
        records.len()
    );
}

/// SPEC-031: the PNG records. Every shape was probed alone under a wall-clock
/// limit before it joined the list (none failed to return, see the SPEC-031
/// handback); `run_exiftool_full` keeps the tool time limit on every run.
fn record_png(root: &std::path::Path, out_dir: &std::path::Path, version: &str) {
    let mut written = 0usize;
    for (name, bytes) in oracle_png::fixtures() {
        let names = oracle::png_names();
        let source = format!("fixture {name}");
        let text = if let Some(d) = oracle::not_run_divergence(&name) {
            println!("oracle-record: NOT RUN on {name}: {}", d.question_id);
            tools::record_text_jpeg_not_run(&names, version, &source, &oracle::not_run_outcome(d))
        } else {
            let path = tools::temp_file_with_ext(&name, "png", &bytes).expect("write temp fixture");
            let run = tools::run_exiftool_full(&path, &names);
            let _ = fs::remove_file(&path);
            let (stdout, stderr) = run.expect("run exiftool");
            let warnings = oracle::png_warning_lines(tools::warning_lines(&stderr, &path));
            tools::record_text_jpeg(&names, version, &source, &warnings, &stdout)
        };
        fs::write(out_dir.join(format!("{name}.txt")), text).expect("write record");
        written += 1;
    }
    let seed_dir = root.join("fuzz/seeds/png_chunks");
    let mut seeds: Vec<PathBuf> = fs::read_dir(&seed_dir)
        .expect("read png seeds")
        .map(|e| e.expect("dir entry").path())
        .collect();
    seeds.sort();
    for path in seeds {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let record = format!("png-seed-{name}");
        let names = oracle::names(oracle::SELECTORS);
        let source = format!("seed png_chunks/{name}");
        let text = if let Some(d) = oracle::not_run_divergence(&record) {
            println!("oracle-record: NOT RUN on {record}: {}", d.question_id);
            tools::record_text_jpeg_not_run(&names, version, &source, &oracle::not_run_outcome(d))
        } else {
            // The seed is read from the committed file itself, not a copy.
            let (stdout, stderr) = tools::run_exiftool_full(&path, &names).expect("run exiftool");
            let warnings = oracle::png_warning_lines(tools::warning_lines(&stderr, &path));
            tools::record_text_jpeg(&names, version, &source, &warnings, &stdout)
        };
        fs::write(out_dir.join(format!("{record}.txt")), text).expect("write record");
        written += 1;
    }
    println!("oracle-record: wrote {written} PNG records");

    // The warning record: ExifTool's own Warning and Error lines per fixture of
    // the table (`-a -G1 -s -n -Warning -Error`), one `## name` section each. The
    // stdout carries no path (only the tag values).
    let fixtures = oracle_png::fixtures();
    let mut text = format!(
        "# command: exiftool -a -G1 -s -n -Warning -Error <file>\n# exiftool-version: {version}\n\
         # source: fixtures {}\n",
        oracle_png::WARNING_FIXTURES.len()
    );
    for name in oracle_png::WARNING_FIXTURES {
        let bytes = &fixtures
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("no PNG fixture {name}"))
            .1;
        let path = tools::temp_file_with_ext(name, "png", bytes).expect("write temp fixture");
        let args: Vec<String> = ["-a", "-G1", "-s", "-n", "-Warning", "-Error"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let run = tools::run_tool("exiftool", &args, &path);
        let _ = fs::remove_file(&path);
        let out = run.expect("run exiftool");
        text.push_str(&format!("## {name}\n"));
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            text.push_str(line.trim_end());
            text.push('\n');
        }
    }
    fs::write(out_dir.join(oracle_png::WARNINGS_RECORD), text).expect("write warnings record");
    println!("oracle-record: wrote the PNG warnings record");

    // SPEC-032: the text-keyword record. `-a -G1 -s -n` over the keyword fixture,
    // the `[PNG]` group only (the other groups are not about keywords). Each
    // line is `[PNG] <tag name> : <value>`; the value is `v:<keyword>`.
    let bytes = oracle_png::text_keywords_png();
    let path = tools::temp_file_with_ext(oracle_png::TEXT_KEYWORDS_NAME, "png", &bytes)
        .expect("write temp fixture");
    let args: Vec<String> = ["-a", "-G1", "-s", "-n"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let run = tools::run_tool("exiftool", &args, &path);
    let _ = fs::remove_file(&path);
    let out = run.expect("run exiftool");
    let mut text = format!(
        "# command: exiftool -a -G1 -s -n <file> (the [PNG] group only)\n# exiftool-version: {version}\n\
         # source: fixture {}\n",
        oracle_png::TEXT_KEYWORDS_NAME
    );
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if line.starts_with("[PNG] ") {
            text.push_str(line.trim_end());
            text.push('\n');
        }
    }
    fs::write(out_dir.join(oracle_png::TEXT_KEYWORDS_RECORD), text)
        .expect("write text-keywords record");
    println!("oracle-record: wrote the PNG text-keywords record");
}

/// SPEC-034: the WebP records, shaped like [`record_png`]. Every shape was
/// probed alone under a wall-clock limit before it joined the list; the files
/// get a `.webp` extension (ExifTool chooses its reader partly by extension, and
/// the `RIFF....WAVE` shape is measured under it). Seeds are recorded for the
/// 20 Exif selectors only, as the PNG seeds are.
fn record_webp(root: &std::path::Path, out_dir: &std::path::Path, version: &str) {
    let mut written = 0usize;
    for (name, bytes) in oracle_webp::fixtures() {
        let names = oracle::webp_names();
        let source = format!("fixture {name}");
        let path = tools::temp_file_with_ext(&name, "webp", &bytes).expect("write temp fixture");
        let run = tools::run_exiftool_full(&path, &names);
        let _ = fs::remove_file(&path);
        let (stdout, stderr) = run.expect("run exiftool");
        let warnings = oracle::webp_warning_lines(tools::warning_lines(&stderr, &path));
        let text = tools::record_text_jpeg(&names, version, &source, &warnings, &stdout);
        fs::write(out_dir.join(format!("{name}.txt")), text).expect("write record");
        written += 1;
    }
    let seed_dir = root.join("fuzz/seeds/webp_riff");
    let mut seeds: Vec<PathBuf> = fs::read_dir(&seed_dir)
        .expect("read webp seeds")
        .map(|e| e.expect("dir entry").path())
        .collect();
    seeds.sort();
    for path in seeds {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let record = format!("webp-seed-{name}");
        let names = oracle::names(oracle::SELECTORS);
        let source = format!("seed webp_riff/{name}");
        // The seed is copied to a `.webp` file: the committed file has no
        // extension, and ExifTool's choice of reader depends on it.
        let bytes = fs::read(&path).expect("read seed");
        let tmp = tools::temp_file_with_ext(&record, "webp", &bytes).expect("write temp seed");
        let run = tools::run_exiftool_full(&tmp, &names);
        let _ = fs::remove_file(&tmp);
        let (stdout, stderr) = run.expect("run exiftool");
        let warnings = oracle::webp_warning_lines(tools::warning_lines(&stderr, &tmp));
        let text = tools::record_text_jpeg(&names, version, &source, &warnings, &stdout);
        fs::write(out_dir.join(format!("{record}.txt")), text).expect("write record");
        written += 1;
    }
    println!("oracle-record: wrote {written} WebP records");

    // The warning record, as for PNG: `-a -G1 -s -n -Warning -Error` per fixture
    // of the table, one `## name` section each.
    let fixtures = oracle_webp::fixtures();
    let mut text = format!(
        "# command: exiftool -a -G1 -s -n -Warning -Error <file>\n# exiftool-version: {version}\n\
         # source: fixtures {}\n",
        oracle_webp::WARNING_FIXTURES.len()
    );
    for name in oracle_webp::WARNING_FIXTURES {
        let bytes = &fixtures
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("no WebP fixture {name}"))
            .1;
        let path = tools::temp_file_with_ext(name, "webp", bytes).expect("write temp fixture");
        let args: Vec<String> = ["-a", "-G1", "-s", "-n", "-Warning", "-Error"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let run = tools::run_tool("exiftool", &args, &path);
        let _ = fs::remove_file(&path);
        let out = run.expect("run exiftool");
        text.push_str(&format!("## {name}\n"));
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            text.push_str(line.trim_end());
            text.push('\n');
        }
    }
    fs::write(out_dir.join(oracle_webp::WARNINGS_RECORD), text).expect("write warnings record");
    println!("oracle-record: wrote the WebP warnings record");
}

/// SPEC-022: the same for the TIFF and DNG fixtures, into
/// `tests/oracle-fixtures/exiftool-strip-tiff/<fixture>.txt`.
fn record_strip_tiff(root: &std::path::Path, version: &str) {
    let dir = root.join("tests/oracle-fixtures/exiftool-strip-tiff");
    fs::create_dir_all(&dir).expect("create strip-tiff record dir");
    let mut n = 0;
    for s in tiff_shapes::all() {
        let text =
            tiff_records::record_text(&s, version).unwrap_or_else(|e| panic!("{}: {e}", s.name));
        fs::write(dir.join(format!("{}.txt", s.name)), text).expect("write strip-tiff record");
        n += 1;
    }
    println!("oracle-record: wrote {n} strip-tiff records");
}

/// SPEC-020: for every strip fixture, ExifTool (`-a -u -G1 -s -t -n`) on the
/// input and on the library's output under each recorded policy that strips,
/// in parallel (one thread per fixture), into
/// `tests/oracle-fixtures/exiftool-strip/<fixture>.txt`.
fn record_strip(root: &std::path::Path, version: &str) {
    let dir = root.join("tests/oracle-fixtures/exiftool-strip");
    fs::create_dir_all(&dir).expect("create strip record dir");
    let shapes: Vec<_> = strip_shapes::all()
        .into_iter()
        .filter(|s| !s.refused)
        .collect();
    let texts: Vec<(&str, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = shapes
            .iter()
            .map(|s| {
                scope.spawn(move || {
                    let text = strip_records::record_text(s, version)
                        .unwrap_or_else(|e| panic!("{}: {e}", s.name));
                    (s.name, text)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread"))
            .collect()
    });
    for (name, text) in &texts {
        fs::write(dir.join(format!("{name}.txt")), text).expect("write strip record");
    }
    println!("oracle-record: wrote {} strip records", texts.len());

    // SPEC-021: the Exif entry fixtures, one file per fixture
    // (`exif-<fixture>.txt`), the input and every policy of the grid that
    // strips.
    let exif: Vec<(&str, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = exif_records::exif_fixtures()
            .into_iter()
            .map(|(name, layouts)| {
                scope.spawn(move || {
                    let jpg = exif_records::jpeg_of(&layouts, false);
                    let text = strip_records::exif_record_text(name, &jpg, version)
                        .unwrap_or_else(|e| panic!("{name}: {e}"));
                    (name, text)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread"))
            .collect()
    });
    for (name, text) in &exif {
        fs::write(dir.join(format!("exif-{name}.txt")), text).expect("write exif record");
    }
    println!("oracle-record: wrote {} exif entry records", exif.len());
}

/// SPEC-013: the inventory records (`exiftool -a -u -G1 -H -s`), one file per
/// corpus. Each distinct input (by content) is run once, alone, under the tool
/// time limit; an input in `inventory_corpus::NOT_RUN_KEYS` is never run.
fn record_inventory(root: &std::path::Path, version: &str) {
    use std::collections::BTreeMap;
    let dir = root.join(inventory_corpus::RECORD_DIR);
    fs::create_dir_all(&dir).expect("create inventory record dir");
    let all = inventory_corpus::all_inputs(root);
    let distinct = inventory_corpus::distinct(&all);

    // bytes -> (outcome, stderr, kept lines)
    type Answer = (Option<String>, Vec<String>, Vec<inventory::Line>);
    let mut answers: BTreeMap<Vec<u8>, Answer> = BTreeMap::new();
    for d in &distinct {
        let input = &d.canonical;
        if inventory_corpus::is_not_run(d) {
            println!("oracle-record: NOT RUN on {}", input.key());
            answers.insert(
                input.bytes.clone(),
                (
                    Some(inventory_corpus::NOT_RUN_OUTCOME.to_string()),
                    Vec::new(),
                    Vec::new(),
                ),
            );
            continue;
        }
        let path = tools::temp_file_with_ext("inventory", input.ext(), &input.bytes)
            .expect("write temp fixture");
        let args: Vec<String> = inventory::COMMAND_ARGS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let run = tools::run_tool("exiftool", &args, &path);
        let _ = fs::remove_file(&path);
        let out = run.unwrap_or_else(|e| panic!("exiftool on {}: {e}", input.key()));
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let lines =
            inventory::parse_stdout(&stdout).unwrap_or_else(|e| panic!("{}: {e}", input.key()));
        let warnings = tools::warning_lines(&stderr, &path);
        answers.insert(input.bytes.clone(), (None, warnings, lines));
    }

    let mut sections = 0usize;
    for corpus in inventory_corpus::CORPORA {
        let mut text = inventory::record_header(version, corpus);
        for input in inventory_corpus::inputs(root, corpus) {
            let (outcome, stderr, lines) = answers.get(&input.bytes).expect("an answer per input");
            text.push_str(&inventory::section_text(
                &input.name,
                outcome.as_deref(),
                stderr,
                lines,
            ));
            sections += 1;
        }
        fs::write(dir.join(inventory_corpus::record_file(corpus)), text)
            .expect("write inventory record");
    }
    println!(
        "oracle-record: wrote {} inventory records ({} sections, {} distinct inputs)",
        inventory_corpus::CORPORA.len(),
        sections,
        distinct.len()
    );

    // SPEC-019: the real-shapes corpus. Its own record, so the five above are
    // untouched; each input is run once, alone, under the tool time limit.
    for corpus in inventory_corpus::NEW_CORPORA {
        let mut text = inventory::record_header(version, corpus);
        let mut count = 0usize;
        for input in inventory_corpus::inputs(root, corpus) {
            let path = tools::temp_file_with_ext("inventory", input.ext(), &input.bytes)
                .expect("write temp fixture");
            let args: Vec<String> = inventory::COMMAND_ARGS
                .iter()
                .map(|s| s.to_string())
                .collect();
            let run = tools::run_tool("exiftool", &args, &path);
            let _ = fs::remove_file(&path);
            let out = run.unwrap_or_else(|e| panic!("exiftool on {}: {e}", input.key()));
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            let lines =
                inventory::parse_stdout(&stdout).unwrap_or_else(|e| panic!("{}: {e}", input.key()));
            let warnings = tools::warning_lines(&stderr, &path);
            text.push_str(&inventory::section_text(
                &input.name,
                None,
                &warnings,
                &lines,
            ));
            count += 1;
        }
        fs::write(dir.join(inventory_corpus::record_file(corpus)), text)
            .expect("write inventory record");
        println!("oracle-record: wrote the {corpus} record ({count} sections)");
    }

    // SPEC-019: the Canon tag values of the absolute-offset MakerNote fixture
    // and of the same Exif with the note moved (`-T -n -s3`, the usual way).
    let all_shapes = real_shapes::fixtures();
    for name in real_shapes::CANON_RECORDED {
        let bytes = &all_shapes
            .iter()
            .find(|(n, _)| *n == name)
            .expect("a canon fixture")
            .1;
        let path = tools::temp_file_with_ext(name, "jpg", bytes).expect("write temp fixture");
        let run = tools::run_exiftool_full(&path, &real_shapes::CANON_SELECTORS);
        let _ = fs::remove_file(&path);
        let (stdout, stderr) = run.expect("run exiftool");
        let warnings = tools::warning_lines(&stderr, &path);
        let text = tools::record_text_jpeg(
            &real_shapes::CANON_SELECTORS,
            version,
            &format!("fixture {name}"),
            &warnings,
            &stdout,
        );
        fs::write(dir.join(real_shapes::canon_record_file(name)), text)
            .expect("write canon record");
    }
    println!("oracle-record: wrote the two Canon value records");
}
