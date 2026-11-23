//! SPEC-038: the WebP strip writer (DEC-022) against an independent model.
//! Every fixture is a list of parts, each with the role it plays (what the
//! policy does to it is computed here from DEC-022 clauses 1 to 7, never from
//! the library), and the expected output is the concatenation of the parts the
//! model keeps with the RIFF size and the `VP8X` flag byte patched by the
//! rules of clauses 6 and 7 (the I1 recomputation).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

#[path = "support/oracle_png.rs"]
mod oracle_png;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/oracle_webp.rs"]
mod oracle_webp;
#[path = "support/webp_carriers.rs"]
mod webp_carriers;
#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use exif_core::audit::{audit, Category, ContainerKind, ItemKind};
use exif_core::policy::{StripPolicy, StripSpec};
use exif_core::write::{PatchKind, RemovalKind, UneditableReason, UnstrippableReason};
use exif_core::{sanitize, strip, StripError};
use webp_carriers::{payload, row_chunk, spellings, Payload, JUNK_NOT_TEXT, JUNK_TEXT, ROWS};
use webp_shapes::{chunk, exif_chunk, raw_chunk, vp8l, vp8x, FLAG_EXIF, FLAG_ICC, FLAG_XMP};

// ── the model ───────────────────────────────────────────────────────────────

/// What a part is, for the policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Image structure and anything the writer has no rule for: always kept.
    Keep,
    /// Removed with this category (DEC-022 clauses 2 and 4).
    Cat(Category),
    /// A carrier (DEC-022 clause 4): removed by every preset.
    Carrier,
    /// An `EXIF` chunk (DEC-022 clause 3): whole, none or edited.
    Exif,
}

/// Which `VP8X` flag a part's kind drives (DEC-022 clause 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flag {
    None,
    Icc,
    Exif,
    Xmp,
}

impl Flag {
    fn bit(self) -> u8 {
        match self {
            Flag::None => 0,
            Flag::Icc => FLAG_ICC,
            Flag::Exif => FLAG_EXIF,
            Flag::Xmp => FLAG_XMP,
        }
    }
}

#[derive(Clone, Debug)]
struct Part {
    name: String,
    bytes: Vec<u8>,
    role: Role,
    flag: Flag,
}

fn part(name: &str, bytes: Vec<u8>, role: Role, flag: Flag) -> Part {
    Part {
        name: name.to_string(),
        bytes,
        role,
        flag,
    }
}

const EXIF9: [Category; 9] = [
    Category::Gps,
    Category::DeviceId,
    Category::Authorship,
    Category::FreeText,
    Category::Timestamps,
    Category::CameraModel,
    Category::Thumbnail,
    Category::MakerNote,
    Category::Software,
];

fn spec_of(cats: &[Category]) -> StripSpec {
    cats.iter().fold(StripSpec::none(), |s, c| s.include(*c))
}

/// DEC-022 clause 4: a carrier goes with any tag category, `Xmp` or `Iptc`, and
/// with its own category `OtherSegment`.
fn carrier_removed(spec: StripSpec) -> bool {
    EXIF9
        .iter()
        .chain([Category::Xmp, Category::Iptc].iter())
        .any(|c| spec.contains(*c))
        || spec.contains(Category::OtherSegment)
}

#[derive(Debug, PartialEq, Eq)]
enum ExifMode {
    Whole,
    Untouched,
    Partial,
}

/// DEC-022 clause 3.
fn exif_mode(spec: StripSpec) -> ExifMode {
    if EXIF9.iter().all(|c| spec.contains(*c)) {
        ExifMode::Whole
    } else if EXIF9
        .iter()
        .chain([Category::Xmp, Category::Iptc, Category::Icc].iter())
        .any(|c| spec.contains(*c))
    {
        ExifMode::Partial
    } else {
        ExifMode::Untouched
    }
}

/// Whether the policy removes `p` whole (clauses 2 to 4).
fn removed(p: &Part, spec: StripSpec) -> bool {
    match p.role {
        Role::Keep => false,
        Role::Cat(c) => spec.contains(c),
        Role::Carrier => carrier_removed(spec),
        Role::Exif => exif_mode(spec) == ExifMode::Whole,
    }
}

/// How the RIFF size field is set in a fixture.
#[derive(Clone, Copy, Debug)]
enum Declared {
    /// The body plus four.
    Exact,
    /// The exact size plus this many (too large; huge values run past the end).
    Plus(u32),
    /// The exact size less this many (too small).
    Minus(u32),
    /// Exactly the first `n` parts: the rest is beyond the declared size.
    FirstParts(usize),
}

#[derive(Clone, Debug)]
struct Fixture {
    name: String,
    parts: Vec<Part>,
    declared: Declared,
}

impl Fixture {
    fn declared_value(&self) -> u32 {
        let exact: usize = self.parts.iter().map(|p| p.bytes.len()).sum::<usize>() + 4;
        let exact = u32::try_from(exact).unwrap();
        match self.declared {
            Declared::Exact => exact,
            Declared::Plus(n) => exact.saturating_add(n),
            Declared::Minus(n) => exact.saturating_sub(n),
            Declared::FirstParts(n) => {
                let first: usize = self.parts.iter().take(n).map(|p| p.bytes.len()).sum();
                u32::try_from(first + 4).unwrap()
            }
        }
    }

    fn bytes(&self) -> Vec<u8> {
        header(self.declared_value(), &self.parts)
    }
}

