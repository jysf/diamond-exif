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
  id: HANDOFF-059
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-21
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-030

project:
  id: PROJ-001
  stage: STAGE-008
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
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: approved                # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 165255              # harness subagent_tokens
  estimated_usd: 1.09              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 40
  branch: claude/spec-030-verify
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "No token count is exposed to the subagent; the orchestrator replaces null with the harness figure. About 35 tool calls."
  synced_at: 2026-11-21
---

# HANDOFF-059: Verify SPEC-030 WebP RIFF walker and metadata regions (DEC-019)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-030` to a cold verifier. You did not write this. Review branch `claude/spec-030-webp` (build commits `009e96e` walker and `20141ef` metadata, cost commit after) against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-030-*.md`, the file without `-timeline`) and `decisions/DEC-019-webp-riff-walk-and-metadata-regions.md`. Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

The harness may start your worktree from main. Run `git checkout -b claude/spec-030-verify claude/spec-030-webp` (local branch; do not `reset --hard`) and confirm HEAD contains `20141ef`.

## BUDGET (hard)

About 80 tool calls or 200k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`.

## What to look hard at

1. **Panic-freedom and arithmetic** in `src/webp/mod.rs` and `src/webp/metadata.rs`: every size from the file (a 32-bit LE size on a possibly 32-bit `usize`), every slice, every loop's progress, pad handling at the end of input, `Chunk::range()` and `tiff_at` arithmetic.
2. **DEC-019 conformance, both directions.** Every rule has code and a test that fails if the rule is removed. Judge the builder's choices where the DEC is silent and say for each whether it is sound or needs a DEC-019 clarification: (4) walk halts suppress the end-of-walk `ChunkFlagMismatch` and `MetadataWithoutVp8x` issues; (5) `vp8x()` is the first VP8X only, `None` if it is short even when a later one is long, a VP8X of any length counts as present for `MetadataWithoutVp8x`, no flag comparison with a short VP8X; (3) up to three walk issues on one chunk; (2) issue offsets for the flag and no-VP8X issues; (6) `range()` includes the pad only when present; (9) the public API shape. Also check the `RiffSizeMismatch` arithmetic for inputs shorter than 8 bytes beyond the header, and whether `actual` (a `usize`) is sensible.
3. **Oracle evidence beyond the DEC (important lesson from SPEC-029: the DEC's measurements missed variants and cost a fix round).** ExifTool 13.55 is installed. Build tiny WebPs and look at `exiftool -a -G1 -s -n` for the cases the DEC did NOT measure and report what you find: `ICCP` with the flag unset; `ALPH` before `VP8 `; a lossy `VP8 ` simple file with EXIF appended; `EXIF` immediately after the 12-byte header; an `XMP ` chunk of zero length; chunk code `XMP\0` with zero length; a RIFF header whose form type is `WEBP` but whose declared size is below 4; an `EXIF` chunk whose size is odd and is the last chunk with no pad; two `VP8X` chunks; an `ANMF` that contains an `EXIF` sub-chunk followed by a top-level `EXIF`. For each, say whether our walker and `read` agree with ExifTool on which metadata is located, and what to change if not. Disagreements on located metadata are Medium; on warnings only, Low.
4. **The model's independence and strength**: `fuzz/shared/webp_riff_invariants.rs` derives chunks, vp8x, issues, blocks and regions from raw bytes with its own four-character-code table (no library parsing); it must be able to fail. Mutate the library on a scratch copy (rsync the tree elsewhere; do not mutate the branch) in at least eight ways (pad rule, declared-size limiting the walk, case-insensitive code match, `XMP\0` handling, flag-mismatch order, MissingPad condition, ExtraExif logic, cap off-by-one, vp8x canvas `+1`, halt suppression) and confirm tests or the model go red; list survivors and say whether each is equivalent. Say which mutants only the unit tests (not the model on the committed seeds) kill, as a seed-shape gap.
5. **Pins, seeds, scope**: 51 `webp_riff` seeds (35 walk, 16 `meta-`), older seeds (tiff, jpeg, png) byte-identical (run `cargo run --example fuzz_seeds` and check the tree is clean), `fuzz/Cargo.lock` not committed with a changed version, test-only arms in other modules change no behaviour, README claims honest (WebP walked and metadata located, still unsupported in `exif-audit`), no CHANGELOG entry, no new dependency, PNG and JPEG behaviour unchanged (the shared `TooManyChunks` wording is the only intended change).
6. Run `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`, and a 20 s fuzz run of `webp_riff` yourself (the recipe hardcodes 60 s; use its cargo command with `-max_total_time=20`).

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input. Fill the `handback:` front matter (`verdict`) and the `## Handback` section; set statuses to `completed`; commit only the handback.

