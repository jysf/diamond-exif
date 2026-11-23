//! SPEC-037: the PNG strip writer (DEC-021) against an independent model. Every
//! fixture is a list of parts, each with the role it plays (what the policy does
//! to it is computed here from DEC-021 clauses 1 to 4, never from the library),
//! and the expected output is the concatenation of the parts the model keeps.

#[path = "support/oracle_png.rs"]
mod oracle_png;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/png_shapes.rs"]
mod png_shapes;

use std::ops::Range;

use exif_core::audit::{
    audit, preview, Category, ContainerKind, Disposition, ItemKind, RegionKind,
};
use exif_core::policy::{StripPolicy, StripSpec};
use exif_core::write::{Check, PatchKind, RemovalKind, UneditableReason, UnstrippableReason};
use exif_core::{sanitize, strip, StripError};
use png_shapes::{
    chunk, chunk_with_crc, crc32, exif_chunk, iccp, idat, iend, ihdr, itxt, raw_chunk, text, ztxt,
    SIG,
};

// ── the model ───────────────────────────────────────────────────────────────

/// What a part is, for the policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Image structure and anything the writer has no rule for: always kept.
    Keep,
    /// Removed with this category (DEC-021 clause 2).
    Cat(Category),
    /// An opaque carrier (DEC-021 clause 4): removed by every preset.
    Carrier,
    /// An `eXIf` chunk (DEC-021 clause 3): whole, none or edited.
    Exif,
    /// Bytes after `IEND` that are not a chunk.
    Tail,
}

#[derive(Clone, Debug)]
struct Part {
    name: String,
    bytes: Vec<u8>,
    role: Role,
    /// After `IEND`: goes whole with the trailer under `OtherSegment`.
    trailer: bool,
}

