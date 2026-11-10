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
  id: HANDOFF-042
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
  branch: worktree-agent-a6c98b4cf6b8abd61
  pr: null
  completed_at: 2026-11-10         # YYYY-MM-DD
  notes: "Verifier cannot see its own token count; the orchestrator transcribes the metered subagent_tokens and duration from the harness. Verdict PUNCH LIST at 7697162: two High leaks on malformed input (straddling out-of-line value not scrubbed; IFD1 thumbnail bytes reachable through a second 0x0201 or through 0x0111 strips left unreferenced), one Medium test gap (rule 3 per-block refusal only tested on the first block), Lows. No code changed, nothing pushed, no cycle advanced."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-042: Verify JPEG in-place Exif entry deletion with scrub (SPEC-021)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-021` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-021-*.md` (use the exact spec file, not its `-timeline.md`): rules 1 to 14, the fixtures and leak inputs, the oracle, the fuzz plan, the named tests and the 30 mutants are the contract.
- **DEC-013 with its 2026-11-10 amendment (normative; part A ratifies SPEC-020's refusal order, part B is the entry-editing contract), DEC-012, DEC-009, DEC-008, DEC-005, DEC-001:** `decisions/`.
- **Build handoff and Handback:** `HANDOFF-041-build-...md` (claims to verify, not to trust; read the eleven deviations). Open questions the builder raised are in `guidance/questions.yaml`: `allpii-refuses-ifd1-with-a-following-ifd`, `writer-refusals-beyond-the-spec` (`HiddenPointer`, `NestedIfd`, `PayloadUnreadable`, always-on table-overlap check), `partial-policy-refuses-unreadable-exif-for-xmp-icc-iptc`. A fourth (joined stray bytes) was answered by a follow-up (below).
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; Python reading ExifTool output needs `errors="replace"`; macOS `sed -i ''`; do not leave the shell in a subdirectory).
- **Code under review:** `git diff 6518356..HEAD` (the build `defcd75` and its WIP commits, the orchestrator's cost record `8ecbb29`, and the follow-up `56bd4a6`, `9315047`). Main: `src/write/{exif,mod,plan,jpeg,verify,verify_tests}.rs`, the tiff accessors in `src/tiff/mod.rs`, `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/fuzz_layout.rs`, `tests/support/{exif_layout,exif_records,strip_shapes,strip_records}.rs`, `tests/oracle-fixtures/exiftool-strip/`, `fuzz/fuzz_targets/rewrite_jpeg.rs`, `fuzz/shared/rewrite_invariants.rs`, `fuzz/seeds/rewrite_jpeg/`, `examples/strip_corpus.rs`.
- **Follow-up already made (judge it):** my independent 60 s `rewrite_jpeg` smoke crashed with guard `VerifyAllowed` (`VerifyFailed("mpf-fields")`, harness predicted Ok). Root cause per the builder: removing an XMP segment makes fill bytes meet stray bytes and read as a marker of length 27,760, so the output walks without the MPF index; the library correctly refuses; reproduces on the SPEC-020 library. Fix was harness-only (`walks_as_other_segments`), with a deterministic test and two seeds. Decide whether the excuse is exact or could hide a real library bug.
- **Local photos:** `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0`. **Privacy rule: read in place, copy nothing, write no derived file (no temp files, no saved ExifTool output; pipes only), record only counts and 8-hex short hashes; never tag values, file contents or paths; run the binary without `--values`.** Public samples may be in the orchestrator scratchpad `.../scratchpad/public/{A,B,C}`.
- Orchestrator's own gate run after the follow-up (all steps exit 0; 2547 tests passed, 0 failed over three runs; five 60 s smokes clean) is evidence, not a substitute: re-run.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- **Gates:** fmt both graphs and `rustfmt --check --edition 2021 fuzz/shared/*.rs`; `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`; `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`; `-F unsafe_code`; `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`; `oracle-record` leaves an empty `git status --porcelain`; the five fuzz smokes; release build of `exif-audit`. Confirm `decisions/` and `scripts/` unchanged, no dependency added, no `allow` outside test modules (one `#[allow(dead_code)]` in the fuzz harness is declared), no `unsafe`, no `unwrap`/`expect`/indexing/panic/arithmetic in `src/write/` and the new tiff accessors.
- **Read `src/write/exif.rs` line by line.** The refusal order (TooManyExifSegments, InventoryTruncated, AfterScan and BlockNotClean for every block, then per-block planning); the keep set K and `SharedBytes` (a scrub range touching any kept byte, including kept IFD tables, kept values, kept thumbnails); GPS child removal (pointer deleted from the parent, table and values scrubbed, nested or doubly-pointed IFDs); thumbnail child removal (IFD0 next word to 0; IFD1 table, values and thumbnail bytes scrubbed; `ChainAfterThumbnail`); entry deletion (count minus k, survivors copied in order, next-IFD word, 12k zero bytes; byte order handling; counts at the 16-bit limit; empty-IFD behaviour); MakerNote scrub; the three extra reasons. Every `Ok` path: can an entry the audit reports in a removed category survive, can a kept entry change, can a byte outside the recorded patches change? Every `Err` path: no bytes escape.
- **Try the inputs the builder did not:** entries sorted and unsorted; duplicate tags; a deleted entry whose value is inline vs out of line vs overlapping another entry's value; value offsets pointing into the IFD table of another IFD; a GPS pointer and an Exif pointer to the same offset; IFD1 reached by two pointers; thumbnail bytes inside the MakerNote; count fields near 0xFFFF; IFD offsets at `u32::MAX`; a TIFF header with a bad magic; Exif block of length 6 or 8; big-endian blocks with the same shapes; Exif plus MPF files where the Exif segment is removed or patched; an Exif segment after the scan; 17 Exif segments; Exif-resident Xmp/Iptc/Icc tags under partial and whole policies; `Orientation` under `Gps`, `AllPii`, `All`.
- **Fail-closed and independence.** Look for a path where `Entries` or `Scrubbed` is skipped, short-circuited, or compares the output with itself; check 7(b) exemption applies only when the whole Exif segment was removed; check (g) derives the deleted set from the input, not the plan. Confirm the fuzz harness's naive TIFF reader/editor shares no code with the library (list every library call the harness makes and judge each) and that `walks_as_other_segments` is exact.
- **Oracle discipline.** ExifTool run, never linked, `-a -u -G1 -s -t -n`. Re-measure on at least three public samples and the fixtures; check the declared tag tables state what ExifTool printed, not what the library did; the six red-proofs still go red; re-record the emptied-IFD measurement (rule 9) and judge allow vs refuse. Confirm the three re-recorded SPEC-020 records changed only for the reason stated.
- **Test the testers.** Re-run the 30 mutants for at least 1, 4, 7, 11, 13, 14, 17, 19, 20, 22, 23, 25, 26, 29 and 30. Write mutants the builder did not try, for example: scrub applied before the shift; deletion shifts survivors but forgets the next-IFD word in big-endian only; `SharedBytes` compares against removed instead of kept; `Entries` compares tags but not values; `Scrubbed` checks only the first range; GPS child removal skipped when GPS has no entries; the audit-derived target set replaced by a hard-coded tag list; the 16-segment limit off by one; `BlockNotClean` testing one counter only; `walks_as_other_segments` always true. Report which the suite catches (name the test or guard) and which survive.
- **Fuzz.** Redo one plant against the real libFuzzer `rewrite_jpeg` target from the committed seeds only; run `rewrite_jpeg` for 5 minutes and report any crash with its input hash; check seeds cover every new policy-byte family.
- **Local real-file run (counts only).** Run `examples/strip_corpus.rs` over `_incoming0` under `Gps`, `GpsAndDevice`, `AllPii`, `All`, `Everything` and reproduce the builder's numbers (21 files, 13 JPEGs, 4 with Multi-Picture; `Gps`/`GpsAndDevice` 13 Ok, 0 bytes removed, length equal, scan identical, audit clean; `AllPii` 12 Ok and one `Uneditable(ChainAfterThumbnail)` on hash `a3b66f84`). Pipe a few outputs to `exiftool -` (in memory only) and compare groups, the `Orientation` count and `MPImage` groups with the input's; confirm 0 new warnings.
- **Judge the questions.** For each open question the builder raised, say whether the pinned behaviour fails closed and whether it needs a decision before SPEC-022.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles. A Low finding goes to a builder follow-up via the orchestrator, so give each a concrete reproducer.

## Out of Scope

- Implementing fixes; TIFF/DNG (SPEC-022); keeping Orientation under `All`/`Everything` (`strip-keeps-orientation`); XMP or IPTC editing; changing DEC-013 (report doubts as questions).

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

- **Branch / PR:** worktree branch `worktree-agent-a6c98b4cf6b8abd61` (fast-forwarded to 7697162; only this handback is committed on top). No PR, nothing pushed.
- **Completed at:** 2026-11-10
- **All acceptance criteria met?** No. "Every leak input refuses or is correct, never `Ok` with a canary left" and DEC-013 amendment B.3 ("the bytes are gone from the file, not just unreferenced") fail on malformed inputs the suite does not have (findings H1 and H2 below).
- **For `verify`:** ⚠ PUNCH LIST at 7697162.

**Gates, re-run by the verifier (all exit 0):** fmt (library, fuzz graph, `rustfmt --edition 2021 fuzz/shared/*.rs`), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `forbid-unsafe`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, release build of `exif-audit` (1.90.0 and stable), `just oracle-record` then `git status --porcelain` empty. `cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`: stable 1.99.1 849 passed, 0 failed, 9 ignored; 1.90.0 the same; no SKIP line. Fuzz: `rewrite_jpeg` 300 s from the committed seeds, 275,995 runs, no crash; 60 s smokes of `jpeg_segments`, `audit`, `audit_render`, `cli_args`. `decisions/`, `scripts/` and every manifest and lock file unchanged since 6518356; no new `allow` outside test modules except the declared `#[allow(dead_code)]` in the harness.

**Findings.**
- **H1 (High, a leak): the out-of-line value of a deleted entry that runs past the end of the blob is not scrubbed.** The tolerant walk does not check ordinary entry payloads, so such a block is "clean"; `payload_range` returns `None` and the planner treats it like an inline value (`None => {}` in `plan_block`), so the bytes of the value that ARE inside the blob stay. Reproducer: LE TIFF, IFD0 at 8 = {0x0112 SHORT 1 = 1; 0x013B ASCII count 40, offset 38}, next 0; 20 bytes `CANARY-ARTIST-PREFIX` at 38 (the blob ends at 58); one Exif APP1 in a minimal JPEG. `strip(.., Custom{Authorship})` and `strip(.., AllPii)`: `Ok`, length unchanged, `entries()` 1, `scrubs()` 0, canary still in the output. Same in BE, and the same for a GPS IFD entry (IFD0 {0x0112, 0x8825 -> 38}; GPS IFD at 38 = {0x001C UNDEFINED count 60, offset 56}; canary at 56) under `Gps`. The fuzz harness has the same blind spot (`out_of_line` in `rewrite_invariants.rs` also returns `None`), so the six inputs pass the real `rewrite_jpeg` target. Fix: refuse (`PayloadUnreadable`) when a deleted or removed-IFD entry has `byte_len > 4` and `payload_range` is `None`; the naive editor must refuse the same way; add the input to the leak list and seeds. 0 of 550 entries on the local corpus have such a value.
- **H2 (High, a leak): thumbnail image bytes left unreferenced.** Rule 7 scrubs only the range the audit located (the first 0x0201/0x0202 pair). (a) IFD1 = {0x0201 -> 80, 0x0202 = 16, 0x0201 -> 96, 0x0202 = 20} with a second JPEG (`FF D8 .. CANARY-THUMB-B .. FF D9`) at 96: `AllPii` and `Custom{Thumbnail}` return `Ok` with the second thumbnail in the output. (b) IFD1 = {0x0111 -> 96, 0x0117 = 20, 0x0201 -> 80, 0x0202 = 16} with strip data at 96: `AllPii` returns `Ok` with the strip data left. Both orders. The harness predicts the same output (its `thumb` uses the first 0x0201 only). Fix: when the thumbnail goes, refuse unless IFD1 has exactly one 0x0201 and one 0x0202 and no 0x0111 (or scrub every range IFD1 points to). Corpus: 0 of 13.
- **M1 (Medium, test gap on a safety rule): rule 3's per-block refusal is only tested on the first Exif block.** Mutant V08 (the `AfterScan`/`BlockNotClean` loop takes only the first block) survives every test. Under it, two Exif segments (the first clean, the second IFD0 {0x0112, 0x8825 -> 38} with a GPS IFD of count 0xFFFF and a canary behind) return `Ok` with the GPS canary under `Gps`; the real code refuses (`Uneditable { BlockNotClean, 38 }`). Add that input as a deterministic test and a seed.
- **L1 (Low):** mutant V04 (`Scrubbed` checks only the first recorded range) survives; `a_scrub_that_leaves_a_byte_fails_check_scrubbed` uses one range. Add a test with two ranges, the second not zero.
- **L2 (Low):** mutant V05 (no GPS child removal when the GPS IFD has 0 entries) survives; no fixture or seed has an empty GPS IFD (the library would then fail closed through `Entries`, but nothing pins it). Add the input.
- **L3 (Low):** mutant V11 (the always-on table-overlap check removed) survives; the check is not pinned by any test. Add one or drop the claim.
- **L4 (Low):** `walks_as_other_segments` excuses any `VerifyFailed` check, not only the one stray-byte joins cause. The excuse is exact for the segment list (V09, "always true", is caught by `a_removal_that_joins_stray_bytes_into_a_marker_is_refused_and_excused_exactly`), and it only ever excuses a refusal, so it cannot hide a leak; it could hide a planner bug whose only symptom is a `VerifyFailed` on such an input. Restrict it to `mpf-fields` and `walk`, or record which check the follow-up crash produced.
- **L5 (Low, equivalent mutant):** V15 (the tag cross-check in `select` removed) survives; the `Entries` self-check is the backstop.
- **Note:** the public sample `sony-d700.jpg` (a real camera file with an uncompressed strips thumbnail) refuses `AllPii` with `ThumbnailNotLocated`, by design; worth a line in `allpii-refuses-ifd1-with-a-following-ifd` since SPEC-022 meets strips thumbnails often.

**Mutants (scratch copy, exact text replacement, `cargo test --lib --test strip --test fuzz_layout --test strip_oracle`, reverted by `git checkout`, empty diff after each).** Of the 30: 1, 4, 7, 11, 13, 14, 17, 19, 20, 22, 23, 25, 26, 29, 30 re-run, all caught (1 by 47 tests incl. `all_pii_matches_the_policy_table_entry_by_entry`; 4 by `custom_software_only`, `each_entry_guard_goes_red_alone_when_planted`; 7 by `custom_makernote_only` and 31 more; 11 by `custom_timestamps_and_gps`; 13 by `gps_removes_the_gps_ifd_and_keeps_everything_else`; 14 by `custom_thumbnail_only`; 17 by `thumbnail_with_a_following_ifd_is_chain_after_thumbnail`, `leak_inputs_refuse_with_the_named_reason_and_offset`; 19 by `custom_software_only` and 32 more; 20 by the four `*_is_shared_bytes` tests; 22 by `a_block_with_a_walk_issue_is_uneditable_for_every_partial_policy`, `a_walk_issue_in_an_ifd_with_no_target_still_refuses`; 23 by `exif_after_the_first_scan_is_uneditable`; 25 by `entries_check_compares_type_count_and_value_not_only_the_tag`; 26 by `a_scrub_that_leaves_a_byte_fails_check_scrubbed`; 29 by `a_plan_that_deletes_a_kept_exif_finding_fails_check_kept`; 30 by `all_removes_every_exif_block_including_the_extra_ones` and 20 more). Verifier mutants: V01 BE-only next word zeroed, caught (`gps_removes_..`, oracle tag tests); V02 keep set built from removed IFDs, caught; V03 `Entries` ignores values, caught (`entries_check_compares_..`); V04 survived (L1); V05 survived (L2); V06 hard-coded tag list instead of the audit, caught (31 tests); V07 16-segment limit off by one, caught (`seventeen_exif_segments_are_uneditable`); V08 survived (M1); V09 `walks_as_other_segments` always true, caught; V10 `HiddenPointer` removed, caught; V11 survived (L3); V12 kept-thumbnail protection removed, caught (harness on every seed); V13 rule-4 byte cross-check removed, caught; V14 `PayloadUnreadable` removed, caught; V15 survived (L5); V16 table patches not checked against kept values, caught (harness); V17 `Entries` check skipped, caught; V18 `InventoryTruncated` removed, caught; V19 keep set holds only the first kept entry per IFD, caught. **Real libFuzzer plant** (deleted-entry values not scrubbed, child IFDs still scrubbed), empty corpus plus committed seeds only: crash on a seed, guards `[Records, EntryPrediction]`, artifact hash `01a950c3`; reverted clean.

**Local corpus (`_incoming0`, read in place, counts and 8-hex hashes only, nothing written).** Reproduced exactly: 21 files, 13 JPEGs, 4 with Multi-Picture. `Gps` 13 Ok, 0 removed, 1,035 bytes patched, 22 entries; `GpsAndDevice` 13 Ok, 1,968 bytes, 24 entries; `AllPii` 12 Ok plus 1 `Uneditable(ChainAfterThumbnail)` on `a3b66f84`, 100,969 bytes, 154 entries; `All` 13 Ok, 313,995 removed; `Everything` 13 Ok. Scan identical, audit clean and values zero on every Ok; length equal 13/13 under `Gps`, `GpsAndDevice`, 12/12 under `AllPii`. ExifTool by pipe, one file at a time (39 output runs): `GPS` group gone in exactly the 4 files that had it; `IFD1` gone under `AllPii` in the 4 Ok files that had it, kept under the other two; `IFD0:Orientation` count unchanged in every run; `MPImage` groups 3 before and after in all 4 Multi-Picture files; 0 new warning kinds. Extra counts: 0 out-of-line values outside the blob, 0 IFD1 with a duplicated 0x0201, 0 IFD1 with both 0x0201 and 0x0111 (H1 and H2 do not touch the corpus).

**Oracle.** 15 public samples under `Gps`, `GpsAndDevice`, `AllPii`: `GPS` lost only where present; kept MakerNote groups byte-equal in ExifTool's output under `Gps` (Canon, Sony); `sony-d700` refuses `AllPii` (`ThumbnailNotLocated`). Rule 9 re-measured: an emptied Exif IFD (pointer kept) and an emptied IFD2 read with no warning and the group simply absent, in both byte orders; allow is right. The six red-proofs run a control first and pass. The three re-recorded SPEC-020 records changed only by dropping the `xmp`/`icc` sections of `gps-count-overflow` and `unreadable-exif` (now refused, deviation 7) and adding `iptc+free_text` to `kitchen-sink`.

**Questions.** `allpii-refuses-ifd1-with-a-following-ifd`: fails closed; decide before SPEC-022 (TIFF chains make it common). `writer-refusals-beyond-the-spec`: all four fail closed and are sound; ratify in a DEC-013 note before SPEC-022, and extend `PayloadUnreadable` per H1. `partial-policy-refuses-unreadable-exif-for-xmp-icc-iptc`: fails closed and follows the spec's definition; a usability regression only on files with broken Exif; no decision needed before SPEC-022. Joined stray bytes: the library refusal is correct and the harness excuse is exact for the segment shape (L4).

### Cost self-report

- **Tokens (total):** null: the verifier cannot see its own token count; the orchestrator transcribes the harness's `subagent_tokens`.
- **Estimated USD:** null (follows tokens).
- **Duration (minutes):** null (the harness reports it).
- **Source of the number:** harness report (to be transcribed).

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. Builder deviations 1 to 11 checked; deviation 3 (always-on overlap check) is untested (L3).
- **Follow-up work identified:** builder follow-up for H1, H2, M1, L1 to L3 (no new spec).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Rule 8 says scrub ranges inside the blob are "guaranteed by a clean walk"; the walk never checks ordinary payloads, which is H1. The spec's assumption was carried into the harness, so both sides shared it.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — None missing; DEC-013 B.3 ("gone from the file, not just unreferenced") is the rule H1 and H2 break, and it should be quoted in the leak-input list.

3. **If you did this task again, what would you do differently?**
   — Start from "which bytes does each deleted entry reach" (value, second pointer, strips) before reading the planner; that found H1 and H2 faster than the mutants did.
