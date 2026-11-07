// The SPEC-016 invariants of the `exif-audit` argument parser, shared by the
// fuzz target (`fuzz/fuzz_targets/cli_args.rs`) and the stable-toolchain layout
// test (`tests/fuzz_layout.rs`) with `include!`. Not a crate root: text
// included at module level, defining one module, `cli_invariants`, which needs
// the real parser included first as `cli_args_src` (the file
// `src/bin/exif-audit/args.rs`, through `#[path]`, so the target runs the real
// parser). `cargo fmt` does not follow `include!`, so format this file by hand
// (`rustfmt --edition 2021 <file>`).
//
// Method (as `audit_invariants.rs` and `render_invariants.rs`): DEC-011 part 1
// is typed AGAIN here as a second, naive scan of the argument list (names,
// arities, value sets, the duplicate and conflict rules), and the real parser's
// outcome is compared with it by named GUARDS. Each guard is a pure function of
// one `Ctx` and runs ALONE through `run_guard`, so the plants in
// `tests/fuzz_layout.rs` show each one going red on its own bug.
//
// Input layout: the arguments joined with NUL. Empty input is no arguments; a
// lone NUL is two empty arguments.

#[allow(dead_code)]
mod cli_invariants {
    use super::cli_args_src::{parse, Options, Outcome};
    use std::ffi::OsString;

    /// The arguments as raw bytes: split on `0x00`; no bytes is no arguments.
    pub fn split_args(data: &[u8]) -> Vec<Vec<u8>> {
        if data.is_empty() {
            return Vec::new();
        }
        data.split(|b| *b == 0).map(<[u8]>::to_vec).collect()
    }

    #[cfg(unix)]
    fn os(bytes: &[u8]) -> OsString {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(bytes.to_vec())
    }

    #[cfg(not(unix))]
    fn os(bytes: &[u8]) -> OsString {
        OsString::from(String::from_utf8_lossy(bytes).into_owned())
    }

    /// One case: the arguments, the real parser's outcome (twice) and the
    /// message of a usage error.
    #[derive(Clone, Debug)]
    pub struct Ctx {
        pub args: Vec<Vec<u8>>,
        pub outcome: Outcome,
        pub again: Outcome,
        pub message: Option<String>,
    }

    pub fn build(data: &[u8]) -> Ctx {
        let args = split_args(data);
        let os_args: Vec<OsString> = args.iter().map(|a| os(a)).collect();
        let outcome = parse(&os_args);
        let again = parse(&os_args);
        let message = match &outcome {
            Outcome::Usage(e) => Some(e.message()),
            _ => None,
        };
        Ctx {
            args,
            outcome,
            again,
            message,
        }
    }

    // ── the naive re-derivation of DEC-011 part 1 ───────────────────────────

    /// Arguments before the first `--`.
    fn before_dd(args: &[Vec<u8>]) -> &[Vec<u8>] {
        match args.iter().position(|a| a.as_slice() == b"--") {
            Some(i) => &args[..i],
            None => args,
        }
    }

    /// What the grammar says the arguments mean, as text (Debug forms), or
    /// that they are a usage error.
    #[derive(Debug, PartialEq)]
    pub struct Want {
        format: &'static str,
        policy: &'static str,
        fail_on: &'static str,
        strict: bool,
        values: bool,
        inventory: bool,
        max_size: u64,
        paths: Vec<Vec<u8>>,
    }

    fn size(v: &str) -> Option<u64> {
        let (digits, mult): (&str, u128) = match v.chars().last() {
            Some('K') => (&v[..v.len() - 1], 1024),
            Some('M') => (&v[..v.len() - 1], 1024 * 1024),
            Some('G') => (&v[..v.len() - 1], 1024 * 1024 * 1024),
            _ => (v, 1),
        };
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let trimmed = digits.trim_start_matches('0');
        if trimmed.is_empty() {
            return None; // zero
        }
        if trimmed.len() > 20 {
            return None; // more than u64 can hold, with or without a factor
        }
        let n: u128 = trimmed.parse().ok()?;
        let total = n * mult;
        if total > u128::from(u64::MAX) {
            None
        } else {
            Some(total as u64)
        }
    }

