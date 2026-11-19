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
  id: HANDOFF-049
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-19
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-023

project:
  id: PROJ-001
  stage: STAGE-005
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
  basis: transcript                # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 4802409            # REAL combined count — what cost-audit reads
  estimated_usd: 3.97              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 16
  branch: claude/dec-008-dng-and-pages
  pr: null
  completed_at: 2026-11-19         # YYYY-MM-DD
  notes: "Transcript subagents/agent-a52315611cf9f3036.jsonl, 37 messages deduplicated by message.id (max output per message): input 73, cache creation 163,801, cache read 4,612,837, output 25,698. Measured live before the handback edit and commit, so a floor. USD assumes Opus list rates of 5 USD/Mtok in and 25 USD/Mtok out (not in docs/cost-tracking.md); re-price if the Opus 5.5 rate differs. The harness subagent_tokens figure, the basis of earlier specs, is a different quantity."  # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-049: Verify SPEC-023 (DEC-008 amendment: DNG identity tags, container-aware TIFF page rule)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-023` to a fresh verifier (`claude-opus-5-5`) for the **verify** cycle. You did not write this code. Review it cold, adversarially, and return exactly ONE verdict: APPROVED / PUNCH LIST / REJECTED, with every finding graded High, Medium or Low.

## BUDGET (hard)

About **80 tool calls or 200k context**. Read large files with offset/limit. Work in this worktree (`/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/diamond-exif/.claude/worktrees/spec-022-tiff-dng-strips-26c250`, branch `claude/dec-008-dng-and-pages`, build commit `5582129`). Do NOT change code, push, or open a PR; write your findings in the handback only. Use `~/.cargo/bin/cargo +1.90.0` and `RUSTUP_TOOLCHAIN=1.90.0` for lint (see `guidance/toolchain-brief.md`). Record a real token count and say how you got it.

## What to read

- Spec: `projects/PROJ-001-diamond-exif-m1/specs/SPEC-023-dec-008-amendment-dng-identity-tag-categories-and-a-containe.md` (Goal, Decisions made here, Acceptance Criteria, Failing Tests, Non-Goals).
- Build handback: HANDOFF-048 (same directory). Diff: `git diff d534384..5582129` (the framing commit to the build commit); start with `src/audit/category.rs`, `src/audit/mod.rs`, `src/write/exif.rs`, `src/write/verify.rs`, `fuzz/shared/audit_invariants.rs`, then the tests.
- DEC-008, DEC-013 (amendments C and D, D.3), DEC-009, DEC-012; `guidance/constraints.yaml`.

## What the builder reports, and where to be suspicious

