//! Recoverable problems found while walking a WebP (DEC-019).
//!
//! A fourth issue type, kept apart from `JpegIssue`, `PngIssue` and the TIFF
//! `ParseIssue` so a caller can tell which layer found the problem.

use core::fmt;

use crate::error::Error;

/// One recoverable problem found while walking a WebP RIFF stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WebpIssue {
    error: Error,
    offset: usize,
}

impl WebpIssue {
    pub(crate) fn new(error: Error, offset: usize) -> WebpIssue {
        WebpIssue { error, offset }
    }

    /// What went wrong: the existing typed error.
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Input offset where the problem was found: the chunk's own offset for a
    /// chunk-level issue, 4 for a RIFF size mismatch, the offset of the `VP8X`
    /// chunk for a flag mismatch.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for WebpIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at offset {})", self.error, self.offset)
    }
}