fn part(name: &str, bytes: Vec<u8>, role: Role) -> Part {
    Part {
        name: name.to_string(),
        bytes,
        role,
        trailer: false,
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

/// DEC-021 clause 4: a carrier goes with any tag category, `Xmp` or `Iptc`, and
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

/// DEC-021 clause 3.
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

/// Whether the policy removes `p` by its own rule (clauses 2 to 4), apart from
/// the trailer rule. An `Exif` part is only whole-removed in `Whole` mode (a
/// partial edit is checked apart).
fn removed_own(p: &Part, spec: StripSpec) -> bool {
    match p.role {
        Role::Keep | Role::Tail => false,
        Role::Cat(c) => spec.contains(c),
        Role::Carrier => carrier_removed(spec),
        Role::Exif => exif_mode(spec) == ExifMode::Whole,
    }
}

/// Whether the model removes `p` whole under `spec`. The bytes after `IEND` are
/// the audit's `Trailer`, `OtherSegment`: with that category everything after
/// `IEND` goes.
fn removed(p: &Part, spec: StripSpec) -> bool {
    (p.trailer && spec.contains(Category::OtherSegment)) || removed_own(p, spec)
}

/// The first part after `IEND` that the policy keeps by its own rule but that
/// has findings of its own, when the trailer goes whole: the strip would cut it
/// (`Overlap`). Offset in the input.
fn expected_overlap(parts: &[Part], spec: StripSpec) -> Option<usize> {
    if !spec.contains(Category::OtherSegment) {
        return None;
    }
    let mut at = SIG.len();
    for p in parts {
        if p.trailer && !matches!(p.role, Role::Keep | Role::Tail) && !removed_own(p, spec) {
            return Some(at);
        }
        at += p.bytes.len();
    }
    None
}

fn assemble(parts: &[Part]) -> Vec<u8> {
    let mut v = SIG.to_vec();
    for p in parts {
        v.extend_from_slice(&p.bytes);
    }
    v
}

fn expected(parts: &[Part], spec: StripSpec) -> Vec<u8> {
    let mut v = SIG.to_vec();
    for p in parts.iter().filter(|p| !removed(p, spec)) {
        v.extend_from_slice(&p.bytes);
    }
    v
}

/// The removal ranges the model predicts, in input coordinates: one per removed
/// chunk, or one for the whole trailer when `OtherSegment` is in the policy.
fn expected_removals(parts: &[Part], spec: StripSpec) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut at = SIG.len();
    let mut trailer_start: Option<usize> = None;
    for p in parts {
        if p.trailer && trailer_start.is_none() {
            trailer_start = Some(at);
        }
        let end = at + p.bytes.len();
        let in_gone_trailer = p.trailer && spec.contains(Category::OtherSegment);
        if !in_gone_trailer && removed(p, spec) {
            out.push(at..end);
        }
        at = end;
    }
    if let Some(start) = trailer_start {
        if spec.contains(Category::OtherSegment) {
            out.push(start..at);
        }
    }
    out
}

// ── fixtures ────────────────────────────────────────────────────────────────

/// A big-endian TIFF with Make, Orientation and a GPS IFD (latitude).
fn exif_gps() -> Vec<u8> {
    let mut d = b"MM\x00\x2A\x00\x00\x00\x08".to_vec();
    // IFD0 at 8: three entries, next IFD 0. Ends at 50.
    d.extend_from_slice(&[0, 3]);
    d.extend_from_slice(&[0x01, 0x0F, 0, 2, 0, 0, 0, 4]);
    d.extend_from_slice(b"abc\0");
    d.extend_from_slice(&[0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 1, 0, 0]);
    d.extend_from_slice(&[0x88, 0x25, 0, 4, 0, 0, 0, 1, 0, 0, 0, 50]);
    d.extend_from_slice(&[0, 0, 0, 0]);
    // GPS IFD at 50: two entries, next 0. Ends at 80; the latitude at 80.
    d.extend_from_slice(&[0, 2]);
    d.extend_from_slice(&[0, 1, 0, 2, 0, 0, 0, 2, b'N', 0, 0, 0]);
    d.extend_from_slice(&[0, 2, 0, 5, 0, 0, 0, 3, 0, 0, 0, 80]);
    d.extend_from_slice(&[0, 0, 0, 0]);
    for (n, den) in [(51u32, 1u32), (30, 1), (0, 1)] {
        d.extend_from_slice(&n.to_be_bytes());
        d.extend_from_slice(&den.to_be_bytes());
    }
    assert_eq!(d.len(), 104);
    d
}

const XMP: &[u8] = b"<x:xmpmeta/>";

fn text_parts() -> Vec<Part> {
    use Category as C;
    vec![
        part(
            "author",
            text(b"Author", b"canary"),
            Role::Cat(C::Authorship),
        ),
        part(
            "copyright",
            text(b"Copyright", b"canary"),
            Role::Cat(C::Authorship),
        ),
        part(
            "creation",
            text(b"Creation Time", b"canary"),
            Role::Cat(C::Timestamps),
        ),
        part(
            "software",
            text(b"Software", b"canary"),
            Role::Cat(C::Software),
        ),
        part("title", text(b"Title", b"canary"), Role::Cat(C::FreeText)),
        part(
            "ztxt",
            ztxt(b"Comment", b"\x78\x01canary"),
            Role::Cat(C::FreeText),
        ),
        part(
            "itxt",
            itxt(b"Description", 0, b"", b"", b"canary"),
            Role::Cat(C::FreeText),
        ),
        part(
            "itxt-z",
            itxt(b"Description", 1, b"", b"", b"\x78\x01c"),
            Role::Cat(C::FreeText),
        ),
        part(
            "xmp-itxt",
            itxt(b"XML:com.adobe.xmp", 0, b"", b"", XMP),
            Role::Cat(C::Xmp),
        ),
        part(
            "xmp-text",
            text(b"XML:com.adobe.xmp", XMP),
            Role::Cat(C::Xmp),
        ),
        part("empty-text", text(b"Title", b""), Role::Cat(C::FreeText)),
        part("iccp", iccp(b"canary", 0, b"\x78\x01xx"), Role::Cat(C::Icc)),
        part("iccp-empty", iccp(b"canary", 0, b""), Role::Cat(C::Icc)),
        part(
            "time",
            chunk(b"tIME", &[7, 228, 1, 2, 3, 4, 5]),
            Role::Cat(C::Timestamps),
        ),
        part("time-0", chunk(b"tIME", &[]), Role::Cat(C::Timestamps)),
        part(
            "time-12",
            chunk(b"tIME", &[7, 228, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            Role::Cat(C::Timestamps),
        ),
        part(
            "jumbf",
            chunk(b"caBX", b"\0\0\0\x08jumb"),
            Role::Cat(C::Jumbf),
        ),
        part(
            "other-chunk",
            chunk(b"dSIG", b"canary"),
            Role::Cat(C::OtherSegment),
        ),
    ]
}

/// The carriers measured on ExifTool 13.55 (SPEC-037 Part A step 1): the chunk
/// types ExifTool decodes into Exif or XMP tags that the audit does not list.
const CARRIER_TYPES: [&[u8; 4]; 9] = [
    b"zxIf", b"zxif", b"exif", b"ZXIF", b"EXIF", b"tXMP", b"meTa", b"seAl", b"eXIf",
];

/// The non-metadata carriers: `OtherSegment`, removed only with that category.
const OTHER_TYPES: [&[u8; 4]; 6] = [b"dSIG", b"cpIp", b"gIFx", b"gIFg", b"gIFt", b"fRAc"];

fn carrier_parts() -> Vec<Part> {
    let mut v = Vec::new();
    for ty in CARRIER_TYPES.iter().filter(|t| ***t != *b"eXIf") {
        let name = String::from_utf8_lossy(&ty[..]).to_string();
        v.push(part(&name, chunk(ty, b"canary payload"), Role::Carrier));
    }
    for name in ["exif", "APP1", "xmp", "iptc", "icc", "8bim", "generic"] {
        let kw = format!("Raw profile type {name}");
        v.push(part(
            &format!("raw-text-{name}"),
            text(kw.as_bytes(), b"\n4\n68\n"),
            Role::Carrier,
        ));
        v.push(part(
            &format!("raw-ztxt-{name}"),
            ztxt(kw.as_bytes(), b"\x78\x01x"),
            Role::Carrier,
        ));
        v.push(part(
            &format!("raw-itxt-{name}"),
            itxt(kw.as_bytes(), 0, b"", b"", b"\n4\n"),
            Role::Carrier,
        ));
    }
    v.push(part(
        "raw-lower",
        text(b"raw profile type exif", b"x"),
        Role::Carrier,
    ));
    // Not a raw profile: only an ordinary text tag.
    v.push(part(
        "raw-wrong-case",
        text(b"Raw Profile Type exif", b"x"),
        Role::Cat(Category::FreeText),
    ));
    for ty in OTHER_TYPES {
        let name = String::from_utf8_lossy(&ty[..]).to_string();
        v.push(part(
            &name,
            chunk(ty, b"canary payload"),
            Role::Cat(Category::OtherSegment),
        ));
    }
    v
}

fn structure() -> Vec<Part> {
    vec![
        part(
            "pHYs",
            chunk(b"pHYs", &[0, 0, 0xB, 0x13, 0, 0, 0xB, 0x13, 1]),
            Role::Keep,
        ),
        part("gAMA", chunk(b"gAMA", &[0, 1, 0xB1, 0x8F]), Role::Keep),
        part("unknown", chunk(b"abCd", b"canary"), Role::Keep),
        part("iDOT", chunk(b"iDOT", &[0; 28]), Role::Keep),
        part("pCAL", chunk(b"pCAL", b"x"), Role::Keep),
    ]
}

/// IHDR, `pre`, IDAT, `post`, IEND, then `trail` (chunks) and `tail` (bytes).
fn shape(pre: &[Part], post: &[Part], trail: &[Part], tail: &[u8]) -> Vec<Part> {
    let mut v = vec![part("IHDR", ihdr(), Role::Keep)];
    v.extend(pre.iter().cloned());
    v.push(part("IDAT", idat(), Role::Keep));
    v.extend(post.iter().cloned());
    v.push(part("IEND", iend(), Role::Keep));
    for t in trail {
        let mut t = t.clone();
        t.trailer = true;
        v.push(t);
    }
    if !tail.is_empty() {
        let mut t = part("tail", tail.to_vec(), Role::Tail);
        t.trailer = true;
        v.push(t);
    }
    v
}

fn shapes() -> Vec<(String, Vec<Part>)> {
    let all: Vec<Part> = text_parts()
        .into_iter()
        .chain(carrier_parts())
        .chain(structure())
        .collect();
    let exif_whole = |name: &str| part(name, exif_chunk(&exif_gps()), Role::Exif);
    let mut out: Vec<(String, Vec<Part>)> = vec![
        ("minimal".into(), shape(&[], &[], &[], b"")),
        ("everything-before-idat".into(), shape(&all, &[], &[], b"")),
        ("everything-after-idat".into(), shape(&[], &all, &[], b"")),
        ("everything-after-iend".into(), shape(&[], &[], &all, b"")),
        (
            "everything-after-iend-tail".into(),
            shape(&[], &[], &all, b"junk bytes"),
        ),
        (
            "everything-everywhere".into(),
            shape(&all, &all, &all, b"\0\0\0"),
        ),
        ("tail-only".into(), shape(&[], &[], &[], b"trailing bytes")),
        (
            "exif-three-places".into(),
            shape(
                &[exif_whole("exif-pre")],
                &[exif_whole("exif-post")],
                &[exif_whole("exif-trailer")],
                b"",
            ),
        ),
        (
            "exif-and-text".into(),
            shape(
                &[
                    exif_whole("exif"),
                    part(
                        "author",
                        text(b"Author", b"canary"),
                        Role::Cat(Category::Authorship),
                    ),
                ],
                &[],
                &[],
                b"",
            ),
        ),
    ];
    // One chunk kind at a time, in each position.
    for p in &all {
        for (place, pre, post, trail) in [
            ("pre", vec![p.clone()], vec![], vec![]),
            ("post", vec![], vec![p.clone()], vec![]),
            ("trailer", vec![], vec![], vec![p.clone()]),
        ] {
            out.push((
                format!("{}-{place}", p.name),
                shape(&pre, &post, &trail, b""),
            ));
        }
    }
    out
}

fn policies() -> Vec<(&'static str, StripPolicy)> {
    vec![
        ("gps", StripPolicy::Gps),
        ("gps_and_device", StripPolicy::GpsAndDevice),
        ("all_pii", StripPolicy::AllPii),
        ("all", StripPolicy::All),
        ("everything", StripPolicy::Everything),
        ("none", StripPolicy::Custom(StripSpec::none())),
        ("icc", StripPolicy::Custom(spec_of(&[Category::Icc]))),
        ("xmp", StripPolicy::Custom(spec_of(&[Category::Xmp]))),
        ("jumbf", StripPolicy::Custom(spec_of(&[Category::Jumbf]))),
        (
            "other",
            StripPolicy::Custom(spec_of(&[Category::OtherSegment])),
        ),
        (
            "time",
            StripPolicy::Custom(spec_of(&[Category::Timestamps])),
        ),
        (
            "icc_other",
            StripPolicy::Custom(spec_of(&[Category::Icc, Category::OtherSegment])),
        ),
    ]
}

// ── the strip against the model ─────────────────────────────────────────────

/// Pixel integrity (I1) and every category rule: the output is exactly the input
/// with the model's removals deleted, for every shape under every policy; the
/// output is no longer than the input; stripping it again changes nothing; and
/// the removal records are the model's ranges.
#[test]
fn every_shape_under_every_policy_is_the_input_minus_the_model_removals() {
    for (name, parts) in shapes() {
        let input = assemble(&parts);
        for (pname, policy) in policies() {
            let spec = policy.spec();
            let ctx = format!("{name} under {pname}");
            if let Some(offset) = expected_overlap(&parts, spec) {
                assert_eq!(
                    strip(&input, policy).unwrap_err(),
                    StripError::Overlap { offset },
                    "{ctx}"
                );
                continue;
            }
            let out = strip(&input, policy).unwrap_or_else(|e| panic!("{ctx}: {e:?}"));
            // The model's `Exif` parts are whole or untouched here (the policy
            // is never partial for an `Exif` part in a shape with Make and GPS
            // unless the policy is partial: handled in its own test).
            let has_exif = parts.iter().any(|p| p.role == Role::Exif);
            if has_exif && exif_mode(spec) == ExifMode::Partial {
                continue;
            }
            assert_eq!(out.bytes(), expected(&parts, spec).as_slice(), "{ctx}");
            assert!(out.bytes().len() <= input.len(), "{ctx}");
            let again = strip(out.bytes(), policy).unwrap_or_else(|e| panic!("{ctx} again: {e:?}"));
            assert_eq!(again.bytes(), out.bytes(), "{ctx}: idempotent");
            assert!(again.removals().is_empty(), "{ctx}: nothing left to remove");
            let got: Vec<Range<usize>> = out
                .removals()
                .iter()
                .map(|r| r.offset..r.offset + r.len)
                .collect();
            assert_eq!(
                got,
                expected_removals(&parts, spec),
                "{ctx}: removal ranges"
            );
            // What the audit finds in the output is outside the policy.
            for f in audit(out.bytes()).findings() {
                assert!(!spec.contains(f.category), "{ctx}: {:?} left", f.category);
            }
            // `sanitize` is the same bytes.
            assert_eq!(sanitize(&input, policy).unwrap(), out.bytes(), "{ctx}");
        }
    }
}

/// The kept chunks come out in order and byte for byte, `IHDR`, `PLTE`, `IDAT`
/// and every unknown chunk included (the I1 statement for PNG).
#[test]
fn structure_chunks_are_untouched_in_order() {
    let all: Vec<Part> = text_parts().into_iter().chain(structure()).collect();
    let parts = shape(&all, &all, &all, b"tail");
    let input = assemble(&parts);
    let out = strip(&input, StripPolicy::Everything).unwrap();
    let mut want = SIG.to_vec();
    for p in parts.iter().filter(|p| matches!(p.role, Role::Keep)) {
        if !p.trailer {
            want.extend_from_slice(&p.bytes);
        }
    }
    assert_eq!(out.bytes(), want.as_slice());
}

#[test]
fn removal_kinds_and_categories_are_recorded() {
    let parts = shape(
        &[
            part(
                "author",
                text(b"Author", b"x"),
                Role::Cat(Category::Authorship),
            ),
            part(
                "time",
                chunk(b"tIME", &[0; 7]),
                Role::Cat(Category::Timestamps),
            ),
            part(
                "xmp",
                itxt(b"XML:com.adobe.xmp", 0, b"", b"", XMP),
                Role::Cat(Category::Xmp),
            ),
            part("icc", iccp(b"n", 0, b"x"), Role::Cat(Category::Icc)),
            part("raw", text(b"Raw profile type exif", b"x"), Role::Carrier),
            part("zxIf", chunk(b"zxIf", b"x"), Role::Carrier),
            part(
                "dSIG",
                chunk(b"dSIG", b"x"),
                Role::Cat(Category::OtherSegment),
            ),
            part("exif", exif_chunk(&exif_gps()), Role::Exif),
            part("jumbf", chunk(b"caBX", b"x"), Role::Cat(Category::Jumbf)),
        ],
        &[],
        &[],
        b"",
    );
    let out = strip(&assemble(&parts), StripPolicy::Everything).unwrap();
    let got: Vec<(RemovalKind, Category)> = out
        .removals()
        .iter()
        .map(|r| (r.kind, r.category))
        .collect();
    assert_eq!(
        got,
        vec![
            (RemovalKind::Text, Category::Authorship),
            (RemovalKind::Time, Category::Timestamps),
            (RemovalKind::Xmp, Category::Xmp),
            (RemovalKind::Icc, Category::Icc),
            (RemovalKind::RawProfile, Category::OtherSegment),
            (RemovalKind::Carrier, Category::OtherSegment),
            (RemovalKind::OtherChunk, Category::OtherSegment),
            (RemovalKind::Exif, Category::CameraModel),
            (RemovalKind::Jumbf, Category::Jumbf),
        ]
    );
}

// ── DEC-021 clause 2: categories by keyword ─────────────────────────────────

#[test]
fn a_policy_without_a_category_leaves_its_chunks() {
    // `Gps` keeps XMP and ICC, as for JPEG; `AllPii` keeps XMP and ICC and
    // Software; `All` keeps ICC and Jumbf.
    let parts = shape(&text_parts(), &[], &[], b"");
    let input = assemble(&parts);
    let name_left = |policy: StripPolicy| -> Vec<String> {
        let out = strip(&input, policy).unwrap();
        parts
            .iter()
            .filter(|p| {
                out.bytes()
                    .windows(p.bytes.len())
                    .any(|w| w == p.bytes.as_slice())
            })
            .map(|p| p.name.clone())
            .collect()
    };
    let gps = name_left(StripPolicy::Gps);
    for kept in [
        "xmp-itxt", "xmp-text", "iccp", "author", "time", "software", "title",
    ] {
        assert!(gps.iter().any(|n| n == kept), "Gps keeps {kept}");
    }
    let all_pii = name_left(StripPolicy::AllPii);
    assert!(
        all_pii.iter().any(|n| n == "software"),
        "AllPii keeps Software"
    );
    assert!(all_pii.iter().any(|n| n == "xmp-itxt"), "AllPii keeps XMP");
    assert!(!all_pii.iter().any(|n| n == "author"));
    let all = name_left(StripPolicy::All);
    assert!(all.iter().any(|n| n == "iccp"), "All keeps ICC");
    assert!(all.iter().any(|n| n == "jumbf"), "All keeps Jumbf");
    assert!(!all.iter().any(|n| n == "software" || n == "xmp-itxt"));
    let everything = name_left(StripPolicy::Everything);
    for kept in ["IHDR", "IDAT", "IEND"] {
        assert!(everything.iter().any(|n| n == kept));
    }
    for gone in ["author", "iccp", "time", "jumbf", "other-chunk", "xmp-text"] {
        assert!(
            !everything.iter().any(|n| n == gone),
            "Everything removes {gone}"
        );
    }
}

// ── DEC-021 clause 3: eXIf ──────────────────────────────────────────────────

fn with_exif(e: Vec<u8>) -> Vec<u8> {
    let parts = shape(&[part("exif", exif_chunk(&e), Role::Exif)], &[], &[], b"");
    assemble(&parts)
}

/// The data of the first `eXIf` chunk of `png`, and whether its CRC is right.
fn exif_chunks_of(png: &[u8]) -> Vec<(Vec<u8>, bool)> {
    let mut out = Vec::new();
    let mut at = 8usize;
    while at + 12 <= png.len() {
        let len = u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]) as usize;
        let ty = &png[at + 4..at + 8];
        if ty == b"eXIf" {
            let data = png[at + 8..at + 8 + len].to_vec();
            let stored = u32::from_be_bytes([
                png[at + 8 + len],
                png[at + 9 + len],
                png[at + 10 + len],
                png[at + 11 + len],
            ]);
            out.push((data, crc32(&png[at + 4..at + 8 + len]) == stored));
        }
        at += 12 + len;
    }
    out
}

#[test]
fn a_partial_policy_edits_the_entries_and_patches_the_crc() {
    let input = with_exif(exif_gps());
    let before = audit(&input);
    assert!(before
        .findings()
        .iter()
        .any(|f| f.category == Category::Gps));
    assert!(before
        .findings()
        .iter()
        .any(|f| f.category == Category::CameraModel));
    for (policy, gps_left, make_left) in [
        (StripPolicy::Gps, false, true),
        (StripPolicy::GpsAndDevice, false, true),
        (StripPolicy::AllPii, false, false),
    ] {
        let out = strip(&input, policy).unwrap();
        let chunks = exif_chunks_of(out.bytes());
        assert_eq!(
            chunks.len(),
            1,
            "{policy:?}: the chunk is edited, not removed"
        );
        assert!(chunks[0].1, "{policy:?}: the chunk CRC is valid");
        assert_eq!(out.bytes().len(), input.len(), "{policy:?}: same length");
        assert_ne!(out.bytes(), input.as_slice());
        let after = audit(out.bytes());
        assert_eq!(
            after.findings().iter().any(|f| f.category == Category::Gps),
            gps_left,
            "{policy:?}"
        );
        assert_eq!(
            after
                .findings()
                .iter()
                .any(|f| f.category == Category::CameraModel),
            make_left,
            "{policy:?}"
        );
        // A CRC patch is recorded, and the zeroed ranges read zero.
        assert!(
            out.patches().iter().any(|p| p.kind == PatchKind::ChunkCrc),
            "{policy:?}"
        );
        for s in out.scrubs() {
            assert!(out.bytes()[s.offset..s.offset + s.len]
                .iter()
                .all(|b| *b == 0));
        }
        assert!(!out.entries().is_empty());
        // Outside the eXIf chunk the output is the input.
        let strip_exif = |b: &[u8]| -> Vec<u8> {
            let start = 8 + 25;
            let end = start + 12 + exif_gps().len();
            [&b[..start], &b[end..]].concat()
        };
        assert_eq!(strip_exif(out.bytes()), strip_exif(&input), "{policy:?}");
        // And the edited chunk is the input's chunk with only its CRC and the
        // patched ranges different.
        let again = strip(out.bytes(), policy).unwrap();
        assert_eq!(again.bytes(), out.bytes(), "{policy:?}: idempotent");
    }
}

#[test]
fn a_partial_edit_works_in_every_position_and_with_several_chunks() {
    let e = exif_chunk(&exif_gps());
    let parts = shape(
        &[part("a", e.clone(), Role::Exif)],
        &[part("b", e.clone(), Role::Exif)],
        &[part("c", e.clone(), Role::Exif)],
        b"",
    );
    let input = assemble(&parts);
    let out = strip(&input, StripPolicy::Gps).unwrap();
    let chunks = exif_chunks_of(out.bytes());
    assert_eq!(chunks.len(), 3);
    assert!(chunks.iter().all(|c| c.1), "every chunk CRC is valid");
    assert!(!audit(out.bytes())
        .findings()
        .iter()
        .any(|f| f.category == Category::Gps));
    // With `OtherSegment` too the trailer goes whole, and an `eXIf` in it that
    // the policy would only edit would be cut with it: `Overlap`.
    let both = StripPolicy::Custom(spec_of(&[Category::Gps, Category::OtherSegment]));
    assert!(matches!(
        strip(&input, both),
        Err(StripError::Overlap { .. })
    ));
}

#[test]
fn an_all_category_policy_removes_every_exif_chunk_and_none_leaves_them() {
    let e = exif_chunk(&exif_gps());
    let parts = shape(
        &[
            part("a", e.clone(), Role::Exif),
            part("b", e.clone(), Role::Exif),
        ],
        &[part("c", e.clone(), Role::Exif)],
        &[part("d", e.clone(), Role::Exif)],
        b"",
    );
    let input = assemble(&parts);
    for policy in [StripPolicy::All, StripPolicy::Everything] {
        let out = strip(&input, policy).unwrap();
        assert!(exif_chunks_of(out.bytes()).is_empty(), "{policy:?}");
    }
    // 17 chunks: more than the reader parses; still all removed whole.
    let many: Vec<Part> = (0..17)
        .map(|i| part(&format!("e{i}"), e.clone(), Role::Exif))
        .collect();
    let input = assemble(&shape(&many, &[], &[], b""));
    let out = strip(&input, StripPolicy::Everything).unwrap();
    assert!(exif_chunks_of(out.bytes()).is_empty());
    // ... and refused under a partial policy.
    assert!(matches!(
        strip(&input, StripPolicy::Gps),
        Err(StripError::Uneditable {
            reason: UneditableReason::TooManyExifSegments,
            ..
        })
    ));
    // A policy with no category an Exif entry can have leaves them.
    for policy in [
        StripPolicy::Custom(StripSpec::none()),
        StripPolicy::Custom(spec_of(&[Category::OtherSegment])),
        StripPolicy::Custom(spec_of(&[Category::Jumbf])),
    ] {
        let input = assemble(&shape(&[part("a", e.clone(), Role::Exif)], &[], &[], b""));
        let out = strip(&input, policy).unwrap();
        assert_eq!(exif_chunks_of(out.bytes()).len(), 1, "{policy:?}");
    }
}

#[test]
fn an_unparseable_exif_is_removed_whole_or_refused_when_a_partial_edit_is_needed() {
    let input = with_exif(b"not a tiff at all".to_vec());
    for policy in [StripPolicy::All, StripPolicy::Everything] {
        let out = strip(&input, policy).unwrap();
        assert!(exif_chunks_of(out.bytes()).is_empty(), "{policy:?}");
    }
    for policy in [
        StripPolicy::Gps,
        StripPolicy::GpsAndDevice,
        StripPolicy::AllPii,
    ] {
        let e = strip(&input, policy).unwrap_err();
        assert!(
            matches!(
                e,
                StripError::Uneditable {
                    reason: UneditableReason::BlockNotClean,
                    offset: 33
                }
            ),
            "{policy:?}: {e:?}"
        );
    }
    // No category an entry can have: left as it is.
    let out = strip(&input, StripPolicy::Custom(spec_of(&[Category::Jumbf]))).unwrap();
    assert_eq!(out.bytes(), input.as_slice());
}

#[test]
fn an_exif_chunk_with_the_improper_header_is_still_edited() {
    let mut payload = b"Exif\0\0".to_vec();
    payload.extend_from_slice(&exif_gps());
    let input = with_exif(payload);
    let out = strip(&input, StripPolicy::Gps).unwrap();
    let chunks = exif_chunks_of(out.bytes());
    assert!(chunks[0].1);
    assert!(!audit(out.bytes())
        .findings()
        .iter()
        .any(|f| f.category == Category::Gps));
}

// ── DEC-021 clause 4: opaque carriers ───────────────────────────────────────

/// The measured carriers go with every preset, and with any `Custom` policy that
/// has a tag category, `Xmp`, `Iptc` or `OtherSegment`; they stay under a policy
/// of `Icc`, `Jumbf` or nothing. The non-metadata ones go only with
/// `OtherSegment`.
#[test]
fn carriers_follow_the_clause_4_rule_for_every_policy() {
    let carriers: Vec<Part> = carrier_parts()
        .into_iter()
        .filter(|p| p.role == Role::Carrier)
        .collect();
    assert!(carriers.len() >= 7 + 3 * 7);
    for p in &carriers {
        let input = assemble(&shape(std::slice::from_ref(p), &[], &[], b""));
        for policy in StripPolicy::PRESETS {
            let out = strip(&input, policy).unwrap();
            assert_eq!(
                out.bytes(),
                expected(
                    &shape(std::slice::from_ref(p), &[], &[], b""),
                    policy.spec()
                )
                .as_slice()
            );
            assert!(
                out.removals()
                    .iter()
                    .any(|r| matches!(r.kind, RemovalKind::Carrier | RemovalKind::RawProfile)),
                "{} under {policy:?}",
                p.name
            );
        }
        for (cats, goes) in [
            (vec![Category::Icc], false),
            (vec![Category::Jumbf], false),
            (vec![Category::Thumbnail], true),
            (vec![Category::Iptc], true),
            (vec![Category::Xmp], true),
            (vec![Category::OtherSegment], true),
        ] {
            let spec = spec_of(&cats);
            let out = strip(&input, StripPolicy::Custom(spec)).unwrap();
            let gone = out.bytes().len() < input.len();
            assert_eq!(gone, goes, "{} under {cats:?}", p.name);
        }
    }
    for ty in OTHER_TYPES {
        let p = part("o", chunk(ty, b"canary"), Role::Cat(Category::OtherSegment));
        let input = assemble(&shape(&[p], &[], &[], b""));
        for policy in StripPolicy::PRESETS {
            let out = strip(&input, policy).unwrap();
            let gone = out.bytes().len() < input.len();
            assert_eq!(
                gone,
                policy == StripPolicy::Everything,
                "{ty:?} under {policy:?}"
            );
        }
    }
}

/// Adler-32, for a zlib stream built by hand.
fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// `\0`, a 4-byte length and a zlib stream (stored block) around `tiff`: the
/// second encoding ExifTool decodes in a `zxIf` chunk.
fn zlib_wrapped(tiff: &[u8]) -> Vec<u8> {
    let mut z = vec![0x78, 0x01, 0x01];
    z.extend_from_slice(&(tiff.len() as u16).to_le_bytes());
    z.extend_from_slice(&(!(tiff.len() as u16)).to_le_bytes());
    z.extend_from_slice(tiff);
    z.extend_from_slice(&adler32(tiff).to_be_bytes());
    let mut d = vec![0];
    d.extend_from_slice(&(tiff.len() as u32).to_be_bytes());
    d.extend_from_slice(&z);
    d
}

/// Every chunk type of a PNG, in order (framing only; the output of a strip is
/// well formed).
fn chunk_types(png: &[u8]) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    let mut at = 8;
    while at + 12 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        out.push(png[at + 4..at + 8].try_into().unwrap());
        at += 12 + len;
    }
    out
}

