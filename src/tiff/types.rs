//! TIFF field types and the integer / rational readers over a payload.
//!
//! Provenance: copied from irradiance `src/ifd.rs:186-215` (type constants and
//! `type_size`) and `:1025-1193` (`uints`, `rationals`), then edited (DEC-003):
//! the DNG per-tag `RATIONAL` gate and integer-division rule are removed, and
//! `RATIONAL`/`SRATIONAL` are held verbatim as unsigned/signed pairs.

use super::{Container, Entry, MAX_TAG_VALUES};
use crate::error::Error;

// TIFF 6.0 §2, "Types". Only the types a reader in this module accepts are
// named; the rest are sized by `type_size` and refused by the readers.

/// TIFF 6.0 §2: 8-bit unsigned integer.
pub const TYPE_BYTE: u16 = 1;
/// TIFF 6.0 §2: 16-bit unsigned integer.
pub const TYPE_SHORT: u16 = 3;
/// TIFF 6.0 §2: 32-bit unsigned integer.
pub const TYPE_LONG: u16 = 4;
/// TIFF 6.0 §2: two `LONG`s, numerator then denominator.
pub const TYPE_RATIONAL: u16 = 5;
/// TIFF 6.0 §2: an 8-bit byte that may contain anything.
pub const TYPE_UNDEFINED: u16 = 7;
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
}
