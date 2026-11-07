//! Reports (SPEC-015, DEC-010): pure renderers from per-file results to text,
//! JSON and CSV.
//!
//! The renderers take a list of [`FileEntry`] (a label and either a
//! [`FileAudit`] or a failure the caller reports) and [`RenderOptions`], and
//! return a `String`. They do no I/O. Everything in a report that did not come
//! from this library's own constants (labels, value excerpts, failure details)
//! is hostile text: it goes through [`safe_text`] and then through the escaping
//! of its own format, so each output is well-formed whatever the input bytes.
//!
//! Three rules from DEC-010 shape every format:
//!
//! * **Value-free by default.** No byte derived from a tag payload appears
//!   unless [`RenderOptions::values`] is set; then an entry shows a bounded raw
//!   excerpt ([`EXCERPT_MAX`] bytes before escaping), never a decoded value.
//! * **`ok` is earned.** Each file has one [`Status`]; `Ok` is emitted only for
//!   a clean audit. An unsupported or unreadable file is never `ok`, and a
//!   report never says "no personal data".
//! * **Pinned shapes.** The JSON keys and the CSV columns are API (see
//!   `schema_version` and [`csv::HEADER`]).
//!
//! Provenance (DEC-003, class 1): net-new code over this crate's own audit.

#[cfg(test)]
mod builder_tests;
mod csv;
mod json;
#[cfg(test)]
mod render_tests;
mod safe;
mod text;

use crate::audit::{
    preview, Category, ContainerKind, FileAudit, Item, ItemKind, Preview, Severity, Sniffed,
};
use crate::policy::StripPolicy;
use crate::tiff::IfdKind;

pub use csv::render_csv;
pub use json::render_json;
pub use safe::safe_text;
pub use text::render_text;

/// The most payload bytes an excerpt takes (before escaping).
pub const EXCERPT_MAX: usize = 64;

/// What a report shows. The default is the privacy-safe one: no policy
/// preview, no inventory listing, no value excerpts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderOptions {
    /// Preview this policy over every audited file (dispositions, residuals).
    pub policy: Option<StripPolicy>,
    /// List every inventory item, not only the findings.
    pub inventory: bool,
    /// Show a bounded raw excerpt of each tag payload. Off by default: a report
    /// must not spread the data it was run to find.
    pub values: bool,
}

/// Why a file has no audit (the caller's failure, not the library's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailReason {
    /// The file could not be read.
    Unreadable,
    /// The file exceeded the caller's size limit.
    TooLarge,
    /// The path is not a regular file.
    NotRegularFile,
    /// Anything else.
    Other,
}

impl FailReason {
    /// A stable lowercase name for reports.
    pub fn name(self) -> &'static str {
        match self {
            FailReason::Unreadable => "unreadable",
            FailReason::TooLarge => "too_large",
            FailReason::NotRegularFile => "not_regular_file",
            FailReason::Other => "other",
        }
    }
}

/// The outcome for one file: an audit, or a failure the caller reports.
#[derive(Debug, Clone)]
pub enum FileResult<'a> {
    /// The file was read and audited.
    Audited(FileAudit<'a>),
    /// The caller could not read the file; there is no audit. `detail` is
    /// hostile text (an OS error message may quote a path).
    Failed {
        /// The class of failure.
        reason: FailReason,
        /// A message from the caller, treated as hostile bytes.
        detail: &'a [u8],
    },
}

/// One file in a report: its label (a path; hostile bytes) and its result.
#[derive(Debug, Clone)]
pub struct FileEntry<'a> {
    /// The label (usually the path), as raw bytes.
    pub label: &'a [u8],
    /// What happened to the file.
    pub result: FileResult<'a>,
}

/// The one status each file has (DEC-010 part 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    /// The caller could not read the file; there is no audit.
    Failed,
    /// The container is not one the library reads.
    Unsupported,
    /// A supported container whose metadata could not be read.
    Unreadable,
    /// Readable, but the readers reported issues or the inventory was
    /// truncated.
    Issues,
    /// A clean audit.
    Ok,
}