/// The 16 ASCII-case spellings of a four-letter chunk type.
fn case_spellings(base: [u8; 4]) -> Vec<[u8; 4]> {
    (0..16u8)
        .map(|mask| {
            let mut ty = base;
            for (i, b) in ty.iter_mut().enumerate() {
                if (mask >> i) & 1 == 1 {
                    *b ^= 0x20;
                }
            }
            ty
        })
        .collect()
}

/// H1 (SPEC-037 verify): ExifTool 13.55 matches the chunk types `eXIf` and
/// `zxIf` in any ASCII case (every spelling decodes, as a bare TIFF or as `\0`,
/// a length and a zlib stream), so every spelling but the exact `eXIf` (the
/// walked Exif chunk, covered by its own tests) is a carrier, removed by every
/// preset and reported as such. Types other than those two stay case-sensitive.
#[test]
fn every_case_spelling_of_exif_and_zxif_is_a_carrier_removed_by_every_preset() {
    let tiff = exif_gps();
    let encodings: [(&str, Vec<u8>); 2] = [("bare", tiff.clone()), ("zlib", zlib_wrapped(&tiff))];
    let mut spellings = 0;
    for base in [*b"eXIf", *b"zxIf"] {
        for ty in case_spellings(base) {
            if &ty == b"eXIf" {
                continue;
            }
            spellings += 1;
            for (enc, data) in &encodings {
                let name = format!("{}-{enc}", String::from_utf8_lossy(&ty));
                let parts = shape(
                    &[part(&name, chunk(&ty, data), Role::Carrier)],
                    &[],
                    &[],
                    b"",
                );
                let input = assemble(&parts);
                assert!(
                    audit(&input).items().iter().any(|i| matches!(
                        i.kind(),
                        ItemKind::Region {
                            kind: RegionKind::Carrier,
                            ..
                        }
                    )),
                    "{name} is not a Carrier region"
                );
                for policy in StripPolicy::PRESETS {
                    let out = strip(&input, policy).unwrap();
                    assert_eq!(
                        out.bytes(),
                        expected(&parts, policy.spec()).as_slice(),
                        "{name} under {policy:?}"
                    );
                    assert!(
                        !chunk_types(out.bytes()).iter().any(|t| {
                            t.eq_ignore_ascii_case(b"exif") || t.eq_ignore_ascii_case(b"zxif")
                        }),
                        "{name} survives {policy:?}"
                    );
                    assert!(out
                        .removals()
                        .iter()
                        .any(|r| matches!(r.kind, RemovalKind::Carrier)));
                }
            }
        }
    }
    // 2 types x 16 spellings, less the exact `eXIf`.
    assert_eq!(spellings, 31);
    // Case stays significant for every other type.
    let lower = assemble(&shape(
        &[part("txmp", chunk(b"txmp", b"x"), Role::Keep)],
        &[],
        &[],
        b"",
    ));
    assert_eq!(
        strip(&lower, StripPolicy::Everything).unwrap().bytes(),
        lower
    );
}

