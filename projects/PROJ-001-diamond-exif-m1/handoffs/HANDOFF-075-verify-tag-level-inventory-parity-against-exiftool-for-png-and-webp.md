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
  id: HANDOFF-075
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-22
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-036

project:
  id: PROJ-001
  stage: STAGE-010
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
  verdict: punch-list              # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-036-verify
  pr: null
  completed_at: 2026-11-22         # YYYY-MM-DD
  notes: "tokens_total left null by instruction: the orchestrator records the harness figure. Verify of claude/spec-036-part-b at 1df62be; punch list H1, H2, M1 to M3."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-075: <Task Title — same as the spec's title>

## Delegation Summary

One sentence: `<from_agent>` (acting as `<from_role>`) hands `SPEC-036`
to `<to_agent>` (acting as `<to_role>`) for the **`<cycle>`** cycle.

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-<slug>/brief.md`
- **Stage:** `./projects/PROJ-001-<slug>/stages/STAGE-010-<slug>.md`
- **Spec:** `./projects/PROJ-001-<slug>/specs/SPEC-036-<slug>.md`
- **Reviewer brief (OPTIONAL, verify handoffs):** `./guidance/agents/<lens>.md` —
  pick a lens if this review should look at one thing hard. Omit for a
  general review; see `./guidance/agents/README.md`.
- **Toolchain brief:** `./guidance/toolchain-brief.md` — this repo's test
  framework, lint quirks, runtime globals and gotchas. Read it; it exists so a
  cold agent doesn't rediscover them (DEC-004 rule 5).

### Decisions that apply

- `DEC-NNN` — <one-line summary of why this matters here>
- `DEC-MMM` — <one-line summary>

### Constraints that apply

Check `./guidance/constraints.yaml` for full text. These constraints
apply to the paths touched by this task:

- `constraint-id-1` — <one-line summary>
- `constraint-id-2` — <one-line summary>

### Prior related work

- `HANDOFF-YYY` — <one-line summary, if relevant>
- `PR #NNN` — <link, if relevant>

## Expected Deliverables

*(For a `verify` handoff, replace this block with the verify contract below.)*

- Code changes implementing SPEC-036's Acceptance Criteria.
- All failing tests in SPEC-036 now passing.
- Any new tests required to cover edge cases.
- A PR against `main` from branch `feat/spec-XXX-<slug>`.
- PR description referencing: this handoff ID, the spec ID, the stage
  ID, the project ID, all referenced `DEC-*`, and any new `DEC-*`
  created during implementation.

### If `cycle: verify` — the verify contract

You are **not** the agent that implemented this. Review it cold.

- Acceptance criteria met? Tests actually pass? Build reflection answered
  honestly (*"nothing was unclear"* is suspicious)?
- Decision drift — run `just decisions-audit --changed`.
- Constraint violations; non-trivial choices missing a `DEC-*`.
- For any criterion claiming **runtime behavior** (a component registers, a hook
  fires, a binary resolves on PATH, a config takes effect), confirm the
  *behavioral* surface was exercised — not just the shape validated. That is the
  defect class that escapes (AGENTS.md §12).
- Output exactly ONE of: ✅ APPROVED / ⚠ PUNCH LIST / ❌ REJECTED.

## Out of Scope

Explicit list of what this handoff does NOT include. If the receiving agent
thinks any of these need to happen, they should create a new spec in
the stage's backlog, not expand this handoff.

- ...

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

- **Branch / PR:** `claude/spec-036-verify` (from `claude/spec-036-part-b` at `1df62be`); no PR, by instruction.
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** On the corpus, yes: every box is exercised by a test that
  passes, and the 56 rows are exact. Off the corpus, the comparator model has blind spots
  (H2, M1, M2), and one pinned cause hides an audit bug (H1). See the punch list.
- **For `verify`:** ⚠ PUNCH LIST

### Gates (pinned 1.90.0, `RUSTUP_TOOLCHAIN=1.90.0`, `~/.cargo/bin` first)