impl Status {
    /// A stable lowercase name for reports.
    pub fn name(self) -> &'static str {
        match self {
            Status::Failed => "failed",
            Status::Unsupported => "unsupported",
            Status::Unreadable => "unreadable",
            Status::Issues => "issues",
            Status::Ok => "ok",
        }
    }
}

/// The status of one file (DEC-010 part 5, nothing else is consulted).
pub fn status_of(result: &FileResult<'_>) -> Status {
    match result {
        FileResult::Failed { .. } => Status::Failed,
        FileResult::Audited(a) => {
            if matches!(a.kind(), ContainerKind::Unsupported(_)) {
                Status::Unsupported
            } else if a.health().clean {
                Status::Ok
            } else if a.health().failed_blocks > 0 {
                Status::Unreadable
            } else {
                Status::Issues
            }
        }
    }
}

/// The run summary a binary reads for its exit code and its last lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Files in the run.
    pub files: usize,
    /// Files with status `failed`.
    pub failed: usize,
    /// Files with status `unsupported`.
    pub unsupported: usize,
    /// Files with status `unreadable`.
    pub unreadable: usize,
    /// Files with status `issues`.
    pub issues: usize,
    /// Files with status `ok`.
    pub ok: usize,
    /// Findings over every audited file.
    pub findings: usize,
    /// Findings per category, in [`Category::ALL`] order.
    pub by_category: [(Category, usize); 13],
    /// The highest severity over every audited file's findings.
    pub max_severity: Option<Severity>,
    /// With a policy and at least one reliable file: the findings that would
    /// remain, summed over the **reliable** files only. `None` without a
    /// policy and when no audited file is reliable (a zero there would read as
    /// "nothing remains").
    pub residual_findings: Option<usize>,
    /// Audited files whose preview is reliable (a clean audit).
    pub reliable_files: usize,
    /// Audited files whose preview is not reliable. `reliable_files +
    /// unreliable_files` is the number of audited files (not `files`: a failed
    /// file has no audit).
    pub unreliable_files: usize,
}

impl Summary {
    /// The summary of no files under `opts`.
    fn new(opts: &RenderOptions) -> Summary {
        Summary {
            files: 0,
            failed: 0,
            unsupported: 0,
            unreadable: 0,
            issues: 0,
            ok: 0,
            findings: 0,
            by_category: Category::ALL.map(|c| (c, 0usize)),
            max_severity: None,
            residual_findings: opts.policy.map(|_| 0usize),
            reliable_files: 0,
            unreliable_files: 0,
        }
    }

    /// Count one more file. The one step both [`summarize`] and
    /// [`ReportBuilder`] use, so they cannot drift. The residual is left at
    /// its running value (`Some(0)` under a policy); [`Summary::finalized`]
    /// applies the "no reliable file" rule.
    fn add_file(&mut self, entry: &FileEntry<'_>, opts: &RenderOptions) {
        self.files = self.files.saturating_add(1);
        let counter = match status_of(&entry.result) {
            Status::Failed => &mut self.failed,
            Status::Unsupported => &mut self.unsupported,
            Status::Unreadable => &mut self.unreadable,
            Status::Issues => &mut self.issues,
            Status::Ok => &mut self.ok,
        };
        *counter = counter.saturating_add(1);
        let FileResult::Audited(a) = &entry.result else {
            return;
        };
        self.findings = self.findings.saturating_add(a.findings().len());
        for f in a.findings() {
            if let Some(slot) = self.by_category.iter_mut().find(|(c, _)| *c == f.category) {
                slot.1 = slot.1.saturating_add(1);
            }
        }
        if let Some(m) = a.max_severity() {
            if self.max_severity.is_none_or(|cur| cur < m) {
                self.max_severity = Some(m);
            }
        }
        let reliable = a.health().clean && !a.health().inventory_truncated;
        if reliable {
            self.reliable_files = self.reliable_files.saturating_add(1);
            if let (Some(policy), Some(total)) = (opts.policy, self.residual_findings.as_mut()) {
                *total = total.saturating_add(preview(a, policy).residual_findings());
            }
        } else {
            self.unreliable_files = self.unreliable_files.saturating_add(1);
        }
    }

