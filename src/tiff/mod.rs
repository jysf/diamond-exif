//! TIFF/IFD container reader: bounded, panic-free, cycle-guarded, tolerant.
//!
//! This module walks the container and nothing else: it finds the IFDs, reads
//! their entries, and resolves each entry's payload **inside the buffer it was
//! handed**. It decodes no pixels.
//!
//! Tolerant policy (DEC-001): only a bad header or an unreadable IFD0 makes
//! [`Container::parse`] return `Err`. Any other structurally bad IFD is omitted
//! and reported as a [`ParseIssue`] on the container, so one corrupt sub-IFD
//! cannot discard a readable IFD0. [`Container::parse_strict`] turns any issue
//! back into `Err` for callers that want the old all-or-nothing behaviour.
//!
//! # Provenance
//!
//! Copied from irradiance `src/ifd.rs` and edited (DNG removed, `payload`
//! returns slices tied to the input). The original was written from the TIFF
//! 6.0 specification (1992) §2 "Image File Header" and "Image File
//! Directory". See `docs/provenance-ledger.md` and DEC-003.
//!
//! # Panic policy, and what makes it mechanical
//!
//! Every offset, length and count in a TIFF comes from the file, which means it
//! comes from whoever made the file. So every one of them is checked:
//! `.get(range)` for every slice, `try_into()` for every fixed-width read
//! (never `.get()` followed by an index; that pattern is what the crate's
//! `#![deny(clippy::indexing_slicing)]` exists to reject), `checked_*` for
//! every product or sum that could overflow, and a typed [`Error`] on every
//! failure.
//!
//! # The three guards
//!
//! A TIFF is a graph, not a tree, and nothing in the format stops it being
//! cyclic:
//!
//! - **Cycle guard.** Every IFD offset reached is recorded; reaching one twice
//!   is an [`Error::CyclicIfd`] issue, not a second visit.
//! - **Depth guard.** `SubIFD` recursion stops at [`MAX_IFD_DEPTH`] with
//!   [`Error::IfdDepthExceeded`]. Native stack depth is bounded by that
//!   constant, which is why the recursion below is safe to write as recursion.
//! - **Count guard.** [`MAX_IFDS`] caps the total number of IFDs walked, so an
//!   acyclic-but-enormous chain cannot burn unbounded time, and
//!   [`MAX_TAG_VALUES`] caps how many values one entry can be asked to
//!   materialise, so a tag declaring 4 billion `LONG`s cannot allocate a Vec
//!   the size of the machine.

pub mod issue;
pub mod tags;
pub mod types;

pub use issue::{Link, ParseIssue};
pub use types::{SRational, URational};

use crate::error::Error;
use crate::reader::{offset_to_usize, read_u16, read_u32, ByteOrder};

// ─────────────────────────────────────────────────────────────────────────────
// Limits
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum `SubIFD` nesting depth. IFD0 is depth 0.
///
/// Real TIFFs nest one level (`IFD0` → `SubIFD`); TIFF/EP and Exif chains add
/// a second. Exif → Interop → MakerNote is deeper than that, but still three
/// or four levels at most, so eight holds: far past anything a camera writes
/// and far short of anything that could exhaust the stack.
pub const MAX_IFD_DEPTH: u32 = 8;

/// Maximum number of IFDs walked in one container.
///
/// The cycle guard already makes a walk terminate; this bounds how long
/// terminating takes on a file with a very long acyclic chain. The cycle
/// guard's `visited` list is searched linearly, which is fine at this size.
pub const MAX_IFDS: usize = 512;

/// Maximum number of values one entry may be asked to materialise.
///
/// Bounds allocation from a hostile `count` independently of the file's own
/// length: 1,048,576 values.
pub const MAX_TAG_VALUES: usize = 1 << 20;

/// TIFF 6.0 §2: the 42 that follows the byte-order mark.
const TIFF_VERSION: u16 = 42;

/// TIFF 6.0 §2: every IFD entry is exactly 12 bytes.
const ENTRY_SIZE: usize = 12;

// ─────────────────────────────────────────────────────────────────────────────
// Entries and IFDs
// ─────────────────────────────────────────────────────────────────────────────

/// One 12-byte IFD entry (TIFF 6.0 §2): tag, type, count, and a 4-byte field
/// that holds the value itself when it fits and an offset to it when it does
/// not.
///
/// The 4-byte field is kept **raw**. Whether it is a value or an offset
/// depends on `count × sizeof(type)`, which is exactly the product a hostile
/// file wants to overflow, so it is computed once, checked, in
/// [`Container::payload`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    tag: u16,
    field_type: u16,
    count: u32,
    value: [u8; 4],
    /// Where in the input the 4-byte value field sits. Lets an inline payload
    /// be returned as a slice of the input (DEC-002) rather than of this
    /// `Copy` entry, so every payload outlives the `Container`.
    value_at: usize,
}

impl Entry {
    /// The tag number.
    pub fn tag(&self) -> u16 {
        self.tag
    }

    /// The TIFF field type (TIFF 6.0 §2, "Types").
    pub fn field_type(&self) -> u16 {
        self.field_type
    }

    /// The declared value count. **Unvalidated**: this is a number from the
    /// file, and it is only trustworthy after [`Container::payload`] has
    /// checked it against the buffer.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Total payload size in bytes, or `None` for an unknown field type or a
    /// `count × size` that overflows.
    pub fn byte_len(&self) -> Option<u64> {
        types::type_size(self.field_type)?.checked_mul(u64::from(self.count))
    }

    /// `value_at` is the input offset of this entry's 4-byte value field.
    fn parse(chunk: &[u8], order: ByteOrder, value_at: usize) -> Result<Entry, Error> {
        let tag: [u8; 2] = chunk
            .get(0..2)
            .ok_or(Error::Truncated {
                at: 0,
                len: ENTRY_SIZE,
            })?
            .try_into()
            .map_err(|_| Error::Truncated {
                at: 0,
                len: ENTRY_SIZE,
            })?;
        let field_type: [u8; 2] = chunk
            .get(2..4)
            .ok_or(Error::Truncated {
                at: 2,
                len: ENTRY_SIZE,
            })?
            .try_into()
            .map_err(|_| Error::Truncated {
                at: 2,
                len: ENTRY_SIZE,
            })?;
        let count: [u8; 4] = chunk
            .get(4..8)
            .ok_or(Error::Truncated {
                at: 4,
                len: ENTRY_SIZE,
            })?
            .try_into()
            .map_err(|_| Error::Truncated {
                at: 4,
                len: ENTRY_SIZE,
            })?;
        let value: [u8; 4] = chunk
            .get(8..12)
            .ok_or(Error::Truncated {
                at: 8,
                len: ENTRY_SIZE,
            })?
            .try_into()
            .map_err(|_| Error::Truncated {
                at: 8,
                len: ENTRY_SIZE,
            })?;

        Ok(Entry {
            tag: order.u16(tag),
            field_type: order.u16(field_type),
            count: order.u32(count),
            value,
            value_at,
        })
    }
}

