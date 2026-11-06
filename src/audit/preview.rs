//! The strip-policy preview (SPEC-014, DEC-009): what a policy would do to
//! each finding of an audit, before any writer exists.
//!
//! [`preview`] is a lookup of each finding's [`Category`] in the policy's
//! [`StripSpec`], plus one honest-limit rule: a block the library cannot read
//! that is **kept** but could hold what the policy removes is
//! [`Disposition::KeptUnread`], not [`Disposition::Kept`]. Nothing else is
//! consulted (not the severity, the container or the origin).
//!
//! The preview covers findings only. Inventory-only entries have no category
//! and are out of scope (DEC-009, "What the preview does not say").
//!
//! Provenance (DEC-003, class 1): net-new code over [`Category`] and DEC-009.

use super::{Category, FileAudit, Severity};
use crate::policy::{may_hold, StripPolicy, StripSpec};

/// What a policy does to one finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// The finding's category is in the policy's set.
    Removed,
    /// Not removed, and nothing the policy targets can hide inside it.
    Kept,
    /// Not removed; a block the library cannot read that may hold a category
    /// the policy removes. The honest-limit outcome.
    KeptUnread,
}

impl Disposition {
    /// A stable lowercase name for reports.
    pub fn name(self) -> &'static str {
        match self {
            Disposition::Removed => "removed",
            Disposition::Kept => "kept",
            Disposition::KeptUnread => "kept_unread",
        }
    }
}

/// How many findings fell into each [`Disposition`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    /// Findings the policy removes.
    pub removed: usize,
    /// Findings kept, with nothing targeted possibly inside.
    pub kept: usize,
    /// Findings kept that may hold what the policy removes.
    pub kept_unread: usize,
}

/// The result of [`preview`]: one disposition per finding, in the audit's
/// finding order, and the facts derived from them.
///
/// "Zero residual findings" supports "nothing the audit found would remain"
/// only when [`Preview::reliable`] is true: an unsupported, empty, unreadable
/// or truncated audit is never reliable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    policy: StripPolicy,
    dispositions: Vec<Disposition>,
    counts: Counts,
    residual_severity: Option<Severity>,
    reliable: bool,
}

impl Preview {
    /// The policy previewed.
    pub fn policy(&self) -> StripPolicy {
        self.policy
    }

    /// One disposition per finding, parallel to [`FileAudit::findings`].
    pub fn dispositions(&self) -> &[Disposition] {
        &self.dispositions
    }

    /// The disposition of finding `finding`, if there is one.
    pub fn disposition(&self, finding: usize) -> Option<Disposition> {
        self.dispositions.get(finding).copied()
    }

    /// The three counts.
    pub fn counts(&self) -> Counts {
        self.counts
    }

    /// Findings that would remain: kept plus kept-unread.
    pub fn residual_findings(&self) -> usize {
        self.counts.kept.saturating_add(self.counts.kept_unread)
    }

    /// The highest severity among the findings that would remain. When the
    /// audit's findings were truncated the dropped findings' categories are
    /// unknown, so this is the audit's own maximum (nothing assumed removed),
    /// except that a policy removing every category leaves `None`.
    pub fn residual_severity(&self) -> Option<Severity> {
        self.residual_severity
    }

    /// Whether the audit was clean and its findings complete.
    pub fn reliable(&self) -> bool {
        self.reliable
    }
}

/// The disposition of one category under one spec (DEC-009 rule 2).
fn disposition_of(c: Category, spec: StripSpec) -> Disposition {
    if spec.contains(c) {
        Disposition::Removed
    } else if c.is_block() && may_hold(c).intersects(spec) {
        Disposition::KeptUnread
    } else {
        Disposition::Kept
    }
}