/// The self-check exempts a removed carrier from the category rule: the output
/// of `Gps` has no raw profile although `OtherSegment` is not in the policy, and
/// its audit has no `RawProfile` or `Carrier` region.
#[test]
fn the_output_audit_has_no_carrier_the_policy_removes() {
    let parts = shape(
        &[
            part("raw", text(b"Raw profile type exif", b"x"), Role::Carrier),
            part("zxIf", chunk(b"zxIf", b"canary"), Role::Carrier),
        ],
        &[],
        &[],
        b"",
    );
    let input = assemble(&parts);
    let before = audit(&input);
    assert!(before.items().iter().any(|i| matches!(
        i.kind(),
        ItemKind::Region {
            kind: RegionKind::RawProfile,
            ..
        }
    )));
    assert!(before.items().iter().any(|i| matches!(
        i.kind(),
        ItemKind::Region {
            kind: RegionKind::Carrier,
            ..
        }
    )));
    let out = strip(&input, StripPolicy::Gps).unwrap();
    let after = audit(out.bytes());
    assert!(!after.items().iter().any(|i| matches!(
        i.kind(),
        ItemKind::Region {
            kind: RegionKind::RawProfile | RegionKind::Carrier,
            ..
        }
    )));
}

// ── DEC-021 clause 5: refusals and non-refusals ─────────────────────────────

