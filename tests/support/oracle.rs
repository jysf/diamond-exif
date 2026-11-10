//! The ExifTool parity comparator (SPEC-005): a normalisation table that maps
//! what `exif-core` reads onto ExifTool's `-n` text forms, and a field-by-field
//! `diff`. Net-new; nothing here is ported.
//!
//! Test support only (`#[path]`-included). ExifTool is run, never read: every
//! claim about its output in this file is a measurement with a command and a
//! version in `tests/oracle-fixtures/exiftool/` or in `KNOWN_DIVERGENCES`, not
//! a statement about what ExifTool does in general.

#![allow(dead_code)]

use exif_core::jpeg::{JpegMetadata, Region};
use exif_core::tiff::{Container, IfdKind};

/// How a selector's value is rendered and compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Norm {
    /// ASCII text, compared exactly.
    Ascii,
    /// Unsigned integer(s), decimal, compared exactly.
    Uint,
    /// Signed integer(s), decimal, compared exactly.
    Sint,
    /// `RATIONAL` as its quotient, compared as `f64` with tolerance.
    Rational,
    /// `SRATIONAL` as its quotient, compared as `f64` with tolerance.
    SRational,
    /// `FLOAT`/`DOUBLE`, compared as `f64` with tolerance.
    Float,
    /// Three `RATIONAL`s (degrees, minutes, seconds) as signed decimal
    /// degrees, negated for `S`/`W` per the ref tag in the same IFD.
    GpsCoord { ref_tag: u16 },
}

/// One compared field: the ExifTool selector and where we read the same thing.
#[derive(Debug, Clone, Copy)]
pub struct Selector {
    /// `Group:Tag` in ExifTool's family-1 spelling, e.g. `IFD0:Make`.
    pub exiftool: &'static str,
    /// The kind of IFD we look in (first IFD of that kind that has the tag).
    pub kind: IfdKind,
    pub tag: u16,
    pub norm: Norm,
}

const fn sel(exiftool: &'static str, kind: IfdKind, tag: u16, norm: Norm) -> Selector {
    Selector {
        exiftool,
        kind,
        tag,
        norm,
    }
}

/// The whole selector table. Minimal and exact. The GPS selectors are
/// ExifTool's `Composite:` (signed) forms: measured with ExifTool 13.55 on the
/// `exif-gps-sw-be` fixture, `-n -GPS:GPSLatitude` prints `37.4252083333333`
/// (unsigned, the ref is the separate `GPS:GPSLatitudeRef` tag) while
/// `-n -Composite:GPSLatitude` prints `-37.4252083333333`. Selector names are ExifTool's
/// own spellings (measured by running it on `exif-basic`): where its name for a
/// tag differs from the spec name, the spec name is in the comment.
pub const SELECTORS: &[Selector] = &[
    sel("IFD0:ImageWidth", IfdKind::Ifd0, 256, Norm::Uint),
    sel("IFD0:ImageHeight", IfdKind::Ifd0, 257, Norm::Uint), // spec: ImageLength
    sel("IFD0:Make", IfdKind::Ifd0, 271, Norm::Ascii),
    sel("IFD0:Model", IfdKind::Ifd0, 272, Norm::Ascii),
    sel("IFD0:Orientation", IfdKind::Ifd0, 274, Norm::Uint),
    sel("IFD0:XResolution", IfdKind::Ifd0, 282, Norm::Rational),
    sel("IFD0:Software", IfdKind::Ifd0, 305, Norm::Ascii),
    sel("IFD0:ModifyDate", IfdKind::Ifd0, 306, Norm::Ascii), // spec: DateTime
    sel("IFD0:Artist", IfdKind::Ifd0, 315, Norm::Ascii),
    sel("IFD0:Copyright", IfdKind::Ifd0, 33432, Norm::Ascii),
    sel("ExifIFD:FNumber", IfdKind::Exif, 0x829D, Norm::Rational),
    sel("ExifIFD:ISO", IfdKind::Exif, 0x8827, Norm::Uint), // spec: ISOSpeedRatings
    sel(
        "ExifIFD:DateTimeOriginal",
        IfdKind::Exif,
        0x9003,
        Norm::Ascii,
    ),
    sel(
        "ExifIFD:ExposureCompensation", // spec: ExposureBiasValue
        IfdKind::Exif,
        0x9204,
        Norm::SRational,
    ),
    // SPEC-008: the first selectors that read a signed integer and a float.
    // Measured with ExifTool 13.55 on hand-built files (`exiftool -n -G1 -a -u
    // -U`): a type-8 SSHORT -2 in tag 0x8822 prints `-2`, a type-9 SLONG -5 in
    // 0xA002 prints `-5`, a type-11 FLOAT 0.125 in 0x829A prints `0.125`, a
    // type-12 DOUBLE 4.5 in 0x920A prints `4.5` (the full command/output list
    // is in the SPEC-008 handback).
    sel("ExifIFD:ExposureTime", IfdKind::Exif, 0x829A, Norm::Float),
    sel("ExifIFD:ExposureProgram", IfdKind::Exif, 0x8822, Norm::Sint),
    sel("ExifIFD:FocalLength", IfdKind::Exif, 0x920A, Norm::Float),
    sel("ExifIFD:ExifImageWidth", IfdKind::Exif, 0xA002, Norm::Sint),
    sel(
        "Composite:GPSLatitude",
        IfdKind::Gps,
        2,
        Norm::GpsCoord { ref_tag: 1 },
    ),
    sel(
        "Composite:GPSLongitude",
        IfdKind::Gps,
        4,
        Norm::GpsCoord { ref_tag: 3 },
    ),
];

