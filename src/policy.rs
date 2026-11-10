//! Strip policies as data (SPEC-014, DEC-009).
//!
//! A policy is a **set of DEC-008 categories** ([`StripSpec`]); a preset is a
//! named set ([`StripPolicy`]). Nothing here reads or writes bytes: the policy
//! preview ([`crate::audit::preview`]) is a lookup of each finding's category
//! in the set, and the future writer takes the same sets.
//!
//! DEC-009 is the normative statement of every preset cell and of the
//! may-hold relation ([`may_hold`]); the tests carry both again as hand-typed
//! grids and compare cell by cell.
//!
//! Provenance (DEC-003, class 1): net-new code over this crate's own
//! [`Category`] and DEC-009. No ExifTool source is read.

use crate::audit::Category;

/// A set of categories, held as a bit set over [`Category::ALL`].
///
/// There is no constructor from a raw integer: a set can only be built from
/// categories, so every value is a subset of the categories that exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct StripSpec(u16);

/// The bit of one category: a literal per category (the library's lint set
/// forbids `<<` on integers), in [`Category::ALL`] order.
fn bit(c: Category) -> u16 {
    match c {
        Category::Gps => 0x0001,
        Category::DeviceId => 0x0002,
        Category::Authorship => 0x0004,
        Category::FreeText => 0x0008,
        Category::Thumbnail => 0x0010,
        Category::MakerNote => 0x0020,
        Category::Xmp => 0x0040,
        Category::Iptc => 0x0080,
        Category::Jumbf => 0x0100,
        Category::Timestamps => 0x0200,
        Category::CameraModel => 0x0400,
        Category::Software => 0x0800,
        Category::Icc => 0x1000,
        Category::EmbeddedImage => 0x2000,
        Category::OtherSegment => 0x4000,
    }
}

impl StripSpec {
    /// The empty set: removes nothing.
    pub const fn none() -> StripSpec {
        StripSpec(0)
    }

    /// Every category in [`Category::ALL`].
    pub fn all() -> StripSpec {
        StripSpec::of(&Category::ALL)
    }

    /// The set of exactly these categories.
    fn of(categories: &[Category]) -> StripSpec {
        categories
            .iter()
            .fold(StripSpec::none(), |s, c| s.include(*c))
    }

    /// This set plus `c`.
    pub fn include(self, c: Category) -> StripSpec {
        StripSpec(self.0 | bit(c))
    }

    /// This set minus `c`.
    pub fn exclude(self, c: Category) -> StripSpec {
        StripSpec(self.0 & !bit(c))
    }

    /// Whether `c` is in the set.
    pub fn contains(self, c: Category) -> bool {
        self.0 & bit(c) != 0
    }

    /// How many categories are in the set.
    pub fn len(self) -> usize {
        self.categories().count()
    }

    /// Whether the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every category of `self` is in `other`.
    pub fn is_subset_of(self, other: StripSpec) -> bool {
        self.0 & !other.0 == 0
    }

    /// Whether the two sets share a category.
    pub fn intersects(self, other: StripSpec) -> bool {
        self.0 & other.0 != 0
    }

    /// The categories of the set, in [`Category::ALL`] order.
    pub fn categories(self) -> impl Iterator<Item = Category> {
        Category::ALL.into_iter().filter(move |c| self.contains(*c))
    }
}

/// A strip policy: one of the five DEC-009 presets, or any set of categories.
///
/// The presets are nested: `Gps` is inside `GpsAndDevice`, which is inside
/// `AllPii`, then `All`, then `Everything` (every category).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StripPolicy {
    /// GPS only.
    Gps,
    /// GPS plus device identification.
    GpsAndDevice,
    /// Everything that can name a person or a device, including the thumbnail
    /// and the MakerNote; keeps XMP, IPTC, ICC, JUMBF and `Software`.
    AllPii,
    /// All Exif, XMP and IPTC; keeps ICC and JUMBF.
    All,
    /// Every category.
    Everything,
    /// Any set of categories.
    Custom(StripSpec),
}

impl StripPolicy {
    /// The five presets, smallest first.
    pub const PRESETS: [StripPolicy; 5] = [
        StripPolicy::Gps,
        StripPolicy::GpsAndDevice,
        StripPolicy::AllPii,
        StripPolicy::All,
        StripPolicy::Everything,
    ];

