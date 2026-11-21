//! `exif-audit`: a read-only privacy scan of files and directories (DEC-011).
//!
//! `exif-audit [OPTIONS] [--] PATH...` prints one report (DEC-010) on stdout and
//! exits with a code a CI job can act on. `main` is thin: parse, loop, write
//! once, exit. Every decision lives in `args.rs` (the grammar and defaults),
//! `exit.rs` (the exit code), `walk.rs` and `read.rs` (the only `std::fs` code).
//! One input file is held in memory at a time: it is read, audited, added to
//! the incremental report builder and dropped.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

mod args;
mod exit;
mod read;
mod walk;

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use exif_core::audit::audit;
use exif_core::report::{FileEntry, FileResult, RenderOptions, ReportBuilder};

use args::{Options, Outcome};
use read::Loaded;
use walk::Item;

fn main() -> ExitCode {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    match args::parse(&argv) {
        // Help and version are not the report: a write error there is ignored.
        Outcome::Help => {
            let _ = write_stdout(args::USAGE.as_bytes());
            ExitCode::SUCCESS
        }
        Outcome::Version => {
            let text = format!("{}\n", version_line(option_env!("EXIF_AUDIT_BUILD_REF")));
            let _ = write_stdout(text.as_bytes());
            ExitCode::SUCCESS
        }
        Outcome::Usage(e) => {
            // Never `eprintln!`: it panics when stderr is closed.
            let msg = format!(
                "exif-audit: {}\nTry 'exif-audit --help' for usage.\n",
                e.message()
            );
            let _ = std::io::stderr().write_all(msg.as_bytes());
            ExitCode::from(exit::USAGE)
        }
        Outcome::Run(opts) => ExitCode::from(run(&opts)),
    }
}

/// The `--version` line (DEC-011): `exif-audit <crate version>`, plus ` (<ref>)`
/// when a build ref was injected at compile time (the release workflow sets
/// `EXIF_AUDIT_BUILD_REF` to the short commit SHA). An empty ref counts as none.
fn version_line(build_ref: Option<&str>) -> String {
    match build_ref {
        Some(r) if !r.is_empty() => format!("exif-audit {} ({r})", env!("CARGO_PKG_VERSION")),
        _ => format!("exif-audit {}", env!("CARGO_PKG_VERSION")),
    }
}

/// Scan, write the report once, and return the exit code.
fn run(opts: &Options) -> u8 {
    let render = RenderOptions {
        policy: opts.policy,
        inventory: opts.inventory,
        values: opts.values,
    };
    let mut builder = ReportBuilder::new(opts.format, render);
    for cand in walk::Walker::new(&opts.paths) {
        let label = cand.path.as_os_str().as_encoded_bytes();
        let loaded = match cand.item {
            Item::Failed(f) => Loaded::Failed(f),
            Item::File => read::read_file(&cand.path, opts.max_size),
        };
        match &loaded {
            Loaded::Whole(bytes) | Loaded::Head(bytes) => builder.add(&FileEntry {
                label,
                result: FileResult::Audited(audit(bytes)),
            }),
            Loaded::Failed(f) => builder.add(&FileEntry {
                label,
                result: FileResult::Failed {
                    reason: f.reason,
                    detail: f.detail.as_bytes(),
                },
            }),
        }
        // `loaded` (the file's bytes) is dropped here, before the next file.
    }
    let summary = builder.summary();
    let report = builder.finish();
    let write_failed = write_stdout(report.as_bytes()).is_err();
    exit::exit_code(&summary, opts, write_failed)
}

/// Write all of `bytes` to stdout and flush. A closed pipe or a full disk is an
/// error here (Rust ignores SIGPIPE, so it arrives as `BrokenPipe`).
fn write_stdout(bytes: &[u8]) -> std::io::Result<()> {
    let mut out = std::io::stdout().lock();
    out.write_all(bytes)?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::version_line;

    #[test]
    fn version_line_without_a_ref_is_the_crate_version() {
        assert_eq!(
            version_line(None),
            format!("exif-audit {}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn version_line_with_a_ref_appends_it_in_parentheses() {
        assert_eq!(
            version_line(Some("abc1234")),
            format!("exif-audit {} (abc1234)", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn version_line_treats_an_empty_ref_as_none() {
        assert_eq!(version_line(Some("")), version_line(None));
    }
}
