//! The exit code (DEC-011 part 2). Pure: a function of the run summary, the
//! options and whether the report could be written. Usage errors (2), `--help`
//! and `--version` (0) are decided in `main` from the argument outcome.

use exif_core::report::Summary;

use crate::args::Options;

/// A usage error.
pub const USAGE: u8 = 2;
/// The report could not be written.
pub const WRITE_FAILED: u8 = 4;
/// Incomplete: a file failed or was unreadable (with `--strict`, also
/// unsupported or with issues).
pub const INCOMPLETE: u8 = 3;
/// Complete, and a finding reaches the `--fail-on` threshold.
pub const FINDINGS: u8 = 1;
/// Complete, and nothing reaches the threshold.
pub const CLEAN: u8 = 0;

/// Exactly one code, chosen in this order: write failure, incomplete,
/// findings at or above the threshold, else 0.
pub fn exit_code(summary: &Summary, opts: &Options, write_failed: bool) -> u8 {
    if write_failed {
        return WRITE_FAILED;
    }
    let mut incomplete = summary.failed.saturating_add(summary.unreadable);
    if opts.strict {
        incomplete = incomplete
            .saturating_add(summary.unsupported)
            .saturating_add(summary.issues);
    }
    if incomplete > 0 {
        return INCOMPLETE;
    }
    // `Option` orders `None` below every `Some`, so a file-less or finding-less
    // run never reaches a threshold.
    let over = match (opts.fail_on, summary.max_severity) {
        (Some(threshold), Some(max)) => max >= threshold,
        _ => false,
    };
    if over {
        FINDINGS
    } else {
        CLEAN
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;
    use exif_core::audit::Severity;
    use exif_core::report::{summarize, RenderOptions};

    fn summary(
        failed: usize,
        unreadable: usize,
        unsupported: usize,
        issues: usize,
        sev: Option<Severity>,
    ) -> Summary {
        let mut s = summarize(&[], &RenderOptions::default());
        s.failed = failed;
        s.unreadable = unreadable;
        s.unsupported = unsupported;
        s.issues = issues;
        s.max_severity = sev;
        s
    }

    fn opts(strict: bool, fail_on: Option<Severity>) -> Options {
        Options {
            format: exif_core::report::Format::Text,
            policy: None,
            fail_on,
            strict,
            values: false,
            inventory: false,
            max_size: 1,
            paths: vec!["p".into()],
        }
    }

    const NONE: Option<Severity> = None;
    const LOW: Option<Severity> = Some(Severity::Low);
    const MED: Option<Severity> = Some(Severity::Medium);
    const HIGH: Option<Severity> = Some(Severity::High);

    #[test]
    fn table_matches_dec_011_cell_by_cell() {
        // (failed, unreadable, unsupported, issues, max severity, strict,
        //  fail_on, write_failed) -> code. Typed from the DEC-011 part 2 table.
        #[rustfmt::skip]
        let grid: &[((usize, usize, usize, usize, Option<Severity>, bool, Option<Severity>, bool), u8)] = &[
            // 0: complete and under the threshold (also: no files, fail-on none).
            ((0, 0, 0, 0, NONE, false, HIGH, false), 0),
            ((0, 0, 0, 0, NONE, true,  HIGH, false), 0),
            ((0, 0, 0, 0, LOW,  false, HIGH, false), 0),
            ((0, 0, 0, 0, MED,  false, HIGH, false), 0),
            ((0, 0, 0, 0, HIGH, false, NONE, false), 0),
            ((0, 0, 0, 0, HIGH, true,  NONE, false), 0),
            ((0, 0, 0, 0, LOW,  false, MED,  false), 0),
            // 0: unsupported and issues are fine without --strict.
            ((0, 0, 4, 0, NONE, false, HIGH, false), 0),
            ((0, 0, 0, 5, LOW,  false, HIGH, false), 0),
            ((0, 0, 2, 2, NONE, false, LOW,  false), 0),
            // 1: complete, at or above the threshold.
            ((0, 0, 0, 0, HIGH, false, HIGH, false), 1),
            ((0, 0, 0, 0, HIGH, false, MED,  false), 1),
            ((0, 0, 0, 0, HIGH, false, LOW,  false), 1),
            ((0, 0, 0, 0, MED,  false, MED,  false), 1),
            ((0, 0, 0, 0, MED,  false, LOW,  false), 1),
            ((0, 0, 0, 0, LOW,  false, LOW,  false), 1),
            ((0, 0, 3, 3, HIGH, false, HIGH, false), 1),
            // 3: incomplete (failed or unreadable), with or without findings.
            ((1, 0, 0, 0, NONE, false, HIGH, false), 3),
            ((0, 1, 0, 0, NONE, false, HIGH, false), 3),
            ((1, 1, 0, 0, NONE, false, NONE, false), 3),
            ((1, 0, 0, 0, HIGH, false, HIGH, false), 3),
            ((0, 2, 0, 0, HIGH, false, LOW,  false), 3),
            // 3: --strict adds unsupported and issues.
            ((0, 0, 1, 0, NONE, true,  HIGH, false), 3),
            ((0, 0, 0, 1, NONE, true,  HIGH, false), 3),
            ((0, 0, 1, 1, NONE, true,  NONE, false), 3),
            ((0, 0, 1, 0, HIGH, true,  HIGH, false), 3),
            ((0, 0, 0, 1, LOW,  true,  HIGH, false), 3),
            // 4: write failure outranks everything.
            ((0, 0, 0, 0, NONE, false, HIGH, true),  4),
            ((0, 0, 0, 0, HIGH, false, HIGH, true),  4),
            ((1, 0, 0, 0, HIGH, false, HIGH, true),  4),
            ((0, 1, 2, 3, MED,  true,  LOW,  true),  4),
            ((0, 0, 0, 0, NONE, false, NONE, true),  4),
        ];
        for (i, ((f, u, s, is, sev, strict, fail_on, wf), want)) in grid.iter().enumerate() {
            let got = exit_code(&summary(*f, *u, *s, *is, *sev), &opts(*strict, *fail_on), *wf);
            assert_eq!(
                got, *want,
                "row {i}: failed {f} unreadable {u} unsupported {s} issues {is} max {sev:?} strict {strict} fail_on {fail_on:?} write_failed {wf}"
            );
        }
    }

    #[test]
    fn incomplete_outranks_findings() {
        for fail_on in [LOW, MED, HIGH] {
            assert_eq!(exit_code(&summary(1, 0, 0, 0, HIGH, ), &opts(false, fail_on), false), 3);
            assert_eq!(exit_code(&summary(0, 1, 0, 0, HIGH), &opts(false, fail_on), false), 3);
            assert_eq!(exit_code(&summary(0, 0, 1, 0, HIGH), &opts(true, fail_on), false), 3);
            assert_eq!(exit_code(&summary(0, 0, 0, 1, HIGH), &opts(true, fail_on), false), 3);
        }
    }

    #[test]
    fn write_failure_outranks_everything() {
        for (f, u) in [(0, 0), (1, 0), (0, 1)] {
            for sev in [NONE, LOW, HIGH] {
                for strict in [false, true] {
                    assert_eq!(exit_code(&summary(f, u, 1, 1, sev), &opts(strict, HIGH), true), 4);
                }
            }
        }
    }

    #[test]
    fn fail_on_none_never_returns_one() {
        for sev in [NONE, LOW, MED, HIGH] {
            for strict in [false, true] {
                for (f, u, s, i) in [(0, 0, 0, 0), (1, 0, 0, 0), (0, 0, 1, 1)] {
                    let code = exit_code(&summary(f, u, s, i, sev), &opts(strict, NONE), false);
                    assert_ne!(code, 1, "{sev:?} {strict}");
                }
            }
        }
    }

    #[test]
    fn threshold_is_inclusive() {
        // At the threshold: 1. One rank below: 0. One rank above: 1.
        for (fail_on, sev, want) in [
            (LOW, LOW, 1),
            (LOW, MED, 1),
            (LOW, HIGH, 1),
            (MED, LOW, 0),
            (MED, MED, 1),
            (MED, HIGH, 1),
            (HIGH, LOW, 0),
            (HIGH, MED, 0),
            (HIGH, HIGH, 1),
        ] {
            assert_eq!(
                exit_code(&summary(0, 0, 0, 0, sev), &opts(false, fail_on), false),
                want,
                "fail_on {fail_on:?} max {sev:?}"
            );
        }
    }
}
