//! The edit plan (DEC-013 part 1): whole-range removals and same-length
//! patches, nothing else.
//!
//! The output of [`EditPlan::apply`] is the input with the removal ranges
//! deleted and the patches applied. There is no insert, no grow and no move,
//! so the output is never longer than the input and its length is the input
//! length less the merged removal length. A plan is valid only if every range
//! lies inside the input and no patch touches a removal or another patch.
//!
//! Every offset and length here is untrusted (they come from parsed files):
//! the arithmetic is checked and nothing is indexed. Application allocates
//! once, for the output, which is never larger than the input.

use core::fmt;
use core::ops::Range;

/// One edit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Edit {
    /// Delete these input bytes.
    Remove(Range<usize>),
    /// Overwrite `bytes.len()` input bytes at `offset` with `bytes` (same
    /// length, in place).
    Patch {
        /// Input offset of the first byte overwritten.
        offset: usize,
        /// The replacement bytes.
        bytes: Vec<u8>,
    },
}

/// Why a plan cannot be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlanError {
    /// A removal whose start is after its end.
    InvertedRange {
        /// The range start.
        start: usize,
        /// The range end.
        end: usize,
    },
    /// A removal or a patch that is not inside the input (or whose end
    /// overflows `usize`).
    OutsideInput {
        /// The first offset of the range.
        start: usize,
        /// The input length it was checked against.
        len: usize,
    },
    /// A patch that overlaps a removal.
    PatchInsideRemoval {
        /// The patch offset.
        offset: usize,
    },
    /// Two patches that overlap.
    PatchesOverlap {
        /// The offset of the later of the two.
        offset: usize,
    },
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::InvertedRange { start, end } => {
                write!(f, "removal range {start}..{end} ends before it starts")
            }
            PlanError::OutsideInput { start, len } => {
                write!(
                    f,
                    "edit at offset {start} is not inside the {len}-byte input"
                )
            }
            PlanError::PatchInsideRemoval { offset } => {
                write!(f, "patch at offset {offset} overlaps a removal")
            }
            PlanError::PatchesOverlap { offset } => {
                write!(f, "patch at offset {offset} overlaps another patch")
            }
        }
    }
}

impl std::error::Error for PlanError {}

/// A list of [`Edit`]s. Order does not matter: the plan is normalised (removals
/// merged, patches sorted) when it is validated or applied.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct EditPlan {
    edits: Vec<Edit>,
}

/// A validated plan: the merged removals and the sorted patches.
type Checked<'a> = (Vec<Range<usize>>, Vec<(usize, &'a [u8])>);

/// Merge overlapping and adjacent ranges into sorted, disjoint, non-empty
/// ranges. Empty ranges are dropped. Ranges with `start > end` are dropped too:
/// callers that need to reject them check first ([`EditPlan::merged_removals`]).
pub(crate) fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.retain(|r| r.start < r.end);
    ranges.sort_unstable_by_key(|r| (r.start, r.end));
    let mut out: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for r in ranges {
        match out.last_mut() {
            Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
            _ => out.push(r),
        }
    }
    out
}

/// How many bytes of the sorted, disjoint `merged` ranges lie inside
/// `window` (`removed(a, b)` of DEC-013 part 5).
pub(crate) fn removed_in(merged: &[Range<usize>], window: Range<usize>) -> usize {
    let mut total = 0usize;
    for r in merged {
        let start = r.start.max(window.start);
        let end = r.end.min(window.end);
        if start < end {
            // `end - start` is at most the window length; the running total is
            // at most the sum of disjoint ranges inside `window`, so it cannot
            // exceed `usize::MAX`; saturating keeps the lint set satisfied.
            total = total.saturating_add(end.saturating_sub(start));
        }
    }
    total
}

impl EditPlan {
    /// An empty plan.
    pub fn new() -> EditPlan {
        EditPlan::default()
    }

    /// Add a removal.
    pub fn remove(&mut self, range: Range<usize>) {
        self.edits.push(Edit::Remove(range));
    }

    /// Add a same-length patch at input offset `offset`.
    pub fn patch(&mut self, offset: usize, bytes: &[u8]) {
        self.edits.push(Edit::Patch {
            offset,
            bytes: bytes.to_vec(),
        });
    }

