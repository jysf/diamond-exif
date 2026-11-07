//! `exif-core`: a panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata
//! reader, writer and stripper. Pre-release: the crate reads TIFF ([`tiff`]:
//! tolerant walk, DEC-001), walks JPEG segments and locates its metadata
//! regions ([`jpeg`]), and builds a read-only privacy audit over both
//! ([`audit`]: container sniff, inventory, PII findings). No writer yet.

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

pub use error::{Error, Result};
pub use policy::{StripPolicy, StripSpec};
