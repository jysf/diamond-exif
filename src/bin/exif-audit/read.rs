//! Reading one file (DEC-011 part 3 and 4). With `walk.rs`, the only
//! `std::fs` code of the binary.
//!
//! Sniff first: the first 12 bytes are read and sniffed with the library's
//! `sniff`. A container the library does not read is answered from those bytes
//! alone and the rest of the file is never read. A TIFF or a JPEG is read
//! whole, bounded by `--max-size`.
//!
//! Every failure text is one of the closed list below, chosen by
//! `io::ErrorKind` or by the condition, never from the operating system's own
//! message (it varies by host and can quote a path).

use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::Path;

use exif_core::audit::{sniff, Sniffed};
use exif_core::report::FailReason;

/// The closed list of `detail` texts of a `failed` entry (DEC-011 part 4).
pub const NOT_FOUND: &str = "not found";
pub const PERMISSION_DENIED: &str = "permission denied";
pub const TOO_LARGE: &str = "larger than --max-size";
pub const NOT_REGULAR: &str = "not a regular file";
pub const SYMLINK: &str = "symlink not followed";
pub const DEPTH: &str = "directory depth limit";
pub const READ_ERROR: &str = "read error";
pub const IO_ERROR: &str = "i/o error";

/// All eight, for the tests that pin the list.
#[cfg(test)]
pub const CLOSED_LIST: [&str; 8] = [
    NOT_FOUND,
    PERMISSION_DENIED,
    TOO_LARGE,
    NOT_REGULAR,
    SYMLINK,
    DEPTH,
    READ_ERROR,
    IO_ERROR,
];

/// Bytes sniffed before anything else is read (the longest magic is 12).
const HEAD_LEN: u64 = 12;

/// Why a file has no audit: a reason and one text of the closed list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub reason: FailReason,
    pub detail: &'static str,
}

impl Failure {
    pub const fn new(reason: FailReason, detail: &'static str) -> Failure {
        Failure { reason, detail }
    }
}

/// What reading a regular file gave.
#[derive(Debug)]
pub enum Loaded {
    /// A TIFF or a JPEG, whole: audit it.
    Whole(Vec<u8>),
    /// A container the library does not read (or an empty or unknown file):
    /// the first bytes only; `audit` of them is the `unsupported` entry.
    Head(Vec<u8>),
    /// No audit.
    Failed(Failure),
}

/// The failure for an I/O error, by kind alone. `reading` is true once the
/// file is open and a `read` failed (`read error`); otherwise the unrecognised
/// kinds are `i/o error`.
pub fn io_failure(kind: ErrorKind, reading: bool) -> Failure {
    let detail = match kind {
        ErrorKind::NotFound => NOT_FOUND,
        ErrorKind::PermissionDenied => PERMISSION_DENIED,
        _ if reading => READ_ERROR,
        _ => IO_ERROR,
    };
    Failure::new(FailReason::Unreadable, detail)
}

/// Read `path`, a candidate the walk found to be a regular file (or a named
/// path that is one). The regular-file check is repeated on the open handle: a
/// path can change between the check and the open.
pub fn read_file(path: &Path, max_size: u64) -> Loaded {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Loaded::Failed(io_failure(e.kind(), false)),
    };
    let len = match file.metadata() {
        Ok(m) if m.is_file() => m.len(),
        Ok(_) => return Loaded::Failed(Failure::new(FailReason::NotRegularFile, NOT_REGULAR)),
        Err(e) => return Loaded::Failed(io_failure(e.kind(), false)),
    };
    let mut head: Vec<u8> = Vec::new();
    if let Err(e) = (&mut file).take(HEAD_LEN).read_to_end(&mut head) {
        return Loaded::Failed(io_failure(e.kind(), true));
    }
    if !matches!(sniff(&head), Sniffed::Tiff | Sniffed::Jpeg) {
        return Loaded::Head(head);
    }
    // A file longer than the cap is rejected from its length without reading
    // it; the bounded read below is the guard for a file that lies about it.
    if len > max_size {
        return Loaded::Failed(Failure::new(FailReason::TooLarge, TOO_LARGE));
    }
    let have = u64::try_from(head.len()).unwrap_or(HEAD_LEN);
    // One byte more than the cap is enough to know the file is too large.
    let room = max_size.saturating_add(1).saturating_sub(have);
    let expected = len.min(max_size).saturating_sub(have);
    let reserve = usize::try_from(expected).unwrap_or(usize::MAX);
    if head.try_reserve_exact(reserve).is_err() {
        return Loaded::Failed(Failure::new(FailReason::Unreadable, READ_ERROR));
    }
    if let Err(e) = (&mut file).take(room).read_to_end(&mut head) {
        return Loaded::Failed(io_failure(e.kind(), true));
    }
    if u64::try_from(head.len()).map_or(true, |n| n > max_size) {
        return Loaded::Failed(Failure::new(FailReason::TooLarge, TOO_LARGE));
    }
    Loaded::Whole(head)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