    /// The categories the policy removes (DEC-009, one column per preset).
    pub fn spec(self) -> StripSpec {
        match self {
            StripPolicy::Gps => StripSpec::of(&[Category::Gps]),
            StripPolicy::GpsAndDevice => StripSpec::of(&[Category::Gps, Category::DeviceId]),
            StripPolicy::AllPii => StripSpec::of(&[
                Category::Gps,
                Category::DeviceId,
                Category::Authorship,
                Category::FreeText,
                Category::Timestamps,
                Category::CameraModel,
                Category::Thumbnail,
                Category::MakerNote,
            ]),
            StripPolicy::All => StripSpec::of(&[
                Category::Gps,
                Category::DeviceId,
                Category::Authorship,
                Category::FreeText,
                Category::Timestamps,
                Category::CameraModel,
                Category::Thumbnail,
                Category::MakerNote,
                Category::Software,
                Category::Xmp,
                Category::Iptc,
            ]),
            StripPolicy::Everything => StripSpec::all(),
            StripPolicy::Custom(s) => s,
        }
    }

    /// A stable lowercase name for reports and flags.
    pub fn name(self) -> &'static str {
        match self {
            StripPolicy::Gps => "gps",
            StripPolicy::GpsAndDevice => "gps_and_device",
            StripPolicy::AllPii => "all_pii",
            StripPolicy::All => "all",
            StripPolicy::Everything => "everything",
            StripPolicy::Custom(_) => "custom",
        }
    }
}