fn header(declared: u32, parts: &[Part]) -> Vec<u8> {
    let mut v = b"RIFF".to_vec();
    v.extend_from_slice(&declared.to_le_bytes());
    v.extend_from_slice(b"WEBP");
    for p in parts {
        v.extend_from_slice(&p.bytes);
    }
    v
}

/// The I1 recomputation: the input with exactly the removals deleted and the
/// two patches applied by the rules of DEC-022 clauses 6 and 7. Returns the
/// bytes, whether the size was patched and whether the flags were.
fn expected(fx: &Fixture, spec: StripSpec) -> (Vec<u8>, bool, bool) {
    let declared = fx.declared_value();
    let len: usize = 12 + fx.parts.iter().map(|p| p.bytes.len()).sum::<usize>();
    let window_end = (8usize + declared as usize).min(len);
    let mut at = 12usize;
    let mut inside = 0usize;
    let mut kept: Vec<Part> = Vec::new();
    for p in &fx.parts {
        let (start, end) = (at, at + p.bytes.len());
        at = end;
        if removed(p, spec) {
            let (s, e) = (start.max(8), end.min(window_end));
            if s < e {
                inside += e - s;
            }
        } else {
            kept.push(p.clone());
        }
    }
    let new_declared = declared - u32::try_from(inside).unwrap();
    // Flags: a kind whose chunks were all removed (and were present) loses its
    // bit, in the first `VP8X` (the first part, when it is one).
    let mut flag_patched = false;
    if let Some(first) = kept.first_mut() {
        if first.bytes.get(..4) == Some(b"VP8X") && first.bytes.len() >= 18 {
            for flag in [Flag::Icc, Flag::Exif, Flag::Xmp] {
                let all: Vec<&Part> = fx.parts.iter().filter(|p| p.flag == flag).collect();
                if !all.is_empty()
                    && all.iter().all(|p| removed(p, spec))
                    && first.bytes[8] & flag.bit() != 0
                {
                    first.bytes[8] &= !flag.bit();
                    flag_patched = true;
                }
            }
        }
    }
    (
        header(new_declared, &kept),
        new_declared != declared,
        flag_patched,
    )
}

// ── fixtures ────────────────────────────────────────────────────────────────

fn xmp_chunk() -> Vec<u8> {
    chunk(b"XMP ", &payload(Payload::Xmp))
}

fn icc_chunk() -> Vec<u8> {
    let mut d = vec![0u8; 132];
    d[0..4].copy_from_slice(&132u32.to_be_bytes());
    d[36..40].copy_from_slice(b"acsp");
    chunk(b"ICCP", &d)
}

fn make_exif() -> Vec<u8> {
    exif_chunk(&webp_shapes::exif_with_make())
}

/// Every measured row as a part, plus `EXIF`, `XMP ` and `ICCP`.
fn rows() -> Vec<Part> {
    let mut out = vec![
        part("EXIF", make_exif(), Role::Exif, Flag::Exif),
        part("XMP ", xmp_chunk(), Role::Cat(Category::Xmp), Flag::Xmp),
        part("ICCP", icc_chunk(), Role::Cat(Category::Icc), Flag::Icc),
        part(
            "XMP\\0",
            chunk(b"XMP\0", &payload(Payload::Xmp)),
            Role::Cat(Category::Xmp),
            Flag::Xmp,
        ),
    ];
    for row in ROWS {
        let role = match (row.region, row.category) {
            ("carrier", _) => Role::Carrier,
            ("jumbf", _) => Role::Cat(Category::Jumbf),
            _ => Role::Cat(Category::OtherSegment),
        };
        out.push(part(
            &String::from_utf8_lossy(&row.code),
            row_chunk(&row.code, row.payload),
            role,
            Flag::None,
        ));
    }
    out
}

fn keep(name: &str, bytes: Vec<u8>) -> Part {
    part(name, bytes, Role::Keep, Flag::None)
}

/// A `VP8X` chunk with every metadata flag, alpha and animation set.
fn vp8x_all() -> Part {
    keep(
        "VP8X",
        vp8x(FLAG_ICC | FLAG_EXIF | FLAG_XMP | 0x10 | 0x02, 1, 1),
    )
}

/// The positions of the grid: the part, before the image data, after it,
/// beyond the declared size, after an empty null chunk, and without a `VP8X`.
fn placements(p: &Part) -> Vec<(String, Vec<Part>, Declared)> {
    let image = keep("VP8L", vp8l());
    let null = keep("null", vec![0u8; 8]);
    vec![
        (
            "before".into(),
            vec![vp8x_all(), p.clone(), image.clone()],
            Declared::Exact,
        ),
        (
            "after".into(),
            vec![vp8x_all(), image.clone(), p.clone()],
            Declared::Exact,
        ),
        (
            "beyond".into(),
            vec![vp8x_all(), image.clone(), p.clone()],
            Declared::FirstParts(2),
        ),
        (
            "after-null".into(),
            vec![vp8x_all(), image.clone(), null, p.clone()],
            Declared::Exact,
        ),
        (
            "no-vp8x".into(),
            vec![image.clone(), p.clone()],
            Declared::Exact,
        ),
    ]
}

fn grid() -> Vec<Fixture> {
    let mut out = Vec::new();
    for p in rows() {
        for (pos, parts, declared) in placements(&p) {
            out.push(Fixture {
                name: format!("{}@{pos}", p.name),
                parts: parts.clone(),
                declared,
            });
            if matches!(pos.as_str(), "after") {
                for d in [
                    Declared::Plus(7),
                    Declared::Plus(u32::MAX - 100_000),
                    Declared::Minus(5),
                ] {
                    out.push(Fixture {
                        name: format!("{}@{pos}-{d:?}", p.name),
                        parts: parts.clone(),
                        declared: d,
                    });
                }
            }
        }
    }
    out
}

