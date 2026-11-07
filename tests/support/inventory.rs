//! The inventory model of the ExifTool parity check (SPEC-013, tier A).
//!
//! The audit model (`exif_core::audit::audit`) lists every IFD entry it found
//! and every located region. ExifTool, run as `exiftool -a -u -G1 -H -s <file>`,
//! lists every tag it reads with its specific group (`-G1`), its id in hex
//! (`-H`), duplicates kept (`-a`) and unknown tags kept (`-u`). This file turns
//! both into the same shape, a **multiset of `(group, tag id)`** plus the **set
//! of region kinds**, and diffs them. A difference is either explained by an
//! entry of [`KNOWN_DIVERGENCES`] (an exact, pinned multiset difference with its
//! measured cause) or it is a failure.
//!
//! ExifTool is run, never read, never linked. Every claim about it in this file
//! is a command, a version and measured output (`tests/oracle-fixtures/
//! exiftool-inventory/`). Test support only, std plus `exif_core`: it is
//! `#[path]`-included by `tests/inventory_oracle.rs`, by
//! `tests/mutation_oracle.rs`, by `examples/oracle_record.rs` and, copied next
//! to the generated binary, by the mutated-copy probe crate (SPEC-008), so it
//! must not depend on any other support module.

#![allow(dead_code)]

use exif_core::audit::{FileAudit, ItemKind, RegionKind as LibRegionKind};
use exif_core::tiff::IfdKind;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The ExifTool command (the file path is appended).
pub const COMMAND_ARGS: [&str; 5] = ["-a", "-u", "-G1", "-H", "-s"];

/// ExifTool groups that describe the file rather than a tag in it; their lines
/// never reach a record.
pub const FILE_LEVEL_GROUPS: [&str; 4] = ["System", "File", "ExifTool", "Composite"];

/// Groups ExifTool lists that the audit model has no item for (Class G): the
/// JFIF APP0 header and the Photoshop resources that are not IPTC. Dropped, by
/// name; any other group that is not modelled is a [`ParseError`].
pub const UNMODELLED_GROUPS: [&str; 2] = ["JFIF", "Photoshop"];

/// Tag ids whose entries are not compared as tags (Class H): the four pointer
/// tags (their child IFDs' entries are compared in their own groups) and the
/// four whose *content* ExifTool decodes into other groups (XMP 0x02BC, IPTC
/// 0x83BB, Photoshop 0x8649, ICC profile 0x8773) and never lists as a tag.
pub const DIRECTORY_VALUED: [u16; 8] = [
    0x8769, 0x8825, 0xA005, 0x014A, 0x02BC, 0x83BB, 0x8649, 0x8773,
];

/// A multiset of `(group, tag id)` with counts.
pub type Multiset = BTreeMap<(String, u16), usize>;

/// The region kinds both sides can name (extended XMP is XMP for ExifTool).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionKind {
    Xmp,
    Iptc,
    Icc,
    Jumbf,
}

/// One side's inventory.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inventory {
    pub tags: Multiset,
    pub regions: BTreeSet<RegionKind>,
}

/// The ExifTool group name of a tag item, from the audit's [`IfdKind`]. `Chain`
/// IFDs are named by their position (`IFD2`, `IFD3`, ...) and `Sub` IFDs
/// `SubIFD*` (ExifTool numbers them `SubIFD`, `SubIFD1`, ...; the numbering is
/// normalised away, and pinned on one fixture by a separate test).
pub fn group_of(kind: IfdKind, chain_position: usize) -> String {
    match kind {
        IfdKind::Ifd0 => "IFD0".to_string(),
        IfdKind::Ifd1 => "IFD1".to_string(),
        IfdKind::Exif => "ExifIFD".to_string(),
        IfdKind::Gps => "GPS".to_string(),
        IfdKind::Interop => "InteropIFD".to_string(),
        IfdKind::Chain => format!("IFD{}", chain_position.saturating_add(2)),
        IfdKind::Sub => "SubIFD*".to_string(),
        other => panic!("an IfdKind the inventory model does not know: {other:?}"),
    }
}

