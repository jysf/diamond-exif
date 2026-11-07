// The SPEC-015 invariants of the report renderers, shared by the fuzz target
// (`fuzz/fuzz_targets/audit_render.rs`) and the stable-toolchain layout test
// (`tests/fuzz_layout.rs`) with `include!`. Not a crate root: text included at
// module level, defining one module, `render_invariants`, which needs the
// modules `audit_invariants` and `render_check` included before it. `cargo
// fmt` does not follow `include!`, so format this file by hand
// (`rustfmt --edition 2021 <file>`).
//
// Same method as `audit_invariants.rs`: the report is re-derived here from
// DEC-010, DEC-009 and the SPEC-015 schema WRITTEN AGAIN (the key lists, the
// enumerations, the CSV header, the escape list through `naive_safe_text`, the
// DEC-009 grid through `audit_invariants::H_PRESETS`); nothing is imported from
// `exif_core::report` except the renderers under test and their option and
// entry types. The library's three outputs are parsed by the independent
// strict parsers and compared with the re-derivation by named GUARDS. Each
// guard is a pure function of one `Ctx` and runs ALONE through `run_guard`, so
// the plants in `tests/fuzz_layout.rs` show each one going red on its own bug.

#[allow(dead_code)]
mod render_invariants {
    use super::audit_invariants::{h_want, rederive, severity_of, Facts, What, H_PRESETS};
    use super::render_check::{naive_safe_text, parse_csv, parse_json, J};
    use exif_core::audit::audit;
    use exif_core::jpeg;
    use exif_core::report::{
        render_csv, render_json, render_text, summarize, FailReason, FileEntry, FileResult,
        RenderOptions,
    };
    use exif_core::tiff::Container;
    use exif_core::{StripPolicy, StripSpec};

    thread_local! {
        /// Work units the render guards have done on this thread (one per
        /// element they touch), counted instead of timed: see
        /// `render_guards_are_linear` in `tests/fuzz_layout.rs`.
        static STEPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    fn tick(n: usize) {
        STEPS.with(|s| s.set(s.get().saturating_add(n as u64)));
    }

    pub fn steps() -> u64 {
        STEPS.with(std::cell::Cell::get)
    }

    pub fn reset_steps() {
        STEPS.with(|s| s.set(0));
    }

    // ── DEC-010 and the SPEC-015 schema, typed again ────────────────────────

    /// `Debug` name and report name of every category, in `Category::ALL` order.
    pub const CATS: [(&str, &str); 13] = [
        ("Gps", "gps"),
        ("DeviceId", "device_id"),
        ("Authorship", "authorship"),
        ("FreeText", "free_text"),
        ("Thumbnail", "thumbnail"),
        ("MakerNote", "maker_note"),
        ("Xmp", "xmp"),
        ("Iptc", "iptc"),
        ("Jumbf", "jumbf"),
        ("Timestamps", "timestamps"),
        ("CameraModel", "camera_model"),
        ("Software", "software"),
        ("Icc", "icc"),
    ];

    pub const STATUSES: [&str; 5] = ["failed", "unsupported", "unreadable", "issues", "ok"];
    pub const CONTAINERS: [&str; 9] = [
        "tiff", "jpeg", "png", "webp", "heic", "gif", "bigtiff", "unknown", "empty",
    ];
    pub const REASONS: [&str; 4] = ["unreadable", "too_large", "not_regular_file", "other"];
    pub const IFDS: [&str; 8] = [
        "ifd0", "ifd1", "chain", "sub", "exif", "gps", "interop", "other",
    ];
    pub const REGIONS: [&str; 5] = ["xmp", "extended_xmp", "iptc", "icc", "jumbf"];
    pub const DISPS: [&str; 3] = ["removed", "kept", "kept_unread"];
    pub const SEVS: [&str; 3] = ["low", "medium", "high"];
    pub const POLICY_NAMES: [&str; 6] = [
        "gps",
        "gps_and_device",
        "all_pii",
        "all",
        "everything",
        "custom",
    ];

    pub const CSV_HEADER: [&str; 19] = [
        "label",
        "status",
        "container",
        "category",
        "severity",
        "kind",
        "ifd",
        "ifd_index",
        "tag",
        "field_type",
        "count",
        "region",
        "region_len",
        "block",
        "segment",
        "after_scan",
        "disposition",
        "value",
        "value_truncated",
    ];

    const FILE_KEYS: [&str; 10] = [
        "label",
        "status",
        "container",
        "failure",
        "health",
        "max_severity",
        "inventory_items",
        "findings",
        "inventory",
        "preview",
    ];
    const SUMMARY_KEYS: [&str; 12] = [
        "files",
        "failed",
        "unsupported",
        "unreadable",
        "issues",
        "ok",
        "findings",
        "by_category",
        "max_severity",
        "residual_findings",
        "reliable_files",
        "unreliable_files",
    ];
    const FINDING_KEYS: [&str; 9] = [
        "category",
        "severity",
        "at",
        "block",
        "segment",
        "after_scan",
        "disposition",
        "thumbnail",
        "value",
    ];
    const ITEM_KEYS: [&str; 6] = ["at", "block", "segment", "after_scan", "category", "value"];
    const TAG_AT_KEYS: [&str; 6] = ["kind", "ifd", "ifd_index", "tag", "field_type", "count"];
    const REGION_AT_KEYS: [&str; 4] = ["kind", "region", "segment", "len"];
    const HEALTH_KEYS: [&str; 5] = [
        "clean",
        "jpeg_issues",
        "exif_issues",
        "failed_blocks",
        "inventory_truncated",
    ];
    const PREVIEW_KEYS: [&str; 5] = [
        "policy",
        "counts",
        "reliable",
        "residual_findings",
        "residual_severity",
    ];

    fn snake(debug_name: &str) -> &'static str {
        CATS.iter()
            .find(|c| c.0 == debug_name)
            .map(|c| c.1)
            .unwrap_or("?")
    }

    fn lower(s: &str) -> String {
        s.to_lowercase()
    }

