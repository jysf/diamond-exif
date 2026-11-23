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
  id: HANDOFF-085
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-23
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-033

project:
  id: PROJ-001
  stage: STAGE-009
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
  tokens_total: 10909378               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 9
  branch: feat/spec-033-rights-category
  pr: https://github.com/jysf/diamond-exif/pull/57
  completed_at: 2026-11-23         # YYYY-MM-DD
  notes: "Approved at 43eb634 with three Low findings (CHANGELOG PNG preview clause, a vacuous kept assertion, no PNG Custom(Rights) carrier row). tokens_total measured from this session's transcript just before the commit (61 messages, each counted once, largest output; no subagents): input 123, cache creation 186056, cache read 10692996, output 30203; a floor by the last few messages. estimated_usd is null because I do not have Opus 5.5 list rates; price the four components."
  synced_at: 2026-11-23
---

# HANDOFF-085: <Task Title — same as the spec's title>

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-033` to a fresh `claude-opus-5-5` (verifier) for the **verify** cycle
of PR #57 (branch `feat/spec-033-rights-category`, built by `claude-sonnet-5-5` under HANDOFF-053). You did not
write the spec or the code. Review it cold.

## Context the Receiving Agent Needs

Read in this order. You share no memory with the architect or the builder.

### Primary

- **Spec (the contract):** `./projects/PROJ-001-diamond-exif-m1/specs/SPEC-033-dec-008-amendment-a-rights-category-for-copyright-kept-by-al.md`.
  Read its "Revision (2026-11-22): PNG and WebP" paragraph: the scope covers JPEG, TIFF, PNG and WebP.
- **The DEC amendments (normative):** the bottom sections of `./decisions/DEC-008-…`, `DEC-009-…`, `DEC-013-…`
  (2026-11-21) and `DEC-020-…`, `DEC-021-…` (2026-11-22). `DEC-022` (WebP strip) applies DEC-013's Exif rule
  unchanged.
- **Build handoff and its handback:** `./projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-053-…md`. The handback
  states what the builder claims: its gate results, its deviations and its grep list. Check every claim; do not
  trust any of them.
- **Stage:** `./projects/PROJ-001-diamond-exif-m1/stages/STAGE-009-keep-the-copyright-notice-under-allpii.md`
- **Toolchain brief:** `./guidance/toolchain-brief.md` (the `RUSTUP_TOOLCHAIN=1.90.0` lint quirk, the nightly fuzz
  invocation, ExifTool 13.55).

### Decisions that apply

- `DEC-008`: Copyright 0x8298 is `Rights`, Medium. `Authorship` means "names a person". No severity changes.
- `DEC-009`: `Rights` is removed by `All` and `Everything` only. May-hold rows for Thumbnail, Xmp, Iptc, Jumbf,
  EmbeddedImage and OtherSegment. No other preset disposition changes.
- `DEC-010`: the reports name the category `rights`.
- `DEC-013`: EXIF9 becomes EXIF10. `All` still removes Exif segments whole. Artist and Copyright sharing bytes
  under `AllPii` is `SharedBytes`.
- `DEC-020`, `DEC-021`: the PNG `Copyright` text keyword is `Rights`. The PNG strip keeps it and the `eXIf`
  Copyright under `AllPii`. A `Rights`-only `Custom` set removes carriers.

### Constraints that apply

From `./guidance/constraints.yaml`: `test-before-implementation`, `panic-free-library`, `forbid-unsafe-code`,
`oracle-must-be-shown-red`, `fuzz-target-per-surface`, `exiftool-never-linked`,
`no-new-top-level-deps-without-decision`, `one-spec-per-pr`.

### Prior related work

- `HANDOFF-083` (SPEC-038 verify): the most recent verify in this repo, with the punch-list format and how
  findings were graded H/M/L.
- `SPEC-023` / PR #36: the last category amendment. A fuzz model in `fuzz/shared/rewrite_invariants.rs` was
  missed there, and only CI's smoke caught it.

## Expected Deliverables

Check out `origin/feat/spec-033-rights-category` (PR #57). Do not push code fixes. Commit only your handback
on that branch: this file, with the SPEC-033 timeline's verify line ticked. Then report.

### If `cycle: verify` — the verify contract

You are **not** the agent that implemented this. Review it cold.

1. **Every acceptance criterion**, one by one, with the evidence (a test name and its run, or a command and its
   output). Run `just test`, `RUSTUP_TOOLCHAIN=1.90.0 just lint`, `just lint-fuzz`, `just deny` and
   `just mutation-oracle` yourself.
2. **Completeness grep.** Run `grep -rn "exif9\|EXIF9\|all9\|exif_set\|Authorship\|Copyright\|copyright"` over
   `src`, `fuzz/shared` and `tests`. Every Exif-set copy must hold ten categories, and every Copyright row must say
   `Rights`, unless it is `COPYRIGHT`-style free text or an XMP or IPTC field (both non-goals). Compare the result
   with the builder's list.
3. **The model copies are independent.** The fuzz models and `verify.rs`'s list must be typed by hand, not call
   the library's function.
4. **Record diff discipline.** In `git diff origin/main -- tests/oracle-fixtures`, the strip records may change
   only in the `## all_pii` and `## authorship` sections (a Copyright line appears) and the new `## rights`
   sections. The inventory record may gain only the new seed. Anything else is a finding.
