//! Records live ExifTool output for the parity oracle (SPEC-005).
//!
//! `just oracle-record` (or `cargo run --example oracle_record`) runs the
//! installed ExifTool over `fixtures()` and the committed `tiff_ifd` seeds and
//! writes `tests/oracle-fixtures/exiftool/<name>.txt`: a `#` header with the
//! exact command line and ExifTool's version, then its stdout verbatim. This is
//! a recorder, not a test; it is never run in CI. ExifTool is only run, never
//! read. std only; examples are outside the library lint set.

#[path = "../tests/support/oracle.rs"]
mod oracle;
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
    println!(
        "oracle-record: wrote {written} records to {}",
        out_dir.display()
    );
}