/// The three selectors the seed sweep compares.
pub const SEED_SELECTORS: &[Selector] = &[
    sel("IFD0:ImageWidth", IfdKind::Ifd0, 256, Norm::Uint),
    sel("IFD0:ImageHeight", IfdKind::Ifd0, 257, Norm::Uint),
    sel("IFD0:Orientation", IfdKind::Ifd0, 274, Norm::Uint),
];

/// The ExifTool selector strings of a table, in order.
pub fn names(selectors: &[Selector]) -> Vec<&'static str> {
    selectors.iter().map(|s| s.exiftool).collect()
}

/// A reading that is deliberately not compared (zero denominator: ExifTool's
/// `inf`/`undef` forms are out of scope for SPEC-005).
pub const SKIPPED: &str = "\u{1}skipped-zero-denominator";

/// Quotient as `f64` text, or `None` for a zero denominator.
fn quotient(num: f64, den: f64) -> Option<f64> {
    if den == 0.0 {
        None
    } else {
        Some(num / den)
    }
}

fn render_f64(v: f64) -> String {
    format!("{v}")
}

/// Our reading of one selector, in ExifTool's `-n` form. `None` when the IFD or
/// tag is absent or its payload cannot be decoded. A zero-denominator rational
/// reads as [`SKIPPED`].
pub fn read_one(container: &Container<'_>, s: &Selector) -> Option<String> {
    let ifd = container
        .ifds()
        .iter()
        .find(|ifd| ifd.kind() == s.kind && ifd.has(s.tag))?;
    let entry = ifd.entry(s.tag)?;
    match s.norm {
        Norm::Ascii => {
            let a = container.ascii(entry).ok()?;
            Some(a.to_string_lossy().into_owned())
        }
        Norm::Uint => {
            let v = container.uints(entry).ok()?;
            join(v.iter().map(|x| x.to_string()))
        }
        Norm::Sint => {
            let v = container.sints(entry).ok()?;
            join(v.iter().map(|x| x.to_string()))
        }
        Norm::Rational => {
            let r = container.urationals(entry).ok()?;
            let first = r.first()?;
            match quotient(f64::from(first.num), f64::from(first.den)) {
                Some(q) => Some(render_f64(q)),
                None => Some(SKIPPED.to_string()),
            }
        }
        Norm::SRational => {
            let r = container.srationals(entry).ok()?;
            let first = r.first()?;
            match quotient(f64::from(first.num), f64::from(first.den)) {
                Some(q) => Some(render_f64(q)),
                None => Some(SKIPPED.to_string()),
            }
        }
        Norm::Float => {
            let v = container.floats(entry).ok()?;
            join(v.iter().map(|x| render_f64(*x)))
        }
        Norm::GpsCoord { ref_tag } => {
            let r = container.urationals(entry).ok()?;
            let ref_entry = ifd.entry(ref_tag)?;
            let ref_text = container.ascii(ref_entry).ok()?;
            let sign = match ref_text.as_bytes().first()? {
                b'N' | b'E' => 1.0,
                b'S' | b'W' => -1.0,
                _ => return None,
            };
            // A coordinate is exactly three values; any other count is
            // undecodable for this selector (None), not a zero denominator.
            let [d, m, sec] = r.as_slice() else {
                return None;
            };
            let part =
                |x: &exif_core::tiff::URational| quotient(f64::from(x.num), f64::from(x.den));
            let (Some(d), Some(m), Some(sec)) = (part(d), part(m), part(sec)) else {
                return Some(SKIPPED.to_string());
            };
            Some(render_f64(sign * (d + m / 60.0 + sec / 3600.0)))
        }
    }
}

fn join(it: impl Iterator<Item = String>) -> Option<String> {
    let parts: Vec<String> = it.collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Our reading of every selector, in order.
pub fn read_ours(container: &Container<'_>, selectors: &[Selector]) -> Vec<Option<String>> {
    selectors.iter().map(|s| read_one(container, s)).collect()
}

/// One field where ours and theirs disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub selector: &'static str,
    pub ours: Option<String>,
    pub theirs: Option<String>,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: ours={:?} theirs={:?}",
            self.selector, self.ours, self.theirs
        )
    }
}