/// The DEC-009 may-hold relation: the categories a block category can contain
/// without the library being able to see them. Empty for the tag categories
/// and for [`Category::Icc`].
pub fn may_hold(block: Category) -> StripSpec {
    const TAGS_AND_SOFTWARE: [Category; 7] = [
        Category::Gps,
        Category::DeviceId,
        Category::Authorship,
        Category::FreeText,
        Category::Timestamps,
        Category::CameraModel,
        Category::Software,
    ];
    match block {
        // A thumbnail JPEG may carry its own Exif.
        Category::Thumbnail => StripSpec::of(&TAGS_AND_SOFTWARE),
        // DEC-012: a MakerNote may also hold an embedded preview image.
        Category::MakerNote => StripSpec::of(&[
            Category::Gps,
            Category::DeviceId,
            Category::CameraModel,
            Category::Timestamps,
            Category::Software,
            Category::EmbeddedImage,
        ]),
        Category::Xmp => StripSpec::of(&TAGS_AND_SOFTWARE),
        Category::Iptc => StripSpec::of(&[
            Category::Authorship,
            Category::FreeText,
            Category::Timestamps,
        ]),
        // A C2PA manifest carries assertions, including Exif ones.
        Category::Jumbf => StripSpec::of(&TAGS_AND_SOFTWARE),
        // DEC-008: an ICC profile is "a fingerprint at most".
        Category::Icc => StripSpec::none(),
        // DEC-012: an embedded image can hold anything the primary can; an
        // unrecognised segment or trailer can hold anything. Each may hold the
        // other kind, and neither holds itself.
        Category::EmbeddedImage => StripSpec::all().exclude(Category::EmbeddedImage),
        Category::OtherSegment => StripSpec::all().exclude(Category::OtherSegment),
        // Tag categories hold nothing the library cannot see.
        Category::Gps
        | Category::DeviceId
        | Category::Authorship
        | Category::FreeText
        | Category::Timestamps
        | Category::CameraModel
        | Category::Software => StripSpec::none(),
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
    use Category::*;

    /// DEC-009's preset table, typed by hand: for each category (DEC-009 row
    /// order) the cells `[Gps, GpsAndDevice, AllPii, All, Everything]`, `true`
    /// meaning "removed". Not generated from `StripPolicy::spec`.
    const REMOVED: [(Category, [bool; 5]); 15] = [
        (Gps, [true, true, true, true, true]),
        (DeviceId, [false, true, true, true, true]),
        (Authorship, [false, false, true, true, true]),
        (FreeText, [false, false, true, true, true]),
        (Timestamps, [false, false, true, true, true]),
        (CameraModel, [false, false, true, true, true]),
        (Thumbnail, [false, false, true, true, true]),
        (MakerNote, [false, false, true, true, true]),
        (Software, [false, false, false, true, true]),
        (Xmp, [false, false, false, true, true]),
        (Iptc, [false, false, false, true, true]),
        (Icc, [false, false, false, false, true]),
        (Jumbf, [false, false, false, false, true]),
        // DEC-012 amendment: removed only by Everything.
        (EmbeddedImage, [false, false, false, false, true]),
        (OtherSegment, [false, false, false, false, true]),
    ];

    /// DEC-009's may-hold table as amended by DEC-012, typed by hand: the eight
    /// block rows.
    fn may_hold_table() -> Vec<(Category, Vec<Category>)> {
        vec![
            (
                Thumbnail,
                vec![
                    Gps,
                    DeviceId,
                    Authorship,
                    FreeText,
                    Timestamps,
                    CameraModel,
                    Software,
                ],
            ),
            (
                MakerNote,
                vec![
                    Gps,
                    DeviceId,
                    CameraModel,
                    Timestamps,
                    Software,
                    EmbeddedImage,
                ],
            ),
            (
                Xmp,
                vec![
                    Gps,
                    DeviceId,
                    Authorship,
                    FreeText,
                    Timestamps,
                    CameraModel,
                    Software,
                ],
            ),
            (Iptc, vec![Authorship, FreeText, Timestamps]),
            (
                Jumbf,
                vec![
                    Gps,
                    DeviceId,
                    Authorship,
                    FreeText,
                    Timestamps,
                    CameraModel,
                    Software,
                ],
            ),
            (Icc, vec![]),
            // DEC-012: every category except itself.
            (
                EmbeddedImage,
                vec![
                    Gps,
                    DeviceId,
                    Authorship,
                    FreeText,
                    Thumbnail,
                    MakerNote,
                    Xmp,
                    Iptc,
                    Jumbf,
                    Timestamps,
                    CameraModel,
                    Software,
                    Icc,
                    OtherSegment,
                ],
            ),
            (
                OtherSegment,
                vec![
                    Gps,
                    DeviceId,
                    Authorship,
                    FreeText,
                    Thumbnail,
                    MakerNote,
                    Xmp,
                    Iptc,
                    Jumbf,
                    Timestamps,
                    CameraModel,
                    Software,
                    Icc,
                    EmbeddedImage,
                ],
            ),
        ]
    }

    #[test]
    fn strip_spec_has_one_distinct_bit_per_category() {
        // Build each singleton and check it holds exactly that category.
        let mut seen: Vec<StripSpec> = Vec::new();
        for c in Category::ALL {
            let s = StripSpec::none().include(c);
            assert_eq!(s.len(), 1, "{c:?}");
            assert!(s.contains(c), "{c:?}");
            for other in Category::ALL {
                assert_eq!(s.contains(other), other == c, "{c:?} vs {other:?}");
            }
            assert!(!seen.contains(&s), "{c:?} shares a bit");
            seen.push(s);
        }
        assert_eq!(seen.len(), 15, "15 distinct bits");
        // All the singletons together are all().
        let union = seen.iter().fold(StripSpec::none(), |a, s| {
            s.categories().fold(a, |a, c| a.include(c))
        });
        assert_eq!(union, StripSpec::all());
    }

    #[test]
    fn bit_set_uses_fifteen_of_sixteen_bits() {
        assert!(Category::ALL.len() <= 16);
        assert_eq!(Category::ALL.len(), 15);
        // The 16th bit is spare: no spec built from categories reaches it.
        assert_eq!(StripSpec::all().0, 0x7FFF);
    }

    #[test]
    fn strip_spec_none_all_include_exclude_contains_len() {
        let none = StripSpec::none();
        assert!(none.is_empty());
        assert_eq!(none.len(), 0);
        assert_eq!(none, StripSpec::default());
        for c in Category::ALL {
            assert!(!none.contains(c));
        }
        let all = StripSpec::all();
        assert!(!all.is_empty());
        assert_eq!(all.len(), 15);
        for c in Category::ALL {
            assert!(all.contains(c));
        }
        let s = none.include(Gps).include(Xmp);
        assert_eq!(s.len(), 2);
        assert!(s.contains(Gps) && s.contains(Xmp) && !s.contains(Icc));
        // Including twice changes nothing.
        assert_eq!(s.include(Gps), s);
        // Exclude removes exactly that category and is not a no-op.
        let t = s.exclude(Gps);
        assert!(!t.contains(Gps) && t.contains(Xmp));
        assert_eq!(t.len(), 1);
        assert_eq!(t.exclude(Gps), t);
        assert_eq!(t.exclude(Xmp), none);
        assert_eq!(all.exclude(Jumbf).len(), 14);
        assert!(!all.exclude(Jumbf).contains(Jumbf));
        assert!(all.exclude(Jumbf).contains(Icc));
    }

    #[test]
    fn strip_spec_categories_iterate_in_category_all_order() {
        let s = StripSpec::none()
            .include(Icc)
            .include(Gps)
            .include(Xmp)
            .include(Timestamps);
        let got: Vec<Category> = s.categories().collect();
        assert_eq!(got, vec![Gps, Xmp, Timestamps, Icc]);
        assert_eq!(
            StripSpec::all().categories().collect::<Vec<_>>(),
            Category::ALL.to_vec()
        );
        assert_eq!(StripSpec::none().categories().count(), 0);
    }

    #[test]
    fn strip_spec_subset_and_intersects() {
        let a = StripSpec::none().include(Gps).include(DeviceId);
        let b = a.include(Xmp);
        let c = StripSpec::none().include(Icc);
        assert!(a.is_subset_of(b));
        assert!(!b.is_subset_of(a));
        assert!(a.is_subset_of(a));
        assert!(StripSpec::none().is_subset_of(a));
        assert!(StripSpec::none().is_subset_of(StripSpec::none()));
        assert!(b.is_subset_of(StripSpec::all()));
        assert!(!StripSpec::all().is_subset_of(b));
        assert!(a.intersects(b));
        assert!(b.intersects(a));
        assert!(!a.intersects(c));
        assert!(!c.intersects(a));
        assert!(!a.intersects(StripSpec::none()));
        assert!(!StripSpec::none().intersects(StripSpec::none()));
        assert!(a.intersects(StripSpec::all()));
        // Partial overlap: intersects, neither is a subset of the other.
        let d = StripSpec::none().include(DeviceId).include(Icc);
        assert!(a.intersects(d));
        assert!(!a.is_subset_of(d) && !d.is_subset_of(a));
    }

    #[test]
    fn each_preset_spec_equals_its_dec_009_column_as_amended() {
        for (col, preset) in StripPolicy::PRESETS.iter().enumerate() {
            let spec = preset.spec();
            for (c, cells) in REMOVED {
                assert_eq!(
                    spec.contains(c),
                    cells[col],
                    "{} / {c:?}: DEC-009 says removed = {}",
                    preset.name(),
                    cells[col]
                );
            }
        }
    }

    #[test]
    fn presets_are_nested_and_everything_is_every_category() {
        for pair in StripPolicy::PRESETS.windows(2) {
            let (small, big) = (pair[0].spec(), pair[1].spec());
            assert!(
                small.is_subset_of(big),
                "{} in {}",
                pair[0].name(),
                pair[1].name()
            );
            assert!(small.len() < big.len(), "strictly nested");
        }
        assert_eq!(StripPolicy::Everything.spec(), StripSpec::all());
        assert_eq!(StripPolicy::Everything.spec().len(), 15);
        let lens: Vec<usize> = StripPolicy::PRESETS
            .iter()
            .map(|p| p.spec().len())
            .collect();
        assert_eq!(lens, vec![1, 2, 8, 11, 15]);
    }

    #[test]
    fn custom_spec_is_returned_unchanged() {
        for s in [
            StripSpec::none(),
            StripSpec::all(),
            StripSpec::none().include(Icc).include(Software),
        ] {
            assert_eq!(StripPolicy::Custom(s).spec(), s);
        }
    }

    #[test]
    fn may_hold_equals_the_dec_012_table() {
        let table = may_hold_table();
        assert_eq!(table.len(), 8);
        for (block, holds) in &table {
            let got = may_hold(*block);
            for c in Category::ALL {
                assert_eq!(
                    got.contains(c),
                    holds.contains(&c),
                    "may_hold({block:?}) / {c:?}"
                );
            }
        }
        // Every tag category holds nothing.
        for c in [
            Gps,
            DeviceId,
            Authorship,
            FreeText,
            Timestamps,
            CameraModel,
            Software,
        ] {
            assert!(may_hold(c).is_empty(), "{c:?}");
        }
        // The eight block rows above are exactly the block categories.
        for c in Category::ALL {
            assert_eq!(c.is_block(), table.iter().any(|(b, _)| *b == c), "{c:?}");
        }
    }

    #[test]
    fn makernote_may_hold_embedded_image() {
        assert!(may_hold(MakerNote).contains(EmbeddedImage));
        // ...and the five it held before are still there.
        for c in [Gps, DeviceId, CameraModel, Timestamps, Software] {
            assert!(may_hold(MakerNote).contains(c), "{c:?}");
        }
        assert_eq!(may_hold(MakerNote).len(), 6);
        // Neither new category holds itself.
        assert!(!may_hold(EmbeddedImage).contains(EmbeddedImage));
        assert!(!may_hold(OtherSegment).contains(OtherSegment));
    }

    #[test]
    fn policy_and_disposition_names_are_pinned() {
        let want = ["gps", "gps_and_device", "all_pii", "all", "everything"];
        for (p, n) in StripPolicy::PRESETS.iter().zip(want) {
            assert_eq!(p.name(), n);
        }
        assert_eq!(StripPolicy::Custom(StripSpec::none()).name(), "custom");
        use crate::audit::Disposition as D;
        assert_eq!(D::Removed.name(), "removed");
        assert_eq!(D::Kept.name(), "kept");
        assert_eq!(D::KeptUnread.name(), "kept_unread");
    }

    #[test]
    fn matrix_covers_every_category() {
        assert_eq!(REMOVED.len(), Category::ALL.len());
        for c in Category::ALL {
            assert_eq!(
                REMOVED.iter().filter(|(g, _)| *g == c).count(),
                1,
                "{c:?} appears exactly once in the DEC-009 grid"
            );
        }
    }
}
