// The SPEC-012 invariants of `audit`, shared by the fuzz target
// (`fuzz/fuzz_targets/audit.rs`) and the stable-toolchain layout test
// (`tests/fuzz_layout.rs`) with `include!`, so the two cannot drift. This file
// is not a crate root; it is text included at module level, and it defines one
// module, `audit_invariants`, whose `check` is the harness body. `cargo fmt`
// does not follow `include!`, so this file is formatted by hand.
//
// The audit is re-derived here, independently of `exif_core::audit` (a second,
// deliberately naive implementation: it writes the magics, the DEC-008 table,
// the severities and the thumbnail rule again and never calls `sniff` or
// `categorize`; it uses only the readers, `Container::parse` and `jpeg::read`,
// which the audit sits on). The library's answer is flattened to `Facts` (plain
// data, categories and kinds as their `Debug` names) and compared with the
// re-derived `Facts` by a list of named GUARDS. Every rule that is an iff is
// checked in BOTH directions: an expected item that is missing and an item that
// was not expected both fail, so a constant-true or constant-false
// implementation fails one of them.
//
// Each guard is a pure function of `(data, got, want)` and runs ALONE through
// `run_guard`, so the plants in `tests/fuzz_layout.rs` can show that every guard
// goes red on its own bug even when other guards overlap with it (the lesson of
// HANDOFF-009).

#[allow(dead_code)]
mod audit_invariants {
    use exif_core::audit::{audit, FileAudit, ItemKind};
    use exif_core::jpeg::{self, Region, SegmentKind};
    use exif_core::tiff::Container;

    /// The documented cap on items and, separately, on findings.
    pub const CAP: usize = 65_536;

    #[derive(Clone, Debug, PartialEq)]
    pub enum What {
        Tag {
            ifd: String,
            ifd_index: usize,
            tag: u16,
            field_type: u16,
            count: u32,
        },
        Region {
            kind: String,
            segment: usize,
            len: usize,
        },
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct ItemF {
        pub block: Option<usize>,
        pub segment: Option<usize>,
        pub after_scan: bool,
        pub what: What,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct ThumbF {
        pub range: Option<(u32, u32)>,
        pub ffd8: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct FindingF {
        pub category: String,
        pub item: Option<usize>,
        pub at: ItemF,
        pub thumb: Option<ThumbF>,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct HealthF {
        pub clean: bool,
        pub jpeg_issues: usize,
        pub exif_issues: usize,
        pub failed_blocks: usize,
        pub truncated: bool,
    }

    /// Everything the audit says, as plain data.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Facts {
        pub kind: String,
        pub items: Vec<ItemF>,
        pub findings: Vec<FindingF>,
        pub health: HealthF,
        pub max: Option<String>,
    }

    fn item_f(i: &exif_core::audit::Item<'_>) -> ItemF {
        let o = i.origin();
        let what = match i.kind() {
            ItemKind::Tag {
                ifd,
                ifd_index,
                tag,
                field_type,
                count,
            } => What::Tag {
                ifd: format!("{ifd:?}"),
                ifd_index,
                tag,
                field_type,
                count,
            },
            ItemKind::Region { kind, segment, len } => What::Region {
                kind: format!("{kind:?}"),
                segment,
                len,
            },
        };
        ItemF {
            block: o.block,
            segment: o.segment,
            after_scan: o.after_scan,
            what,
        }
    }

    /// The library's answer, flattened.
    pub fn facts(a: &FileAudit<'_>) -> Facts {
        let h = a.health();
        Facts {
            kind: format!("{:?}", a.kind()),
            items: a.items().iter().map(item_f).collect(),
            findings: a
                .findings()
                .iter()
                .map(|f| FindingF {
                    category: format!("{:?}", f.category),
                    item: f.item,
                    at: item_f(&f.at),
                    thumb: f.thumbnail.map(|t| ThumbF {
                        range: t.range,
                        ffd8: t.starts_with_ffd8,
                    }),
                })
                .collect(),
            health: HealthF {
                clean: h.clean,
                jpeg_issues: h.jpeg_issues,
                exif_issues: h.exif_issues,
                failed_blocks: h.failed_blocks,
                truncated: h.inventory_truncated,
            },
            max: a.max_severity().map(|s| format!("{s:?}")),
        }
    }

    // ── the independent re-derivation ───────────────────────────────────────

    /// The container's name from the first bytes, written out again.
    fn magic(d: &[u8]) -> &'static str {
        let head = &d[..d.len().min(12)];
        if head.is_empty() {
            "Empty"
        } else if head.starts_with(&[0xFF, 0xD8]) {
            "Jpeg"
        } else if head.starts_with(b"II*\0") || head.starts_with(b"MM\0*") {
            "Tiff"
        } else if head.starts_with(b"II+\0") || head.starts_with(b"MM\0+") {
            "BigTiff"
        } else if head.starts_with(b"\x89PNG\r\n\x1a\n") {
            "Png"
        } else if head.len() == 12 && &head[..4] == b"RIFF" && &head[8..] == b"WEBP" {
            "Webp"
        } else if head.len() >= 8 && &head[4..8] == b"ftyp" {
            "Heic"
        } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
            "Gif"
        } else {
            "Unknown"
        }
    }

