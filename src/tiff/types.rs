//! TIFF field types and the integer / rational readers over a payload.
//!
//! Provenance: copied from irradiance `src/ifd.rs:186-215` (type constants and
//! `type_size`) and `:1025-1193` (`uints`, `rationals`), then edited (DEC-003):
//! the DNG per-tag `RATIONAL` gate and integer-division rule are removed, and
//! `RATIONAL`/`SRATIONAL` are held verbatim as unsigned/signed pairs.

use super::{Container, Entry, MAX_TAG_VALUES};
use crate::error::Error;

// TIFF 6.0 §2, "Types". Every type 1-12 is named (SPEC-007), plus TIFF/EP's
// `IFD`; `type_size` sizes them and each reader accepts only its own set.

/// TIFF 6.0 §2: 8-bit unsigned integer.
pub const TYPE_BYTE: u16 = 1;
/// TIFF 6.0 §2: 8-bit bytes holding 7-bit ASCII, NUL-terminated by the spec.
pub const TYPE_ASCII: u16 = 2;
/// TIFF 6.0 §2: 16-bit unsigned integer.
pub const TYPE_SHORT: u16 = 3;
/// TIFF 6.0 §2: 32-bit unsigned integer.
pub const TYPE_LONG: u16 = 4;
/// TIFF 6.0 §2: two `LONG`s, numerator then denominator.
pub const TYPE_RATIONAL: u16 = 5;
/// TIFF 6.0 §2: an 8-bit byte that may contain anything.
pub const TYPE_UNDEFINED: u16 = 7;
/// TIFF 6.0 §2: 8-bit signed integer.
pub const TYPE_SBYTE: u16 = 6;
/// TIFF 6.0 §2: 16-bit signed integer.
pub const TYPE_SSHORT: u16 = 8;
/// TIFF 6.0 §2: 32-bit signed integer.
pub const TYPE_SLONG: u16 = 9;
/// TIFF 6.0 §2: IEEE 754 binary32.
pub const TYPE_FLOAT: u16 = 11;
/// TIFF 6.0 §2: IEEE 754 binary64.
pub const TYPE_DOUBLE: u16 = 12;
/// TIFF 6.0 §2: two `SLONG`s, numerator then denominator, a **signed**
/// rational.
pub const TYPE_SRATIONAL: u16 = 10;
/// TIFF/EP type 13: a 32-bit IFD offset.
pub const TYPE_IFD: u16 = 13;

/// An unsigned `RATIONAL` (TIFF 6.0 §2), held verbatim: never divided, and a
/// zero denominator or a value above `i32::MAX` is representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct URational {
    /// The numerator as read from the file.
    pub num: u32,
    /// The denominator as read from the file.
    pub den: u32,
}

/// A signed `SRATIONAL` (TIFF 6.0 §2), held verbatim: never divided, and a
/// zero denominator is representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SRational {
    /// The numerator as read from the file.
    pub num: i32,
    /// The denominator as read from the file.
    pub den: i32,
}

/// An `ASCII` field's payload, split at its first NUL. Borrows the input
/// (DEC-002), so it outlives the [`Container`] it was read through.
///
/// TIFF says an ASCII value ends with a NUL; real files often omit it, hold
/// several strings, or carry Latin-1. All of those are data here, not errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ascii<'a> {
    text: &'a [u8],
    terminated: bool,
    rest: &'a [u8],
}

impl<'a> Ascii<'a> {
    /// Split `payload` at its first NUL. Total: no indexing, no panic.
    fn split(payload: &'a [u8]) -> Ascii<'a> {
        match payload.iter().position(|b| *b == 0) {
            Some(nul) => {
                let (text, tail) = payload.split_at_checked(nul).unwrap_or((payload, &[]));
                // `tail` starts with the NUL; everything after it is `rest`.
                let rest = tail.get(1..).unwrap_or(&[]);
                Ascii {
                    text,
                    terminated: true,
                    rest,
                }
            }
            None => Ascii {
                text: payload,
                terminated: false,
                rest: &[],
            },
        }
    }

    /// Bytes before the first NUL (all of the payload if there is none).
    pub fn as_bytes(&self) -> &'a [u8] {
        self.text
    }

    /// Whether a NUL was found (TIFF says ASCII ends with one; real files
    /// often do not).
    pub fn is_terminated(&self) -> bool {
        self.terminated
    }

    /// Bytes after the first NUL (further strings, padding or garbage); empty
    /// if unterminated.
    pub fn rest(&self) -> &'a [u8] {
        self.rest
    }

    /// `Ok` only if [`Ascii::as_bytes`] is valid UTF-8 (camera strings are
    /// often Latin-1).
    pub fn to_str(&self) -> Result<&'a str, core::str::Utf8Error> {
        core::str::from_utf8(self.text)
    }

    /// Invalid sequences replaced with U+FFFD; borrows when already valid.
    pub fn to_string_lossy(&self) -> std::borrow::Cow<'a, str> {
        String::from_utf8_lossy(self.text)
    }
}