/// One Image File Directory: its entries, where it was found, and how it was
/// reached.
#[derive(Debug, Clone)]
pub struct Ifd {
    offset: u32,
    depth: u32,
    parent: Option<usize>,
    next: u32,
    entries: Vec<Entry>,
}

impl Ifd {
    /// Byte offset this IFD was read from.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// `SubIFD` nesting depth; IFD0 and its chain are depth 0.
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Index of the IFD whose `SubIFDs` tag pointed here, if any.
    pub fn parent(&self) -> Option<usize> {
        self.parent
    }

    /// Offset of the next IFD in this chain, or 0 for the last.
    pub fn next(&self) -> u32 {
        self.next
    }

    /// Every entry, in file order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The first entry with this tag, if present.
    pub fn entry(&self, tag: u16) -> Option<&Entry> {
        self.entries.iter().find(|e| e.tag == tag)
    }

    /// Whether this IFD carries a tag at all: presence without reading the
    /// payload.
    pub fn has(&self, tag: u16) -> bool {
        self.entry(tag).is_some()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The container
// ─────────────────────────────────────────────────────────────────────────────

/// A parsed TIFF container: every IFD reachable from IFD0, flattened.
///
/// Borrows the input; nothing is copied out of it except the 12-byte entry
/// headers.
#[derive(Debug, Clone)]
pub struct Container<'a> {
    data: &'a [u8],
    byte_order: ByteOrder,
    ifd0_offset: u32,
    ifds: Vec<Ifd>,
    issues: Vec<ParseIssue>,
}

/// Mutable state of one tolerant walk (private to `parse`).
struct Walk {
    /// Every IFD offset read (or being read), IFD0 included: the cycle guard.
    visited: Vec<u32>,
    /// Non-zero pointers followed so far, IFD0 counting as the first.
    attempts: usize,
    /// Set when the global budget is exhausted: ends the whole walk.
    halted: bool,
}

impl<'a> Container<'a> {
    /// Parse the header and walk every IFD reachable from IFD0, tolerantly
    /// (DEC-001).
    ///
    /// Only a bad header or an unreadable IFD0 (entry table or trailing
    /// next-word) is `Err`. Every other structural failure omits the bad IFD
    /// and is recorded as a [`ParseIssue`], see [`Container::issues`]. Depth
    /// first: an IFD is recorded, then its `SubIFDs` are walked, then the chain
    /// continues.
    pub fn parse(data: &'a [u8]) -> Result<Container<'a>, Error> {
        let mark: [u8; 2] = data
            .get(0..2)
            .ok_or(Error::Truncated { at: 0, len: 8 })?
            .try_into()
            .map_err(|_| Error::Truncated { at: 0, len: 8 })?;
        let byte_order = match &mark {
            b"II" => ByteOrder::Little,
            b"MM" => ByteOrder::Big,
            _ => return Err(Error::NotTiff { mark }),
        };

        let version = read_u16(data, 2, byte_order)?;
        if version != TIFF_VERSION {
            return Err(Error::UnsupportedTiffVersion { version });
        }
        let ifd0_offset = read_u32(data, 4, byte_order)?;

        let mut container = Container {
            data,
            byte_order,
            ifd0_offset,
            ifds: Vec::new(),
            issues: Vec::new(),
        };
        if ifd0_offset == 0 {
            return Ok(container);
        }

        // IFD0 is the only fatal IFD: it does not go through `follow`, and
        // there is no partial IFD0.
        let (entries, next) = container.read_ifd(ifd0_offset)?;
        container.ifds.push(Ifd {
            offset: ifd0_offset,
            depth: 0,
            parent: None,
            next,
            entries,
        });
        let mut walk = Walk {
            visited: vec![ifd0_offset],
            attempts: 1,
            halted: false,
        };
        container.walk_from(&mut walk, 0);
        Ok(container)
    }

    /// Like [`Container::parse`], but any [`ParseIssue`] becomes `Err`: the
    /// first issue's error is returned. This is the pre-DEC-001 behaviour.
    pub fn parse_strict(data: &'a [u8]) -> Result<Container<'a>, Error> {
        let container = Container::parse(data)?;
        // Take the first issue by value: the container is being consumed.
        match container.issues.into_iter().next() {
            Some(issue) => Err(issue.into_error()),
            None => Ok(Container {
                issues: Vec::new(),
                ..container
            }),
        }
    }

    /// The container's byte order.
    pub fn byte_order(&self) -> ByteOrder {
        self.byte_order
    }

    /// Offset of IFD0, as the header declared it.
    pub fn ifd0_offset(&self) -> u32 {
        self.ifd0_offset
    }

    /// Every IFD walked, in depth-first order. IFD0 is first.
    pub fn ifds(&self) -> &[Ifd] {
        &self.ifds
    }

    /// Every recoverable problem found by the walk, in walk order.
    pub fn issues(&self) -> &[ParseIssue] {
        &self.issues
    }

    /// Whether the walk found no issues at all.
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// IFD0: present whenever [`Container::parse`] succeeded with a non-zero
    /// IFD0 offset.
    pub fn ifd0(&self) -> Option<&Ifd> {
        self.ifds.first()
    }

    // ── the walk ────────────────────────────────────────────────────────────

    /// From the IFD just recorded at `index`: walk its `SubIFDs`, then continue
    /// along its chain. The chain is a loop (chains are long and their length
    /// is attacker-chosen); recursion happens only into `SubIFDs` and is
    /// bounded by [`MAX_IFD_DEPTH`], so native stack depth is bounded too.
    fn walk_from(&mut self, walk: &mut Walk, index: usize) {
        let mut cur = index;
        loop {
            if walk.halted {
                return;
            }
            let Some((depth, parent, next)) = self
                .ifds
                .get(cur)
                .map(|ifd| (ifd.depth, ifd.parent, ifd.next))
            else {
                return;
            };

            self.walk_sub_ifds(walk, cur, depth);
            if walk.halted || next == 0 {
                return;
            }
            match self.follow(walk, next, depth, parent, Link::Next, cur) {
                Some(new_index) => cur = new_index,
                None => return,
            }
        }
    }

    /// Walk the `SubIFDs` of the IFD at `holder`. A malformed pointer tag costs
    /// the sub-branch, never the holding IFD (DEC-001).
    fn walk_sub_ifds(&mut self, walk: &mut Walk, holder: usize, depth: u32) {
        // Materialise the offsets before recursing: the Vec ends the borrow of
        // `self.ifds`, which the recursive walk mutates.
        let subs = match self
            .ifds
            .get(holder)
            .and_then(|ifd| ifd.entry(tags::SUB_IFDS))
        {
            None => return,
            Some(entry) => match self.uints(entry) {
                Ok(subs) => subs,
                Err(error) => {
                    self.issues.push(ParseIssue::new(
                        error,
                        Link::Tag(tags::SUB_IFDS),
                        holder,
                        None,
                    ));
                    return;
                }
            },
        };
        for sub in subs {
            if walk.halted {
                return;
            }
            if sub == 0 {
                continue;
            }
            let link = Link::Tag(tags::SUB_IFDS);
            let child_depth = depth.saturating_add(1);
            if let Some(index) = self.follow(walk, sub, child_depth, Some(holder), link, holder) {
                self.walk_from(walk, index);
            }
        }
    }

    /// Try to read and record the non-zero IFD at `offset`. Returns its index,
    /// or `None` after recording an issue: the IFD is omitted entirely and
    /// the branch (or chain) stops.
    fn follow(
        &mut self,
        walk: &mut Walk,
        offset: u32,
        depth: u32,
        parent: Option<usize>,
        link: Link,
        from_ifd: usize,
    ) -> Option<usize> {
        walk.attempts = walk.attempts.saturating_add(1);

        // The budget is global, so exhausting it ends the whole walk.
        if walk.attempts > MAX_IFDS {
            self.issues.push(ParseIssue::new(
                Error::TooManyIfds { limit: MAX_IFDS },
                link,
                from_ifd,
                Some(offset),
            ));
            walk.halted = true;
            return None;
        }
        if depth > MAX_IFD_DEPTH {
            self.issues.push(ParseIssue::new(
                Error::IfdDepthExceeded {
                    limit: MAX_IFD_DEPTH,
                },
                link,
                from_ifd,
                Some(offset),
            ));
            return None;
        }
        // `visited` spans the whole walk, so a SubIFD pointing back at IFD0
        // is caught too.
        if walk.visited.contains(&offset) {
            self.issues.push(ParseIssue::new(
                Error::CyclicIfd { offset },
                link,
                from_ifd,
                Some(offset),
            ));
            return None;
        }
        walk.visited.push(offset);

        match self.read_ifd(offset) {
            Ok((entries, next)) => {
                let index = self.ifds.len();
                self.ifds.push(Ifd {
                    offset,
                    depth,
                    parent,
                    next,
                    entries,
                });
                Some(index)
            }
            Err(error) => {
                self.issues
                    .push(ParseIssue::new(error, link, from_ifd, Some(offset)));
                None
            }
        }
    }

    /// Read the entry table at `offset` and the chain pointer after it.
    fn read_ifd(&self, offset: u32) -> Result<(Vec<Entry>, u32), Error> {
        let at = offset_to_usize(offset)?;
        let count = read_u16(self.data, at, self.byte_order)?;

        let entries_at = at.checked_add(2).ok_or(Error::Truncated { at, len: 2 })?;
        let table_len = usize::from(count)
            .checked_mul(ENTRY_SIZE)
            .ok_or(Error::Truncated {
                at: entries_at,
                len: ENTRY_SIZE,
            })?;
        let entries_end = entries_at.checked_add(table_len).ok_or(Error::Truncated {
            at: entries_at,
            len: table_len,
        })?;
        let table = self
            .data
            .get(entries_at..entries_end)
            .ok_or(Error::Truncated {
                at: entries_at,
                len: table_len,
            })?;

        let mut entries = Vec::with_capacity(usize::from(count));
        // `as_chunks` rather than `chunks_exact`: it yields `&[u8; ENTRY_SIZE]`,
        // so the length is a type-level fact rather than a runtime one.
        let mut chunk_at = entries_at;
        for chunk in table.as_chunks::<ENTRY_SIZE>().0 {
            // The value field is the last four bytes of the 12-byte entry.
            // Both sums stay inside `entries_end`, which was checked above.
            let value_at = chunk_at.checked_add(8).ok_or(Error::Truncated {
                at: chunk_at,
                len: ENTRY_SIZE,
            })?;
            entries.push(Entry::parse(chunk, self.byte_order, value_at)?);
            chunk_at = chunk_at.checked_add(ENTRY_SIZE).ok_or(Error::Truncated {
                at: chunk_at,
                len: ENTRY_SIZE,
            })?;
        }

        let next = read_u32(self.data, entries_end, self.byte_order)?;
        Ok((entries, next))
    }

    // ── payloads ────────────────────────────────────────────────────────────

    /// The raw payload bytes of an entry, as a slice of the **input** (not of
    /// this `Container`), so it stays valid after the `Container` is dropped
    /// (DEC-002).
    ///
    /// Inline when `count × sizeof(type) <= 4`, otherwise the bytes at the
    /// offset the entry carries, bounds-checked against the whole buffer. This
    /// is the one place the hostile product is computed, and it is computed in
    /// `u64` and `checked_mul`.
    pub fn payload(&self, entry: &Entry) -> Result<&'a [u8], Error> {
        let size = types::type_size(entry.field_type).ok_or(Error::UnexpectedFieldType {
            tag: entry.tag,
            field_type: entry.field_type,
        })?;
        let len_u64 = u64::from(entry.count)
            .checked_mul(size)
            .ok_or(Error::ValueOverflow { tag: entry.tag })?;
        let len = usize::try_from(len_u64).map_err(|_| Error::ValueOverflow { tag: entry.tag })?;

        if len <= 4 {
            // Fits in the entry itself (TIFF 6.0 §2, "Value/Offset"): the
            // bytes are the start of the entry's value field in the input.
            let end = entry
                .value_at
                .checked_add(len)
                .ok_or(Error::ValueOverflow { tag: entry.tag })?;
            self.data.get(entry.value_at..end).ok_or(Error::Truncated {
                at: entry.value_at,
                len,
            })
        } else {
            let at = offset_to_usize(self.byte_order.u32(entry.value))?;
            let end = at.checked_add(len).ok_or(Error::Truncated { at, len })?;
            self.data.get(at..end).ok_or(Error::Truncated { at, len })
        }
    }

    /// First value of `tag` in `ifd`, or `None` when the tag is absent.
    pub fn scalar(&self, ifd: &Ifd, tag: u16) -> Result<Option<u32>, Error> {
        match ifd.entry(tag) {
            None => Ok(None),
            Some(entry) => Ok(self.uints(entry)?.first().copied()),
        }
    }

    /// First value of `tag` in `ifd`, or [`Error::MissingTag`].
    pub fn required_scalar(&self, ifd: &Ifd, tag: u16) -> Result<u32, Error> {
        self.scalar(ifd, tag)?.ok_or(Error::MissingTag { tag })
    }

    /// All values of `tag` in `ifd`, or an empty Vec when absent.
    pub fn values(&self, ifd: &Ifd, tag: u16) -> Result<Vec<u32>, Error> {
        match ifd.entry(tag) {
            None => Ok(Vec::new()),
            Some(entry) => self.uints(entry),
        }
    }
}

#[cfg(test)]
#[allow(
    // Test code is not a parse/decode path exercised on untrusted input
    // (AGENTS.md §11, guidance/toolchain-brief.md). Same five lints, same
    // sanctioned exception as irradiance's `ifd.rs` test module. `just
    // forbid-unsafe` / the `-F unsafe_code` re-check runs `--lib`, where this
    // module does not exist.
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::types::{TYPE_LONG, TYPE_RATIONAL, TYPE_SHORT, TYPE_SRATIONAL};
    use super::*;