/// Our inventory of one audit. Tag items in [`DIRECTORY_VALUED`] are left out
/// (Class H); `Chain` IFDs are numbered by their position among the `Chain`
/// IFDs of the same Exif block.
pub fn ours(a: &FileAudit<'_>) -> Inventory {
    let mut inv = Inventory::default();
    // Per block (`None` for a bare TIFF): the container indices of the chain
    // IFDs seen so far, in order.
    let mut chains: BTreeMap<Option<usize>, Vec<usize>> = BTreeMap::new();
    for item in a.items() {
        match item.kind() {
            ItemKind::Tag {
                ifd,
                ifd_index,
                tag,
                ..
            } => {
                // Numbered before the directory-valued filter, so a chain IFD
                // whose first entries are skipped still takes its position.
                let position = if ifd == IfdKind::Chain {
                    let seen = chains.entry(item.origin().block).or_default();
                    match seen.iter().position(|&i| i == ifd_index) {
                        Some(p) => p,
                        None => {
                            seen.push(ifd_index);
                            seen.len().saturating_sub(1)
                        }
                    }
                } else {
                    0
                };
                if DIRECTORY_VALUED.contains(&tag) {
                    continue;
                }
                *inv.tags.entry((group_of(ifd, position), tag)).or_insert(0) += 1;
            }
            ItemKind::Region { kind, .. } => {
                let k = match kind {
                    LibRegionKind::Xmp | LibRegionKind::ExtendedXmp => RegionKind::Xmp,
                    LibRegionKind::Iptc => RegionKind::Iptc,
                    LibRegionKind::Icc => RegionKind::Icc,
                    LibRegionKind::Jumbf => RegionKind::Jumbf,
                    other => panic!("a RegionKind the inventory model does not know: {other:?}"),
                };
                inv.regions.insert(k);
            }
        }
    }
    inv
}

/// One kept line of ExifTool's listing: the group, the tag id (`None` for a
/// synthetic value, printed `-`) and the tag name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub group: String,
    pub id: Option<u32>,
    pub name: String,
}

/// Why ExifTool's output could not be turned into an inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A line that starts `[` but is not `[Group] id name ...`.
    BadLine(String),
    /// The id column is neither `-` nor `0x<hex>`.
    BadId(String),
    /// A group that is not file-level, not an EXIF-family group, not a region
    /// group and not in [`UNMODELLED_GROUPS`]: a new group on a new input is a
    /// decision, never a silent drop.
    UnknownGroup { group: String, line: String },
    /// A record file that does not have the expected shape.
    BadRecord(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::BadLine(l) => write!(f, "unparseable ExifTool line {l:?}"),
            ParseError::BadId(l) => write!(f, "unparseable tag id in ExifTool line {l:?}"),
            ParseError::UnknownGroup { group, line } => write!(
                f,
                "unknown ExifTool group {group:?} (line {line:?}): name it in \
                 UNMODELLED_GROUPS or model it; it is never dropped silently"
            ),
            ParseError::BadRecord(m) => write!(f, "bad inventory record: {m}"),
        }
    }
}

/// The kept lines of `exiftool -a -u -G1 -H -s` output: tag lines only, with
/// the file-level groups removed, in ExifTool's own order. A line that does not
/// start with `[` is a continuation of a multi-line value and is skipped.
pub fn parse_stdout(stdout: &str) -> Result<Vec<Line>, ParseError> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((group, after)) = rest.split_once(']') else {
            return Err(ParseError::BadLine(line.to_string()));
        };
        if FILE_LEVEL_GROUPS.contains(&group) {
            continue;
        }
        let after = after.trim_start();
        let (id_text, tail) = after
            .split_once(char::is_whitespace)
            .ok_or_else(|| ParseError::BadLine(line.to_string()))?;
        let id = parse_id(id_text).ok_or_else(|| ParseError::BadId(line.to_string()))?;
        let name: String = tail
            .trim_start()
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        if name.is_empty() {
            return Err(ParseError::BadLine(line.to_string()));
        }
        out.push(Line {
            group: group.to_string(),
            id,
            name,
        });
    }
    Ok(out)
}

/// The id column: `-` (a synthetic value) is `Some(None)`, `0x<hex>` is
/// `Some(Some(id))`, anything else is `None`.
fn parse_id(text: &str) -> Option<Option<u32>> {
    if text == "-" {
        return Some(None);
    }
    let hex = text.strip_prefix("0x")?;
    u32::from_str_radix(hex, 16).ok().map(Some)
}

/// How a group name of an ExifTool line is treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupClass {
    /// An EXIF-family group: its lines with an id are tags (`IFD0`, `IFD1`,
    /// `IFD<n>`, `ExifIFD`, `GPS`, `InteropIFD`, `SubIFD`, `SubIFD<n>`).
    Exif,
    /// A region group: the presence of the region is what is compared.
    Region(RegionKind),
    /// Named in [`UNMODELLED_GROUPS`].
    Unmodelled,
}