1. **Unrequested changes.** (a) `src/write/verify.rs`: the `entries` self-check now removes IFD1 only where the audit calls it a thumbnail. Check it cannot let a real thumbnail removal slip through unverified, or let a page's entries be removed unchecked. (b) `fuzz/shared/audit_invariants.rs`: the INDEPENDENT fuzz model was edited to mirror the production rule. A differential model that is changed to match the code it checks is weaker; judge whether it still checks something independent, and whether a deliberate production bug would still be caught.
2. **Changed tests.** The multi-page refusal assertion became an Ok assertion; `expect_refusal` in `every_shape_under_every_policy_keeps_the_image_data_and_the_length` is now always false; two `audit-ifd1-strips` plants in `tests/fuzz_layout.rs` now use a generated JPEG-wrapped copy of the seed. Confirm these changes follow the spec and do not just loosen the tests; confirm the JPEG rule (IFD1 with strips and no 0x0201 is still a `Thumbnail` inside a JPEG Exif block) is really exercised by a test that fails if the container-awareness is dropped.
3. **Oracle red-proofs.** The builder showed the unit, audit and strip tests red first, but the oracle tests only by planting faults in the comparator, not by reverting the `by_id` rows or the old IFD1 rule. Check the replay records are real ExifTool output of the new fixtures (re-record with `just oracle-record` into a scratch copy or compare, if ExifTool is installed) and that reverting the production change turns an oracle or strip test red. Do the mutation yourself on a scratch copy of the tree or by a temporary edit you revert; leave the tree clean.
4. **Behaviour to probe.** (a) A kept page IFD1: its own Make/DateTime/Artist entries are deleted by category, its strips and tiles identical; chain pages after IFD1; a page IFD1 that holds a SubIFDs, Exif or GPS pointer. (b) DNGPrivateData 0xC634 and OriginalRawFileName 0xC68B: out-of-line bytes zeroed, nothing shared with kept data; a 0xC634 whose value overlaps a strip or raw data must refuse, not zero image bytes. (c) An IFD1 with both the JPEG pair and strips is still refused; a JPEG-pair thumbnail IFD1 in a bare TIFF is still removed whole. (d) `AllPii` keeps Software, so the page's Software survives `AllPii`; confirm that matches DEC-009 and the docs say nothing different. (e) `GpsAndDevice` leaves 0xC634, as DEC-008 now says.
5. **Independent differential.** Write a throwaway script or test (do not commit it) that runs `strip` under all five presets over every seed in `fuzz/seeds/rewrite_tiff/` and the shapes in `tests/support/tiff_shapes.rs`, and asserts on every Ok result: same length; every strip, tile and raw-data range of the INPUT walk byte-identical; no finding of a removed category in the output audit; second strip is a no-op. Report counts.
6. **Gates.** Re-run `just lint`, `just test`, `just forbid-unsafe`, `just deny`, `just validate`, and `just fuzz-smoke rewrite_tiff` and report results. Check docs: DEC-008 and DEC-013 D.3 amendments are accurate and short; README no longer promises what is not delivered and no longer lists what is now fixed; `guidance/questions.yaml` entries are consistent.

## Out of scope

PNG, WebP, HEIC, BigTIFF, `set` operations, MakerNote parsing, changing preset membership, the `audit-blob-carrier-parsing` question. Findings there are at most Low notes.

## Return Criteria

Fill the `handback:` front matter (`verdict`, a real `tokens_total`, `basis`, `branch`, `completed_at`) and the `## Handback` section with the verdict, every finding (grade, file:line, failing input or test, suggested fix) and the differential counts. Set `handoff.status` and `handback.status` to `completed`. Make no commits other than the handback file edit; do not push.

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

### Execution notes

- **Branch / PR:** `claude/dec-008-dng-and-pages`, no PR (verify cycle). Reviewed build commit `5582129` (diff `d534384..5582129`).
- **Completed at:** 2026-11-19
- **All acceptance criteria met?** yes. Every AC holds on the build commit, checked by the committed tests, by mutation and by an independent probe (below).
- **For `verify`:** ✅ APPROVED at `5582129`. Three Low findings, none blocking.

#### Findings

