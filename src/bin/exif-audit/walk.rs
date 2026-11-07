//! The walk (DEC-011 part 3): the `PATH` arguments become a lazy, ordered
//! sequence of candidates. With `read.rs`, the only `std::fs` code of the
//! binary.
//!
//! * Arguments are processed in the order given; a path named twice appears
//!   twice. A named path is followed (`metadata`), so a named symlink to a file
//!   or a directory works.
//! * A directory is walked depth-first, entries sorted by name (the platform's
//!   `OsString` order), hidden entries included, at most [`MAX_DEPTH`] levels
//!   below its argument.
//! * An entry found in a walk is looked at with `symlink_metadata`: a symlink
//!   is reported, never followed; a FIFO, socket or device is reported, never
//!   opened; nothing is skipped silently.
//! * Labels are the argument as written joined with the names below it, never
//!   canonicalised.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use exif_core::report::FailReason;

use crate::read::{io_failure, Failure, DEPTH, NOT_REGULAR, SYMLINK};

/// Directories deeper than this below their argument are reported, not entered.
pub const MAX_DEPTH: usize = 64;

/// What a candidate is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// A regular file to read.
    File,
    /// Reported as `failed`; never opened.
    Failed(Failure),
}

/// One path to report on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The label and the path to open: the same value.
    pub path: PathBuf,
    pub item: Item,
}

struct Level {
    dir: PathBuf,
    names: std::vec::IntoIter<OsString>,
    depth: usize,
}

/// The lazy walk over the arguments.
pub struct Walker<'a> {
    args: std::slice::Iter<'a, OsString>,
    stack: Vec<Level>,
}

impl<'a> Walker<'a> {
    pub fn new(paths: &'a [OsString]) -> Walker<'a> {
        Walker {
            args: paths.iter(),
            stack: Vec::new(),
        }
    }

    /// List `dir` and make it the top level. Returns the candidate that
    /// reports a listing that failed (wholly or partly).
    fn enter(&mut self, dir: PathBuf, depth: usize) -> Option<Candidate> {
        let mut names: Vec<OsString> = Vec::new();
        let mut failure: Option<Failure> = None;
        match fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(e) => names.push(e.file_name()),
                        Err(e) => failure = Some(io_failure(e.kind(), true)),
                    }
                }
            }
            Err(e) => failure = Some(io_failure(e.kind(), false)),
        }
        names.sort();
        let reported = failure.map(|f| Candidate {
            path: dir.clone(),
            item: Item::Failed(f),
        });
        self.stack.push(Level {
            dir,
            names: names.into_iter(),
            depth,
        });
        reported
    }

    /// An entry found while walking a directory at `parent_depth`.
    fn found(&mut self, path: PathBuf, parent_depth: usize) -> Option<Candidate> {
        let failed = |path: PathBuf, f: Failure| {
            Some(Candidate {
                path,
                item: Item::Failed(f),
            })
        };
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => return failed(path, io_failure(e.kind(), false)),
        };
        let ft = meta.file_type();
        if ft.is_symlink() {
            failed(path, Failure::new(FailReason::NotRegularFile, SYMLINK))
        } else if ft.is_dir() {
            let depth = parent_depth.saturating_add(1);
            if depth > MAX_DEPTH {
                failed(path, Failure::new(FailReason::Other, DEPTH))
            } else {
                self.enter(path, depth)
            }
        } else if ft.is_file() {
            Some(Candidate {
                path,
                item: Item::File,
            })
        } else {
            failed(path, Failure::new(FailReason::NotRegularFile, NOT_REGULAR))
        }
    }

    /// A path named on the command line: followed.
    fn named(&mut self, arg: &OsStr) -> Option<Candidate> {
        let path = Path::new(arg).to_path_buf();
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                return Some(Candidate {
                    path,
                    item: Item::Failed(io_failure(e.kind(), false)),
                })
            }
        };
        if meta.is_dir() {
            self.enter(path, 0)
        } else if meta.is_file() {
            Some(Candidate {
                path,
                item: Item::File,
            })
        } else {
            Some(Candidate {
                path,
                item: Item::Failed(Failure::new(FailReason::NotRegularFile, NOT_REGULAR)),
            })
        }
    }
}

impl Iterator for Walker<'_> {
    type Item = Candidate;

    fn next(&mut self) -> Option<Candidate> {
        loop {
            let step = match self.stack.last_mut() {
                Some(level) => match level.names.next() {
                    Some(name) => Some((level.dir.join(name), level.depth)),
                    None => {
                        self.stack.pop();
                        continue;
                    }
                },
                None => None,
            };
            let produced = match step {
                Some((path, depth)) => self.found(path, depth),
                None => {
                    let arg = self.args.next()?;
                    self.named(arg)
                }
            };
            if produced.is_some() {
                return produced;
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;
    use crate::read::tests::Tmp;

    fn labels(paths: &[OsString]) -> Vec<String> {
        Walker::new(paths)
            .map(|c| {
                let tag = match c.item {
                    Item::File => "file".to_string(),
                    Item::Failed(f) => format!("failed:{}", f.detail),
                };
                format!("{} {tag}", c.path.display())
            })
            .collect()
    }

    #[test]
    fn order_is_sorted_by_name_and_arguments_keep_their_order() {
        let tmp = Tmp::new("walk");
        let root = &tmp.0;
        fs::create_dir_all(root.join("m/sub")).unwrap();
        fs::create_dir_all(root.join("z-empty")).unwrap();
        for f in ["b", "a", "Z", ".hidden", "m/y", "m/x", "m/sub/q", "m/sub/p"] {
            fs::write(root.join(f), b"x").unwrap();
        }
        let r = root.display().to_string();
        let got = labels(&[root.as_os_str().to_owned()]);
        // Bytewise name order: ".hidden" < "Z" < "a" < "b" < "m" < "z-empty";
        // a directory is expanded in place; hidden entries are scanned.
        let want: Vec<String> = [".hidden", "Z", "a", "b", "m/sub/p", "m/sub/q", "m/x", "m/y"]
            .iter()
            .map(|n| format!("{r}/{n} file"))
            .collect();
        assert_eq!(got, want);
        // Arguments keep their order, and a path named twice appears twice.
        let a = root.join("b").into_os_string();
        let b = root.join("a").into_os_string();
        let got = labels(&[a.clone(), b.clone(), a.clone()]);
        assert_eq!(
            got,
            vec![
                format!("{r}/b file"),
                format!("{r}/a file"),
                format!("{r}/b file")
            ]
        );
        // A missing argument is a failed candidate and the next one still runs.
        let missing = root.join("nope").into_os_string();
        let got = labels(&[missing, b]);
        assert_eq!(
            got,
            vec![format!("{r}/nope failed:not found"), format!("{r}/a file")]
        );
    }

    #[test]
    fn depth_limit_enters_64_levels_and_reports_the_65th() {
        let tmp = Tmp::new("depth");
        let mut p = tmp.0.clone();
        for _ in 0..65 {
            p.push("d");
        }
        fs::create_dir_all(&p).unwrap();
        // A file at depth 64 (inside the 64th subdirectory).
        let mut at64 = tmp.0.clone();
        for _ in 0..64 {
            at64.push("d");
        }
        fs::write(at64.join("f"), b"x").unwrap();
        let got = labels(&[tmp.0.as_os_str().to_owned()]);
        assert_eq!(got.len(), 2, "{got:?}");
        // In name order: the 65th directory "d" comes before the file "f".
        assert!(
            got[0].ends_with("/d failed:directory depth limit"),
            "{got:?}"
        );
        assert!(got[1].ends_with("/d/f file"), "{got:?}");
    }
}