/// Classify a group name, or `None` when it is none of the three.
pub fn classify_group(group: &str) -> Option<GroupClass> {
    if UNMODELLED_GROUPS.contains(&group) {
        return Some(GroupClass::Unmodelled);
    }
    if group.starts_with("XMP") {
        return Some(GroupClass::Region(RegionKind::Xmp));
    }
    if group.starts_with("IPTC") {
        return Some(GroupClass::Region(RegionKind::Iptc));
    }
    if group.starts_with("ICC") {
        return Some(GroupClass::Region(RegionKind::Icc));
    }
    if group.starts_with("JUMBF") {
        return Some(GroupClass::Region(RegionKind::Jumbf));
    }
    let numbered = |prefix: &str, min: u32| {
        group
            .strip_prefix(prefix)
            .and_then(|n| n.parse::<u32>().ok())
            .is_some_and(|n| n >= min)
    };
    if matches!(
        group,
        "IFD0" | "IFD1" | "ExifIFD" | "GPS" | "InteropIFD" | "SubIFD"
    ) || numbered("IFD", 2)
        || numbered("SubIFD", 1)
    {
        return Some(GroupClass::Exif);
    }
    None
}

/// Their inventory from the kept lines. Region-group lines (of any id) put the
/// region in the set; EXIF-family lines with an id count as tags (`SubIFD<n>`
/// normalised to `SubIFD*`); EXIF-family lines without an id are synthetic and
/// dropped; unmodelled groups are dropped; an unknown group is an error. Ids
/// are kept as `u16` (an EXIF-family id wider than 16 bits is a `BadId`).
pub fn inventory_of(lines: &[Line]) -> Result<Inventory, ParseError> {
    let mut inv = Inventory::default();
    for l in lines {
        let describe = || format!("{} {:?} {}", l.group, l.id, l.name);
        match classify_group(&l.group) {
            Some(GroupClass::Exif) => {
                let Some(id) = l.id else { continue };
                let id = u16::try_from(id).map_err(|_| ParseError::BadId(describe()))?;
                let group = if l.group.starts_with("SubIFD") {
                    "SubIFD*".to_string()
                } else {
                    l.group.clone()
                };
                *inv.tags.entry((group, id)).or_insert(0) += 1;
            }
            Some(GroupClass::Region(kind)) => {
                inv.regions.insert(kind);
            }
            Some(GroupClass::Unmodelled) => {}
            None => {
                return Err(ParseError::UnknownGroup {
                    group: l.group.clone(),
                    line: describe(),
                })
            }
        }
    }
    Ok(inv)
}

/// Their inventory from raw `exiftool -a -u -G1 -H -s` output.
pub fn theirs(stdout: &str) -> Result<Inventory, ParseError> {
    inventory_of(&parse_stdout(stdout)?)
}

/// What differs between two inventories.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InventoryDiff {
    pub ours_only: Multiset,
    pub theirs_only: Multiset,
    pub regions_ours_only: BTreeSet<RegionKind>,
    pub regions_theirs_only: BTreeSet<RegionKind>,
}

impl InventoryDiff {
    pub fn is_empty(&self) -> bool {
        self.ours_only.is_empty()
            && self.theirs_only.is_empty()
            && self.regions_ours_only.is_empty()
            && self.regions_theirs_only.is_empty()
    }
}

fn multiset_minus(a: &Multiset, b: &Multiset) -> Multiset {
    let mut out = Multiset::new();
    for (key, &n) in a {
        let have = b.get(key).copied().unwrap_or(0);
        if n > have {
            out.insert(key.clone(), n - have);
        }
    }
    out
}

/// The multiset difference in both directions and the region set difference,
/// in deterministic (sorted) order.
pub fn diff(ours: &Inventory, theirs: &Inventory) -> InventoryDiff {
    InventoryDiff {
        ours_only: multiset_minus(&ours.tags, &theirs.tags),
        theirs_only: multiset_minus(&theirs.tags, &ours.tags),
        regions_ours_only: ours.regions.difference(&theirs.regions).copied().collect(),
        regions_theirs_only: theirs.regions.difference(&ours.regions).copied().collect(),
    }
}

impl fmt::Display for InventoryDiff {
    /// One line per difference, `(group, id, count)` named, sorted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "(no difference)");
        }
        let mut first = true;
        let mut line = |f: &mut fmt::Formatter<'_>, s: String| -> fmt::Result {
            if !first {
                writeln!(f)?;
            }
            first = false;
            write!(f, "{s}")
        };
        for ((g, id), n) in &self.ours_only {
            line(f, format!("ours only:   {g} {id:#06x} x{n}"))?;
        }
        for ((g, id), n) in &self.theirs_only {
            line(f, format!("theirs only: {g} {id:#06x} x{n}"))?;
        }
        for r in &self.regions_ours_only {
            line(f, format!("ours only:   region {r:?}"))?;
        }
        for r in &self.regions_theirs_only {
            line(f, format!("theirs only: region {r:?}"))?;
        }
        Ok(())
    }
}

// ── the pinned divergences ──────────────────────────────────────────────────

/// Which side is right about a pinned difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Ours (listing the entry is evidence an audit must not hide).
    Ours,
    /// ExifTool's.
    Theirs,
    /// Neither: two documented limits, or not comparable.
    Neither,
}