    /// The second scan. `Err` is a usage error.
    pub fn naive(args: &[Vec<u8>]) -> Result<Want, String> {
        let mut want = Want {
            format: "Text",
            policy: "Some(AllPii)",
            fail_on: "Some(High)",
            strict: false,
            values: false,
            inventory: false,
            max_size: 256 * 1024 * 1024,
            paths: Vec::new(),
        };
        let mut seen: Vec<String> = Vec::new();
        let mut format_group = false;
        let mut i = 0;
        while i < args.len() {
            let a = &args[i];
            i += 1;
            if a.as_slice() == b"--" {
                want.paths.extend(args[i..].iter().cloned());
                break;
            }
            if !a.starts_with(b"-") {
                want.paths.push(a.clone());
                continue;
            }
            if a.as_slice() == b"-" {
                return Err("lone dash".into());
            }
            let (name_bytes, inline) = match a.iter().position(|c| *c == b'=') {
                Some(p) => (&a[..p], Some(a[p + 1..].to_vec())),
                None => (&a[..], None),
            };
            let name = std::str::from_utf8(name_bytes).map_err(|_| "unknown option")?;
            let takes_value = matches!(name, "--format" | "--policy" | "--fail-on" | "--max-size");
            let known = takes_value
                || matches!(
                    name,
                    "--json" | "--csv" | "--strict" | "--values" | "--inventory"
                );
            if !known {
                return Err(format!("unknown option {name}"));
            }
            let value: Option<String> = if takes_value {
                let raw = match inline {
                    Some(v) => v,
                    None => {
                        let v = args.get(i).ok_or("missing value")?.clone();
                        i += 1;
                        v
                    }
                };
                Some(String::from_utf8(raw).map_err(|_| "non-UTF-8 value")?)
            } else if inline.is_some() {
                return Err("flag with value".into());
            } else {
                None
            };
            if seen.iter().any(|s| s == name) {
                return Err(format!("{name} twice"));
            }
            seen.push(name.to_string());
            if matches!(name, "--format" | "--json" | "--csv") {
                if format_group {
                    return Err("two format selections".into());
                }
                format_group = true;
            }
            let v = value.as_deref().unwrap_or("");
            match name {
                "--json" => want.format = "Json",
                "--csv" => want.format = "Csv",
                "--format" => {
                    want.format = match v {
                        "text" => "Text",
                        "json" => "Json",
                        "csv" => "Csv",
                        _ => return Err("bad format".into()),
                    }
                }
                "--policy" => {
                    want.policy = match v {
                        "gps" => "Some(Gps)",
                        "gps-and-device" => "Some(GpsAndDevice)",
                        "all-pii" => "Some(AllPii)",
                        "all" => "Some(All)",
                        "everything" => "Some(Everything)",
                        "none" => "None",
                        _ => return Err("bad policy".into()),
                    }
                }
                "--fail-on" => {
                    want.fail_on = match v {
                        "none" => "None",
                        "low" => "Some(Low)",
                        "medium" => "Some(Medium)",
                        "high" => "Some(High)",
                        _ => return Err("bad fail-on".into()),
                    }
                }
                "--max-size" => want.max_size = size(v).ok_or("bad size")?,
                "--strict" => want.strict = true,
                "--values" => want.values = true,
                "--inventory" => want.inventory = true,
                _ => return Err("unreachable".into()),
            }
        }
        if want.paths.is_empty() {
            return Err("no path".into());
        }
        Ok(want)
    }

    fn show(o: &Options) -> Want {
        Want {
            format: match o.format {
                exif_core::report::Format::Text => "Text",
                exif_core::report::Format::Json => "Json",
                exif_core::report::Format::Csv => "Csv",
            },
            policy: match o.policy {
                None => "None",
                Some(exif_core::StripPolicy::Gps) => "Some(Gps)",
                Some(exif_core::StripPolicy::GpsAndDevice) => "Some(GpsAndDevice)",
                Some(exif_core::StripPolicy::AllPii) => "Some(AllPii)",
                Some(exif_core::StripPolicy::All) => "Some(All)",
                Some(exif_core::StripPolicy::Everything) => "Some(Everything)",
                Some(_) => "Some(other)",
            },
            fail_on: match o.fail_on {
                None => "None",
                Some(exif_core::audit::Severity::Low) => "Some(Low)",
                Some(exif_core::audit::Severity::Medium) => "Some(Medium)",
                Some(exif_core::audit::Severity::High) => "Some(High)",
            },
            strict: o.strict,
            values: o.values,
            inventory: o.inventory,
            max_size: o.max_size,
            paths: o
                .paths
                .iter()
                .map(|p| p.as_encoded_bytes().to_vec())
                .collect(),
        }
    }

    // ── the guards ──────────────────────────────────────────────────────────

    /// `parse` returns on every input and is a function of its arguments: the
    /// second call gives the same outcome.
    fn g_args_total(c: &Ctx) -> Result<(), String> {
        if c.outcome != c.again {
            return Err(format!(
                "two parses of the same arguments differ: {:?} then {:?}",
                c.outcome, c.again
            ));
        }
        Ok(())
    }

