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
  id: HANDOFF-043
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-10
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-021

project:
  id: PROJ-001
  stage: STAGE-004
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
  branch: worktree-agent-a4b6cc47777fd05ee
  pr: null
  completed_at: 2026-11-10         # YYYY-MM-DD
  notes: "tokens_total, estimated_usd and duration left null on purpose: the verifier cannot see its own token count; the orchestrator transcribes the harness's metered subagent_tokens and duration. Verdict PUNCH LIST at c1c01bd (code 1cf8bd1): one High leak sibling of H2 (IFD1 tile, JPEG-table and free-space offsets left unreferenced and unzeroed when the thumbnail goes), one Medium test gap (rule-3 per-block refusal pinned only when the bad block is last), Lows. No code changed, nothing pushed, no cycle advanced."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-043: Re-verify JPEG in-place Exif entry deletion with scrub (SPEC-021, round 2)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-021` to `claude-opus-5-5` (verifier, a fresh cold agent with no memory of round 1) for a second **verify** cycle after a punch list.

## Context the Receiving Agent Needs

- Everything in `HANDOFF-042-verify-*.md` still applies (spec, DEC-013 with its 2026-11-10 amendment, gates, privacy rule for `_incoming0`: read in place, copy nothing, write no derived file, pipes only, counts and 8-hex hashes only, never values or paths; host traps; do not fix code, push, open a PR or advance cycles). Read it, including the round-1 verdict in its Handback.
- **Round 1 (PUNCH LIST at 7697162)** found: H1 (a deleted entry's out-of-line value running past the blob end was left unscrubbed, `Ok` with the canary in the output); H2 (thumbnail removal scrubbed only the first 0x0201/0x0202 range, leaving a second pair or strip data); M1 (the per-block refusal loop was tested only on the first Exif block); Lows L1 to L5. The fixes are `git diff 7697162..HEAD` (commit `1cf8bd1` and its WIP commits; the follow-up for the stray-bytes harness crash is earlier, `56bd4a6`, `9315047`). Review the fixes as a new reader, not as a diff checklist.
- **Do not anchor on round 1.** H1 and H2 were the same class (the planner assumed a range or locator it had not proved). Hunt for siblings by reading `src/write/exif.rs` and `src/tiff/mod.rs` accessors for every place that takes `None`, `first`, `get(..)`, a count, a pointer or a length and acts as if the answer were complete. Specifically try: entries whose type size times count overflows or wraps; counts above the blob; value offsets in the header or inside another IFD's table; two IFDs sharing a value; a thumbnail range inside a kept IFD or a MakerNote; 0x0201 with count != 1 or a non-LONG type; 0x0117 without 0x0111; GPS and thumbnail entries of unknown field types; a SubIFD (0x014A) pointing at the GPS IFD or IFD1; Interop pointing into a removed IFD; IFD1 reached from a SubIFD chain; truncated entries at the blob end; both byte orders for each; and Exif segments that differ (two blocks, one clean, one not; the second block holding the only GPS).
- **Judge the L4 widening.** The harness excuse `walks_as_other_segments` now covers `VerifyFailed` checks `mpf-fields`, `walk` and `entries` (the spec asked for the first two; the builder added `entries` after a 60 s smoke failed on a 4.4 KB input, now seed `crash-entries-joined-stray`, claiming the same stray-bytes class by structure, not verified at byte level). Establish at byte level why the library returns `VerifyFailed{Entries}` there: is it truly the joined-stray class, or a planner bug that `Entries` caught? If a planner bug, that is at least High.
- Orchestrator's own gate run at `1cf8bd1` (all steps exit 0; 2577 tests passed, 0 failed over three runs; five 60 s smokes clean) is evidence, not a substitute: re-run everything in HANDOFF-042's gate list, plus `rewrite_jpeg` for 5 minutes.
- Re-run mutants V04, V05, V08, V11 (round 1 survivors) and the builder's new ones from the HANDOFF-041 Handback addendum; write at least six mutants of your own aimed at the H1/H2 fixes (e.g. the straddle test using `>` instead of `>=`; H2 counting only 0x0201; H2 skipping 0x0117; the mirror in the harness removed; the later-block loop checking only the last block) and name the test or guard that catches each.
- Re-run the local corpus counts (expected: `Gps` 13 Ok, `GpsAndDevice` 13 Ok, `AllPii` 12 Ok + 1 `Uneditable(ChainAfterThumbnail)` on hash `a3b66f84`, `All` 13 Ok, `Everything` 13 Ok; 21 files, 13 JPEGs, 4 with Multi-Picture) and pipe a few outputs to `exiftool -` as in round 1.

## Expected Deliverables

Output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA, with findings by severity and concrete reproducers, mutant results, local-corpus counts, and a verdict on each open builder question in `guidance/questions.yaml`. Fill the HANDOFF-043 handback front matter (`tokens_total: null` with a note; the orchestrator transcribes the metered figure).

## Out of Scope

- Implementing fixes; TIFF/DNG (SPEC-022); `strip-keeps-orientation`; changing DEC-013.

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

- **Branch / PR:** worktree branch `worktree-agent-a4b6cc47777fd05ee` (fast-forwarded to c1c01bd; only this handback is committed on top). No PR, nothing pushed.
- **Completed at:** 2026-11-10
- **All acceptance criteria met?** No. DEC-013 amendment B.3 ("every table or data range of a removed child IFD ... gone from the file, not just unreferenced") still fails on a sibling of H2 (finding H3 below).
- **For `verify`:** ⚠ PUNCH LIST at c1c01bd (code 1cf8bd1).

**Gates, re-run by the verifier (all exit 0):** `cargo fmt --all -- --check`, fuzz-graph fmt, `rustfmt --check --edition 2021 fuzz/shared/*.rs`, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `forbid-unsafe`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, release build of `exif-audit` on 1.90.0, `just oracle-record` and then `git status --porcelain` empty. `cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`: stable (Homebrew 1.99.1) 859 passed, 0 failed, 9 ignored; `+1.90.0` the same; no SKIP line. Fuzz from the committed seeds: `rewrite_jpeg` 301 s, 314,482 runs, no crash; 60 s each of `tiff_ifd` (1,483,007 runs), `jpeg_segments` (181,968), `audit` (51,178), `audit_render` (35,811), `cli_args` (2,339,961): no crash. `decisions/`, `scripts/` and every manifest and lock file unchanged since 6518356; the new `unwrap`s in the diff are all in test modules.

**Findings.**
- **H3 (High, a leak; a sibling of H2 that the fix missed): IFD1 data reached through tile, JPEG-table or free-space offsets stays in the file after the thumbnail is removed.** The new `single` check in `plan_block` (and its mirror in the harness) only looks at 0x0201/0x0202/0x0111/0x0117. An IFD1 that also holds `TileOffsets`/`TileByteCounts` (0x0144/0x0145), `JPEGQTables` (0x0207, which also covers 0x0208/0x0209) or `FreeOffsets`/`FreeByteCounts` (0x0120/0x0121) has those entries deleted (their in-line 4-byte offset goes with the table), but the bytes they point to are never scrubbed. Reproducer (LE and BE): a 160-byte TIFF with IFD0 at 8 = {0x0112 SHORT 1 = 1} and next 26; IFD1 at 26 = {0x0103 SHORT 6; 0x0201 LONG 1 -> 100; 0x0202 LONG 1 = 20; 0x0144 LONG 1 -> 140; 0x0145 LONG 1 = 20} and next 0; at 100 `FF D8 CANARY-THUMB-AAA FF D9`; at 140 `CANARY-TILE-XXXXXXXX`; one Exif APP1 in a minimal JPEG. `AllPii` and `Custom{Thumbnail}` both return `Ok`, same length, 5 entries, 2 scrubs, and `CANARY-TILE` is still in the output. The same happens with 0x0207 -> 140, and with 0x0120 -> 140 plus 0x0121 = 20. `rewrite_invariants::check` holds on all six inputs (policy byte 6, mask 0x0010), so the harness has the same blind spot. Suggested fix: when IFD1 goes, refuse `ThumbnailNotLocated` unless every IFD1 tag is on an allow-list of tags that hold no offset (or scrub every range an IFD1 offset tag names). Mirror it in the harness and add the inputs to the leak list and the seeds. Corpus: 0 of 13 local JPEGs have such a tag in IFD1 (ExifTool by pipe, tag names counted).
- **M2 (Medium, test gap on a safety rule; the M1 fix is only half done): rule 3's per-block refusal is pinned only when the bad block is the last one.** Mutant N10 (`for block in meta.exif_blocks().iter().rev().take(1)`) survives every test. With it applied, two Exif segments where the FIRST is unclean (LE, IFD0 {0x0112, 0x8825 -> 38}, a GPS IFD at 38 whose count is 0xFFFF and whose one real entry 0x001B UNDEFINED 19 points at the canary `CANARY-GPS-FIRSTBLK` at 80) and the second is clean (IFD0 {0x0112}) return `Ok` with the canary left, under `Gps` and `AllPii`. The real code refuses (`Uneditable { BlockNotClean, offset 2 }`). Add this ordering, and ideally three blocks with the bad one in the middle, as a test and a seed.
- **L6 (Low, test gap): the H1 refusal is pinned only for long values.** Mutants N03 (`len > 4` becomes `len > 8`) and N15 (`len > 7`) survive: every straddling fixture is 40 or 60 bytes long. A regression that only refused values longer than 8 bytes would let a straddling RATIONAL (8 bytes) or a 5-to-8-byte ASCII leak again. Add straddlers whose `byte_len` is 5 and 8.
- **L7 (Low, test gap): two 0x0201 entries with one 0x0202 are not tested.** Mutant N07 (`count(0x0201) >= 1`) survives because every two-locator fixture also repeats 0x0202. Add IFD1 = {0x0201 -> A, 0x0201 -> B, 0x0202 = n}.
- **L8 (Low, integrity on malformed input): a kept entry whose payload cannot be located is not protected.** The keep set uses `payload_range`, which is `None` for a kept value that runs past the blob end or has an unknown type, so a scrub can zero its in-blob bytes. Reproducer: IFD0 at 8 = {0x010F ASCII 40 -> 60 (runs past the 80-byte blob); 0x0112; 0x013B ASCII 10 -> 60}, with `KEPTMAKE-ARTIST-QQQQ` at 60. Under `Custom{Authorship}` the result is `Ok`, and the kept Make's first 10 in-blob bytes are zeroed (`KEPTMAKE` is gone). The same happens with a kept entry of type 99. No reader can read the value before or after, so nothing leaks, but "a kept entry cannot change" does not hold byte-wise. Apply the H1 rule symmetrically: refuse `SharedBytes` when a scrub touches `[value offset, blob end)` of a kept entry with `byte_len > 4` (or of unknown size) whose `payload_range` is `None`.
- **Note (scope question for SPEC-022 and the DEC-013 note, no change asked here):** a MakerNote is scrubbed over its declared byte range only. Data that a MakerNote's own internal offsets point to outside that range is not reachable without parsing the MakerNote, which is out of scope for M1. Worth a sentence in DEC-013.

**L4 judged at byte level (the `entries` widening is justified).** Seed `crash-entries-joined-stray`, policy mask 0x6008 (FreeText, EmbeddedImage, OtherSegment). The input has JFIF, Exif (126-byte payload), 128 one-byte COM segments with stray bytes among them (1 byte at 150, at 176 and at 467; a 125-byte run of `7A` at 523; 5 bytes at 918), then a second JFIF and a second Exif at 941. COM is FreeText, so the only change under this policy is removing the 128 COMs. Cutting them by hand and walking the result gives JFIF plus the first Exif only: the joined stray bytes swallow the rest, so the output walks with 1 Exif block instead of 2. `Entries` sees the second block's entries go missing. With the 128 COMs flipped to APP15, FreeText plans nothing (0 entries, Ok), so there is no planner bug; `Entries` is the check that sees a changed walk when there is no MPF. Because the excuse only ever covers a refusal, it cannot hide a leak. Mutant N12 (drop `entries` from the excuse) is caught by the crash-seed test and the seed sweep; N11 (excuse every check) is caught by `a_removal_that_joins_stray_bytes_...`.

**Siblings tried that fail closed or are correct (both byte orders):** SubIFDs at the GPS IFD (`BlockNotClean`); two blocks with the GPS only in the second (Ok, canary gone, under `Gps` and `AllPii`); a GPS entry of unknown type (`PayloadUnreadable`); a thumbnail inside a kept MakerNote under `Custom{Thumbnail}` (`SharedBytes`); 0x0202 with count 2 (`ThumbnailNotLocated`); 0x0117 without 0x0111 (`ThumbnailNotLocated`); BYTE count 0xFFFFFFFF and RATIONAL count 0x20000000 (`PayloadUnreadable`); a deleted value in the TIFF header (`SharedBytes`); SHORT-typed 0x0201/0x0202 (Ok, canary gone).

**Mutants (scratch copy, exact text replacement, `cargo test --lib --test strip --test fuzz_layout --test strip_oracle`, restored from the saved text and diffed against the worktree).** Round-1 survivors are now caught: V04 by `l1_scrubbed_checks_every_range_not_only_the_first`; V05 by `l2_an_empty_gps_ifd_is_removed_with_its_pointer` and `an_empty_gps_ifd_behind_a_present_pointer_is_removed_with_it`; V08 by `a_later_block_with_a_walk_issue_still_refuses_under_every_partial_policy`, `a_later_block_after_the_first_scan_is_uneditable` and the seed sweep; V11 by `l3_overlapping_ifd_tables_are_shared_bytes_even_when_only_one_is_edited`. My mutants: N01 (H1 fix reverted) caught by both `h1_*` unit tests, `leak_inputs_refuse_with_the_named_reason_and_offset`, `no_leak_input_is_ok_with_a_canary_left_under_any_policy` and the harness seed sweep; N02 (`>= 4`) caught by 29 tests; N03 (`> 8`) and N15 (`> 7`) SURVIVED (L6); N04 (0x0202 count ignored), N05 (0x0117 skipped) and N06 (0x0111 skipped) each caught only by `h2_a_thumbnail_ifd1_with_other_locators_is_thumbnail_not_located`; N07 (`0x0201 >= 1`) SURVIVED (L7); N14 (the `single` filter removed) caught by the `h2_*` unit test, both leak tests and the harness seed sweep; N08 and N09 (harness mirrors removed) survive by design, because the harness accepts any library refusal as fail-closed (N01 and N14 show the mirrors catch a library regression); N10 (rule-3 loop takes the last block only) SURVIVED and leaks (M2); N11 caught; N12 caught; N13 (BlockNotClean dropped from the loop) caught by 13 tests.

**Local corpus (`_incoming0`, read in place, counts and 8-hex hashes only, nothing written).** Same counts as round 1: 21 files, 13 JPEGs, 4 with Multi-Picture. `Gps` 13 Ok, 0 bytes removed, 1,035 bytes patched, 22 entries; `GpsAndDevice` 13 Ok, 1,968 bytes patched, 24 entries; `AllPii` 12 Ok plus 1 `Uneditable(ChainAfterThumbnail)` on `a3b66f84`, 100,969 bytes patched, 154 entries; `All` 13 Ok, 313,995 bytes removed; `Everything` 13 Ok. On every Ok: scan identical, audit clean, deleted values zero; same length on every Ok under the three partial policies. ExifTool by pipe (`-a -u -G1 -s -n -t`), one file at a time, input vs `gps`/`gps_and_device`/`all_pii`, 51 runs plus 13 IFD1 tag counts: `GPS` gone only from the 4 files that had it; `IFD1` gone under `AllPii` in the 4 Ok files that had it and kept under the other two policies; Orientation count unchanged except where it lived in the removed IFD1 (`8a083030` goes from 2 to 1, IFD0's kept); `MPImage1-3` and `MPF0` unchanged in all 4 Multi-Picture files; 0 new warnings; 0 files with an IFD1 tile, JPEG-table, free-space or strip tag.

**Questions.** `allpii-refuses-ifd1-with-a-following-ifd`: fails closed. Decide it before SPEC-022, where TIFF chains are the norm. `writer-refusals-beyond-the-spec`: all fail closed. Ratify them in a DEC-013 note before SPEC-022, and widen the `ThumbnailNotLocated` wording to an allow-list per H3; the note's "sweep of the same class" missed tiles and JPEG tables. `partial-policy-refuses-unreadable-exif-for-xmp-icc-iptc`: fails closed and follows the definition; no decision needed before SPEC-022. `writer-self-check-fires-on-segments-joined-by-stray-bytes` (answered): agreed. With `entries` added, the excuse is exact at byte level (see L4 above).

### Cost self-report

- **Tokens (total):** null. The verifier cannot see its own token count; the orchestrator transcribes the harness's `subagent_tokens`.
- **Estimated USD:** null (follows the tokens).
- **Duration (minutes):** null (the harness reports it).
- **Source of the number:** harness report, to be transcribed.

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier.
- **Follow-up work identified:** a builder follow-up for H3, M2 and L6 to L8 (no new spec). Scope note on MakerNote-internal offsets for DEC-013.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Rule 7 lists the thumbnail's locators by tag instead of saying "every offset an IFD1 entry holds"; the fix copied that enumerative shape, so the next tag the list did not name leaked.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-013 B.3 already covers it ("every table or data range of a removed child IFD"); the planner should implement it as an allow-list of offset-free tags, not a deny-list of known locators.

3. **If you did this task again, what would you do differently?**
   — Keep my scratch files in a fresh subdirectory from the start. The scratchpad is shared with the orchestrator, and my first gate script and probe overwrote its `gates.sh`, `gates.out`, `g.*.txt` and `probe/{Cargo.toml,src/main.rs}`.