fn specs() -> Vec<(String, StripPolicy)> {
    let mut out: Vec<(String, StripPolicy)> = StripPolicy::PRESETS
        .iter()
        .map(|p| (p.name().to_string(), *p))
        .collect();
    for (name, cats) in [
        ("icc", vec![Category::Icc]),
        ("jumbf", vec![Category::Jumbf]),
        ("other-segment", vec![Category::OtherSegment]),
        ("xmp", vec![Category::Xmp]),
        ("authorship", vec![Category::Authorship]),
        ("none", vec![]),
        ("xmp+icc", vec![Category::Xmp, Category::Icc]),
        ("iptc", vec![Category::Iptc]),
    ] {
        out.push((
            format!("custom-{name}"),
            StripPolicy::Custom(spec_of(&cats)),
        ));
    }
    out
}

// ── the I1 grid ─────────────────────────────────────────────────────────────

#[test]
fn every_row_in_every_position_matches_the_model_under_every_policy() {
    let mut checked = 0usize;
    for fx in grid() {
        let input = fx.bytes();
        for (pname, policy) in specs() {
            let spec = policy.spec();
            let has_exif = fx.parts.iter().any(|p| p.role == Role::Exif);
            if has_exif && exif_mode(spec) == ExifMode::Partial {
                // A partial edit rewrites entries; tested apart.
                continue;
            }
            let out = strip(&input, policy)
                .unwrap_or_else(|e| panic!("{} under {pname}: {e:?}", fx.name));
            let (want, size_patched, flags_patched) = expected(&fx, spec);
            assert_eq!(
                out.bytes(),
                want.as_slice(),
                "{} under {pname}: output differs from the I1 recomputation",
                fx.name
            );
            assert_eq!(
                out.patches().iter().any(|p| p.kind == PatchKind::RiffSize),
                size_patched,
                "{} under {pname}: RiffSize patch",
                fx.name
            );
            assert_eq!(
                out.patches().iter().any(|p| p.kind == PatchKind::Vp8xFlags),
                flags_patched,
                "{} under {pname}: Vp8xFlags patch",
                fx.name
            );
            let removed_parts = fx.parts.iter().filter(|p| removed(p, spec)).count();
            assert_eq!(out.removals().len(), removed_parts, "{} {pname}", fx.name);
            assert!(out.bytes().len() <= input.len());
            // Idempotent.
            let again = strip(out.bytes(), policy)
                .unwrap_or_else(|e| panic!("{} under {pname} twice: {e:?}", fx.name));
            assert_eq!(again.bytes(), out.bytes(), "{} {pname} idempotent", fx.name);
            assert!(again.removals().is_empty() && again.patches().is_empty());
            checked += 1;
        }
    }
    assert!(checked > 3000, "{checked} cases");
}

#[test]
fn removal_kinds_follow_the_category() {
    let image = keep("VP8L", vp8l());
    let all = |name: &str| rows().into_iter().find(|p| p.name == name).unwrap();
    for (name, kind) in [
        ("XMP ", RemovalKind::Xmp),
        ("ICCP", RemovalKind::Icc),
        ("EXIF", RemovalKind::Exif),
        ("IDIT", RemovalKind::Carrier),
        ("LIST", RemovalKind::Carrier),
        ("C2PA", RemovalKind::Jumbf),
        ("fmt ", RemovalKind::OtherChunk),
    ] {
        let fx = Fixture {
            name: name.to_string(),
            parts: vec![vp8x_all(), image.clone(), all(name)],
            declared: Declared::Exact,
        };
        let out = strip(&fx.bytes(), StripPolicy::Everything).unwrap();
        assert_eq!(out.removals().len(), 1, "{name}");
        assert_eq!(out.removals()[0].kind, kind, "{name}");
    }
}

// ── presets and the carriers' category ──────────────────────────────────────

#[test]
fn presets_keep_xmp_icc_and_c2pa_as_for_jpeg_and_png_but_remove_carriers() {
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    let fx = Fixture {
        name: "mix".into(),
        parts: vec![
            vp8x_all(),
            image,
            by("XMP "),
            by("ICCP"),
            by("C2PA"),
            by("LIST"),
            by("fmt "),
        ],
        declared: Declared::Exact,
    };
    let input = fx.bytes();
    let has = |out: &[u8], code: &[u8; 4]| out.windows(4).any(|w| w == code);
    for policy in [
        StripPolicy::Gps,
        StripPolicy::GpsAndDevice,
        StripPolicy::AllPii,
    ] {
        let out = sanitize(&input, policy).unwrap();
        // `Gps`, `GpsAndDevice` and `AllPii` keep XMP (DEC-009); `All` removes it.
        assert!(has(&out, b"XMP "), "{policy:?} keeps XMP");
        assert!(has(&out, b"ICCP"), "{policy:?} keeps ICC");
        assert!(has(&out, b"C2PA"), "{policy:?} keeps C2PA");
        assert!(!has(&out, b"LIST"), "{policy:?} removes the carrier");
        assert!(has(&out, b"fmt "), "{policy:?} keeps the structure chunk");
    }
    let out = sanitize(&input, StripPolicy::All).unwrap();
    assert!(has(&out, b"C2PA") && !has(&out, b"XMP ") && has(&out, b"ICCP"));
    let out = sanitize(&input, StripPolicy::Everything).unwrap();
    for code in [b"C2PA", b"LIST", b"fmt ", b"XMP ", b"ICCP"] {
        assert!(!has(&out, code), "everything removes {code:?}");
    }
}

