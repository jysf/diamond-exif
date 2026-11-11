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
  id: HANDOFF-044
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
  tokens_total: 281942               # REAL combined count — what cost-audit reads
  estimated_usd: 1.86              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 19
  branch: worktree-agent-a0d62bd640ed8121c
  pr: null
  completed_at: 2026-11-10         # YYYY-MM-DD
  notes: "Cost transcribed by the orchestrator from the harness final subagent_tokens 281,942 at 6.60 USD per Mtok = 1.86 USD; duration 1,154,977 ms = 19.2 min rounded to 19. tokens_total, estimated_usd and duration_minutes left null on purpose: the verifier cannot see its own token count; the orchestrator transcribes the harness's metered subagent_tokens and duration. Verdict PUNCH LIST at e4062f3 (code 4911058): one Medium (the closed-world allow-lists are by tag only; a field of type 13 IFD makes an allow-listed IFD1 tag or any GPS tag an offset whose target stays after the IFD is removed), Lows (kept-side targets of kept offset tags unprotected; VerifyFailed Kept when a removed IFD1 holds a kept-category tag, seen on a real local file; mutant A5 survives; a pre-existing SPEC-020 rewrite_jpeg crash ebc677e5). No code changed, nothing pushed, no cycle advanced; this handback is left uncommitted in the worktree."
  synced_at: 2026-11-10
---

# HANDOFF-044: Re-verify JPEG in-place Exif entry deletion with scrub (SPEC-021, round 3)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-021` to `claude-opus-5-5` (verifier, a fresh cold agent with no memory of rounds 1 and 2) for a third **verify** cycle after a second punch list.

## Context the Receiving Agent Needs

- `HANDOFF-042-verify-*.md` and `HANDOFF-043-verify-*.md` still apply (gate list, privacy rule for `_incoming0`: read in place, copy nothing, write no derived file, pipes only, counts and 8-hex hashes only, never values or paths; host traps; do not fix code, push, open a PR or advance cycles). Read both, including their Handbacks (the findings and reproducers).
- **History.** Round 1 (7697162): H1 (straddling out-of-line value left unscrubbed), H2 (thumbnail scrub covered only the first locator pair), M1, L1-L5. Round 2 (c1c01bd): H3 (IFD1 reaches data through tags 0x0144/0x0145, 0x0207, 0x0120/0x0121, so the second fix missed siblings), M2 (rule-3 loop pinned only when the bad block is last), L6-L8. Every round found a new sibling of one class: **the planner acted as if it knew everything a removed or kept structure points to.**
- **Fixes under review:** `git diff c1c01bd..HEAD` (code `4911058`: closed-world allow-lists `THUMB_IFD_TAGS` (24 tags) and `GPS_LAST_TAG` in `src/write/exif.rs`; `kept_value_range` and `thumb_clip` keep-set protection; harness mirrors typed from the TIFF/Exif specs; 621 seeds). Also read the builder's "Sweep after the second verify (HANDOFF-043)" table in the HANDOFF-041 Handback and **challenge every row**: for each "safe" verdict try to break it.
- **Your main task is to attack the closed-world claim itself, not the listed cases.** Method: enumerate every tag the TIFF 6.0, Exif 2.32 and DNG/TIFF-EP tag lists give to IFD0, Exif IFD, GPS IFD, Interop and IFD1 that carry an offset, a pointer, a byte-count pair, or an in-place subordinate structure (including 0x014A SubIFDs, 0x8769, 0x8825, 0xA005, 0x02BC, 0x83BB, 0x8649, 0x8773, 0xC4A5, 0x927C, 0x9286, 0x9C9B-0x9C9F XP tags, 0xA302, 0xA300). For each, decide: can it appear in a structure the writer deletes or scrubs, and does anything in the file remain addressed by it after the strip? Try the leak in both byte orders through the public `strip`. Also check the **allow-list is truly offset-free** (read each listed tag: a tag in the list that actually carries an offset is a leak). Check the allow-list applies to IFD1 reached by every route (IFD0 next, SubIFD chain, second Exif block) and to GPS reached by every route.
- **Other siblings to try:** IFD0 entries that the policy deletes (Authorship, FreeText, Timestamps, CameraModel, DeviceId, Software kept) whose value overlaps a kept entry's value, another IFD's table, the TIFF header, or the thumbnail range; deleted entries with count 0 and byte_len 0; type with unusual size (RATIONAL, SRATIONAL, DOUBLE, SLONG8, IFD) and count*size overflowing 32 bits; ASCII with count 1 and 4; 0xFFFF entries on a 65535-entry IFD table that wraps the blob; zero-length Exif; an Exif block that is the MPF/APP2 segment; trailing `FF D9` handling; policies as byte masks over every 16-bit combination for 20 random fixtures with an independent byte-level check (write a differential test of your own in the scratchpad: build random IFD structures, run `strip` over all 2^k masks, and assert for each `Ok` that no canary byte of a removed category survives and no byte outside the recorded patches changed).
- **Re-run:** all gates in HANDOFF-042, `rewrite_jpeg` for 5 minutes, `tiff_ifd` 60 s; the mutants N03, N07, N10, V08, L8 keep-set revert, H3 allow-list off (IFD1 and GPS); at least eight mutants of your own aimed at the allow-lists, `kept_value_range`, `thumb_clip` and the harness mirrors; name the test or guard that catches each and list survivors with a concrete input.
- Judge the stray-bytes `entries` excuse again only if its code changed (it should not have).
- Local corpus counts expected: `Gps` 13 Ok, `GpsAndDevice` 13 Ok, `AllPii` 12 Ok + 1 `Uneditable(ChainAfterThumbnail)` on `a3b66f84`, `All` 13 Ok, `Everything` 13 Ok (21 files, 13 JPEGs, 4 with Multi-Picture); pipe a few outputs to `exiftool -` in memory only as before.
- Orchestrator's gate run at 4911058 (all exit 0; 2595 tests passed, 0 failed over three runs; five 60 s smokes clean) is evidence, not a substitute.