5. **No other disposition moved.** Spot-check the preview grid and `max_severity` against the DEC-009 amendment's
   claim that only Copyright's disposition changes.
6. **Red-proofs (oracle-must-be-shown-red).**
   - Confirm that the committed comparator red-proof ("comparator is red when copyright is removed under
     all_pii") exists and fails when it should.
   - Then do the **library red-proof** in a scratch copy, never committed: put 0x8298 back in the `Authorship`
     arm of `by_id`, and the PNG `Copyright` keyword row back to `Authorship`. Confirm that the replay strip
     oracle test, the `tests/strip.rs` AllPii test, the PNG strip test and the WebP strip test each go red.
   - Record which tests went red.
7. **Live and fuzz.**
   - Run `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test --test strip_png_live --test strip_webp_live` and the
     ExifTool strip oracle live.
   - Run `just fuzz-smoke` for `audit`, `audit_render`, `rewrite_jpeg`, `rewrite_tiff`, `rewrite_png` and
     `rewrite_webp` (60 s each). Run them in the background; they are slow.
8. **Docs.** Check the CHANGELOG `[Unreleased]` entry and the two README edits against the spec's Outputs.
9. **The handback** is answered honestly, and its cost basis is stated.
10. Run `just decisions-audit --changed`.

Grade each finding H, M or L, with file:line and a concrete failure scenario. Output exactly ONE of:
✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED.

## Out of Scope

- Fixing anything. A punch list goes back to a build fix round.
- Re-deciding the DEC amendments. If one is factually wrong, report it as a finding.
- A CLI option to pick categories, XMP `dc:rights` or IPTC, and widening `StripSpec`.

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

- **Branch / PR:** `feat/spec-033-rights-category`, https://github.com/jysf/diamond-exif/pull/57 (verified detached at `43eb634`, compared with `origin/main` at `408ab3f`)
- **Completed at:** 2026-11-23
- **All acceptance criteria met?** yes. Three Low findings, none of them a behaviour defect.
- **For `verify`:** ✅ APPROVED at `43eb634`

**Gates (all run by me on `43eb634`, ExifTool 13.55, rustc 1.90.0 pinned for lint).**

| Gate | Result |
|---|---|
| `just test` | exit 0; 39 suites, 1,455 passed, 0 failed, 18 ignored; no `SKIP` line. The builder reported 1,036; my count is every `test result` line, doctests included. Both are green. |
| `RUSTUP_TOOLCHAIN=1.90.0 just lint` | exit 0 (fmt check and clippy `-D warnings`) |
| `just lint-fuzz` | exit 0 |
| `just deny` | exit 0, `licenses ok` |
| `just mutation-oracle` | exit 0; 67 + 40 + 41 passed, 0 failed |
| `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test --test strip_png_live --test strip_webp_live --test strip_oracle -- --include-ignored` | strip_oracle 33 passed, strip_png_live 22 passed, strip_webp_live 24 passed; 0 failed, 0 ignored, no skip |
| `just fuzz-smoke` x6 (60 s each) | no crash: audit 114,719 runs, audit_render 24,220, rewrite_jpeg 87,229, rewrite_tiff 253,749, rewrite_png 420,777, rewrite_webp 855,479 |
| `just decisions-audit --changed` | advisory only; every hit is `fuzz/Cargo.lock`, which the fuzz build rewrites locally. I restored it; it is not in the PR or in this commit. |

**Contract points.**

1. **Acceptance criteria.** Each one has a test that passed in the run above: `copyright_is_rights_and_artist_stays_authorship` (category, severity, name, `is_block`, three IFD kinds); `policy.rs` `REMOVED` grid (16 rows), may-hold grid and `rights_bit_is_distinct_and_the_set_is_full`; `policy_preview.rs` grid, `all_pii_keeps_a_copyright_finding` and `nesting_holds_on_every_committed_seed_for_every_preset`; `strip.rs` `all_pii_keeps_copyright_byte_identical` (both byte orders, entry and value bytes, Artist zeroed, `Custom([Authorship])`), `custom_rights_removes_only_copyright`, `old_exif9_custom_set_is_now_partial_and_keeps_copyright`, `artist_and_copyright_sharing_bytes_is_refused_under_all_pii`; `strip_tiff.rs` `tiff_all_pii_keeps_copyright_and_deletes_artist`; `strip_png.rs` `png_all_pii_keeps_the_copyright_text_chunk_and_the_exif_copyright`; `strip_webp.rs` `webp_all_pii_keeps_the_exif_copyright`; `report_formats.rs` `copyright_finding_renders_as_rights` (but see L2); the replay oracle `exiftool_tag_sets_equal_the_declared_tables_per_policy_and_fixture`; the live tests above. Severity is unchanged (Medium), so no `max_severity` can move.
2. **Completeness grep** (`exif9|EXIF9|all9|exif_set|Authorship|Copyright|copyright` over `src`, `fuzz/shared`, `tests`). No `exif9`, `EXIF9` or `all9` is left (the only hit is the test name `old_exif9_custom_set_is_now_partial_and_keeps_copyright`, deliberate). Every Exif-set copy holds ten: `exif.rs` `exif10`, `verify.rs` `exif_gone` list and `exif_set() -> [Category; 10]`, `EXIF10` in `rewrite_invariants.rs`, `rewrite_png_invariants.rs`, `rewrite_webp_invariants.rs`, `strip_shapes.rs`, `strip_png.rs`, `strip_webp.rs`, and `FULL_CATS` in `strip_shapes.rs`. Every 0x8298 row is `Rights`: `category.rs` `by_id`, `audit_invariants.rs` tag table, `rewrite_invariants.rs` `by_id` copy, `exif_layout.rs` `category`. Every `Copyright` keyword row is `Rights`: `TEXT_KEYWORDS`, `audit_invariants.rs` keyword model, `rewrite_png_invariants.rs` `ROWS`. Remaining `Copyright` hits are doc comments, the WebP `LIST/INFO` comment, and ICC `ProfileCopyright` (Icc, out of scope). The remaining `Authorship` hits are Artist, CameraOwnerName, XPAuthor, Photographer, ImageEditor and `Author`/`Artist` keywords. This matches the builder's list.
3. **Model independence.** The fuzz models (`audit_invariants.rs` tag table, severity, order, may-hold, keyword model; `render_invariants.rs` `CATS`; `rewrite_invariants.rs` `EXIF10` and its own `by_id`; the PNG and WebP `EXIF10` and keyword `ROWS`) and `verify.rs`'s two lists are typed by hand. None calls `categorize`, `exif10()` or `StripPolicy::spec`.
4. **Record diff.** Checked by a script, section by section, against `origin/main`: in all 33 `exiftool-strip` records every section other than `all_pii` and `authorship` is byte-identical, a new `## rights` section is inserted after `## authorship` (before `## free_text+device` where that exists), and in `all_pii`/`authorship` the only change is an added `IFD0 Copyright CANARY-COPYRIGHT-…` line, in place, in exactly the four Copyright fixtures (`exif-full-le/be`, `exif-makernote-canon-style`, `exif-at-the-ceiling`). `kitchen-sink` and `no-exif` print a Copyright in `all` and `rights` only as `ICC_Profile ProfileCopyright`, which is Icc. The inventory record gains only `audit-copyright-only-le` (`IFD0 0x8298 Copyright`). Nothing else changed.
5. **No other disposition moved.** The hand-typed `REMOVED` grid in `policy_preview.rs` is checked over every committed seed under five presets and passes; the only changed row is `Rights`. The preview unit pins move from `(1,8,6) (2,7,6) (8,2,5) (11,1,3) (15,0,0)` to `(1,9,6) (2,8,6) (8,3,5) (12,1,3) (16,0,0)`: one more finding, kept by the first three presets and removed by the last two, which is the `Rights` row and nothing else.
6. **Red-proofs.** The committed `comparator_is_red_when_copyright_is_removed_under_all_pii` exists (`tests/strip_oracle.rs:560`), drops the Copyright line from the `all_pii` record, asserts the line was there, and requires the error to name `missing` and `Copyright`; it passes. **Library red-proof** in a scratch copy (outside the repo, never committed): 0x8298 back in the `Authorship` arm of `by_id` and the `Copyright` keyword row back to `Authorship`. Red: `strip_oracle` 3 (`exiftool_tag_sets_equal_the_declared_tables_per_policy_and_fixture`, `strip_records_replay_and_reproduce`, `exiftool_values_of_kept_tags_equal_the_inputs`); `strip` 10 (including `all_pii_keeps_copyright_byte_identical`, `all_pii_matches_the_policy_table_entry_by_entry`, `custom_rights_removes_only_copyright`, `artist_and_copyright_sharing_bytes_is_refused_under_all_pii`, `old_exif9_custom_set_is_now_partial_and_keeps_copyright`, `every_kept_canary_survives`); `strip_png` 2 (`png_all_pii_keeps_the_copyright_text_chunk_and_the_exif_copyright`, the shape grid); `strip_webp` 1 (`webp_all_pii_keeps_the_exif_copyright`); `strip_tiff` 1; `policy_preview` 3. All four required tests went red. Extra: the keyword row alone reverted (by_id left as `Rights`) turns 7 tests red (`strip_png` 2, `audit_png_webp` 1, `audit_png_webp_oracle` 4), so the PNG row is guarded on its own.
7. **Live and fuzz.** See the gate table. Additionally, an **independent probe** outside the builders' tables: a scratch crate calling `exif_core::strip` on files I built by hand (Exif with Artist, Copyright in IFD0, Copyright in ExifIFD and Copyright in IFD1, both byte orders), as TIFF, JPEG, PNG (`eXIf` plus `tEXt Copyright`, `iTXt copyright`, `tEXt COPYRIGHT`, `tEXt Author`) and WebP (`EXIF`), stripped under the five presets, `Custom([Rights])`, `Custom([Authorship])` and `Custom(old nine)`, with ExifTool 13.55 run on every output. 64 outputs, no refusal, 0 mismatches against DEC-009 as amended: every Copyright (IFD0, ExifIFD, IFD1, both PNG keyword spellings) is kept by `Gps`, `GpsAndDevice`, `AllPii`, `Authorship` and the old nine, and gone under `All`, `Everything` and `Rights`; Artist and `Author` gone under `AllPii`, `All`, `Everything`, `Authorship` and the old nine; `COPYRIGHT` behaves as free text.
8. **Docs.** README's two edits match the spec's Outputs (line 19 PNG keyword sentence, line 63 "copyright notices"). The CHANGELOG `[Unreleased]` `### Changed` entry covers the category rename in all three formats, `by_category`'s new key, the preview, the library strip, `All`/`Everything`, the non-goals and `StripSpec` being full; one clause is wrong for PNG (L1).
9. **Builder's handback.** Honest and checkable: the gate claims reproduce, the grep list matches, the pins it lists moved, and both deviations are stated with reasons. I accept both: the full-house inputs exempting `Rights` (as they already exempt the two DEC-012 categories; `audit-copyright-only-le` carries `Rights` and `fuzz_layout.rs` asserts it), and the `release_layout.rs` relaxation (the old test required an empty `[Unreleased]`, which the spec's own CHANGELOG requirement contradicts; the new check guards the `is_some` case). Its cost basis is stated (transcript, four components).
10. `just decisions-audit --changed`: see the gate table.