/// The measured classes of difference (see the SPEC-013 table and the
/// questions each one points to).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// An entry whose value is unreadable (payload or count past the end).
    A,
    /// An IFD reached a second time (ExifTool re-reads the offset).
    B,
    /// The IFD depth limit.
    C,
    /// The caps and the entry budget.
    D,
    /// ExifTool does not return: not comparable, never run live.
    E,
    /// A JPEG shape (segment placement, the 17th Exif block).
    F,
    /// A located region whose payload ExifTool cannot decode: we list the
    /// region, ExifTool lists no group for it (not in the SPEC-013 design
    /// table; measured by the build on stub and minimal seed payloads).
    R,
}

/// One pinned divergence: a fixture whose inventory differs from ExifTool's
/// by exactly this and nothing else.
#[derive(Debug, Clone, Copy)]
pub struct Divergence {
    /// The input's canonical key (`<corpus>/<name>`).
    pub fixture: &'static str,
    pub class: Class,
    /// `(group, id, count)` we list and ExifTool does not.
    pub ours_only: &'static [(&'static str, u16, usize)],
    /// `(group, id, count)` ExifTool lists and we do not.
    pub theirs_only: &'static [(&'static str, u16, usize)],
    pub regions_ours_only: &'static [RegionKind],
    pub regions_theirs_only: &'static [RegionKind],
    pub right_side: Side,
    /// The `guidance/questions.yaml` id that explains it.
    pub question: &'static str,
    /// What ExifTool 13.55 printed for the differing entries, in one line (the
    /// full listing is in the committed record of the input's corpus).
    pub measured: &'static str,
    /// True iff ExifTool lists something we do not (`theirs_only` or
    /// `regions_theirs_only` is non-empty). The hard invariant: such a file
    /// must not be reported clean.
    pub ours_hides: bool,
}

impl Divergence {
    /// The exact diff this entry pins.
    pub fn pinned(&self) -> InventoryDiff {
        let set = |rows: &[(&str, u16, usize)]| -> Multiset {
            rows.iter()
                .map(|&(g, id, n)| ((g.to_string(), id), n))
                .collect()
        };
        InventoryDiff {
            ours_only: set(self.ours_only),
            theirs_only: set(self.theirs_only),
            regions_ours_only: self.regions_ours_only.iter().copied().collect(),
            regions_theirs_only: self.regions_theirs_only.iter().copied().collect(),
        }
    }

    /// Whether the flags are consistent with the pinned sets.
    pub fn hides_is_consistent(&self) -> bool {
        self.ours_hides == (!self.theirs_only.is_empty() || !self.regions_theirs_only.is_empty())
    }
}

/// Why a comparison against the table failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// A difference with no table entry.
    Unexplained { fixture: String, diff: String },
    /// A table entry whose input no longer differs.
    StoppedDiverging { fixture: String },
    /// A table entry whose input differs, but not by the pinned diff.
    ChangedDiff {
        fixture: String,
        pinned: String,
        actual: String,
    },
    /// `ours_hides` but the audit said the file is clean.
    HidesButClean { fixture: String, diff: String },
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::Unexplained { fixture, diff } => {
                write!(
                    f,
                    "{fixture}: UNEXPLAINED difference from ExifTool:\n{diff}"
                )
            }
            CheckError::StoppedDiverging { fixture } => write!(
                f,
                "{fixture}: pinned as a divergence but now agrees with ExifTool; \
                 remove the KNOWN_DIVERGENCES entry (and its question) if this is intended"
            ),
            CheckError::ChangedDiff {
                fixture,
                pinned,
                actual,
            } => write!(
                f,
                "{fixture}: the difference changed.\n-- pinned --\n{pinned}\n-- actual --\n{actual}"
            ),
            CheckError::HidesButClean { fixture, diff } => write!(
                f,
                "{fixture}: ExifTool lists entries we do not, yet the audit says the file is \
                 clean (a silent omission):\n{diff}"
            ),
        }
    }
}

/// Compare one input against the table `known`: no entry means the diff must be
/// empty; an entry means the diff must be exactly the pinned one, and, if it
/// hides anything, `clean` must be false. `clean` is the audit's
/// `Health::clean`.
pub fn check_against(
    known: &[Divergence],
    fixture: &str,
    d: &InventoryDiff,
    clean: bool,
) -> Result<(), CheckError> {
    match known.iter().find(|k| k.fixture == fixture) {
        // Not comparable: ExifTool is never run on it (Class E).
        Some(k) if k.class == Class::E => Ok(()),
        None if d.is_empty() => Ok(()),
        None => Err(CheckError::Unexplained {
            fixture: fixture.to_string(),
            diff: d.to_string(),
        }),
        Some(_) if d.is_empty() => Err(CheckError::StoppedDiverging {
            fixture: fixture.to_string(),
        }),
        Some(k) => {
            let pinned = k.pinned();
            if &pinned != d {
                return Err(CheckError::ChangedDiff {
                    fixture: fixture.to_string(),
                    pinned: pinned.to_string(),
                    actual: d.to_string(),
                });
            }
            if k.ours_hides && clean {
                return Err(CheckError::HidesButClean {
                    fixture: fixture.to_string(),
                    diff: d.to_string(),
                });
            }
            Ok(())
        }
    }
}