fn minimal_parts(extra: &[Vec<u8>]) -> Vec<u8> {
    let mut parts = vec![ihdr()];
    parts.extend(extra.iter().cloned());
    parts.push(idat());
    parts.push(iend());
    png_shapes::png(&parts)
}

#[test]
fn a_halted_walk_is_refused_with_its_offset() {
    // The chunk cap: 65_536 chunks are recorded, the next halts the walk.
    let mut parts = vec![ihdr()];
    for _ in 0..65_536 {
        parts.push(chunk(b"abCd", b""));
    }
    parts.push(iend());
    let input = png_shapes::png(&parts);
    for policy in StripPolicy::PRESETS {
        let e = strip(&input, policy).unwrap_err();
        assert!(
            matches!(
                e,
                StripError::Unstrippable {
                    reason: UnstrippableReason::TooManySegments,
                    ..
                }
            ),
            "{e:?}"
        );
    }
    // The issue cap: more than 1024 bad CRCs.
    let mut parts = vec![ihdr()];
    for _ in 0..1030 {
        parts.push(chunk_with_crc(b"abCd", b"", 1));
    }
    parts.push(iend());
    let e = strip(&png_shapes::png(&parts), StripPolicy::Everything).unwrap_err();
    assert!(matches!(
        e,
        StripError::Unstrippable {
            reason: UnstrippableReason::TooManyIssues,
            ..
        }
    ));
    // A chunk whose length runs past the end of the input.
    let mut input = minimal_parts(&[]);
    let at = input.len();
    input.extend_from_slice(&raw_chunk(1000, b"tEXt", b"x", 0)[..]);
    // (This one is after `IEND`, so it is the trailer, not a halt.)
    assert!(strip(&input, StripPolicy::AllPii).is_ok());
    // Before `IEND` it halts the walk.
    let mut parts = vec![
        ihdr(),
        text(b"Author", b"x"),
        raw_chunk(1000, b"tEXt", b"x", 0),
    ];
    let input = png_shapes::png(&parts);
    let e = strip(&input, StripPolicy::Everything).unwrap_err();
    assert_eq!(
        e,
        StripError::Unstrippable {
            reason: UnstrippableReason::ChunkPastEnd,
            offset: 8 + 25 + text(b"Author", b"x").len()
        }
    );
    // Fewer than twelve bytes left where a chunk should start.
    parts = vec![ihdr(), idat()];
    let mut input = png_shapes::png(&parts);
    input.extend_from_slice(b"short");
    let e = strip(&input, StripPolicy::Everything).unwrap_err();
    assert!(matches!(
        e,
        StripError::Unstrippable {
            reason: UnstrippableReason::ChunkPastEnd,
            ..
        }
    ));
    // A length word above 2^31 - 1.
    let parts = vec![ihdr(), raw_chunk(0x8000_0000, b"tEXt", b"", 0)];
    let e = strip(&png_shapes::png(&parts), StripPolicy::Everything).unwrap_err();
    assert!(matches!(
        e,
        StripError::Unstrippable {
            reason: UnstrippableReason::BadSegmentLength,
            ..
        }
    ));
    let _ = at;
}

