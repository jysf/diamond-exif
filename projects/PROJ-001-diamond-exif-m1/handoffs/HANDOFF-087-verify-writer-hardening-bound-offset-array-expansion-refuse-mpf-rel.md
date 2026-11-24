---
# Maps to ContextCore handoff.* semantic conventions.
#
# ONE handoff per delegated CYCLE. With build and verify running on different
# agents you get TWO handoffs per spec (HANDOFF-N build, HANDOFF-M verify) —
# `handoff.cycle` is what distinguishes them.
#
# The `handback:` block below is the RETURN path and it is not optional: it is
# how cost gets into the spec without the orchestrator hand-counting anything.
# `just handback-sync SPEC-NNN` reads it and appends the cost session for you.
# Rationale + the full contract: docs/decisions/DEC-013-delegated-cost-handback.md

handoff:
  id: HANDOFF-087
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-23
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: PATCH-001

project:
  id: PROJ-001
  stage: null                      # a patch attaches to the project, not a stage
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number,
# and say in `basis` how you got it:
#   Claude Code   → your session transcript, measured per docs/cost-tracking.md
#                   "Measuring a cycle": count each message.id once, take its
#                   largest output_tokens, include subagents/ (basis: transcript).
#                   Summing every line over-counts ~2×. `/cost` in the desktop
#                   Code tab reports context size, not spend.
#   API           → the `usage` objects, summed (basis: api-usage)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed                # completed | blocked | rejected
  agent: claude-opus-5-5                # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: approved                 # verify handoffs: approved | punch-list | rejected
  basis: agent-result                # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 197356             # orchestrator-transcribed: subagent_tokens of the Agent result
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 20             # Agent result duration_ms 1,172,442 (the agent's own 45 was an estimate)
  branch: claude/patch-001-verify
  pr: null
  completed_at: 2026-11-23           # YYYY-MM-DD
  notes: "Approved at 613c3b3 with three Low findings (L1 refusal order vs DEC-013 for capped MPF plus uneditable Exif; L2 unrelated fuzz/Cargo.lock version bump; L3 finding-1 fix committed under a test: message). tokens_total 197356 and duration 20 min transcribed by the orchestrator from the Agent result (subagent_tokens, duration_ms); estimated_usd null: no input/output split."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-087: PATCH-001 verify (writer hardening)

**FIRST STEP:** `git checkout --detach fix/patch-001-writer-hardening` (that branch is checked
out in another worktree, so detach at its tip, `247b366` or later). Work read-only on the
library. Commit only to this handoff file and PATCH-001, on a new branch
`claude/patch-001-verify` cut from that tip.

## Delegation Summary

`claude-opus-5-5` (architect) hands PATCH-001 to an independent `claude-opus-5-5` verifier for
the **verify** cycle. You did not write this code, so review it cold. AGENTS.md §8 makes this
independent verify the one non-negotiable step for a patch.