    pub fn kind_label(d: &[u8]) -> String {
        match magic(d) {
            m @ ("Jpeg" | "Tiff") => m.to_string(),
            m => format!("Unsupported({m})"),
        }
    }

    /// DEC-008, typed again: the category of a tag id outside GPS and Interop
    /// IFDs, where the id decides.
    fn by_id(tag: u16) -> Option<&'static str> {
        let table: [(u16, &str); 40] = [
            (0x010E, "FreeText"),
            (0x010F, "CameraModel"),
            (0x0110, "CameraModel"),
            (0x0131, "Software"),
            (0x0132, "Timestamps"),
            (0x013B, "Authorship"),
            (0x8298, "Authorship"),
            (0x9003, "Timestamps"),
            (0x9004, "Timestamps"),
            (0x9010, "Timestamps"),
            (0x9011, "Timestamps"),
            (0x9012, "Timestamps"),
            (0x9290, "Timestamps"),
            (0x9291, "Timestamps"),
            (0x9292, "Timestamps"),
            (0x9286, "FreeText"),
            (0x927C, "MakerNote"),
            (0xA420, "DeviceId"),
            (0xA430, "Authorship"),
            (0xA431, "DeviceId"),
            (0xA435, "DeviceId"),
            (0xA433, "CameraModel"),
            (0xA434, "CameraModel"),
            (0xC62F, "DeviceId"),
            (0x9C9B, "FreeText"),
            (0x9C9C, "FreeText"),
            (0x9C9E, "FreeText"),
            (0x9C9F, "FreeText"),
            (0x9C9D, "Authorship"),
            (0xA436, "FreeText"),
            (0xA437, "Authorship"),
            (0xA438, "Authorship"),
            (0xA439, "Software"),
            (0xA43A, "Software"),
            (0xA43B, "Software"),
            (0xA43C, "Software"),
            (0x02BC, "Xmp"),
            (0x83BB, "Iptc"),
            (0x8649, "Iptc"),
            (0x8773, "Icc"),
        ];
        table.iter().find(|r| r.0 == tag).map(|r| r.1)
    }