## Out of Scope

Fixing anything, DEC edits, audit wiring, ExifTool parity tests (SPEC-031).

---

## Handback` section below.** An unfilled handback is an
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

- **Branch / PR:** local branch `claude/spec-030-verify` (from `claude/spec-030-webp` at `f3a9717`, which contains `009e96e` and `20141ef`); not pushed, no PR.
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** yes, with two test-strength gaps (L1, L2): the "first `VP8X`" rule and the cap constants' values have no test that fails if they change. Behaviour is correct in both.
- **For `verify`:** ✅ APPROVED (at `f3a9717`). No High or Medium findings. Six Low findings, best fixed in a small follow-up or at SPEC-031.

### Findings

**High:** none. **Medium:** none. On every probe ExifTool and `read` locate the same metadata.

**Low**

- **L1. Nothing tests the "first `VP8X`" rule.** `src/webp/mod.rs:347` (`if !self.seen_vp8x`). Mutant M15 makes it `if true`, so the last `VP8X` wins. M15 passes every unit test, the model on all 51 seeds, and every seed prefix and flip. The test at `tests/webp_walk.rs:347` says "the info is still the first VP8X", but its input has only one `VP8X`. Failing input: `RIFF` + `VP8X`(flags 0x00) + `VP8X`(flags 0x08) + `VP8L` + `EXIF`. Correct behaviour: `vp8x().flags() == 0` and one `ChunkFlagMismatch{EXIF, false}`. Under M15 the flags are 8 and there is no mismatch. Fix: add one unit test and one seed (`vp8x-two-differ`).
- **L2. The cap values are not pinned.** `src/webp/mod.rs:44,49`. With `MAX_WEBP_CHUNKS = 65_000` and `MAX_WEBP_ISSUES = 1000`, all of `webp_walk`, `webp_metadata` and `fuzz_layout` still pass, because every test uses the constant by name. `MAX_WEBP_EXIF_BLOCKS` is pinned indirectly by the literal 15 at `tests/webp_metadata.rs:483`. Fix: `assert_eq!` on the three values that DEC-019 fixes.
- **L3. A zero-length `XMP ` (or `XMP\0`) chunk gives an empty `Xmp` region; ExifTool shows nothing.** `src/webp/metadata.rs:260`. Input: probe 05, `VP8X(0x0c)` `VP8L` `XMP `(size 0) `EXIF`. ExifTool prints no XMP tag and no warning; we return `Xmp { offset: 48, data: [] }`. No data is lost and the walk agrees (the `EXIF` after it is read on both sides). SPEC-032 must not count an empty region as XMP present. Fix: add one DEC-019 line saying empty regions are reported as located and empty.
- **L4. `XMP\0` with zero length: we report `NonCanonicalChunkId`, ExifTool gives no warning.** `src/webp/mod.rs:331`. Probe 06. This is a warning-only difference. DEC-019 measured the warning only on a non-empty `XMP\0`.
- **L5. A short first `VP8X` followed by a full one: ExifTool decodes the second, we return `vp8x() = None`.** `src/webp/mod.rs:349`. Probe 12: `VP8X`(3 bytes, 0x08) `VP8X`(10 bytes, 0x08) `VP8L` `EXIF`. ExifTool prints `WebP_Flags 8` from the second chunk; we report `BadVp8x` and `MisplacedVp8x` and skip the flag check. The `EXIF` is located on both sides, so the only difference is header info. This is the builder's choice 5; keep it, but write it into DEC-019.
- **L6. Grammar in a Display message.** `src/error.rs:571` renders "a ICC chunk exists" and "a EXIF chunk exists". It should say "an". This is cosmetic, but it reaches users through the audit at SPEC-032.