    /// The summary as reported: a residual of zero over no reliable file would
    /// read as "nothing remains", so it is `None` there.
    fn finalized(&self) -> Summary {
        let mut s = self.clone();
        if s.reliable_files == 0 {
            s.residual_findings = None;
        }
        s
    }
}

/// Summarise a run. Linear in the findings.
pub fn summarize(files: &[FileEntry<'_>], opts: &RenderOptions) -> Summary {
    let mut s = Summary::new(opts);
    for entry in files {
        s.add_file(entry, opts);
    }
    s.finalized()
}

/// The report formats a [`ReportBuilder`] can assemble.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Line-oriented text for a terminal.
    Text,
    /// One JSON document.
    Json,
    /// RFC 4180 CSV.
    Csv,
}

impl Format {
    /// A stable lowercase name: `text`, `json` or `csv`.
    pub fn name(self) -> &'static str {
        match self {
            Format::Text => "text",
            Format::Json => "json",
            Format::Csv => "csv",
        }
    }
}

/// Assembles a report one file at a time (SPEC-018).
///
/// A [`FileEntry`] borrows the bytes of its file. A run over a large
/// directory must not keep every file's bytes alive until the report is
/// rendered, so the builder renders each file's fragment in [`add`] and keeps
/// only that fragment and a running [`Summary`]; it holds no reference to the
/// entry. The JSON document puts `summary` before `files`, so [`finish`]
/// assembles the document from the fragments and the final summary.
///
/// **Output equals the one-shot renderers byte for byte.** `render_text`,
/// `render_json` and `render_csv` are this builder (new, add each, finish), so
/// there is one code path and they cannot drift.
///
/// **Memory bound.** The retained memory is the *capacity* of the fragment
/// strings (a `String` may reserve up to about twice its length, so at most a
/// small constant factor over the fragment lengths), plus one `String` header
/// per fragment and a constant for the summary and options. It grows with the
/// fragment of each added file and never holds an input byte; [`finish`]
/// allocates the document once, of the fragments' total length plus the frame.
///
/// [`add`]: ReportBuilder::add
/// [`finish`]: ReportBuilder::finish
#[derive(Debug, Clone)]
pub struct ReportBuilder {
    format: Format,
    opts: RenderOptions,
    /// One rendered fragment per added file, in `add` order.
    fragments: Vec<String>,
    /// The running summary (residual not yet finalized).
    summary: Summary,
}

impl ReportBuilder {
    /// An empty builder for `format` under `opts`.
    pub fn new(format: Format, opts: RenderOptions) -> ReportBuilder {
        ReportBuilder {
            format,
            opts,
            fragments: Vec::new(),
            summary: Summary::new(&opts),
        }
    }

    /// Render `entry`'s fragment now and count it in the summary. Keeps no
    /// borrow of `entry` or of the bytes it borrows.
    pub fn add(&mut self, entry: &FileEntry<'_>) {
        let fragment = match self.format {
            Format::Text => text::fragment(entry, &self.opts),
            Format::Json => json::fragment(entry, &self.opts),
            Format::Csv => csv::fragment(entry, &self.opts),
        };
        self.fragments.push(fragment);
        self.summary.add_file(entry, &self.opts);
    }

    /// The summary of the entries added so far; equals [`summarize`] over
    /// them.
    pub fn summary(&self) -> Summary {
        self.summary.finalized()
    }

    /// How many entries have been added.
    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    /// Whether no entry has been added.
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Assemble the document: the format's frame around the fragments and the
    /// summary of every added entry.
    pub fn finish(self) -> String {
        let summary = self.summary.finalized();
        match self.format {
            Format::Text => text::assemble(&self.fragments, &summary, &self.opts),
            Format::Json => json::assemble(&self.fragments, &summary, &self.opts),
            Format::Csv => csv::assemble(&self.fragments),
        }
    }
}

