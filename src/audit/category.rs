//! The DEC-008 category table, as code (SPEC-012).
//!
//! DEC-008 is the normative statement; this file is the one place the table
//! lives in code. [`categorize`] is a `match` on `(kind, tag)` with no other
//! source, and the tests below carry the table again as data (typed out by
//! hand, deliberately not derived from [`categorize`]) and compare row by row.

use crate::tiff::IfdKind;

use super::RegionKind;

/// What kind of personal data a finding is (DEC-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Category {
    /// Where the picture was taken: every tag of a GPS IFD.
    Gps,
    /// Identifies one physical device: serial numbers, `ImageUniqueID`.
    DeviceId,
    /// Names a person or rights holder.
    Authorship,
    /// Free text the author typed.
    FreeText,
    /// An embedded thumbnail.
    Thumbnail,
    /// A vendor block: present, not parsed.
    MakerNote,
    /// An XMP packet or extended-XMP part (not decoded).
    Xmp,
    /// An IPTC / Photoshop-resource block (not decoded).
    Iptc,
    /// A JUMBF box (not decoded, not identified as C2PA).
    Jumbf,
    /// When the picture was taken or edited, and timezone offsets.
    Timestamps,
    /// Make, model, lens make and model.
    CameraModel,
    /// What edited or produced the file.
    Software,
    /// An embedded colour profile.
    Icc,
}

/// How much a finding matters (DEC-008). `Low < Medium < High`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Fingerprints, does not locate or identify.
    Low,
    /// Can name a person or hold anything.
    Medium,
    /// Locates or identifies a device.
    High,
}

impl Category {
    /// Every category, in DEC-008 table order.
    pub const ALL: [Category; 13] = [
        Category::Gps,
        Category::DeviceId,
        Category::Authorship,
        Category::FreeText,
        Category::Thumbnail,
        Category::MakerNote,
        Category::Xmp,
        Category::Iptc,
        Category::Jumbf,
        Category::Timestamps,
        Category::CameraModel,
        Category::Software,
        Category::Icc,
    ];

    /// The category's severity (DEC-008).
    pub fn severity(self) -> Severity {
        match self {
            Category::Gps | Category::DeviceId => Severity::High,
            Category::Authorship
            | Category::FreeText
            | Category::Thumbnail
            | Category::MakerNote
            | Category::Xmp
            | Category::Iptc
            | Category::Jumbf => Severity::Medium,
            Category::Timestamps | Category::CameraModel | Category::Software | Category::Icc => {
                Severity::Low
            }
        }
    }

    /// Whether the category is a block the library locates but does not decode
    /// (DEC-009): `Thumbnail`, `MakerNote`, `Xmp`, `Iptc`, `Jumbf`, `Icc`.
    pub fn is_block(self) -> bool {
        matches!(
            self,
            Category::Thumbnail
                | Category::MakerNote
                | Category::Xmp
                | Category::Iptc
                | Category::Jumbf
                | Category::Icc
        )
    }

    /// A stable lowercase identifier (`snake_case`) for reports.
    pub fn name(self) -> &'static str {
        match self {
            Category::Gps => "gps",
            Category::DeviceId => "device_id",
            Category::Authorship => "authorship",
            Category::FreeText => "free_text",
            Category::Thumbnail => "thumbnail",
            Category::MakerNote => "maker_note",
            Category::Xmp => "xmp",
            Category::Iptc => "iptc",
            Category::Jumbf => "jumbf",
            Category::Timestamps => "timestamps",
            Category::CameraModel => "camera_model",
            Category::Software => "software",
            Category::Icc => "icc",
        }
    }
}

impl Severity {
    /// A stable lowercase name for reports.
    pub fn name(self) -> &'static str {
        match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
        }
    }
}