`just lint` 0, `just test` 0 (33 suites ok, no `SKIP`, so `png_webp_live_parity` ran live on
the 317 inputs), `just mutation-oracle` 0 (p1 to p3 ok), `just forbid-unsafe` 0,
`just validate` 0, `just cost-audit` 0, `just oracle-record` twice with `git status` clean
both times. `git diff origin/main -- src` is empty. The diff under
`tests/oracle-fixtures/` adds only `exiftool-inventory/png-webp.txt`, so the five old records
are byte-identical. The old-corpus pin (34 rows) and its counts are unchanged.
`decisions/INDEX.md` is not tracked. `fuzz/Cargo.lock` was tracked on `main` before this
spec (since `0fb2f88`) and the spec does not change it. No `RED-PROOF INJECTION` marker is in
`src/`. (The local `main` ref was stale, at `ec078c8`, so every comparison here is against
`origin/main`, `40703fa`.)

### Punch list (ranked)

**H1. An audit bug is hidden behind a pin whose stated cause is wrong
(`png-raw-profile-keyword-case`).** ExifTool 13.55 matches the keyword
`raw profile type exif` without regard to case and DECODES the payload. A tEXt or zTXt chunk
with that lower-case keyword and a valid Exif payload makes ExifTool list `IFD0 Make` and
`GPS GPSLatitudeRef`. The audit lists one `Text` region (FreeText), and
`health().clean == true`. Measured on `v036/files3/png3-raw-lower-gps.png` and
`png3-raw-lower-gps-ztxt.png`. Diff: theirs only `GPS 0x0001 x1`, `IFD0 0x010f x1`; ours only
`text tags x1`. The upper-case `Raw profile type exif` is a RawProfile and not clean. The
all-caps keyword is not decoded by ExifTool either, so it agrees. The corpus row
`png-shape-meta-raw-lowercase` is also `clean == true`, although ExifTool prints a raw-profile
tag (`EXIF_Profile`) for it. DEC-020 §5 says such a chunk makes the file unreadable. The
question's "neither is wrong ... the raw profile is not decoded either way" is false.
Per the spec this is a real `src/audit` bug, to be recorded and fixed in a fix round:
match the raw-profile keyword case-insensitively. Then re-measure the row and raise the
question's priority.

**H2. The text count is a net scalar, so differences in opposite directions cancel and the
harness reports agreement while ExifTool's listing differs.** `diff` keeps only
`ours.saturating_sub(theirs)` and the reverse. Measured cases with `AGREE=true`:
`png-cancel-after-iend-text-plus-badtext` (ExifTool `PNG:TextualData` for a tEXt with no
NUL, which the audit does not list, cancels the after-IEND `Comment` that ExifTool puts in
`Trailer`); `png-cancel-gama-plus-after-iend-text` (`clean == true`; `PNG:Gamma` cancels the
after-IEND Comment); `png-kw-xmp-plus-badtext` (a text keyword `XMP` dropped by name cancels a
`TextualData` the audit does not list). Fix: count text and time tags by name, or at least by
cause, rather than as one net number. Also cover the corpus with at least one combined
ours-only plus theirs-only text input.

**M1. Non-text PNG chunk tags with no id are counted as text tags.** The spec and the
`PNG_CHUNK_GROUP_PREFIX` doc say other structural chunks print as `PNG-<chunk>`. That holds
only for `pHYs` and `cICP`. ExifTool prints these id-less in group `PNG`, so each one is
counted: `Gamma` (gAMA), `SRGBRendering` (sRGB), `BackgroundColor` (bKGD), `Transparency`
(tRNS), `SignificantBits` (sBIT), `ImageOffset` (oFFs), `AppleDataOffsets` (iDOT, which
every macOS screenshot has), `PixelCalibration` (pCAL), `PaletteHistogram` (hIST) and
`SuggestedPalette` (sPLT). Most real PNGs carry gAMA or sRGB, so every such file gives
`theirs only: text tags xN`. Measured: `png-real-chunks` theirs x6, `png-gama-only`,
`png-srgb-only`, `png2-*`. `cHRM`, `sCAL`, `sTER`, `vpAg` and `acTL` print hex ids and are
dropped correctly. The corpus has none of these chunks, so the tests are green. On its own
this is a false red, not a silent pass, but it feeds H2 (gAMA plus after-IEND text agrees).
Fix: drop these names (measured list) or classify by chunk, and add them to the corpus.