/// Relative tolerance for `f64` comparisons, absolute near zero (see
/// [`floats_agree`]).
pub const REL_TOL: f64 = 1e-9;
pub const ABS_TOL: f64 = 1e-12;

fn floats_agree(a: f64, b: f64) -> bool {
    let d = (a - b).abs();
    d <= ABS_TOL || d <= REL_TOL * a.abs().max(b.abs())
}

fn texts_agree(norm: Norm, ours: &str, theirs: &str) -> bool {
    match norm {
        // Text and integers compare exactly.
        Norm::Ascii | Norm::Uint | Norm::Sint => ours == theirs,
        // Rational, signed rational, float and GPS compare as f64 with a
        // relative tolerance of 1e-9 (absolute 1e-12 near zero).
        Norm::Rational | Norm::SRational | Norm::Float | Norm::GpsCoord { .. } => {
            match (ours.parse::<f64>(), theirs.parse::<f64>()) {
                (Ok(a), Ok(b)) => floats_agree(a, b),
                _ => false,
            }
        }
    }
}

/// Compare readings field by field. `None` equals `None`; `Some` vs `None` in
/// either direction is a mismatch. A selector whose reading is [`SKIPPED`] is
/// skipped with a printed note.
pub fn diff(
    ours: &[Option<String>],
    theirs: &[Option<String>],
    selectors: &[Selector],
) -> Vec<Mismatch> {
    assert_eq!(ours.len(), selectors.len(), "ours/selectors length");
    assert_eq!(theirs.len(), selectors.len(), "theirs/selectors length");
    let mut out = Vec::new();
    for ((s, o), t) in selectors.iter().zip(ours).zip(theirs) {
        if o.as_deref() == Some(SKIPPED) {
            println!("NOTE {}: zero denominator, not compared", s.exiftool);
            continue;
        }
        let agree = match (o, t) {
            (None, None) => true,
            (Some(a), Some(b)) => texts_agree(s.norm, a, b),
            _ => false,
        };
        if !agree {
            out.push(Mismatch {
                selector: s.exiftool,
                ours: o.clone(),
                theirs: t.clone(),
            });
        }
    }
    out
}

/// A measured disagreement between ExifTool and `exif-core` that has been
/// reported and is tracked in `guidance/questions.yaml`. An entry is only
/// allowed with all of these fields filled from a real measurement; it is
/// never a way to loosen a comparison.
#[derive(Debug)]
pub struct Divergence {
    /// Fixture or seed name.
    pub fixture: &'static str,
    /// The selector that disagrees, or `"*"` when ExifTool produces no output
    /// at all for the input (then neither the recorder nor the live sweep runs
    /// it, see [`not_run_divergence`]).
    pub selector: &'static str,
    pub command: &'static str,
    pub exiftool_version: &'static str,
    pub measured_stdout: &'static str,
    /// Which side is right per the format spec, in one sentence.
    pub right_side: &'static str,
    pub question_id: &'static str,
}