// ── the probe's text form (SPEC-013, mutated-copy red-proofs) ───────────────

/// The inventory as the probe crate prints it: `tag<TAB>group<TAB>id<TAB>count`
/// per entry and `region<TAB>Kind` per region, sorted. Round-trips through
/// [`parse_probe_text`].
pub fn probe_text(inv: &Inventory) -> String {
    let mut s = String::new();
    for ((g, id), n) in &inv.tags {
        s.push_str(&format!("tag\t{g}\t{id}\t{n}\n"));
    }
    for r in &inv.regions {
        s.push_str(&format!("region\t{r:?}\n"));
    }
    s
}

/// The inverse of [`probe_text`].
pub fn parse_probe_text(text: &str) -> Result<Inventory, String> {
    let mut inv = Inventory::default();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["tag", g, id, n] => {
                let id = id.parse::<u16>().map_err(|e| format!("{line:?}: {e}"))?;
                let n = n.parse::<usize>().map_err(|e| format!("{line:?}: {e}"))?;
                inv.tags.insert((g.to_string(), id), n);
            }
            ["region", k] => {
                let kind = match *k {
                    "Xmp" => RegionKind::Xmp,
                    "Iptc" => RegionKind::Iptc,
                    "Icc" => RegionKind::Icc,
                    "Jumbf" => RegionKind::Jumbf,
                    other => return Err(format!("unknown region kind {other:?}")),
                };
                inv.regions.insert(kind);
            }
            _ => return Err(format!("unparseable probe line {line:?}")),
        }
    }
    Ok(inv)
}

// ── records ─────────────────────────────────────────────────────────────────

/// The command line as recorded.
pub fn command_line() -> String {
    format!("exiftool {} <file>", COMMAND_ARGS.join(" "))
}

/// One section of an inventory record: one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    /// `Some(reason)` when ExifTool was not run on this input.
    pub outcome: Option<String>,
    pub stderr: Vec<String>,
    pub lines: Vec<Line>,
}

/// A parsed record file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFile {
    pub command: String,
    pub version: String,
    pub corpus: String,
    pub sections: Vec<Section>,
}

/// The header of a record file.
pub fn record_header(version: &str, corpus: &str) -> String {
    format!(
        "# command: {}\n# exiftool-version: {version}\n# corpus: {corpus}\n",
        command_line()
    )
}

fn id_text(id: Option<u32>) -> String {
    match id {
        Some(n) => format!("0x{n:04x}"),
        None => "-".to_string(),
    }
}

/// One section: `## name`, an optional `# outcome:`, `# stderr:` lines, then one
/// `group<TAB>id<TAB>name` line per kept line.
pub fn section_text(
    name: &str,
    outcome: Option<&str>,
    stderr: &[String],
    lines: &[Line],
) -> String {
    let mut s = format!("## {name}\n");
    if let Some(o) = outcome {
        s.push_str(&format!("# outcome: {o}\n"));
    }
    for e in stderr {
        s.push_str(&format!("# stderr: {e}\n"));
    }
    for l in lines {
        s.push_str(&format!("{}\t{}\t{}\n", l.group, id_text(l.id), l.name));
    }
    s
}

