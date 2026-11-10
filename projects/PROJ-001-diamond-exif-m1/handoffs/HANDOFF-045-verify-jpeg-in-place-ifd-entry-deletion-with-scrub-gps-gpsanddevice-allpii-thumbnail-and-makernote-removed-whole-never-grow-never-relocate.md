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
  id: HANDOFF-045
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
  branch: worktree-agent-a24756c36e015cd68
  pr: null
  completed_at: 2026-11-10         # YYYY-MM-DD
  notes: "tokens_total, estimated_usd and duration_minutes left null on purpose: the verifier cannot see its own token count; the orchestrator transcribes the harness's metered subagent_tokens and duration. Verdict PUNCH LIST at dbf3567 (code c1958b2): one High (IFD pointer tags that ExifTool follows but the walk and the HiddenPointer guard do not know, 0x0190 GlobalParametersIFD first; a GPS IFD behind it survives Gps and AllPii with ExifTool still printing GPS:GPSLatitude), one Medium (category tags inside the Interop IFD; ExifTool prints InteropIFD:Artist after an Authorship strip; DEC-008 scope), Lows (target of a removed GPS IFD's next-word left; kept-target protection by tag only; five surviving mutants). No code changed, nothing pushed, no cycle advanced."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-045: Re-verify JPEG in-place Exif entry deletion with scrub (SPEC-021, round 4)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-021` to `claude-opus-5-5` (verifier, a fresh cold agent with no memory of earlier rounds) for a fourth **verify** cycle after a third punch list.

## Context the Receiving Agent Needs

- `HANDOFF-042`, `HANDOFF-043` and `HANDOFF-044` verify files still apply (gate list, privacy rule for `_incoming0`: read in place, copy nothing, write no derived file, pipes only, counts and 8-hex hashes only, never values or paths; host traps; do not fix code, push, open a PR or advance cycles). Read all three, including their Handbacks.
- **History.** R1 (7697162): H1 straddling value, H2 thumbnail pair, M1, L1-L5. R2 (c1c01bd): H3 off-list offset tags in IFD1, M2, L6-L8. R3 (e4062f3): M3 (allow-lists ignored the field type; type 13/18 entries carry offsets whatever the tag), L9 (targets of kept offset tags unprotected), L10 (removed IFD1 holding a kept-category tag gave `VerifyFailed{Kept}`), L11, L12 (MPF primary-size patch landing on the MP index's own table). Each round found a new sibling of one class: **the planner acting as if it knew everything a removed or kept structure points to.**
- **Fixes under review:** `git diff e4062f3..HEAD` (code `c1958b2`: field type must be 1..=12 for deleted and removed-IFD entries else `PayloadUnreadable`, type 129 not accepted; kept offset-tag targets protected (all 0x0201/0x0202 pairs, 0x0111/0x0117, 0x0144/0x0145, 0x0120/0x0121, 0x0207-0x0209), unknown length means offset to blob end; typed `ThumbnailNotLocated` for a removed IFD1 holding a kept-category tag; `mpf_patches` refuses `MpfNotIndexed` when a patch field overlaps the index header, its IFD0 table or a located image; harness early-exit mirror; 713 seeds).
- **The round-3 verifier's differential (scratchpad `v44/diff`, copy `v44/diffmine` pointed at this tree) now reports zero violations for the orchestrator (81,920 strips, 63,288 Ok).** Do not rely on it: write your own independent differential with different random structure (different seed, include types 1-12, 13, 18, 129 and unknown, entry counts up to 0xFFFF, value offsets in the header/other tables/past the blob, two Exif blocks, MPF present with Exif removed, 17 segments, nested pointers, and every policy mask over all 12 Exif categories plus the `Xmp/Iptc/Icc/Comment` bits) and assert for every `Ok`: no canary byte of a removed category survives anywhere in the output, no kept-category canary is lost, output length is unchanged for partial policies, no byte differs outside `patches()`/`scrubs()`, and the audit of the output reports no finding in a removed category. For every `Err` assert the typed reason is one of the documented ones and no bytes escape.
- **Your main task is to find the next sibling**, by a different route than the earlier verifiers: think in terms of data that remains *reachable by some reader* after a strip, not tags. Candidates: values that a kept entry's TYPE implies (type 13/18 kept, handled?), pointer-valued entries of other IFD kinds (0x8769 duplicates, SubIFD arrays with count > 1 pointing into removed IFDs, Interop inside GPS), IFD offsets that point into the middle of another structure, thumbnail range partially overlapping the Exif segment end or the next marker, a thumbnail inside a kept MakerNote, big-endian vs little-endian values that read differently under each byte order (a TIFF header that lies about its byte order versus the MakerNote's own), a second TIFF header embedded in a removed value, and the Exif/XMP/IPTC/ICC cross-listing (Exif tags 0x02BC/0x83BB/0x8773). Also re-read `src/write/jpeg.rs` MPF code after the L12 change: can the new overlap refusal be bypassed, or does it refuse files that real cameras produce (check with the local corpus: the 4 Multi-Picture files must still be Ok under `All`).
- **Re-run:** all gates in HANDOFF-042, `rewrite_jpeg` for 5 minutes, `tiff_ifd` 60 s; mutants of your own aimed at the type rule, the kept-target protection, `ThumbnailNotLocated` for kept-category tags, the MPF overlap refusal and the harness mirrors (at least eight; name the test or guard that catches each; list survivors with a concrete input).
- **Local corpus** counts expected: `Gps` 13 Ok, `GpsAndDevice` 13 Ok, `AllPii` 12 Ok + 1 `Uneditable(ChainAfterThumbnail)` on `a3b66f84`, `All` 13 Ok, `Everything` 13 Ok (21 files, 13 JPEGs, 4 with Multi-Picture); `Custom{Thumbnail}` 11 Ok + 1 `ChainAfterThumbnail` + 1 `ThumbnailNotLocated` (`8a083030`); pipe a few outputs to `exiftool -` in memory only as before.
- Orchestrator's gate run at `c1958b2` (all 26 steps exit 0; 2610 tests passed, 0 failed over three runs; five 60 s smokes clean) is evidence, not a substitute.

## Expected Deliverables

Exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA; findings by severity with concrete reproducers; mutants caught/survived; a statement of what your differential covered and did NOT cover (so the orchestrator can judge residual risk); corpus counts; judgment on each open builder question in `guidance/questions.yaml` (`makernote-internal-offsets`, `preview-outside-ifd1-not-thumbnail`, `field-type-129-utf8`, `writer-refusals-beyond-the-spec`, `allpii-refuses-ifd1-with-a-following-ifd`): safe to ship with a DEC note, or must be decided/changed first. If you find nothing above Low, say APPROVED and list the Lows. Use your own scratch directory `scratchpad/v45/` and do not overwrite files outside it. Fill the HANDOFF-045 handback front matter (`tokens_total: null` with a note; the orchestrator transcribes the metered figure) and COMMIT it on your branch.

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

- **Branch / PR:** worktree branch `worktree-agent-a24756c36e015cd68`, fast-forwarded to dbf3567 (code c1958b2). Only this handback is committed on top. No PR, nothing pushed.
- **Completed at:** 2026-11-10
- **All acceptance criteria met?** No. ExifTool agreement at tag level fails on a crafted input: a GPS IFD reached through an IFD pointer tag that the walk does not follow is still printed by ExifTool as `GPS:GPSLatitude` after an `Ok` `Gps` strip (H4).
- **For `verify`:** ⚠ PUNCH LIST at dbf3567 (code c1958b2).

**Gates (re-run by the verifier; all exit 0).** fmt (library, fuzz graph, `rustfmt --check --edition 2021 fuzz/shared/*.rs`), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `forbid-unsafe`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, release build of `exif-audit` on 1.90.0, then `just oracle-record`, after which `git status --porcelain` was empty. `cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1` gave 870 passed, 0 failed, 9 ignored on both stable (Homebrew 1.99.1) and `+1.90.0`, with no SKIP line. Fuzz from the committed seeds, no crash in any run: `rewrite_jpeg` 301 s (292,577 runs), `tiff_ifd` 61 s (1,006,503), and 60 s each of `jpeg_segments` (105,164), `audit` (60,913), `audit_render` (25,766) and `cli_args` (1,754,686). `decisions/`, `scripts/` and every manifest and lock file are unchanged since e4062f3. The library diff adds no `unwrap`, `expect`, `allow` or `unsafe`.

**Findings.**
- **H4 (High; fails open, and ExifTool can see the result; rare in practice): ExifTool follows IFD pointer tags that the walk does not, and the `HiddenPointer` guard knows only the four DEC-005 tags.** I scanned every tag 0x0000 to 0xFFFF as a LONG pointer in IFD0, the Exif IFD and IFD1 against ExifTool 13.55. Besides 0x014A, 0x8769, 0x8825 and 0xA005, ExifTool opens four more as IFDs: 0x0190 GlobalParametersIFD (read with the Exif main table, so it includes `GPSInfo`), 0x888A LeafSubIFD, 0x8290 KodakIFD and 0xFE00 KDC_IFD. From inside a GPS IFD it opens none. The walk follows none of the four, the audit reports nothing under them, and `plan_block` refuses only `POINTER_TAGS`. Reproducers, in both byte orders:
  - (a) IFD0 = {0x0112; 0x0190 LONG → 40}; IFD at 40 = {0x8825 → 60}; GPS IFD at 60 = {0x0001 ASCII "N"; 0x0002 RATIONAL 3 → 120}. ExifTool prints `GPS:GPSLatitudeRef` and `GPS:GPSLatitude` for the input. `strip(.., Gps)` and `strip(.., AllPii)` both return `Ok` with 0 entries and 0 scrubs, and ExifTool prints the same GPS tags from the output. `All` is correct (the whole segment goes).
  - (b) IFD0 = {0x0112; 0x0190 → IFD{0x013B ASCII 16 → canary}}. After `Custom{Authorship}` and after `AllPii` the result is `Ok`, and ExifTool still prints `GlobParamIFD:Artist`. The same holds through 0x8290 and 0xFE00, printed as `Kodak_IFD_0x013b` and `Kodak_KDC_IFD_0x013b`, and through 0x888A (`LeafSubIFD`). A type-13 0x0190 behaves the same way.

  This is the round-4 sibling: the writer acted as if it knew every pointer that a kept structure holds. It is a guard gap, because the audit misses the data in the input too (read-parity since SPEC-019). Fix: add 0x0190, 0x888A, 0x8290 and 0xFE00, in any IFD kind and of any type, to the `HiddenPointer` refusal for partial policies, and mirror it in the harness. Better still, record the ExifTool subdirectory set in a DEC-005 note, so the audit can name the gap. Add (a) and (b) to the leak inputs and the seeds. Local corpus: 0 of 550 entries use these tags. I graded this High rather than Medium because the leaked tag is GPS under its standard group name and is visible to the spec's own oracle after an `Ok`.
- **M4 (Medium; DEC-008 scope, same "reachable by a reader" class): category tags inside the Interop IFD.** IFD0 {0x0112, 0x8769 → Exif{0xA005 → Interop{0x0001 "R98", 0x013B ASCII 16 → canary}}}. ExifTool prints `InteropIFD:Artist`. `categorize(Interop, _)` is `None` by DEC-008, so `Custom{Authorship}` and `AllPii` return `Ok` with the Artist intact and still printed by ExifTool. This is pinned behaviour, not a regression. Decide it in DEC-008 (categorise by id in Interop as in the other IFD kinds), or have the writer refuse a partial strip when Interop holds an id in a removed category. Corpus: 0 such tags.
- **L13 (Low): the target of a removed GPS IFD's next-word stays.** GPS IFD at 40 with next = 70; at 70 an IFD {0x0002 RATIONAL 3 → canary}. `Gps` and `AllPii` return `Ok` and scrub the GPS table (its next-word too), but the IFD at 70 and its value remain, now unreferenced. Neither ExifTool nor the walk reads that next-word (DEC-005), so this differs from IFD1, whose next-word is followed and therefore refused as `ChainAfterThumbnail`. Corpus: 0 of 4 GPS IFDs, and 0 Exif or Interop IFDs, have a non-zero next-word, so a refusal would cost nothing on real files. Either refuse it, or state in the DEC-013 note that B.3 does not cover a leaf IFD's next-word.
- **L14 (Low, kept-side integrity, sibling of L9): kept-target protection is by tag (`OFFSET_TAGS`).** IFD0 {0x0112; 0x013B ASCII 16 → 120; 0xC634 LONG 1 → 120 (DNGPrivateData-style)}. `Custom{Authorship}` returns `Ok`, and the bytes the kept 0xC634 names are zeroed. The same applies to a kept 0x0190, 0x8290, 0xFE00 or 0x888A that points at a deleted value. Nothing leaks. Record it with L9 in the DEC-013 note.
- **L15 (Low, test gaps; mutants that survive):**
  - MB: type 129 accepted. Equivalent today, because the unknown-size refusal backstops it.
  - ML: MPF located-image ranges not forbidden. Reproducer: an MPF whose secondary entry (size 42, offset 50) names an `FF D8 .. FF D9` range at header+50 to 92 that contains the MP entry array, with the primary size field at header+62, under `All`. The real code returns `MpfNotIndexed`; the mutant returns `VerifyFailed{MpfFields}`.
  - MN: MPF header not forbidden. The shape is an MPEntry value at header offset 0, so the size field lands on the IFD0-offset word. I did not build it.
  - MS: GPS pointer entries no longer exempt from the type rule. Reproducer: IFD0 {0x0112, 0x8825 type 13 → GPS{0x0002 RATIONAL 3}} under `Gps`. The real code returns `Ok` with the canary gone; the mutant returns `PayloadUnreadable`, a false refusal.
  - MT: a kept type-13 entry protects only 4 bytes. The shape is a kept 0xABCD type 13 → D with a deleted value at D+8 (the real code returns `SharedBytes`). I did not build it.

  All five fail closed or are integrity-only, so this is test pinning only.
- **Notes.**
  - A thumbnail whose 0x0202 understates the JPEG keeps the bytes past the declared length (`P4`: `Ok`, tail canary left). This is by design, since no reader references those bytes. Corpus: all 5 IFD1 thumbnails end in `FF D9` exactly at the declared length.
  - With two MPF APP2 segments, the second index is not patched.
  - An MP attribute IFD, or an ImageUIDList, that overlaps the MP entry array is patched with `Ok`.

  All three are integrity-only on crafted input.

**My differential** (scratch crate `v45/diff`, a path dependency on this worktree, public `strip`/`audit` only). Its generator, category table and byte model are my own, and it shares no code with v44 or the harness. It builds 1 or 2 Exif blocks, LE or BE, with random IFD0/Exif/GPS/Interop/IFD1/Chain/Sub structures. Extras include hidden GPS pointers in Exif, GPS from IFD1 and Sub, IFD1 extras (Software, Artist, Make, DateTime, strips, tiles, 0x0207, a second 0x0201, a GPS pointer), and type-13 extras. Types are mutated to 1-12, 13, 18, 129, 99, 16, 17 and 0, counts go up to 0xFFFF, and IFD entry counts are set to 0xFFFF. Retargets point into the header, other blocks, other tables, blob end - 2, past the end and 0xFFFFFFF0. The JPEG wrapper adds XMP, IPTC, ICC and COM canaries, an MPF index with an indexed secondary, and 17 Exif segments.

Each fixture runs under all 4,096 masks of the 12 categories. For every `Ok` it checks:
- no removed-reference canary byte is non-zero;
- no kept-only canary byte changes;
- removed IFD tables are zero;
- no byte outside `patches()` changes, after mapping through `removals()`;
- the length is the input minus removals;
- the output audit has no finding in a removed category;
- kept-category finding counts are equal;
- segment canaries are present exactly when kept;
- the MPF index still points at the secondary;
- under whole-Exif policies, no Exif token survives.

Results: 8,000 fixtures (seeds 1-2000 and 100000-105999), 32,768,000 strips, 10,505,135 `Ok`, **0 violations**. Errors were all typed `Uneditable` (`SharedBytes`, `PayloadUnreadable`, `ThumbnailNotLocated`, `BlockNotClean`, `HiddenPointer`, `ChainAfterThumbnail`, `NestedIfd`, `TooManyExifSegments`), with no `VerifyFailed`. The checker has teeth: against mutant MA (type 13 accepted) it reports 4,642 canary leaks.

**What it did NOT cover:**
- pointer tags outside the four DEC-005 tags, and category tags in Interop as an assertion (H4 and M4 came from targeted probes and the ExifTool scan, not from the generator);
- leaf next-words (L13);
- MakerNote-internal offsets;
- hostile MPF indexes (its MPF is always well-formed, so `MpfNotIndexed` never fires; the MPF overlap code was probed by hand);
- Exif after the scan, and APP1 segments over 64 KB;
- thumbnails in IFD2 or SubIFDs as a category;
- type 18 targets as a modelled reference.

**Mutants** (scratch copy `v45/mut` = `git archive c1958b2`; exact text replacements; `cargo test --lib --test strip --test fuzz_layout --test strip_oracle`, plus my differential on 400 fixtures; restored and diffed clean). Caught:
- MA type 13 accepted, MC type rule off for deleted entries, MD type rule off for removed IFDs: `m3_an_entry_of_an_offset_type_is_refused_when_deleted_or_removed`, `leak_inputs_refuse_with_the_named_reason_and_offset`, `no_leak_input_is_ok_with_a_canary_left_under_any_policy`, the seed sweep, and the differential (V1).
- ME kept type-13 unprotected: `m3_the_target_of_a_kept_offset_typed_entry_is_protected`.
- MF offset targets not kept: `l9_the_targets_of_kept_offset_tags_are_protected`, `leak_inputs_...`, the seed sweep.
- MG no tiles, MH first value only, MI first entry only: `l9_...`.
- MJ kept IFD1 tags ignored: `l10_a_removed_ifd1_with_a_kept_category_tag_is_thumbnail_not_located`, `leak_inputs_...`, the seed sweep.
- MK kept IFD1 tags not collected: `leak_inputs_...`, the seed sweep.
- MM MPF table not forbidden, MO MPF check off: `a_removal_that_joins_stray_bytes_into_a_marker_is_refused_and_excused_exactly`, the seed sweep.
- MP harness early exit removed: the same two.
- MQ library and harness type rule off: `m3_...`, `leak_inputs_...`, `no_leak_input_...`, the differential.
- MR library and harness kept targets off: `l9_...`, `leak_inputs_...`.
- MU harness MPF forbidden off, MV library and harness MPF off: the stray-bytes test and the seed sweep.
- MW library and harness kept-IFD1 rule off: `l10_...`, `leak_inputs_...`.

Survived: MB, ML, MN, MS, MT (L15).

**Local corpus (`_incoming0`; read in place, pipes only, nothing written; counts and short hashes only).** 21 files, 13 JPEGs, 4 with Multi-Picture.

| Policy | Result | Bytes removed | Bytes patched | Entries |
|---|---|---|---|---|
| `Gps` | 13 Ok | 0 | 1,035 | 22 |
| `GpsAndDevice` | 13 Ok | 0 | 1,968 | 24 |
| `AllPii` | 12 Ok + 1 `Uneditable(ChainAfterThumbnail)` on `a3b66f84` | 0 | 100,969 | 154 |
| `All` | 13 Ok, including all 4 Multi-Picture files (the MPF overlap refusal does not fire on real files) | 313,995 | 16 | |
| `Everything` | 13 Ok | 3,352,855 | | |

Under the three partial policies every `Ok` had an identical scan, a clean audit, the same length and zero values (13/13, 13/13, 12/12). `Custom{Thumbnail}` gave 11 Ok, 1 `ChainAfterThumbnail` (`a3b66f84`) and 1 `ThumbnailNotLocated` (`8a083030`), all as expected.

Over 124 sampled masks × 13 files: 1,530 Ok, 55 `ChainAfterThumbnail`, 27 `ThumbnailNotLocated`, 0 `VerifyFailed`, 0 `Ok` with a removed-category finding left, and 0 length mismatches. Structure counts:
- 550 entries;
- 0 entries of a type outside 1-12;
- 0 entries with the H4 pointer tags;
- 0 category tags in Interop;
- 0 non-zero next-words in a GPS, Exif or Interop IFD.

ExifTool by pipe on the 4 Multi-Picture files under `none`, `gps`, `all_pii` and `all`: the `GPS` group goes under every policy; `IFD1` is kept under `gps` and gone under `all_pii`; `MPImage` groups stay at 3 everywhere; the Orientation count is unchanged under the partial policies; 0 warnings.

**Questions.**
- `makernote-internal-offsets`: safe to ship with the DEC-013 note as proposed. It is the same class as L13 and L14 (data named by a structure the planner does not parse); list all three in the one note.
- `preview-outside-ifd1-not-thumbnail`: safe to ship with a DEC-008 note. Decide it together with M4, since both are audit-scope gaps that the writer inherits.
- `field-type-129-utf8`: safe to ship as refused, with a DEC note; pin it with a test (MB survives).
- `writer-refusals-beyond-the-spec`: ratify them, but H4 must change first. `HiddenPointer` needs the ExifTool subdirectory tags.
- `allpii-refuses-ifd1-with-a-following-ifd`: fails closed. Safe to ship with a DEC note; decide before SPEC-022.

### Cost self-report

- **Tokens (total):** null. The verifier cannot see its own token count; the orchestrator transcribes the harness's metered `subagent_tokens`.
- **Estimated USD:** null (follows the tokens).
- **Duration (minutes):** null (the harness reports it).
- **Source of the number:** harness report, to be transcribed.

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. The full-mask corpus run was cut to 124 sampled masks per file for time; the preset and `Custom{Thumbnail}` runs are complete.
- **Follow-up work identified:**
  - A builder follow-up for H4, L13, L15 and the leak inputs and seeds for each.
  - A DEC-005 or DEC-008 decision for M4 and the ExifTool subdirectory set.
  - The DEC-013 note covering MakerNote offsets, L13 and L14.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Whether "ExifTool agreement" covers data that the audit never sees. The writer's contract is relative to the audit, but `HiddenPointer` exists precisely to close what the walk skips. I treated the guard as the contract.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-005's pointer set is narrower than ExifTool's subdirectory set, and no record says so. DEC-008's "Interop yields nothing" hides an ExifTool-visible tag.

3. **If you did this task again, what would you do differently?**
   — Start with the exhaustive ExifTool tag scan (65,536 tags in 164 files, about a minute). It found H4 directly, which the random differential could not, because both of its models share the walk's view of which pointers exist.
