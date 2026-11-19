//! SPEC-022: the TIFF and DNG in-place strip against the independent fixtures
//! in `tests/support/tiff_shapes.rs`. Every shape is run under every policy;
//! the expectations come from the builder's record (what each byte range is,
//! which canary sits in which carrier), never from the library.

#[path = "support/tiff_shapes.rs"]
mod tiff_shapes;

use exif_core::audit::{audit, Category, ContainerKind, Sniffed};
use exif_core::policy::{StripPolicy, StripSpec};
use exif_core::tiff::Container;
use exif_core::write::UneditableReason;
use exif_core::{sanitize, strip, StripError};
use tiff_shapes::{all, hostile, Shape, K};

fn spec_of(cats: &[Category]) -> StripSpec {
    cats.iter().fold(StripSpec::none(), |s, c| s.include(*c))
}

fn policies() -> Vec<(&'static str, StripPolicy)> {
    vec![
        ("gps", StripPolicy::Gps),
        ("gps_and_device", StripPolicy::GpsAndDevice),
        ("all_pii", StripPolicy::AllPii),
        ("all", StripPolicy::All),
        ("everything", StripPolicy::Everything),
        ("none", StripPolicy::Custom(StripSpec::none())),
        ("xmp", StripPolicy::Custom(spec_of(&[Category::Xmp]))),
        ("icc", StripPolicy::Custom(spec_of(&[Category::Icc]))),
        ("jumbf", StripPolicy::Custom(spec_of(&[Category::Jumbf]))),
        (
            "free_text",
            StripPolicy::Custom(spec_of(&[Category::FreeText])),
        ),
        (
            "thumbnail",
            StripPolicy::Custom(spec_of(&[Category::Thumbnail])),
        ),
        (
            "iptc+software",
            StripPolicy::Custom(spec_of(&[Category::Iptc, Category::Software])),
        ),
    ]
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// The policy's expected result for a shape. SPEC-023: no shape in `all()` is
/// refused any more (a bare TIFF's page IFD1 is kept, not a thumbnail); the one
/// refused shape, `page_and_jpeg`, has its own test.
fn expect_refusal(_s: &Shape, _spec: StripSpec) -> bool {
    false
}

fn offsets_of(bytes: &[u8]) -> Vec<(u16, Vec<u32>)> {
    let c = Container::parse(bytes).unwrap();
    let mut out = Vec::new();
    for ifd in c.ifds() {
        for e in ifd.entries() {
            if matches!(e.tag(), 0x0111 | 0x0144) {
                out.push((e.tag(), c.uints(e).unwrap()));
            }
        }
    }
    out
}

#[test]
fn every_shape_under_every_policy_keeps_the_image_data_and_the_length() {
    for s in all() {
        for (pname, policy) in policies() {
            let spec = policy.spec();
            let res = strip(&s.bytes, policy);
            let ctx = format!("{} under {pname}", s.name);
            if expect_refusal(&s, spec) {
                assert_eq!(
                    res.unwrap_err(),
                    StripError::Uneditable {
                        reason: UneditableReason::ThumbnailNotLocated,
                        offset: res_offset(&s)
                    },
                    "{ctx}"
                );
                continue;
            }
            let out = res.unwrap_or_else(|e| panic!("{ctx}: {e:?}")).into_bytes();
            // Same length; the header is the same bytes.
            assert_eq!(out.len(), s.bytes.len(), "{ctx}: length");
            // Nothing moved or changed in the image data.
            for p in s.parts.iter().filter(|p| p.kind == K::Image) {
                assert_eq!(
                    &out[p.range.clone()],
                    &s.bytes[p.range.clone()],
                    "{ctx}: image data {:?}",
                    p.range
                );
            }
            for p in s.parts.iter().filter(|p| p.kind == K::Header) {
                assert_eq!(&out[p.range.clone()], &s.bytes[p.range.clone()], "{ctx}");
            }
            // Strip and tile offsets read the same.
            assert_eq!(offsets_of(&out), s.offsets, "{ctx}: offsets");
            // Canaries: gone iff their category is removed.
            for (canary, cat) in &s.canaries {
                assert_eq!(
                    contains(&out, canary),
                    !spec.contains(*cat),
                    "{ctx}: canary {} ({cat:?})",
                    String::from_utf8_lossy(canary)
                );
            }
            // The audit of the output has nothing of a removed category.
            let a = audit(&out);
            assert!(
                a.findings().iter().all(|f| !spec.contains(f.category)),
                "{ctx}: findings left"
            );
            // A second strip is a no-op.
            assert_eq!(sanitize(&out, policy).unwrap(), out, "{ctx}: idempotent");
        }
    }
}

/// The offset a `ThumbnailNotLocated` refusal reports: IFD1's table.
fn res_offset(s: &Shape) -> usize {
    let tables: Vec<_> = s.parts.iter().filter(|p| p.kind == K::Table).collect();
    tables.get(1).map_or(0, |p| p.range.start)
}

#[test]
fn an_empty_policy_returns_the_input() {
    for s in all() {
        let out = sanitize(&s.bytes, StripPolicy::Custom(StripSpec::none())).unwrap();
        assert_eq!(out, s.bytes, "{}", s.name);
    }
}

#[test]
fn a_gps_policy_removes_the_gps_ifd_and_its_pointer_and_nothing_else() {
    let s = tiff_shapes::stripped(false);
    let r = strip(&s.bytes, StripPolicy::Gps).unwrap();
    assert!(r.entries().iter().any(|e| e.tag == 0x8825));
    assert!(r.entries().len() >= 4, "pointer plus the three GPS entries");
    assert!(!contains(r.bytes(), tiff_shapes::C_GPS));
    // Everything non-GPS is still there.
    for (c, cat) in &s.canaries {
        if *cat != Category::Gps {
            assert!(contains(r.bytes(), c));
        }
    }
}

#[test]
fn the_dng_subifd_gps_and_artist_are_removed_and_the_raw_data_kept() {
    for be in [false, true] {
        let s = tiff_shapes::dng(be);
        let out = sanitize(&s.bytes, StripPolicy::AllPii).unwrap();
        assert!(!contains(&out, tiff_shapes::C_SUB_GPS));
        assert!(!contains(&out, tiff_shapes::C_SUB_ARTIST));
        assert!(!contains(&out, tiff_shapes::C_DNG_SERIAL));
        // DNGVersion, the colour matrix and the three raw tiles are structural.
        let c = Container::parse(&out).unwrap();
        for tag in [0xC612u16, 0xC621, 0x014A, 0x0144] {
            assert!(
                c.ifds().iter().any(|i| i.has(tag)),
                "tag {tag:#x} kept ({be})"
            );
        }
        let raw_tiles = s.parts.iter().filter(|p| p.kind == K::Image).count();
        assert_eq!(raw_tiles, 1 + 3 + 1);
    }
}

#[test]
fn a_page_ifd1_is_never_removed_and_a_jpeg_thumbnail_is() {
    let page = tiff_shapes::multipage(false);
    // Policies without Thumbnail keep every page and delete only entries.
    let out = sanitize(&page.bytes, StripPolicy::GpsAndDevice).unwrap();
    assert_eq!(offsets_of(&out), page.offsets);
    // Software lives in the IFD1 page and goes under a Software policy.
    let sw = StripPolicy::Custom(spec_of(&[Category::Software]));
    let out = sanitize(&page.bytes, sw).unwrap();
    assert!(!contains(&out, tiff_shapes::C_SOFTWARE1));
    assert_eq!(offsets_of(&out), page.offsets);
    // SPEC-023: a policy with Thumbnail keeps the page (it used to be refused).
    let out = sanitize(&page.bytes, StripPolicy::AllPii).unwrap();
    assert_eq!(offsets_of(&out), page.offsets);
    // A JPEG thumbnail goes: the bytes are zero, the IFD is cut off the chain.
    let t = tiff_shapes::thumb(false);
    let r = strip(&t.bytes, StripPolicy::AllPii).unwrap();
    let range = t
        .parts
        .iter()
        .find(|p| p.kind == K::Thumb)
        .unwrap()
        .range
        .clone();
    assert!(r.bytes()[range].iter().all(|b| *b == 0));
    let c = Container::parse(r.bytes()).unwrap();
    assert_eq!(c.ifds().iter().filter(|i| i.has(0x0201)).count(), 0);
}

#[test]
fn hostile_inputs_are_refused_typed_and_never_modified() {
    for (name, bytes) in hostile() {
        let r = strip(&bytes, StripPolicy::Gps);
        match name {
            "strip-overlaps-gps-value" | "strip-offset-into-ifd-table" => assert!(
                matches!(
                    r,
                    Err(StripError::Uneditable {
                        reason: UneditableReason::SharedBytes,
                        ..
                    })
                ),
                "{name}: {r:?}"
            ),
            _ => assert!(
                matches!(r, Err(StripError::Uneditable { .. })),
                "{name}: {r:?}"
            ),
        }
        // Under a policy that touches nothing, the self-check still passes or
        // refuses; it never panics and returns no changed length.
        if let Ok(ok) = strip(&bytes, StripPolicy::Custom(StripSpec::none())) {
            assert_eq!(ok.bytes(), &bytes[..], "{name}");
        }
    }
}

#[test]
fn bigtiff_is_still_unsupported() {
    let big = b"II\x2B\x00\x08\x00\x00\x00\x10\x00\x00\x00\x00\x00\x00\x00".to_vec();
    assert_eq!(
        strip(&big, StripPolicy::All).unwrap_err(),
        StripError::Unsupported {
            kind: ContainerKind::Unsupported(Sniffed::BigTiff)
        }
    );
}

#[test]
fn the_jpeg_path_is_unchanged() {
    let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xFE, 0x00, 0x06, b'h', b'i', b'!', b'!'];
    jpeg.extend_from_slice(&[
        0xFF, 0xDA, 0x00, 0x08, 1, 1, 0, 0, 0x3F, 0, 1, 2, 0xFF, 0xD9,
    ]);
    let out = sanitize(&jpeg, StripPolicy::All).unwrap();
    assert!(out.len() < jpeg.len());
    assert!(!contains(&out, b"hi!!"));
}