#[test]
fn custom_policies_decide_carriers_by_clause_4() {
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    let fx = Fixture {
        name: "carriers".into(),
        parts: vec![vp8x_all(), image, by("LIST"), by("fmt "), by("C2PA")],
        declared: Declared::Exact,
    };
    let input = fx.bytes();
    let has = |out: &[u8], code: &[u8; 4]| out.windows(4).any(|w| w == code);
    // Icc, Jumbf and the empty policy leave a carrier.
    for cats in [vec![Category::Icc], vec![], vec![Category::EmbeddedImage]] {
        let out = sanitize(&input, StripPolicy::Custom(spec_of(&cats))).unwrap();
        assert!(has(&out, b"LIST"), "{cats:?} leaves a carrier");
    }
    // Jumbf removes C2PA only; `Xmp` or `Iptc` alone remove the carrier.
    let out = sanitize(&input, StripPolicy::Custom(spec_of(&[Category::Jumbf]))).unwrap();
    assert!(!has(&out, b"C2PA") && has(&out, b"LIST") && has(&out, b"fmt "));
    for cat in [Category::Xmp, Category::Iptc, Category::Gps] {
        let out = sanitize(&input, StripPolicy::Custom(spec_of(&[cat]))).unwrap();
        assert!(!has(&out, b"LIST") && has(&out, b"fmt ") && has(&out, b"C2PA"));
    }
    // OtherSegment removes the carrier and the structure chunk.
    let out = sanitize(
        &input,
        StripPolicy::Custom(spec_of(&[Category::OtherSegment])),
    )
    .unwrap();
    assert!(!has(&out, b"LIST") && !has(&out, b"fmt ") && has(&out, b"C2PA"));
}

// ── case ────────────────────────────────────────────────────────────────────

#[test]
fn the_case_grid_removes_only_the_measured_spellings_under_every_preset() {
    let image = keep("VP8L", vp8l());
    for row in ROWS {
        let role = if row.region == "carrier" {
            Role::Carrier
        } else {
            continue;
        };
        for sp in spellings(row.code) {
            let known = ROWS.iter().any(|r| r.code == sp);
            let p = part(
                &String::from_utf8_lossy(&sp),
                chunk(&sp, &payload(row.payload)),
                if known { role } else { Role::Keep },
                Flag::None,
            );
            let fx = Fixture {
                name: format!("case {:?}", String::from_utf8_lossy(&sp)),
                parts: vec![vp8x_all(), image.clone(), p],
                declared: Declared::Exact,
            };
            for policy in StripPolicy::PRESETS {
                let out = strip(&fx.bytes(), policy).unwrap();
                let (want, _, _) = expected(&fx, policy.spec());
                assert_eq!(out.bytes(), want.as_slice(), "{} {policy:?}", fx.name);
                // Past the 12-byte header (whose first four bytes are `RIFF`).
                let survives = out.bytes()[12..].windows(4).any(|w| w == sp);
                assert_eq!(survives, !known, "{} {policy:?}", fx.name);
            }
        }
    }
}

#[test]
fn junk_goes_only_when_it_is_text_junk() {
    let image = keep("VP8L", vp8l());
    for (data, text) in JUNK_TEXT
        .iter()
        .map(|d| (*d, true))
        .chain(JUNK_NOT_TEXT.iter().map(|d| (*d, false)))
    {
        let p = part(
            "JUNK",
            chunk(b"JUNK", data),
            if text { Role::Carrier } else { Role::Keep },
            Flag::None,
        );
        let fx = Fixture {
            name: format!("junk {data:?}"),
            parts: vec![vp8x_all(), image.clone(), p],
            declared: Declared::Exact,
        };
        for policy in StripPolicy::PRESETS {
            let out = strip(&fx.bytes(), policy).unwrap();
            let (want, _, _) = expected(&fx, policy.spec());
            assert_eq!(out.bytes(), want.as_slice(), "{} {policy:?}", fx.name);
        }
    }
}

// ── the RIFF size ───────────────────────────────────────────────────────────

fn size_of(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[4..8].try_into().unwrap())
}

