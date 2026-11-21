//! Recoverable problems found while walking a PNG (DEC-017).
//!
//! A third issue type, kept apart from `JpegIssue` and the TIFF `ParseIssue`
//! so a caller can tell which layer found the problem.

use core::fmt;

use crate::error::Error;

/// One recoverable problem found while walking a PNG chunk stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PngIssue {
    error: Error,
    offset: usize,
}

impl PngIssue {
    pub(crate) fn new(error: Error, offset: usize) -> PngIssue {
        PngIssue { error, offset }
    }

    /// What went wrong: the existing typed error.
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Input offset where the problem was found: the chunk's own offset for a
    /// chunk-level issue, the end of the input for a missing chunk.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for PngIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at offset {})", self.error, self.offset)
    }
}
