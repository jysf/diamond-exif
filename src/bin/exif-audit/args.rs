//! Argument parsing for `exif-audit` (DEC-011 part 1).
//!
//! Pure: no I/O, total, never panics. It is a separate module so the unit
//! tests, the integration tests and the `cli_args` fuzz target (which includes
//! this very file through `#[path]`) reach the real parser without a process.
//!
//! Precedence: `--help` / `-h` anywhere before the first `--` gives
//! [`Outcome::Help`] even when other arguments are invalid; otherwise
//! `--version` / `-V` gives [`Outcome::Version`]; otherwise the first usage
//! error in argument order, and "no path" last.

use std::ffi::OsString;

use exif_core::audit::Severity;
use exif_core::report::{safe_text, Format};
use exif_core::StripPolicy;

/// The default `--max-size`: 256 MiB (256 * 1024 * 1024).
pub const DEFAULT_MAX_SIZE: u64 = 268_435_456;

/// The longest part of an argument quoted in a usage message.
const QUOTE_MAX: usize = 64;

/// What a run is asked to do (DEC-011 part 1; every default is stated here,
/// in `--help` and in the tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// `--format` / `--json` / `--csv`; default text.
    pub format: Format,
    /// `--policy`; default `all-pii`, `none` gives `None`.
    pub policy: Option<StripPolicy>,
    /// `--fail-on`; default high, `none` gives `None`.
    pub fail_on: Option<Severity>,
    /// `--strict`; default off.
    pub strict: bool,
    /// `--values`; default off.
    pub values: bool,
    /// `--inventory`; default off.
    pub inventory: bool,
    /// `--max-size`; default [`DEFAULT_MAX_SIZE`].
    pub max_size: u64,
    /// The paths, at least one, in the order given.
    pub paths: Vec<OsString>,
}

/// What the arguments ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Scan.
    Run(Options),
    /// Print [`USAGE`] and exit 0.
    Help,
    /// Print the version and exit 0.
    Version,
    /// A usage error (exit 2).
    Usage(UsageError),
}

/// A usage error: fixed ASCII text, an offending argument quoted through
/// `safe_text` (and cut, and any non-ASCII character escaped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError {
    message: String,
}

impl UsageError {
    /// The message: non-empty ASCII without control characters, at most 300
    /// bytes.
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

/// The `--help` text, pinned by `tests/golden/usage.txt`.
pub const USAGE: &str = "\
Usage: exif-audit [OPTIONS] [--] PATH...

Scan files and directories for metadata that can identify a person or a
device. Prints one report on stdout and exits with a code a CI job can act on.
Reads nothing but the named paths; writes nothing but the report.

Options:
      --format <F>    Report format: text, json or csv [default: text]
      --json          Shorthand for --format json [default: off]
      --csv           Shorthand for --format csv [default: off]
      --policy <P>    Strip policy the report previews: gps, gps-and-device,
                      all-pii, all, everything, or none for no preview
                      [default: all-pii]
      --fail-on <S>   Exit 1 when a file has a finding of at least this
                      severity: none, low, medium or high [default: high]
      --strict        Count unsupported files and files with issues as
                      incomplete (exit 3) [default: off]
      --values        Include a raw value excerpt of each tag [default: off]
      --inventory     List every inventory entry, not only findings
                      [default: off]
      --max-size <N>  Largest file read: bytes with an optional K, M or G
                      suffix (powers of 1024), at least 1 [default: 256M]
  -h, --help          Print this help and exit 0
  -V, --version       Print the version and exit 0

Options take their value as the next argument or as --name=value. Each option
may be given once. A lone - is an error (stdin is not read). After -- every
argument is a path. Directories are walked recursively in name order; a symlink
found in a walk is reported, not followed.

Exit codes:
  0  complete, and no file reaches the --fail-on threshold
  1  complete, and a file has a finding at or above the threshold
  2  usage error
  3  incomplete: a file failed or was unreadable (with --strict, also
     unsupported or with issues)
  4  the report could not be written
";

/// A value that is not valid UTF-8 can never be one of the allowed values.
fn as_str(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes).ok()
}

