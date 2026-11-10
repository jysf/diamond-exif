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
//! SPEC-013 adds the inventory records, one per corpus, in
//! `tests/oracle-fixtures/exiftool-inventory/`: `exiftool -a -u -G1 -H -s` over
//! every input of the hand-built fixtures, the DEC-008 one-entry TIFFs and the
//! three seed directories, as `group<TAB>id<TAB>name` lines (file-level groups
//! removed). ExifTool is run once per distinct input (by content); an input it
//! does not return on is recorded as not run.

#[path = "../tests/support/audit_tags.rs"]
mod audit_tags;
#[path = "../tests/support/hostile.rs"]
mod hostile;
#[path = "../tests/support/inventory.rs"]
mod inventory;
#[path = "../tests/support/inventory_corpus.rs"]
mod inventory_corpus;
#[path = "../tests/support/oracle.rs"]
mod oracle;
#[path = "../tests/support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "../tests/support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "../tests/support/real_shapes.rs"]
mod real_shapes;
#[path = "../tests/support/tools.rs"]
mod tools;

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