    /// `ExposureBiasValue`: an Exif `SRATIONAL` tag. Local to the tests because
    /// Exif-IFD tags arrive with their consumer in SPEC-004.
    const EXPOSURE_BIAS_VALUE: u16 = 0x9204;

    /// A minimal byte-level TIFF builder, deliberately independent of any
    /// shared test builder: these are the unit tests of the reader, and a
    /// shared builder would let one bug hide the other.
    struct Build {
        big: bool,
        out: Vec<u8>,
    }

    impl Build {
        fn new(big: bool, ifd0: u32) -> Build {
            let mut b = Build {
                big,
                out: Vec::new(),
            };
            b.out.extend_from_slice(if big { b"MM" } else { b"II" });
            let v = b.u16_bytes(42);
            b.out.extend_from_slice(&v);
            let o = b.u32_bytes(ifd0);
            b.out.extend_from_slice(&o);
            b
        }
        fn u16_bytes(&self, v: u16) -> [u8; 2] {
            if self.big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        }
        fn u32_bytes(&self, v: u32) -> [u8; 4] {
            if self.big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        }
        /// Write an IFD at `at`. Entries are `(tag, type, count, value word)`.
        fn ifd(mut self, at: u32, entries: &[(u16, u16, u32, u32)], next: u32) -> Build {
            let mut block = Vec::new();
            block.extend_from_slice(&self.u16_bytes(entries.len() as u16));
            for (tag, ty, count, value) in entries {
                block.extend_from_slice(&self.u16_bytes(*tag));
                block.extend_from_slice(&self.u16_bytes(*ty));
                block.extend_from_slice(&self.u32_bytes(*count));
                block.extend_from_slice(&self.u32_bytes(*value));
            }
            block.extend_from_slice(&self.u32_bytes(next));
            let at = at as usize;
            if self.out.len() < at + block.len() {
                self.out.resize(at + block.len(), 0);
            }
            self.out[at..at + block.len()].copy_from_slice(&block);
            self
        }
        fn done(self) -> Vec<u8> {
            self.out
        }
        /// A SHORT value packs at the START of the 4-byte field, so it sits in
        /// the high half on a big-endian file.
        fn short_word(&self, v: u16) -> u32 {
            if self.big {
                u32::from(v) << 16
            } else {
                u32::from(v)
            }
        }
        /// Write raw bytes at `at`, growing the buffer: the external payload
        /// block a `RATIONAL`/`SRATIONAL` array (8+ bytes, never inline) needs.
        fn data_at(mut self, at: u32, bytes: &[u8]) -> Build {
            let at = at as usize;
            if self.out.len() < at + bytes.len() {
                self.out.resize(at + bytes.len(), 0);
            }
            self.out[at..at + bytes.len()].copy_from_slice(bytes);
            self
        }
        /// `SRATIONAL` payload bytes for `(num, den)` pairs, byte-order-correct.
        fn srational_bytes(&self, pairs: &[(i32, i32)]) -> Vec<u8> {
            let mut v = Vec::new();
            for (n, d) in pairs {
                v.extend_from_slice(&if self.big {
                    n.to_be_bytes()
                } else {
                    n.to_le_bytes()
                });
                v.extend_from_slice(&if self.big {
                    d.to_be_bytes()
                } else {
                    d.to_le_bytes()
                });
            }
            v
        }
        /// `RATIONAL` (unsigned) payload bytes for `(num, den)` pairs.
        fn rational_bytes(&self, pairs: &[(u32, u32)]) -> Vec<u8> {
            let mut v = Vec::new();
            for (n, d) in pairs {
                v.extend_from_slice(&self.u32_bytes(*n));
                v.extend_from_slice(&self.u32_bytes(*d));
            }
            v
        }
    }