/// Found by `rewrite_tiff` (first 60 s run): an offsets entry of a type that
/// holds no integers made the self-check fail on a file the planner had
/// accepted. The planner now refuses it, typed, even when it edits nothing.
#[test]
fn an_unreadable_offsets_entry_is_a_typed_refusal_even_with_no_edit() {
    let t = tiff_shapes::tiled(false);
    let mut bytes = t.bytes.clone();
    let ifd0 = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let n = usize::from(u16::from_le_bytes([bytes[ifd0], bytes[ifd0 + 1]]));
    let e = (0..n)
        .map(|i| ifd0 + 2 + i * 12)
        .find(|e| bytes[*e] == 0x44 && bytes[*e + 1] == 0x01)
        .unwrap();
    bytes[e + 2] = 2; // ASCII
    for policy in [
        StripPolicy::Custom(StripSpec::none()),
        StripPolicy::Custom(spec_of(&[Category::Jumbf])),
        StripPolicy::AllPii,
    ] {
        assert!(
            matches!(
                strip(&bytes, policy),
                Err(StripError::Uneditable {
                    reason: UneditableReason::PayloadUnreadable,
                    ..
                })
            ),
            "{policy:?}"
        );
    }
}

/// A policy that removes anything must meet the hidden-pointer guard even when
/// no category it names can occur in an Exif entry: `Custom(Jumbf)` on a TIFF
/// holding a 0xCD41 entry used to return the input unchanged and report success.
#[test]
fn a_custom_jumbf_policy_on_a_tiff_with_a_jumbf_entry_is_refused() {
    for be in [false, true] {
        let s = tiff_shapes::jumbf(be);
        for policy in [
            StripPolicy::Custom(spec_of(&[Category::Jumbf])),
            StripPolicy::Everything,
        ] {
            let r = strip(&s.bytes, policy);
            assert!(
                matches!(
                    r,
                    Err(StripError::Uneditable {
                        reason: UneditableReason::HiddenPointer,
                        ..
                    })
                ),
                "be={be}: {r:?}"
            );
        }
        // A policy that removes nothing is still the identity.
        let r = strip(&s.bytes, StripPolicy::Custom(StripSpec::none())).unwrap();
        assert_eq!(r.bytes(), &s.bytes[..]);
    }
}