/// The DEC-008 tag table (tag id to category), for the IFD kinds it covers.
///
/// An IFD of kind [`IfdKind::Gps`] is all [`Category::Gps`] whatever the id;
/// an [`IfdKind::Interop`] IFD yields nothing (DEC-008: the table is for
/// "non-GPS, non-Interop IFDs"). `0x0201` is a thumbnail only in an
/// [`IfdKind::Ifd1`]. The IFD1-with-strips rule (`0x0111` when `0x0201` is
/// absent) needs the whole IFD and is applied by [`super::audit`], not here.
/// `None` means inventory only.
pub fn categorize(kind: IfdKind, tag: u16) -> Option<Category> {
    match kind {
        IfdKind::Gps => return Some(Category::Gps),
        IfdKind::Interop => return None,
        IfdKind::Ifd1 if tag == 0x0201 => return Some(Category::Thumbnail),
        _ => {}
    }
    by_id(tag)
}

/// The id-decides rows of the table.
fn by_id(tag: u16) -> Option<Category> {
    match tag {
        // FreeText: ImageDescription, UserComment, XPTitle/XPComment/XPKeywords/
        // XPSubject, ImageTitle.
        0x010E | 0x9286 | 0x9C9B | 0x9C9C | 0x9C9E | 0x9C9F | 0xA436 => Some(Category::FreeText),
        // CameraModel: Make, Model, LensMake, LensModel.
        0x010F | 0x0110 | 0xA433 | 0xA434 => Some(Category::CameraModel),
        // Software: Software, CameraFirmware, RAWDevelopingSoftware,
        // ImageEditingSoftware, MetadataEditingSoftware.
        0x0131 | 0xA439..=0xA43C => Some(Category::Software),
        // Timestamps: DateTime, DateTimeOriginal, DateTimeDigitized, OffsetTime*,
        // SubSecTime*.
        0x0132 | 0x9003 | 0x9004 | 0x9010 | 0x9011 | 0x9012 | 0x9290 | 0x9291 | 0x9292 => {
            Some(Category::Timestamps)
        }
        // Authorship: Artist, Copyright, CameraOwnerName, XPAuthor, ImageEditor.
        0x013B | 0x8298 | 0xA430 | 0x9C9D | 0xA437 | 0xA438 => Some(Category::Authorship),
        0x927C => Some(Category::MakerNote),
        // DeviceId: ImageUniqueID, BodySerialNumber, LensSerialNumber,
        // CameraSerialNumber (DNG).
        0xA420 | 0xA431 | 0xA435 | 0xC62F => Some(Category::DeviceId),
        0x02BC => Some(Category::Xmp),
        // Iptc: IPTC-NAA, Photoshop resources.
        0x83BB | 0x8649 => Some(Category::Iptc),
        0x8773 => Some(Category::Icc),
        _ => None,
    }
}