fn err(message: String) -> UsageError {
    UsageError { message }
}

/// `arg` for a message: through `safe_text`, non-ASCII escaped, cut.
fn quote(arg: &[u8]) -> String {
    let safe = safe_text(arg);
    let mut out = String::new();
    for ch in safe.chars() {
        if out.len() >= QUOTE_MAX {
            out.push_str("...");
            break;
        }
        if ch.is_ascii() {
            out.push(ch);
        } else {
            out.extend(ch.escape_unicode());
        }
    }
    out
}

fn parse_format(v: &str) -> Option<Format> {
    match v {
        "text" => Some(Format::Text),
        "json" => Some(Format::Json),
        "csv" => Some(Format::Csv),
        _ => None,
    }
}

/// `Some(None)` is `none`.
fn parse_policy(v: &str) -> Option<Option<StripPolicy>> {
    match v {
        "gps" => Some(Some(StripPolicy::Gps)),
        "gps-and-device" => Some(Some(StripPolicy::GpsAndDevice)),
        "all-pii" => Some(Some(StripPolicy::AllPii)),
        "all" => Some(Some(StripPolicy::All)),
        "everything" => Some(Some(StripPolicy::Everything)),
        "none" => Some(None),
        _ => None,
    }
}

/// `Some(None)` is `none`.
fn parse_fail_on(v: &str) -> Option<Option<Severity>> {
    match v {
        "none" => Some(None),
        "low" => Some(Some(Severity::Low)),
        "medium" => Some(Some(Severity::Medium)),
        "high" => Some(Some(Severity::High)),
        _ => None,
    }
}

/// Digits, then an optional `K`, `M` or `G` (powers of 1024); at least 1; no
/// sign, no space; an overflow is a bad value.
fn parse_size(v: &str) -> Option<u64> {
    let (digits, mult): (&str, u64) = if let Some(d) = v.strip_suffix('K') {
        (d, 1024)
    } else if let Some(d) = v.strip_suffix('M') {
        (d, 1_048_576)
    } else if let Some(d) = v.strip_suffix('G') {
        (d, 1_073_741_824)
    } else {
        (v, 1)
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u64 = digits.parse().ok()?;
    let total = n.checked_mul(mult)?;
    if total == 0 {
        None
    } else {
        Some(total)
    }
}

/// Which option is being parsed, for messages and duplicate tracking.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Opt {
    Format,
    Json,
    Csv,
    Policy,
    FailOn,
    Strict,
    Values,
    Inventory,
    MaxSize,
}

impl Opt {
    fn name(self) -> &'static str {
        match self {
            Opt::Format => "--format",
            Opt::Json => "--json",
            Opt::Csv => "--csv",
            Opt::Policy => "--policy",
            Opt::FailOn => "--fail-on",
            Opt::Strict => "--strict",
            Opt::Values => "--values",
            Opt::Inventory => "--inventory",
            Opt::MaxSize => "--max-size",
        }
    }

    fn from_name(name: &str) -> Option<Opt> {
        Some(match name {
            "--format" => Opt::Format,
            "--json" => Opt::Json,
            "--csv" => Opt::Csv,
            "--policy" => Opt::Policy,
            "--fail-on" => Opt::FailOn,
            "--strict" => Opt::Strict,
            "--values" => Opt::Values,
            "--inventory" => Opt::Inventory,
            "--max-size" => Opt::MaxSize,
            _ => return None,
        })
    }

    fn takes_value(self) -> bool {
        matches!(self, Opt::Format | Opt::Policy | Opt::FailOn | Opt::MaxSize)
    }
}