    /// The edits as added.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// Whether the plan has no edit at all.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// The removals of the plan, checked against `input_len`, merged: sorted,
    /// disjoint, non-empty.
    pub fn merged_removals(&self, input_len: usize) -> Result<Vec<Range<usize>>, PlanError> {
        let mut ranges = Vec::new();
        for edit in &self.edits {
            if let Edit::Remove(r) = edit {
                if r.start > r.end {
                    return Err(PlanError::InvertedRange {
                        start: r.start,
                        end: r.end,
                    });
                }
                if r.end > input_len {
                    return Err(PlanError::OutsideInput {
                        start: r.start,
                        len: input_len,
                    });
                }
                ranges.push(r.clone());
            }
        }
        Ok(merge_ranges(ranges))
    }

    /// The patches as `(offset, bytes)`, sorted by offset, checked to lie inside
    /// `input_len` and not to overlap each other.
    fn sorted_patches(&self, input_len: usize) -> Result<Vec<(usize, &[u8])>, PlanError> {
        let mut patches: Vec<(usize, &[u8])> = Vec::new();
        for edit in &self.edits {
            if let Edit::Patch { offset, bytes } = edit {
                let inside = offset
                    .checked_add(bytes.len())
                    .is_some_and(|end| end <= input_len);
                if !inside {
                    return Err(PlanError::OutsideInput {
                        start: *offset,
                        len: input_len,
                    });
                }
                if !bytes.is_empty() {
                    patches.push((*offset, bytes.as_slice()));
                }
            }
        }
        patches.sort_unstable_by_key(|(offset, _)| *offset);
        let mut prev_end = 0usize;
        for (offset, bytes) in &patches {
            if *offset < prev_end {
                return Err(PlanError::PatchesOverlap { offset: *offset });
            }
            prev_end = offset.saturating_add(bytes.len());
        }
        Ok(patches)
    }

    /// Check the plan against an input of `input_len` bytes: every range inside
    /// the input, no patch inside a removal or inside another patch.
    pub fn validate(&self, input_len: usize) -> Result<(), PlanError> {
        self.checked(input_len).map(|_| ())
    }