#[test]
fn a_missing_iend_and_bad_crcs_are_not_refusals() {
    // No IEND: the file ends on a chunk boundary.
    let mut parts = vec![ihdr(), text(b"Author", b"canary"), idat()];
    let input = png_shapes::png(&parts);
    let out = strip(&input, StripPolicy::AllPii).unwrap();
    assert_eq!(out.bytes(), png_shapes::png(&[ihdr(), idat()]).as_slice());
    // A bad CRC on a chunk that is removed.
    parts = vec![
        ihdr(),
        chunk_with_crc(b"tEXt", b"Author\0x", 7),
        idat(),
        iend(),
    ];
    let out = strip(&png_shapes::png(&parts), StripPolicy::AllPii).unwrap();
    assert_eq!(out.bytes(), minimal_parts(&[]).as_slice());
    // A bad CRC on kept chunks stays bad, byte for byte: a text chunk the policy
    // leaves, and the image data.
    let bad_text = chunk_with_crc(b"tEXt", b"Author\0x", 7);
    let bad_idat = chunk_with_crc(b"IDAT", &[0x78, 0x01, 0x63, 0x60, 0x00, 0x00], 9);
    parts = vec![
        ihdr(),
        bad_text.clone(),
        bad_idat.clone(),
        text(b"Title", b"y"),
        iend(),
    ];
    let input = png_shapes::png(&parts);
    let out = strip(&input, StripPolicy::Gps).unwrap();
    assert_eq!(out.bytes(), input.as_slice());
    let out = strip(&input, StripPolicy::Custom(spec_of(&[Category::FreeText]))).unwrap();
    assert_eq!(
        out.bytes(),
        png_shapes::png(&[ihdr(), bad_text, bad_idat, iend()]).as_slice()
    );
}

