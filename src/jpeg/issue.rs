//! Recoverable problems found while walking a JPEG (DEC-006).
//!
//! Kept apart from the TIFF-level `ParseIssue`: the layers do not lose their
//! identity; a caller sees both at once through `AnyIssue` (SPEC-010).

use core::fmt;

use crate::error::Error;

/// One recoverable problem found while walking a JPEG segment stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct JpegIssue {
    error: Error,
    offset: usize,
}

impl JpegIssue {
    pub(crate) fn new(error: Error, offset: usize) -> JpegIssue {
        JpegIssue { error, offset }
    }

    /// What went wrong: the existing typed error.
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Input offset where the problem was found.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for JpegIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at offset {})", self.error, self.offset)
    }
}
