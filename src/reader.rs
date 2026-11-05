//! Byte order and bounds-checked fixed-width reads over `&[u8]`.
//!
//! Provenance: copied from irradiance `src/ifd.rs:262-322` (DEC-003). Only the
//! reads with a consumer are here; the signed and float reads arrived with
//! their decoders (SPEC-007).

use crate::error::Error;

/// TIFF 6.0 §2: the container's byte order, from the `II`/`MM` header mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    /// `II`: little-endian (Intel).
    Little,
    /// `MM`: big-endian (Motorola).
    Big,
}

impl ByteOrder {
    pub(crate) fn u16(self, bytes: [u8; 2]) -> u16 {
        match self {
            ByteOrder::Little => u16::from_le_bytes(bytes),
            ByteOrder::Big => u16::from_be_bytes(bytes),
        }
    }

    pub(crate) fn u32(self, bytes: [u8; 4]) -> u32 {
        match self {
            ByteOrder::Little => u32::from_le_bytes(bytes),
            ByteOrder::Big => u32::from_be_bytes(bytes),
        }
    }

    /// A **signed** 32-bit read: `SRATIONAL`'s numerator and denominator are
    /// `SLONG`s. The byte order is the file's, same as [`ByteOrder::u32`].
    pub(crate) fn i32(self, bytes: [u8; 4]) -> i32 {
        match self {
            ByteOrder::Little => i32::from_le_bytes(bytes),
            ByteOrder::Big => i32::from_be_bytes(bytes),
        }
    }
}

impl ByteOrder {
    /// A **signed** 16-bit read: `SSHORT`. The byte order is the file's.
    pub(crate) fn i16(self, bytes: [u8; 2]) -> i16 {
        match self {
            ByteOrder::Little => i16::from_le_bytes(bytes),
            ByteOrder::Big => i16::from_be_bytes(bytes),
        }
    }

    /// An IEEE 754 binary32 read: `FLOAT`. Bits are taken verbatim, so NaN
    /// payloads and signed zeros survive.
    pub(crate) fn f32(self, bytes: [u8; 4]) -> f32 {
        match self {
            ByteOrder::Little => f32::from_le_bytes(bytes),
            ByteOrder::Big => f32::from_be_bytes(bytes),
        }
    }

    /// An IEEE 754 binary64 read: `DOUBLE`. Same discipline as
    /// [`ByteOrder::f32`].
    pub(crate) fn f64(self, bytes: [u8; 8]) -> f64 {
        match self {
            ByteOrder::Little => f64::from_le_bytes(bytes),
            ByteOrder::Big => f64::from_be_bytes(bytes),
        }
    }
}

/// Read a `u16` at `at`. Bounds-checked with `.get()` and converted with
/// `try_into()`, never `.get()` then index.
pub(crate) fn read_u16(data: &[u8], at: usize, order: ByteOrder) -> Result<u16, Error> {
    let end = at.checked_add(2).ok_or(Error::Truncated { at, len: 2 })?;
    let bytes: [u8; 2] = data
        .get(at..end)
        .ok_or(Error::Truncated { at, len: 2 })?
        .try_into()
        .map_err(|_| Error::Truncated { at, len: 2 })?;
    Ok(order.u16(bytes))
}

/// Read a `u32` at `at`. Same discipline as [`read_u16`].
pub(crate) fn read_u32(data: &[u8], at: usize, order: ByteOrder) -> Result<u32, Error> {
    let end = at.checked_add(4).ok_or(Error::Truncated { at, len: 4 })?;
    let bytes: [u8; 4] = data
        .get(at..end)
        .ok_or(Error::Truncated { at, len: 4 })?
        .try_into()
        .map_err(|_| Error::Truncated { at, len: 4 })?;
    Ok(order.u32(bytes))
}

/// `u32` to `usize`, typed rather than `as`. Infallible on 64-bit; on a 32-bit
/// host a file offset above `usize::MAX` is genuinely unreadable, and this is
/// what says so instead of truncating silently.
pub(crate) fn offset_to_usize(offset: u32) -> Result<usize, Error> {
    usize::try_from(offset).map_err(|_| Error::OffsetOutOfRange { offset })
}