/// Size in bytes of one value of a TIFF field type (TIFF 6.0 §2, "Types";
/// type 13 `IFD` from TIFF/EP). `None` for a type this reader does not know,
/// TIFF says a reader should skip such a field rather than reject the file, so
/// the entry is still recorded and only reading its payload fails.
pub(super) fn type_size(field_type: u16) -> Option<u64> {
    match field_type {
        1 | 2 | 6 | 7 => Some(1),   // BYTE, ASCII, SBYTE, UNDEFINED
        3 | 8 => Some(2),           // SHORT, SSHORT
        4 | 9 | 11 | 13 => Some(4), // LONG, SLONG, FLOAT, IFD
        5 | 10 | 12 => Some(8),     // RATIONAL, SRATIONAL, DOUBLE
        _ => None,
    }
}

impl<'a> Container<'a> {
    /// An entry's values widened to `u32`.
    ///
    /// Handles `BYTE`, `SHORT`, `LONG`, `IFD` and `UNDEFINED`. Every other
    /// type, `RATIONAL` included, is a typed [`Error::UnexpectedFieldType`]
    /// rather than a silent zero; rationals have their own readers
    /// ([`Container::urationals`], [`Container::srationals`]).
    pub fn uints(&self, entry: &Entry) -> Result<Vec<u32>, Error> {
        // The field type is a property of the entry, so it is checked before
        // the buffer is touched: otherwise a RATIONAL that also happens to
        // point out of bounds reports `Truncated` and hides the real problem.
        match entry.field_type {
            TYPE_BYTE | TYPE_UNDEFINED | TYPE_SHORT | TYPE_LONG | TYPE_IFD => {}
            other => {
                return Err(Error::UnexpectedFieldType {
                    tag: entry.tag,
                    field_type: other,
                })
            }
        }

        let count =
            usize::try_from(entry.count).map_err(|_| Error::ValueOverflow { tag: entry.tag })?;
        if count > MAX_TAG_VALUES {
            return Err(Error::TagTooLarge {
                tag: entry.tag,
                count: entry.count,
                limit: MAX_TAG_VALUES,
            });
        }

        let bytes = self.payload(entry)?;
        let mut out = Vec::with_capacity(count);
        match entry.field_type {
            TYPE_BYTE | TYPE_UNDEFINED => {
                for b in bytes {
                    out.push(u32::from(*b));
                }
            }
            TYPE_SHORT => {
                // `as_chunks` yields `&[u8; 2]`, so the `try_into` that could
                // never fail is gone rather than merely unreachable.
                for chunk in bytes.as_chunks::<2>().0 {
                    out.push(u32::from(self.byte_order.u16(*chunk)));
                }
            }
            TYPE_LONG | TYPE_IFD => {
                for chunk in bytes.as_chunks::<4>().0 {
                    out.push(self.byte_order.u32(*chunk));
                }
            }
            // Unreachable given the guard above, and kept because the match
            // must stay total if a type is ever added to one arm and not the
            // other.
            other => {
                return Err(Error::UnexpectedFieldType {
                    tag: entry.tag,
                    field_type: other,
                })
            }
        }
        Ok(out)
    }

    /// An entry's values as unsigned [`URational`]s, verbatim.
    ///
    /// Accepts `RATIONAL` (type 5) only; any other type is
    /// [`Error::UnexpectedFieldType`]. Nothing is divided or narrowed, so
    /// `0/0` and values above `i32::MAX` are returned as the file wrote them.
    ///
    /// Same bounds discipline as [`Container::uints`]: `count` is capped by
    /// [`MAX_TAG_VALUES`], the payload is read bounds-checked, and the 8-byte
    /// values are taken with `as_chunks::<8>` so the width is a type-level
    /// fact.
    pub fn urationals(&self, entry: &Entry) -> Result<Vec<URational>, Error> {
        self.rational_chunks(entry, TYPE_RATIONAL, |order, num, den| URational {
            num: order.u32(num),
            den: order.u32(den),
        })
    }

    /// An entry's values as signed [`SRational`]s, verbatim.
    ///
    /// Accepts `SRATIONAL` (type 10) only; same discipline as
    /// [`Container::urationals`].
    pub fn srationals(&self, entry: &Entry) -> Result<Vec<SRational>, Error> {
        self.rational_chunks(entry, TYPE_SRATIONAL, |order, num, den| SRational {
            num: order.i32(num),
            den: order.i32(den),
        })
    }