    /// A small baseline-TIFF entry set: 4x2, 14-bit, Orientation 6.
    fn baseline(b: &Build) -> Vec<(u16, u16, u32, u32)> {
        vec![
            (tags::IMAGE_WIDTH, TYPE_LONG, 1, 4),
            (tags::IMAGE_LENGTH, TYPE_LONG, 1, 2),
            (tags::BITS_PER_SAMPLE, TYPE_SHORT, 1, b.short_word(14)),
            (tags::ORIENTATION, TYPE_SHORT, 1, b.short_word(6)),
        ]
    }

    // ── header ──────────────────────────────────────────────────────────────

    #[test]
    fn header_reads_both_byte_orders() {
        for big in [false, true] {
            let b = Build::new(big, 8);
            let entries = baseline(&b);
            let data = b.ifd(8, &entries, 0).done();
            let c = Container::parse(&data).unwrap();
            assert_eq!(
                c.byte_order(),
                if big {
                    ByteOrder::Big
                } else {
                    ByteOrder::Little
                }
            );
            assert_eq!(c.ifd0_offset(), 8);
            let ifd0 = c.ifd0().unwrap();
            // The same values must come out of MM and II alike; a byte-order
            // bug shows up here as 1024 instead of 4.
            assert_eq!(c.required_scalar(ifd0, tags::IMAGE_WIDTH).unwrap(), 4);
            assert_eq!(c.required_scalar(ifd0, tags::IMAGE_LENGTH).unwrap(), 2);
            assert_eq!(c.required_scalar(ifd0, tags::BITS_PER_SAMPLE).unwrap(), 14);
            assert_eq!(c.scalar(ifd0, tags::ORIENTATION).unwrap(), Some(6));
            assert_eq!(c.scalar(ifd0, tags::COMPRESSION).unwrap(), None);
            assert!(matches!(
                c.required_scalar(ifd0, tags::COMPRESSION),
                Err(Error::MissingTag { tag: 259 })
            ));
            assert_eq!(
                c.values(ifd0, tags::COMPRESSION).unwrap(),
                Vec::<u32>::new()
            );
        }
    }

    #[test]
    fn empty_input_is_truncated_not_a_panic() {
        assert!(matches!(
            Container::parse(&[]),
            Err(Error::Truncated { .. })
        ));
    }

