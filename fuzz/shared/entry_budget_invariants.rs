// The SPEC-017 invariants of the total entry budget (DEC-001), shared by the
// `tiff_ifd` and `audit` fuzz targets and the stable-toolchain layout test
// (`tests/fuzz_layout.rs`) with `include!`, so they cannot drift. This file is
// not a crate root; it is text included at module level and defines one module,
// `entry_budget_invariants`. `cargo fmt` does not follow `include!`, so format
// it with `rustfmt --edition 2021 <file>`.
//
// The budget is re-stated here independently of the library (`len / 12`, the
// crossing IFD's declared count read from the input bytes), and each rule runs
// ALONE through `run_guard`, so the plants in `tests/fuzz_layout.rs` can show
// every guard going red on its own bug even where two guards overlap.

#[allow(dead_code)]
mod entry_budget_invariants {
    use exif_core::jpeg;
    use exif_core::tiff::Container;
    use exif_core::Error;

    /// What an issue was, as far as the budget rules care.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Halt {
        Entries,
        Ifds,
        Other,
    }

    /// The container's answer, flattened to plain data.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Shape {
        /// Input length.
        pub len: usize,
        /// The budget, re-derived here: `len / 12`.
        pub budget: usize,
        /// Entries summed over the recorded IFDs by the harness itself.
        pub entry_count: usize,
        /// What `Container::entry_count` said.
        pub reported_count: usize,
        /// Offsets of the recorded IFDs.
        pub ifd_offsets: Vec<u32>,
        /// Every issue in order: its kind and offset.
        pub issues: Vec<(Halt, Option<u32>)>,
        /// The `limit` carried by each `TooManyEntries`.
        pub limits: Vec<usize>,
        /// The entry count the input's own bytes declare at the offset of the
        /// (first) `TooManyEntries` issue.
        pub declared_at_halt: Option<usize>,
    }

    /// Flatten `container` (parsed from `data`) to a `Shape`.
    pub fn shape(data: &[u8], container: &Container<'_>) -> Shape {
        let big = data.starts_with(b"MM");
        let issues: Vec<(Halt, Option<u32>)> = container
            .issues()
            .iter()
            .map(|i| {
                let kind = match i.error() {
                    Error::TooManyEntries { .. } => Halt::Entries,
                    Error::TooManyIfds { .. } => Halt::Ifds,
                    _ => Halt::Other,
                };
                (kind, i.offset())
            })
            .collect();
        let limits = container
            .issues()
            .iter()
            .filter_map(|i| match i.error() {
                Error::TooManyEntries { limit } => Some(*limit),
                _ => None,
            })
            .collect();
        let declared_at_halt = issues
            .iter()
            .find(|(k, _)| *k == Halt::Entries)
            .and_then(|(_, at)| *at)
            .and_then(|at| {
                let at = at as usize;
                let b: [u8; 2] = data.get(at..at.checked_add(2)?)?.try_into().ok()?;
                Some(usize::from(if big {
                    u16::from_be_bytes(b)
                } else {
                    u16::from_le_bytes(b)
                }))
            });
        Shape {
            len: data.len(),
            budget: data.len() / 12,
            entry_count: container.ifds().iter().map(|i| i.entries().len()).sum(),
            reported_count: container.entry_count(),
            ifd_offsets: container.ifds().iter().map(|i| i.offset()).collect(),
            issues,
            limits,
            declared_at_halt,
        }
    }

    /// Never more entries than the budget, and the library's own count agrees
    /// with the sum of the IFDs.
    fn g_budget(s: &Shape) -> Result<(), String> {
        if s.entry_count > s.budget {
            return Err(format!(
                "{} entries retained, budget {} for {} bytes",
                s.entry_count, s.budget, s.len
            ));
        }
        if s.reported_count != s.entry_count {
            return Err(format!(
                "entry_count() says {}, the IFDs hold {}",
                s.reported_count, s.entry_count
            ));
        }
        Ok(())
    }

    /// `TooManyEntries` at most once, and only as the last issue.
    fn g_once_last(s: &Shape) -> Result<(), String> {
        let positions: Vec<usize> = s
            .issues
            .iter()
            .enumerate()
            .filter(|(_, (k, _))| *k == Halt::Entries)
            .map(|(i, _)| i)
            .collect();
        if positions.len() > 1 {
            return Err(format!("TooManyEntries recorded {} times", positions.len()));
        }
        if let Some(p) = positions.first() {
            if p + 1 != s.issues.len() {
                return Err("an issue was recorded after TooManyEntries".into());
            }
        }
        Ok(())
    }

    /// The two halting issues never appear together.
    fn g_exclusive(s: &Shape) -> Result<(), String> {
        let entries = s.issues.iter().any(|(k, _)| *k == Halt::Entries);
        let ifds = s.issues.iter().any(|(k, _)| *k == Halt::Ifds);
        if entries && ifds {
            return Err("TooManyEntries together with TooManyIfds".into());
        }
        Ok(())
    }

    /// A `TooManyEntries` issue is genuine (what was retained plus the IFD's
    /// declared count really exceeds the budget: an off-by-one in either
    /// direction fails here), names the right budget, and the IFD it refused
    /// is not among the recorded ones.
    fn g_crossing(s: &Shape) -> Result<(), String> {
        let Some((_, at)) = s.issues.iter().find(|(k, _)| *k == Halt::Entries) else {
            if !s.limits.is_empty() {
                return Err("limits recorded without an issue".into());
            }
            return Ok(());
        };
        if s.limits.iter().any(|l| *l != s.budget) {
            return Err(format!("limit {:?}, budget {}", s.limits, s.budget));
        }
        let Some(declared) = s.declared_at_halt else {
            return Err("the refused IFD's count word is not readable".into());
        };
        if s.entry_count + declared <= s.budget {
            return Err(format!(
                "TooManyEntries but {} + {} fits the budget {}",
                s.entry_count, declared, s.budget
            ));
        }
        if let Some(at) = at {
            if s.ifd_offsets.contains(at) {
                return Err(format!("the refused IFD at {at} was recorded"));
            }
        }
        Ok(())
    }

    /// One named guard.
    type Guard = fn(&Shape) -> Result<(), String>;

    /// Every guard, by name.
    pub const GUARDS: [(&str, Guard); 4] = [
        ("budget", g_budget),
        ("once-last", g_once_last),
        ("exclusive", g_exclusive),
        ("crossing", g_crossing),
    ];

    /// Every guard on `s`; the first violation, named.
    pub fn violations(s: &Shape) -> Vec<String> {
        GUARDS
            .iter()
            .filter_map(|(name, g)| g(s).err().map(|e| format!("{name}: {e}")))
            .collect()
    }

    /// Only the named guard.
    pub fn run_guard(name: &str, s: &Shape) -> Result<(), String> {
        match GUARDS.iter().find(|(n, _)| *n == name) {
            Some((_, g)) => g(s),
            None => Err(format!("no guard named {name}")),
        }
    }

    /// The harness body for one parsed container.
    pub fn check_container(data: &[u8], container: &Container<'_>) {
        let v = violations(&shape(data, container));
        assert!(v.is_empty(), "entry budget: {v:?}");
    }

    /// The harness body for the `audit` target: every TIFF container the
    /// audit would read from `data` (the file itself, or each Exif block of a
    /// JPEG).
    pub fn check_input(data: &[u8]) {
        if data.starts_with(b"II*\0") || data.starts_with(b"MM\0*") {
            if let Ok(c) = Container::parse(data) {
                check_container(data, &c);
            }
        } else if data.starts_with(&[0xFF, 0xD8]) {
            if let Ok(meta) = jpeg::read(data) {
                for block in meta.exif_blocks() {
                    if let Ok(c) = block.container() {
                        check_container(block.tiff(), c);
                    }
                }
            }
        }
    }
}