#[test]
fn the_riff_size_loses_exactly_what_was_removed_inside_the_declared_range() {
    let image = keep("VP8L", vp8l());
    let idit = rows().into_iter().find(|p| p.name == "IDIT").unwrap();
    let removed_len = u32::try_from(idit.bytes.len()).unwrap();
    let parts = vec![vp8x_all(), image.clone(), idit.clone()];
    let exact = Fixture {
        name: "exact".into(),
        parts: parts.clone(),
        declared: Declared::Exact,
    };
    let input = exact.bytes();
    let out = sanitize(&input, StripPolicy::Gps).unwrap();
    // An exact size stays consistent with the new length.
    assert_eq!(size_of(&out), u32::try_from(out.len() - 8).unwrap());
    assert_eq!(size_of(&input) - size_of(&out), removed_len);

    // The removed chunk straddles a declared end that is 5 bytes short: only
    // the part inside the range is subtracted, so the lie disappears.
    let small = Fixture {
        declared: Declared::Minus(5),
        ..exact.clone()
    };
    let out = sanitize(&small.bytes(), StripPolicy::Gps).unwrap();
    assert_eq!(size_of(&out) as usize, out.len() - 8);

    // With a kept chunk after it, a size 5 too small stays 5 too small, and a
    // size 7 too large stays 7 too large.
    let tail = keep("abcd", chunk(b"abcd", &[1u8; 20]));
    let with_tail = vec![vp8x_all(), image.clone(), idit.clone(), tail];
    let small = Fixture {
        name: "small".into(),
        parts: with_tail.clone(),
        declared: Declared::Minus(5),
    };
    let out = sanitize(&small.bytes(), StripPolicy::Gps).unwrap();
    assert_eq!(size_of(&out) as usize, out.len() - 8 - 5);
    let large = Fixture {
        name: "large".into(),
        parts: with_tail,
        declared: Declared::Plus(7),
    };
    let out = sanitize(&large.bytes(), StripPolicy::Gps).unwrap();
    assert_eq!(size_of(&out) as usize, out.len() - 8 + 7);

    // Past the end by far: the window is clipped to the input, the field
    // loses what was removed.
    let huge = Fixture {
        declared: Declared::Plus(1_000_000),
        ..exact.clone()
    };
    let before = size_of(&huge.bytes());
    let out = sanitize(&huge.bytes(), StripPolicy::Gps).unwrap();
    assert_eq!(before - size_of(&out), removed_len);

    // Beyond the declared end: nothing inside was removed, the field is
    // untouched and no RiffSize patch is made.
    let beyond = Fixture {
        declared: Declared::FirstParts(2),
        ..exact
    };
    let stripped = strip(&beyond.bytes(), StripPolicy::Gps).unwrap();
    assert_eq!(size_of(stripped.bytes()), size_of(&beyond.bytes()));
    assert!(stripped
        .patches()
        .iter()
        .all(|p| p.kind != PatchKind::RiffSize));
    assert_eq!(
        stripped.bytes().len() + idit.bytes.len(),
        beyond.bytes().len()
    );
}

#[test]
fn a_declared_size_of_zero_or_odd_survives_without_underflow() {
    let image = keep("VP8L", vp8l());
    let idit = rows().into_iter().find(|p| p.name == "IDIT").unwrap();
    for declared in [0u32, 1, 3, 4, 5] {
        let mut v = header(declared, &[vp8x_all(), image.clone(), idit.clone()]);
        v[4..8].copy_from_slice(&declared.to_le_bytes());
        let out = sanitize(&v, StripPolicy::Gps).unwrap();
        // The window (8..8+declared) holds none of the removed chunk.
        assert_eq!(size_of(&out), declared);
    }
}

// ── the VP8X flags ──────────────────────────────────────────────────────────

fn flags_of(bytes: &[u8]) -> u8 {
    // The first chunk is a `VP8X` at offset 12; its flags are at 20.
    assert_eq!(&bytes[12..16], b"VP8X");
    bytes[20]
}

#[test]
fn flag_bits_clear_exactly_for_the_kinds_removed_and_nothing_else_moves() {
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    let alpha_anim = 0x10 | 0x02;
    let fx = Fixture {
        name: "flags".into(),
        parts: vec![vp8x_all(), image, by("EXIF"), by("XMP "), by("ICCP")],
        declared: Declared::Exact,
    };
    let input = fx.bytes();
    assert_eq!(
        flags_of(&input),
        FLAG_ICC | FLAG_EXIF | FLAG_XMP | alpha_anim
    );
    // `Gps` keeps XMP and ICC and edits Exif in place: no bit moves.
    let out = sanitize(&input, StripPolicy::Gps).unwrap();
    assert_eq!(flags_of(&out), flags_of(&input));
    // A policy that removes Xmp only clears the XMP bit.
    let out = sanitize(&input, StripPolicy::Custom(spec_of(&[Category::Xmp]))).unwrap();
    assert_eq!(flags_of(&out), FLAG_ICC | FLAG_EXIF | alpha_anim);
    // Icc only.
    let out = sanitize(&input, StripPolicy::Custom(spec_of(&[Category::Icc]))).unwrap();
    assert_eq!(flags_of(&out), FLAG_EXIF | FLAG_XMP | alpha_anim);
    // `All` removes all three; the alpha and animation bits stay.
    let out = sanitize(&input, StripPolicy::Everything).unwrap();
    assert_eq!(flags_of(&out), alpha_anim);
}

#[test]
fn a_bit_clears_only_when_every_chunk_of_its_kind_is_gone() {
    // `XMP ` and `XMP\0` are both XMP: one policy removes both or neither.
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    let fx = Fixture {
        name: "two-xmp".into(),
        parts: vec![vp8x_all(), image, by("XMP "), by("XMP\\0")],
        declared: Declared::Exact,
    };
    let out = sanitize(&fx.bytes(), StripPolicy::All).unwrap();
    assert_eq!(flags_of(&out) & FLAG_XMP, 0);
    assert_eq!(out.windows(4).filter(|w| *w == b"XMP ").count(), 0);
    let out = sanitize(&fx.bytes(), StripPolicy::Gps).unwrap();
    assert_eq!(flags_of(&out) & FLAG_XMP, FLAG_XMP);
}

#[test]
fn a_flag_bit_that_was_never_set_is_not_invented_and_a_stray_flag_stays() {
    // EXIF removed, flag clear: no flag patch. XMP flag set with no XMP chunk
    // (a file that already lied): left as it was.
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    let fx = Fixture {
        name: "stray".into(),
        parts: vec![keep("VP8X", vp8x(FLAG_XMP, 1, 1)), image, by("EXIF")],
        declared: Declared::Exact,
    };
    let stripped = strip(&fx.bytes(), StripPolicy::Everything).unwrap();
    assert_eq!(flags_of(stripped.bytes()), FLAG_XMP);
    assert!(stripped
        .patches()
        .iter()
        .all(|p| p.kind != PatchKind::Vp8xFlags));
}

