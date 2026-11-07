//! The JSON report (DEC-010 part 3): one document, UTF-8, `schema_version` 1.
//!
//! Keys are written in a fixed order by this code and nowhere else; there is
//! no JSON dependency and no generic serializer, so no key can be added,
//! reordered or duplicated by accident. Every string that is not one of this
//! library's own constants is `safe_text` first and then JSON-escaped (the
//! second layer cannot add a control character, but is checked on its own).
//! Numbers are non-negative integers.

use crate::audit::{Finding, Item, ItemKind, Preview};

use super::{
    container_name, excerpt, ifd_name, item_categories, preview_of, region_name, safe_text,
    status_of, summarize, tag_hex, FileEntry, FileResult, RenderOptions,
};

/// The schema version; a change to any key or enumeration bumps it.
pub(crate) const SCHEMA_VERSION: usize = 1;

/// Append `text` as a JSON string literal (RFC 8259 escaping).
pub(super) fn string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if u32::from(c) < 0x20 => {
                out.push_str("\\u00");
                let n = u32::from(c);
                out.push(char::from_digit(n / 16, 16).unwrap_or('0'));
                out.push(char::from_digit(n % 16, 16).unwrap_or('0'));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn opt_string(out: &mut String, text: Option<&str>) {
    match text {
        Some(t) => string(out, t),
        None => out.push_str("null"),
    }
}

fn number(out: &mut String, n: usize) {
    out.push_str(&n.to_string());
}

fn opt_number(out: &mut String, n: Option<usize>) {
    match n {
        Some(n) => number(out, n),
        None => out.push_str("null"),
    }
}

fn boolean(out: &mut String, b: bool) {
    out.push_str(if b { "true" } else { "false" });
}

/// `"key":` (the key is one of this library's own ASCII constants).
fn key(out: &mut String, name: &str) {
    out.push('"');
    out.push_str(name);
    out.push_str("\":");
}

fn where_json(out: &mut String, item: &Item<'_>) {
    match item.kind() {
        ItemKind::Tag {
            ifd,
            ifd_index,
            tag,
            field_type,
            count,
        } => {
            out.push_str("{\"kind\":\"tag\",\"ifd\":");
            string(out, ifd_name(ifd));
            out.push_str(",\"ifd_index\":");
            number(out, ifd_index);
            out.push_str(",\"tag\":");
            string(out, &tag_hex(tag));
            out.push_str(",\"field_type\":");
            number(out, usize::from(field_type));
            out.push_str(",\"count\":");
            number(out, usize::try_from(count).unwrap_or(usize::MAX));
            out.push('}');
        }
        ItemKind::Region { kind, segment, len } => {
            out.push_str("{\"kind\":\"region\",\"region\":");
            string(out, region_name(kind));
            out.push_str(",\"segment\":");
            number(out, segment);
            out.push_str(",\"len\":");
            number(out, len);
            out.push('}');
        }
    }
}

/// `"block":..,"segment":..,"after_scan":..` for an item's origin.
fn origin_json(out: &mut String, item: &Item<'_>) {
    let o = item.origin();
    out.push_str("\"block\":");
    opt_number(out, o.block);
    out.push_str(",\"segment\":");
    opt_number(out, o.segment);
    out.push_str(",\"after_scan\":");
    boolean(out, o.after_scan);
}

fn value_json(out: &mut String, item: &Item<'_>, opts: &RenderOptions) {
    match excerpt(item, opts) {
        Some(e) => {
            out.push_str("{\"text\":");
            string(out, &e.text);
            out.push_str(",\"truncated\":");
            boolean(out, e.truncated);
            out.push('}');
        }
        None => out.push_str("null"),
    }
}

fn finding_json(
    out: &mut String,
    f: &Finding<'_>,
    disposition: Option<&str>,
    opts: &RenderOptions,
) {
    out.push_str("{\"category\":");
    string(out, f.category.name());
    out.push_str(",\"severity\":");
    string(out, f.category.severity().name());
    out.push_str(",\"at\":");
    where_json(out, &f.at);
    out.push(',');
    origin_json(out, &f.at);
    out.push_str(",\"disposition\":");
    opt_string(out, disposition);
    out.push_str(",\"thumbnail\":");
    match f.thumbnail {
        Some(t) => {
            out.push_str("{\"offset\":");
            opt_number(
                out,
                t.range
                    .map(|(o, _)| usize::try_from(o).unwrap_or(usize::MAX)),
            );
            out.push_str(",\"length\":");
            opt_number(
                out,
                t.range
                    .map(|(_, l)| usize::try_from(l).unwrap_or(usize::MAX)),
            );
            out.push_str(",\"starts_with_ffd8\":");
            boolean(out, t.starts_with_ffd8);
            out.push('}');
        }
        None => out.push_str("null"),
    }
    out.push_str(",\"value\":");
    value_json(out, &f.at, opts);
    out.push('}');
}

fn preview_json(out: &mut String, p: &Preview) {
    let c = p.counts();
    out.push_str("{\"policy\":");
    string(out, p.policy().name());
    out.push_str(",\"counts\":{\"removed\":");
    number(out, c.removed);
    out.push_str(",\"kept\":");
    number(out, c.kept);
    out.push_str(",\"kept_unread\":");
    number(out, c.kept_unread);
    out.push_str("},\"reliable\":");
    boolean(out, p.reliable());
    // DEC-009 amendment: residual numbers mean nothing unless the audit was
    // read cleanly, so they are null otherwise.
    out.push_str(",\"residual_findings\":");
    opt_number(out, p.reliable().then(|| p.residual_findings()));
    out.push_str(",\"residual_severity\":");
    opt_string(
        out,
        p.reliable()
            .then(|| p.residual_severity())
            .flatten()
            .map(|s| s.name()),
    );
    out.push('}');
}

fn file_json(out: &mut String, entry: &FileEntry<'_>, opts: &RenderOptions) {
    out.push_str("{\"label\":");
    string(out, &safe_text(entry.label));
    out.push_str(",\"status\":");
    string(out, status_of(&entry.result).name());
    match &entry.result {
        FileResult::Failed { reason, detail } => {
            out.push_str(",\"container\":null,\"failure\":{\"reason\":");
            string(out, reason.name());
            out.push_str(",\"detail\":");
            string(out, &safe_text(detail));
            out.push_str(
                "},\"health\":null,\"max_severity\":null,\"inventory_items\":0,\
                 \"findings\":[],\"inventory\":null,\"preview\":null}",
            );
        }
        FileResult::Audited(a) => {
            out.push_str(",\"container\":");
            string(out, container_name(a.kind()));
            let h = a.health();
            out.push_str(",\"failure\":null,\"health\":{\"clean\":");
            boolean(out, h.clean);
            out.push_str(",\"jpeg_issues\":");
            number(out, h.jpeg_issues);
            out.push_str(",\"exif_issues\":");
            number(out, h.exif_issues);
            out.push_str(",\"failed_blocks\":");
            number(out, h.failed_blocks);
            out.push_str(",\"inventory_truncated\":");
            boolean(out, h.inventory_truncated);
            out.push_str("},\"max_severity\":");
            opt_string(out, a.max_severity().map(|s| s.name()));
            out.push_str(",\"inventory_items\":");
            number(out, a.items().len());
            let pv = preview_of(a, opts);
            out.push_str(",\"findings\":[");
            for (i, f) in a.findings().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let d = pv.as_ref().and_then(|p| p.disposition(i)).map(|d| d.name());
                finding_json(out, f, d, opts);
            }
            out.push_str("],\"inventory\":");
            if opts.inventory {
                let cats = item_categories(a);
                out.push('[');
                for (i, item) in a.items().iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str("{\"at\":");
                    where_json(out, item);
                    out.push(',');
                    origin_json(out, item);
                    out.push_str(",\"category\":");
                    opt_string(out, cats.get(i).copied().flatten().map(|c| c.name()));
                    out.push_str(",\"value\":");
                    value_json(out, item, opts);
                    out.push('}');
                }
                out.push(']');
            } else {
                out.push_str("null");
            }
            out.push_str(",\"preview\":");
            match &pv {
                Some(p) => preview_json(out, p),
                None => out.push_str("null"),
            }
            out.push('}');
        }
    }
}