- **L1 (Low), test gap: no committed fixture has a removable identity tag or a sub-IFD pointer inside a page IFD1.** `tests/support/tiff_shapes.rs:386` (`multipage`) and `:496` (`tiled_page`) put only Software in IFD1, so the AC "its IFD1-held Make/DateTime entries are deleted per policy" is not guarded by the suite. Behaviour is correct: a throwaway probe (bare TIFF, page IFD1 with strips, Model, Software, DateTime, Artist, DNGPrivateData, an Exif pointer to DateTimeOriginal, a GPS pointer, then a chain page with an Artist) stripped under all five presets in both byte orders with every canary gone exactly when its category is in the preset, strips identical, idempotent, self-check passing. Suggested fix: add Make, DateTime and a GPS pointer to `tiled_page`'s IFD1 (or a new shape) and let `every_shape_under_every_policy_keeps_the_image_data_and_the_length` cover it.
- **L2 (Low), the independent fuzz model's four new rows are never exercised by a committed seed.** `fuzz/shared/audit_invariants.rs:250-253`. Mutation M2 (the four `by_id` rows reverted in `src/audit/category.rs`) turned the unit tests, `strip_tiff`, the live ExifTool tests and `report_render` red, but not `fuzz_layout`'s audit-harness replay, because `fuzz/seeds/audit/` has no file carrying 0xC614/0xC615/0xC68B/0xC634 (the dngidentity seeds went to `rewrite_tiff` only, whose invariants use the production audit). Also by design the replay oracle (`recorded_exiftool_agrees_with_every_stripped_output`) cannot see a library regression, since the records hold ExifTool's reading of the output at record time; only the live tests catch it. Suggested fix: emit a `tiff-dngidentity-*` seed into `fuzz/seeds/audit/` from `examples/fuzz_seeds.rs` so the model's rows are compared on every replay.
- **L3 (Low), DEC-008 amendment wording overstates the page rule.** `decisions/DEC-008-audit-categories-and-severity.md:148`: "an IFD1 with strips or tiles is a page and yields no `Thumbnail` finding". The decided edge case (spec, Part 2) and the code keep a `Thumbnail` finding on 0x0201 when IFD1 holds the JPEG pair and strips (`page_and_jpeg`), and the same for the pair with tiles (probe: refused `ThumbnailNotLocated` under AllPii/All/Everything, both byte orders). Suggested fix: "an IFD1 with strips or tiles and no 0x0201 is a page ...; one with both is reported as today and the writer refuses it (DEC-013 D.3 amendment)."

#### Handoff checks

1. **Unrequested changes.** (a) `src/write/verify.rs:261-275`: the IFD1 set the self-check expects gone is derived from the input audit's IFD1 `Thumbnail` findings, the same source `select` uses (`src/write/exif.rs:929-942`), so a page IFD1 is expected kept and a thumbnail IFD1 expected gone. A writer that removed a page, or kept a thumbnail, still fails the check. Mutation M4 (IFD1 always expected gone) turned `strip_tiff` (4), `strip_tiff_oracle` (2) and `strip::no_fixture_or_seed_gives_verify_failed` red. The only asymmetry (verify keys on `f.item`, select on `f.at`) fails closed. (b) The fuzz model edit adds `block.is_some()`, the spec's rule typed again over the model's own walk. It still checks independently: M1 (old rule restored) turned `audit_harness_holds_on_every_seed_and_degenerate_inputs` red, and M1b (JPEG strips IFD1 never a thumbnail) turned `audit_harness_each_iff_invariant_goes_red_when_planted` and `rewrite_jpeg_harness_holds_on_every_seed_including_the_entry_seeds` red. Gap: L2.
2. **Changed tests.** The multipage-refusal assertion becoming Ok and `expect_refusal` returning `false` follow AC 5 and 6: no shape in `all()` is refused now, and the one refused shape (`page_and_jpeg`) has its own test. The `audit-ifd1-strips` plants now use the generated JPEG-wrapped copy, which is right because the bare seed has no `Thumbnail` finding any more. The JPEG rule is guarded: M1b turns `audit_model::jpeg_exif_ifd1_with_strips_stays_a_thumbnail`, the lib unit test, three `fuzz_layout` tests and `strip::leak_inputs_refuse_with_the_named_reason_and_offset` red.
3. **Oracle red-proofs.** `just oracle-record` with ExifTool 13.55 rewrote all 14 strip-tiff records (and the 23 strip and 12 exif-entry ones) byte-identical to the committed files, so the six new records are real ExifTool output. Reverting production turns the oracle red: M1 makes `recorded_exiftool_agrees_with_every_stripped_output` and `live_exiftool_agrees_on_the_spec_023_fixtures` fail; M2 makes `live_exiftool_agrees_on_the_spec_023_fixtures` and `live_exiftool_agrees_with_every_stripped_output` fail (replay stays green, see L2). M3 (`select` no longer gated on the audit) turned `strip_tiff` (5) and `strip_tiff_oracle` (2) red. Every mutation was a temporary edit reverted with `git checkout`; the tree is clean.
4. **Probes (throwaway test, deleted).** (a) Page IFD1 with identity tags, Exif and GPS pointers, chain page after it: OK as L1 describes. (b) 0xC634, 0xC68B and 0xC614 whose out-of-line value lies inside a strip: refused `SharedBytes` under AllPii and Everything, both orders; image bytes never zeroed. The dngidentity fixture: canaries gone, image and SubIFD tile data identical. (c) JPEG pair + strips IFD1 and JPEG pair + tiles IFD1: refused `ThumbnailNotLocated`. A JPEG-pair thumbnail IFD1 holding a Make in a bare TIFF is removed whole (one IFD left, thumbnail and Make canaries gone). (d) AllPii keeps the page's Software, which matches DEC-009 line 64 and `src/policy.rs:117`; no doc says otherwise. (e) GpsAndDevice and Gps leave all four DNG tags byte-identical (`dng_identity_tags_go_under_everything_and_all_pii_stay_under_gps_and_device`, plus the probe for 0xC634 in a page IFD1).
5. **Independent differential** (a throwaway test, not committed): every seed in `fuzz/seeds/rewrite_tiff/` (policy prefix removed), every shape in `tiff_shapes::all()` plus `page_and_jpeg`, and every `hostile()` input, under all five presets: **215 inputs, 1,075 runs; 662 Ok, 208 typed refusals, the other 205 runs (41 inputs, by subtraction) skipped as non-TIFF: the `soi-then-tiff-*` seeds sniff as JPEG, 0 assertion failures.** On every Ok: same length; every strip and tile range of the INPUT walk byte-identical (and the 0x0201 range when the preset keeps `Thumbnail`); no finding of a removed category in the output audit; a second strip returns the same bytes. Refusals: `ThumbnailNotLocated` only on the pagejpeg inputs (16 per Thumbnail preset), `SharedBytes` and `BlockNotClean` only on the hostile inputs (pre-existing).
6. **Gates** (`~/.cargo/bin` first, `RUSTUP_TOOLCHAIN=1.90.0`, clippy 0.1.90): `just test` green (919 tests passed, 0 failed, ExifTool present so the live tests ran); `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just forbid-unsafe`, `just deny`, `just deny-fuzz`, `just validate` all exit 0; `just fuzz-smoke rewrite_tiff` ran 496,839 executions in 61 s with no crash. Docs: the DEC-008 and DEC-013 D.3 amendments are short and accurate apart from L3; the README known-limits bullet now names only PNG and WebP; both `questions.yaml` entries are `answered` and name SPEC-023.