/// Parse a record file written by [`record_header`] and [`section_text`].
pub fn parse_record(text: &str) -> Result<RecordFile, ParseError> {
    let bad = |m: &str| ParseError::BadRecord(m.to_string());
    let mut command = None;
    let mut version = None;
    let mut corpus = None;
    let mut sections: Vec<Section> = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("## ") {
            sections.push(Section {
                name: name.to_string(),
                outcome: None,
                stderr: Vec::new(),
                lines: Vec::new(),
            });
        } else if let Some(v) = line.strip_prefix("# command: ") {
            command = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("# exiftool-version: ") {
            version = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("# corpus: ") {
            corpus = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("# outcome: ") {
            sections
                .last_mut()
                .ok_or_else(|| bad("an outcome before the first section"))?
                .outcome = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("# stderr: ") {
            sections
                .last_mut()
                .ok_or_else(|| bad("a stderr line before the first section"))?
                .stderr
                .push(v.to_string());
        } else if line.starts_with('#') || line.is_empty() {
            continue;
        } else {
            let mut f = line.split('\t');
            let (Some(group), Some(id), Some(name), None) =
                (f.next(), f.next(), f.next(), f.next())
            else {
                return Err(bad(&format!(
                    "a line that is not group<TAB>id<TAB>name: {line:?}"
                )));
            };
            let id = parse_id(id).ok_or_else(|| ParseError::BadId(line.to_string()))?;
            sections
                .last_mut()
                .ok_or_else(|| bad("a tag line before the first section"))?
                .lines
                .push(Line {
                    group: group.to_string(),
                    id,
                    name: name.to_string(),
                });
        }
    }
    Ok(RecordFile {
        command: command.ok_or_else(|| bad("no command header"))?,
        version: version.ok_or_else(|| bad("no version header"))?,
        corpus: corpus.ok_or_else(|| bad("no corpus header"))?,
        sections,
    })
}

// ── the table, written by hand from the build's own measurement ─────────────

/// The fourteen entries of the IFD0 and Exif IFD of `exif-basic-le` that
/// ExifTool does not list when the Exif segment follows the scan or ends the
/// file (Class F), in sorted order.
const EXIF_BASIC_LE_ENTRIES: &[(&str, u16, usize)] = &[
    ("ExifIFD", 0x829d, 1),
    ("ExifIFD", 0x8827, 1),
    ("ExifIFD", 0x9003, 1),
    ("ExifIFD", 0x9204, 1),
    ("IFD0", 0x0100, 1),
    ("IFD0", 0x0101, 1),
    ("IFD0", 0x010f, 1),
    ("IFD0", 0x0110, 1),
    ("IFD0", 0x0112, 1),
    ("IFD0", 0x011a, 1),
    ("IFD0", 0x0131, 1),
    ("IFD0", 0x0132, 1),
    ("IFD0", 0x013b, 1),
    ("IFD0", 0x8298, 1),
];

const Q_UNREADABLE: &str = "exiftool-omits-entries-with-unreadable-values";
const Q_REVISITED: &str = "exiftool-rereads-a-revisited-ifd";
const Q_LIMITS: &str = "audit-depth-limit-and-ifd-caps-differ-from-exiftool";
const Q_REGIONS: &str = "exiftool-lists-no-group-for-undecodable-regions";
const Q_AFTER_SCAN: &str = "exiftool-ignores-exif-after-scan";
const Q_AT_EOF: &str = "exiftool-drops-exif-when-last-segment-ends-at-eof";
const Q_17TH: &str = "exiftool-reads-17th-exif-block";

/// A region-only divergence (Class R).
const fn region_only(
    fixture: &'static str,
    regions: &'static [RegionKind],
    measured: &'static str,
) -> Divergence {
    Divergence {
        fixture,
        class: Class::R,
        ours_only: &[],
        theirs_only: &[],
        regions_ours_only: regions,
        regions_theirs_only: &[],
        right_side: Side::Ours,
        question: Q_REGIONS,
        ours_hides: false,
        measured,
    }
}

/// An entries-only divergence with the other sets empty.
const fn entries(
    fixture: &'static str,
    class: Class,
    ours_only: &'static [(&'static str, u16, usize)],
    theirs_only: &'static [(&'static str, u16, usize)],
    right_side: Side,
    question: &'static str,
    measured: &'static str,
) -> Divergence {
    Divergence {
        fixture,
        class,
        ours_only,
        theirs_only,
        regions_ours_only: &[],
        regions_theirs_only: &[],
        right_side,
        question,
        // Derived, never typed: ExifTool lists something we do not.
        ours_hides: !theirs_only.is_empty(),
        measured,
    }
}

