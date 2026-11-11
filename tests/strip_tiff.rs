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

/// The policy's expected result for a shape: a refusal when the shape's IFD1 is
/// a page and the policy removes `Thumbnail` (the audit calls a strips IFD1 a
/// thumbnail; cutting it would destroy a page).
fn expect_refusal(s: &Shape, spec: StripSpec) -> bool {
    s.ifd1_page && spec.contains(Category::Thumbnail)
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
    // A policy with Thumbnail is refused: fail closed.
    assert!(matches!(
        strip(&page.bytes, StripPolicy::AllPii),
        Err(StripError::Uneditable {
            reason: UneditableReason::ThumbnailNotLocated,
            ..
        })
    ));
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