### The builder's choices where DEC-019 is silent

- **(4) A walk halt suppresses the end-of-walk `ChunkFlagMismatch` and `MetadataWithoutVp8x`.** Sound. A truncated walk cannot know which chunks came after the damage, so a flag comparison would be guesswork. It mirrors DEC-018's halt rule. Needs one DEC-019 line. Killed by M12.
- **(5) `vp8x()` is the first `VP8X` only, `None` when it is short even if a later one is full; any-length `VP8X` counts as present for `MetadataWithoutVp8x`; no flag comparison with a short `VP8X`.** Sound and conservative. It differs from ExifTool only on header info (L5). Needs a DEC-019 clarification. The "first" part is untested (L1). "Present at any length" is killed by M16.
- **(3) Up to three walk issues on one chunk** (`BadVp8x`, `MisplacedVp8x`, `MissingPad` on a short, odd, last, misplaced `VP8X`; an `EXIF` can carry `MissingPad`, `ExtraExif` and `ImproperExifHeader`). Sound. DEC-019's "at most two issues per chunk" sits in the metadata paragraph and reads as an extraction bound. The total stays bounded by `MAX_WEBP_ISSUES + 1 + 2 * MAX_WEBP_CHUNKS`, and the model asserts that bound. DEC-019 should say "extraction issues".
- **(2) Issue offsets.** A flag mismatch is at the first `VP8X`'s offset; `MetadataWithoutVp8x` is at the first metadata chunk. Sound, documented in `issue.rs:28`, and the model asserts both.
- **(6) `range()` includes the pad byte only when it is present.** Sound. With `MissingPad` the range ends at the end of the input, so the range never runs past the data. Killed by M22.
- **(9) Public API shape.** Matches the spec's list exactly. `halted()` is `pub(crate)`, as in PNG. `WebpRegion` and `ChunkKind` are `#[non_exhaustive]`.
- **`RiffSizeMismatch` arithmetic.** `actual = len - 8` saturates and is at least 4 for any input that passes the header check. A 12-to-19-byte input gives exact values; probe 07 (declared size 2) gives `declared 2, actual 74`. A `usize` is sensible for `actual`: on 64-bit an input over 4 GiB can never match a `u32` and is reported correctly. Killed by M18.

### Panic-freedom and arithmetic (`src/webp/mod.rs`, `src/webp/metadata.rs`)

Every slice uses `get`. Each size is widened with `usize::try_from(u32)`, and `size + 8` is a saturating add compared with `remaining` before any offset is formed. On 32-bit, a saturated `total` is still greater than `remaining`, because a slice length never exceeds `isize::MAX`. Every iteration either advances by `end - p >= 8` or ends the walk. A missing pad at the end of the input sets `end = data_end = len`. `tiff_at` is a saturating `offset + 8 (+6)`, always inside the input. `Vp8x::from_data` is checked-get only. I found no reachable panic. The 20 s fuzz run executed 737,535 runs clean.

### Oracle evidence beyond DEC-019 (ExifTool 13.55, `-a -G1 -s -n`; builder: scratch `gen.py`)