#[test]
fn only_the_vp8x_the_reader_reports_is_patched() {
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    // A short first `VP8X` (3 data bytes): the reader reports none, nothing is
    // patched, no refusal.
    let short = keep("VP8X3", chunk(b"VP8X", &[FLAG_XMP, 0, 0]));
    let fx = Fixture {
        name: "short".into(),
        parts: vec![short.clone(), image.clone(), by("XMP ")],
        declared: Declared::Exact,
    };
    let stripped = strip(&fx.bytes(), StripPolicy::All).unwrap();
    assert_eq!(stripped.bytes()[20], FLAG_XMP);
    assert!(stripped
        .patches()
        .iter()
        .all(|p| p.kind != PatchKind::Vp8xFlags));
    // A second, misplaced `VP8X` is never patched; the first full one is.
    let second = keep("VP8X2", vp8x(FLAG_XMP, 1, 1));
    let fx = Fixture {
        name: "second".into(),
        parts: vec![
            keep("VP8X", vp8x(FLAG_XMP, 1, 1)),
            image,
            second.clone(),
            by("XMP "),
        ],
        declared: Declared::Exact,
    };
    let out = sanitize(&fx.bytes(), StripPolicy::All).unwrap();
    assert_eq!(flags_of(&out) & FLAG_XMP, 0);
    let again = out
        .windows(18)
        .position(|w| w == second.bytes.as_slice())
        .unwrap();
    assert_eq!(
        out[again + 8] & FLAG_XMP,
        FLAG_XMP,
        "the second VP8X is untouched"
    );
}

// ── EXIF ────────────────────────────────────────────────────────────────────

/// A big-endian TIFF: IFD0 holds `Make` and a GPS pointer; the GPS IFD holds
/// `GPSLatitudeRef`.
fn tiff_make_and_gps() -> Vec<u8> {
    let mut d = b"MM\x00\x2A\x00\x00\x00\x08".to_vec();
    d.extend_from_slice(&[0, 2]);
    d.extend_from_slice(&[0x01, 0x0F, 0, 2, 0, 0, 0, 4]);
    d.extend_from_slice(b"abc\0");
    d.extend_from_slice(&[0x88, 0x25, 0, 4, 0, 0, 0, 1, 0, 0, 0, 38]);
    d.extend_from_slice(&[0, 0, 0, 0]);
    d.extend_from_slice(&[0, 1, 0, 1, 0, 2, 0, 0, 0, 2, b'N', 0, 0, 0]);
    d.extend_from_slice(&[0, 0, 0, 0]);
    d
}

