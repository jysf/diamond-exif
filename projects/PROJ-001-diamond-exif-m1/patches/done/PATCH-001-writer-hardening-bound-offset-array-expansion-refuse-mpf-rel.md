---
# A PATCH is a lightweight fix to ALREADY-SHIPPED behavior (a bug or UX
# papercut) that adds NO new feature/command and doesn't warrant a full
# spec + stage. See AGENTS.md "Patch lane" and docs/decisions/DEC-003.
#
# Collapsed cycle: patch -> verify -> ship (design+build fused into one
# test-first pass; the INDEPENDENT verify is KEPT). It uses the same task.*
# schema as a spec, so `just validate`, `just cost-audit`, and `just status`
# treat a patch as first-class.

task:
  id: PATCH-001
  type: patch                      # epic | story | task | bug | chore | patch
  cycle: ship  # patch | verify | ship  (collapsed from a spec's 5)
  blocked: false
  priority: high
  complexity: M                    # S | M  (an L fix is probably a spec, not a patch)
  verify_verdict: approved
  verify_rounds: [approved, approved]   # 613c3b3 (three Lows), re-verify 5a84024 (L1, L2 fixed)

project:
  id: PROJ-001
  # No `stage:` — a patch attaches to the PROJECT, not a stage.
repo:
  id: diamond-exif

agents:
  implementer: claude-sonnet-5-5  # the patch pass (tier_map.build; DEC-005)
  verifier: claude-opus-5-5        # independent verify — KEPT (tier_map.verify; a separate session/agent)
  created_at: 2026-11-23

references:
  decisions: [DEC-012, DEC-013]

# Cost: patch + verify are the metered cycles — `just cost-audit` requires a
# real tokens_total on both for a shipped patch. ship is main-loop (null-with-note).
cost:
  sessions:
    - cycle: patch
      handoff: HANDOFF-086
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 196650
      estimated_usd: null
      duration_minutes: 28
      recorded_at: 2026-11-23
      notes: "transcribed from HANDOFF-086 by hand (handback-sync takes SPEC-NNN only); USD null: the Agent result gives no input/output split"
    - cycle: verify
      handoff: HANDOFF-087
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 197356
      estimated_usd: null
      duration_minutes: 20
      recorded_at: 2026-11-23
      notes: "transcribed from HANDOFF-087 by hand (handback-sync takes SPEC-NNN only); verdict approved at 613c3b3 with L1-L3; USD null: no input/output split"
    - cycle: patch
      handoff: HANDOFF-086
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 216525
      estimated_usd: null
      duration_minutes: 7
      recorded_at: 2026-11-23
      notes: "fix round L1+L2, same agent resumed (23 tool uses, duration_ms 436,488). Raw subagent_tokens as reported; a resumed agent's figure includes its carried-over context, so it is not this round's marginal spend. USD null"
    - cycle: verify
      handoff: HANDOFF-087
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 226695
      estimated_usd: null
      duration_minutes: 10
      recorded_at: 2026-11-23
      notes: "re-verify of L1+L2, same agent resumed (31 tool uses, duration_ms 601,047); approved at 5a84024. Raw subagent_tokens as reported; includes carried-over context. USD null"
    - cycle: ship
      agent: claude-opus-5-5
      interface: claude-code
      basis: null
      tokens_total: null
      estimated_usd: null
      duration_minutes: null
      recorded_at: 2026-11-23
      notes: "main-loop ship (port onto main after SPEC-033, gates, CHANGELOG, archive); un-metered, null by design"
  totals:
    tokens_total: 837226
    estimated_usd: 0.00
    session_count: 5
---

# PATCH-001: Writer hardening — bound offset-array expansion, refuse MPF relocation past the index cap, linear entry deletion

## Problem

An external code review (ChatGPT, against `fd94d4f`, 2026-11-23) reported four
writer defects. The architect checked three of them against `9133727` and
confirmed them in the code; this patch fixes those three. The reviewer's
measurements below are from their report and were **not** re-run here. (The
fourth finding, the JFIF APP0 thumbnail, needs a DEC-012 amendment and is a
separate spec.)