    /// DEC-010 part 5, re-derived from the flattened audit.
    pub fn status_of_facts(f: &Facts) -> &'static str {
        if f.kind.starts_with("Unsupported") {
            "unsupported"
        } else if f.health.clean {
            "ok"
        } else if f.health.failed_blocks > 0 {
            "unreadable"
        } else {
            "issues"
        }
    }

    fn container_of(f: &Facts) -> String {
        match f.kind.as_str() {
            "Tiff" => "tiff".to_string(),
            "Jpeg" => "jpeg".to_string(),
            other => lower(
                other
                    .trim_start_matches("Unsupported(")
                    .trim_end_matches(')'),
            ),
        }
    }

    /// A custom policy's category set from one input byte; bit `i` is
    /// `Category::ALL[i]`, which is `CATS[i]`.
    pub fn custom_mask(b: u8) -> u16 {
        (u16::from(b) | (u16::from(b.rotate_left(3)) << 8)) & 0x1FFF
    }

    fn removed_names(policy: Option<usize>, mask: u16) -> Vec<&'static str> {
        match policy {
            None => Vec::new(),
            Some(i) if i < 5 => H_PRESETS[i].1.to_vec(),
            Some(_) => CATS
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1u16 << i) != 0)
                .map(|(_, c)| c.0)
                .collect(),
        }
    }

    /// The `StripPolicy` for a policy index (the library side of the case).
    fn policy_of(policy: Option<usize>, mask: u16) -> Option<StripPolicy> {
        use exif_core::audit::Category;
        policy.map(|i| match i {
            0 => StripPolicy::Gps,
            1 => StripPolicy::GpsAndDevice,
            2 => StripPolicy::AllPii,
            3 => StripPolicy::All,
            4 => StripPolicy::Everything,
            _ => StripPolicy::Custom(
                Category::ALL
                    .into_iter()
                    .enumerate()
                    .filter(|(i, _)| mask & (1u16 << i) != 0)
                    .fold(StripSpec::none(), |s, (_, c)| s.include(c)),
            ),
        })
    }

    fn reason_of(i: usize) -> FailReason {
        match i {
            0 => FailReason::Unreadable,
            1 => FailReason::TooLarge,
            2 => FailReason::NotRegularFile,
            _ => FailReason::Other,
        }
    }

    /// The payload prefix (at most 65 bytes: one more than the excerpt cap)
    /// of every tag entry in audit order, `None` for a region or an
    /// unreadable payload, read with the container's own `payload`.
    pub fn payloads(data: &[u8]) -> Vec<Option<Vec<u8>>> {
        fn walk(c: &Container<'_>, out: &mut Vec<Option<Vec<u8>>>) {
            for ifd in c.ifds() {
                for e in ifd.entries() {
                    out.push(c.payload(e).ok().map(|p| p[..p.len().min(65)].to_vec()));
                }
            }
        }
        let mut out = Vec::new();
        match super::audit_invariants::kind_label(data).as_str() {
            "Tiff" => {
                if let Ok(c) = Container::parse(data) {
                    walk(&c, &mut out);
                }
            }
            "Jpeg" => {
                if let Ok(meta) = jpeg::read(data) {
                    for block in meta.exif_blocks() {
                        if let Ok(c) = block.container() {
                            walk(c, &mut out);
                        }
                    }
                    for _ in meta.regions() {
                        out.push(None);
                    }
                }
            }
            _ => {}
        }
        out.truncate(super::audit_invariants::CAP);
        out
    }

    /// DEC-010 part 1: the excerpt bytes and whether bytes were cut.
    fn excerpt_of(field_type: u16, prefix: &[u8]) -> (Vec<u8>, bool) {
        let mut b = prefix.to_vec();
        if field_type == 2 {
            if let Some(n) = b.iter().position(|x| *x == 0) {
                b.truncate(n);
            }
        }
        if b.len() > 64 {
            b.truncate(64);
            (b, true)
        } else {
            (b, false)
        }
    }

    // ── the case ────────────────────────────────────────────────────────────

    /// A flattened `Summary`.
    #[derive(Clone, Debug, PartialEq)]
    pub struct SummaryF {
        pub files: usize,
        pub failed: usize,
        pub unsupported: usize,
        pub unreadable: usize,
        pub issues: usize,
        pub ok: usize,
        pub findings: usize,
        pub by_category: Vec<(String, usize)>,
        pub max_severity: Option<String>,
        pub residual: Option<usize>,
        pub reliable: usize,
        pub unreliable: usize,
    }

    /// One rendering case: the input, the options, the re-derivation and the
    /// library's three outputs (the part a plant changes).
    #[derive(Clone, Debug)]
    pub struct Ctx {
        pub file: Vec<u8>,
        pub label: Vec<u8>,
        pub detail: Vec<u8>,
        pub policy: Option<usize>,
        pub mask: u16,
        pub inventory: bool,
        pub values: bool,
        pub reason: usize,
        pub want: Facts,
        pub payloads: Vec<Option<Vec<u8>>>,
        pub removed: Vec<&'static str>,
        pub text: String,
        pub text_again: String,
        pub json: String,
        pub csv: String,
        pub summary: SummaryF,
    }

    /// The options of one case.
    #[derive(Clone, Copy, Debug)]
    pub struct CaseOpts {
        /// `None`, or 0..=4 for the five presets and 5 for a custom set.
        pub policy: Option<usize>,
        /// The byte a custom category set comes from.
        pub custom_byte: u8,
        pub inventory: bool,
        pub values: bool,
        /// 0..=3, an index into `REASONS`.
        pub reason: usize,
    }

    /// Build a case: render `file` under the options with the library, and
    /// re-derive what the report must say.
    pub fn build_case(file: &[u8], label: &[u8], detail: &[u8], o: CaseOpts) -> Ctx {
        let CaseOpts {
            policy,
            custom_byte,
            inventory,
            values,
            reason,
        } = o;
        let mask = custom_mask(custom_byte);
        let opts = RenderOptions {
            policy: policy_of(policy, mask),
            inventory,
            values,
        };
        let entries = vec![
            FileEntry {
                label,
                result: FileResult::Audited(audit(file)),
            },
            FileEntry {
                label,
                result: FileResult::Failed {
                    reason: reason_of(reason),
                    detail,
                },
            },
        ];
        let s = summarize(&entries, &opts);
        let summary = SummaryF {
            files: s.files,
            failed: s.failed,
            unsupported: s.unsupported,
            unreadable: s.unreadable,
            issues: s.issues,
            ok: s.ok,
            findings: s.findings,
            by_category: s
                .by_category
                .iter()
                .map(|(c, n)| (format!("{c:?}"), *n))
                .collect(),
            max_severity: s.max_severity.map(|m| format!("{m:?}")),
            residual: s.residual_findings,
            reliable: s.reliable_files,
            unreliable: s.unreliable_files,
        };
        Ctx {
            file: file.to_vec(),
            label: label.to_vec(),
            detail: detail.to_vec(),
            policy,
            mask,
            inventory,
            values,
            reason,
            want: rederive(file),
            payloads: payloads(file),
            removed: removed_names(policy, mask),
            text: render_text(&entries, &opts),
            text_again: render_text(&entries, &opts),
            json: render_json(&entries, &opts),
            csv: render_csv(&entries, &opts),
            summary,
        }
    }

    /// The fuzz input layout: byte 0 options (bits 0..3 policy: none, Gps,
    /// GpsAndDevice, AllPii, All, Everything, Custom, none; bit 3 inventory;
    /// bit 4 values; bits 5..7 the failure reason), byte 1 the label length
    /// `L` (and, for a Custom policy, the byte the category set comes from),
    /// the next `L` bytes the label, the rest the file. The failed entry's
    /// detail is the first 64 bytes of the file.
    pub fn build(data: &[u8]) -> Ctx {
        let opt = data.first().copied().unwrap_or(0);
        let l_byte = data.get(1).copied().unwrap_or(0);
        let rest = data.get(2..).unwrap_or(&[]);
        let l = usize::from(l_byte).min(rest.len());
        let label = &rest[..l];
        let file = &rest[l..];
        let policy = match opt & 7 {
            1 => Some(0),
            2 => Some(1),
            3 => Some(2),
            4 => Some(3),
            5 => Some(4),
            6 => Some(5),
            _ => None,
        };
        build_case(
            file,
            label,
            &file[..file.len().min(64)],
            CaseOpts {
                policy,
                custom_byte: l_byte,
                inventory: opt & 8 != 0,
                values: opt & 16 != 0,
                reason: usize::from((opt >> 5) & 3),
            },
        )
    }

    // ── small helpers over the parsed trees ─────────────────────────────────

    fn req<'a>(j: &'a J, k: &str) -> Result<&'a J, String> {
        j.get(k).ok_or_else(|| format!("missing key {k:?}"))
    }

    fn keys_exact(j: &J, want: &[&str], what: &str) -> Result<(), String> {
        if !matches!(j, J::Obj(_)) {
            return Err(format!("{what}: not an object"));
        }
        let got = j.keys();
        if got != want {
            return Err(format!("{what}: keys {got:?}, want {want:?}"));
        }
        Ok(())
    }

    fn uint(j: &J, what: &str) -> Result<u64, String> {
        j.as_uint()
            .ok_or_else(|| format!("{what}: not a non-negative integer: {j:?}"))
    }

    fn boolean(j: &J, what: &str) -> Result<bool, String> {
        j.as_bool().ok_or_else(|| format!("{what}: not a bool"))
    }

    fn string<'a>(j: &'a J, what: &str) -> Result<&'a str, String> {
        j.as_str().ok_or_else(|| format!("{what}: not a string"))
    }

    fn null_or_in(j: &J, set: &[&str], what: &str) -> Result<(), String> {
        if j.is_null() {
            return Ok(());
        }
        let s = string(j, what)?;
        if set.contains(&s) {
            Ok(())
        } else {
            Err(format!("{what}: {s:?} is not one of {set:?}"))
        }
    }

    fn in_set(j: &J, set: &[&str], what: &str) -> Result<(), String> {
        if j.is_null() {
            return Err(format!("{what}: null"));
        }
        null_or_in(j, set, what)
    }

    fn null_or_uint(j: &J, what: &str) -> Result<(), String> {
        if j.is_null() {
            Ok(())
        } else {
            uint(j, what).map(|_| ())
        }
    }

    fn parsed_json(c: &Ctx) -> Result<J, String> {
        parse_json(c.json.as_bytes())
    }

    // ── guards ──────────────────────────────────────────────────────────────

    pub type Guard = fn(&Ctx) -> Result<(), String>;

    fn g_json_wellformed(c: &Ctx) -> Result<(), String> {
        tick(1);
        parsed_json(c).map(|_| ())
    }

    fn where_shape(at: &J) -> Result<(), String> {
        match string(req(at, "kind")?, "at.kind")? {
            "tag" => {
                keys_exact(at, &TAG_AT_KEYS, "tag location")?;
                in_set(req(at, "ifd")?, &IFDS, "at.ifd")?;
                uint(req(at, "ifd_index")?, "at.ifd_index")?;
                let tag = string(req(at, "tag")?, "at.tag")?;
                let hex_ok = tag.len() == 6
                    && tag.starts_with("0x")
                    && tag[2..]
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
                if !hex_ok {
                    return Err(format!(
                        "at.tag {tag:?} is not 0x and four lowercase hex digits"
                    ));
                }
                uint(req(at, "field_type")?, "at.field_type")?;
                uint(req(at, "count")?, "at.count")?;
            }
            "region" => {
                keys_exact(at, &REGION_AT_KEYS, "region location")?;
                in_set(req(at, "region")?, &REGIONS, "at.region")?;
                uint(req(at, "segment")?, "at.segment")?;
                uint(req(at, "len")?, "at.len")?;
            }
            other => return Err(format!("at.kind {other:?}")),
        }
        Ok(())
    }

    fn value_shape(v: &J) -> Result<(), String> {
        if v.is_null() {
            return Ok(());
        }
        keys_exact(v, &["text", "truncated"], "value")?;
        string(req(v, "text")?, "value.text")?;
        boolean(req(v, "truncated")?, "value.truncated")?;
        Ok(())
    }

    fn origin_shape(j: &J) -> Result<(), String> {
        null_or_uint(req(j, "block")?, "block")?;
        null_or_uint(req(j, "segment")?, "segment")?;
        boolean(req(j, "after_scan")?, "after_scan")?;
        Ok(())
    }

    fn file_shape(f: &J) -> Result<(), String> {
        keys_exact(f, &FILE_KEYS, "file")?;
        string(req(f, "label")?, "label")?;
        in_set(req(f, "status")?, &STATUSES, "status")?;
        null_or_in(req(f, "container")?, &CONTAINERS, "container")?;
        let failure = req(f, "failure")?;
        if !failure.is_null() {
            keys_exact(failure, &["reason", "detail"], "failure")?;
            in_set(req(failure, "reason")?, &REASONS, "failure.reason")?;
            string(req(failure, "detail")?, "failure.detail")?;
        }
        let health = req(f, "health")?;
        if !health.is_null() {
            keys_exact(health, &HEALTH_KEYS, "health")?;
            boolean(req(health, "clean")?, "health.clean")?;
            uint(req(health, "jpeg_issues")?, "health.jpeg_issues")?;
            uint(req(health, "exif_issues")?, "health.exif_issues")?;
            uint(req(health, "failed_blocks")?, "health.failed_blocks")?;
            boolean(
                req(health, "inventory_truncated")?,
                "health.inventory_truncated",
            )?;
        }
        null_or_in(req(f, "max_severity")?, &SEVS, "file.max_severity")?;
        uint(req(f, "inventory_items")?, "inventory_items")?;
        let findings = req(f, "findings")?
            .as_arr()
            .ok_or("findings: not an array")?;
        for fi in findings {
            tick(1);
            keys_exact(fi, &FINDING_KEYS, "finding")?;
            in_set(req(fi, "category")?, &CATS.map(|c| c.1), "finding.category")?;
            in_set(req(fi, "severity")?, &SEVS, "finding.severity")?;
            where_shape(req(fi, "at")?)?;
            origin_shape(fi)?;
            null_or_in(req(fi, "disposition")?, &DISPS, "finding.disposition")?;
            let t = req(fi, "thumbnail")?;
            if !t.is_null() {
                keys_exact(t, &["offset", "length", "starts_with_ffd8"], "thumbnail")?;
                null_or_uint(req(t, "offset")?, "thumbnail.offset")?;
                null_or_uint(req(t, "length")?, "thumbnail.length")?;
                boolean(req(t, "starts_with_ffd8")?, "thumbnail.starts_with_ffd8")?;
            }
            value_shape(req(fi, "value")?)?;
        }
        let inv = req(f, "inventory")?;
        if !inv.is_null() {
            for item in inv.as_arr().ok_or("inventory: not an array")? {
                tick(1);
                keys_exact(item, &ITEM_KEYS, "inventory item")?;
                where_shape(req(item, "at")?)?;
                origin_shape(item)?;
                null_or_in(req(item, "category")?, &CATS.map(|c| c.1), "item.category")?;
                value_shape(req(item, "value")?)?;
            }
        }
        let p = req(f, "preview")?;
        if !p.is_null() {
            keys_exact(p, &PREVIEW_KEYS, "preview")?;
            in_set(req(p, "policy")?, &POLICY_NAMES, "preview.policy")?;
            let counts = req(p, "counts")?;
            keys_exact(counts, &["removed", "kept", "kept_unread"], "counts")?;
            for k in ["removed", "kept", "kept_unread"] {
                uint(req(counts, k)?, k)?;
            }
            boolean(req(p, "reliable")?, "preview.reliable")?;
            null_or_uint(req(p, "residual_findings")?, "preview.residual_findings")?;
            null_or_in(
                req(p, "residual_severity")?,
                &SEVS,
                "preview.residual_severity",
            )?;
        }
        Ok(())
    }

    fn g_json_shape(c: &Ctx) -> Result<(), String> {
        let j = parsed_json(c)?;
        keys_exact(
            &j,
            &["schema_version", "policy", "options", "summary", "files"],
            "top level",
        )?;
        if uint(req(&j, "schema_version")?, "schema_version")? != 1 {
            return Err("schema_version is not 1".to_string());
        }
        null_or_in(req(&j, "policy")?, &POLICY_NAMES, "policy")?;
        let o = req(&j, "options")?;
        keys_exact(o, &["inventory", "values"], "options")?;
        boolean(req(o, "inventory")?, "options.inventory")?;
        boolean(req(o, "values")?, "options.values")?;
        let s = req(&j, "summary")?;
        keys_exact(s, &SUMMARY_KEYS, "summary")?;
        for k in SUMMARY_KEYS {
            match k {
                "by_category" => {
                    let b = req(s, k)?;
                    keys_exact(b, &CATS.map(|c| c.1), "by_category")?;
                    for (_, name) in CATS {
                        uint(req(b, name)?, name)?;
                    }
                }
                "max_severity" => null_or_in(req(s, k)?, &SEVS, "summary.max_severity")?,
                "residual_findings" => null_or_uint(req(s, k)?, "summary.residual_findings")?,
                _ => {
                    uint(req(s, k)?, k)?;
                }
            }
        }
        for f in req(&j, "files")?.as_arr().ok_or("files: not an array")? {
            file_shape(f)?;
        }
        Ok(())
    }

    fn at_matches(at: &J, w: &What) -> Result<(), String> {
        match w {
            What::Tag {
                ifd,
                ifd_index,
                tag,
                field_type,
                count,
            } => {
                let want_ifd = lower(ifd);
                if string(req(at, "kind")?, "kind")? != "tag"
                    || string(req(at, "ifd")?, "ifd")? != want_ifd
                    || uint(req(at, "ifd_index")?, "ifd_index")? != *ifd_index as u64
                    || string(req(at, "tag")?, "tag")? != format!("0x{tag:04x}")
                    || uint(req(at, "field_type")?, "field_type")? != u64::from(*field_type)
                    || uint(req(at, "count")?, "count")? != u64::from(*count)
                {
                    return Err(format!("location {at:?} is not {w:?}"));
                }
            }
            What::Region { kind, segment, len } => {
                let want_kind = match kind.as_str() {
                    "ExtendedXmp" => "extended_xmp".to_string(),
                    k => lower(k),
                };
                if string(req(at, "kind")?, "kind")? != "region"
                    || string(req(at, "region")?, "region")? != want_kind
                    || uint(req(at, "segment")?, "segment")? != *segment as u64
                    || uint(req(at, "len")?, "len")? != *len as u64
                {
                    return Err(format!("location {at:?} is not {w:?}"));
                }
            }
        }
        Ok(())
    }

    fn origin_matches(
        j: &J,
        block: Option<usize>,
        segment: Option<usize>,
        after: bool,
    ) -> Result<(), String> {
        let n = |k: &str| -> Result<Option<u64>, String> {
            let v = req(j, k)?;
            if v.is_null() {
                Ok(None)
            } else {
                uint(v, k).map(Some)
            }
        };
        if n("block")? != block.map(|b| b as u64)
            || n("segment")? != segment.map(|s| s as u64)
            || boolean(req(j, "after_scan")?, "after_scan")? != after
        {
            return Err(format!("origin {j:?} is not {block:?} {segment:?} {after}"));
        }
        Ok(())
    }

    /// The expected excerpt of the item at `index`, if one is shown.
    fn expected_value(c: &Ctx, index: Option<usize>) -> Option<(String, bool)> {
        if !c.values {
            return None;
        }
        let i = index?;
        let field_type = match &c.want.items.get(i)?.what {
            What::Tag { field_type, .. } => *field_type,
            What::Region { .. } => return None,
        };
        let prefix = c.payloads.get(i)?.as_ref()?;
        let (bytes, truncated) = excerpt_of(field_type, prefix);
        Some((naive_safe_text(&bytes), truncated))
    }

    fn value_matches(v: &J, want: &Option<(String, bool)>, what: &str) -> Result<(), String> {
        match want {
            None => {
                if !v.is_null() {
                    return Err(format!("{what}: value {v:?}, want null"));
                }
            }
            Some((text, truncated)) => {
                if v.is_null() {
                    return Err(format!("{what}: value is null, want {text:?}"));
                }
                if string(req(v, "text")?, "value.text")? != text
                    || boolean(req(v, "truncated")?, "value.truncated")? != *truncated
                {
                    return Err(format!(
                        "{what}: value {v:?}, want {text:?} truncated {truncated}"
                    ));
                }
            }
        }
        Ok(())
    }

    fn opt_name(j: &J, what: &str) -> Result<Option<String>, String> {
        if j.is_null() {
            Ok(None)
        } else {
            Ok(Some(string(j, what)?.to_string()))
        }
    }

    fn g_json_facts(c: &Ctx) -> Result<(), String> {
        let j = parsed_json(c)?;
        let files = req(&j, "files")?.as_arr().ok_or("files: not an array")?;
        if files.len() != 2 {
            return Err(format!("{} files, want 2", files.len()));
        }
        let (a, failed) = (&files[0], &files[1]);
        let want_label = naive_safe_text(&c.label);
        for f in files {
            if string(req(f, "label")?, "label")? != want_label {
                return Err(format!("label {:?}, want {want_label:?}", req(f, "label")?));
            }
        }
        // The failed entry.
        if string(req(failed, "status")?, "status")? != "failed"
            || !req(failed, "container")?.is_null()
            || !req(failed, "health")?.is_null()
            || !req(failed, "max_severity")?.is_null()
            || uint(req(failed, "inventory_items")?, "inventory_items")? != 0
            || !req(failed, "findings")?
                .as_arr()
                .ok_or("findings")?
                .is_empty()
            || !req(failed, "inventory")?.is_null()
            || !req(failed, "preview")?.is_null()
        {
            return Err("the failed entry is not the failure shape".to_string());
        }
        let failure = req(failed, "failure")?;
        if string(req(failure, "reason")?, "reason")? != REASONS[c.reason]
            || string(req(failure, "detail")?, "detail")? != naive_safe_text(&c.detail)
        {
            return Err(format!(
                "failure {failure:?} is not reason {} detail {:?}",
                REASONS[c.reason], c.detail
            ));
        }
        // The audited entry.
        let w = &c.want;
        if string(req(a, "status")?, "status")? != status_of_facts(w) {
            return Err(format!(
                "status {:?}, want {}",
                req(a, "status")?,
                status_of_facts(w)
            ));
        }
        if !req(a, "failure")?.is_null() {
            return Err("an audited file has a failure".to_string());
        }
        if string(req(a, "container")?, "container")? != container_of(w) {
            return Err(format!(
                "container {:?}, want {}",
                req(a, "container")?,
                container_of(w)
            ));
        }
        let h = req(a, "health")?;
        if boolean(req(h, "clean")?, "clean")? != w.health.clean
            || uint(req(h, "jpeg_issues")?, "jpeg_issues")? != w.health.jpeg_issues as u64
            || uint(req(h, "exif_issues")?, "exif_issues")? != w.health.exif_issues as u64
            || uint(req(h, "failed_blocks")?, "failed_blocks")? != w.health.failed_blocks as u64
            || boolean(req(h, "inventory_truncated")?, "inventory_truncated")? != w.health.truncated
        {
            return Err(format!("health {h:?} is not {:?}", w.health));
        }
        if opt_name(req(a, "max_severity")?, "max_severity")? != w.max.as_ref().map(|m| lower(m)) {
            return Err(format!(
                "max_severity {:?}, want {:?}",
                req(a, "max_severity")?,
                w.max
            ));
        }
        if uint(req(a, "inventory_items")?, "inventory_items")? != w.items.len() as u64 {
            return Err(format!(
                "inventory_items {:?}, want {}",
                req(a, "inventory_items")?,
                w.items.len()
            ));
        }
        // Dispositions from the DEC-009 grid, typed in `audit_invariants`.
        let want_prev = c.policy.map(|_| h_want(w, &c.removed));
        let findings = req(a, "findings")?.as_arr().ok_or("findings")?;
        if findings.len() != w.findings.len() {
            return Err(format!(
                "{} findings, want {}",
                findings.len(),
                w.findings.len()
            ));
        }
        for (i, (fj, fw)) in findings.iter().zip(&w.findings).enumerate() {
            tick(1);
            if string(req(fj, "category")?, "category")? != snake(&fw.category) {
                return Err(format!(
                    "finding {i}: category {:?}, want {}",
                    req(fj, "category")?,
                    snake(&fw.category)
                ));
            }
            if string(req(fj, "severity")?, "severity")? != lower(severity_of(&fw.category)) {
                return Err(format!("finding {i}: severity {:?}", req(fj, "severity")?));
            }
            at_matches(req(fj, "at")?, &fw.at.what).map_err(|e| format!("finding {i}: {e}"))?;
            origin_matches(fj, fw.at.block, fw.at.segment, fw.at.after_scan)
                .map_err(|e| format!("finding {i}: {e}"))?;
            let disp = opt_name(req(fj, "disposition")?, "disposition")?;
            let want_disp = want_prev
                .as_ref()
                .and_then(|p| p.dispositions.get(i))
                .cloned();
            if disp != want_disp {
                return Err(format!(
                    "finding {i}: disposition {disp:?}, want {want_disp:?}"
                ));
            }
            let t = req(fj, "thumbnail")?;
            match (&fw.thumb, t.is_null()) {
                (None, true) => {}
                (Some(tw), false) => {
                    let off = opt_u(req(t, "offset")?)?;
                    let len = opt_u(req(t, "length")?)?;
                    let want_off = tw.range.map(|r| u64::from(r.0));
                    let want_len = tw.range.map(|r| u64::from(r.1));
                    if off != want_off
                        || len != want_len
                        || boolean(req(t, "starts_with_ffd8")?, "ffd8")? != tw.ffd8
                    {
                        return Err(format!("finding {i}: thumbnail {t:?}, want {tw:?}"));
                    }
                }
                _ => return Err(format!("finding {i}: thumbnail {t:?}, want {:?}", fw.thumb)),
            }
            // A finding without an item (past the inventory cap) has no
            // re-derived payload here; its value is only checked for shape.
            if fw.item.is_some() {
                value_matches(
                    req(fj, "value")?,
                    &expected_value(c, fw.item),
                    &format!("finding {i}"),
                )?;
            } else if !c.values && !req(fj, "value")?.is_null() {
                return Err(format!("finding {i}: a value with values off"));
            }
        }
        // The inventory.
        let inv = req(a, "inventory")?;
        if !c.inventory {
            if !inv.is_null() {
                return Err("an inventory with inventory off".to_string());
            }
        } else {
            let items = inv.as_arr().ok_or("inventory is null with inventory on")?;
            if items.len() != w.items.len() {
                return Err(format!(
                    "{} inventory items, want {}",
                    items.len(),
                    w.items.len()
                ));
            }
            let mut category_of: Vec<Option<&str>> = vec![None; w.items.len()];
            for fw in &w.findings {
                tick(1);
                if let Some(slot) = fw.item.and_then(|i| category_of.get_mut(i)) {
                    *slot = Some(snake(&fw.category));
                }
            }
            for (i, (ij, iw)) in items.iter().zip(&w.items).enumerate() {
                tick(1);
                at_matches(req(ij, "at")?, &iw.what).map_err(|e| format!("item {i}: {e}"))?;
                origin_matches(ij, iw.block, iw.segment, iw.after_scan)
                    .map_err(|e| format!("item {i}: {e}"))?;
                let cat = opt_name(req(ij, "category")?, "category")?;
                if cat.as_deref() != category_of[i] {
                    return Err(format!(
                        "item {i}: category {cat:?}, want {:?}",
                        category_of[i]
                    ));
                }
                value_matches(
                    req(ij, "value")?,
                    &expected_value(c, Some(i)),
                    &format!("item {i}"),
                )?;
            }
        }
        // The preview.
        let p = req(a, "preview")?;
        match (&want_prev, p.is_null()) {
            (None, true) => {}
            (Some(wp), false) => {
                let reliable = w.health.clean && !w.health.truncated;
                let counts = req(p, "counts")?;
                if string(req(p, "policy")?, "policy")? != POLICY_NAMES[c.policy.unwrap_or(0)]
                    || boolean(req(p, "reliable")?, "reliable")? != reliable
                    || uint(req(counts, "removed")?, "removed")? != wp.removed as u64
                    || uint(req(counts, "kept")?, "kept")? != wp.kept as u64
                    || uint(req(counts, "kept_unread")?, "kept_unread")? != wp.kept_unread as u64
                {
                    return Err(format!("preview {p:?} is not {wp:?}"));
                }
                let rf = req(p, "residual_findings")?;
                let rs = req(p, "residual_severity")?;
                if reliable {
                    if uint(rf, "residual_findings")? != wp.residual as u64
                        || opt_name(rs, "residual_severity")?
                            != wp.residual_sev.as_ref().map(|s| lower(s))
                    {
                        return Err(format!(
                            "preview residuals {rf:?} {rs:?}, want {} {:?}",
                            wp.residual, wp.residual_sev
                        ));
                    }
                } else if !rf.is_null() || !rs.is_null() {
                    return Err("residuals printed for an unreliable preview".to_string());
                }
            }
            _ => return Err(format!("preview {p:?} with policy {:?}", c.policy)),
        }
        Ok(())
    }

    fn opt_u(j: &J) -> Result<Option<u64>, String> {
        if j.is_null() {
            Ok(None)
        } else {
            uint(j, "number").map(Some)
        }
    }

    fn guarded(s: &str) -> String {
        if matches!(s.chars().next(), Some('=' | '+' | '-' | '@' | '\t' | '\r')) {
            format!("'{s}")
        } else {
            s.to_string()
        }
    }

    fn g_csv_wellformed(c: &Ctx) -> Result<(), String> {
        tick(1);
        parse_csv(c.csv.as_bytes()).map(|_| ())
    }

    /// One expected CSV row.
    fn exp_row(
        c: &Ctx,
        status: &str,
        container: &str,
        category: Option<&str>,
        disp: Option<&str>,
        at: Option<&super::audit_invariants::ItemF>,
        value: Option<(String, bool)>,
    ) -> Vec<String> {
        let mut r = vec![String::new(); 19];
        r[0] = guarded(&naive_safe_text(&c.label));
        r[1] = status.to_string();
        r[2] = container.to_string();
        if let Some(cat) = category {
            r[3] = snake(cat).to_string();
            r[4] = lower(severity_of(cat));
        }
        if let Some(at) = at {
            match &at.what {
                What::Tag {
                    ifd,
                    ifd_index,
                    tag,
                    field_type,
                    count,
                } => {
                    r[5] = "tag".to_string();
                    r[6] = lower(ifd);
                    r[7] = ifd_index.to_string();
                    r[8] = format!("0x{tag:04x}");
                    r[9] = field_type.to_string();
                    r[10] = count.to_string();
                }
                What::Region { kind, len, .. } => {
                    r[5] = "region".to_string();
                    r[11] = match kind.as_str() {
                        "ExtendedXmp" => "extended_xmp".to_string(),
                        k => lower(k),
                    };
                    r[12] = len.to_string();
                }
            }
            r[13] = at.block.map(|b| b.to_string()).unwrap_or_default();
            r[14] = at.segment.map(|s| s.to_string()).unwrap_or_default();
            r[15] = at.after_scan.to_string();
        }
        r[16] = disp.unwrap_or("").to_string();
        if let Some((text, truncated)) = value {
            r[17] = guarded(&text);
            r[18] = truncated.to_string();
        }
        r
    }

    /// The rows the audited file must produce (rule 6 of SPEC-015).
    fn expected_csv_rows(c: &Ctx) -> Vec<Vec<String>> {
        let w = &c.want;
        let status = status_of_facts(w);
        let container = container_of(w);
        let prev = c.policy.map(|_| h_want(w, &c.removed));
        let disp_of = |fi: usize| -> Option<String> {
            prev.as_ref().and_then(|p| p.dispositions.get(fi)).cloned()
        };
        let mut rows = Vec::new();
        if c.inventory {
            let mut finding_of: Vec<Option<usize>> = vec![None; w.items.len()];
            for (fi, f) in w.findings.iter().enumerate() {
                tick(1);
                if let Some(slot) = f.item.and_then(|i| finding_of.get_mut(i)) {
                    *slot = Some(fi);
                }
            }
            for (i, item) in w.items.iter().enumerate() {
                tick(1);
                let (cat, disp) = match finding_of[i] {
                    Some(fi) => (Some(w.findings[fi].category.clone()), disp_of(fi)),
                    None => (None, None),
                };
                rows.push(exp_row(
                    c,
                    status,
                    &container,
                    cat.as_deref(),
                    disp.as_deref(),
                    Some(item),
                    expected_value(c, Some(i)),
                ));
            }
            for (fi, f) in w.findings.iter().enumerate() {
                tick(1);
                if f.item.is_none() {
                    rows.push(exp_row(
                        c,
                        status,
                        &container,
                        Some(&f.category),
                        disp_of(fi).as_deref(),
                        Some(&f.at),
                        None,
                    ));
                }
            }
        } else {
            for (fi, f) in w.findings.iter().enumerate() {
                tick(1);
                rows.push(exp_row(
                    c,
                    status,
                    &container,
                    Some(&f.category),
                    disp_of(fi).as_deref(),
                    Some(&f.at),
                    expected_value(c, f.item),
                ));
            }
        }
        if rows.is_empty() {
            rows.push(exp_row(c, status, &container, None, None, None, None));
        }
        rows
    }

    fn g_csv_rows(c: &Ctx) -> Result<(), String> {
        let rows = parse_csv(c.csv.as_bytes())?;
        let header: Vec<String> = CSV_HEADER.iter().map(|s| s.to_string()).collect();
        if rows[0] != header {
            return Err(format!("header {:?}", rows[0]));
        }
        let mut want = expected_csv_rows(c);
        // The failed file: label, status, and the detail in `value`.
        let mut failed = vec![String::new(); 19];
        failed[0] = guarded(&naive_safe_text(&c.label));
        failed[1] = "failed".to_string();
        failed[17] = guarded(&naive_safe_text(&c.detail));
        want.push(failed);
        if rows.len() != want.len() + 1 {
            return Err(format!("{} rows, want {}", rows.len() - 1, want.len()));
        }
        for (i, (got, exp)) in rows.iter().skip(1).zip(&want).enumerate() {
            tick(1);
            if got.len() != 19 {
                return Err(format!("row {i} has {} fields", got.len()));
            }
            // The first character of every label and value field is never a
            // formula character (an excerpt may be ASCII text from the file).
            for col in [0usize, 17] {
                if matches!(
                    got[col].chars().next(),
                    Some('=' | '+' | '-' | '@' | '\t' | '\r')
                ) {
                    return Err(format!(
                        "row {i}: column {col} starts a formula: {:?}",
                        got[col]
                    ));
                }
            }
            if got != exp {
                let col = got.iter().zip(exp).position(|(g, e)| g != e).unwrap_or(0);
                return Err(format!(
                    "row {i}: column {} ({}) is {:?}, want {:?}",
                    col, CSV_HEADER[col], got[col], exp[col]
                ));
            }
        }
        Ok(())
    }

    /// The text with the hostile pieces removed: the `file:` lines (labels),
    /// the `failure:` lines (details) and the `value="..."` excerpts.
    fn text_without_hostile_parts(text: &str) -> String {
        let mut out = String::new();
        for line in text.lines() {
            tick(1);
            if line.starts_with("file: ") || line.starts_with("  failure: ") {
                continue;
            }
            let mut rest = line;
            while let Some(at) = rest.find("value=\"") {
                out.push_str(&rest[..at]);
                let after = &rest[at + 7..];
                // The excerpt holds no raw quote, so the first one closes it.
                match after.find('"') {
                    Some(end) => rest = &after[end + 1..],
                    None => {
                        rest = "";
                    }
                }
            }
            out.push_str(rest);
            out.push('\n');
        }
        out
    }

    fn g_text_safe(c: &Ctx) -> Result<(), String> {
        for (i, ch) in c.text.chars().enumerate() {
            let n = ch as u32;
            let bad = (n < 0x20 && ch != '\n')
                || n == 0x7F
                || (0x80..=0x9F).contains(&n)
                || n == 0x061C
                || n == 0x200E
                || n == 0x200F
                || (0x202A..=0x202E).contains(&n)
                || (0x2066..=0x2069).contains(&n)
                || n == 0x2028
                || n == 0x2029;
            if bad {
                return Err(format!("forbidden character {ch:?} at char {i}"));
            }
        }
        tick(c.text.len() / 64 + 1);
        if c.text != c.text_again {
            return Err("the text report is not deterministic".to_string());
        }
        if !c.text.ends_with('\n') {
            return Err("the text report does not end with a newline".to_string());
        }
        let rest = lower(&text_without_hostile_parts(&c.text));
        for phrase in ["no personal data", "safe", "clean file"] {
            if rest.contains(phrase) {
                return Err(format!("the text report says {phrase:?}"));
            }
        }
        // Every file label appears, escaped.
        // Whole lines: a failure detail may itself end in "file: ", so a
        // substring count would over-count (found by the fuzzer, empty label).
        let want = format!("file: {}", naive_safe_text(&c.label));
        if c.text.lines().filter(|l| *l == want).count() != 2 {
            return Err(format!("the label line {want:?} is not there twice"));
        }
        Ok(())
    }

    fn g_values_gate(c: &Ctx) -> Result<(), String> {
        let j = parsed_json(c)?;
        let a = &req(&j, "files")?.as_arr().ok_or("files")?[0];
        let rows = parse_csv(c.csv.as_bytes())?;
        // The audited file's rows: everything between the header and the
        // failed file's row.
        let audited_rows = rows.get(1..rows.len().saturating_sub(1)).unwrap_or(&[]);
        let json_values = |arr: &J| -> Result<Vec<J>, String> {
            let mut v = Vec::new();
            for x in arr.as_arr().ok_or("not an array")? {
                tick(1);
                v.push(req(x, "value")?.clone());
            }
            Ok(v)
        };
        let finding_values = json_values(req(a, "findings")?)?;
        let item_values = if c.inventory {
            json_values(req(a, "inventory")?)?
        } else {
            Vec::new()
        };
        let findings_shown = finding_values.iter().filter(|v| !v.is_null()).count();
        let items_shown = item_values.iter().filter(|v| !v.is_null()).count();
        let csv_shown = audited_rows.iter().filter(|r| !r[18].is_empty()).count();
        let text_shown = c
            .text
            .lines()
            .filter(|l| l.starts_with("    - ") && l.contains(" value=\""))
            .count();
        if !c.values {
            if findings_shown + items_shown != 0 {
                return Err("a JSON value with values off".to_string());
            }
            if audited_rows
                .iter()
                .any(|r| !r[17].is_empty() || !r[18].is_empty())
            {
                return Err("a CSV value with values off".to_string());
            }
            if c.text.contains(" value=\"") {
                return Err("a text excerpt with values off".to_string());
            }
            return Ok(());
        }
        // Values on: each excerpt at most 64 source bytes (so at most 256
        // escaped characters) and `truncated` iff the source was longer,
        // re-derived from the payload; the formats show the same excerpts.
        for v in finding_values
            .iter()
            .chain(&item_values)
            .filter(|v| !v.is_null())
        {
            tick(1);
            let text = string(req(v, "text")?, "text")?;
            if text.chars().count() > 256 {
                return Err(format!("an excerpt of {} characters", text.chars().count()));
            }
        }
        for (i, v) in finding_values.iter().enumerate() {
            tick(1);
            if let Some(fw) = c.want.findings.get(i) {
                if fw.item.is_some() {
                    value_matches(v, &expected_value(c, fw.item), &format!("finding {i}"))?;
                }
            }
        }
        let want_csv = if c.inventory {
            items_shown
        } else {
            findings_shown
        };
        if csv_shown != want_csv {
            return Err(format!("{csv_shown} CSV excerpts, want {want_csv}"));
        }
        if text_shown != findings_shown + items_shown {
            return Err(format!(
                "{text_shown} text excerpts, want {}",
                findings_shown + items_shown
            ));
        }
        let cut_json = finding_values
            .iter()
            .chain(&item_values)
            .filter(|v| matches!(v.get("truncated"), Some(J::Bool(true))))
            .count();
        let cut_text = c
            .text
            .lines()
            .filter(|l| {
                l.starts_with("    - ") && l.contains(" value=\"") && l.contains("\" (truncated)")
            })
            .count();
        if cut_json != cut_text {
            return Err(format!(
                "{cut_text} text excerpts cut, JSON says {cut_json}"
            ));
        }
        let cut_csv = audited_rows.iter().filter(|r| r[18] == "true").count();
        let want_cut_csv = if c.inventory {
            item_values
                .iter()
                .filter(|v| matches!(v.get("truncated"), Some(J::Bool(true))))
                .count()
        } else {
            finding_values
                .iter()
                .filter(|v| matches!(v.get("truncated"), Some(J::Bool(true))))
                .count()
        };
        if cut_csv != want_cut_csv {
            return Err(format!("{cut_csv} CSV excerpts cut, want {want_cut_csv}"));
        }
        Ok(())
    }

    fn g_summary_facts(c: &Ctx) -> Result<(), String> {
        let w = &c.want;
        let st = status_of_facts(w);
        let count =
            |name: &str| -> usize { usize::from(st == name) + usize::from(name == "failed") };
        let reliable = w.health.clean && !w.health.truncated;
        let mut by: Vec<(String, usize)> = CATS.iter().map(|c| (c.0.to_string(), 0)).collect();
        for f in &w.findings {
            tick(1);
            if let Some(slot) = by.iter_mut().find(|(n, _)| *n == f.category) {
                slot.1 += 1;
            }
        }
        // Some only with a policy AND a reliable file (the case has one
        // audited file); never a zero over no reliable file.
        let want_residual = if reliable {
            c.policy.map(|_| h_want(w, &c.removed).residual)
        } else {
            None
        };
        let s = &c.summary;
        let want = SummaryF {
            files: 2,
            failed: count("failed"),
            unsupported: count("unsupported"),
            unreadable: count("unreadable"),
            issues: count("issues"),
            ok: count("ok"),
            findings: w.findings.len(),
            by_category: by,
            max_severity: w.max.clone(),
            residual: want_residual,
            reliable: usize::from(reliable),
            unreliable: usize::from(!reliable),
        };
        if *s != want {
            return Err(format!("summary {s:?}, want {want:?}"));
        }
        if s.reliable + s.unreliable != s.files - s.failed {
            return Err("reliable + unreliable is not the audited files".to_string());
        }
        // The same numbers in the JSON document.
        let j = parsed_json(c)?;
        let js = req(&j, "summary")?;
        let n = |k: &str| -> Result<usize, String> { Ok(uint(req(js, k)?, k)? as usize) };
        if n("files")? != want.files
            || n("failed")? != want.failed
            || n("unsupported")? != want.unsupported
            || n("unreadable")? != want.unreadable
            || n("issues")? != want.issues
            || n("ok")? != want.ok
            || n("findings")? != want.findings
            || n("reliable_files")? != want.reliable
            || n("unreliable_files")? != want.unreliable
        {
            return Err(format!("the JSON summary {js:?} is not {want:?}"));
        }
        for (name, count) in &want.by_category {
            let got = uint(req(req(js, "by_category")?, snake(name))?, name)? as usize;
            if got != *count {
                return Err(format!("by_category {name}: {got}, want {count}"));
            }
        }
        if opt_name(req(js, "max_severity")?, "max_severity")?
            != want.max_severity.as_ref().map(|m| lower(m))
        {
            return Err(format!(
                "summary max_severity {:?}",
                req(js, "max_severity")?
            ));
        }
        let rj = req(js, "residual_findings")?;
        if opt_u(rj)?.map(|v| v as usize) != want.residual {
            return Err(format!(
                "summary residual_findings {rj:?}, want {:?}",
                want.residual
            ));
        }
        Ok(())
    }

    /// Output size is linear in items plus findings (and the label and detail,
    /// each at most 4x their length escaped), with fixed per-item ceilings.
    fn g_render_linear(c: &Ctx) -> Result<(), String> {
        let n = c.want.items.len() + c.want.findings.len();
        tick(n + 1);
        let l = naive_safe_text(&c.label).len();
        let d = naive_safe_text(&c.detail).len();
        let rows = c.csv.matches("\r\n").count();
        let json_cap = 8_192 + 3 * (l + d) + 2_048 * n;
        let text_cap = 8_192 + 3 * (l + d) + 1_024 * n;
        let csv_cap = 4_096 + rows * (3 * l + 1_024) + 3 * d;
        for (name, len, cap) in [
            ("json", c.json.len(), json_cap),
            ("text", c.text.len(), text_cap),
            ("csv", c.csv.len(), csv_cap),
        ] {
            if len > cap {
                return Err(format!(
                    "{name} is {len} bytes for {n} items and findings (ceiling {cap})"
                ));
            }
        }
        if rows > n + 4 {
            return Err(format!("{rows} CSV rows for {n} items and findings"));
        }
        Ok(())
    }

    /// Every guard, by name. Each one runs alone through [`run_guard`].
    pub const GUARDS: [(&str, Guard); 9] = [
        ("json_wellformed", g_json_wellformed),
        ("json_shape", g_json_shape),
        ("json_facts", g_json_facts),
        ("csv_wellformed", g_csv_wellformed),
        ("csv_rows", g_csv_rows),
        ("text_safe", g_text_safe),
        ("values_gate", g_values_gate),
        ("summary_facts", g_summary_facts),
        ("render_linear", g_render_linear),
    ];

    /// Run ONE guard by name.
    pub fn run_guard(name: &str, c: &Ctx) -> Result<(), String> {
        let guard = GUARDS
            .iter()
            .find(|g| g.0 == name)
            .unwrap_or_else(|| panic!("no render guard named {name}"));
        (guard.1)(c)
    }

    /// Every guard that fails, with its message.
    pub fn violations(c: &Ctx) -> Vec<(&'static str, String)> {
        GUARDS
            .iter()
            .filter_map(|(name, g)| g(c).err().map(|m| (*name, m)))
            .collect()
    }

    /// The harness body: the three renderers return, are deterministic, and
    /// satisfy every guard.
    pub fn check(data: &[u8]) {
        let c = build(data);
        let bad = violations(&c);
        assert!(bad.is_empty(), "render invariants violated: {bad:?}");
    }
}