/// Divergences measured so far (SPEC-005, ExifTool 13.55, macOS).
pub const KNOWN_DIVERGENCES: &[Divergence] = &[
    Divergence {
        fixture: "two-hop-cycle",
        selector: "*",
        command: "exiftool -T -n -s3 -IFD0:ImageWidth -IFD0:ImageHeight -IFD0:Orientation \
              fuzz/seeds/tiff_ifd/two-hop-cycle",
        exiftool_version: "13.55",
        measured_stdout: "empty stdout; the process grew to about 8.9 GB resident within 5 s, \
                      was killed by SIGKILL (exit status 137) after 53.4 s, and wrote \
                      'Deep recursion on subroutine ProcessDirectory' and \
                      'Deep recursion on subroutine ProcessExif' to stderr. exif-core reads \
                      IFD0:ImageWidth = 1, records a CyclicIfd issue at offset 8, and returns.",
        right_side:
            "exif-core: the TIFF Supplement / Tech Note 1 define SubIFDs as a list of IFDs \
                 hanging off IFD0, so one pointing back at IFD0 is malformed input that a \
                 reader must survive, which exif-core does by reporting the cycle and keeping what \
                 it read.",
        question_id: "exiftool-two-hop-cycle-does-not-return",
    },
    // ── SPEC-011: JPEG (ExifTool 13.55, macOS). Each command line is the one the
    // recorder runs: the 20 Exif selectors, then the two region selectors for a
    // hand-built fixture, the 20 for a fuzz seed. ─────────────────────────────────
    Divergence {
        fixture: "jpeg-exif-after-scan",
        selector: EXIF_ANY,
        command: "exiftool -T -n -s3 <the 22 selectors of oracle::jpeg_names()> \
              jpeg-exif-after-scan (SOI, SOS header, 2 scan bytes, APP1 Exif\\0\\0 + \
              exif-basic-le, EOI)",
        exiftool_version: "13.55",
        measured_stdout: "all 22 fields are '-' and nothing is written to stderr. exif-core reads \
                      the Exif block (Segment::after_scan() is true for it): IFD0:Make = \
                      DiamondCam and the rest of exif-basic-le.",
        right_side:
            "exif-core, for an upload audit and sanitiser: metadata that sits after the scan \
                 is still metadata another reader can pick up, and reading more than the \
                 reference is the safe direction; callers that care ask Segment::after_scan().",
        question_id: "exiftool-ignores-exif-after-scan",
    },
    Divergence {
        fixture: "jpeg-seed-exif-after-scan",
        selector: EXIF_ANY,
        command: "exiftool -T -n -s3 <the 20 Exif selectors of oracle::SELECTORS> \
              fuzz/seeds/jpeg_segments/exif-after-scan",
        exiftool_version: "13.55",
        measured_stdout: "all 20 fields are '-' and nothing is written to stderr. exif-core reads \
                      the Exif block after the scan: IFD0:Make = DiamondCam.",
        right_side: "exif-core, as for jpeg-exif-after-scan (the same finding on the fuzz seed).",
        question_id: "exiftool-ignores-exif-after-scan",
    },
    Divergence {
        fixture: "jpeg-exif-last-segment-at-eof",
        selector: EXIF_ANY,
        command: "exiftool -T -n -s3 <the 22 selectors of oracle::jpeg_names()> \
              jpeg-exif-last-segment-at-eof (SOI, APP1 Exif\\0\\0 + exif-basic-le, and the \
              file ends exactly at the end of that segment)",
        exiftool_version: "13.55",
        measured_stdout: "all 22 fields are '-'; stderr is 'Warning: JPEG format error - <file>'. \
                      exif-core reads the Exif block and records MissingSos and MissingEoi \
                      at the JPEG layer.",
        right_side:
            "exif-core: the segment is complete (its declared length fits), so the Exif is \
                 readable; reporting the missing SOS and EOI is the right place for the damage. \
                 Reading more than the reference is the safe direction for an audit.",
        question_id: "exiftool-drops-exif-when-last-segment-ends-at-eof",
    },
    Divergence {
        fixture: "jpeg-seventeen-exif-last-differs",
        selector: "IFD0:Make",
        command: "exiftool -T -n -s3 <the 22 selectors of oracle::jpeg_names()> \
              jpeg-seventeen-exif-last-differs (seventeen Exif APP1s; the first sixteen say \
              Make = SameFirst16, the seventeenth says LastOnly17)",
        exiftool_version: "13.55",
        measured_stdout: "IFD0:Make = LastOnly17 and every other field '-': ExifTool reads all \
                      seventeen segments and the last one wins. exif-core parses at most \
                      MAX_EXIF_BLOCKS = 16 blocks, so the harness's last-block model reads \
                      IFD0:Make = SameFirst16.",
        right_side: "neither is wrong per the format (the Exif standard allows one Exif APP1); \
                 exif-core's cap is the documented bounded-work choice (DEC-006) and the 17th \
                 segment still costs an ExtraExif issue.",
        question_id: "exiftool-reads-17th-exif-block",
    },
    Divergence {
        fixture: "jpeg-exif-before-and-after-scan",
        selector: "IFD0:Make",
        command: "exiftool -T -n -s3 <the 22 selectors of oracle::jpeg_names()> \
              jpeg-exif-before-and-after-scan (SOI, Exif A: Make AlphaCam + Model OnlyInA, SOS \
              header, 2 scan bytes, Exif B: Make BravoCam, EOI)",
        exiftool_version: "13.55",
        measured_stdout: "IFD0:Make = AlphaCam, IFD0:Model = OnlyInA, every other field '-', no \
                      stderr: the block after the scan is ignored. The harness model reads \
                      blocks after the scan too (the library does), so the later block B wins: \
                      IFD0:Make = BravoCam, IFD0:Model = OnlyInA.",
        right_side:
            "exif-core for an audit (a block after the scan is metadata another reader can \
                 pick up); the divergence is the same finding as exiftool-ignores-exif-after-scan \
                 seen on a value both sides read.",
        question_id: "exiftool-ignores-exif-after-scan",
    },
    Divergence {
        fixture: "jpeg-gps-split-across-two-blocks",
        selector: "Composite:GPSLatitude",
        command: "exiftool -T -n -s3 <the 22 selectors of oracle::jpeg_names()> \
              jpeg-gps-split-across-two-blocks (Exif A: GPS IFD with GPSLatitudeRef N and \
              GPSLatitude 37/1 30/1 0/1; Exif B: GPS IFD with only GPSLatitudeRef S)",
        exiftool_version: "13.55",
        measured_stdout:
            "Composite:GPSLatitude = -37.5 (ExifTool combines A's latitude with B's ref \
                      S; it also reports GPS:GPSLatitudeRef = S). The harness model computes the \
                      composite inside ONE block, the latest block that can: A alone, N, so it \
                      reads 37.5.",
        right_side: "neither: a documented limit of the harness model (it merges tags per block, \
                 ExifTool merges them across blocks before deriving the composite); the library \
                 reads each block on its own by design.",
        question_id: "exiftool-composite-gps-across-exif-blocks",
    },
    // ── SPEC-019 / DEC-012 section 1: every `Photoshop 3.0` APP13 segment is an
    // `Iptc` region. ───────────────────────────────────────────────────────────
    Divergence {
        fixture: "jpeg-iptc-without-0404",
        selector: "IPTC:ObjectName",
        command: "exiftool -T -n -s3 <the 22 selectors of oracle::jpeg_names()> \
              jpeg-iptc-without-0404 (one Photoshop 3.0 APP13 segment holding a single \
              0x0405 resource, no 0x0404)",
        exiftool_version: "13.55",
        measured_stdout: "all 22 fields are '-' (ExifTool prints no IPTC group for it); \
                      exif-core now locates the segment as an Iptc region, so the region \
                      presence selector reads 'present'.",
        right_side: "exif-core, by DEC-012: a Photoshop block is a block the writer must handle \
                 whatever it holds (SPIKE-001 found one split across two segments), so every \
                 Photoshop segment is a region; this is a presence difference, not a decode.",
        question_id: "iptc-photoshop-resources-across-app13-segments",
    },
];