## Expected Deliverables

Exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA; findings by severity with concrete reproducers; mutants caught/survived (name the test); the closed-world audit table (tag, where it can appear, offset-carrying yes/no, handled how); corpus counts; judgment on each open builder question, including `makernote-internal-offsets` (is the gap acceptable to ship with a DEC note, or does it break DEC-013 B.3 as written?). Fill the HANDOFF-044 handback front matter (`tokens_total: null` with a note; the orchestrator transcribes the metered figure).

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

- **Branch / PR:** worktree branch `worktree-agent-a0d62bd640ed8121c`, fast-forwarded to e4062f3 (code 4911058). This handback is left uncommitted. No PR, nothing pushed.
- **Completed at:** 2026-11-10
- **All acceptance criteria met?** No. The closed-world claim does not hold: the allow-lists check the tag and ignore the field type (M3 below). DEC-013 B.3 ("every table or data range of a removed child IFD ... gone") fails for data that an entry of type IFD (13) points at.
- **For `verify`:** ⚠ PUNCH LIST at e4062f3 (code 4911058).

**Gates, re-run by the verifier (all exit 0):** `cargo fmt --all -- --check`, fuzz-graph fmt, `rustfmt --check --edition 2021 fuzz/shared/*.rs`, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `forbid-unsafe`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, release build of `exif-audit` on 1.90.0, and `just oracle-record`, after which `git status --porcelain` was empty. `cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`: stable (Homebrew) passed 865, failed 0, ignored 9; `+1.90.0` gave the same; no SKIP line. Fuzz from the committed seeds only: `tiff_ifd` ran 61 s, 1,783,912 runs, no crash. `rewrite_jpeg` CRASHED after about 110k runs (L12 below; the input's short hash is `ebc677e5`).

**Findings.**
- **M3 (Medium; a leak by the contract, not visible in ExifTool): the closed-world allow-lists check the tag only, so an entry of type 13 (IFD) carries an offset whatever its tag.** `THUMB_IFD_TAGS` and `GPS_LAST_TAG` accept any field type, and so does the harness mirror `IFD1_OK` (a copy of the same list). An allow-listed IFD1 tag, or any GPS tag up to 0x001F, typed IFD (13) with count 1 holds a 4-byte inline offset. The planner deletes the entry with its table and never zeroes the bytes it names. Reproducers, in both byte orders, from my scratch probes:
  - (a) IFD0 = {0x0112}, next = IFD1. IFD1 = {0x0100 type 13 count 1 → D; 0x0103 = 6; 0x0201 → a JPEG thumbnail; 0x0202 = 16}. At D, 16 bytes holding a canary. Under `Custom{Thumbnail}` the result is `Ok` with 4 entries and 2 scrubs, and the canary at D is still in the output.
  - (b) IFD0 = {0x0112, 0x8825 → G}. G = {0x0000 BYTE 4; 0x0001 type 13 → D}. Under `Gps` the result is `Ok` and the canary is left.
  - (c) The same class on a deleted entry: IFD0 = {0x0112, 0x013B type 13 count 1 → D}. Under `Custom{Authorship}` the result is `Ok` with 0 scrubs, and D is left.

  My differential (below) found (a) on its own: 28,052 `Ok` results with the canary left, all with entry type 13, over 600 random structures. ExifTool 13.55 does not follow type-13 entries on non-pointer tags (I checked tags 0x0100, 0x013B, 0xABCD in IFD0 and IFD1, and 0x0001 and 0x0099 in GPS, in both byte orders), and neither does this library's walk. So the bytes were invisible before the strip as well as after it, which is why this is not High. Even so, the claim "each listed tag's only offset is its own out-of-line value" is false whenever the type is 13. Fix: refuse (`ThumbnailNotLocated` in IFD1, `PayloadUnreadable` in GPS and for deleted entries) any removed-IFD entry or deleted entry whose type is 13 or 18 (IFD8). Better still, pin each allow-listed tag to its specified types. Mirror the change in the harness and add the inputs to the leak list and the seeds.
- **L9 (Low, kept-data integrity on malformed input): what a kept offset tag points at is not in the keep set.** `kept_value_range` protects a kept entry's own value. It does not protect the data that a kept 0x0111/0x0144/0x0120/0x0207 entry, a second 0x0201, or a kept MakerNote's internal offset names. Three reproducers, both byte orders:
  - P1: IFD0 = {0x0111 LONG → D, 0x0112, 0x0117 = 16, 0x013B ASCII 16 → D}. Under `Custom{Authorship}` and under `AllPii` the result is `Ok`, and the kept strip data at D is zeroed.
  - P2: under `Gps`, a kept IFD1 has 0x0144/0x0145 → D, and a GPS RATIONAL value is also at D. The result is `Ok` with the tile data zeroed.
  - P11: the same as P2 with a second IFD1 0x0201 → D. `Ok`, and the second thumbnail is zeroed.

  Nothing leaks, and DEC-013 B.4's must-stay list does not name these targets, but B.1 says "nothing else is touched". Either refuse `SharedBytes` when a scrub touches the target range of a kept offset tag, or record the limit in the DEC-013 note.
- **L10 (Low; seen on a real local file): a removed IFD1 that holds a tag of a kept category gives `VerifyFailed{Kept}`, not a typed refusal.** B.1 removes IFD1 whole. Self-check 7(b) then sees a kept-category finding disappear, and the B.5 exemption applies only when the whole segment goes. Reproducers: P14, an IFD1 holding 0x0131 Software under `AllPii`, gives `VerifyFailed{Kept}` in both byte orders. Local file `8a083030` under `Custom{Thumbnail}` (and `Custom{Thumbnail, Gps}`) also gives `VerifyFailed{Kept}`. Random structures produced 78,208 such results. It fails closed. Decide it with `writer-refusals-beyond-the-spec`: exempt findings inside a removed IFD1 from 7(b), or refuse with a typed reason before planning.
- **L11 (Low, test gap): my mutant A5 (`kept_value_range` treats `l <= 8` as inline) survives every test.** Input P13: a 58-byte blob, IFD0 = {0x0112; 0x011A RATIONAL 1 → 54, a kept value that runs past the end; 0x013B ASCII 6 → 52}, policy `Custom{Authorship}`. The real code returns `Uneditable{SharedBytes, 66}`. The mutant returns `Ok`, and the kept in-blob bytes `XRES` are zeroed (confirmed by building my probe against the mutant). Add a kept straddler of 5 to 8 bytes, the kept-side twin of L6.
- **L12 (Low, existing since SPEC-020, blocks the 24 h fuzz gate): `rewrite_jpeg` crash `ebc677e5` (3,108 bytes, policy byte 0x07 = `All`), guard `VerifyAllowed`.** The library returns `VerifyFailed{MpfFields}`; the harness predicts `Ok` with one MPF patch at output offset 66. There are no stray bytes (segments: JFIF, Exif 128 bytes, MPF 328 bytes, SOS), so `walks_as_other_segments` does not apply. Cause: the index's entry 0 has offset 34, so its "image" range lies on the MP entry table itself. The primary-size patch therefore lands inside a located embedded image's bytes, and check (f) sees that image change. This fails closed, and the MPF code is unchanged since 751140c. Fix: a typed refusal when an MPF field patch falls inside a kept embedded-image range, or model it in the harness. Add the crash as a seed.
- **Note (scope, not this spec):** a JPEG preview in IFD2 or a SubIFD (`0x0201` outside IFD1) is not a `Thumbnail` finding under DEC-008, so `AllPii` keeps it. Local file `a3b66f84` has an IFD2 preview. It refuses `AllPii` today only because of `ChainAfterThumbnail`.
- **Note:** a deleted entry of the Exif 3.0 type 129 (UTF-8) is refused as `PayloadUnreadable` (unknown size). That is fail-closed, but it will be a false refusal on Exif 3.0 files.

**Closed-world audit table** (L = library, H = harness; "removed" = GPS IFD or IFD1).

| Tag / type | Where it can appear | Carries an offset? | How it is handled |
|---|---|---|---|
| 0x014A SubIFDs | any IFD (walk follows from every kind) | yes (IFD array) | child walked; under a removed IFD it is `NestedIfd`, and it is off-list (IFD1) or above 0x1F (GPS); a kept child table is in `kept_tables` |
| 0x8769 ExifIFD | IFD0/1/Chain/Sub follow it; others do not | yes | a second entry, or one in another kind, is `HiddenPointer`; in IFD1 it is `NestedIfd` and off-list; never removed |
| 0x8825 GPS | same | yes | pointer deleted from its parent; a second one is `HiddenPointer`; a cycle is not clean; all GPS kinds are removed (by IFD0, IFD1, Chain or Sub) |
| 0xA005 Interop | Exif only | yes | elsewhere `HiddenPointer`; its table is kept |
| 0x0111/0x0117 strips | IFD0, IFD1, Chain, Sub | yes | IFD1 removed: off-list, refused. **Kept: target not protected (L9)** |
| 0x0144/0x0145 tiles, 0x0120/0x0121 free, 0x0207/0x0208/0x0209 JPEG tables | same | yes | same as strips |
| 0x0201/0x0202 | IFD1 (Thumbnail category); elsewhere uncategorised | yes | IFD1 removed: exactly one pair, scrubbed. Kept: audit range plus `thumb_clip` (first pair only; **second pair not protected, L9**). Outside IFD1: not a target (Note) |
| 0x927C MakerNote | Exif IFD | its value is the data; **internal offsets can point outside** | deleted: declared range scrubbed; P3 shows external data left (open question) |
| 0x02BC XMP, 0x83BB IPTC, 0x8649 Photoshop, 0x8773 ICC | IFD0 or Exif | no (the value is the data) | deleted and scrubbed under Xmp/Iptc/Icc; in IFD1 off-list, refused (false refusal, safe) |
| 0x9286 UserComment, 0x9C9B-0x9C9F XP*, 0xA302 CFAPattern, 0xA300 FileSource, 0xC4A5 PrintIM | Exif or IFD0 | no | values; 0xC4A5 in IFD1 is off-list and refused |
| the 24 `THUMB_IFD_TAGS` | IFD1 | no, **unless typed 13 (M3)** | correct by tag (I read each: all are values or the locator pair); type not checked |
| GPS 0x0000-0x001F | GPS | no, **unless typed 13 (M3)** | correct by tag; type not checked |
| GPS > 0x001F | GPS | unknown | `PayloadUnreadable` |
| type 13 IFD / 18 IFD8 on a non-pointer tag | anywhere | yes, by type | **not handled (M3)**; ExifTool 13.55 and the walk do not follow it |
| types 16/17 (8-byte), 129 UTF-8, other unknown | anywhere | size unknown | deleted: `PayloadUnreadable`; kept: protected from the offset to the blob end |
| kept value straddling the blob end | any kept IFD | n/a | clipped in-blob part kept (`kept_value_range`, `thumb_clip`); the 5-8 byte case is not pinned (L11) |

How IFD1 and GPS are reached: IFD1 is only ever IFD0's next. An IFD at the end of a SubIFD chain is `Sub`, and a later IFD is `Chain`; neither is a Thumbnail target. Each Exif block is planned on its own, so the allow-list applies to the IFD1 of every block. GPS is any IFD of kind Gps, and the list applies to all of them. Every route (IFD0 next, a SubIFD chain, a second Exif block) was tried in both byte orders. Apart from M3, the allow-list holds on all of them.

**Builder's sweep table, row by row.** Rows 1-6 and 9-17 hold under attack; tried in both byte orders, every one was refused or correct. Rows 7 and 8 (the IFD1 and GPS allow-lists) are broken by M3. Row 11 (kept thumbnail) covers the first pair only (L9). Row 18 (MakerNote) is not closed, as the builder says. Two rows are missing from the table: the targets of kept offset tags (L9), and a removed IFD1 that holds kept-category tags (L10).

**My differential** (a separate crate in my scratch dir, path dependency on the worktree, public `strip` only; every model and canary typed independently). It builds random TIFFs in a JPEG, one or two Exif blocks, LE or BE. They hold IFD0 tags from every category, kept strips, an Exif IFD with a MakerNote, Interop, GPS (sometimes an off-list tag pointing at data), IFD1 (sometimes extra allow-listed values, an Artist, an off-list offset tag, or a type-13 entry), and in 20% of fixtures one value retargeted into another blob. Each fixture runs under all 4,096 subsets of the 12 Exif-relevant categories. For each `Ok` it asserts: no canary that some removed reference names survives; no canary that only kept references name is lost; and the output length is unchanged with no byte changed outside `patches()` (on partial outputs). Results:
- 600 fixtures (seeds 1-300 and 1001-1300), 2,457,600 strips, 1,849,934 `Ok`. Violations: 28,052, all M3 (type 13). Bytes outside patches: none. Length changes: none. Kept canaries lost: none, because the random overlaps did not hit a kept offset target; the targeted probes P1/P2/P11 show L9.
- With the type-13 shape left out (seeds 1-300): 908,158 `Ok`, 0 violations.

**Mutants** (scratch copy; exact text replacements; `cargo test --lib --test strip --test fuzz_layout --test strip_oracle`; the copy was diffed clean against the worktree afterwards).

| Mutant | Result |
|---|---|
| N03 | caught by `l6_straddling_values_of_five_and_eight_bytes_are_refused_and_an_exact_fit_is_not` |
| N07 | caught by `h2_a_thumbnail_ifd1_with_other_locators_is_thumbnail_not_located`, `leak_inputs_refuse_with_the_named_reason_and_offset`, and the seed sweep `rewrite_jpeg_harness_holds_on_every_seed_including_the_entry_seeds` |
| N10 | caught by `an_unclean_block_anywhere_refuses_first_middle_or_last` and the seed sweep |
| V08 | caught by the same two plus `a_later_block_after_the_first_scan_is_uneditable` and `a_later_block_with_a_walk_issue_still_refuses_under_every_partial_policy` |
| L8 keep-set revert | caught by `l8_a_kept_entry_that_straddles_the_blob_end_protects_its_in_blob_bytes` and `leak_inputs_refuse_...` |
| H3 IFD1 allow-list off | caught by `h3_an_off_list_tag_in_a_removed_ifd1_is_thumbnail_not_located`, `leak_inputs_refuse_...`, `no_leak_input_is_ok_with_a_canary_left_under_any_policy`, and the seed sweep |
| H3 GPS allow-list off | caught by `h3_an_off_list_tag_in_a_removed_gps_ifd_is_refused` and the same three |

My own mutants:

| Mutant | Result |
|---|---|
| A1: 0x0144 (or A1b: 0x0207) added to the list | caught by the four tests above |
| A2: `GPS_LAST_TAG = 0xFF` | caught by `h3_an_off_list_tag_in_a_removed_gps_ifd_is_refused` |
| A3: allow-list checked on the first removed IFD only | caught by `no_leak_input_...` and the seed sweep |
| A4: kept value of unknown type unprotected | caught by `l8_...` |
| A5: kept `l <= 8` | **SURVIVED (L11)** |
| A6: `thumb_clip` dropped | caught by `a_kept_thumbnail_that_straddles_the_blob_end_protects_its_in_blob_bytes` |
| A7: `thumb_clip` without the clip | caught by the same test |
| A8a: harness IFD1 mirror off | survived, by design: the harness accepts any library refusal |
| A8b: library and harness IFD1 lists both off | caught by the unit, leak and no-leak tests, but NOT by the seed sweep, because a shared blind spot passes the harness |
| A8c: the same for GPS | caught by the unit, leak and no-leak tests |
| A9: harness `kept_bytes` reverted | survived, by design |
| A9b: library and harness keep set reverted | caught by `l8_...` and `leak_inputs_...` |
| A10: table patch vs kept values check dropped | caught only by the seed sweep |
| A11: kept audit thumbnail range dropped | survived; equivalent, since the audit and `thumb_clip` both take the first 0x0201/0x0202 |
| A12: `thumb_clip` accepts any count | survived; a weak mutant (a kept IFD1 with 0x0201 count 2 has no located thumbnail anyway) |
| A13: GPS `>=` | caught by `h3_..._gps_ifd_is_refused` |

Note: the harness mirror `IFD1_OK` was copied from the library list, not typed independently, so it shares M3.

**Local corpus (`_incoming0`; read in place; pipes only; nothing written; counts and short hashes only).** 21 files, 13 JPEGs, 4 with Multi-Picture.

| Policy | Result | Bytes removed | Bytes patched | Entries | Checks |
|---|---|---|---|---|---|
| `Gps` | 13 Ok | 0 | 1,035 | 22 | scan identical, audit clean, same length, values zero: 13/13 |
| `GpsAndDevice` | 13 Ok | 0 | 1,968 | 24 | the same, 13/13 |
| `AllPii` | 12 Ok + 1 `Uneditable(ChainAfterThumbnail)` on `a3b66f84` | 0 | 100,969 | 154 | the same, 12/12 |
| `All` | 13 Ok | 313,995 | | | |
| `Everything` | 13 Ok | 3,352,855 | | | |

All of these match the expected counts.

ExifTool by pipe (`-a -u -G1 -H -s -`), 52 runs (13 files, each input plus three policies):
- `GPS` group gone in exactly the 4 files that had it.
- `IFD1` gone under `AllPii` in the 4 Ok files that had it, and kept under the other two policies.
- Orientation count unchanged, except `8a083030`, where IFD1 had its own Orientation (2 → 1, as in round 2).
- `MPImage` groups 3 before and after in all 4 Multi-Picture files.
- 0 new warnings (`a3b66f84` has one warning in its input, unchanged).
- 0 real IFD1 tags off the allow-list. 0 offset tags in a MakerNote group; the one offset tag outside the standard groups is an IFD2 preview in `a3b66f84`.

Custom policies on the corpus:
- `Custom{Thumbnail}`: 11 Ok, 1 `ChainAfterThumbnail` (`a3b66f84`), 1 `VerifyFailed{Kept}` (`8a083030`, L10).
- `Custom{MakerNote}`: 13 Ok.
- `Custom{Software}`: 13 Ok.

**Stray-bytes `entries` excuse:** `walks_as_other_segments` is unchanged since c1c01bd, so I did not re-judge it. The L12 crash is a different shape and is not excused.

**Questions.**
- `makernote-internal-offsets`: acceptable to ship with a DEC-013 note. It does not break B.3 as written, because B.1 defines a MakerNote as "an ordinary entry whose value is scrubbed whole", and B.3 scrubs "every out-of-line value of a deleted entry". The data a note's internal offsets name is not that value in TIFF terms. P3 shows it is a real gap: the canary at the external target survives `Custom{MakerNote}`. The note should extend part C to removed MakerNotes, in the builder's suggested wording. 0 of 13 local files show it.
- `writer-refusals-beyond-the-spec`: all of them fail closed. Ratify them. Also add the M3 type rule and decide L10 (exempt from 7(b), or refuse typed) before SPEC-022, where IFD chains and typed IFD entries are common.
- `allpii-refuses-ifd1-with-a-following-ifd`: fails closed. Decide before SPEC-022, and note that the one local case is an IFD2 preview that `AllPii` would otherwise keep (see the Note above).
- `partial-policy-refuses-unreadable-exif-for-xmp-icc-iptc`: fails closed and follows the definition. No decision needed before SPEC-022.

### Cost self-report

- **Tokens (total):** null. The verifier cannot see its own token count; the orchestrator transcribes the harness's metered `subagent_tokens`.
- **Estimated USD:** null (follows the tokens).
- **Duration (minutes):** null (the harness reports it).
- **Source of the number:** harness report, to be transcribed.

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. The handback is left uncommitted in the worktree (earlier rounds committed theirs).
- **Follow-up work identified:**
  - Builder follow-up for M3 and L9 to L12 (no new spec).
  - A DEC-013 note for MakerNote internal offsets, the targets of kept offset tags, and the type-13 rule.
  - DEC-008 scope question on previews outside IFD1.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — "Closed world" was stated over tags, but an offset can come from a field's type as well as its tag. The brief's own enumeration was by tag too, so I only found M3 because the random generator varied the type.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-013 B.4's must-stay list ("a kept entry's value, a kept IFD table, ...") says nothing about the data a kept offset tag points at (L9). B.1 removes IFD1 whole, while B.5/7(b) forbid losing kept findings; the two conflict when IFD1 holds a categorised tag (L10).

3. **If you did this task again, what would you do differently?**
   — Write the random differential first, with every field (tag, type, count, overlap) varied. It found M3 within seconds, while the tag-by-tag table did not.
