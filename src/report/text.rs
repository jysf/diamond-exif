//! The text report (DEC-010): line-oriented, UTF-8, `\n` line ends, for a
//! person at a terminal.
//!
//! Every hostile string goes through `safe_text`, so the output holds no
//! control character other than `\n`, no bidi control, no C1 control and no
//! DEL. The words "no personal data", "safe" and "clean file" are never used:
//! a count of zero is printed as `0 findings` beside the status, and the
//! residual numbers of a policy preview appear only for a reliable audit.

use crate::audit::{Disposition, Finding, Item, ItemKind, Preview};

use super::{
    container_name, excerpt, ifd_name, item_categories, preview_of, region_name, safe_text,
    status_of, tag_hex, FileEntry, FileResult, Format, RenderOptions, Summary,
};

/// The disposition's wording (DEC-009: `kept` is not an all-clear).
fn disposition_text(d: Disposition) -> &'static str {
    match d {
        Disposition::Removed => "removed",
        Disposition::Kept => "kept (not targeted by this policy)",
        Disposition::KeptUnread => "kept_unread (may hold what this policy removes)",
    }
}

fn count_line(out: &mut String, n: usize) {
    out.push_str("  ");
    out.push_str(&n.to_string());
    out.push_str(if n == 1 { " finding\n" } else { " findings\n" });
}

fn opt_index(n: Option<usize>) -> String {
    n.map_or_else(|| "-".to_string(), |n| n.to_string())
}

/// Where an item is, on one line.
fn location(out: &mut String, item: &Item<'_>) {
    match item.kind() {
        ItemKind::Tag {
            ifd,
            ifd_index,
            tag,
            field_type,
            count,
        } => {
            out.push_str(&format!(
                "tag {} ifd={}#{} type={} count={}",
                tag_hex(tag),
                ifd_name(ifd),
                ifd_index,
                field_type,
                count
            ));
        }
        ItemKind::Region { kind, len, .. } => {
            out.push_str(&format!("region {} len={}", region_name(kind), len));
        }
    }
    let o = item.origin();
    out.push_str(&format!(
        " block={} segment={}",
        opt_index(o.block),
        opt_index(o.segment)
    ));
    if o.after_scan {
        out.push_str(" after_scan");
    }
}

fn value_text(out: &mut String, item: &Item<'_>, opts: &RenderOptions) {
    if let Some(e) = excerpt(item, opts) {
        out.push_str(" value=\"");
        out.push_str(&e.text);
        out.push('"');
        if e.truncated {
            out.push_str(" (truncated)");
        }
    }
}

fn finding_line(
    out: &mut String,
    f: &Finding<'_>,
    disposition: Option<Disposition>,
    opts: &RenderOptions,
) {
    out.push_str(&format!(
        "    - {} [{}] ",
        f.category.name(),
        f.category.severity().name()
    ));
    location(out, &f.at);
    if let Some(t) = f.thumbnail {
        match t.range {
            Some((offset, length)) => out.push_str(&format!(
                " thumbnail offset={offset} length={length} starts_with_ffd8={}",
                t.starts_with_ffd8
            )),
            None => out.push_str(" thumbnail range=none"),
        }
    }
    if let Some(e) = f.embedded {
        let holds: Vec<&str> = e.holds.categories().map(|c| c.name()).collect();
        out.push_str(&format!(
            " embedded offset={} length={} starts_with_ffd8={} holds=[{}]",
            e.offset,
            e.length,
            e.starts_with_ffd8,
            holds.join(",")
        ));
    }
    value_text(out, &f.at, opts);
    if let Some(d) = disposition {
        out.push_str(": ");
        out.push_str(disposition_text(d));
    }
    out.push('\n');
}

fn preview_lines(out: &mut String, p: &Preview) {
    if p.reliable() {
        let c = p.counts();
        out.push_str(&format!(
            "  preview ({}): removed {}, kept {}, kept_unread {}; residual findings {}, residual severity {}\n",
            p.policy().name(),
            c.removed,
            c.kept,
            c.kept_unread,
            p.residual_findings(),
            p.residual_severity().map_or("none", |s| s.name())
        ));
    } else {
        out.push_str("  preview not reliable: the file was not read cleanly\n");
    }
}