**M2. The dropped-name rule does not look at where a line came from.** A real text keyword
spelled like a dropped name loses ExifTool's text tag. Measured: keywords `XMP`, `Palette`,
`ProfileName`, `ICC_Profile`, `My_Profile`, `EXIF_Profile` and `RawProfileTypeX` each print
`[PNG] <keyword>` and are dropped, giving ours only `text tags x1`. Alone each is a loud
difference. Combined with any theirs-only line it is a silent one (H2). `eXIf` is not
over-broad: ExifTool prints a keyword `eXIf` as `EXIf`, which is counted, and they agree. The
suffix `_Profile` is the broadest rule. At least pin this as a documented limit; better,
derive the dropped lines from chunk structure.

**M3. The stated cause of `png-after-iend-in-trailer` (and the `STRUCTURAL_GROUPS` doc) is
overgeneralised.** "An eXIf only as a byte order, never the entries" and "no XMP or ICC
group" are false in general. An eXIf after IEND with Make makes ExifTool list `IFD0 Make`
(`png-exif-after-iend`, which agrees). A valid iTXt XMP after IEND gives `XMP-xmp`
(`png-xmp-itxt-after-iend`, which agrees). On `png-shape-meta-trailer-chunks` the `Xmp` and
`Icc` regions differ because the contents are invalid (`<y/>`, an iCCP that does not
inflate), not because of where they sit. ExifTool prints them as `Trailer:TagY`,
`Trailer:ProfileName` and `Trailer:ICC_Profile`. Dropping `Trailer` wholesale also drops the
real text tag `Trailer:Comment`, which is the H2 mechanism. Correct the measured text and
the question notes. Consider counting `Trailer` text lines.

**L1.** No test asserts that the Class P rows name canonical, unique png-webp keys. The
old-corpus check now skips P. I measured all 56 as canonical and unique, so no row is dead
today and the replay checks each one exactly.

**L2.** p1 to p3 mutate `src/audit`, not the harness's block rule. "First-block rule removed"
is covered only indirectly, by p3 (block index forced to 0). My harness mutants below close
that gap.

**L3.** The README's "317 distinct PNG and WebP inputs" is accurate. The png-webp set is
disjoint from the older corpora: union 519 = 202 + 317. The pre-existing "183 distinct JPEG
and TIFF" is unchanged, and this spec does not touch it.

### Pinned divergences (56)

All 56 Class P rows are canonical, unique keys, and each is replayed with an exact
`pinned() == diff` check that covers tags, regions and both text counts. All 12 question ids
exist in `guidance/questions.yaml`. `a_png_webp_divergence_that_stops_being_one_fails` shows
three outcomes: `StoppedDiverging`, `ChangedDiff` and `HidesButClean`. The causes I
re-checked hold for the corpus inputs, except H1 (wrong right-side or cause) and M3
(placement given as the cause, where the contents are).

### Red-proofs

Replay-and-perturb (`png_webp_oracle_names_a_perturbed_record`) names `IFD0 0x010f`, the text
count, and both sides of a regroup, and an unknown group is a parse error. p1 to p3 run the
control first, use one injection and the marker, and pass under `just mutation-oracle`. My
own mutants ran on a scratch copy of the repo. The control there is green (79 passed). The
copy was restored afterwards and compared byte-identical (`cmp`) with the worktree file.

| mutant (tests/support/inventory.rs) | failing tests |
|---|---|
| `Palette` renamed out of `PNG_DROPPED_NAMES` (array kept at 5) | png_dropped_names_are_pinned, png_webp_replay_is_clean_or_pinned, png_webp_live_parity, text_and_time_count_agrees |
| first-OK block -> first block (`position(\|_\| true)`) | png_first_block_rule_and_later_blocks_listed, png_webp_replay_is_clean_or_pinned, png_webp_live_parity |
| WebP counts only block 0 | per_format_block_rule_of_ours, webp_every_block_compared_and_cap_pinned, png_webp_replay_is_clean_or_pinned, png_webp_live_parity |
| RawProfile regions counted as text | text_and_time_count_agrees, png_webp_replay_is_clean_or_pinned, png_webp_live_parity |