/// For a fixture covered by an [`EXIF_ANY`] divergence: the committed record of
/// the SAME TIFF in a place both tools read, whose ExifTool fields our Exif
/// reading must equal. Without it the wildcard would let any value of ours pass
/// (verify finding M1).
pub const EXIF_PINS: &[(&str, &str)] = &[
    ("jpeg-exif-after-scan", "jpeg-exif-basic-le"),
    ("jpeg-exif-last-segment-at-eof", "jpeg-exif-basic-le"),
    // The seed's TIFF is the one in seed `exif-in-jpeg`.
    ("jpeg-seed-exif-after-scan", "jpeg-seed-exif-in-jpeg"),
];

/// The record name that pins `fixture`, if it has an [`EXIF_ANY`] divergence.
pub fn exif_pin_record(fixture: &str) -> Option<&'static str> {
    EXIF_PINS
        .iter()
        .find(|(f, _)| *f == fixture)
        .map(|(_, r)| *r)
}

/// An input ExifTool 13.55 does not return on (a `"*"` divergence). Neither the
/// recorder nor the live sweep runs ExifTool on it: the measured behaviour is
/// unbounded memory growth (about 9 GB in 5 s) until the OS kills it.
pub fn not_run_divergence(name: &str) -> Option<&'static Divergence> {
    known_divergence(name, "*")
}

/// The `# outcome:` text the recorder writes for a not-run input.
pub fn not_run_outcome(d: &Divergence) -> String {
    format!(
        "not run: exiftool {} does not return on this input (see {}); measured: {}",
        d.exiftool_version, d.question_id, d.measured_stdout
    )
}

/// The `selector` of a [`Divergence`] that covers every Exif value selector in
/// [`SELECTORS`] (not the region presence selectors): "ExifTool reads none of
/// the Exif in this input, ours reads it" is one finding, not twenty.
pub const EXIF_ANY: &str = "exif:*";

/// Whether `selector` is one of the Exif value selectors in [`SELECTORS`].
pub fn is_exif_selector(selector: &str) -> bool {
    SELECTORS.iter().any(|s| s.exiftool == selector)
}

/// Whether `(fixture, selector)` is a reported divergence. A divergence whose
/// selector is [`EXIF_ANY`] covers every Exif value selector.
pub fn known_divergence(fixture: &str, selector: &str) -> Option<&'static Divergence> {
    KNOWN_DIVERGENCES.iter().find(|d| {
        d.fixture == fixture
            && (d.selector == selector || (d.selector == EXIF_ANY && is_exif_selector(selector)))
    })
}

/// A value that differs from `v` in exactly the way `norm` compares: used by
/// the perturbation red-proofs.
pub fn perturb(norm: Norm, v: &str) -> String {
    match norm {
        Norm::Ascii => format!("{v}!"),
        Norm::Uint | Norm::Sint => match v.parse::<i64>() {
            Ok(n) => (n + 1).to_string(),
            Err(_) => format!("{v}!"),
        },
        Norm::Rational | Norm::SRational | Norm::Float | Norm::GpsCoord { .. } => {
            match v.parse::<f64>() {
                Ok(x) => render_f64(x + 1.0),
                Err(_) => format!("{v}!"),
            }
        }
    }
}