    pub fn severity_of(category: &str) -> &'static str {
        match category {
            "Gps" | "DeviceId" => "High",
            "Authorship" | "FreeText" | "Thumbnail" | "MakerNote" | "Xmp" | "Iptc" | "Jumbf" => {
                "Medium"
            }
            _ => "Low",
        }
    }

    fn rank(severity: &str) -> u8 {
        match severity {
            "High" => 2,
            "Medium" => 1,
            _ => 0,
        }
    }

    /// Accumulates the expected audit under the caps.
    struct Acc {
        items: Vec<ItemF>,
        findings: Vec<FindingF>,
        truncated: bool,
        max: Option<&'static str>,
    }

    impl Acc {
        fn push(&mut self, at: ItemF, category: Option<&'static str>, thumb: Option<ThumbF>) {
            let index = if self.items.len() < CAP {
                self.items.push(at.clone());
                Some(self.items.len() - 1)
            } else {
                self.truncated = true;
                None
            };
            let Some(category) = category else { return };
            let sev = severity_of(category);
            if self.max.is_none_or(|m| rank(m) < rank(sev)) {
                self.max = Some(sev);
            }
            if self.findings.len() < CAP {
                let thumb = if category == "Thumbnail" { thumb } else { None };
                self.findings.push(FindingF {
                    category: category.to_string(),
                    item: index,
                    at,
                    thumb,
                });
            } else {
                self.truncated = true;
            }
        }
    }

    /// The thumbnail range of an IFD1, read byte by byte.
    fn thumb_of(c: &Container<'_>, ifd: &exif_core::tiff::Ifd, blob: &[u8]) -> ThumbF {
        let none = ThumbF {
            range: None,
            ffd8: false,
        };
        let le = blob.starts_with(b"II");
        let get = |tag: u16| -> Option<u32> {
            let e = ifd.entries().iter().find(|e| e.tag() == tag)?;
            if e.count() != 1 || !(e.field_type() == 3 || e.field_type() == 4) {
                return None;
            }
            let p = c.payload(e).ok()?;
            if e.field_type() == 3 {
                let b: [u8; 2] = p.get(..2)?.try_into().ok()?;
                Some(u32::from(if le {
                    u16::from_le_bytes(b)
                } else {
                    u16::from_be_bytes(b)
                }))
            } else {
                let b: [u8; 4] = p.get(..4)?.try_into().ok()?;
                Some(if le {
                    u32::from_le_bytes(b)
                } else {
                    u32::from_be_bytes(b)
                })
            }
        };
        let (Some(o), Some(l)) = (get(0x0201), get(0x0202)) else {
            return none;
        };
        let end = u64::from(o) + u64::from(l);
        if end > blob.len() as u64 {
            return none;
        }
        ThumbF {
            range: Some((o, l)),
            ffd8: blob[o as usize..end as usize].starts_with(&[0xFF, 0xD8]),
        }
    }

    fn walk_container(
        acc: &mut Acc,
        c: &Container<'_>,
        blob: &[u8],
        block: Option<usize>,
        segment: Option<usize>,
        after_scan: bool,
    ) {
        for (ifd_index, ifd) in c.ifds().iter().enumerate() {
            let kind = format!("{:?}", ifd.kind());
            let has_0201 = ifd.entries().iter().any(|e| e.tag() == 0x0201);
            let thumb = (kind == "Ifd1").then(|| thumb_of(c, ifd, blob));
            for e in ifd.entries() {
                let tag = e.tag();
                let category: Option<&'static str> = if kind == "Gps" {
                    Some("Gps")
                } else if kind == "Interop" {
                    None
                } else if kind == "Ifd1" && (tag == 0x0201 || (tag == 0x0111 && !has_0201)) {
                    Some("Thumbnail")
                } else {
                    by_id(tag)
                };
                let at = ItemF {
                    block,
                    segment,
                    after_scan,
                    what: What::Tag {
                        ifd: kind.clone(),
                        ifd_index,
                        tag,
                        field_type: e.field_type(),
                        count: e.count(),
                    },
                };
                let t = if category == Some("Thumbnail") {
                    if tag == 0x0201 {
                        thumb.clone()
                    } else {
                        Some(ThumbF {
                            range: None,
                            ffd8: false,
                        })
                    }
                } else {
                    None
                };
                acc.push(at, category, t);
            }
        }
    }

    /// The expected audit of `data`, from scratch.
    pub fn rederive(data: &[u8]) -> Facts {
        let kind = kind_label(data);
        let mut acc = Acc {
            items: Vec::new(),
            findings: Vec::new(),
            truncated: false,
            max: None,
        };
        let (mut jpeg_issues, mut exif_issues, mut failed) = (0usize, 0usize, 0usize);
        match kind.as_str() {
            "Tiff" => match Container::parse(data) {
                Err(_) => failed = 1,
                Ok(c) => {
                    exif_issues = c.issues().len();
                    walk_container(&mut acc, &c, data, None, None, false);
                }
            },
            "Jpeg" => match jpeg::read(data) {
                Err(_) => failed = 1,
                Ok(meta) => {
                    jpeg_issues = meta.jpeg_issues().len();
                    let segs = meta.jpeg().segments();
                    let first_sos = segs.iter().position(|s| s.kind() == SegmentKind::Sos);
                    let after = |i: usize| first_sos.is_some_and(|p| i > p);
                    for (b, block) in meta.exif_blocks().iter().enumerate() {
                        match block.container() {
                            Err(_) => failed += 1,
                            Ok(c) => {
                                exif_issues += c.issues().len();
                                walk_container(
                                    &mut acc,
                                    c,
                                    block.tiff(),
                                    Some(b),
                                    Some(block.segment()),
                                    after(block.segment()),
                                );
                            }
                        }
                    }
                    for r in meta.regions() {
                        let (name, category) = match r {
                            Region::Xmp { .. } => ("Xmp", "Xmp"),
                            Region::ExtendedXmp { .. } => ("ExtendedXmp", "Xmp"),
                            Region::Icc { .. } => ("Icc", "Icc"),
                            Region::Iptc { .. } => ("Iptc", "Iptc"),
                            Region::Jumbf { .. } => ("Jumbf", "Jumbf"),
                            _ => panic!("a Region variant the harness does not know"),
                        };
                        let at = ItemF {
                            block: None,
                            segment: Some(r.segment()),
                            after_scan: after(r.segment()),
                            what: What::Region {
                                kind: name.to_string(),
                                segment: r.segment(),
                                len: r.data().len(),
                            },
                        };
                        acc.push(at, Some(category), None);
                    }
                }
            },
            _ => {}
        }
        let supported = kind == "Tiff" || kind == "Jpeg";
        let clean =
            supported && jpeg_issues == 0 && exif_issues == 0 && failed == 0 && !acc.truncated;
        Facts {
            kind,
            items: acc.items,
            findings: acc.findings,
            health: HealthF {
                clean,
                jpeg_issues,
                exif_issues,
                failed_blocks: failed,
                truncated: acc.truncated,
            },
            max: acc.max.map(str::to_string),
        }
    }

    // ── the guards ──────────────────────────────────────────────────────────

    /// The first position at which two slices differ, as a short message.
    fn same<T: PartialEq + std::fmt::Debug>(
        what: &str,
        got: &[T],
        want: &[T],
    ) -> Result<(), String> {
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            if g != w {
                return Err(format!("{what}[{i}]: got {g:?}, want {w:?}"));
            }
        }
        if got.len() != want.len() {
            return Err(format!(
                "{what}: {} entries, want {}",
                got.len(),
                want.len()
            ));
        }
        Ok(())
    }

    type Guard = fn(&[u8], &Facts, &Facts) -> Result<(), String>;

    fn is_gps_tag(w: &What) -> bool {
        matches!(w, What::Tag { ifd, .. } if ifd == "Gps")
    }

    fn g_kind(_: &[u8], got: &Facts, want: &Facts) -> Result<(), String> {
        if got.kind != want.kind {
            return Err(format!("kind {:?}, want {:?}", got.kind, want.kind));
        }
        Ok(())
    }

    fn g_unsupported(_: &[u8], got: &Facts, _: &Facts) -> Result<(), String> {
        let unsupported = got.kind.starts_with("Unsupported");
        if unsupported {
            if !got.items.is_empty() || !got.findings.is_empty() {
                return Err("an unsupported container has items or findings".into());
            }
            if got.health.clean {
                return Err("an unsupported container is clean".into());
            }
            if got.max.is_some() {
                return Err("an unsupported container has a severity".into());
            }
        }
        // And the other direction: clean means a readable, supported container.
        if got.health.clean && unsupported {
            return Err("clean but unsupported".into());
        }
        if got.health.clean && !(got.kind == "Tiff" || got.kind == "Jpeg") {
            return Err(format!("clean for kind {:?}", got.kind));
        }
        Ok(())
    }

    fn g_caps(_: &[u8], got: &Facts, _: &Facts) -> Result<(), String> {
        if got.items.len() > CAP {
            return Err(format!("{} items over the cap", got.items.len()));
        }
        if got.findings.len() > CAP {
            return Err(format!("{} findings over the cap", got.findings.len()));
        }
        // A finding that did not fit the inventory means the inventory is full.
        if got.findings.iter().any(|f| f.item.is_none()) && got.items.len() != CAP {
            return Err("an unlisted finding but the inventory is not full".into());
        }
        Ok(())
    }

    fn g_indices(_: &[u8], got: &Facts, _: &Facts) -> Result<(), String> {
        for (n, f) in got.findings.iter().enumerate() {
            match f.item {
                Some(i) => match got.items.get(i) {
                    None => return Err(format!("finding {n}: item index {i} out of range")),
                    Some(item) if *item != f.at => {
                        return Err(format!("finding {n}: item {i} is not the finding's item"))
                    }
                    Some(_) => {}
                },
                None => {
                    if !got.health.truncated {
                        return Err(format!("finding {n} has no item but nothing was truncated"));
                    }
                }
            }
        }
        // Findings are in item order: the listed indices never go backwards.
        let listed: Vec<usize> = got.findings.iter().filter_map(|f| f.item).collect();
        if listed.windows(2).any(|w| w[0] >= w[1]) {
            return Err("finding item indices are not strictly increasing".into());
        }
        Ok(())
    }

    fn g_category_shape(_: &[u8], got: &Facts, _: &Facts) -> Result<(), String> {
        for (n, f) in got.findings.iter().enumerate() {
            // A region item is exactly its DEC-008 region category. The other
            // direction is NOT "region categories only on regions": DEC-008
            // also puts the TIFF tags 0x02BC (XMP), 0x83BB / 0x8649 (IPTC) and
            // 0x8773 (ICC) in Xmp, Iptc and Icc. Only Jumbf is region-only.
            match &f.at.what {
                What::Region { kind, .. } => {
                    let want = match kind.as_str() {
                        "Xmp" | "ExtendedXmp" => "Xmp",
                        "Iptc" => "Iptc",
                        "Icc" => "Icc",
                        "Jumbf" => "Jumbf",
                        other => return Err(format!("finding {n}: unknown region kind {other}")),
                    };
                    if f.category != want {
                        return Err(format!(
                            "finding {n}: a {kind} region categorised {}",
                            f.category
                        ));
                    }
                }
                What::Tag { .. } => {
                    if f.category == "Jumbf" {
                        return Err(format!("finding {n}: Jumbf on a tag item"));
                    }
                }
            }
            // Gps iff the item is a tag of a GPS IFD (both directions).
            if (f.category == "Gps") != is_gps_tag(&f.at.what) {
                return Err(format!(
                    "finding {n}: Gps iff a GPS-IFD tag ({})",
                    f.category
                ));
            }
            // A thumbnail range accompanies exactly the Thumbnail category.
            if (f.category == "Thumbnail") != f.thumb.is_some() {
                return Err(format!("finding {n}: thumbnail range iff Thumbnail"));
            }
        }
        // Every region item has exactly one finding (they are all categorised).
        for (i, item) in got.items.iter().enumerate() {
            if matches!(item.what, What::Region { .. })
                && !got.findings.iter().any(|f| f.item == Some(i))
            {
                return Err(format!("region item {i} has no finding"));
            }
        }
        Ok(())
    }

    fn g_inventory(_: &[u8], got: &Facts, want: &Facts) -> Result<(), String> {
        same("items", &got.items, &want.items)
    }

    fn g_findings(_: &[u8], got: &Facts, want: &Facts) -> Result<(), String> {
        same("findings", &got.findings, &want.findings)
    }

    fn g_gps_iff(_: &[u8], got: &Facts, _: &Facts) -> Result<(), String> {
        let gps_item = got
            .items
            .iter()
            .chain(got.findings.iter().map(|f| &f.at))
            .any(|i| is_gps_tag(&i.what));
        let gps_finding = got.findings.iter().any(|f| f.category == "Gps");
        if gps_item != gps_finding {
            return Err(format!(
                "a GPS-IFD tag item exists: {gps_item}, a Gps finding exists: {gps_finding}"
            ));
        }
        Ok(())
    }

    /// The TIFF bytes an origin's offsets are relative to.
    fn blob_for(data: &[u8], block: Option<usize>) -> Option<Vec<u8>> {
        match block {
            None => Some(data.to_vec()),
            Some(b) => jpeg::read(data)
                .ok()?
                .exif_blocks()
                .get(b)
                .map(|x| x.tiff().to_vec()),
        }
    }

    fn g_thumbnail(data: &[u8], got: &Facts, _: &Facts) -> Result<(), String> {
        for (n, f) in got.findings.iter().enumerate() {
            let (Some(t), "Thumbnail") = (&f.thumb, f.category.as_str()) else {
                continue;
            };
            match t.range {
                None => {
                    if t.ffd8 {
                        return Err(format!("finding {n}: FF D8 flag without a range"));
                    }
                }
                Some((o, l)) => {
                    let blob = blob_for(data, f.at.block).ok_or_else(|| {
                        format!("finding {n}: no blob for block {:?}", f.at.block)
                    })?;
                    let end = u64::from(o) + u64::from(l);
                    if end > blob.len() as u64 {
                        return Err(format!(
                            "finding {n}: range ({o}, {l}) outside a {} byte blob",
                            blob.len()
                        ));
                    }
                    let starts = blob[o as usize..end as usize].starts_with(&[0xFF, 0xD8]);
                    if starts != t.ffd8 {
                        return Err(format!(
                            "finding {n}: FF D8 flag is {}, the bytes say {starts}",
                            t.ffd8
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn g_health(_: &[u8], got: &Facts, want: &Facts) -> Result<(), String> {
        if got.health != want.health {
            return Err(format!("health {:?}, want {:?}", got.health, want.health));
        }
        // `clean` iff readable, no issue of any layer, no failed block, no
        // truncation; restated from `got` alone so it cannot ride on `want`.
        let supported = got.kind == "Tiff" || got.kind == "Jpeg";
        let clean = supported
            && got.health.jpeg_issues == 0
            && got.health.exif_issues == 0
            && got.health.failed_blocks == 0
            && !got.health.truncated;
        if got.health.clean != clean {
            return Err(format!(
                "clean is {}, its definition says {clean}",
                got.health.clean
            ));
        }
        Ok(())
    }

    fn g_max(_: &[u8], got: &Facts, want: &Facts) -> Result<(), String> {
        if got.max != want.max {
            return Err(format!("max severity {:?}, want {:?}", got.max, want.max));
        }
        // Never below the severity of a finding the audit lists.
        let listed = got
            .findings
            .iter()
            .map(|f| rank(severity_of(&f.category)))
            .max();
        if let Some(l) = listed {
            if got.max.as_deref().map(rank) < Some(l) {
                return Err("max severity below a listed finding".into());
            }
        }
        Ok(())
    }

    /// Every guard, by name. Each one runs alone through [`run_guard`].
    pub const GUARDS: [(&str, Guard); 11] = [
        ("kind", g_kind),
        ("unsupported", g_unsupported),
        ("caps", g_caps),
        ("indices", g_indices),
        ("category_shape", g_category_shape),
        ("inventory", g_inventory),
        ("findings", g_findings),
        ("gps_iff", g_gps_iff),
        ("thumbnail", g_thumbnail),
        ("health", g_health),
        ("max_severity", g_max),
    ];

    /// Run ONE guard by name.
    pub fn run_guard(name: &str, data: &[u8], got: &Facts, want: &Facts) -> Result<(), String> {
        let guard = GUARDS
            .iter()
            .find(|g| g.0 == name)
            .unwrap_or_else(|| panic!("no guard named {name}"));
        (guard.1)(data, got, want)
    }

    /// Every guard that fails, with its message.
    pub fn violations(data: &[u8], got: &Facts, want: &Facts) -> Vec<(&'static str, String)> {
        GUARDS
            .iter()
            .filter_map(|(name, g)| g(data, got, want).err().map(|m| (*name, m)))
            .collect()
    }

    /// The harness body: `audit` returns, is deterministic, and satisfies
    /// every guard.
    pub fn check(data: &[u8]) {
        let a = audit(data);
        let got = facts(&a);
        let again = facts(&audit(data));
        assert!(got == again, "audit is not deterministic");
        let want = rederive(data);
        let bad = violations(data, &got, &want);
        assert!(bad.is_empty(), "audit invariants violated: {bad:?}");
    }
}