#[test]
fn the_trailer_tail_and_unknown_trailing_chunks_follow_other_segment() {
    let trailing = [chunk(b"abCd", b"x"), b"not a chunk".to_vec()].concat();
    let mut input = minimal_parts(&[]);
    input.extend_from_slice(&trailing);
    // Kept by every preset but `Everything`.
    for policy in [StripPolicy::Gps, StripPolicy::AllPii, StripPolicy::All] {
        assert_eq!(strip(&input, policy).unwrap().bytes(), input.as_slice());
    }
    let out = strip(&input, StripPolicy::Everything).unwrap();
    assert_eq!(out.bytes(), minimal_parts(&[]).as_slice());
    assert_eq!(out.removals().len(), 1);
    assert_eq!(out.removals()[0].kind, RemovalKind::Trailer);
    // A kept trailer shrinks by the chunks the policy removes from it.
    let mut input = minimal_parts(&[]);
    input.extend_from_slice(&text(b"Author", b"x"));
    input.extend_from_slice(b"tail");
    let out = strip(&input, StripPolicy::AllPii).unwrap();
    let mut want = minimal_parts(&[]);
    want.extend_from_slice(b"tail");
    assert_eq!(out.bytes(), want.as_slice());
}

#[test]
fn a_trailer_with_more_chunks_than_the_scan_examines_is_refused_unless_it_goes_whole() {
    let mut input = minimal_parts(&[]);
    for _ in 0..4100 {
        input.extend_from_slice(&chunk(b"abCd", b""));
    }
    let e = strip(&input, StripPolicy::AllPii).unwrap_err();
    assert!(matches!(
        e,
        StripError::Unstrippable {
            reason: UnstrippableReason::TooManySegments,
            ..
        }
    ));
    assert_eq!(
        strip(&input, StripPolicy::Everything).unwrap().bytes(),
        minimal_parts(&[]).as_slice()
    );
}