/// Render `files` in `format`: [`render_text`], [`render_json`] or
/// [`render_csv`].
pub fn render(format: Format, files: &[FileEntry<'_>], opts: &RenderOptions) -> String {
    match format {
        Format::Text => render_text(files, opts),
        Format::Json => render_json(files, opts),
        Format::Csv => render_csv(files, opts),
    }
}

/// The one-shot path every renderer takes: a builder, every file added, then
/// finished.
fn build_all(format: Format, files: &[FileEntry<'_>], opts: &RenderOptions) -> String {
    let mut builder = ReportBuilder::new(format, *opts);
    for entry in files {
        builder.add(entry);
    }
    builder.finish()
}

// ── helpers shared by the three renderers ───────────────────────────────────

/// The container's report name: `tiff`, `jpeg` or the sniffed name.
pub(crate) fn container_name(kind: ContainerKind) -> &'static str {
    match kind {
        ContainerKind::Tiff => "tiff",
        ContainerKind::Jpeg => "jpeg",
        ContainerKind::Unsupported(s) => match s {
            Sniffed::Png => "png",
            Sniffed::Webp => "webp",
            Sniffed::Heic => "heic",
            Sniffed::Gif => "gif",
            Sniffed::BigTiff => "bigtiff",
            Sniffed::Empty => "empty",
            // `Jpeg` and `Tiff` never reach `Unsupported`; the type is
            // non-exhaustive for later containers.
            _ => "unknown",
        },
    }
}

/// An IFD kind's report name.
pub(crate) fn ifd_name(kind: IfdKind) -> &'static str {
    match kind {
        IfdKind::Ifd0 => "ifd0",
        IfdKind::Ifd1 => "ifd1",
        IfdKind::Chain => "chain",
        IfdKind::Sub => "sub",
        IfdKind::Exif => "exif",
        IfdKind::Gps => "gps",
        IfdKind::Interop => "interop",
    }
}

/// A region kind's report name.
pub(crate) fn region_name(kind: crate::audit::RegionKind) -> &'static str {
    use crate::audit::RegionKind;
    match kind {
        RegionKind::Xmp => "xmp",
        RegionKind::ExtendedXmp => "extended_xmp",
        RegionKind::Iptc => "iptc",
        RegionKind::Icc => "icc",
        RegionKind::Jumbf => "jumbf",
    }
}

/// A value excerpt: display-safe text and whether bytes were cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Excerpt {
    pub text: String,
    pub truncated: bool,
}

/// The bytes of an excerpt and whether they were cut (DEC-010 part 1): for an
/// ASCII-typed entry (type 2) the payload up to the first NUL (or all of it),
/// otherwise the payload; at most [`EXCERPT_MAX`] bytes from the start.
///
/// Looks at no more than `EXCERPT_MAX + 1` bytes, so many entries pointing at
/// one large payload cost a constant each.
pub(crate) fn excerpt_bytes(field_type: u16, payload: &[u8]) -> (&[u8], bool) {
    let probe = payload
        .get(..EXCERPT_MAX.saturating_add(1))
        .unwrap_or(payload);
    let probe = if field_type == 2 {
        let nul = probe.iter().position(|b| *b == 0).unwrap_or(probe.len());
        probe.get(..nul).unwrap_or(probe)
    } else {
        probe
    };
    // One byte more than the cap proves bytes were cut.
    if probe.len() > EXCERPT_MAX {
        (probe.get(..EXCERPT_MAX).unwrap_or(probe), true)
    } else {
        (probe, false)
    }
}

/// The excerpt of an item, only when the caller asked for values and the item
/// is a tag with a readable payload.
pub(crate) fn excerpt(item: &Item<'_>, opts: &RenderOptions) -> Option<Excerpt> {
    if !opts.values {
        return None;
    }
    let ItemKind::Tag { field_type, .. } = item.kind() else {
        return None;
    };
    let (bytes, truncated) = excerpt_bytes(field_type, item.value()?);
    Some(Excerpt {
        text: safe_text(bytes),
        truncated,
    })
}

/// The preview of an audit, only when a policy is set.
pub(crate) fn preview_of(a: &FileAudit<'_>, opts: &RenderOptions) -> Option<Preview> {
    opts.policy.map(|p| preview(a, p))
}

/// The category of each inventory item that has a finding, parallel to
/// [`FileAudit::items`]. Linear in items plus findings.
pub(crate) fn item_categories(a: &FileAudit<'_>) -> Vec<Option<Category>> {
    let mut out = vec![None; a.items().len()];
    for f in a.findings() {
        if let Some(slot) = f.item.and_then(|i| out.get_mut(i)) {
            *slot = Some(f.category);
        }
    }
    out
}