**Findings.**

- **L1 (Low, docs) The CHANGELOG says a PNG `Copyright` text chunk's `all-pii` preview changed from removed to kept; it did not.** `CHANGELOG.md:13`: "The `all-pii` preview now shows a Copyright finding, in an Exif block or a PNG text chunk, as kept (it was removed)". `ContainerKind::strippable` (`src/audit/mod.rs:103`) is true only for JPEG and TIFF, and `preview` (`src/audit/preview.rs:148`) gives every finding of a non-strippable container `Kept`, so a PNG preview showed `Kept` before this change too, and still says `kept (not strippable)`. Failure scenario: an alpha user diffs the `all-pii` report of a PNG before and after upgrading to see the documented change and finds none, except the category name. The wording comes from the spec's Outputs, so the spec carried the error. Fix: say the PNG text chunk is now `rights` and that the library's PNG `AllPii` strip keeps it, and drop the preview claim for PNG (or say it applies once PNG becomes strippable, SPEC-039).
- **L2 (Low, test teeth) `copyright_finding_renders_as_rights` asserts `text.contains("kept")`, which is always true.** `tests/report_formats.rs:1307`. The text report's preview summary line always prints `kept {n}` (`src/report/text.rs:150`), so the assertion would pass if `AllPii` removed the Copyright. Failure scenario: a regression that removes Copyright under `AllPii` in the preview leaves this test green. Not a gap in the suite as a whole: `policy_preview.rs` `all_pii_keeps_a_copyright_finding` checks the disposition and went red in the library red-proof. Fix: assert the finding line's disposition text (`kept (not targeted by this policy)`) or the summary's `kept 1`.
- **L3 (Low, coverage) No PNG test that a `Custom([Rights])` policy removes the opaque carriers (DEC-021 amendment, clause 3).** `tests/strip_png.rs:442` `policies()` has single-category rows for Icc, Xmp, Jumbf, OtherSegment and Timestamps but none for `Rights` (strip_webp's `specs()` at `tests/strip_webp.rs:366` has `custom-rights`). Failure scenario: someone rewrites PNG's carrier rule without going through `png::removes_carriers` and leaves `Rights` out; no PNG test fails. Today the shared function is exercised by the WebP `custom-rights` grid row and by the `rewrite_png` fuzz model (`carriers_removed` uses `EXIF10`), so it is covered indirectly. Fix: add `("rights", Custom([Rights]))` to `strip_png.rs` `policies()`.

### Cost self-report

- **Tokens (total):** see `handback.tokens_total` (10909378: input 123, cache creation 186056, cache read 10692996, output 30203)
- **Estimated USD:** null; I do not have Opus 5.5 list rates, so the four components are given for pricing per docs/cost-tracking.md "Pricing".
- **Duration (minutes):** 9 (first to last timestamp of my transcript)
- **Source of the number:** this session's transcript (`c67f6675-…jsonl`), measured per docs/cost-tracking.md "Measuring a cycle": 61 messages, each counted once, largest output; no subagents. Taken just before the commit, so a floor by the last few messages.

### Drift and new artifacts

- **New decisions emitted:**
  - none
- **Deviations from spec:**
  - None by the build that I would send back. The two the builder stated (full-house inputs exempt `Rights`; `release_layout.rs` relaxed) are accepted above.
  - The spec's Outputs sentence for the CHANGELOG is itself wrong for PNG (L1).
- **Follow-up work identified:**
  - L1 to L3 are small enough for ship or a patch: fix the CHANGELOG clause, tighten `report_formats.rs:1307`, add a `rights` row to `strip_png.rs` `policies()`. No second verify round needed (spec: only for High or Medium).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Little. The handoff's title is still the template placeholder (`<Task Title …>`). The spec's CHANGELOG wording asserted a PNG preview change that the PNG preview cannot show (L1); the spec should have checked `strippable` before writing it.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-020's rule that a non-strippable container's preview keeps everything; it decides what a PNG user sees from this change.

3. **If you did this task again, what would you do differently?**
   — Start the independent ExifTool probe (hand-built files, every preset, ExifTool on the output) first: it is cheap, it covered IFD1 and ExifIFD Copyright and the `iTXt` spelling that no committed test uses, and it would have found a leak fastest if there were one.