Read: `AGENTS.md` (§8 patch lane, §12, §15 "During verify"), `guidance/toolchain-brief.md`,
`guidance/constraints.yaml`, `projects/PROJ-001-diamond-exif-m1/patches/PATCH-001-*.md`
(Problem, Fix, Failing Tests, and the implementer's Patch Completion),
`HANDOFF-086-*.md` (the build brief and its handback), DEC-001, DEC-012, DEC-013. The change is
`git diff a32bf05 fix/patch-001-writer-hardening`.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` and confirm clippy
0.1.90. Fuzz needs nightly. macOS has no `timeout` (use `perl -e 'alarm 8; exec @ARGV' --`).
Restore `fuzz/Cargo.lock` after fuzz or lint-fuzz runs. Never commit `decisions/INDEX.md`.
Leave no background processes. Write scratch scripts in a scratch directory, not the repo.

## What to establish (evidence for each, not opinion)

1. **The three defects are fixed, shown on the real behavior.** Independently rebuild the
   reviewer's cases from PATCH-001's Problem section. Do not reuse the implementer's test
   builders as your only evidence. Run each on `a32bf05` (red) and on the branch tip
   (green): (a) about 1,000 `StripOffsets` entries sharing a 10,000-value array, plus the
   shifted-window variant (measure peak RSS with `/usr/bin/time -l`); (b) a 17-entry MPF
   JPEG with a COM, stripped under a comment-removing policy and under `Everything` (on
   red, show image 17's stale index offset); (c) about 65,535 targeted entries in one IFD
   (wall time, release build).
2. **Can the cap be bypassed?** Look for any input that still expands past the budget, or
   still relocates or keeps MPF images past the cap with `Ok`. Ideas: offsets split across
   IFDs; lengths without offsets; `protected_ranges` reached through a path that skips the
   budget; MPF with exactly 16 entries plus a second `MPF\0` segment; a strip whose only
   change is an in-place Exif patch on a 17-image file. The implementer chose to refuse only
   when there is a removal target; judge whether that is sound.
3. **False refusals.** Run every TIFF/JPEG/DNG fixture under `tests/oracle-fixtures` (and
   any real-shape fixture) through `strip` for every preset, on both `a32bf05` and the
   tip. List every case whose result changed. Any `Ok` → refusal on a legitimate file is a
   finding.
4. **The refactor is byte-identical.** For finding 4, compare the outputs and records from
   step 3 between `a32bf05` and the tip. They must be identical wherever both return `Ok`.
5. **The test-suite changes are justified.** `tests/fuzz_layout.rs` changed two crash-seed
   expectations (`CRASH_MPF_FIELDS`, `CRASH_MPF_TABLE`) and a seed count.
   `fuzz/shared/rewrite_invariants.rs` gained a predicted refusal. Confirm each change follows
   from the fix and doesn't hide a regression, and that the refusal they used to show is
   still covered somewhere.
6. **Panic-freedom and constraints.** Check new arithmetic for overflow and slicing for
   panics; check `forbid-unsafe-code`, `test-before-implementation` (one known slip: the
   finding-1 fix landed in `07259de`, the finding-2 test commit; say whether its red test
   `3a63bd6` still proves test-first), and `fuzz-target-per-surface`. Run
   `just decisions-audit --changed`.
7. **Gates on 1.90.0:** `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just test`,
   `just mutation-oracle`, `just forbid-unsafe`, `just deny`, `just validate`, the strip live
   tests with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, and 60 s smokes `rewrite_tiff`,
   `rewrite_jpeg`.

Output exactly ONE verdict: ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST (numbered, each with
severity H/M/L, evidence and a suggested fix) / ❌ REJECTED. Write it into the `## Handback`
section and `handback.verdict` (approved | punch-list | rejected).

## Budget

About 100 tool calls or 250k context. If you get near either, report what you established
and what you did not, and stop.

## Out of Scope

Fixing anything (report it instead); the JFIF thumbnail (SPEC-040); CHANGELOG; push or PR.

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback` section below.** An unfilled handback is an
incomplete cycle, and `just handback-sync` will tell the orchestrator so.

On success:
1. Fill the `handback:` front-matter — **including a real `tokens_total`**.
2. Fill the `## Handback` section (the reflection is part of it, not optional).
3. Set `handoff.status` → `completed` and `handback.status` → `completed`.
4. Open a PR (build) or return your verdict (verify).

If you cannot complete the task:
1. Fill the `## Handback` section with what was done and what blocked you.
2. Set `handoff.status` → `rejected`, `handback.status` → `blocked`.
3. **Still report your token usage** — blocked work costs money too.
4. Set the spec's `task.blocked: true` and add a question to
   `/guidance/questions.yaml`.

---

## Handback

*Filled in by the receiving agent. The orchestrator does not reconstruct any of
this — it transcribes it. The reflection questions are part of completion.*

### Execution notes

- **Branch / PR:** `claude/patch-001-verify` (local, cut from `613c3b3`; no push, no PR)
- **Completed at:** 2026-11-23
- **All acceptance criteria met?** yes. The three defects are fixed and every gate is green. Three Low findings are below; none of them blocks.
- **For `verify`:** ✅ **APPROVED at `613c3b3`**, with three Low findings (L1–L3) for the orchestrator to fix now or defer.

**Method.** I extracted `a32bf05` (red) and `613c3b3` (green) into scratch directories with `git archive`. For each one I built a scratch `zz_strip` example (input, policy, output; it times `strip` and prints the result) and a scratch dump test. Neither file was committed. I built the reproduction inputs with my own Python builders, not the implementer's `shared_array_blob` or `mpo_with_entries`. The toolchain was 1.90.0 (clippy 0.1.90). Timings are from release builds.

**1. The three defects are fixed (red on `a32bf05`, green on the tip).**
- (a) Shared offset array: 1,000 `StripOffsets` and 1,000 `StripByteCounts` entries, each a LONG×10,000 pointing at one array (a 104 KB file). On red, `Everything` and `AllPii` both return `Ok`, with a peak RSS of **968 MB** (`/usr/bin/time -l`) and about 0.25 s inside `strip`. On green both return `Uneditable { PayloadUnreadable, offset: 12046 }`, with a peak RSS of **2.7 MB** and 0.2 ms. The offset is the third lengths entry, where the running count passes `blob.len()/2` = 52,040; I checked that by hand. The shifted-window variant (entry k starts k values in, 112 KB) gives the same results: red `Ok` at **968 MB**, green refused at 2.7 MB. A small legitimate shape (2 entries × 64 values) returns `Ok` on both.
- (b) 17-entry MPF index with a 10-byte COM. The first layout puts the COM *between* the index and the images, as in the reviewer's case. On red, `FreeText` returns `Ok` (525 → 515 bytes, 16 patches). **Image 17 moved from 521 to 511, but its index entry still says 511** (relative to the header at 10), so it now points into image 16's `FF D9`. Entries 1–16 were patched correctly. Under `EmbeddedImage` red returns `Ok` and leaves image 17 in the file (135 bytes against 131 for the 16-entry file; one `FF D8 FF D9` remains). In my layout image 17 is the last bytes of the file, so `Everything` removed it as an `OtherSegment` trailer. The `Everything` leak therefore depends on layout, but the stale index under `FreeText` and `All` does not. On green, `FreeText`, `All`, `Everything` and `EmbeddedImage` all return `Unstrippable { TooManyMpfImages, offset: 2 }` (the APP2 segment), and `none` returns the input unchanged. The 16-entry boundary file returns `Ok` on both builds and all 15 secondary offsets check out. With 40 entries the results are the same. With the COM placed *before* the index, red was not actually wrong: the header moves with the images, so no relative offset changes. Green still refuses that file, which is a conservative refusal by design.
- (c) 65,535 inline Artist entries in one IFD (786 KB), under `AllPii` and `Everything`. Red: **2.22 s**. Green: **0.021 s**. Peak RSS is 51 MB on red and 43 MB on green. The outputs are **byte-identical** (`cmp`). For the reviewer's tag 0x9999, no public policy targets it (7 ms on both builds).

**2. Can the cap be bypassed? No path that expands past the budget or returns `Ok` over a stale index was found.**
- `offset_targets` is the only place that expands an offset array. Its two callers each create one budget per walk and share it across all IFDs, so splitting offsets across IFDs does not help. A lengths entry is read only when it pairs with a kept offsets entry, so lengths without offsets are never expanded. The charge is taken before `uints`, and `checked_sub` makes an overflow a refusal. `protected_ranges` is reached from `write/tiff.rs:59`, before any planning, even for `none`, and from `verify.rs:178`. Both paths have a budget, and the `Err` from either one is a typed refusal (`PayloadUnreadable` or `VerifyFailed { Walk }`), not a panic. With the budget, the ranges vector is bounded by about 8× the blob between merges (16 bytes per range, at most `blob/2` values).
- MPF with exactly 16 entries plus a second `MPF\0` APP2 holding 17: red and green both return `MpfNotIndexed { offset: 316 }` for any policy that removes bytes, and `Ok` when nothing is removed. The existing rule covers this case.
- A 17-image file whose only change is an in-place Exif edit (`Gps` / `AllPii` on an APP1 placed after the index): `Ok` on both builds, the length is unchanged, and the index is valid (checked). JPEG partial-mode Exif edits are same-length patches and never targets, so refusing only when there is a removal target is **sound**: if nothing is removed, nothing moves, and no image is due for removal. Under `EmbeddedImage` the index segment itself is always a target, so "the regions are all unlocatable, so targets is empty" cannot happen either.
- The refusal depends on the `TooManyMpfImages` issue surviving `record()`'s cap. `MAX_JPEG_METADATA_ISSUES` reserves `MAX_MPF_IMAGES` slots for the index: 15 entry issues plus this one. A walk halted by a cap stops recording, but such a walk is already refused by `refuse_untrusted_walk`. I found no way to drop the issue.

**3. False refusals.** The dump test ran `strip_shapes::all()`, `tiff_shapes::all()` and `hostile()`, `real_shapes::fixtures()` and `mpo_seventeen()`, and every seed under `fuzz/seeds/{rewrite_jpeg,rewrite_tiff,jpeg_segments,tiff_ifd,audit}` (raw, plus the decoded body for the rewrite seeds). Each input ran under 22 policies: the 5 presets, `none`, each of the 15 categories alone, and `iptc+freetext`. That is **62,788 common (input, policy) results; 40 changed**. All 40 are on **10 inputs**, and every one of them is an index with more than 16 entries:

  | input | policies |
  |---|---|
  | `real/mpo-seventeen`, `real_shapes::mpo_seventeen` | All, Everything, EmbeddedImage, OtherSegment |
  | the `mpo-seventeen` copies in `audit/`, `jpeg_segments/` and `rewrite_jpeg/{all,allpreset,everything,mask1040}-shape-mpo-seventeen` | All, Everything, EmbeddedImage, OtherSegment |
  | `rewrite_jpeg/crash-mpf-entry-table-overlap` | All, Everything, EmbeddedImage, OtherSegment |
  | `rewrite_jpeg/crash-mpf-fields-joined-stray` | All, Everything, EmbeddedImage, Xmp |

  How they changed: 36 went from `Ok` to `Unstrippable { TooManyMpfImages }`, 2 from `MpfNotIndexed{150}` to `TooManyMpfImages{150}`, and 2 from `VerifyFailed{MpfFields}` to `TooManyMpfImages{507}`. No TIFF or DNG result changed, so the budget refused **no** fixture. The only new refusals are on files with more than 16 images, which the patch refuses by design. I do not count these as false refusals.

**4. The refactor is byte-identical.** **25,606** results return `Ok` on both builds, and **0 differ**. Each comparison is an FNV hash of the full `Debug` of `Stripped`: bytes, removals, patches, entries and scrubs. Case (c)'s outputs are also identical with `cmp`. One subtle difference exists: `targeted_entries` now refuses `BlockNotClean` for a `Some(e)` that is out of range, where the old loop silently matched nothing. `gone` comes from the audit's own entry indices, so I found no input that reaches this. It is only stricter.

**5. The test-suite changes are justified.** `CRASH_MPF_FIELDS` and `CRASH_MPF_TABLE` both carry an index of more than 16 entries (the dump confirms it), so DEC-013's Unstrippable-first order requires the new refusal there. The behaviour they used to show is still covered elsewhere. The joined-stray `VerifyFailed` and its excuse are covered by `joined_stray_bytes_input` / `mask0040-joined-stray-bytes` in the same test (still `VerifyFailed`, and the L4 loop excuses `mpf-fields`). `Check::MpfFields` is covered in `verify_tests.rs` (3 sites). The table-overlap `MpfNotIndexed` is covered by `l15_ml_…` and `l15_mn_…` in `tests/strip.rs`; both still pass and both assert `MpfNotIndexed`. `REWRITE_SEEDS` 1038 → 1044 equals the six new `rewrite_jpeg` files, and `rewrite_jpeg/` holds 1,044 files. The model's new `capped` uses the same count as the library (`len/16 > 16`). It refuses only when there is a removal target, as the library does.

**6. Panic-freedom and constraints.** The new arithmetic is `try_from` / `checked_sub` / `saturating_*`. `all.get(e..=e)` returns `None` and does not panic, even for `e == usize::MAX`. `refuse_capped_mpf` does no arithmetic. `just lint-no-allow` (unwrap/expect/indexing/panic/arithmetic `-F`) and `forbid-unsafe` are clean. Test-first: I ran each step in its own scratch tree.
- `0410798` was red for finding 3 (the implementer recorded 7 s).
- `3a63bd6`: both offset-array tests **fail**, so it is red for finding 1. (My standalone repro: 968 MB.)
- `07259de`: those tests **pass**, while `seventeen_image_mpf_is_refused_{when_bytes_move,under_everything}` **fail**, so it is red for finding 2.
- `e730d8c` and the tip: green.

  So `3a63bd6` still proves test-first for finding 1. The slip is only that the fix landed under a `test:` subject (L3). `fuzz-target-per-surface`: no new input surface. `rewrite_tiff` and `rewrite_jpeg` got 4 and 6 seeds, and both pass smoke. `just decisions-audit --changed`: advisory list only (DEC-001/002/003/004/006/007/013/017–022). The relevant one is DEC-013, and it has a deviation (L1).

**7. Gates (1.90.0).** All of the following exit 0:
- `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just forbid-unsafe`.
- `just deny`: licenses ok; the usual unmatched-allowance warnings.
- `just validate`: 41 artifacts.
- `just test`: **1,452 passed, 0 failed, 18 ignored**.
- `just mutation-oracle`: last suite 41 passed / 1 ignored, no failures.
- `DIAMOND_EXIF_REQUIRE_ORACLE=1` with ExifTool 13.55: `strip_oracle` 31, `strip_tiff_oracle` 21, `strip_png_live` 16, `strip_webp_live` 23, `strip_webp_sweep_live` 13. All passed, none skipped.
- `fuzz-smoke rewrite_tiff`: 205,921 runs in 61 s. `fuzz-smoke rewrite_jpeg`: 66,704 runs in 61 s. No crash.

The worktree is clean afterwards (`fuzz/Cargo.lock` is unchanged by the runs), and no background processes remain.

**Low findings**

- **L1: refusal order against DEC-013 when a capped index meets an uneditable Exif block.** `refuse_capped_mpf` runs after `exif_targets(...)?`, so a partial-mode `Uneditable` is returned first. That breaks DEC-013 §"Refusal order is part of the contract: `Unstrippable`, then `Uneditable`". Evidence: a 17-entry index, a COM, and an Exif APP1 whose GPS pointer is out of range. Under `AllPii` (which removes the COM) or `FreeText`, green returns `Uneditable { BlockNotClean, 342 }` and not `Unstrippable { TooManyMpfImages }`. The result is still fail-closed; only the reported reason differs. The fuzz model copies the library's order (`EntryRefused` comes before the cap at `rewrite_invariants.rs:1553–1623`), so the fuzzer cannot catch it. `the_refusal_order_is_…` does not include the new reason. Suggested fix: decide the cap before Exif planning. Run `segment_targets` and `region_targets`, count Whole-mode Exif segments as targets, refuse if the index is capped and anything is targeted, and only then call `exif_targets`. Add the new reason to the order test, and change the model to match. Alternatively, amend DEC-013 to place this reason after `Uneditable`.
- **L2: unrelated `fuzz/Cargo.lock` change.** `9f2f2e5` bumps `exif-core` from `0.0.0` to `0.1.0-alpha.1` in `fuzz/Cargo.lock`. It does match `Cargo.toml` (already `0.1.0-alpha.1` at `a32bf05`), but HANDOFF-086 and this handoff both say to restore `fuzz/Cargo.lock` after fuzz runs. Fix: revert it in this patch, or keep it deliberately and say so in the ship notes or CHANGELOG.
- **L3: commit hygiene.** `07259de` ("test(patch-001): a 17-image MPF index must be refused…") also contains the finding-1 fix (`src/write/exif.rs` +42). Test-first still holds (see 6). Fix: squash-merge, or reword the subject to name both changes before merge.

### Cost self-report

- **Tokens (total):** 197,356 (transcribed by the orchestrator from the Agent result's `subagent_tokens`).
- **Estimated USD:** null
- **Duration (minutes):** 20 (Agent result `duration_ms` 1,172,442)
- **Source of the number:** harness report (`agent-result`)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:**
  - The `UnstrippableReason::TooManyMpfImages` refusal is ordered after `Uneditable` from Exif planning (L1), against DEC-013's contract order.
  - Out-of-scope `fuzz/Cargo.lock` version bump (L2).
  - PATCH-001's test list asks for 65,535 entries in `bulk_entry_deletion_is_linear`. The test uses 20,000 entries with a 4 s bound, which is reasonable for a debug build. My release repro covers 65,535 (2.22 s → 0.021 s).
- **Follow-up work identified:**
  - L1 if it is not fixed in this patch: a small follow-up patch that hoists the cap check and extends the refusal-order test and the fuzz model.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — "Run every fixture under `tests/oracle-fixtures`": those are ExifTool `.txt` records, not inputs. The inputs come from the builders in `tests/support/*_shapes.rs` and from the fuzz seeds. Naming the builders would have saved some exploration.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-013's refusal-order clause ("Refusal order is part of the contract"). Every new refusal reason has to be placed in it, and the fix brief did not say where `TooManyMpfImages` goes relative to `Uneditable`.

3. **If you did this task again, what would you do differently?**
   — Give every historical commit tree its own `CARGO_TARGET_DIR` from the start. A shared target dir across `git archive` trees of the same crate gave a stale (false-red) result for `07259de` until I rebuilt it in isolation.

### Re-verify (L1, L2)

**Verdict: ✅ APPROVED at `5a84024`.** L1 and L2 are fixed, and nothing else moved. This round was scoped to L1 and L2 only. L3 (the finding-1 fix sits in `07259de`, a `test:` commit) is commit hygiene: history was not rewritten, so it still holds unless the branch is squash-merged. Re-verify tokens: null. A subagent cannot read its own session transcript, so the orchestrator should transcribe the number from the Agent result.

Method: `claude/patch-001-verify` was fast-forwarded to `fix/patch-001-writer-hardening` (`4cb0ecc..5a84024`). I extracted `5a84024` with `git archive` into a fresh scratch tree with its own target dir and reused the round-1 `zz_strip` and `zz_dump` harnesses. Toolchain: 1.90.0, clippy 0.1.90.

1. **The L1 case from round 1 now gets the right refusal.** The input has a 17-entry index, a COM before it, and an Exif APP1 whose GPS pointer is out of range (`0xFFF0`).

   | policy | `a32bf05` | `5a84024` |
   |---|---|---|
   | `AllPii` | `Uneditable{BlockNotClean,342}` | `Unstrippable{TooManyMpfImages,12}` |
   | `FreeText` | `Uneditable{BlockNotClean,342}` | `Unstrippable{TooManyMpfImages,12}` |
   | `All` | `Ok` | `TooManyMpfImages` |
   | `Everything` | `Ok` | `TooManyMpfImages` |
   | `EmbeddedImage` | `Ok` | `TooManyMpfImages` |
   | `Gps` | `Uneditable` | `Uneditable{BlockNotClean,342}` |
   | `none` | `Ok` | `Ok`, unchanged |

   `Gps` removes nothing, so the cap does not apply and `Uneditable` is the correct answer there.

   The other probes still behave as expected:
   - A 17-image file whose only change is an in-place Exif edit: `Gps` and `AllPii` return `Ok`, the length is unchanged (605 bytes), and the index is still valid when re-read. `All` (whole-segment Exif removal) refuses.
   - The 16-entry boundary file returns `Ok` under `AllPii`, `FreeText` and `All`, with 16 patches.
   - 16 entries plus a second 17-entry `MPF\0` segment still returns `MpfNotIndexed{316}`.
   - The reviewer-shape 17-entry file is refused under every policy that removes something.

2. **Nothing else moved.** I re-ran the before/after harness: the same inputs and 22 policies, 63,228 results.
   - Re-running `a32bf05` reproduced its earlier dump byte for byte.
   - **`613c3b3` → `5a84024`: 0 results changed.** The corpus has no file with both a capped index and an uneditable Exif block, so the L1 cases appear only in the probes in item 1.
   - `a32bf05` → `5a84024`: still the same 40 changes on the same 10 inputs, all with more than 16 entries.
   - 25,606 results are `Ok` on both builds, and none of them differ.
   - On `removes_exif`: `exif_targets` in `Mode::Whole` pushes every `SegmentKind::Exif` segment as a target. So "Whole mode and any Exif segment" holds exactly when an Exif removal target will exist, and no file can be refused because of an Exif segment that is not removed. The dump confirms it: no `Ok` became a refusal between `613c3b3` and `5a84024`.
   - `region_targets` now runs before `exif_targets`. Targets are sorted afterwards, and output did not change.

3. **The fuzz model, the order test and the library agree.**
   - Both now decide the cap before the Exif planner. The library's condition is: any segment or region target, or Whole mode with an Exif segment. The model's condition is: segment targets, `all_exif_removed && typed has Exif`, `OtherSegment` with trailer pieces, or `EmbeddedImage`. These are the same, because with a capped index `EmbeddedImage` always targets the index segment.
   - `the_refusal_order_is_unstrippable_uneditable_mpf_not_indexed_overlap_verify_failed` now includes the capped-MPF-beats-unreadable-Exif case.
   - I wrapped the 7 probe files (including the L1 case) in 21 policy prefixes each: presets 0–4, `none`, and every single-category mask. All 147 inputs ran through the `rewrite_jpeg` target (`-runs=0`) with no invariant failure.
   - `just fuzz-smoke rewrite_jpeg`: 69,404 runs in 61 s, no crash.

4. **L2 is fixed.** `git diff a32bf05 fix/patch-001-writer-hardening -- fuzz/Cargo.lock` is empty. The fuzz runs dirtied it again (`0.1.0-alpha.1`), and I restored it with `git checkout -- fuzz/Cargo.lock`.

5. **Gates on 1.90.0** (`~/.cargo/bin` first in PATH):
   - `just lint`: exit 0.
   - `just lint-fuzz`: exit 0.
   - `just test`: **1,454 passed, 0 failed, 18 ignored**. That is the round-1 count plus the two new L1 tests.
   - `just validate`: ok, 41 artifacts.
   - With `DIAMOND_EXIF_REQUIRE_ORACLE=1` and ExifTool 13.55, none skipped: `strip_oracle` 31, `strip_tiff_oracle` 21, `strip_png_live` 16, `strip_webp_live` 23, `strip_webp_sweep_live` 13.

   The worktree is clean afterwards, and no background processes remain.