    /// Shared body of the two rational readers: type gate, count cap, bounds-
    /// checked payload, then `build` over each 8-byte `(numerator, denominator)`.
    fn rational_chunks<T>(
        &self,
        entry: &Entry,
        accepted: u16,
        build: impl Fn(crate::reader::ByteOrder, [u8; 4], [u8; 4]) -> T,
    ) -> Result<Vec<T>, Error> {
        // Type is a property of the entry, checked before the buffer is
        // touched, same ordering reason as `uints`.
        if entry.field_type != accepted {
            return Err(Error::UnexpectedFieldType {
                tag: entry.tag,
                field_type: entry.field_type,
            });
        }

        let count =
            usize::try_from(entry.count).map_err(|_| Error::ValueOverflow { tag: entry.tag })?;
        if count > MAX_TAG_VALUES {
            return Err(Error::TagTooLarge {
                tag: entry.tag,
                count: entry.count,
                limit: MAX_TAG_VALUES,
            });
        }

        let bytes = self.payload(entry)?;
        let mut out = Vec::with_capacity(count);
        for chunk in bytes.as_chunks::<8>().0 {
            // Destructured, not indexed: `as_chunks` makes the eight bytes a
            // type-level fact, so no fallible `get`/`try_into` is needed.
            let [n0, n1, n2, n3, d0, d1, d2, d3] = *chunk;
            out.push(build(self.byte_order, [n0, n1, n2, n3], [d0, d1, d2, d3]));
        }
        Ok(out)
    }

    /// ASCII (type 2) only.
    ///
    /// The text is the payload up to its first NUL; see [`Ascii`]. Count 0 is
    /// an empty, unterminated value. The count is a byte count and is capped
    /// by [`MAX_TAG_VALUES`]. `BYTE` and `UNDEFINED` are refused with
    /// [`Error::UnexpectedFieldType`]; read those with [`Container::payload`].
    pub fn ascii(&self, entry: &Entry) -> Result<Ascii<'a>, Error> {
        let (bytes, _) = self.gated_payload(entry, |t| t == TYPE_ASCII)?;
        Ok(Ascii::split(bytes))
    }

    /// `SBYTE` (6), `SSHORT` (8) and `SLONG` (9), sign-extended to `i32`.
    /// Unsigned types are refused: signedness is the type's.
    pub fn sints(&self, entry: &Entry) -> Result<Vec<i32>, Error> {
        let (bytes, count) = self.gated_payload(entry, |t| {
            matches!(t, TYPE_SBYTE | TYPE_SSHORT | TYPE_SLONG)
        })?;
        let mut out = Vec::with_capacity(count);
        match entry.field_type {
            TYPE_SBYTE => {
                for b in bytes {
                    out.push(i32::from(i8::from_ne_bytes([*b])));
                }
            }
            TYPE_SSHORT => {
                for chunk in bytes.as_chunks::<2>().0 {
                    out.push(i32::from(self.byte_order.i16(*chunk)));
                }
            }
            _ => {
                for chunk in bytes.as_chunks::<4>().0 {
                    out.push(self.byte_order.i32(*chunk));
                }
            }
        }
        Ok(out)
    }

    /// `FLOAT` (11) and `DOUBLE` (12), widened to `f64`.
    ///
    /// No arithmetic is done: NaN (payload included), infinities, subnormals
    /// and `-0.0` come back as the file wrote them, so callers must check
    /// `is_finite`.
    pub fn floats(&self, entry: &Entry) -> Result<Vec<f64>, Error> {
        let (bytes, count) =
            self.gated_payload(entry, |t| matches!(t, TYPE_FLOAT | TYPE_DOUBLE))?;
        let mut out = Vec::with_capacity(count);
        if entry.field_type == TYPE_FLOAT {
            for chunk in bytes.as_chunks::<4>().0 {
                out.push(f64::from(self.byte_order.f32(*chunk)));
            }
        } else {
            for chunk in bytes.as_chunks::<8>().0 {
                out.push(self.byte_order.f64(*chunk));
            }
        }
        Ok(out)
    }

    /// Shared prelude of `ascii`, `sints` and `floats`, in the one order every
    /// reader uses: type gate (a property of the entry, before the buffer is
    /// touched), count cap, then the bounds-checked payload. Returns the
    /// payload and the validated count.
    fn gated_payload(
        &self,
        entry: &Entry,
        accepts: impl Fn(u16) -> bool,
    ) -> Result<(&'a [u8], usize), Error> {
        if !accepts(entry.field_type) {
            return Err(Error::UnexpectedFieldType {
                tag: entry.tag,
                field_type: entry.field_type,
            });
        }
        let count =
            usize::try_from(entry.count).map_err(|_| Error::ValueOverflow { tag: entry.tag })?;
        if count > MAX_TAG_VALUES {
            return Err(Error::TagTooLarge {
                tag: entry.tag,
                count: entry.count,
                limit: MAX_TAG_VALUES,
            });
        }
        Ok((self.payload(entry)?, count))
    }
}