/// Render the report as one JSON document (DEC-010 part 3), followed by a
/// newline. Well-formed for any input bytes; deterministic.
pub fn render_json(files: &[FileEntry<'_>], opts: &RenderOptions) -> String {
    let s = summarize(files, opts);
    let mut out = String::new();
    out.push_str("{\"schema_version\":");
    number(&mut out, SCHEMA_VERSION);
    out.push_str(",\"policy\":");
    opt_string(&mut out, opts.policy.map(|p| p.name()));
    out.push_str(",\"options\":{\"inventory\":");
    boolean(&mut out, opts.inventory);
    out.push_str(",\"values\":");
    boolean(&mut out, opts.values);
    out.push_str("},\"summary\":{\"files\":");
    number(&mut out, s.files);
    for (name, n) in [
        ("failed", s.failed),
        ("unsupported", s.unsupported),
        ("unreadable", s.unreadable),
        ("issues", s.issues),
        ("ok", s.ok),
        ("findings", s.findings),
    ] {
        out.push(',');
        key(&mut out, name);
        number(&mut out, n);
    }
    out.push_str(",\"by_category\":{");
    for (i, (c, n)) in s.by_category.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        key(&mut out, c.name());
        number(&mut out, *n);
    }
    out.push_str("},\"max_severity\":");
    opt_string(&mut out, s.max_severity.map(|m| m.name()));
    out.push_str(",\"residual_findings\":");
    opt_number(&mut out, s.residual_findings);
    out.push_str(",\"reliable_files\":");
    number(&mut out, s.reliable_files);
    out.push_str(",\"unreliable_files\":");
    number(&mut out, s.unreliable_files);
    out.push_str("},\"files\":[");
    for (i, entry) in files.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        file_json(&mut out, entry, opts);
    }
    out.push_str("]}\n");
    out
}