// ── honesty: the preview still says not strippable ──────────────────────────

#[test]
fn strip_works_while_strippable_is_still_false_for_png() {
    assert!(!ContainerKind::Png.strippable());
    let input = minimal_parts(&[text(b"Author", b"x")]);
    assert!(strip(&input, StripPolicy::AllPii).is_ok());
    assert!(!ContainerKind::Png.strippable());
    // The audit still previews every finding as kept.
    let p = exif_core::audit::preview(&audit(&input), StripPolicy::AllPii);
    assert!(!p.strippable());
}

#[test]
fn unused_check_variants_compile() {
    // Keeps the `Check` import honest: the typed self-check names exist.
    let _ = Check::Preview;
}

// ── SPEC-037 verify punch list: M2 (`caBX`) and L2 (every oracle fixture) ───

fn jumbf_box(ty: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    v.extend_from_slice(ty);
    v.extend_from_slice(body);
    v
}

/// A C2PA-style `caBX`: a `c2pa` manifest box holding an `stds.exif` JSON
/// assertion with a GPS position and a creator (measured on ExifTool 13.55: it
/// prints the GPS and `dc:creator` from such a chunk).
fn c2pa_cabx() -> Vec<u8> {
    let description = |uuid: [u8; 16], label: &[u8]| {
        let mut body = uuid.to_vec();
        body.push(3);
        body.extend_from_slice(label);
        body.push(0);
        jumbf_box(b"jumd", &body)
    };
    let json_uuid = *b"json\0\x11\0\x10\x80\0\0\xAA\0\x38\x9B\x71";
    let c2pa_uuid = *b"c2pa\0\x11\0\x10\x80\0\0\xAA\0\x38\x9B\x71";
    let json = br#"{"exif:GPSLatitude":"51,30.0N","exif:GPSLongitude":"0,7.0W","dc:creator":["Jane Jumbf Secret"]}"#;
    let assertion = [
        description(json_uuid, b"stds.exif"),
        jumbf_box(b"json", json),
    ]
    .concat();
    let manifest = [
        description(c2pa_uuid, b"c2pa"),
        jumbf_box(b"jumb", &assertion),
    ]
    .concat();
    chunk(b"caBX", &jumbf_box(b"jumb", &manifest))
}

/// M2 (SPEC-037 verify), the stated residual: `caBX` is `Jumbf`, as in a JPEG
/// (C2PA provenance). Every preset but `Everything` keeps it, so a C2PA
/// manifest that carries Exif or creator assertions survives them. The preview
/// claims no removal for a PNG yet (`strippable` is false).
#[test]
fn a_cabx_with_exif_assertions_is_kept_by_the_presets_and_removed_by_everything() {
    let parts = shape(
        &[part("c2pa", c2pa_cabx(), Role::Cat(Category::Jumbf))],
        &[],
        &[],
        b"",
    );
    let input = assemble(&parts);
    let a = audit(&input);
    let finding = a
        .findings()
        .iter()
        .position(|f| f.category == Category::Jumbf)
        .expect("a Jumbf finding");
    let want = [
        (StripPolicy::Gps, false),
        (StripPolicy::GpsAndDevice, false),
        (StripPolicy::AllPii, false),
        (StripPolicy::All, false),
        (StripPolicy::Everything, true),
    ];
    for (policy, goes) in want {
        let out = strip(&input, policy).unwrap();
        let has = out.bytes().windows(4).any(|w| w == b"caBX");
        assert_eq!(!has, goes, "{policy:?}");
        // While `strippable` is false for PNG (until SPEC-039) the preview
        // describes no removal at all: `Kept`, and not reliable. The
        // `KeptUnread` a JPEG `caBX`-like block gets applies once it flips; the
        // JPEG grid in `tests/policy_preview.rs` pins that disposition.
        let p = preview(&a, policy);
        assert_eq!(
            p.disposition(finding),
            Some(Disposition::Kept),
            "{policy:?}"
        );
        assert!(!p.strippable() && !p.reliable(), "{policy:?}");
        if !goes {
            assert_eq!(out.bytes(), input.as_slice(), "{policy:?} keeps it whole");
        }
    }
}

/// L2: every `oracle_png` fixture (61 at the time of writing: the wrapped TIFFs,
/// the shapes and the hostile ones) under the five presets. A typed refusal is
/// allowed; a panic or `VerifyFailed` (the writer's own self-check failing) is
/// not. Every output is a fixed point: stripping it again changes nothing.
#[test]
fn every_oracle_png_fixture_strips_or_refuses_under_every_preset_and_is_a_fixed_point() {
    let fixtures = oracle_png::fixtures();
    assert!(fixtures.len() >= 61, "{} fixtures", fixtures.len());
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