| Probe | ExifTool | Our walk and `read` | Agree on located metadata? |
|---|---|---|---|
| `ICCP`, flag unset | ICC processed ("Bad ICC_Profile table" warning shows it was read) | `Icc` region; `ChunkFlagMismatch{ICC,false}` | yes |
| `ICCP`, no `VP8X` | processed | `Icc` region; `MetadataWithoutVp8x` | yes |
| `ALPH` before `VP8 ` (+`EXIF`) | ALPH and VP8 decoded, Orientation 6 | EXIF block, clean | yes |
| simple lossy `VP8 ` + `EXIF` | Orientation 6, no warning | EXIF block; `MetadataWithoutVp8x` | yes (the issue is ours alone, by design) |
| `EXIF` right after the header | Orientation 6 | EXIF block at 12; `MetadataWithoutVp8x` | yes |
| `XMP ` size 0, then `EXIF` | no XMP, Orientation 6, no warning | empty `Xmp` region; EXIF block | yes on EXIF; empty region (L3) |
| `XMP\0` size 0, then `EXIF` | no XMP, Orientation 6, **no** "Incorrect XMP tag ID" warning | empty `Xmp`; `NonCanonicalChunkId` | yes; warning differs (L4) |
| declared size 2 | Orientation 6 | EXIF block; `RiffSizeMismatch{2,74}` | yes |
| odd `EXIF`, last, no pad | Orientation 6; warning "No padding on odd-sized EXIF chunk" | EXIF block (27 bytes); `MissingPad` | yes (warning matches too) |
| odd `XMP `, last, no pad | XMP read; same warning for XMP | `Xmp` region; `MissingPad` | yes |
| odd `EXIF` mid-file, no pad, then `XMP ` | EXIF read, XMP not read | EXIF block; `Truncated` at 84; no XMP | yes |
| two `VP8X` (0x00, 0x08) | both decoded; Orientation 6 | `vp8x()` from the first; `MisplacedVp8x`, `ChunkFlagMismatch{EXIF,false}` | yes |
| short `VP8X` then full `VP8X` | flags from the second; Orientation 6 | `vp8x() = None`; `BadVp8x`, `MisplacedVp8x` | yes on EXIF; header info differs (L5) |
| `ANMF` holding an `EXIF` (Orientation 3), then top-level `EXIF` (Orientation 6) | Orientation **6** only | one EXIF block at 120 (the top-level one) | yes |

### Model independence and mutation results (scratch rsync copy; branch untouched)

The model has its own code table, its own `le32`/`le24` and its own 64-bit arithmetic; its only library imports are the three constants. Each mutant was run against four suites: the metadata seeds alone, all 51 seeds (a scratch test), seeds with their prefixes and flips (`webp_harness_holds…`), and the unit tests (`webp_walk` and `webp_metadata`).

| Mutant | Meta seeds | All seeds | Prefix and flip | Unit |
|---|---|---|---|---|
| M01 pad ignored | killed | killed | killed | killed |
| M02 declared size limits the walk | killed | killed | killed | killed |
| M03 case-insensitive codes | survived | killed | killed | killed |
| M04 `XMP\0` read as `Other` | killed | killed | killed | killed |
| M05 flag order EXIF, ICC | killed | killed | killed | killed |
| M06 `MissingPad` never | survived | killed | killed | killed |
| M07 `MissingPad` on every odd chunk | killed | killed | killed | killed |
| M08 `ExtraExif` from the third | killed | killed | killed | killed |
| M09 chunk cap `>` | survived | survived | survived | killed |
| M10 issue cap `>` | survived | killed | killed | killed |
| M11 canvas `+1` dropped | killed | killed | killed | killed |
| M12 `finish` ignores halt | killed | killed | killed | killed |
| M13 extraction ignores halt | killed | killed | killed | killed |
| M14 EXIF block cap `<=` | killed | killed | killed | killed |
| **M15 last `VP8X` wins** | **survived** | **survived** | **survived** | **survived** |
| M16 `MetadataWithoutVp8x` despite a short `VP8X` | survived | killed | killed | killed |
| M17 `BadVp8x` threshold 9 | survived | survived | survived | killed |
| M18 `actual = len - 12` | killed | killed | killed | killed |
| M19 `Exif\0\0` not stripped | killed | killed | killed | killed |
| M20 `tiff_at` misses the prefix | killed | killed | killed | killed |
| M21 `MisplacedVp8x` never | survived | killed | killed | killed |
| M22 `range()` omits the pad | killed | killed | killed | killed |
| constants 65_000 and 1000 | n/a | pass | pass | pass (L2) |

