//! The ExifTool parity comparator (SPEC-005): a normalisation table that maps
//! what `exif-core` reads onto ExifTool's `-n` text forms, and a field-by-field
//! `diff`. Net-new; nothing here is ported.
//!
//! Test support only (`#[path]`-included). ExifTool is run, never read: every
//! claim about its output in this file is a measurement with a command and a
//! version in `tests/oracle-fixtures/exiftool/` or in `KNOWN_DIVERGENCES`, not
//! a statement about what ExifTool does in general.

#![allow(dead_code)]

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
            let parts: Vec<f64> = r
                .iter()
                .map(|x| quotient(f64::from(x.num), f64::from(x.den)))
                .collect::<Option<Vec<f64>>>()
                .or_else(|| Some(Vec::new()))?;
            if parts.len() != 3 {
                return Some(SKIPPED.to_string());
            }
            let (d, m, sec) = (parts[0], parts[1], parts[2]);
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
pub const KNOWN_DIVERGENCES: &[Divergence] = &[Divergence {
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
    right_side: "exif-core: TIFF 6.0 section 2 makes an IFD chain a list that ends at a zero \
                 offset, so a SubIFD pointing back at IFD0 is malformed input that a reader \
                 must survive, which exif-core does by reporting the cycle and keeping what \
                 it read.",
    question_id: "exiftool-two-hop-cycle-does-not-return",
}];

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

/// Whether `(fixture, selector)` is a reported divergence.
pub fn known_divergence(fixture: &str, selector: &str) -> Option<&'static Divergence> {
    KNOWN_DIVERGENCES
        .iter()
        .find(|d| d.fixture == fixture && d.selector == selector)
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