1. **Offset-array expansion is unbounded (memory DoS), P1.**
   `offset_targets` (`src/write/exif.rs:249`) runs `c.uints(e)` on every
   offset/length entry (StripOffsets, TileOffsets, FreeOffsets, …) and pushes
   one `Range` per value. The walk's entry budget limits *entries*, not
   *values*. Many entries can point at the same out-of-line array, so the
   expansion grows as entries × values from a tiny input. Reported: a 52 KB
   TIFF with 1,000 `StripOffsets` entries sharing 10,000 values used about
   647 MB. Both callers are exposed: the planner (`exif.rs:667`) and the
   self-check's `protected_ranges` (`exif.rs:1009`). Skipping duplicate entries
   is **not** enough: an attacker can give each entry a window shifted by one
   value into the same array.
2. **MPF index entries past `MAX_MPF_IMAGES` are left stale (silent
   corruption), P1.** `ex.mpf` (`src/jpeg/metadata.rs:545`) stops at 16
   entries and records `TooManyMpfImages`. `refuse_untrusted_walk`
   (`src/write/jpeg.rs:106`) does not refuse on that issue, so `mpf_patches`
   fixes up only the entries it saw. The self-check (`verify.rs` `mpf_fields`)
   has the same cap and cannot catch the stale ones. Reported: in a 17-image
   file, removing a 10-byte COM gave `Ok`, image 17 moved 596 → 586, and its
   index still said 596. A related problem: images past the cap are never
   recorded as `EmbeddedImage` regions, so `EmbeddedImage` / `Everything` can
   leave them in the file while returning `Ok`.
3. **Per-entry deletion is quadratic, P2.** The deletion-record loop
   (`exif.rs` ~565) and the scrub loop (`exif.rs` ~613) iterate over
   `(ifd, Some(entry))` pairs, and each pair rescans the whole IFD to find one
   entry. Reported: 2.6 s for a 786 KB TIFF with 65,535 targeted entries.

## Fix

All three refuse safely or are pure refactors. No new command or flag.

1. **Expansion budget.** In `offset_targets`, sum each offset/length entry's
   `count()` *before* calling `c.uints`. Once the running total passes a budget
   proportional to the blob, stop. The budget is `blob.len() / 2`, the most
   SHORT values that could exist without sharing bytes, and no real file comes
   near it. When the budget is passed, return the existing
   `Err(entry start)` path. The planner already maps that to
   `Uneditable { PayloadUnreadable }`. Use a new
   `UneditableReason::OffsetArraysTooLarge` instead only if verify asks for a
   distinct reason; the enum is `#[non_exhaustive]`, so either is
   semver-safe. Also merge ranges as they are added (or merge per entry), so
   memory stays at O(distinct ranges) and not O(values).
   `protected_ranges` uses the same function and gets the bound for free; its
   `Err` must stay a self-check failure, not a panic.