// ── JPEG (SPEC-011) ─────────────────────────────────────────────────────────

/// Our reading of every selector from a JPEG, modelled on what ExifTool 13.55
/// was measured to do with several Exif segments: the UNION of the tags of the
/// blocks, and where two blocks carry the same tag the LATER block's value wins
/// (`jpeg-two-exif-same-tag` and `-reversed`, `jpeg-seventeen-exif-last-differs`).
/// So each selector reads from the last Exif block, in file order, whose
/// container parsed and that has the tag; `None` if no block does. For a file
/// with at most one Exif block this equals [`read_ours`] of that block.
///
/// This is the harness's model, deliberately not a library feature: the library
/// keeps "the first block is primary, the rest cost `ExtraExif`" (DEC-006) and
/// the question of exposing a merged view is open (`several-exif-segments`).
pub fn read_ours_jpeg(meta: &JpegMetadata<'_>, selectors: &[Selector]) -> Vec<Option<String>> {
    selectors
        .iter()
        .map(|s| {
            meta.exif_blocks()
                .iter()
                .rev()
                .find_map(|b| b.container().ok().and_then(|c| read_one(c, s)))
        })
        .collect()
}

/// A selector compared by PRESENCE only: some located region exists on our side
/// and ExifTool prints a value for its tag. No value comparison (SPEC-011
/// non-goal).
#[derive(Debug, Clone, Copy)]
pub struct RegionSelector {
    pub exiftool: &'static str,
    pub kind: RegionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// Any `Region::Xmp`.
    Xmp,
    /// Any `Region::Iptc`.
    Iptc,
}

/// `XMP-xmp:CreatorTool` <-> a located XMP packet, `IPTC:ObjectName` <-> a
/// located IPTC (`0x0404`) resource. Only meaningful on fixtures whose packet
/// or dataset really carries that tag (the hand-built `jpeg-xmp*` and
/// `jpeg-iptc*` fixtures); the fuzz seeds are compared on [`SELECTORS`] only.
pub const REGION_SELECTORS: &[RegionSelector] = &[
    RegionSelector {
        exiftool: "XMP-xmp:CreatorTool",
        kind: RegionKind::Xmp,
    },
    RegionSelector {
        exiftool: "IPTC:ObjectName",
        kind: RegionKind::Iptc,
    },
];

/// What a present region reads as.
pub const PRESENT: &str = "present";

/// The ExifTool selector strings of [`SELECTORS`] then [`REGION_SELECTORS`]:
/// the command line of a hand-built JPEG fixture.
pub fn jpeg_names() -> Vec<&'static str> {
    let mut v = names(SELECTORS);
    v.extend(REGION_SELECTORS.iter().map(|r| r.exiftool));
    v
}

/// Our region presence, one entry per [`REGION_SELECTORS`].
pub fn read_regions(meta: &JpegMetadata<'_>) -> Vec<Option<String>> {
    REGION_SELECTORS
        .iter()
        .map(|r| {
            let found = meta.regions().iter().any(|region| {
                matches!(
                    (r.kind, region),
                    (RegionKind::Xmp, Region::Xmp { .. }) | (RegionKind::Iptc, Region::Iptc { .. })
                )
            });
            found.then(|| PRESENT.to_string())
        })
        .collect()
}

/// Our whole JPEG reading: the Exif selectors (the harness model above), then
/// the region presence.
pub fn read_jpeg_all(meta: &JpegMetadata<'_>) -> Vec<Option<String>> {
    let mut v = read_ours_jpeg(meta, SELECTORS);
    v.extend(read_regions(meta));
    v
}

/// ExifTool's values reduced to presence (`Some(_)` -> `Some("present")`).
fn presence(theirs: &[Option<String>]) -> Vec<Option<String>> {
    theirs
        .iter()
        .map(|t| t.as_ref().map(|_| PRESENT.to_string()))
        .collect()
}

/// Compare a [`read_jpeg_all`] reading with ExifTool's fields for
/// [`jpeg_names`]: value comparison for the Exif selectors, presence for the
/// region selectors. Nothing is filtered: [`compare_jpeg`] applies the known
/// divergences.
pub fn diff_jpeg(ours: &[Option<String>], theirs: &[Option<String>]) -> Vec<Mismatch> {
    let n = SELECTORS.len();
    assert_eq!(ours.len(), n + REGION_SELECTORS.len(), "ours length");
    assert_eq!(theirs.len(), n + REGION_SELECTORS.len(), "theirs length");
    let mut out = diff(
        ours.get(..n).unwrap_or_default(),
        theirs.get(..n).unwrap_or_default(),
        SELECTORS,
    );
    let (o, t) = (
        ours.get(n..).unwrap_or_default(),
        presence(theirs.get(n..).unwrap_or_default()),
    );
    for ((r, a), b) in REGION_SELECTORS.iter().zip(o).zip(&t) {
        if a != b {
            out.push(Mismatch {
                selector: r.exiftool,
                ours: a.clone(),
                theirs: b.clone(),
            });
        }
    }
    out
}