    /// `Help` iff `-h` or `--help` is an argument before the first `--`.
    /// Otherwise `Version` iff `-V` or `--version` is. Both directions.
    fn g_args_help_iff(c: &Ctx) -> Result<(), String> {
        let head = before_dd(&c.args);
        let help = head
            .iter()
            .any(|a| a.as_slice() == b"-h" || a.as_slice() == b"--help");
        let version = !help
            && head
                .iter()
                .any(|a| a.as_slice() == b"-V" || a.as_slice() == b"--version");
        let is_help = matches!(c.outcome, Outcome::Help);
        let is_version = matches!(c.outcome, Outcome::Version);
        if is_help != help {
            return Err(format!(
                "Help is {is_help} but a help argument before `--` is {help}: {:?}",
                c.outcome
            ));
        }
        if is_version != version {
            return Err(format!(
                "Version is {is_version} but a version argument (and no help) is {version}: {:?}",
                c.outcome
            ));
        }
        Ok(())
    }

    /// When neither help nor version applies: `Run` iff the naive scan says
    /// the arguments are valid, and then every field equals the naive
    /// derivation (so the defaults hold exactly when an option is absent, and
    /// `paths` is the non-option tokens and everything after `--`, in order,
    /// never empty).
    fn g_args_run_shape(c: &Ctx) -> Result<(), String> {
        if matches!(c.outcome, Outcome::Help | Outcome::Version) {
            return Ok(());
        }
        match (naive(&c.args), &c.outcome) {
            (Ok(want), Outcome::Run(o)) => {
                let got = show(o);
                if got != want {
                    return Err(format!("Run with {got:?}, the naive scan says {want:?}"));
                }
                if o.paths.is_empty() {
                    return Err("Run with no path".into());
                }
                Ok(())
            }
            (Ok(want), other) => Err(format!(
                "the naive scan accepts the arguments ({want:?}) but the parser gave {other:?}"
            )),
            (Err(why), Outcome::Run(o)) => Err(format!(
                "the naive scan rejects the arguments ({why}) but the parser gave Run({o:?})"
            )),
            (Err(_), Outcome::Usage(_)) => Ok(()),
            (Err(why), other) => Err(format!("{why}: unexpected outcome {other:?}")),
        }
    }

    /// A usage message is non-empty ASCII without a control character, at
    /// most 300 bytes.
    fn g_args_usage_message(c: &Ctx) -> Result<(), String> {
        match (&c.outcome, &c.message) {
            (Outcome::Usage(_), Some(m)) => {
                if m.is_empty() {
                    return Err("an empty usage message".into());
                }
                if !m.is_ascii() {
                    return Err(format!("a non-ASCII usage message: {m:?}"));
                }
                if m.chars().any(|ch| ch.is_ascii_control()) {
                    return Err(format!("a control character in a usage message: {m:?}"));
                }
                if m.len() > 300 {
                    return Err(format!("a usage message of {} bytes", m.len()));
                }
                Ok(())
            }
            (Outcome::Usage(_), None) => Err("a usage error without a message".into()),
            (_, Some(m)) => Err(format!("a message {m:?} on an outcome that is not Usage")),
            (_, None) => Ok(()),
        }
    }

    /// Two format selections (`--format`, `--json`, `--csv`) before the first
    /// `--` never give `Run`: they are a usage error. (A value of an option is
    /// never `--`-prefixed and valid, so every such token is a selection or
    /// makes the arguments invalid anyway.)
    fn g_args_format_conflict(c: &Ctx) -> Result<(), String> {
        if matches!(c.outcome, Outcome::Help | Outcome::Version) {
            return Ok(());
        }
        let selections = before_dd(&c.args)
            .iter()
            .filter(|a| {
                a.as_slice() == b"--json"
                    || a.as_slice() == b"--csv"
                    || a.as_slice() == b"--format"
                    || a.starts_with(b"--format=")
            })
            .count();
        if selections >= 2 && !matches!(c.outcome, Outcome::Usage(_)) {
            return Err(format!(
                "{selections} format selections gave {:?}, want a usage error",
                c.outcome
            ));
        }
        Ok(())
    }

    pub type Guard = fn(&Ctx) -> Result<(), String>;

    pub const GUARDS: &[(&str, Guard)] = &[
        ("args_total", g_args_total),
        ("args_help_iff", g_args_help_iff),
        ("args_run_shape", g_args_run_shape),
        ("args_usage_message", g_args_usage_message),
        ("args_format_conflict", g_args_format_conflict),
    ];

    /// Run ONE guard by name.
    pub fn run_guard(name: &str, c: &Ctx) -> Result<(), String> {
        let guard = GUARDS
            .iter()
            .find(|g| g.0 == name)
            .unwrap_or_else(|| panic!("no cli guard named {name}"));
        (guard.1)(c)
    }

    /// Every guard that fails, with its message.
    pub fn violations(c: &Ctx) -> Vec<(&'static str, String)> {
        GUARDS
            .iter()
            .filter_map(|(name, g)| g(c).err().map(|m| (*name, m)))
            .collect()
    }

    /// The harness body: the parser returns and every guard holds.
    pub fn check(data: &[u8]) {
        let c = build(data);
        let bad = violations(&c);
        assert!(bad.is_empty(), "cli invariants violated: {bad:?}");
    }
}
