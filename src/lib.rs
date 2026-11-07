//! `exif-core`: a panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata
//! reader, writer and stripper. Pre-release: so far the crate holds the strict
//! TIFF/IFD skeleton ([`tiff::Container`]) and its error type; every bad IFD is
//! still an `Err` (the tolerant walk of DEC-001 arrives in a later spec).

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