    #[test]
    fn a_non_tiff_mark_is_rejected() {
        assert!(matches!(
            Container::parse(b"XX\x2a\x00\x08\x00\x00\x00"),
            Err(Error::NotTiff { .. })
        ));
    }

    #[test]
    fn a_wrong_version_word_is_rejected() {
        assert!(matches!(
            Container::parse(b"II\x2b\x00\x08\x00\x00\x00"),
            Err(Error::UnsupportedTiffVersion { version: 43 })
        ));
    }

    #[test]
    fn an_ifd0_offset_of_zero_yields_no_ifds() {
        let data = Build::new(false, 0).done();
        let c = Container::parse(&data).unwrap();
        assert!(c.ifds().is_empty());
        assert!(c.ifd0().is_none());
        assert!(c.is_clean());
        assert!(c.issues().is_empty());
    }

    // ── bounds ──────────────────────────────────────────────────────────────

    #[test]
    fn an_ifd_offset_past_eof_is_truncated() {
        let data = Build::new(false, 0xFFFF_0000).done();
        assert!(matches!(
            Container::parse(&data),
            Err(Error::Truncated { .. })
        ));
    }

    #[test]
    fn an_entry_count_past_eof_is_truncated() {
        let mut data = Build::new(false, 8)
            .ifd(8, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 4)], 0)
            .done();
        data[8..10].copy_from_slice(&1000u16.to_le_bytes());
        assert!(matches!(
            Container::parse(&data),
            Err(Error::Truncated { .. })
        ));
    }

    #[test]
    fn ifd0_with_its_next_word_cut_off_is_still_fatal() {
        // The entry table is complete; the file ends before the 4-byte next
        // word. IFD0 is atomic, so this is `Err`, not a partial IFD0.
        let mut data = Build::new(false, 8)
            .ifd(8, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 4)], 0)
            .done();
        data.truncate(data.len() - 4);
        assert!(matches!(
            Container::parse(&data),
            Err(Error::Truncated { .. })
        ));
        assert!(matches!(
            Container::parse_strict(&data),
            Err(Error::Truncated { .. })
        ));
    }

    #[test]
    fn parse_strict_agrees_with_parse_on_fatal_input() {
        let past_eof = Build::new(false, 0xFFFF_0000).done();
        let mut bad_count = Build::new(false, 8)
            .ifd(8, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 4)], 0)
            .done();
        bad_count[8..10].copy_from_slice(&1000u16.to_le_bytes());
        let inputs: [&[u8]; 5] = [
            b"",
            b"XX\x2a\x00\x08\x00\x00\x00",
            b"II\x2b\x00\x08\x00\x00\x00",
            &past_eof,
            &bad_count,
        ];
        for data in inputs {
            let lenient = Container::parse(data).err();
            assert!(lenient.is_some(), "{data:?} should be fatal");
            assert_eq!(lenient, Container::parse_strict(data).err());
        }
    }

    #[test]
    fn a_payload_offset_past_eof_is_truncated() {
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::STRIP_OFFSETS, TYPE_LONG, 64, 0xFFFF_FF00)], 0)
            .done();
        // The walk itself succeeds: the entry table is intact.
        let c = Container::parse(&data).unwrap();
        let ifd = c.ifd0().unwrap();
        let entry = ifd.entry(tags::STRIP_OFFSETS).unwrap();
        assert!(matches!(c.payload(entry), Err(Error::Truncated { .. })));
    }

    #[test]
    fn a_count_that_overflows_the_size_product_is_typed() {
        // u32::MAX LONGs is 16 GiB; the product must be computed in a width
        // that cannot wrap, and rejected rather than truncated.
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::STRIP_OFFSETS, TYPE_LONG, u32::MAX, 64)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        let entry = c.ifd0().unwrap().entry(tags::STRIP_OFFSETS).unwrap();
        assert!(matches!(c.payload(entry), Err(Error::Truncated { .. })));
        assert!(matches!(c.uints(entry), Err(Error::TagTooLarge { .. })));
    }

    #[test]
    fn an_inline_value_is_read_from_the_entry_not_the_file() {
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 0xDEAD_BEEF)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        let entry = c.ifd0().unwrap().entry(tags::IMAGE_WIDTH).unwrap();
        assert_eq!(c.payload(entry).unwrap().len(), 4);
        assert_eq!(c.uints(entry).unwrap(), vec![0xDEAD_BEEFu32]);
    }

    #[test]
    fn an_unknown_field_type_costs_the_entry_not_the_file() {
        let data = Build::new(false, 8)
            .ifd(8, &[(60000, 250, 4, 400)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        let entry = c.ifd0().unwrap().entry(60000).unwrap();
        assert_eq!(entry.byte_len(), None);
        assert!(matches!(
            c.payload(entry),
            Err(Error::UnexpectedFieldType {
                field_type: 250,
                ..
            })
        ));
    }

    #[test]
    fn a_readable_type_this_module_does_not_widen_is_typed() {
        // SRATIONAL (10) has a known size, so `payload` works; `uints` is
        // what refuses, naming the type. Plain RATIONAL (5) is refused by
        // `uints` too: rationals are read verbatim by their own readers.
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::IMAGE_WIDTH, 10, 1, 200)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        let entry = c.ifd0().unwrap().entry(tags::IMAGE_WIDTH).unwrap();
        assert_eq!(entry.byte_len(), Some(8));
        assert!(matches!(
            c.uints(entry),
            Err(Error::UnexpectedFieldType { field_type: 10, .. })
        ));
    }

    // ── the guards ──────────────────────────────────────────────────────────

    /// `n` IFDs nested by `SubIFDs`, one per level (depths `0..n`), at offsets
    /// `8 + i * 40`. The deepest carries no `SubIFDs`, so only a limit can stop
    /// the walk. Distinct offsets, so the cycle guard is never what stops it.
    fn nested(n: u32) -> Vec<u8> {
        let mut b = Build::new(false, 8);
        for i in 0..n {
            let at = 8 + i * 40;
            b = if i + 1 < n {
                b.ifd(at, &[(tags::SUB_IFDS, TYPE_LONG, 1, at + 40)], 0)
            } else {
                b.ifd(at, &[], 0)
            };
        }
        b.done()
    }

    /// `n` empty IFDs chained by their next-words, at offsets `8 + i * 20`.
    fn chained(n: u32) -> Vec<u8> {
        let mut b = Build::new(false, 8);
        for i in 0..n {
            let at = 8 + i * 20;
            let next = if i + 1 < n { at + 20 } else { 0 };
            b = b.ifd(at, &[], next);
        }
        b.done()
    }

    #[test]
    fn a_self_referential_subifd_terminates() {
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::SUB_IFDS, TYPE_LONG, 1, 8)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 1);
        assert!(!c.is_clean());
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert_eq!(issue.error(), &Error::CyclicIfd { offset: 8 });
        assert_eq!(issue.link(), Link::Tag(tags::SUB_IFDS));
        assert_eq!(issue.from_ifd(), 0);
        assert_eq!(issue.offset(), Some(8));
        assert!(matches!(
            Container::parse_strict(&data),
            Err(Error::CyclicIfd { offset: 8 })
        ));
    }

    #[test]
    fn a_two_hop_subifd_cycle_terminates() {
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::SUB_IFDS, TYPE_LONG, 1, 200)], 0)
            .ifd(200, &[(tags::SUB_IFDS, TYPE_LONG, 1, 8)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 2);
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert_eq!(issue.error(), &Error::CyclicIfd { offset: 8 });
        assert_eq!(issue.link(), Link::Tag(tags::SUB_IFDS));
        assert_eq!(issue.from_ifd(), 1);
        assert_eq!(issue.offset(), Some(8));
        assert!(matches!(
            Container::parse_strict(&data),
            Err(Error::CyclicIfd { offset: 8 })
        ));
    }

    #[test]
    fn a_chain_that_points_at_itself_terminates() {
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 4)], 8)
            .done();
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 1);
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert_eq!(issue.error(), &Error::CyclicIfd { offset: 8 });
        assert_eq!(issue.link(), Link::Next);
        assert_eq!(issue.from_ifd(), 0);
        assert_eq!(issue.offset(), Some(8));
        assert!(matches!(
            Container::parse_strict(&data),
            Err(Error::CyclicIfd { offset: 8 })
        ));
    }

    #[test]
    fn nesting_past_the_depth_limit_terminates() {
        // Depths 0..=9: one more level than MAX_IFD_DEPTH allows.
        let data = nested(MAX_IFD_DEPTH + 2);
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 9);
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert_eq!(
            issue.error(),
            &Error::IfdDepthExceeded {
                limit: MAX_IFD_DEPTH
            }
        );
        assert_eq!(issue.link(), Link::Tag(tags::SUB_IFDS));
        assert_eq!(issue.from_ifd(), 8);
        assert_eq!(issue.offset(), Some(8 + 9 * 40));
        assert!(matches!(
            Container::parse_strict(&data),
            Err(Error::IfdDepthExceeded {
                limit: MAX_IFD_DEPTH
            })
        ));
    }

    #[test]
    fn nesting_at_exactly_the_depth_limit_is_clean() {
        // Depths 0..=8: the deepest allowed IFD is reached, with no issue.
        let data = nested(MAX_IFD_DEPTH + 1);
        let c = Container::parse(&data).unwrap();
        assert!(c.is_clean());
        assert_eq!(c.ifds().len(), 9);
        assert_eq!(c.ifds().last().unwrap().depth(), MAX_IFD_DEPTH);
    }

    #[test]
    fn a_long_chain_stops_at_the_ifd_limit() {
        let data = chained(MAX_IFDS as u32 + 1);
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), MAX_IFDS);
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert_eq!(issue.error(), &Error::TooManyIfds { limit: MAX_IFDS });
        assert_eq!(issue.link(), Link::Next);
        assert_eq!(issue.from_ifd(), MAX_IFDS - 1);
        assert_eq!(issue.offset(), Some(8 + MAX_IFDS as u32 * 20));
        assert!(matches!(
            Container::parse_strict(&data),
            Err(Error::TooManyIfds { limit: MAX_IFDS })
        ));
    }

    #[test]
    fn a_chain_of_exactly_max_ifds_is_clean() {
        let data = chained(MAX_IFDS as u32);
        let c = Container::parse(&data).unwrap();
        assert!(c.is_clean());
        assert_eq!(c.ifds().len(), MAX_IFDS);
    }

    // ── tolerant behaviour ──────────────────────────────────────────────────

    #[test]
    fn a_corrupt_subifd_leaves_ifd0_readable() {
        let b = Build::new(false, 8);
        let mut entries = baseline(&b);
        entries.push((tags::SUB_IFDS, TYPE_LONG, 1, 0xFFFF_FF00));
        let data = b.ifd(8, &entries, 0).done();
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 1);
        let ifd0 = c.ifd0().unwrap();
        assert_eq!(c.required_scalar(ifd0, tags::IMAGE_WIDTH).unwrap(), 4);
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert!(matches!(issue.error(), Error::Truncated { .. }));
        assert_eq!(issue.link(), Link::Tag(tags::SUB_IFDS));
        assert_eq!(issue.from_ifd(), 0);
        assert_eq!(issue.offset(), Some(0xFFFF_FF00));
        assert!(matches!(
            Container::parse_strict(&data),
            Err(Error::Truncated { .. })
        ));
    }

    #[test]
    fn one_bad_subifd_does_not_hide_its_siblings() {
        let b = Build::new(false, 8);
        let mut data = b
            .ifd(8, &[(tags::SUB_IFDS, TYPE_LONG, 3, 600)], 0)
            .ifd(200, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 5)], 0)
            .ifd(400, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 7)], 0)
            .done();
        data.resize(612, 0);
        data[600..604].copy_from_slice(&200u32.to_le_bytes());
        data[604..608].copy_from_slice(&0xFFFF_FF00u32.to_le_bytes());
        data[608..612].copy_from_slice(&400u32.to_le_bytes());
        let c = Container::parse(&data).unwrap();
        let offsets: Vec<u32> = c.ifds().iter().map(Ifd::offset).collect();
        assert_eq!(offsets, vec![8, 200, 400]);
        assert_eq!(c.ifds()[1].parent(), Some(0));
        assert_eq!(c.ifds()[2].parent(), Some(0));
        assert_eq!(c.issues().len(), 1);
        assert_eq!(c.issues()[0].offset(), Some(0xFFFF_FF00));
    }

    #[test]
    fn a_corrupt_ifd1_keeps_ifd0() {
        let mut data = Build::new(false, 8)
            .ifd(8, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 4)], 200)
            .ifd(200, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 5)], 0)
            .done();
        data[200..202].copy_from_slice(&1000u16.to_le_bytes());
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 1);
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert!(matches!(issue.error(), Error::Truncated { .. }));
        assert_eq!(issue.link(), Link::Next);
        assert_eq!(issue.from_ifd(), 0);
        assert_eq!(issue.offset(), Some(200));
    }

    #[test]
    fn a_bad_subifd_is_omitted_atomically() {
        // The sub-IFD's entry table is complete but the file ends before its
        // next-word: the IFD is omitted whole, never partially included.
        let mut data = Build::new(false, 8)
            .ifd(8, &[(tags::SUB_IFDS, TYPE_LONG, 1, 200)], 0)
            .ifd(200, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 5)], 0)
            .done();
        data.truncate(data.len() - 4);
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 1);
        assert_eq!(c.issues().len(), 1);
        assert!(matches!(c.issues()[0].error(), Error::Truncated { .. }));
    }

    #[test]
    fn a_malformed_subifds_tag_costs_the_branch_not_the_ifd() {
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::SUB_IFDS, TYPE_SRATIONAL, 1, 200)], 300)
            .ifd(300, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 5)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        // IFD0 is kept and its chain still followed.
        assert_eq!(c.ifds().len(), 2);
        assert_eq!(c.ifds()[1].offset(), 300);
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert_eq!(
            issue.error(),
            &Error::UnexpectedFieldType {
                tag: tags::SUB_IFDS,
                field_type: 10
            }
        );
        assert_eq!(issue.link(), Link::Tag(tags::SUB_IFDS));
        assert_eq!(issue.from_ifd(), 0);
        assert_eq!(issue.offset(), None);
    }

    #[test]
    fn a_subifds_payload_past_eof_is_a_pointer_issue() {
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::SUB_IFDS, TYPE_LONG, 4, 0xFFFF_FF00)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 1);
        assert_eq!(c.issues().len(), 1);
        let issue = &c.issues()[0];
        assert!(matches!(issue.error(), Error::Truncated { .. }));
        assert_eq!(issue.link(), Link::Tag(tags::SUB_IFDS));
        assert_eq!(issue.offset(), None);
    }

    #[test]
    fn issues_arrive_in_walk_order() {
        let mut data = Build::new(false, 8)
            .ifd(8, &[(tags::SUB_IFDS, TYPE_LONG, 1, 0xFFFF_FF00)], 200)
            .ifd(200, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 5)], 0)
            .done();
        data[200..202].copy_from_slice(&1000u16.to_le_bytes());
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.issues().len(), 2);
        assert_eq!(c.issues()[0].link(), Link::Tag(tags::SUB_IFDS));
        assert_eq!(c.issues()[1].link(), Link::Next);
    }

    #[test]
    fn a_hostile_subifds_array_cannot_flood_the_issue_list() {
        // 100,000 sub-offsets, every one pointing back at IFD0. IFD0 is attempt
        // 1; sub-offsets 1..=511 are attempts 2..=512 (cyclic, one issue each);
        // sub-offset 512 is attempt 513, over budget, and halts the walk.
        let n: u32 = 100_000;
        let mut data = Build::new(false, 8)
            .ifd(8, &[(tags::SUB_IFDS, TYPE_LONG, n, 600)], 0)
            .done();
        data.resize(600 + n as usize * 4, 0);
        for i in 0..n as usize {
            let at = 600 + i * 4;
            data[at..at + 4].copy_from_slice(&8u32.to_le_bytes());
        }
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 1);
        assert_eq!(c.issues().len(), 512);
        assert!(c.issues().len() <= 2 * MAX_IFDS + 1);
        let (last, cyclic) = c.issues().split_last().unwrap();
        assert_eq!(cyclic.len(), 511);
        for issue in cyclic {
            assert_eq!(issue.error(), &Error::CyclicIfd { offset: 8 });
        }
        assert_eq!(last.error(), &Error::TooManyIfds { limit: MAX_IFDS });
    }

    #[test]
    fn parse_issue_display_names_error_and_location() {
        let data = Build::new(false, 8)
            .ifd(8, &[(tags::SUB_IFDS, TYPE_LONG, 1, 8)], 0)
            .done();
        let c = Container::parse(&data).unwrap();
        let text = c.issues()[0].to_string();
        assert!(!text.is_empty());
        assert!(text.contains(&Error::CyclicIfd { offset: 8 }.to_string()));
        assert!(text.contains("IFD #0"));
        assert!(text.contains("to offset 8"));
        assert_eq!(Link::Tag(330).to_string(), "tag 0x014a");
        assert_eq!(Link::Next.to_string(), "next-IFD pointer");
        let no_offset = ParseIssue::new(Error::MissingTag { tag: 1 }, Link::Next, 3, None);
        assert!(!no_offset.to_string().contains("to offset"));
    }

    // ── rationals, verbatim ─────────────────────────────────────────────────

    const EXPOSURE_BIASES: &[(i32, i32)] = &[(-382, 10000), (11150, 10000), (-1, 3), (7, -2)];

    #[test]
    fn srational_reads_signed_and_fractional_verbatim() {
        // Both byte orders: a signed-read byte-order bug would surface as a
        // different (or wildly wrong) numerator out of one of them.
        for big in [false, true] {
            let b = Build::new(big, 8);
            let block = b.srational_bytes(EXPOSURE_BIASES);
            let data = b
                .ifd(8, &[(EXPOSURE_BIAS_VALUE, TYPE_SRATIONAL, 4, 300)], 0)
                .data_at(300, &block)
                .done();

            let c = Container::parse(&data).unwrap();
            let entry = c.ifd0().unwrap().entry(EXPOSURE_BIAS_VALUE).unwrap();
            let expected: Vec<SRational> = EXPOSURE_BIASES
                .iter()
                .map(|&(num, den)| SRational { num, den })
                .collect();
            assert_eq!(c.srationals(entry).unwrap(), expected);
            // `uints()` (integer-only) must refuse this very entry, and the
            // unsigned reader must too: the signedness is the type's.
            assert!(matches!(
                c.uints(entry),
                Err(Error::UnexpectedFieldType { field_type: 10, .. })
            ));
            assert!(matches!(
                c.urationals(entry),
                Err(Error::UnexpectedFieldType { field_type: 10, .. })
            ));
        }
    }

    #[test]
    fn rational_reads_verbatim_without_integer_division() {
        // 606276/1000000 is non-integral: exactly the kind of value an
        // integer-dividing reader would reject. `urationals` keeps it verbatim.
        let pairs: &[(u32, u32)] = &[(606276, 1000000), (1000000, 1000000), (461885, 1000000)];
        for big in [false, true] {
            let b = Build::new(big, 8);
            let block = b.rational_bytes(pairs);
            let data = b
                .ifd(8, &[(tags::X_RESOLUTION, TYPE_RATIONAL, 3, 300)], 0)
                .data_at(300, &block)
                .done();

            let c = Container::parse(&data).unwrap();
            let entry = c.ifd0().unwrap().entry(tags::X_RESOLUTION).unwrap();
            let expected: Vec<URational> = pairs
                .iter()
                .map(|&(num, den)| URational { num, den })
                .collect();
            assert_eq!(c.urationals(entry).unwrap(), expected);
            // The integer reader and the signed reader both refuse the entry.
            assert!(matches!(
                c.uints(entry),
                Err(Error::UnexpectedFieldType { field_type: 5, .. })
            ));
            assert!(matches!(
                c.srationals(entry),
                Err(Error::UnexpectedFieldType { field_type: 5, .. })
            ));
        }
    }

    #[test]
    fn rational_holds_zero_denominator_and_values_above_i32_max_verbatim() {
        // Unsigned: 0/0, 5/0 and values above i32::MAX are representable and
        // returned as-is; nothing here divides or narrows.
        let u: &[(u32, u32)] = &[(0, 0), (5, 0), (u32::MAX, 0x8000_0000), (0x8000_0001, 1)];
        // Signed: the extremes and a zero denominator.
        let s: &[(i32, i32)] = &[(i32::MIN, 0), (i32::MAX, i32::MIN), (0, 0), (-1, -1)];
        for big in [false, true] {
            let b = Build::new(big, 8);
            let ublock = b.rational_bytes(u);
            let sblock = b.srational_bytes(s);
            let data = b
                .ifd(
                    8,
                    &[
                        (tags::X_RESOLUTION, TYPE_RATIONAL, 4, 300),
                        (EXPOSURE_BIAS_VALUE, TYPE_SRATIONAL, 4, 400),
                    ],
                    0,
                )
                .data_at(300, &ublock)
                .data_at(400, &sblock)
                .done();
            let c = Container::parse(&data).unwrap();
            let ifd0 = c.ifd0().unwrap();

            let got_u = c
                .urationals(ifd0.entry(tags::X_RESOLUTION).unwrap())
                .unwrap();
            let want_u: Vec<URational> =
                u.iter().map(|&(num, den)| URational { num, den }).collect();
            assert_eq!(got_u, want_u);
            assert_eq!(got_u.get(2).map(|r| r.num), Some(u32::MAX));
            assert_eq!(got_u.get(2).map(|r| r.den), Some(0x8000_0000));

            let got_s = c
                .srationals(ifd0.entry(EXPOSURE_BIAS_VALUE).unwrap())
                .unwrap();
            let want_s: Vec<SRational> =
                s.iter().map(|&(num, den)| SRational { num, den }).collect();
            assert_eq!(got_s, want_s);
        }
    }

    // ── borrowing ───────────────────────────────────────────────────────────

    #[test]
    fn payload_borrows_from_the_input_not_the_container() {
        let b = Build::new(false, 8);
        let block = b.rational_bytes(&[(1, 2), (3, 4)]);
        let data = b
            .ifd(
                8,
                &[
                    (tags::X_RESOLUTION, TYPE_RATIONAL, 2, 300),
                    (tags::IMAGE_WIDTH, TYPE_LONG, 1, 0xCAFE_F00D),
                ],
                0,
            )
            .data_at(300, &block)
            .done();

        // Both slices are taken while a `Container` exists and used after it
        // is dropped. This only compiles if `payload` returns `&'a [u8]` tied
        // to `data`, not to `&self` (DEC-002).
        let (external, inline): (&[u8], &[u8]) = {
            let c = Container::parse(&data).unwrap();
            let ifd0 = c.ifd0().unwrap();
            let external = c.payload(ifd0.entry(tags::X_RESOLUTION).unwrap()).unwrap();
            let inline = c.payload(ifd0.entry(tags::IMAGE_WIDTH).unwrap()).unwrap();
            (external, inline)
        };

        assert_eq!(external, &data[300..316]);
        assert_eq!(inline, &0xCAFE_F00Du32.to_le_bytes());
        // No copy: each slice points into the input buffer itself.
        let range = data.as_ptr_range();
        assert!(range.contains(&external.as_ptr()));
        assert!(range.contains(&inline.as_ptr()));
        assert_eq!(external.as_ptr(), data[300..].as_ptr());
    }

    // ── walk order and error surface ────────────────────────────────────────

    #[test]
    fn subifd_offsets_are_walked_in_order_and_recorded_with_their_parent() {
        let b = Build::new(false, 8);
        let plane = baseline(&b);
        let mut data = b
            .ifd(
                8,
                &[
                    (tags::NEW_SUBFILE_TYPE, TYPE_LONG, 1, 1),
                    (tags::SUB_IFDS, TYPE_LONG, 2, 600),
                ],
                0,
            )
            .ifd(200, &plane, 0)
            .ifd(400, &[(tags::IMAGE_WIDTH, TYPE_LONG, 1, 7)], 0)
            .done();
        data.resize(608, 0);
        data[600..604].copy_from_slice(&200u32.to_le_bytes());
        data[604..608].copy_from_slice(&400u32.to_le_bytes());
        let c = Container::parse(&data).unwrap();
        assert_eq!(c.ifds().len(), 3);
        assert_eq!(c.ifds()[1].offset(), 200);
        assert_eq!(c.ifds()[1].depth(), 1);
        assert_eq!(c.ifds()[1].parent(), Some(0));
        assert_eq!(c.ifds()[2].offset(), 400);
        assert!(c.is_clean());
        assert!(c.issues().is_empty());
    }

    #[test]
    fn every_error_variant_this_module_returns_displays_something() {
        // Display is the only surface a consumer has for these; an empty one
        // is a silent error (constraint: errors carry their context).
        let errors = [
            Error::Truncated { at: 1, len: 2 },
            Error::NotTiff { mark: *b"XX" },
            Error::UnsupportedTiffVersion { version: 43 },
            Error::CyclicIfd { offset: 8 },
            Error::IfdDepthExceeded { limit: 8 },
            Error::TooManyIfds { limit: 512 },
            Error::UnexpectedFieldType {
                tag: 1,
                field_type: 250,
            },
            Error::ValueOverflow { tag: 1 },
            Error::TagTooLarge {
                tag: 1,
                count: 2,
                limit: 3,
            },
            Error::MissingTag { tag: 256 },
            Error::OffsetOutOfRange { offset: 9 },
        ];
        for e in &errors {
            assert!(!e.to_string().is_empty(), "{e:?} has an empty Display");
        }

        // The list above must be the whole enum: this match has no wildcard
        // (within the crate `#[non_exhaustive]` does not force one), so a new
        // variant fails to compile here until it is added, and the index check
        // proves every variant is in the list exactly once.
        fn index(e: &Error) -> usize {
            match e {
                Error::Truncated { .. } => 0,
                Error::NotTiff { .. } => 1,
                Error::UnsupportedTiffVersion { .. } => 2,
                Error::CyclicIfd { .. } => 3,
                Error::IfdDepthExceeded { .. } => 4,
                Error::TooManyIfds { .. } => 5,
                Error::UnexpectedFieldType { .. } => 6,
                Error::ValueOverflow { .. } => 7,
                Error::TagTooLarge { .. } => 8,
                Error::MissingTag { .. } => 9,
                Error::OffsetOutOfRange { .. } => 10,
            }
        }
        let indices: Vec<usize> = errors.iter().map(index).collect();
        assert_eq!(indices, (0..11).collect::<Vec<usize>>());
    }
}