### Cost self-report

- **Tokens (total):** 4,802,409 (input 73 + cache creation 163,801 + cache read 4,612,837 + output 25,698, over 37 messages). A floor: measured before the handback edit and commit.
- **Estimated USD:** 3.97, assuming Opus list rates of 5 USD/Mtok in and 25 USD/Mtok out, with cache reads at 0.10× and 5-minute cache writes at 1.25×.
- **Duration (minutes):** 16
- **Source of the number:** transcript (`subagents/agent-a52315611cf9f3036.jsonl` of session `c6de7258`), counted per `docs/cost-tracking.md` "Measuring a cycle". This is not the harness `subagent_tokens` basis that earlier specs used.

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none found in the build.
- **Follow-up work identified:** L1 and L2 (test and seed additions) and L3 (a one-sentence DEC-008 wording fix) can go into one small chore or the next TIFF spec.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing material. The handoff says to use "strip, tile and raw-data ranges of the INPUT walk", while the fuzz target uses the OUTPUT walk; a removed thumbnail's range has to be left out of the input form, which I did.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Replay records cannot catch a library regression, only the live run can. That should be stated wherever "oracle red-proof" is asked for, so verifiers mutate production with ExifTool present.

3. **If you did this task again, what would you do differently?**
   — Write each mutation as a single, narrow edit from the start. The M3 edit (`&&` changed to `||`) was broader than needed. It was still red, but a narrower edit would show more precisely which tests depend on the gate.
