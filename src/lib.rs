//! `exif-core`: a panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata
//! reader, writer and stripper. Pre-release: the crate reads TIFF ([`tiff`]:
//! tolerant walk, DEC-001), walks JPEG segments and locates its metadata
//! regions ([`jpeg`]), and builds a read-only privacy audit over both
//! ([`audit`]: container sniff, inventory, PII findings), and strips JPEG
//! segment by segment ([`write`]: `strip` and `sanitize`, fail closed). PNG is
//! walked chunk by chunk ([`png`], DEC-017) and its metadata is located by
//! [`png::read`]: Exif from `eXIf` through the TIFF engine, and borrowed,
//! never-decoded XMP, ICC, raw-profile and text regions (DEC-018). The audit
//! does not read PNG yet. WebP is walked chunk by
//! chunk ([`webp`], DEC-019: the RIFF size never limits the walk); the audit does
//! not read WebP yet.

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
pub mod png;
pub mod policy;
pub mod reader;
pub mod report;
pub mod tiff;
pub mod webp;
pub mod write;

pub use error::{Error, Result};
pub use policy::{StripPolicy, StripSpec};
pub use write::{sanitize, strip, StripError, Stripped};