/// Every input whose inventory differs from ExifTool's 13.55, with the exact
/// difference, measured by the SPEC-013 build (`exiftool -a -u -G1 -H -s`; the
/// listings are in `tests/oracle-fixtures/exiftool-inventory/`). Rows are keyed
/// by the canonical `<corpus>/<name>` of the first input with the bytes.
pub const KNOWN_DIVERGENCES: &[Divergence] = &[
    // Class A: an entry whose value is unreadable (payload or count runs past
    // the end). ExifTool lists nothing for it; we list it (an entry is evidence
    // even when its value cannot be read).
    entries(
        "seed-tiff_ifd/ascii-count-past-eof",
        Class::A,
        &[
            ("IFD0", 0x010f, 1),
            ("IFD0", 0x0110, 1),
            ("IFD0", 0x0131, 1),
            ("IFD0", 0x0132, 1),
            ("IFD0", 0x013b, 1),
        ],
        &[],
        Side::Ours,
        Q_UNREADABLE,
        "ExifTool lists no IFD0 tag (five ASCII entries whose count runs past the end)",
    ),
    entries(
        "seed-tiff_ifd/count-overflow",
        Class::A,
        &[
            ("IFD0", 0x02bd, 1),
            ("IFD0", 0x02be, 1),
            ("IFD0", 0x02bf, 1),
            ("IFD0", 0x02c1, 1),
        ],
        &[],
        Side::Ours,
        Q_UNREADABLE,
        "ExifTool lists no IFD0 tag (entries 0x02bc to 0x02c1 with a count that overflows)",
    ),
    entries(
        "seed-tiff_ifd/count-overflow-be",
        Class::A,
        &[
            ("IFD0", 0x02bd, 1),
            ("IFD0", 0x02be, 1),
            ("IFD0", 0x02bf, 1),
            ("IFD0", 0x02c1, 1),
        ],
        &[],
        Side::Ours,
        Q_UNREADABLE,
        "ExifTool lists no IFD0 tag (big-endian twin of count-overflow)",
    ),
    entries(
        "seed-tiff_ifd/payload-past-eof",
        Class::A,
        &[("IFD0", 0x02bd, 1)],
        &[],
        Side::Ours,
        Q_UNREADABLE,
        "ExifTool lists no IFD0 tag (an out-of-line payload that starts past the end)",
    ),
    entries(
        "seed-tiff_ifd/payload-past-eof-be",
        Class::A,
        &[("IFD0", 0x02bd, 1)],
        &[],
        Side::Ours,
        Q_UNREADABLE,
        "ExifTool lists no IFD0 tag (big-endian twin of payload-past-eof)",
    ),
    // Class B: an IFD reached a second time. ExifTool re-reads the offset under
    // the second group; we visit an offset once (DEC-005). The second reading
    // is the same bytes (nothing new to strip), and the file is not clean
    // (CyclicIfd).
    entries(
        "seed-tiff_ifd/exif-points-at-ifd0",
        Class::B,
        &[],
        &[("ExifIFD", 0x0100, 1)],
        Side::Ours,
        Q_REVISITED,
        "ExifTool lists IFD0 0x0100 again as ExifIFD 0x0100 (the Exif pointer names IFD0)",
    ),
    entries(
        "seed-tiff_ifd/self-cycle",
        Class::B,
        &[],
        &[("IFD1", 0x0100, 1)],
        Side::Ours,
        Q_REVISITED,
        "ExifTool lists IFD0 0x0100 again as IFD1 0x0100 (IFD0's next-IFD word names IFD0)",
    ),
    // Class C: the depth limit. ExifTool lists three more nested SubIFD
    // entries than our eight levels reach; the file is not clean
    // (IfdDepthExceeded).
    entries(
        "seed-tiff_ifd/deep-nesting",
        Class::C,
        &[],
        &[("SubIFD*", 0x0100, 3)],
        Side::Neither,
        Q_LIMITS,
        "ExifTool lists 11 SubIFD 0x0100 entries, we list 8 (the depth limit)",
    ),
    // Class D: caps and budget. ExifTool keeps only the first 10 SubIFDs of an
    // array; we keep the entries within the len/12 budget. Two documented
    // limits; neither is the answer.
    entries(
        "seed-tiff_ifd/entry-budget-exact",
        Class::D,
        &[("SubIFD*", 0x0002, 1), ("SubIFD*", 0xffff, 3)],
        &[],
        Side::Neither,
        Q_LIMITS,
        "ExifTool lists none of the overlapping SubIFD entries that we list (entry budget)",
    ),
    entries(
        "seed-tiff_ifd/entry-budget-over",
        Class::D,
        &[("SubIFD*", 0x0003, 1), ("SubIFD*", 0xffff, 2)],
        &[],
        Side::Neither,
        Q_LIMITS,
        "ExifTool lists none of the overlapping SubIFD entries that we list (entry budget)",
    ),
    entries(
        "seed-tiff_ifd/overlapping-subifds",
        Class::D,
        &[("SubIFD*", 0x00ff, 255)],
        &[],
        Side::Neither,
        Q_LIMITS,
        "ExifTool lists no SubIFD entry (it keeps the first 10 of the SubIFDs array)",
    ),
    entries(
        "seed-audit/audit-many-overlapping-ifds",
        Class::D,
        &[("SubIFD*", 0x02ff, 767)],
        &[],
        Side::Neither,
        Q_LIMITS,
        "ExifTool lists no SubIFD entry (it keeps the first 10 of the SubIFDs array)",
    ),
    // Class E: not comparable. ExifTool does not return (about 9 GB in 5 s);
    // never run, recorded as "not run".
    entries(
        "seed-tiff_ifd/two-hop-cycle",
        Class::E,
        &[],
        &[],
        Side::Neither,
        "exiftool-two-hop-cycle-does-not-return",
        "not run: ExifTool does not return (Deep recursion on ProcessDirectory)",
    ),
    // Class F: JPEG shapes (SPEC-011 names).
    entries(
        "fixtures/jpeg-exif-after-scan",
        Class::F,
        EXIF_BASIC_LE_ENTRIES,
        &[],
        Side::Ours,
        Q_AFTER_SCAN,
        "ExifTool lists no EXIF tag (the APP1 follows the first SOS)",
    ),
    entries(
        "fixtures/jpeg-exif-last-segment-at-eof",
        Class::F,
        EXIF_BASIC_LE_ENTRIES,
        &[],
        Side::Ours,
        Q_AT_EOF,
        "ExifTool lists no EXIF tag (the APP1 is the last segment and the file ends there)",
    ),
    entries(
        "fixtures/jpeg-exif-before-and-after-scan",
        Class::F,
        &[("IFD0", 0x010f, 1)],
        &[],
        Side::Ours,
        Q_AFTER_SCAN,
        "ExifTool lists IFD0 0x010f once (the block before the scan); we list it twice",
    ),
    entries(
        "fixtures/jpeg-seventeen-exif-last-differs",
        Class::F,
        &[],
        &[("IFD0", 0x010f, 1)],
        Side::Neither,
        Q_17TH,
        "ExifTool lists IFD0 0x010f seventeen times, we list sixteen (the block cap)",
    ),
    entries(
        "seed-jpeg_segments/exif-after-scan",
        Class::F,
        &[
            ("ExifIFD", 0x9003, 1),
            ("GPS", 0x0001, 1),
            ("IFD0", 0x010f, 1),
        ],
        &[],
        Side::Ours,
        Q_AFTER_SCAN,
        "ExifTool lists no EXIF tag (the APP1 follows the first SOS)",
    ),
    entries(
        "seed-jpeg_segments/seventeen-exif",
        Class::F,
        &[],
        &[
            ("ExifIFD", 0x9003, 1),
            ("GPS", 0x0001, 1),
            ("IFD0", 0x010f, 1),
        ],
        Side::Neither,
        Q_17TH,
        "ExifTool lists seventeen copies of each tag, we list sixteen (the block cap)",
    ),
    // Class R: a located region whose payload ExifTool cannot decode.
    region_only(
        "fixtures/jpeg-icc-stub",
        &[RegionKind::Icc],
        "ExifTool lists nothing for a header-only ICC profile (zero tags)",
    ),
    region_only(
        "fixtures/jpeg-jumbf-stub",
        &[RegionKind::Jumbf],
        "ExifTool lists nothing for a bare jumb box",
    ),
    region_only(
        "seed-jpeg_segments/exif-and-xmp",
        &[RegionKind::Xmp],
        "ExifTool lists no XMP group for the minimal XMP payload",
    ),
    region_only(
        "seed-jpeg_segments/extended-xmp-pair",
        &[RegionKind::Xmp],
        "ExifTool lists no XMP group for the minimal XMP and extended-XMP payloads",
    ),
    region_only(
        "seed-jpeg_segments/extended-xmp-short",
        &[RegionKind::Xmp],
        "ExifTool lists no XMP group for the short extended-XMP chunk",
    ),
    region_only(
        "seed-audit/audit-full-house-jpeg",
        &[RegionKind::Icc, RegionKind::Jumbf],
        "ExifTool lists no ICC or JUMBF group for the stub payloads of the full-house JPEG",
    ),
    region_only(
        "seed-jpeg_segments/icc-chunks-mismatch",
        &[RegionKind::Icc],
        "ExifTool lists no ICC group for the mismatched ICC chunks",
    ),
    region_only(
        "seed-jpeg_segments/icc-chunks-ok",
        &[RegionKind::Icc],
        "ExifTool lists no ICC group for the minimal ICC chunks",
    ),
    region_only(
        "seed-jpeg_segments/icc-iptc-jumbf",
        &[RegionKind::Icc, RegionKind::Jumbf],
        "ExifTool lists no ICC or JUMBF group for the minimal payloads",
    ),
    region_only(
        "seed-jpeg_segments/icc-seq-zero",
        &[RegionKind::Icc],
        "ExifTool lists no ICC group for the sequence-zero ICC chunk",
    ),
    region_only(
        "seed-jpeg_segments/iptc-8bim",
        &[RegionKind::Iptc],
        "ExifTool lists only Photoshop 0x0408 (its 0x0404 resource holds no IPTC dataset, so ExifTool prints no IPTC group)",
    ),
    region_only(
        "seed-jpeg_segments/iptc-truncated-resource",
        &[RegionKind::Iptc],
        "ExifTool lists no IPTC group for the truncated resource",
    ),
    region_only(
        "seed-jpeg_segments/jumbf-header",
        &[RegionKind::Jumbf],
        "ExifTool lists no JUMBF group for the header-only JUMBF segment",
    ),
    region_only(
        "seed-jpeg_segments/xmp-and-extended-xmp",
        &[RegionKind::Xmp],
        "ExifTool lists no XMP group for the minimal XMP payloads",
    ),
];
