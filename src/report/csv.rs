//! The CSV report (DEC-010 part 4): RFC 4180, CRLF line ends, a fixed header,
//! 19 fields in every row.

use crate::audit::{FileAudit, Item, ItemKind};

use super::{
    container_name, excerpt, ifd_name, preview_of, region_name, safe_text, status_of, tag_hex,
    FileEntry, FileResult, Format, RenderOptions,
};

/// The header row, without its line end. Column order is API.
pub(crate) const HEADER: &str = "label,status,container,category,severity,kind,ifd,ifd_index,tag,\
field_type,count,region,region_len,block,segment,after_scan,disposition,value,value_truncated";

/// The number of columns in every row.
pub(crate) const COLUMNS: usize = 19;

pub(super) const LABEL: usize = 0;
pub(super) const STATUS: usize = 1;
pub(super) const CONTAINER: usize = 2;
pub(super) const CATEGORY: usize = 3;
pub(super) const SEVERITY: usize = 4;
pub(super) const KIND: usize = 5;
pub(super) const IFD: usize = 6;
pub(super) const IFD_INDEX: usize = 7;
pub(super) const TAG: usize = 8;
pub(super) const FIELD_TYPE: usize = 9;
pub(super) const COUNT: usize = 10;
pub(super) const REGION: usize = 11;
pub(super) const REGION_LEN: usize = 12;
pub(super) const BLOCK: usize = 13;
pub(super) const SEGMENT: usize = 14;
pub(super) const AFTER_SCAN: usize = 15;
pub(super) const DISPOSITION: usize = 16;
pub(super) const VALUE: usize = 17;
pub(super) const VALUE_TRUNCATED: usize = 18;

pub(super) type Row = [String; COLUMNS];

/// Whether a hostile field starts with a character a spreadsheet may take as
/// the start of a formula (OWASP "CSV Injection").
pub(super) fn starts_like_formula(text: &str) -> bool {
    matches!(
        text.chars().next(),
        Some('=' | '+' | '-' | '@' | '\t' | '\r')
    )
}

/// One CSV field: quoted iff it holds a comma, a semicolon, a quote, CR or LF;
/// quotes are doubled. The semicolon is the field separator of spreadsheets in
/// semicolon locales: an unquoted `a;=1+cmd` would split into a cell that
/// starts a formula, past the guard.
pub(super) fn push_field(out: &mut String, text: &str) {
    if text.contains([',', ';', '"', '\r', '\n']) {
        out.push('"');
        for c in text.chars() {
            if c == '"' {
                out.push('"');
            }
            out.push(c);
        }
        out.push('"');
    } else {
        out.push_str(text);
    }
}

pub(super) fn push_row(out: &mut String, row: &Row) {
    for (i, text) in row.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        // The formula guard changes hostile data on purpose and applies to
        // the label, the value (an excerpt or a failure detail) and nothing
        // else.
        if (i == LABEL || i == VALUE) && starts_like_formula(text) {
            let mut guarded = String::with_capacity(text.len().saturating_add(1));
            guarded.push('\'');
            guarded.push_str(text);
            push_field(out, &guarded);
        } else {
            push_field(out, text);
        }
    }
    out.push_str("\r\n");
}

fn base_row(entry: &FileEntry<'_>) -> Row {
    let mut row = Row::default();
    row[LABEL] = safe_text(entry.label);
    row[STATUS] = status_of(&entry.result).name().to_string();
    if let FileResult::Audited(a) = &entry.result {
        row[CONTAINER] = container_name(a.kind()).to_string();
    }
    row
}

fn put_location(row: &mut Row, item: &Item<'_>) {
    match item.kind() {
        ItemKind::Tag {
            ifd,
            ifd_index,
            tag,
            field_type,
            count,
        } => {
            row[KIND] = "tag".to_string();
            row[IFD] = ifd_name(ifd).to_string();
            row[IFD_INDEX] = ifd_index.to_string();
            row[TAG] = tag_hex(tag);
            row[FIELD_TYPE] = field_type.to_string();
            row[COUNT] = count.to_string();
        }
        ItemKind::Region { kind, len, .. } => {
            row[KIND] = "region".to_string();
            row[REGION] = region_name(kind).to_string();
            row[REGION_LEN] = len.to_string();
        }
    }
    let o = item.origin();
    row[BLOCK] = o.block.map(|b| b.to_string()).unwrap_or_default();
    row[SEGMENT] = o.segment.map(|s| s.to_string()).unwrap_or_default();
    row[AFTER_SCAN] = o.after_scan.to_string();
}