fn tags(bytes: &[u8]) -> Vec<(Option<usize>, u16)> {
    audit(bytes)
        .items()
        .iter()
        .filter_map(|i| match i.kind() {
            ItemKind::Tag { tag, .. } => Some((i.origin().block, tag)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_partial_policy_deletes_exif_entries_in_place_with_and_without_the_prefix() {
    let image = keep("VP8L", vp8l());
    for prefixed in [false, true] {
        let tiff = tiff_make_and_gps();
        let payload = if prefixed {
            webp_shapes::with_exif_prefix(&tiff)
        } else {
            tiff.clone()
        };
        let fx = Fixture {
            name: "partial".into(),
            parts: vec![
                keep("VP8X", vp8x(FLAG_EXIF, 1, 1)),
                image.clone(),
                part("EXIF", exif_chunk(&payload), Role::Exif, Flag::Exif),
            ],
            declared: Declared::Exact,
        };
        let input = fx.bytes();
        assert!(tags(&input).iter().any(|t| t.1 == 0x8825 || t.1 == 1));
        let out = strip(&input, StripPolicy::Gps).unwrap();
        // Same length: an in-place edit, nothing removed, no size change.
        assert_eq!(out.bytes().len(), input.len(), "prefixed={prefixed}");
        assert!(out.removals().is_empty());
        assert!(out
            .patches()
            .iter()
            .all(|p| p.kind != PatchKind::RiffSize && p.kind != PatchKind::Vp8xFlags));
        assert!(!out.entries().is_empty());
        // The GPS entries are gone, Make is kept, and the flag stays (the
        // chunk is still there).
        let after = tags(out.bytes());
        assert!(
            !after.iter().any(|t| t.1 == 0x8825 || t.1 == 1),
            "{after:?}"
        );
        assert!(after.iter().any(|t| t.1 == 0x010F));
        assert_eq!(flags_of(out.bytes()), FLAG_EXIF);
        // Bytes outside the EXIF chunk are untouched.
        let at = input.len() - (8 + payload.len() + payload.len() % 2);
        assert_eq!(&out.bytes()[..at], &input[..at]);
        // Idempotent.
        let again = strip(out.bytes(), StripPolicy::Gps).unwrap();
        assert_eq!(again.bytes(), out.bytes());
        // The audit's own policy view of the output: no Gps finding left.
        assert_eq!(audit(out.bytes()).findings_in(Category::Gps).count(), 0);
    }
}

#[test]
fn seventeen_or_more_exif_chunks_all_go_whole_and_a_partial_edit_refuses() {
    let image = keep("VP8L", vp8l());
    for n in [16usize, 17, 20] {
        let mut parts = vec![vp8x_all(), image.clone()];
        for _ in 0..n {
            parts.push(part("EXIF", make_exif(), Role::Exif, Flag::Exif));
        }
        let fx = Fixture {
            name: format!("{n} exif"),
            parts,
            declared: Declared::Exact,
        };
        let input = fx.bytes();
        let out = strip(&input, StripPolicy::Everything).unwrap();
        let (want, _, flags) = expected(&fx, StripPolicy::Everything.spec());
        assert_eq!(out.bytes(), want.as_slice(), "{n}");
        assert!(flags);
        assert_eq!(out.removals().len(), n);
        assert_eq!(flags_of(out.bytes()) & FLAG_EXIF, 0);
        // A partial policy: with more chunks than the reader parses (16) the
        // edit cannot be shown complete.
        let partial = strip(&input, StripPolicy::Gps);
        if n > 16 {
            match partial {
                Err(StripError::Uneditable { reason, offset }) => {
                    assert_eq!(reason, UneditableReason::TooManyExifSegments);
                    let first = 12 + 18 + vp8l().len();
                    let per = make_exif().len();
                    assert_eq!(offset, first + 16 * per, "the 17th chunk");
                }
                other => panic!("{n}: {other:?}"),
            }
        }
    }
}

#[test]
fn an_unparseable_exif_goes_whole_but_blocks_a_partial_edit() {
    let image = keep("VP8L", vp8l());
    let bad = part(
        "EXIF",
        exif_chunk(b"this is not a tiff at all"),
        Role::Exif,
        Flag::Exif,
    );
    let fx = Fixture {
        name: "bad exif".into(),
        parts: vec![keep("VP8X", vp8x(FLAG_EXIF, 1, 1)), image, bad],
        declared: Declared::Exact,
    };
    let input = fx.bytes();
    let out = strip(&input, StripPolicy::Everything).unwrap();
    let (want, _, _) = expected(&fx, StripPolicy::Everything.spec());
    assert_eq!(out.bytes(), want.as_slice());
    assert_eq!(
        strip(&input, StripPolicy::Gps).unwrap_err(),
        StripError::Uneditable {
            reason: UneditableReason::BlockNotClean,
            offset: 12 + 18 + vp8l().len()
        }
    );
    // A policy with no Exif category leaves it alone.
    let out = strip(&input, StripPolicy::Custom(spec_of(&[Category::Jumbf]))).unwrap();
    assert_eq!(out.bytes(), input.as_slice());
}

// ── refusals and tolerated faults ───────────────────────────────────────────

#[test]
fn a_chunk_running_past_the_end_is_refused_with_its_offset() {
    let image = keep("VP8L", vp8l());
    let mut v = header(
        0,
        &[
            vp8x_all(),
            image,
            rows().into_iter().find(|p| p.name == "IDIT").unwrap(),
        ],
    );
    let at = v.len();
    v.extend_from_slice(b"XMP \xe8\x03\0\0");
    v.extend_from_slice(&[1, 2, 3]);
    for policy in StripPolicy::PRESETS {
        assert_eq!(
            strip(&v, policy).unwrap_err(),
            StripError::Unstrippable {
                reason: UnstrippableReason::ChunkPastEnd,
                offset: at
            },
            "{policy:?}"
        );
    }
}

#[test]
fn a_walk_halted_by_a_cap_is_refused() {
    // 65,537 empty chunks: the walk stops at the chunk cap.
    let mut v = header(0, &[vp8x_all()]);
    for _ in 0..65_537 {
        v.extend_from_slice(&chunk(b"abcd", b""));
    }
    match strip(&v, StripPolicy::Gps).unwrap_err() {
        StripError::Unstrippable { reason, .. } => {
            assert_eq!(reason, UnstrippableReason::TooManySegments)
        }
        other => panic!("{other:?}"),
    }
    // 1,100 misplaced `VP8X` chunks: one issue each, so the issue cap halts it.
    let mut v = header(0, &[vp8x_all()]);
    for _ in 0..1_100 {
        v.extend_from_slice(&vp8x(0, 1, 1));
    }
    match strip(&v, StripPolicy::Gps).unwrap_err() {
        StripError::Unstrippable { reason, .. } => {
            assert_eq!(reason, UnstrippableReason::TooManyIssues)
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn faults_that_are_not_refusals() {
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    // Trailing bytes that are not a chunk (fewer than eight): kept as they are.
    let fx = Fixture {
        name: "tail".into(),
        parts: vec![vp8x_all(), image.clone(), by("IDIT")],
        declared: Declared::Exact,
    };
    let mut input = fx.bytes();
    input.extend_from_slice(&[9, 8, 7]);
    let out = sanitize(&input, StripPolicy::Gps).unwrap();
    assert_eq!(&out[out.len() - 3..], &[9, 8, 7]);
    // A missing pad byte on an odd last chunk: removed without one.
    let mut v = header(0, &[vp8x_all(), image.clone()]);
    let base = v.len();
    v.extend_from_slice(b"IDIT\x03\0\0\0abc"); // odd size, no pad byte
    let exact = u32::try_from(v.len() - 8).unwrap();
    v[4..8].copy_from_slice(&exact.to_le_bytes());
    let out = sanitize(&v, StripPolicy::Gps).unwrap();
    assert_eq!(out.len(), base);
    assert_eq!(size_of(&out), exact - 11);
    // A wrong declared size and an exact one give the same bytes up to the
    // field (the lie is kept): covered by the grid.
}

#[test]
fn a_null_chunk_does_not_hide_what_follows() {
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    let fx = Fixture {
        name: "null".into(),
        parts: vec![
            vp8x_all(),
            image,
            keep("null", vec![0u8; 8]),
            by("LIST"),
            by("XMP "),
        ],
        declared: Declared::Exact,
    };
    let out = sanitize(&fx.bytes(), StripPolicy::All).unwrap();
    let (want, _, _) = expected(&fx, StripPolicy::All.spec());
    assert_eq!(out, want);
    assert!(!out.windows(4).any(|w| w == b"LIST" || w == b"XMP "));
}

// ── the oracle fixtures and the honest preview ──────────────────────────────

#[test]
fn every_oracle_webp_fixture_strips_or_refuses_under_every_preset_and_is_a_fixed_point() {
    let fixtures = oracle_webp::fixtures();
    assert!(fixtures.len() >= 98, "{} fixtures", fixtures.len());
    let (mut ok, mut refused) = (0, 0);
    for (name, input) in &fixtures {
        for policy in StripPolicy::PRESETS {
            match strip(input, policy) {
                Ok(out) => {
                    ok += 1;
                    assert!(out.bytes().len() <= input.len(), "{name} {policy:?}");
                    let again = strip(out.bytes(), policy)
                        .unwrap_or_else(|e| panic!("{name} {policy:?} twice: {e:?}"));
                    assert_eq!(
                        again.bytes(),
                        out.bytes(),
                        "{name} {policy:?} not idempotent"
                    );
                }
                Err(StripError::VerifyFailed { check }) => {
                    panic!("{name} {policy:?}: self-check failed ({check:?})")
                }
                Err(_) => refused += 1,
            }
        }
    }
    println!(
        "{} fixtures: {ok} stripped, {refused} refused",
        fixtures.len()
    );
    assert!(ok > refused, "most strip ({ok} ok, {refused} refused)");
}

#[test]
fn a_working_strip_and_strippable_false_coexist_for_webp() {
    // DEC-022 clause 10: the audit and the CLI keep saying "not strippable"
    // until SPEC-039 shows ExifTool agrees and flips it.
    assert!(!ContainerKind::Webp.strippable());
    let image = keep("VP8L", vp8l());
    let by = |n: &str| rows().into_iter().find(|p| p.name == n).unwrap();
    let fx = Fixture {
        name: "coexist".into(),
        parts: vec![vp8x_all(), image, by("LIST"), by("XMP ")],
        declared: Declared::Exact,
    };
    let input = fx.bytes();
    let out = strip(&input, StripPolicy::AllPii).unwrap();
    assert!(out.bytes().len() < input.len());
    let a = audit(&input);
    let pv = exif_core::audit::preview(&a, StripPolicy::AllPii);
    assert!(!pv.strippable());
    assert_eq!(pv.counts().removed, 0);
}

// ── a chunk coded `RIFF` (SPEC-038 fix round, DEC-022 clause 4) ─────────────

fn riff_of(parts: &[Vec<u8>]) -> Vec<u8> {
    webp_shapes::riff(parts)
}

/// ExifTool 13.55 walks into a chunk coded exactly `RIFF`, whatever its form
/// type, size, depth or position (measured), so the strip removes it under
/// every preset and the audit reads the file as `unreadable`.
#[test]
fn a_riff_chunk_is_an_opaque_carrier_wherever_it_lies_and_however_deep() {
    let image = vp8l();
    let nested = |depth| chunk(b"RIFF", &webp_carriers::riff_nested(depth));
    let bare = riff_of(std::slice::from_ref(&image));
    // The verifier's two minimal cases: a nested one before the image, and a
    // whole second file appended after the first.
    let mut appended = bare.clone();
    appended.extend_from_slice(&riff_of(&[chunk(b"LIST", &payload(Payload::ListInfo))]));
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("nested before", riff_of(&[nested(1), image.clone()])),
        ("nested after", riff_of(&[image.clone(), nested(1)])),
        ("depth 3", riff_of(&[image.clone(), nested(3)])),
        ("depth 12", riff_of(&[image.clone(), nested(12)])),
        ("appended second file", appended),
        (
            "form type AVI",
            riff_of(&[image.clone(), chunk(b"RIFF", b"AVI LIST\0\0\0\0")]),
        ),
        (
            "no form type",
            riff_of(&[image.clone(), chunk(b"RIFF", b"LI")]),
        ),
    ];
    for (name, input) in &cases {
        // A non-empty chunk the audit cannot read is never `clean`.
        let a = audit(input);
        assert!(a.health().failed_blocks >= 1, "{name}: failed block");
        for policy in StripPolicy::PRESETS {
            let out = strip(input, policy).unwrap_or_else(|e| panic!("{name} {policy:?}: {e:?}"));
            assert!(
                !out.bytes()[4..].windows(4).any(|w| w == b"RIFF"),
                "{name} {policy:?}: a RIFF chunk is still there"
            );
            assert_eq!(
                audit(out.bytes()).health().failed_blocks,
                0,
                "{name} {policy:?}"
            );
        }
    }
    // A size too short to hold the nested file leaves the walker inside the
    // chunk with a bogus next chunk that runs past the end: refused, fail
    // closed (DEC-022 clause 8), never a silent partial strip.
    let lying = riff_of(&[
        raw_chunk(b"RIFF", 6, &webp_carriers::riff_nested(1)),
        image.clone(),
    ]);
    assert!(matches!(
        strip(&lying, StripPolicy::Everything),
        Err(StripError::Unstrippable { .. })
    ));
    // Not walked by ExifTool: `RF64`, `RIFX`, lower-case spellings and the
    // other case spellings are kept (measured: they print `Unknown_*` only).
    for code in [b"RF64", b"RIFX", b"riff", b"Riff", b"rIFF", b"RIFf"] {
        let input = riff_of(&[image.clone(), chunk(code, &webp_carriers::riff_nested(1))]);
        let out = strip(&input, StripPolicy::Everything).unwrap();
        assert_eq!(out.bytes(), input.as_slice(), "{code:?} must be kept");
    }
}