/// SPEC-023 replaces the SPEC-022 refusal of a multi-page TIFF under a
/// Thumbnail policy: pages are kept byte-identical, entries go by category.
#[test]
fn multipage_and_tiled_page_tiffs_strip_under_all_pii_all_everything() {
    for be in [false, true] {
        for s in [tiff_shapes::multipage(be), tiff_shapes::tiled_page(be)] {
            for p in [
                StripPolicy::AllPii,
                StripPolicy::All,
                StripPolicy::Everything,
            ] {
                let ctx = format!("{} under {p:?}", s.name);
                let out = strip(&s.bytes, p)
                    .unwrap_or_else(|e| panic!("{ctx}: {e:?}"))
                    .into_bytes();
                for part in s.parts.iter().filter(|x| x.kind == K::Image) {
                    assert_eq!(
                        &out[part.range.clone()],
                        &s.bytes[part.range.clone()],
                        "{ctx}"
                    );
                }
                assert_eq!(offsets_of(&out), s.offsets, "{ctx}");
                // Software is in All and Everything only (AllPii keeps it).
                assert_eq!(
                    contains(&out, tiff_shapes::C_SOFTWARE1),
                    p == StripPolicy::AllPii,
                    "{ctx}: page Software"
                );
                assert!(!contains(&out, tiff_shapes::C_MAKE), "{ctx}: Make");
                assert_eq!(sanitize(&out, p).unwrap(), out, "{ctx}: idempotent");
            }
        }
    }
}