### Variant grid (ExifTool 13.55 `-a -u -G1 -H -s`, all under the 8 s alarm; files under the scratchpad `v036/files*`)

| input | harness | true listing vs audit |
|---|---|---|
| png-exif-{FEV,FVE,EFV,EVF,VEF,VFE}, FF, FM, VM, trunc-then-V | agree | correct: ExifTool reads the first block that parses as TIFF (an empty valid block stops it); `png_first_ok_block` matches in all orders |
| png-exif-before-idat / after-idat / after-iend / before+after | agree | correct; after IEND, ExifTool lists the IFD0 entries (see M3) |
| png-time-text-spellings (tIME, Creation Time x3 cases, Author/author, iTXt Title-en, zTXt Comment, Warning) | agree 9 = 9 | correct |
| png-two-time | agree 2 = 2 | correct |
| png-xmp-{itxt, itxt-z, text, ztxt, itxt-after-iend} | agree | correct (XMP-xmp in every case) |
| png-iccp-real (+ text) | agree | correct (ProfileName dropped, ICC groups present) |
| png-kw-{XMP, Palette, ProfileName, ICC_Profile, My_Profile, EXIF_Profile, RawProfileTypeX} | ours text x1 (loud) | M2: ExifTool's text tag dropped by name |
| png-kw-eXIf / Comment / Title | agree | correct (`eXIf` prints `EXIf`) |
| png-gama-only, png-srgb-only, png-real-chunks, png2-{idot, pcal, plte-hist, splt, gama-srgb-text} | theirs text xN (loud) | M1 |
| png-cicp-only, png-actl, png2-{scal, ster, vpag} | agree | correct (PNG-cICP or hex ids) |
| **png-cancel-after-iend-text-plus-badtext** | **agree** | **blind spot (H2)**: TextualData theirs-only cancels the Trailer Comment |
| **png-cancel-gama-plus-after-iend-text** (clean) | **agree** | **blind spot (H2)** |
| **png-kw-xmp-plus-badtext** | **agree** | **blind spot (H2 + M2)** |
| png-cancel-rawlower-plus-badtext | theirs IFD0 Make | H1 (text counts cancel too) |
| **png3-raw-lower-gps(-ztxt)** (clean == true) | theirs GPS + Make, ours text | **H1: audit bug** |
| png3-raw-upper-gps | theirs GPS + Make, RawProfile | expected (`png-raw-profile-not-decoded`) |
| png3-raw-allcaps-gps | agree | correct (ExifTool does not decode it) |
| webp-exif-{A-broken-B, broken-A-B, A-empty-B} | agree | correct (every listed block, broken skipped) |
| webp-flags-none-but-exif-xmp, flags-all-no-chunks | agree | correct (flags ignored on both sides) |
| webp-odd-exif-nopad, trailing-bytes, no-vp8x-exif, animated-exif, xmp-icc-real | agree | correct |

### Cost self-report

- **Tokens (total):** null. Left for the orchestrator to record from the harness, by instruction.
- **Estimated USD:** null
- **Duration (minutes):** null
- **Source of the number:** harness report (orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the build. The text-count model (a net count, with names
  dropped regardless of origin) follows the spec's "the PNG text and `tIME` tags as a count"
  literally. The count design itself causes H2.
- **Follow-up work identified:** a fix round for H1 (src: raw-profile keyword match that
  ignores case) and for H2, M1 to M3 (harness: text tags by name or cause, the non-text chunk
  names, the Trailer causes, and corpus inputs for each).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing blocking. The design's claim that structural chunks print as `PNG-<chunk>`
   (repeated in a doc comment) was taken as measured but was not, which is M1.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-020 §5 (raw profile = unreadable) should have been the check on every raw-profile
   pin. Applied to `png-raw-profile-keyword-case`, it shows H1.

3. **If you did this task again, what would you do differently?**
   — Probe combinations (one ours-only plus one theirs-only difference in a single file)
   whenever a comparator reduces two sides to one number.