pub(crate) mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// A unique temp directory, removed on drop (also when a test fails).
    pub(crate) struct Tmp(pub PathBuf);

    impl Tmp {
        pub(crate) fn new(tag: &str) -> Tmp {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "exif-audit-unit-{}-{}-{tag}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&p).unwrap();
            Tmp(p)
        }
    }

    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn failure_texts_are_the_closed_list_and_never_the_os_message() {
        // Every kind maps into the closed list, in both phases.
        let kinds = [
            ErrorKind::NotFound,
            ErrorKind::PermissionDenied,
            ErrorKind::ConnectionRefused,
            ErrorKind::AlreadyExists,
            ErrorKind::InvalidInput,
            ErrorKind::InvalidData,
            ErrorKind::TimedOut,
            ErrorKind::UnexpectedEof,
            ErrorKind::Interrupted,
            ErrorKind::OutOfMemory,
            ErrorKind::Unsupported,
            ErrorKind::Other,
        ];
        for kind in kinds {
            for reading in [false, true] {
                let f = io_failure(kind, reading);
                assert!(CLOSED_LIST.contains(&f.detail), "{kind:?}: {}", f.detail);
                assert_eq!(f.reason, FailReason::Unreadable);
            }
        }
        // The pinned cells.
        assert_eq!(io_failure(ErrorKind::NotFound, false).detail, "not found");
        assert_eq!(io_failure(ErrorKind::NotFound, true).detail, "not found");
        assert_eq!(io_failure(ErrorKind::PermissionDenied, false).detail, "permission denied");
        assert_eq!(io_failure(ErrorKind::Other, false).detail, "i/o error");
        assert_eq!(io_failure(ErrorKind::Other, true).detail, "read error");
        assert_eq!(io_failure(ErrorKind::InvalidData, true).detail, "read error");
        // The list itself, typed from DEC-011 part 4, all ASCII.
        assert_eq!(
            CLOSED_LIST,
            [
                "not found",
                "permission denied",
                "larger than --max-size",
                "not a regular file",
                "symlink not followed",
                "directory depth limit",
                "read error",
                "i/o error"
            ]
        );
        // A real error: the text is the list's, not the OS message (which for
        // a missing file reads "No such file or directory (os error 2)" and
        // quotes nothing, but differs by host).
        let tmp = Tmp::new("texts");
        let missing = tmp.0.join("a-name-that-must-not-leak");
        let os_message = File::open(&missing).unwrap_err().to_string();
        match read_file(&missing, 100) {
            Loaded::Failed(f) => {
                assert_eq!(f.detail, "not found");
                assert_ne!(f.detail, os_message);
                assert!(!f.detail.contains("os error"));
            }
            other => panic!("{other:?}"),
        }
        // A directory opened as a file: not a regular file (unix opens it).
        match read_file(&tmp.0, 100) {
            Loaded::Failed(f) => {
                assert!(CLOSED_LIST.contains(&f.detail), "{}", f.detail);
                assert!(!f.detail.contains("os error"));
            }
            other => panic!("{other:?}"),
        }
    }

    fn jpeg_of(len: usize) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8];
        v.resize(len, 0x41);
        v
    }

    #[test]
    fn sniff_first_and_size_cap_are_exact() {
        let tmp = Tmp::new("read");
        let p = |n: &str| tmp.0.join(n);
        // Exactly the cap: read. One more: too large.
        fs::write(p("at-cap.jpg"), jpeg_of(100)).unwrap();
        fs::write(p("over-cap.jpg"), jpeg_of(101)).unwrap();
        assert!(matches!(read_file(&p("at-cap.jpg"), 100), Loaded::Whole(v) if v.len() == 100));
        assert!(matches!(
            read_file(&p("over-cap.jpg"), 100),
            Loaded::Failed(Failure { reason: FailReason::TooLarge, detail: "larger than --max-size" })
        ));
        // A cap smaller than the head: a 5-byte JPEG at cap 4 is too large, at 5 read.
        fs::write(p("five.jpg"), jpeg_of(5)).unwrap();
        assert!(matches!(read_file(&p("five.jpg"), 5), Loaded::Whole(v) if v.len() == 5));
        assert!(matches!(read_file(&p("five.jpg"), 4), Loaded::Failed(f) if f.reason == FailReason::TooLarge));
        // A non-image larger than the cap is answered from its head.
        let mut big = b"\x89PNG\r\n\x1a\n".to_vec();
        big.resize(10_000, 0);
        fs::write(p("big.png"), &big).unwrap();
        match read_file(&p("big.png"), 100) {
            Loaded::Head(h) => assert_eq!(h.len(), 12),
            other => panic!("{other:?}"),
        }
        // Empty and short files.
        fs::write(p("empty"), b"").unwrap();
        assert!(matches!(read_file(&p("empty"), 100), Loaded::Head(h) if h.is_empty()));
        fs::write(p("short"), b"abc").unwrap();
        assert!(matches!(read_file(&p("short"), 100), Loaded::Head(h) if h == b"abc"));
        // A TIFF is read whole.
        let tiff = b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00".to_vec();
        fs::write(p("t.tif"), &tiff).unwrap();
        assert!(matches!(read_file(&p("t.tif"), 100), Loaded::Whole(v) if v == tiff));
    }
}
