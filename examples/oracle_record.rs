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

#[path = "../tests/support/audit_tags.rs"]
mod audit_tags;
#[path = "../tests/support/hostile.rs"]
mod hostile;
#[path = "../tests/support/oracle.rs"]
mod oracle;
#[path = "../tests/support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "../tests/support/oracle_tiff.rs"]
mod oracle_tiff;
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
}
