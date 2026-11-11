//! Fuzz target for the TIFF and DNG in-place strip (SPEC-022, DEC-013).
//!
//! `exif_core::strip(bytes, policy)` on a bare TIFF: the first byte selects the
//! policy (0 to 4 the presets, 5 `Custom(none)`, 6 a custom set from the 16-bit
//! mask in bytes 1 and 2, 7 and above `All`); the rest is the TIFF candidate.
//! Invariants, for every input and policy:
//!
//! * for input `sniff` calls TIFF (anything else takes the JPEG path and only
//!   has to not panic; `rewrite_jpeg` checks it): no panic; a refusal is typed: only `Uneditable` for a TIFF (never
//!   `Unstrippable`, `Overlap`, `MpfNotIndexed`, or the self-check's
//!   `VerifyFailed`, which would mean the plan and the audit disagree);
//! * on `Ok`: the output has the input's length and the same 8 header bytes;
//!   every strip, tile and JPEG-data range of the OUTPUT's walk (computed here
//!   from `Container`, not from the planner) is byte-identical to the input;
//!   the output's audit has no finding of the policy's categories; an empty
//!   policy returns the input; stripping the output again returns the output;
//!   stripping twice gives the same answer.
//!
//! Run with `just fuzz-smoke rewrite_tiff` (see `audit.rs` for the PATH trap).
//! Seeds: `cargo run --example fuzz_seeds`.

#![no_main]

use exif_core::audit::{audit, Category};
use exif_core::policy::{StripPolicy, StripSpec};
use exif_core::tiff::Container;
use exif_core::{strip, StripError};
use libfuzzer_sys::fuzz_target;

fn decode(data: &[u8]) -> (StripPolicy, &[u8]) {
    let Some(&b0) = data.first() else {
        return (StripPolicy::All, data);
    };
    let rest = data.get(1..).unwrap_or(&[]);
    match b0 {
        0..=4 => (StripPolicy::PRESETS[usize::from(b0)], rest),
        5 => (StripPolicy::Custom(StripSpec::none()), rest),
        6 => {
            let mask = u16::from_le_bytes([
                data.get(1).copied().unwrap_or(0),
                data.get(2).copied().unwrap_or(0),
            ]);
            let mut spec = StripSpec::none();
            for (i, c) in Category::ALL.iter().enumerate() {
                if mask & (1u16 << i) != 0 {
                    spec = spec.include(*c);
                }
            }
            (StripPolicy::Custom(spec), data.get(3..).unwrap_or(&[]))
        }
        _ => (StripPolicy::All, rest),
    }
}

/// The ranges the entries of the walk name as image data: the k-th offsets
/// entry with the k-th lengths entry of each IFD; an offset with no length runs
/// to the end of the file.
fn protected(bytes: &[u8]) -> Vec<(usize, usize)> {
    let Ok(c) = Container::parse(bytes) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ifd in c.ifds() {
        for (off_tag, len_tag) in [(0x0111u16, 0x0117u16), (0x0144, 0x0145), (0x0201, 0x0202)] {
            let offs: Vec<_> = ifd
                .entries()
                .iter()
                .filter(|e| e.tag() == off_tag)
                .collect();
            let lens: Vec<_> = ifd
                .entries()
                .iter()
                .filter(|e| e.tag() == len_tag)
                .collect();
            for (k, e) in offs.iter().enumerate() {
                let Ok(o) = c.uints(e) else { continue };
                let l = lens
                    .get(k)
                    .and_then(|l| c.uints(l).ok())
                    .unwrap_or_default();
                for (j, at) in o.iter().enumerate() {
                    let at = *at as usize;
                    let end = match l.get(j) {
                        Some(n) => at.saturating_add(*n as usize),
                        None => bytes.len(),
                    }
                    .min(bytes.len());
                    if at < end {
                        out.push((at, end));
                    }
                }
            }
        }
    }
    out
}

fuzz_target!(|data: &[u8]| {
    let (policy, tiff) = decode(data);
    // The invariants below are TIFF-specific. Input the sniffer calls JPEG (for
    // instance `FF D8` followed by a TIFF header) takes the JPEG path in
    // `strip`, which has its own typed refusals (`Unstrippable`, `Overlap`, ...)
    // and is checked by `rewrite_jpeg`; here it only has to not panic.
    let is_tiff = matches!(
        exif_core::audit::sniff(tiff),
        exif_core::audit::Sniffed::Tiff
    );
    let first = strip(tiff, policy);
    if !is_tiff {
        return;
    }
    match first {
        Ok(out) => {
            let b = out.bytes();
            assert_eq!(b.len(), tiff.len(), "length");
            assert_eq!(b.get(..8), tiff.get(..8), "header");
            for (s, e) in protected(b) {
                assert_eq!(b.get(s..e), tiff.get(s..e), "image data {s}..{e}");
            }
            let spec = policy.spec();
            assert!(
                audit(b)
                    .findings()
                    .iter()
                    .all(|f| !spec.contains(f.category)),
                "a finding of a removed category is left"
            );
            if spec.is_empty() {
                assert_eq!(b, tiff, "an empty policy changed the input");
            }
            let again = strip(b, policy).expect("the output must strip again");
            assert_eq!(again.bytes(), b, "not idempotent");
            assert_eq!(
                strip(tiff, policy).ok().map(|s| s.into_bytes()),
                Some(b.to_vec())
            );
        }
        Err(StripError::Uneditable { .. }) => {}
        Err(StripError::Unsupported { .. }) | Err(StripError::NotJpeg) => {
            panic!("a classic TIFF was called unsupported");
        }
        Err(other) => panic!("a TIFF got an untyped refusal: {other:?}"),
    }
});