/// The result of comparing one JPEG with ExifTool, with the known divergences
/// applied in BOTH directions: a mismatch covered by an entry is `explained`;
/// an entry that no mismatch matches any more is `stale` (a divergence that
/// stops being one fails the test, so the list cannot rot).
#[derive(Debug, Default)]
pub struct JpegComparison {
    /// Mismatches no [`KNOWN_DIVERGENCES`] entry covers.
    pub unexplained: Vec<Mismatch>,
    /// Mismatches a [`KNOWN_DIVERGENCES`] entry covers.
    pub explained: Vec<Mismatch>,
    /// Entries for this fixture that no mismatch exercises.
    pub stale: Vec<String>,
}

impl JpegComparison {
    pub fn is_clean(&self) -> bool {
        self.unexplained.is_empty() && self.stale.is_empty()
    }
}

/// [`diff_jpeg`] plus the known-divergence accounting for fixture `name`. For a
/// wildcard ([`EXIF_ANY`]) entry every covered mismatch must also be of one
/// shape: ours reads a value, ExifTool reads none.
pub fn compare_jpeg(
    name: &str,
    ours: &[Option<String>],
    theirs: &[Option<String>],
) -> JpegComparison {
    compare_jpeg_pinned(name, ours, theirs, None)
}

/// [`compare_jpeg`] with the pin for an [`EXIF_ANY`] fixture: `pin` is
/// ExifTool's fields (at least the Exif selectors) for the record named by
/// [`exif_pin_record`]. Our Exif reading must equal them; a fixture that has a
/// wildcard entry and is compared WITHOUT a pin is itself a failure (`stale`).
pub fn compare_jpeg_pinned(
    name: &str,
    ours: &[Option<String>],
    theirs: &[Option<String>],
    pin: Option<&[Option<String>]>,
) -> JpegComparison {
    let found = diff_jpeg(ours, theirs);
    let mut cmp = JpegComparison::default();
    for m in &found {
        match known_divergence(name, m.selector) {
            Some(d) if d.selector == EXIF_ANY && !(m.ours.is_some() && m.theirs.is_none()) => {
                cmp.unexplained.push(m.clone());
            }
            Some(_) => cmp.explained.push(m.clone()),
            None => cmp.unexplained.push(m.clone()),
        }
    }
    if KNOWN_DIVERGENCES
        .iter()
        .any(|d| d.fixture == name && d.selector == EXIF_ANY)
    {
        let n = SELECTORS.len();
        match (pin, ours.get(..n)) {
            (Some(p), Some(o)) if p.len() >= n => {
                let p = p.get(..n).unwrap_or_default();
                cmp.unexplained.extend(diff(o, p, SELECTORS));
            }
            _ => cmp.stale.push(format!(
                "{name}: a wildcard ({EXIF_ANY}) divergence needs the pinned reading of {:?}",
                exif_pin_record(name)
            )),
        }
    }
    for d in KNOWN_DIVERGENCES
        .iter()
        .filter(|d| d.fixture == name && d.selector != "*")
    {
        let exercised = cmp.explained.iter().any(|m| {
            d.selector == m.selector || (d.selector == EXIF_ANY && is_exif_selector(m.selector))
        });
        if !exercised {
            cmp.stale.push(format!(
                "{} {}: the divergence ({}) is no longer present",
                d.fixture, d.selector, d.question_id
            ));
        }
    }
    cmp
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &[(Norm, &str)] = &[
        (Norm::Ascii, "DiamondCam"),
        (Norm::Uint, "6"),
        (Norm::Sint, "-3"),
        (Norm::Rational, "2.8"),
        (Norm::SRational, "-0.333333333333333"),
        (Norm::Float, "0.5"),
        (Norm::GpsCoord { ref_tag: 1 }, "-37.4175"),
    ];

    fn synthetic(norm: Norm) -> Selector {
        sel("Synthetic:Field", IfdKind::Ifd0, 1, norm)
    }

    #[test]
    fn diff_is_clean_for_equal_readings() {
        for (norm, v) in ALL {
            let s = [synthetic(*norm)];
            let r = [Some((*v).to_string())];
            assert!(diff(&r, &r, &s).is_empty(), "{norm:?}");
        }
    }

    #[test]
    fn diff_applies_the_float_tolerance() {
        let s = [synthetic(Norm::Float)];
        let ours = [Some("1.0".to_string())];
        // 1e-10 relative passes.
        let close = [Some("1.0000000001".to_string())];
        assert!(diff(&ours, &close, &s).is_empty());
        // 1e-8 relative fails: pins the tolerance within ~10x of 1e-9, so
        // widening it by orders of magnitude turns this red.
        let near = [Some("1.00000001".to_string())];
        assert_eq!(diff(&ours, &near, &s).len(), 1);
        // 1e-6 relative fails.
        let far = [Some("1.000001".to_string())];
        assert_eq!(diff(&ours, &far, &s).len(), 1);
        // Near zero the absolute tolerance applies.
        let tiny_a = [Some("0".to_string())];
        let tiny_b = [Some("1e-13".to_string())];
        assert!(diff(&tiny_a, &tiny_b, &s).is_empty());
    }

    #[test]
    fn diff_reports_a_missing_field() {
        for (norm, v) in ALL {
            let s = [synthetic(*norm)];
            let got = diff(&[None], &[Some((*v).to_string())], &s);
            assert_eq!(got.len(), 1, "{norm:?}");
            assert_eq!(got[0].selector, "Synthetic:Field");
            assert_eq!(got[0].ours, None);
        }
    }

    #[test]
    fn diff_reports_an_extra_field() {
        for (norm, v) in ALL {
            let s = [synthetic(*norm)];
            let got = diff(&[Some((*v).to_string())], &[None], &s);
            assert_eq!(got.len(), 1, "{norm:?}");
            assert_eq!(got[0].theirs, None);
        }
    }

    #[test]
    fn diff_skips_a_zero_denominator_reading() {
        let s = [synthetic(Norm::Rational)];
        let got = diff(&[Some(SKIPPED.to_string())], &[Some("inf".to_string())], &s);
        assert!(got.is_empty());
    }

    #[test]
    fn perturbing_one_value_yields_exactly_one_mismatch_per_norm() {
        // One field per Norm variant, all in one comparison: perturb each in
        // turn and require that exactly that selector is named.
        let sels: Vec<Selector> = ALL
            .iter()
            .enumerate()
            .map(|(i, (n, _))| {
                let names = [
                    "S:Ascii",
                    "S:Uint",
                    "S:Sint",
                    "S:Rational",
                    "S:SRational",
                    "S:Float",
                    "S:GpsCoord",
                ];
                sel(names[i], IfdKind::Ifd0, 1, *n)
            })
            .collect();
        let theirs: Vec<Option<String>> = ALL.iter().map(|(_, v)| Some((*v).to_string())).collect();
        for i in 0..sels.len() {
            let mut ours = theirs.clone();
            ours[i] = ours[i].as_deref().map(|v| perturb(sels[i].norm, v));
            let got = diff(&ours, &theirs, &sels);
            assert_eq!(got.len(), 1, "{:?}: {got:?}", sels[i].norm);
            assert_eq!(got[0].selector, sels[i].exiftool);
        }
    }

    // ── GPS sign follows the ref tag ────────────────────────────────────────

    fn gps_container(lat_ref: u8, lon_ref: u8) -> Vec<u8> {
        // Local copy of the layout via the fixture builder.
        use crate::oracle_tiff as t;
        let ifd0 = t::ifd(vec![t::ptr(t::T_GPS_IFD, 1)]);
        let gps = t::ifd(vec![
            t::ascii(t::T_GPS_LAT_REF, &[lat_ref, 0]),
            t::rational(false, t::T_GPS_LAT, &[(10, 1), (30, 1), (0, 1)]),
            t::ascii(t::T_GPS_LON_REF, &[lon_ref, 0]),
            t::rational(false, t::T_GPS_LON, &[(20, 1), (15, 1), (36, 1)]),
        ]);
        t::assemble(false, &[ifd0, gps])
    }

    #[test]
    fn gps_coordinate_sign_follows_the_ref_tag() {
        let cases = [
            (b'N', b'E', 1.0, 1.0),
            (b'N', b'W', 1.0, -1.0),
            (b'S', b'E', -1.0, 1.0),
            (b'S', b'W', -1.0, -1.0),
        ];
        for (la, lo, lat_sign, lon_sign) in cases {
            let bytes = gps_container(la, lo);
            let c = Container::parse(&bytes).expect("parses");
            let got = read_ours(&c, &SELECTORS[SELECTORS.len() - 2..]);
            let lat: f64 = got[0].as_deref().expect("lat").parse().expect("num");
            let lon: f64 = got[1].as_deref().expect("lon").parse().expect("num");
            assert!(floats_agree(lat, lat_sign * 10.5), "{la} {lat}");
            assert!(floats_agree(lon, lon_sign * 20.26), "{lo} {lon}");
        }
    }
}
