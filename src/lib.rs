//! `exif-core`: a panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata
//! reader, writer and stripper. Pre-release: the crate is a gated, empty
//! scaffold and exports nothing yet.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