fn put_value(row: &mut Row, item: &Item<'_>, opts: &RenderOptions) {
    if let Some(e) = excerpt(item, opts) {
        row[VALUE] = e.text;
        row[VALUE_TRUNCATED] = e.truncated.to_string();
    }
}

fn audited_rows(out: &mut String, entry: &FileEntry<'_>, a: &FileAudit<'_>, opts: &RenderOptions) {
    let base = base_row(entry);
    let pv = preview_of(a, opts);
    let disposition = |finding: usize| -> String {
        pv.as_ref()
            .and_then(|p| p.disposition(finding))
            .map(|d| d.name().to_string())
            .unwrap_or_default()
    };
    let mut rows = 0usize;
    let finding_row = |f_index: usize, f: &crate::audit::Finding<'_>| -> Row {
        let mut row = base.clone();
        row[CATEGORY] = f.category.name().to_string();
        row[SEVERITY] = f.category.severity().name().to_string();
        row[DISPOSITION] = disposition(f_index);
        put_location(&mut row, &f.at);
        put_value(&mut row, &f.at, opts);
        row
    };
    if opts.inventory {
        // The finding (if any) of each item, then the findings whose item did
        // not fit in the inventory.
        let mut of_item: Vec<Option<usize>> = vec![None; a.items().len()];
        for (fi, f) in a.findings().iter().enumerate() {
            if let Some(slot) = f.item.and_then(|i| of_item.get_mut(i)) {
                *slot = Some(fi);
            }
        }
        for (item, finding) in a.items().iter().zip(&of_item) {
            let mut row = match finding.and_then(|fi| a.findings().get(fi).map(|f| (fi, f))) {
                Some((fi, f)) => finding_row(fi, f),
                None => {
                    let mut row = base.clone();
                    put_location(&mut row, item);
                    put_value(&mut row, item, opts);
                    row
                }
            };
            // The item's own origin and payload (equal to the finding's).
            put_location(&mut row, item);
            put_value(&mut row, item, opts);
            push_row(out, &row);
            rows = rows.saturating_add(1);
        }
        for (fi, f) in a.findings().iter().enumerate() {
            if f.item.is_none() {
                push_row(out, &finding_row(fi, f));
                rows = rows.saturating_add(1);
            }
        }
    } else {
        for (fi, f) in a.findings().iter().enumerate() {
            push_row(out, &finding_row(fi, f));
            rows = rows.saturating_add(1);
        }
    }
    if rows == 0 {
        // A file with no findings (and no listed items) is still visible.
        push_row(out, &base);
    }
}

/// One file's fragment: its rows (a file with no row otherwise gets one).
pub(super) fn fragment(entry: &FileEntry<'_>, opts: &RenderOptions) -> String {
    let mut out = String::new();
    match &entry.result {
        FileResult::Audited(a) => audited_rows(&mut out, entry, a, opts),
        FileResult::Failed { detail, .. } => {
            let mut row = base_row(entry);
            row[VALUE] = safe_text(detail);
            push_row(&mut out, &row);
        }
    }
    out
}

/// The report: the header row, then the fragments.
pub(super) fn assemble(fragments: &[String]) -> String {
    let mut out = String::new();
    out.push_str(HEADER);
    out.push_str("\r\n");
    for fragment in fragments {
        out.push_str(fragment);
    }
    out
}

/// Render the report as CSV (RFC 4180, CRLF), one row per finding, plus one row
/// for every file that has no row otherwise, so no file is ever absent. With
/// `inventory` on, one row per inventory item replaces the finding rows of
/// items that have findings (findings whose item did not fit follow).
///
/// **The formula guard changes data.** A hostile field (the label, the value
/// excerpt, or a failure detail carried in `value`) that begins with `=`, `+`,
/// `-`, `@`, TAB or CR is prefixed with a single quote before quoting, so a
/// spreadsheet does not take it for a formula (DEC-010 part 4). JSON and text
/// apply no such guard. This is a [`ReportBuilder`](super::ReportBuilder) with
/// every file added.
pub fn render_csv(files: &[FileEntry<'_>], opts: &RenderOptions) -> String {
    super::build_all(Format::Csv, files, opts)
}