/// Parse the arguments (the program name excluded). Total.
pub fn parse(args: &[OsString]) -> Outcome {
    // Help and version look only before the first `--`, and at every
    // argument there, including a would-be option value.
    let before = || {
        args.iter()
            .map(|a| a.as_encoded_bytes())
            .take_while(|b| *b != b"--")
    };
    if before().any(|b| b == b"-h" || b == b"--help") {
        return Outcome::Help;
    }
    if before().any(|b| b == b"-V" || b == b"--version") {
        return Outcome::Version;
    }
    match parse_run(args) {
        Ok(o) => Outcome::Run(o),
        Err(e) => Outcome::Usage(e),
    }
}

fn parse_run(args: &[OsString]) -> Result<Options, UsageError> {
    let mut format: Option<(Format, Opt)> = None;
    let mut policy: Option<Option<StripPolicy>> = None;
    let mut fail_on: Option<Option<Severity>> = None;
    let mut strict = false;
    let mut values = false;
    let mut inventory = false;
    let mut max_size: Option<u64> = None;
    let mut paths: Vec<OsString> = Vec::new();

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let bytes = arg.as_encoded_bytes();
        if bytes == b"--" {
            paths.extend(it.by_ref().cloned());
            break;
        }
        if bytes.first() != Some(&b'-') {
            paths.push(arg.clone());
            continue;
        }
        if bytes == b"-" {
            return Err(err("a lone - is not a path (stdin is not read)".to_string()));
        }
        // `--name=value`: split at the first `=`.
        let (name_bytes, inline): (&[u8], Option<&[u8]>) = match bytes
            .iter()
            .position(|b| *b == b'=')
            .and_then(|i| bytes.split_at_checked(i))
        {
            Some((n, rest)) => (n, rest.get(1..)),
            None => (bytes, None),
        };
        let opt = as_str(name_bytes).and_then(Opt::from_name);
        let Some(opt) = opt else {
            return Err(err(format!("unknown option: {}", quote(bytes))));
        };
        let name = opt.name();
        let value: Option<&[u8]> = if opt.takes_value() {
            match inline {
                Some(v) => Some(v),
                None => match it.next() {
                    Some(v) => Some(v.as_encoded_bytes()),
                    None => return Err(err(format!("missing value for {name}"))),
                },
            }
        } else if inline.is_some() {
            return Err(err(format!("{name} does not take a value")));
        } else {
            None
        };
        let bad = |v: &[u8]| err(format!("bad value for {name}: {}", quote(v)));
        let again = || err(format!("{name} given more than once"));
        let text = value.and_then(as_str);
        match opt {
            Opt::Format | Opt::Json | Opt::Csv => {
                let chosen = match opt {
                    Opt::Json => Format::Json,
                    Opt::Csv => Format::Csv,
                    _ => match text.and_then(parse_format) {
                        Some(f) => f,
                        None => return Err(bad(value.unwrap_or_default())),
                    },
                };
                if let Some((_, first)) = format {
                    return Err(if first == opt {
                        again()
                    } else {
                        err("only one of --format, --json and --csv may be given".to_string())
                    });
                }
                format = Some((chosen, opt));
            }
            Opt::Policy => {
                let Some(p) = text.and_then(parse_policy) else {
                    return Err(bad(value.unwrap_or_default()));
                };
                if policy.replace(p).is_some() {
                    return Err(again());
                }
            }
            Opt::FailOn => {
                let Some(f) = text.and_then(parse_fail_on) else {
                    return Err(bad(value.unwrap_or_default()));
                };
                if fail_on.replace(f).is_some() {
                    return Err(again());
                }
            }
            Opt::MaxSize => {
                let Some(n) = text.and_then(parse_size) else {
                    return Err(bad(value.unwrap_or_default()));
                };
                if max_size.replace(n).is_some() {
                    return Err(again());
                }
            }
            Opt::Strict => {
                if std::mem::replace(&mut strict, true) {
                    return Err(again());
                }
            }
            Opt::Values => {
                if std::mem::replace(&mut values, true) {
                    return Err(again());
                }
            }
            Opt::Inventory => {
                if std::mem::replace(&mut inventory, true) {
                    return Err(again());
                }
            }
        }
    }
    if paths.is_empty() {
        return Err(err("no path given".to_string()));
    }
    Ok(Options {
        format: format.map_or(Format::Text, |(f, _)| f),
        policy: policy.unwrap_or(Some(StripPolicy::AllPii)),
        fail_on: fail_on.unwrap_or(Some(Severity::High)),
        strict,
        values,
        inventory,
        max_size: max_size.unwrap_or(DEFAULT_MAX_SIZE),
        paths,
    })
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

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn run(args: &[&str]) -> Options {
        match parse(&os(args)) {
            Outcome::Run(o) => o,
            other => panic!("{args:?}: expected Run, got {other:?}"),
        }
    }

    fn usage(args: &[&str]) -> String {
        match parse(&os(args)) {
            Outcome::Usage(e) => e.message(),
            other => panic!("{args:?}: expected Usage, got {other:?}"),
        }
    }

    #[test]
    fn defaults_are_exactly_the_dec_011_table() {
        // Typed from DEC-011 part 1, not from the code.
        let o = run(&["p"]);
        assert_eq!(o.format, Format::Text, "--format default text");
        assert_eq!(
            o.policy,
            Some(StripPolicy::AllPii),
            "--policy default all-pii"
        );
        assert_eq!(o.fail_on, Some(Severity::High), "--fail-on default high");
        assert!(!o.strict, "--strict default off");
        assert!(!o.values, "--values default off");
        assert!(!o.inventory, "--inventory default off");
        assert_eq!(o.max_size, 256 * 1024 * 1024, "--max-size default 256M");
        assert_eq!(o.paths, os(&["p"]));
    }

    #[test]
    fn every_option_and_its_equals_form() {
        type Check = Box<dyn Fn(&Options) -> bool>;
        let cases: [(&[&str], Check); 24] = [
            (
                &["--format", "text", "p"],
                Box::new(|o| o.format == Format::Text),
            ),
            (
                &["--format", "json", "p"],
                Box::new(|o| o.format == Format::Json),
            ),
            (
                &["--format=csv", "p"],
                Box::new(|o| o.format == Format::Csv),
            ),
            (&["--json", "p"], Box::new(|o| o.format == Format::Json)),
            (&["--csv", "p"], Box::new(|o| o.format == Format::Csv)),
            (
                &["--policy", "gps", "p"],
                Box::new(|o| o.policy == Some(StripPolicy::Gps)),
            ),
            (
                &["--policy=gps-and-device", "p"],
                Box::new(|o| o.policy == Some(StripPolicy::GpsAndDevice)),
            ),
            (
                &["--policy", "all-pii", "p"],
                Box::new(|o| o.policy == Some(StripPolicy::AllPii)),
            ),
            (
                &["--policy=all", "p"],
                Box::new(|o| o.policy == Some(StripPolicy::All)),
            ),
            (
                &["--policy", "everything", "p"],
                Box::new(|o| o.policy == Some(StripPolicy::Everything)),
            ),
            (&["--policy=none", "p"], Box::new(|o| o.policy.is_none())),
            (
                &["--fail-on", "none", "p"],
                Box::new(|o| o.fail_on.is_none()),
            ),
            (
                &["--fail-on=low", "p"],
                Box::new(|o| o.fail_on == Some(Severity::Low)),
            ),
            (
                &["--fail-on", "medium", "p"],
                Box::new(|o| o.fail_on == Some(Severity::Medium)),
            ),
            (
                &["--fail-on=high", "p"],
                Box::new(|o| o.fail_on == Some(Severity::High)),
            ),
            (&["--strict", "p"], Box::new(|o| o.strict)),
            (&["--values", "p"], Box::new(|o| o.values)),
            (&["--inventory", "p"], Box::new(|o| o.inventory)),
            (
                &["--max-size", "1000", "p"],
                Box::new(|o| o.max_size == 1000),
            ),
            (&["--max-size=2K", "p"], Box::new(|o| o.max_size == 2048)),
            (
                &["p", "--strict"],
                Box::new(|o| o.strict && o.paths == os(&["p"])),
            ),
            (
                &["a", "--values", "b"],
                Box::new(|o| o.paths == os(&["a", "b"])),
            ),
            (
                &[
                    "--format=json",
                    "--policy=none",
                    "--fail-on=none",
                    "--strict",
                    "--values",
                    "--inventory",
                    "--max-size=7",
                    "x",
                    "y",
                ],
                Box::new(|o| {
                    o.format == Format::Json
                        && o.policy.is_none()
                        && o.fail_on.is_none()
                        && o.strict
                        && o.values
                        && o.inventory
                        && o.max_size == 7
                        && o.paths == os(&["x", "y"])
                }),
            ),
            (
                &["--fail-on", "low", "--policy", "gps", "p"],
                Box::new(|o| {
                    o.fail_on == Some(Severity::Low) && o.policy == Some(StripPolicy::Gps)
                }),
            ),
        ];
        for (args, check) in &cases {
            let o = run(args);
            assert!(check(&o), "{args:?}: {o:?}");
        }
        // An option that is not given keeps its default next to one that is.
        let o = run(&["--strict", "p"]);
        assert_eq!(o.format, Format::Text);
        assert_eq!(o.fail_on, Some(Severity::High));
        // A value that is not in the set is a usage error, for every option.
        for args in [
            &["--format", "xml", "p"][..],
            &["--format=", "p"],
            &["--format=JSON", "p"],
            &["--policy", "most", "p"],
            &["--policy=ALL", "p"],
            &["--fail-on", "critical", "p"],
            &["--fail-on=", "p"],
            &["--max-size", "", "p"],
            &["--max-size", "abc", "p"],
            &["--max-size", "-1", "p"],
            &["--max-size", "+1", "p"],
            &["--max-size", "1 K", "p"],
            &["--max-size", "1T", "p"],
            &["--max-size", "1k", "p"],
            &["--max-size", "KK", "p"],
            &["--max-size", "K", "p"],
        ] {
            let m = usage(args);
            assert!(m.starts_with("bad value for --"), "{args:?}: {m}");
        }
        // A flag takes no value; a value option needs one.
        assert_eq!(
            usage(&["--strict=1", "p"]),
            "--strict does not take a value"
        );
        assert_eq!(usage(&["--json=x", "p"]), "--json does not take a value");
        assert_eq!(usage(&["p", "--format"]), "missing value for --format");
        assert_eq!(usage(&["p", "--max-size"]), "missing value for --max-size");
        // Unknown options, long and short.
        assert_eq!(usage(&["--nope", "p"]), "unknown option: --nope");
        assert_eq!(usage(&["-x", "p"]), "unknown option: -x");
        assert_eq!(usage(&["-hV=1", "p"]), "unknown option: -hV=1");
        assert_eq!(
            usage(&["--Format=json", "p"]),
            "unknown option: --Format=json"
        );
    }

    #[test]
    fn json_and_csv_are_format_shorthands_and_conflict() {
        assert_eq!(run(&["--json", "p"]), run(&["--format", "json", "p"]));
        assert_eq!(run(&["--csv", "p"]), run(&["--format=csv", "p"]));
        let conflict = "only one of --format, --json and --csv may be given";
        for args in [
            &["--json", "--csv", "p"][..],
            &["--csv", "--json", "p"],
            &["--json", "--format", "json", "p"],
            &["--format=csv", "--csv", "p"],
            &["--format", "text", "--json", "p"],
            &["--csv", "--format=text", "p"],
        ] {
            assert_eq!(usage(args), conflict, "{args:?}");
        }
        // The same selection twice is a repeat.
        assert_eq!(
            usage(&["--json", "--json", "p"]),
            "--json given more than once"
        );
        assert_eq!(
            usage(&["--format", "text", "--format", "json", "p"]),
            "--format given more than once"
        );
    }

    #[test]
    fn repeated_option_is_a_usage_error() {
        for (args, name) in [
            (&["--policy", "gps", "--policy", "all", "p"][..], "--policy"),
            (&["--policy=none", "--policy=none", "p"], "--policy"),
            (&["--fail-on", "low", "--fail-on", "low", "p"], "--fail-on"),
            (&["--strict", "--strict", "p"], "--strict"),
            (&["--values", "p", "--values"], "--values"),
            (&["--inventory", "--inventory", "p"], "--inventory"),
            (&["--max-size", "1", "--max-size=2", "p"], "--max-size"),
        ] {
            assert_eq!(
                usage(args),
                format!("{name} given more than once"),
                "{args:?}"
            );
        }
    }

    #[test]
    fn double_dash_makes_everything_a_path() {
        let o = run(&["--", "--strict", "-", "--", "-h", ""]);
        assert!(!o.strict);
        assert_eq!(o.paths, os(&["--strict", "-", "--", "-h", ""]));
        // Options before the `--` still apply, and the path may start with `-`.
        let o = run(&["--strict", "--", "-rf"]);
        assert!(o.strict);
        assert_eq!(o.paths, os(&["-rf"]));
        // Nothing after `--` is help, version or an error.
        assert_eq!(
            run(&["p", "--", "--help", "--version"]).paths,
            os(&["p", "--help", "--version"])
        );
        // After `--` with no path, still no path.
        assert_eq!(usage(&["--strict", "--"]), "no path given");
        assert_eq!(usage(&["--"]), "no path given");
    }

    #[test]
    fn lone_dash_and_no_path_are_usage_errors() {
        assert_eq!(usage(&["-"]), "a lone - is not a path (stdin is not read)");
        assert_eq!(
            usage(&["p", "-"]),
            "a lone - is not a path (stdin is not read)"
        );
        assert_eq!(usage(&[]), "no path given");
        assert_eq!(usage(&["--strict"]), "no path given");
        assert_eq!(usage(&["--format", "json"]), "no path given");
        // A value that happens to be `-` is a bad value, not a path.
        assert_eq!(usage(&["--format", "-", "p"]), "bad value for --format: -");
        // The empty string is a path (it will be reported as not found).
        assert_eq!(run(&[""]).paths, os(&[""]));
    }

    #[test]
    fn max_size_suffixes_and_bounds() {
        for (arg, want) in [
            ("1", 1u64),
            ("1024", 1024),
            ("007", 7),
            ("1K", 1024),
            ("3K", 3072),
            ("1M", 1_048_576),
            ("2M", 2_097_152),
            ("1G", 1_073_741_824),
            ("4G", 4_294_967_296),
            ("256M", 268_435_456),
            ("18446744073709551615", u64::MAX),
        ] {
            assert_eq!(run(&["--max-size", arg, "p"]).max_size, want, "{arg}");
        }
        for arg in [
            "0",
            "0K",
            "00",
            "18446744073709551616",
            "17179869184G",
            "18014398509481984K",
            "17592186044416M",
        ] {
            assert_eq!(
                usage(&["--max-size", arg, "p"]),
                format!("bad value for --max-size: {arg}"),
                "{arg}"
            );
        }
        // The largest size that fits, written with a suffix.
        assert_eq!(
            run(&["--max-size", "17179869183G", "p"]).max_size,
            17_179_869_183 * 1_073_741_824
        );
    }

    #[test]
    fn help_outranks_version_outranks_usage() {
        assert_eq!(parse(&os(&["-h"])), Outcome::Help);
        assert_eq!(parse(&os(&["--help"])), Outcome::Help);
        assert_eq!(parse(&os(&["--bogus", "--help"])), Outcome::Help);
        assert_eq!(parse(&os(&["--format", "xml", "-h", "p"])), Outcome::Help);
        assert_eq!(parse(&os(&["--version", "--help"])), Outcome::Help);
        assert_eq!(parse(&os(&["-V", "-h"])), Outcome::Help);
        assert_eq!(parse(&os(&["-V"])), Outcome::Version);
        assert_eq!(parse(&os(&["--version"])), Outcome::Version);
        assert_eq!(parse(&os(&["--bogus", "-V"])), Outcome::Version);
        assert_eq!(
            parse(&os(&["--json", "--csv", "--version"])),
            Outcome::Version
        );
        // Only before the first `--`.
        assert!(matches!(parse(&os(&["--", "-h"])), Outcome::Run(_)));
        assert!(matches!(
            parse(&os(&["--bogus", "--", "-h"])),
            Outcome::Usage(_)
        ));
        assert!(matches!(
            parse(&os(&["--bogus", "--", "-V"])),
            Outcome::Usage(_)
        ));
        // An option value that is `--help` still counts as the argument.
        assert_eq!(parse(&os(&["--format", "--help"])), Outcome::Help);
        // A near miss is not help.
        assert!(matches!(parse(&os(&["--hel", "p"])), Outcome::Usage(_)));
        assert!(matches!(parse(&os(&["--help=1"])), Outcome::Usage(_)));
        assert!(matches!(parse(&os(&["-H"])), Outcome::Usage(_)));
    }

    #[test]
    fn usage_messages_are_fixed_ascii() {
        let mut hostile: Vec<OsString> = vec![
            OsString::from("--\u{202E}evil"),
            OsString::from("--caf\u{E9}"),
            OsString::from("--x\ny\x1b[31m"),
            OsString::from(format!("--{}", "a".repeat(5000))),
        ];
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            hostile.push(OsString::from_vec(b"--bad\xFF\xFE".to_vec()));
            hostile.push(OsString::from_vec(b"--format=\xFF".to_vec()));
        }
        for h in hostile {
            let m = match parse(std::slice::from_ref(&h)) {
                Outcome::Usage(e) => e.message(),
                other => panic!("{h:?}: {other:?}"),
            };
            assert!(!m.is_empty() && m.len() <= 300, "{m:?}");
            assert!(m.is_ascii(), "{m:?}");
            assert!(m.chars().all(|c| !c.is_control()), "{m:?}");
        }
        // The quoted argument goes through safe_text: a newline is `\n`.
        assert_eq!(usage(&["--a\nb", "p"]), "unknown option: --a\\nb");
        // A long argument is cut.
        let long = format!("--{}", "z".repeat(500));
        let m = usage(&[long.as_str()]);
        assert_eq!(m, format!("unknown option: --{}...", "z".repeat(62)));
    }

    #[test]
    fn usage_text_names_every_default() {
        for needle in [
            "[default: text]",
            "[default: all-pii]",
            "[default: high]",
            "[default: 256M]",
        ] {
            assert!(USAGE.contains(needle), "{needle}");
        }
        assert_eq!(
            USAGE.matches("[default: off]").count(),
            5,
            "json, csv, strict, values, inventory"
        );
        for line in USAGE
            .lines()
            .filter(|l| l.trim_start().starts_with("--json") || l.trim_start().starts_with("--csv"))
        {
            assert!(line.ends_with("[default: off]"), "{line}");
        }
        for opt in [
            "--format",
            "--json",
            "--csv",
            "--policy",
            "--fail-on",
            "--strict",
            "--values",
            "--inventory",
            "--max-size",
            "--help",
            "--version",
        ] {
            assert!(USAGE.contains(opt), "{opt}");
        }
        assert!(USAGE.ends_with('\n') && !USAGE.ends_with("\n\n"));
        assert!(USAGE.is_ascii());
    }
}