/// The category of a region the JPEG layer located (DEC-008).
pub fn region_category(kind: RegionKind) -> Category {
    match kind {
        RegionKind::Xmp | RegionKind::ExtendedXmp => Category::Xmp,
        RegionKind::Iptc => Category::Iptc,
        RegionKind::Icc => Category::Icc,
        RegionKind::Jumbf => Category::Jumbf,
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

    /// DEC-008's tag table, typed out again as data: `(id, tag name, category)`.
    /// Not generated from `categorize`: a wrong row in the code fails here.
    fn dec_008_table() -> Vec<(u16, &'static str, Category)> {
        use Category::*;
        vec![
            (0x010E, "ImageDescription", FreeText),
            (0x010F, "Make", CameraModel),
            (0x0110, "Model", CameraModel),
            (0x0131, "Software", Software),
            (0x0132, "DateTime", Timestamps),
            (0x013B, "Artist", Authorship),
            (0x8298, "Copyright", Authorship),
            (0x9003, "DateTimeOriginal", Timestamps),
            (0x9004, "DateTimeDigitized", Timestamps),
            (0x9010, "OffsetTime", Timestamps),
            (0x9011, "OffsetTimeOriginal", Timestamps),
            (0x9012, "OffsetTimeDigitized", Timestamps),
            (0x9290, "SubSecTime", Timestamps),
            (0x9291, "SubSecTimeOriginal", Timestamps),
            (0x9292, "SubSecTimeDigitized", Timestamps),
            (0x9286, "UserComment", FreeText),
            (0x927C, "MakerNote", MakerNote),
            (0xA420, "ImageUniqueID", DeviceId),
            (0xA430, "CameraOwnerName", Authorship),
            (0xA431, "BodySerialNumber", DeviceId),
            (0xA435, "LensSerialNumber", DeviceId),
            (0xA433, "LensMake", CameraModel),
            (0xA434, "LensModel", CameraModel),
            (0xC62F, "CameraSerialNumber", DeviceId),
            (0x9C9B, "XPTitle", FreeText),
            (0x9C9C, "XPComment", FreeText),
            (0x9C9E, "XPKeywords", FreeText),
            (0x9C9F, "XPSubject", FreeText),
            (0x9C9D, "XPAuthor", Authorship),
            (0xA436, "ImageTitle", FreeText),
            (0xA437, "Photographer", Authorship),
            (0xA438, "ImageEditor", Authorship),
            (0xA439, "CameraFirmware", Software),
            (0xA43A, "RAWDevelopingSoftware", Software),
            (0xA43B, "ImageEditingSoftware", Software),
            (0xA43C, "MetadataEditingSoftware", Software),
            (0x02BC, "XMP", Xmp),
            (0x83BB, "IPTC-NAA", Iptc),
            (0x8649, "Photoshop", Iptc),
            (0x8773, "InterColorProfile", Icc),
        ]
    }

    const NON_GPS_KINDS: [IfdKind; 5] = [
        IfdKind::Ifd0,
        IfdKind::Ifd1,
        IfdKind::Chain,
        IfdKind::Sub,
        IfdKind::Exif,
    ];

    #[test]
    fn categorize_matches_the_dec_008_table_row_for_row() {
        let table = dec_008_table();
        assert_eq!(table.len(), 40, "40 ids in the DEC-008 tag table");
        for (id, name, want) in &table {
            // The id decides: the same category in every IFD kind the table covers.
            for kind in NON_GPS_KINDS {
                assert_eq!(
                    categorize(kind, *id),
                    Some(*want),
                    "{name} (0x{id:04X}) in {kind:?}"
                );
            }
            // Interop is outside the table (DEC-008: "non-GPS, non-Interop IFDs").
            assert_eq!(categorize(IfdKind::Interop, *id), None, "{name} in Interop");
            // A GPS IFD is Gps whatever the id.
            assert_eq!(categorize(IfdKind::Gps, *id), Some(Category::Gps));
        }
        // 0x0201 is a thumbnail only in an Ifd1.
        assert_eq!(categorize(IfdKind::Ifd1, 0x0201), Some(Category::Thumbnail));
        for kind in [
            IfdKind::Ifd0,
            IfdKind::Chain,
            IfdKind::Sub,
            IfdKind::Exif,
            IfdKind::Interop,
        ] {
            assert_eq!(categorize(kind, 0x0201), None, "0x0201 in {kind:?}");
        }
        // The strips rule needs the whole IFD: the per-tag function never says it.
        assert_eq!(categorize(IfdKind::Ifd1, 0x0111), None);
    }

    #[test]
    fn categorize_assigns_nothing_outside_the_table() {
        let ids: Vec<u16> = dec_008_table().iter().map(|r| r.0).collect();
        for kind in NON_GPS_KINDS.iter().copied().chain([IfdKind::Interop]) {
            for tag in 0..=u16::MAX {
                let in_table = ids.contains(&tag) && kind != IfdKind::Interop;
                let thumb = kind == IfdKind::Ifd1 && tag == 0x0201;
                assert_eq!(
                    categorize(kind, tag).is_some(),
                    in_table || thumb,
                    "{kind:?} 0x{tag:04X}"
                );
            }
        }
        // The pointer tags are structure, never findings.
        for tag in [0x8769, 0x8825, 0xA005, 0x014A] {
            for kind in NON_GPS_KINDS {
                assert_eq!(categorize(kind, tag), None, "pointer 0x{tag:04X}");
            }
        }
    }

    #[test]
    fn every_gps_ifd_tag_is_a_gps_finding_whatever_its_id() {
        for tag in 0..=u16::MAX {
            assert_eq!(categorize(IfdKind::Gps, tag), Some(Category::Gps), "{tag}");
        }
    }

    #[test]
    fn the_same_ids_outside_a_gps_ifd_are_not_gps() {
        // GPS tag numbers 0..=31 are ordinary ids elsewhere: none is Gps.
        for kind in NON_GPS_KINDS.iter().copied().chain([IfdKind::Interop]) {
            for tag in 0..=u16::MAX {
                assert_ne!(categorize(kind, tag), Some(Category::Gps), "{kind:?} {tag}");
            }
        }
    }

    #[test]
    fn category_severity_matches_dec_008() {
        use Category::*;
        use Severity::*;
        let want = [
            (Gps, High),
            (DeviceId, High),
            (Authorship, Medium),
            (FreeText, Medium),
            (Thumbnail, Medium),
            (MakerNote, Medium),
            (Xmp, Medium),
            (Iptc, Medium),
            (Jumbf, Medium),
            (Timestamps, Low),
            (CameraModel, Low),
            (Software, Low),
            (Icc, Low),
        ];
        assert_eq!(want.len(), Category::ALL.len());
        for (c, s) in want {
            assert_eq!(c.severity(), s, "{c:?}");
            assert!(Category::ALL.contains(&c));
        }
        // Names are distinct and non-empty.
        let mut names: Vec<&str> = Category::ALL.iter().map(|c| c.name()).collect();
        assert!(names.iter().all(|n| !n.is_empty()));
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Category::ALL.len());
    }

    #[test]
    fn is_block_is_exactly_the_six_block_categories() {
        use Category::*;
        let blocks = [Thumbnail, MakerNote, Xmp, Iptc, Jumbf, Icc];
        let tags = [
            Gps,
            DeviceId,
            Authorship,
            FreeText,
            Timestamps,
            CameraModel,
            Software,
        ];
        assert_eq!(blocks.len() + tags.len(), Category::ALL.len());
        for c in blocks {
            assert!(c.is_block(), "{c:?}");
        }
        for c in tags {
            assert!(!c.is_block(), "{c:?}");
        }
        for c in Category::ALL {
            assert!(blocks.contains(&c) != tags.contains(&c), "{c:?} in one list");
        }
    }

    #[test]
    fn severity_orders_low_medium_high() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::Low < Severity::High);
        assert_eq!(Severity::Low.max(Severity::High), Severity::High);
        assert_eq!(Severity::Low.name(), "low");
        assert_eq!(Severity::Medium.name(), "medium");
        assert_eq!(Severity::High.name(), "high");
    }

    #[test]
    fn the_exif_3_ids_a436_to_a43c_are_as_amended_in_dec_008() {
        let want = [
            (0xA436, Some(Category::FreeText)),
            (0xA437, Some(Category::Authorship)),
            (0xA438, Some(Category::Authorship)),
            (0xA439, Some(Category::Software)),
            (0xA43A, Some(Category::Software)),
            (0xA43B, Some(Category::Software)),
            (0xA43C, Some(Category::Software)),
            (0xA43D, None),
        ];
        for (id, cat) in want {
            for kind in NON_GPS_KINDS {
                assert_eq!(categorize(kind, id), cat, "0x{id:04X} in {kind:?}");
            }
        }
    }

    #[test]
    fn region_kinds_map_to_the_dec_008_categories() {
        assert_eq!(region_category(RegionKind::Xmp), Category::Xmp);
        assert_eq!(region_category(RegionKind::ExtendedXmp), Category::Xmp);
        assert_eq!(region_category(RegionKind::Iptc), Category::Iptc);
        assert_eq!(region_category(RegionKind::Icc), Category::Icc);
        assert_eq!(region_category(RegionKind::Jumbf), Category::Jumbf);
    }
}
