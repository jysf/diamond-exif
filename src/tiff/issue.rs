//! Recoverable problems found while walking a container (DEC-001).
//!
//! The tolerant walk never fails because a non-IFD0 IFD is bad; it omits that
//! IFD and records a [`ParseIssue`] on the returned container instead. This
//! module is net-new policy, not a port (DEC-003).

use core::fmt;

use crate::error::Error;

/// How the walk reached (or tried to reach) the IFD that went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Link {
    /// The "next IFD" word at the end of an IFD (the IFD0 chain: IFD1, ...).
    Next,
    /// A pointer tag: `SubIFDs` (330), or the Exif (0x8769), GPS (0x8825) or
    /// Interop (0xA005) pointer (DEC-005).
    Tag(u16),
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Link::Next => write!(f, "next-IFD pointer"),
            Link::Tag(tag) => write!(f, "tag {tag:#06x}"),
        }
    }
}

/// One recoverable problem found while walking (DEC-001).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseIssue {
    error: Error,
    link: Link,
    from_ifd: usize,
    offset: Option<u32>,
}

impl ParseIssue {
    pub(crate) fn new(
        error: Error,
        link: Link,
        from_ifd: usize,
        offset: Option<u32>,
    ) -> ParseIssue {
        ParseIssue {
            error,
            link,
            from_ifd,
            offset,
        }
    }

    /// What went wrong: the existing typed error.
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Which kind of pointer was being followed.
    pub fn link(&self) -> Link {
        self.link
    }

    /// Index into `Container::ifds()` of the IFD that held the pointer.
    /// Always a recorded IFD, so `ifds().get(from_ifd)` is `Some`.
    pub fn from_ifd(&self) -> usize {
        self.from_ifd
    }

    /// Offset the pointer led to, or `None` when the pointer value itself
    /// could not be read (a malformed pointer tag).
    pub fn offset(&self) -> Option<u32> {
        self.offset
    }

    /// Consume the issue, yielding its error by value (for `parse_strict`).
    pub(crate) fn into_error(self) -> Error {
        self.error
    }
}

impl fmt::Display for ParseIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (following {} from IFD #{}",
            self.error, self.link, self.from_ifd
        )?;
        if let Some(offset) = self.offset {
            write!(f, ", to offset {offset}")?;
        }
        write!(f, ")")
    }
}