/// `0x` and four lowercase hex digits.
pub(crate) fn tag_hex(tag: u16) -> String {
    format!("0x{tag:04x}")
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
    use crate::audit::audit;

    fn tiff_ascii(make: &[u8]) -> Vec<u8> {
        // IFD0 with one ASCII Make (0x010F), value out of line.
        let mut payload = make.to_vec();
        payload.push(0);
        let mut v = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&0x010Fu16.to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(&26u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&payload);
        v
    }

    #[test]
    fn status_of_follows_dec_010() {
        let failed = FileResult::Failed {
            reason: FailReason::Other,
            detail: b"x",
        };
        assert_eq!(status_of(&failed), Status::Failed);
        // unsupported, zero findings: still `unsupported`.
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
        let a = audit(&png);
        assert!(a.findings().is_empty());
        assert_eq!(
            status_of(&FileResult::Audited(a)),
            Status::Unsupported,
            "an unsupported audit with zero findings is never ok"
        );
        for bytes in [&b""[..], b"hello", b"II+\0\0\0\0\0"] {
            assert_eq!(
                status_of(&FileResult::Audited(audit(bytes))),
                Status::Unsupported
            );
        }
        // unreadable: a TIFF whose IFD0 is out of range.
        let bad = b"II\x2A\x00\x40\x00\x00\x00";
        assert_eq!(
            status_of(&FileResult::Audited(audit(bad))),
            Status::Unreadable
        );
        // issues: a JPEG with no scan.
        assert_eq!(
            status_of(&FileResult::Audited(audit(&[0xFF, 0xD8]))),
            Status::Issues
        );
        // ok: a clean TIFF, with findings or without.
        let clean = tiff_ascii(b"Cam");
        let a = audit(&clean);
        assert!(a.health().clean && !a.findings().is_empty());
        assert_eq!(status_of(&FileResult::Audited(a)), Status::Ok);
        assert_eq!(
            status_of(&FileResult::Audited(audit(b"II\x2A\x00\x00\x00\x00\x00"))),
            Status::Ok
        );
        // `ok` is exactly `health().clean` for a supported audit.
        for bytes in [&clean[..], bad, &[0xFF, 0xD8][..], b"x"] {
            let a = audit(bytes);
            let clean = a.health().clean;
            assert_eq!(
                status_of(&FileResult::Audited(a)) == Status::Ok,
                clean,
                "{bytes:?}"
            );
        }
        // Names.
        let names: Vec<&str> = [
            Status::Failed,
            Status::Unsupported,
            Status::Unreadable,
            Status::Issues,
            Status::Ok,
        ]
        .iter()
        .map(|s| s.name())
        .collect();
        assert_eq!(
            names,
            ["failed", "unsupported", "unreadable", "issues", "ok"]
        );
    }

    #[test]
    fn excerpt_cuts_ascii_at_nul_and_caps_at_64_bytes_and_flags_truncation() {
        // Cut at the first NUL.
        assert_eq!(excerpt_bytes(2, b"abc\0def"), (&b"abc"[..], false));
        // No NUL: the whole payload.
        assert_eq!(excerpt_bytes(2, b"abcdef"), (&b"abcdef"[..], false));
        // Exactly 64 bytes, no NUL: not truncated.
        let p64 = [b'a'; 64];
        assert_eq!(excerpt_bytes(2, &p64), (&p64[..], false));
        // 65 bytes: cut to 64 and flagged.
        let p65 = [b'a'; 65];
        assert_eq!(excerpt_bytes(2, &p65), (&p65[..64], true));
        // 64 bytes then a NUL: the NUL ends it exactly at the cap, not cut.
        let mut p = vec![b'a'; 64];
        p.push(0);
        p.extend_from_slice(b"secret tail");
        assert_eq!(excerpt_bytes(2, &p), (&p[..64], false));
        // 300 bytes: cut and flagged.
        let long = [b'x'; 300];
        assert_eq!(excerpt_bytes(2, &long), (&long[..64], true));
        // A NUL first: empty, not truncated.
        assert_eq!(excerpt_bytes(2, b"\0abc"), (&b""[..], false));
        assert_eq!(excerpt_bytes(2, b""), (&b""[..], false));
    }

    #[test]
    fn excerpt_of_a_non_ascii_type_is_the_raw_payload_prefix() {
        // Other types keep their NULs and are cut at 64.
        assert_eq!(excerpt_bytes(7, b"ab\0cd"), (&b"ab\0cd"[..], false));
        assert_eq!(
            excerpt_bytes(4, &[1, 0, 0, 0]),
            (&[1u8, 0, 0, 0][..], false)
        );
        let long = [9u8; 100];
        assert_eq!(excerpt_bytes(7, &long), (&long[..64], true));
        assert_eq!(excerpt_bytes(1, &long[..64]), (&long[..64], false));
        // And the item-level excerpt is the safe text of those bytes, only
        // when values are on.
        let t = tiff_ascii(b"Canon\x01\"");
        let a = audit(&t);
        let item = a.items()[0];
        let on = RenderOptions {
            values: true,
            ..RenderOptions::default()
        };
        let e = excerpt(&item, &on).unwrap();
        assert_eq!(e.text, "Canon\\x01\\\"");
        assert!(!e.truncated);
        assert_eq!(excerpt(&item, &RenderOptions::default()), None);
    }

    #[test]
    fn summarize_counts_each_status_category_and_reliability() {
        let clean = tiff_ascii(b"Cam");
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let bad = b"II\x2A\x00\x40\x00\x00\x00";
        let issues = [0xFF, 0xD8];
        let files = vec![
            FileEntry {
                label: b"a",
                result: FileResult::Audited(audit(&clean)),
            },
            FileEntry {
                label: b"b",
                result: FileResult::Audited(audit(&png)),
            },
            FileEntry {
                label: b"c",
                result: FileResult::Audited(audit(bad)),
            },
            FileEntry {
                label: b"d",
                result: FileResult::Audited(audit(&issues)),
            },
            FileEntry {
                label: b"e",
                result: FileResult::Failed {
                    reason: FailReason::TooLarge,
                    detail: b"big",
                },
            },
            FileEntry {
                label: b"f",
                result: FileResult::Audited(audit(&clean)),
            },
        ];
        let none = summarize(&files, &RenderOptions::default());
        assert_eq!(none.files, 6);
        assert_eq!(
            (
                none.failed,
                none.unsupported,
                none.unreadable,
                none.issues,
                none.ok
            ),
            (1, 1, 1, 1, 2)
        );
        assert_eq!(none.findings, 2);
        assert_eq!(none.max_severity, Some(Severity::Low));
        let cam = none
            .by_category
            .iter()
            .find(|(c, _)| *c == Category::CameraModel)
            .unwrap();
        assert_eq!(cam.1, 2);
        assert_eq!(
            none.by_category.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
            Category::ALL.to_vec()
        );
        assert_eq!(none.by_category.iter().map(|x| x.1).sum::<usize>(), 2);
        assert_eq!(none.residual_findings, None);
        // Audited files: 5; reliable are the two clean ones.
        assert_eq!((none.reliable_files, none.unreliable_files), (2, 3));
        assert_eq!(
            none.reliable_files + none.unreliable_files,
            none.files - none.failed
        );
        // With a policy that removes CameraModel nothing remains; with Gps
        // both findings remain, summed over reliable files only.
        let removing = RenderOptions {
            policy: Some(StripPolicy::AllPii),
            ..RenderOptions::default()
        };
        assert_eq!(summarize(&files, &removing).residual_findings, Some(0));
        let keeping = RenderOptions {
            policy: Some(StripPolicy::Gps),
            ..RenderOptions::default()
        };
        assert_eq!(summarize(&files, &keeping).residual_findings, Some(2));
        // No reliable file: no residual number at all (never a zero).
        let only_png = [FileEntry {
            label: b"b",
            result: FileResult::Audited(audit(&png)),
        }];
        assert_eq!(summarize(&only_png, &keeping).residual_findings, None);
        // An empty run.
        let empty = summarize(&[], &keeping);
        assert_eq!(empty.files, 0);
        assert_eq!(empty.residual_findings, None);
        assert_eq!(empty.max_severity, None);
    }
}