/// Preview `policy` over `audit`. Total: never fails and never panics; work is
/// linear in the findings.
pub fn preview(audit: &FileAudit<'_>, policy: StripPolicy) -> Preview {
    let spec = policy.spec();
    let mut counts = Counts {
        removed: 0,
        kept: 0,
        kept_unread: 0,
    };
    let mut residual: Option<Severity> = None;
    let mut dispositions = Vec::with_capacity(audit.findings().len());
    for f in audit.findings() {
        let d = disposition_of(f.category, spec);
        match d {
            Disposition::Removed => counts.removed = counts.removed.saturating_add(1),
            Disposition::Kept => counts.kept = counts.kept.saturating_add(1),
            Disposition::KeptUnread => counts.kept_unread = counts.kept_unread.saturating_add(1),
        }
        if d != Disposition::Removed {
            let s = f.category.severity();
            if residual.is_none_or(|m| m < s) {
                residual = Some(s);
            }
        }
        dispositions.push(d);
    }
    let health = audit.health();
    let residual_severity = if health.inventory_truncated {
        if spec == StripSpec::all() {
            None
        } else {
            audit.max_severity()
        }
    } else {
        residual
    };
    Preview {
        policy,
        dispositions,
        counts,
        residual_severity,
        reliable: health.clean && !health.inventory_truncated,
    }
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
    use crate::audit::{audit, ContainerKind, Finding, Health, Item, ItemKind, Origin, Sniffed};
    use crate::tiff::IfdKind;
    use Category::*;
    use Disposition::{Kept, KeptUnread, Removed};

    /// DEC-009's block outcomes, typed by hand: for each block the cells
    /// `[Gps, GpsAndDevice, AllPii, All, Everything]`.
    const KEPT_BLOCK: [(Category, [Disposition; 5]); 6] = [
        (
            Thumbnail,
            [KeptUnread, KeptUnread, Removed, Removed, Removed],
        ),
        (
            MakerNote,
            [KeptUnread, KeptUnread, Removed, Removed, Removed],
        ),
        (Xmp, [KeptUnread, KeptUnread, KeptUnread, Removed, Removed]),
        (Iptc, [Kept, Kept, KeptUnread, Removed, Removed]),
        (
            Jumbf,
            [KeptUnread, KeptUnread, KeptUnread, KeptUnread, Removed],
        ),
        (Icc, [Kept, Kept, Kept, Kept, Removed]),
    ];

    /// An audit holding one finding per given category (no real file behind
    /// it), with a chosen health.
    fn synthetic(cats: &[Category], clean: bool, truncated: bool) -> FileAudit<'static> {
        let origin = Origin {
            block: None,
            segment: None,
            after_scan: false,
        };
        let item = Item::new(
            origin,
            ItemKind::Tag {
                ifd: IfdKind::Ifd0,
                ifd_index: 0,
                tag: 0,
                field_type: 2,
                count: 1,
            },
        );
        let findings: Vec<Finding<'static>> = cats
            .iter()
            .map(|c| Finding {
                category: *c,
                item: None,
                at: item,
                thumbnail: None,
            })
            .collect();
        let max = cats.iter().map(|c| c.severity()).max();
        FileAudit {
            kind: ContainerKind::Tiff,
            items: Vec::new(),
            findings,
            health: Health {
                clean,
                jpeg_issues: 0,
                exif_issues: 0,
                failed_blocks: 0,
                inventory_truncated: truncated,
            },
            max_severity: max,
        }
    }

    #[test]
    fn disposition_rule_matches_the_kept_block_grid() {
        for (block, cells) in KEPT_BLOCK {
            let a = synthetic(&[block], true, false);
            for (col, preset) in StripPolicy::PRESETS.iter().enumerate() {
                let p = preview(&a, *preset);
                assert_eq!(
                    p.dispositions(),
                    &[cells[col]],
                    "{} / {block:?}",
                    preset.name()
                );
            }
        }
    }

    #[test]
    fn tag_categories_are_never_kept_unread() {
        for c in [
            Gps,
            DeviceId,
            Authorship,
            FreeText,
            Timestamps,
            CameraModel,
            Software,
        ] {
            let a = synthetic(&[c], true, false);
            for preset in StripPolicy::PRESETS {
                assert_ne!(preview(&a, preset).dispositions()[0], KeptUnread, "{c:?}");
            }
            // Custom sets too, including every singleton and the full set.
            for s in Category::ALL.iter().map(|x| StripSpec::none().include(*x)) {
                let d = preview(&a, StripPolicy::Custom(s)).dispositions()[0];
                assert_ne!(d, KeptUnread, "{c:?}");
            }
        }
        // A kept tag is simply Kept.
        let a = synthetic(&[Software], true, false);
        assert_eq!(preview(&a, StripPolicy::Gps).dispositions(), &[Kept]);
    }

    #[test]
    fn custom_none_keeps_everything_and_flags_nothing_unread() {
        let a = synthetic(&Category::ALL, true, false);
        let p = preview(&a, StripPolicy::Custom(StripSpec::none()));
        assert!(p.dispositions().iter().all(|d| *d == Kept));
        assert_eq!(
            p.counts(),
            Counts {
                removed: 0,
                kept: 13,
                kept_unread: 0
            }
        );
        assert_eq!(p.residual_findings(), 13);
    }

    #[test]
    fn custom_all_equals_everything() {
        let a = synthetic(&Category::ALL, true, false);
        let e = preview(&a, StripPolicy::Everything);
        let c = preview(&a, StripPolicy::Custom(StripSpec::all()));
        assert_eq!(e.dispositions(), c.dispositions());
        assert_eq!(e.counts(), c.counts());
        assert_eq!(e.residual_findings(), 0);
        assert_eq!(c.residual_severity(), None);
        assert!(c.dispositions().iter().all(|d| *d == Removed));
    }

    #[test]
    fn counts_sum_to_findings_and_residual_is_kept_plus_unread() {
        let a = synthetic(&Category::ALL, true, false);
        let want: [(usize, usize, usize); 5] = [
            // (removed, kept, kept_unread) per preset, derived by hand from
            // DEC-009 over one finding per category.
            (1, 8, 4),
            (2, 7, 4),
            (8, 2, 3),
            (11, 1, 1),
            (13, 0, 0),
        ];
        for (preset, (r, k, u)) in StripPolicy::PRESETS.iter().zip(want) {
            let p = preview(&a, *preset);
            let c = p.counts();
            assert_eq!(
                (c.removed, c.kept, c.kept_unread),
                (r, k, u),
                "{}",
                preset.name()
            );
            assert_eq!(c.removed + c.kept + c.kept_unread, a.findings().len());
            assert_eq!(p.residual_findings(), k + u);
            assert_eq!(p.dispositions().len(), a.findings().len());
        }
    }

    #[test]
    fn residual_severity_is_the_max_over_non_removed_findings() {
        // Gps (High) removed under Gps; the rest decide.
        let a = synthetic(&[Gps, Software, Authorship], true, false);
        assert_eq!(
            preview(&a, StripPolicy::Gps).residual_severity(),
            Some(Severity::Medium)
        );
        // Under AllPii: Authorship removed, Software (Low) left.
        assert_eq!(
            preview(&a, StripPolicy::AllPii).residual_severity(),
            Some(Severity::Low)
        );
        assert_eq!(preview(&a, StripPolicy::All).residual_severity(), None);
        // A KeptUnread block counts: a Jumbf (Medium) under Gps/All.
        let b = synthetic(&[Gps, Jumbf], true, false);
        assert_eq!(
            preview(&b, StripPolicy::Gps).residual_severity(),
            Some(Severity::Medium)
        );
        assert_eq!(
            preview(&b, StripPolicy::All).residual_severity(),
            Some(Severity::Medium)
        );
        assert_eq!(
            preview(&b, StripPolicy::Everything).residual_severity(),
            None
        );
        // A High finding that is kept.
        let c = synthetic(&[Gps, Software], true, false);
        assert_eq!(
            preview(&c, StripPolicy::Custom(StripSpec::none())).residual_severity(),
            Some(Severity::High)
        );
        // No findings: None.
        let e = synthetic(&[], true, false);
        assert_eq!(preview(&e, StripPolicy::Gps).residual_severity(), None);
    }

    #[test]
    fn truncated_audit_residual_severity_is_conservative() {
        // The audit's own max_severity is High (a dropped Gps, say), but the
        // stored findings are all Low: the dropped ones are assumed kept.
        let mut a = synthetic(&[Software], true, true);
        a.max_severity = Some(Severity::High);
        // Computed alone it would be Low (Software kept under Gps); truncated
        // gives the audit's own High.
        assert_eq!(
            preview(&a, StripPolicy::Gps).residual_severity(),
            Some(Severity::High)
        );
        // Even where the stored findings would all be removed.
        assert_eq!(
            preview(&a, StripPolicy::All).residual_severity(),
            Some(Severity::High)
        );
        // The all() exception: a spec removing every category leaves None.
        assert_eq!(
            preview(&a, StripPolicy::Everything).residual_severity(),
            None
        );
        assert_eq!(
            preview(&a, StripPolicy::Custom(StripSpec::all())).residual_severity(),
            None
        );
        // Not all(): every other category set, even 12 of 13, is conservative.
        let almost = StripSpec::all().exclude(Icc);
        assert_eq!(
            preview(&a, StripPolicy::Custom(almost)).residual_severity(),
            Some(Severity::High)
        );
        // A truncated audit with no severity at all stays None.
        let mut n = synthetic(&[], true, true);
        n.max_severity = None;
        assert_eq!(preview(&n, StripPolicy::Gps).residual_severity(), None);
        // And is never reliable.
        assert!(!preview(&a, StripPolicy::Gps).reliable());
        // Dispositions are still per stored finding.
        assert_eq!(preview(&a, StripPolicy::All).dispositions(), &[Removed]);
    }

    #[test]
    fn unsupported_empty_and_unreadable_audits_have_no_dispositions_and_are_not_reliable() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
        let tiff_bad = b"II*\0\x08\0\0\0\xFF\xFF"; // IFD0 declares more than exists
        let cases: [(&str, Vec<u8>); 4] = [
            ("png", png.to_vec()),
            ("empty", Vec::new()),
            ("unreadable tiff", tiff_bad.to_vec()),
            ("garbage", b"not an image at all".to_vec()),
        ];
        for (label, bytes) in cases {
            let a = audit(&bytes);
            assert!(!a.health().clean, "{label}: premise");
            for preset in StripPolicy::PRESETS {
                let p = preview(&a, preset);
                assert!(p.dispositions().is_empty(), "{label}");
                assert!(!p.reliable(), "{label}");
                assert_eq!(p.residual_findings(), 0, "{label}");
                assert_eq!(p.residual_severity(), None, "{label}");
            }
        }
        // Sanity: the PNG really is the unsupported kind.
        assert!(matches!(
            audit(png).kind(),
            ContainerKind::Unsupported(Sniffed::Png)
        ));
        // And a reliable one exists: a clean file with no findings.
        let ok = synthetic(&[], true, false);
        assert!(preview(&ok, StripPolicy::Gps).reliable());
        // clean = false alone makes it unreliable.
        let dirty = synthetic(&[Gps], false, false);
        assert!(!preview(&dirty, StripPolicy::Gps).reliable());
    }

    #[test]
    fn preview_is_deterministic() {
        let a = synthetic(&Category::ALL, true, false);
        for preset in StripPolicy::PRESETS {
            assert_eq!(preview(&a, preset), preview(&a, preset));
        }
        assert_eq!(preview(&a, StripPolicy::All).policy(), StripPolicy::All);
        let p = preview(&a, StripPolicy::Gps);
        assert_eq!(p.disposition(0), Some(Removed));
        assert_eq!(p.disposition(12), p.dispositions().last().copied());
        assert_eq!(p.disposition(13), None);
    }

    #[test]
    fn dispositions_come_out_in_finding_order() {
        // Order matters: Gps first, then Icc, under Gps: Removed then Kept.
        let a = synthetic(&[Gps, Icc, Xmp], true, false);
        assert_eq!(
            preview(&a, StripPolicy::Gps).dispositions(),
            &[Removed, Kept, KeptUnread]
        );
    }
}