    /// The merged removals and the sorted patches of a valid plan.
    fn checked(&self, input_len: usize) -> Result<Checked<'_>, PlanError> {
        let removals = self.merged_removals(input_len)?;
        let patches = self.sorted_patches(input_len)?;
        for (offset, bytes) in &patches {
            let end = offset.saturating_add(bytes.len());
            // The first removal that ends after the patch starts is the only
            // one that can touch it (the removals are sorted and disjoint).
            let i = removals.partition_point(|r| r.end <= *offset);
            if removals.get(i).is_some_and(|r| r.start < end) {
                return Err(PlanError::PatchInsideRemoval { offset: *offset });
            }
        }
        Ok((removals, patches))
    }

    /// How many bytes the plan removes from an input of `input_len` bytes.
    pub fn removed_len(&self, input_len: usize) -> Result<usize, PlanError> {
        let removals = self.merged_removals(input_len)?;
        Ok(removed_in(&removals, 0..input_len))
    }

    /// The output offset of input offset `offset`: the offset less the removed
    /// bytes before it. `None` when the plan is invalid, `offset` is past the
    /// input or `offset` lies inside a removal.
    pub fn output_offset(&self, input_len: usize, offset: usize) -> Option<usize> {
        if offset > input_len {
            return None;
        }
        let removals = self.merged_removals(input_len).ok()?;
        if removals.iter().any(|r| r.start <= offset && offset < r.end) {
            return None;
        }
        offset.checked_sub(removed_in(&removals, 0..offset))
    }

    /// Apply the plan to `input`: a new buffer with the removals deleted and the
    /// patches written. `input` is never modified. Deterministic and linear in
    /// the input plus the number of edits.
    pub fn apply(&self, input: &[u8]) -> Result<Vec<u8>, PlanError> {
        let len = input.len();
        let (removals, patches) = self.checked(len)?;
        let removed = removed_in(&removals, 0..len);
        let out_len = len
            .checked_sub(removed)
            .ok_or(PlanError::OutsideInput { start: 0, len })?;
        let mut out: Vec<u8> = Vec::with_capacity(out_len);
        let mut pending = patches.iter().peekable();
        let mut cursor = 0usize;
        // The kept gaps: before each removal, then the tail.
        let gaps = removals
            .iter()
            .map(|r| (r.start, Some(r.end)))
            .chain(core::iter::once((len, None)));
        for (gap_end, next_cursor) in gaps {
            let kept = input
                .get(cursor..gap_end)
                .ok_or(PlanError::OutsideInput { start: cursor, len })?;
            let base = out.len();
            out.extend_from_slice(kept);
            // Patches are sorted and avoid every removal, so the ones starting
            // before `gap_end` lie entirely in this gap.
            while let Some((offset, bytes)) = pending.next_if(|(offset, _)| *offset < gap_end) {
                let rel = offset
                    .checked_sub(cursor)
                    .and_then(|r| base.checked_add(r))
                    .ok_or(PlanError::OutsideInput {
                        start: *offset,
                        len,
                    })?;
                let end = rel
                    .checked_add(bytes.len())
                    .ok_or(PlanError::OutsideInput {
                        start: *offset,
                        len,
                    })?;
                out.get_mut(rel..end)
                    .ok_or(PlanError::OutsideInput {
                        start: *offset,
                        len,
                    })?
                    .copy_from_slice(bytes);
            }
            match next_cursor {
                Some(c) => cursor = c,
                None => break,
            }
        }
        Ok(out)
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

    fn input() -> Vec<u8> {
        (0u8..32).collect()
    }

    #[test]
    fn empty_plan_returns_the_input_unchanged() {
        let plan = EditPlan::new();
        assert!(plan.is_empty());
        assert_eq!(plan.apply(&input()).unwrap(), input());
        assert_eq!(plan.apply(&[]).unwrap(), Vec::<u8>::new());
        assert_eq!(plan.removed_len(32).unwrap(), 0);
    }

    #[test]
    fn remove_deletes_exactly_the_range() {
        let mut plan = EditPlan::new();
        plan.remove(4..10);
        let out = plan.apply(&input()).unwrap();
        let mut want = input();
        want.drain(4..10);
        assert_eq!(out, want);
        assert_eq!(out.len(), 26);
        // The first and last byte, and an empty range (a no-op).
        let mut plan = EditPlan::new();
        plan.remove(0..1);
        plan.remove(31..32);
        plan.remove(7..7);
        assert_eq!(plan.apply(&input()).unwrap(), (1u8..31).collect::<Vec<_>>());
    }

    #[test]
    fn overlapping_and_adjacent_removals_merge() {
        let mut plan = EditPlan::new();
        plan.remove(10..14);
        plan.remove(12..20); // overlaps the first
        plan.remove(20..22); // adjacent to the merged one
        plan.remove(3..5);
        plan.remove(4..5); // nested
        assert_eq!(plan.merged_removals(32).unwrap(), vec![3..5, 10..22]);
        let mut want = input();
        want.drain(10..22);
        want.drain(3..5);
        let out = plan.apply(&input()).unwrap();
        assert_eq!(out, want);
        // Bytes are not counted twice: 2 + 12 removed.
        assert_eq!(out.len(), 32 - 14);
        assert_eq!(plan.removed_len(32).unwrap(), 14);
    }

    #[test]
    fn patch_overwrites_at_the_same_length() {
        let mut plan = EditPlan::new();
        plan.patch(2, &[0xAA, 0xBB]);
        plan.remove(10..12);
        // A patch after the removal is written at its OUTPUT place.
        plan.patch(20, &[0xCC]);
        let out = plan.apply(&input()).unwrap();
        assert_eq!(out.len(), 30);
        assert_eq!(&out[2..4], &[0xAA, 0xBB]);
        assert_eq!(out[18], 0xCC);
        assert_eq!(plan.output_offset(32, 20), Some(18));
        // Everything else is the input.
        let mut want = input();
        want[2] = 0xAA;
        want[3] = 0xBB;
        want[20] = 0xCC;
        want.drain(10..12);
        assert_eq!(out, want);
        // A patch flush against a removal on either side is fine.
        let mut plan = EditPlan::new();
        plan.remove(8..10);
        plan.patch(6, &[1, 2]);
        plan.patch(10, &[3]);
        let out = plan.apply(&input()).unwrap();
        assert_eq!(&out[6..9], &[1, 2, 3]);
    }

    #[test]
    fn patch_inside_a_removal_is_rejected() {
        let before = input();
        for (rem, at, len) in [
            (4..10, 4, 1),
            (4..10, 9, 1),
            (4..10, 3, 2),
            (4..10, 9, 3),
            (4..10, 0, 20),
        ] {
            let mut plan = EditPlan::new();
            plan.remove(rem.clone());
            plan.patch(at, &vec![0u8; len]);
            assert_eq!(
                plan.apply(&before),
                Err(PlanError::PatchInsideRemoval { offset: at }),
                "{rem:?} {at} {len}"
            );
        }
        // Two patches that overlap each other.
        let mut plan = EditPlan::new();
        plan.patch(4, &[1, 2, 3]);
        plan.patch(6, &[9, 9]);
        assert_eq!(
            plan.validate(32),
            Err(PlanError::PatchesOverlap { offset: 6 })
        );
        assert_eq!(before, input());
    }

    #[test]
    fn range_outside_the_input_is_rejected() {
        let mut plan = EditPlan::new();
        plan.remove(30..33);
        assert!(matches!(
            plan.validate(32),
            Err(PlanError::OutsideInput { .. })
        ));
        let mut plan = EditPlan::new();
        plan.patch(31, &[1, 2]);
        assert!(matches!(
            plan.validate(32),
            Err(PlanError::OutsideInput { .. })
        ));
        let mut plan = EditPlan::new();
        plan.remove(Range { start: 9, end: 4 });
        assert_eq!(
            plan.validate(32),
            Err(PlanError::InvertedRange { start: 9, end: 4 })
        );
        // Exactly the end is inside.
        let mut plan = EditPlan::new();
        plan.remove(30..32);
        plan.patch(29, &[1]);
        assert!(plan.validate(32).is_ok());
        assert!(plan.apply(&[]).is_err());
    }

    #[test]
    fn edit_order_does_not_change_the_output() {
        let edits: Vec<Edit> = vec![
            Edit::Remove(2..5),
            Edit::Remove(4..9),
            Edit::Patch {
                offset: 12,
                bytes: vec![7, 7],
            },
            Edit::Remove(20..22),
            Edit::Patch {
                offset: 25,
                bytes: vec![8],
            },
        ];
        let mut reference = None;
        // Every rotation and the reverse.
        for r in 0..edits.len() {
            for rev in [false, true] {
                let mut e = edits.clone();
                e.rotate_left(r);
                if rev {
                    e.reverse();
                }
                let plan = EditPlan { edits: e };
                let out = plan.apply(&input()).unwrap();
                match &reference {
                    None => reference = Some(out),
                    Some(want) => assert_eq!(&out, want),
                }
            }
        }
    }

    /// A tiny deterministic generator (no dependencies).
    fn next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state >> 33
    }

    #[test]
    fn output_is_never_longer_than_the_input() {
        let mut state = 7u64;
        for _ in 0..500 {
            let len = (next(&mut state) % 64) as usize;
            let data: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let mut plan = EditPlan::new();
            let mut any_removed = false;
            for _ in 0..(next(&mut state) % 6) {
                let a = (next(&mut state) as usize) % (len + 1);
                let b = (next(&mut state) as usize) % (len + 1);
                let (a, b) = (a.min(b), a.max(b));
                any_removed |= a < b;
                plan.remove(a..b);
            }
            // Patches only where no removal is, found by trying: invalid
            // plans are rejected, never applied.
            for _ in 0..(next(&mut state) % 3) {
                let a = (next(&mut state) as usize) % (len + 1);
                let l = (next(&mut state) as usize) % 3;
                plan.patch(a, &vec![0xEE; l]);
            }
            match plan.apply(&data) {
                Ok(out) => {
                    assert!(out.len() <= data.len());
                    assert_eq!(out.len() == data.len(), !any_removed);
                    assert_eq!(out.len(), data.len() - plan.removed_len(len).unwrap());
                }
                Err(_) => assert!(plan.validate(len).is_err()),
            }
        }
    }

    #[test]
    fn u32_max_offsets_and_lengths_do_not_overflow() {
        let data = input();
        let big = u32::MAX as usize;
        for plan in [
            {
                let mut p = EditPlan::new();
                p.remove(big..big + 10);
                p
            },
            {
                let mut p = EditPlan::new();
                p.remove(0..usize::MAX);
                p
            },
            {
                let mut p = EditPlan::new();
                p.patch(usize::MAX, &[1, 2, 3]);
                p
            },
            {
                let mut p = EditPlan::new();
                p.patch(usize::MAX - 1, &[1, 2, 3]);
                p
            },
            {
                let mut p = EditPlan::new();
                p.remove(usize::MAX..usize::MAX);
                p
            },
        ] {
            assert!(plan.apply(&data).is_err());
            assert_eq!(plan.output_offset(data.len(), usize::MAX), None);
        }
        assert_eq!(removed_in(&[Range { start: 0, end: usize::MAX }], 0..usize::MAX), usize::MAX);
        assert_eq!(
            merge_ranges(vec![usize::MAX - 1..usize::MAX, 0..usize::MAX - 1]),
            vec![0..usize::MAX]
        );
    }
}