fn file_block(out: &mut String, entry: &FileEntry<'_>, opts: &RenderOptions) {
    out.push_str("file: ");
    out.push_str(&safe_text(entry.label));
    out.push('\n');
    out.push_str("  status: ");
    out.push_str(status_of(&entry.result).name());
    out.push('\n');
    match &entry.result {
        FileResult::Failed { reason, detail } => {
            out.push_str("  failure: ");
            out.push_str(reason.name());
            out.push_str(": ");
            out.push_str(&safe_text(detail));
            out.push('\n');
            count_line(out, 0);
        }
        FileResult::Audited(a) => {
            out.push_str("  container: ");
            out.push_str(container_name(a.kind()));
            out.push('\n');
            let h = a.health();
            out.push_str(&format!(
                "  health: jpeg_issues={} exif_issues={} failed_blocks={} inventory_truncated={}\n",
                h.jpeg_issues, h.exif_issues, h.failed_blocks, h.inventory_truncated
            ));
            out.push_str("  max severity: ");
            out.push_str(a.max_severity().map_or("none", |s| s.name()));
            out.push('\n');
            count_line(out, a.findings().len());
            let pv = preview_of(a, opts);
            for (i, f) in a.findings().iter().enumerate() {
                let d = pv.as_ref().and_then(|p| p.disposition(i));
                finding_line(out, f, d, opts);
            }
            if let Some(p) = &pv {
                preview_lines(out, p);
            }
            if opts.inventory {
                out.push_str(&format!("  inventory: {} items\n", a.items().len()));
                let cats = item_categories(a);
                for (i, item) in a.items().iter().enumerate() {
                    out.push_str("    - ");
                    location(out, item);
                    if let Some(c) = cats.get(i).copied().flatten() {
                        out.push_str(&format!(" category={}", c.name()));
                    }
                    value_text(out, item, opts);
                    out.push('\n');
                }
            }
        }
    }
}

/// One file's fragment: its block and the blank line after it.
pub(super) fn fragment(entry: &FileEntry<'_>, opts: &RenderOptions) -> String {
    let mut out = String::new();
    file_block(&mut out, entry, opts);
    out.push('\n');
    out
}

/// The report: the fragments, then the summary block.
pub(super) fn assemble(fragments: &[String], s: &Summary, opts: &RenderOptions) -> String {
    let mut out = String::new();
    for fragment in fragments {
        out.push_str(fragment);
    }
    out.push_str("summary\n");
    for (name, n) in [
        ("files", s.files),
        ("ok", s.ok),
        ("issues", s.issues),
        ("unreadable", s.unreadable),
        ("unsupported", s.unsupported),
        ("failed", s.failed),
        ("findings", s.findings),
    ] {
        out.push_str(&format!("  {name}: {n}\n"));
    }
    out.push_str("  max severity: ");
    out.push_str(s.max_severity.map_or("none", |m| m.name()));
    out.push('\n');
    for (c, n) in s.by_category.iter().filter(|(_, n)| *n > 0) {
        out.push_str(&format!("  category {}: {n}\n", c.name()));
    }
    out.push_str(&format!(
        "  reliable files: {}, unreliable files: {}\n",
        s.reliable_files, s.unreliable_files
    ));
    if let Some(policy) = opts.policy {
        match s.residual_findings {
            Some(residual) => out.push_str(&format!(
                "  policy {}: residual findings {residual} (reliable files only)\n",
                policy.name()
            )),
            None => out.push_str(&format!(
                "  policy {}: residual not available: no file was read reliably\n",
                policy.name()
            )),
        }
    }
    out
}

/// Render the report as text (see the module docs). Well-formed for any input
/// bytes; deterministic. This is a [`ReportBuilder`](super::ReportBuilder)
/// with every file added.
pub fn render_text(files: &[FileEntry<'_>], opts: &RenderOptions) -> String {
    super::build_all(Format::Text, files, opts)
}