- **The one survivor, M15, is not equivalent.** Probe 09 tells the two behaviours apart (L1).
- **Seed-shape gaps** (the model on the committed seeds misses them; only the unit tests kill them):
  - M09 needs 65,537 chunks, so no seed can carry it. Acceptable.
  - M17 has no 9-byte `VP8X` seed, although `vp8x-three-bytes` exists. Suggest a `vp8x-nine-bytes` seed.
- **Mutants the metadata seeds alone miss** (M03, M06, M10, M16, M21) are caught by the walk seeds.

### Pins, seeds, scope

- **Seeds:** 51 `webp_riff` seeds, of which 35 are walk seeds and 16 are `meta-` seeds. `cargo run --example fuzz_seeds` leaves the tree clean, so the tiff, jpeg and png seeds are byte-identical.
- **`fuzz/Cargo.lock`:** unchanged on the branch. My fuzz run rewrote it locally (`exif-core` 0.0.0 to 0.1.0-alpha.1, a stale lock that predates this branch); I restored it.
- **Dependencies and CHANGELOG:** no new dependency (`Cargo.toml` and `Cargo.lock` unchanged), no CHANGELOG entry.
- **Other modules:** the arms added in `src/jpeg/mod.rs`, `src/jpeg/metadata.rs` and `src/tiff/mod.rs` sit inside `mod tests` only.
- **Shared wording:** `TooManyChunks` and `TooManyIssues` now read "walk reached the limit…", and no test pins the old text. This is the only PNG- or JPEG-visible change.
- **README:** honest. It says nine targets, WebP walked and located, and `exif-audit` still unsupported for WebP.
- **Wiring:** CI matrix and `app.just` updated.

### Commands run

- `just lint`, `just test` (1034 passed, 0 failed), `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`: all exit 0.
- `cargo +nightly fuzz run webp_riff fuzz/corpus/webp_riff fuzz/seeds/webp_riff -- -max_total_time=20`: 737,535 runs, no crash.
- `cargo run --example fuzz_seeds` followed by `git status`: clean, apart from the fuzz-build lock rewrite, which I restored.
- `exiftool -a -G1 -s -n` on the 14 probe files, plus a scratch `probe_webp` example on the same files.
- 22 mutants and one constants mutant, run with a scratch `mutate.py` on an rsync copy.

### Cost self-report

- **Tokens (total):** null. The subagent has no token count; the orchestrator replaces it with the harness figure.
- **Estimated USD:** null
- **Duration (minutes):** about 40
- **Source of the number:** harness report (to be filled by the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none. Recommended clarifications to DEC-019:
  - a halt suppresses the end-of-walk issues;
  - `vp8x()` is the first `VP8X` only, a short one counts as present, and no flag comparison is made against it;
  - "at most two issues per chunk" means extraction issues;
  - zero-length `XMP ` and `ICCP` chunks are located as empty regions.
- **Deviations from spec:** none.
- **Follow-up work identified:**
  - L1 and L2 tests, plus seeds `vp8x-two-differ` and `vp8x-nine-bytes` (small; can ride with SPEC-031);
  - fix the L6 grammar;
  - SPEC-031 parity: add probes 05, 06, 09 and 12;
  - SPEC-032: treat an empty `Xmp` region as no XMP.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Little. "At most two issues per chunk" reads two ways: extraction issues only, or every issue on a chunk.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A rule that constants a DEC fixes by value are pinned by value in a test, not only used by name.

3. **If you did this task again, what would you do differently?**
   — Add an all-seeds-only scratch test from the start. The existing seed test covers only the `meta-` seeds, so my first mutation pass mislabelled some kills.
