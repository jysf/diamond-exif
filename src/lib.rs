//! `exif-core`: a panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata
//! reader, writer and stripper. Pre-release: the crate reads TIFF ([`tiff`]:
//! tolerant walk, DEC-001), walks JPEG segments and locates its metadata
//! regions ([`jpeg`]), and builds a read-only privacy audit over both
//! ([`audit`]: container sniff, inventory, PII findings), and strips JPEG,
//! TIFF and DNG ([`write`]: `strip` and `sanitize`, fail closed: JPEG segment
//! by segment, TIFF and DNG by in-place entry deletion).

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

pub mod audit;
pub mod error;
pub mod jpeg;
pub mod policy;
pub mod reader;
pub mod report;
pub mod tiff;
pub mod write;

pub use error::{Error, Result};
pub use policy::{StripPolicy, StripSpec};
pub use write::{sanitize, strip, StripError, Stripped};