#[test]
fn tiled_page_ifd1_strips() {
    // The tiled IFD1 page was refused before SPEC-023.
    let s = tiff_shapes::tiled_page(false);
    assert!(strip(&s.bytes, StripPolicy::Everything).is_ok());
}

#[test]
fn ifd1_with_jpeg_pair_and_strips_is_still_refused() {
    for be in [false, true] {
        let s = tiff_shapes::page_and_jpeg(be);
        for p in [
            StripPolicy::AllPii,
            StripPolicy::All,
            StripPolicy::Everything,
        ] {
            assert!(
                matches!(
                    strip(&s.bytes, p),
                    Err(StripError::Uneditable {
                        reason: UneditableReason::ThumbnailNotLocated,
                        ..
                    })
                ),
                "{} {p:?}",
                s.name
            );
        }
        // Without Thumbnail it is kept untouched where nothing applies.
        assert!(strip(&s.bytes, StripPolicy::Gps).is_ok());
    }
}

#[test]
fn dng_identity_tags_go_under_everything_and_all_pii_stay_under_gps_and_device() {
    let tags = [0xC614u16, 0xC615, 0xC634, 0xC68B];
    let canaries: [&[u8]; 7] = [
        tiff_shapes::C_UCM,
        tiff_shapes::C_LCM,
        tiff_shapes::C_ORF,
        tiff_shapes::C_DPD,
        tiff_shapes::C_SUB_UCM,
        tiff_shapes::C_SUB_ORF,
        tiff_shapes::C_SUB_DPD,
    ];
    for be in [false, true] {
        let s = tiff_shapes::dng_identity(be);
        let has = |bytes: &[u8], tag: u16| {
            Container::parse(bytes)
                .unwrap()
                .ifds()
                .iter()
                .any(|i| i.has(tag))
        };
        for tag in tags {
            assert!(has(&s.bytes, tag), "fixture has {tag:#x}");
        }
        for p in [StripPolicy::AllPii, StripPolicy::Everything] {
            let out = strip(&s.bytes, p).unwrap().into_bytes();
            for tag in tags {
                assert!(!has(&out, tag), "{} {p:?}: {tag:#x} left", s.name);
            }
            for c in canaries {
                assert!(
                    !contains(&out, c),
                    "{} {p:?}: {}",
                    s.name,
                    String::from_utf8_lossy(c)
                );
            }
            for part in s.parts.iter().filter(|x| x.kind == K::Image) {
                assert_eq!(&out[part.range.clone()], &s.bytes[part.range.clone()]);
            }
            assert_eq!(offsets_of(&out), s.offsets);
            assert_eq!(sanitize(&out, p).unwrap(), out, "idempotent");
        }
        // GpsAndDevice and Gps keep all four, byte-identical (MakerNote is not
        // in GpsAndDevice).
        for p in [StripPolicy::GpsAndDevice, StripPolicy::Gps] {
            let out = strip(&s.bytes, p).unwrap().into_bytes();
            assert_eq!(out, s.bytes, "{} {p:?}", s.name);
        }
    }
}