2. **MPF cap.** When `meta.jpeg_issues()` contains `TooManyMpfImages` (the
   extraction records it there, after the walker's issues; `metadata.rs:404`), *and* the
   strip would remove or shift any byte, refuse with
   `StripError::Unstrippable { reason: UnstrippableReason::TooManyMpfImages,
   offset }`. Add the new `#[non_exhaustive]` variant. This also covers
   `EmbeddedImage`/`Everything`: unread images can be neither patched nor
   removed, so refusing is the only correct answer. A strip that removes
   nothing (no targets) may still return the input unchanged.
3. **Linear deletion.** For `(g, Some(e))`, take `ifd.entries().get(e)`
   directly (`ok_or_else(bad)` on a miss); scan only for `(g, None)`, which is
   a whole removed IFD. Do the same in both loops. The output must be
   byte-identical: no change to ordering (`deleted.sort_by_key(at)` stays) or
   to which records are made.

## Failing Tests

Write these FIRST. Use the repo's existing builder helpers in
`src/write/exif_tests.rs`, `src/write/verify_tests.rs` and `tests/strip*.rs`;
don't add new fixture files unless a builder can't express the shape.

- **`src/write/exif_tests.rs`** — `offset_arrays_sharing_one_value_array_are_refused_not_expanded`:
  a TIFF with ~1,000 `StripOffsets` entries (each with its `StripByteCounts`)
  all pointing at one 10,000-value LONG array. `strip(.., Everything)` returns
  `Err(Uneditable { PayloadUnreadable | OffsetArraysTooLarge, .. })`, fast.
  Also a shifted-window variant (entry k starts k×4 bytes into the array),
  which beats deduplication.
- **`src/write/exif_tests.rs`** — `offset_arrays_within_budget_still_strip`:
  a normal multi-strip TIFF (e.g. 64 strips) still strips `Ok`, output
  unchanged from today (regression guard for the budget).
- **`tests/strip.rs`** — `seventeen_image_mpf_is_refused_when_bytes_move`: a
  primary plus 16 secondaries (17 index entries) and a COM. Removing the COM
  (`Comment` policy) gives `Unstrippable { TooManyMpfImages }`. The reviewer's
  case.
- **`tests/strip.rs`** — `seventeen_image_mpf_is_refused_under_everything`: the
  same file under `Everything` is refused, not `Ok` with image 17 left in.
- **`tests/strip.rs`** — `sixteen_image_mpf_still_strips`: exactly at the cap
  (16 entries) still strips `Ok` and passes the self-check (boundary guard).
- **`src/write/exif_tests.rs`** — `bulk_entry_deletion_is_linear`: 65,535
  targeted entries in one IFD strip in well under a second in a debug test
  build. Assert on output correctness plus a generous wall-clock bound (e.g.
  < 5 s debug), or better, on an operation count if a counter is cheap to
  expose under `cfg(test)`. Keep it from flaking on CI.
- **Fuzz seeds:** add the shared-array TIFF and the 17-image MPF to the
  matching fuzz seed directories (`fuzz/seeds/…` for the strip/TIFF targets).

## Verification (independent — KEPT)

Run in a SEPARATE session/agent from the patch pass. This is the one discipline
the framework retrospective proved catches real defects; it is non-negotiable
for a patch.

- Full gate suite (`just test`, `just lint`, `just lint-fuzz`,
  `just forbid-unsafe`, `just deny`), plus `just fuzz-smoke` on the targets
  that got new seeds.
- Confirm the failing tests now pass and no existing test regressed. In
  particular the strip oracle suites (`strip_oracle`, `strip_tiff_oracle`)
  must be byte-identical before and after (finding 3 is a pure refactor).
- Check the budget can't be passed by a *legitimate* file. Run every TIFF/JPEG
  fixture in `tests/oracle-fixtures` through `strip(Everything)` and compare
  results with `main`.
- Reproduce the reviewer's three cases on `main` (red) and on the branch
  (green).
- Output: ✅ APPROVED / ⚠ PUNCH LIST / ❌ REJECTED.

## Patch Completion

*Filled at the end of the patch pass, before verify.*

- **Branch / PR:** `fix/patch-001-writer-hardening` (local, no PR yet)
- **Fix summary:** `offset_targets` charges every offset and length entry's count against a shared budget of `blob.len() / 2` values before reading it and refuses past it (`PayloadUnreadable`); the JPEG plan refuses with `Unstrippable { TooManyMpfImages }` when the index passes the 16-entry cap and anything is removed; targeted entries are looked up directly, so deleting k entries is linear.
- **New decision emitted:** none
- **Reflection (1 line):** a new refusal also touches the fuzz model and the seed-count and crash-seed pins in `tests/fuzz_layout.rs`; list them in the patch's test section.
- **Defect-catch-stage:** `escaped` (found by an external review after ship)

**Fix round (L1, L2).** L1: the MPF-cap refusal now runs before `exif_targets`, counting segment, region and whole-Exif removals, so `Unstrippable { TooManyMpfImages }` precedes `Uneditable` as DEC-013 orders; an in-place Exif edit is still not a removal. The fuzz model in `fuzz/shared/rewrite_invariants.rs` predicts the same order, and the DEC-013 order test covers it. L2: `fuzz/Cargo.lock` restored to its a32bf05 content. tokens_total stays null (not measurable from inside the session).

## Ship

- Add a CHANGELOG entry under `[Unreleased] → Fixed`.
- Append cost sessions (patch + verify metered; ship null-with-note), then
  compute `cost.totals`.
- `just advance-cycle PATCH-001 ship`, then `just archive-patch PATCH-001`.
- **No stage bookkeeping** — a patch attaches to the project, not a stage.
