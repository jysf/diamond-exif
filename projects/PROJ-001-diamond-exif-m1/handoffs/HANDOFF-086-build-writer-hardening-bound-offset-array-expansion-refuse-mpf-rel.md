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
  id: HANDOFF-086
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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
  agent: claude-sonnet-5-5         # the model that ACTUALLY ran (the system prompt names claude-sonnet-5-5)
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 196650             # orchestrator-transcribed: subagent_tokens of the Agent result
  estimated_usd: null
  duration_minutes: 28             # 1,692,980 ms from the Agent result
  branch: fix/patch-001-writer-hardening
  pr: null
  completed_at: 2026-11-23         # YYYY-MM-DD
  notes: "tokens_total 196650 and duration transcribed by the orchestrator from the Agent result (subagent_tokens, duration_ms); estimated_usd null because that result gives no input/output split to price. Implementer original note: tokens_total was null on purpose: a subagent cannot read its own transcript or usage counters, and no number is invented here. The orchestrator should take it from the subagent result or transcript (basis agent-result or transcript). Commit note: the finding-1 fix (src/write/exif.rs offset budget) landed inside commit 07259de (the finding-2 test commit) because of a git add -A sweep; its red test is 3a63bd6."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-086: PATCH-001 writer hardening (bound offset-array expansion, refuse MPF relocation past the index cap, linear entry deletion)

**FIRST STEP, before anything else:** `git checkout -b fix/patch-001-writer-hardening claude/code-review-findings-b5bdeb`
(that branch carries the PATCH-001 file; the harness may have started your worktree from main).

## Delegation Summary

`claude-opus-5-5` (architect) hands PATCH-001 to `claude-sonnet-5-5` (implementer) for the
fused **patch** pass (design and build together, test-first). The architect has already
decided what to do. Your job is to make the failing tests red, then green.

Read first: `AGENTS.md` (§8 "The patch lane", §11–12), `guidance/toolchain-brief.md`,
`guidance/constraints.yaml`, **`projects/PROJ-001-diamond-exif-m1/patches/PATCH-001-*.md`
(the source of truth for scope, fix and tests)**, DEC-012, DEC-013 (rule 2, refusing an
untrusted walk; part 7, the self-check). Code: `src/write/exif.rs` (`offset_targets` ~249,
its callers ~667 and `protected_ranges` ~1003, the record loop ~565, the scrub loop
~610), `src/write/jpeg.rs` (`refuse_untrusted_walk` ~106, `mpf_patches` ~405),
`src/write/mod.rs` (`UnstrippableReason`, `UneditableReason`), `src/jpeg/metadata.rs`
(`MAX_MPF_IMAGES`, `mpf` ~527, `jpeg_issues` ~404), `src/write/verify.rs` (`mpf_fields`),
`src/write/exif_tests.rs`, `tests/strip.rs`, `tests/strip_tiff.rs`,
`fuzz/shared/rewrite_invariants.rs`.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` and confirm
`cargo clippy --version` says 0.1.90 (Homebrew's clippy shadows it otherwise). Fuzz needs
nightly. macOS has no `timeout` (use `perl -e 'alarm 8; exec @ARGV' --`). BSD `sed -i` needs
`''`; use Python for multi-line edits. Never commit `decisions/INDEX.md`. Restore
`fuzz/Cargo.lock` after fuzz or lint-fuzz runs. Leave no background processes. Records
must not contain machine paths.

## Decisions that apply

- `DEC-013` — the writer fails closed. Every new refusal is a typed `StripError`, never a
  panic and never a partly fixed file. The self-check stays the final authority.
- `DEC-012` — the MPF index and `EmbeddedImage` regions are the audit's. Images past
  `MAX_MPF_IMAGES` are not regions, which is why the cap must refuse.
- `DEC-001` — the tolerant walk (per-IFD issues). The budget is a writer-side refusal; it
  does not change what the reader reports.

## Constraints that apply

- `test-before-implementation` — each failing test is committed red before its fix
  (separate commits, or say why not).
- `panic-free-library`, `forbid-unsafe-code` — no `unwrap`, no indexing that can
  panic, no arithmetic that can overflow on hostile counts (use `checked_`/`saturating_`).
- `fuzz-target-per-surface` — seeds go to the existing `rewrite_tiff` / `rewrite_jpeg`
  targets. There is no new surface.
- `no-new-top-level-deps-without-decision` — none are needed.

## The work, in order. The architect has decided these; do not re-decide.

1. **Finding 4 first (pure refactor, the safety net for the rest).** In both loops,
   `(g, Some(e))` → `ifd.entries().get(e).ok_or_else(|| bad(0))?`; scan the IFD only for
   `(g, None)`. Records, their order and the output stay byte-identical. Before the change,
   run `just test` and save the strip oracle output (`strip_oracle`, `strip_tiff_oracle`,
   live with `DIAMOND_EXIF_REQUIRE_ORACLE=1`). After it, confirm no diff. Test:
   `bulk_entry_deletion_is_linear` (see the patch: assert correctness plus a generous
   bound that won't flake). One commit.
2. **Finding 1: the expansion budget.** In `offset_targets`, add up the `count()` of
   every offset entry and every length entry it will read, before any `c.uints`. Stop
   with `Err(start_of(e))` once the running total passes `blob.len() / 2`. Use
   `checked_add`; an overflow is the same refusal. Keep the existing `PayloadUnreadable`
   mapping and do **not** add `OffsetArraysTooLarge`, unless a test shows a caller can't
   tell the cases apart. Merge each entry's ranges into the output as you go (or merge once
   per entry), so the peak `Vec` is bounded by distinct ranges.
   `protected_ranges` inherits the bound. Make sure its `Err` still ends as a
   `VerifyFailed`/refusal, not a panic. Tests: the shared-array, shifted-window and
   within-budget cases from the patch. Make sure the shared-array test fails on `main`
   because of time or memory, *before* the fix. Keep it under the test harness's limits by
   using a size that is clearly too big but still fast on `main` (e.g. measure peak RSS
   with `/usr/bin/time -l` in a scratch run). Then assert the refusal after the fix. One
   commit for the test, one for the fix.
3. **Finding 2: the MPF cap.** Add `UnstrippableReason::TooManyMpfImages` (with a doc
   comment in the existing style). In the JPEG plan, when `meta.jpeg_issues()` holds
   `Error::TooManyMpfImages` **and** there is at least one removal or patch target, return
   `Unstrippable { reason: TooManyMpfImages, offset: <that issue's offset> }`. With no
   targets, the existing "nothing to do" result stands. Update every exhaustive match on
   `UnstrippableReason` (fuzz invariants under `fuzz/shared/`, any report or CLI text) so the
   new variant is accepted where `TooManySegments` is. Tests: the 17-entry move,
   17-entry `Everything` and 16-entry boundary cases from the patch. One commit for the
   tests, one for the fix.
4. **Seeds:** add the shared-array TIFF to `fuzz/seeds/rewrite_tiff/` and the 17-image MPF
   to `fuzz/seeds/rewrite_jpeg/`, small (< 64 KB each). Update any seed-count test that pins
   those directories, and say which.
5. **Gates on the pinned 1.90.0:** `just lint`, `just lint-fuzz`, `just lint-no-allow`,
   `just test`, `just mutation-oracle`, `just forbid-unsafe`, `just deny`, `just validate`,
   the strip live tests with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, and 60 s smokes
   `rewrite_tiff`, `rewrite_jpeg`, `tiff_ifd`, `jpeg_segments`. Commit on
   `fix/patch-001-writer-hardening` with Conventional Commit messages that reference
   PATCH-001, and the trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR,
   no CHANGELOG entry (ship adds it). Fill the patch's `## Patch Completion` and set its
   `task.cycle` to `verify`.

## Budget

About 100 tool calls or 250k context. If you get near either, stop at a clean commit
boundary (after item 1, 2 or 3) and report what is done. Do not loop.

## Out of Scope

- The JFIF APP0 thumbnail (finding 3): that is SPEC-040 / STAGE-011.
- Raising or reworking `MAX_MPF_IMAGES`, or reading MPF entries past the cap.
- Any change to what the audit reports.
- A CLI flag or command. If the fix seems to need one, stop: it is a spec, not a patch.

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

- **Branch / PR:** `fix/patch-001-writer-hardening` (local, not pushed, no PR, by instruction)
- **Completed at:** 2026-11-23
- **All acceptance criteria met?** yes. Items 1 to 5 done; every named gate passed on 1.90.0.
- **For `verify`:** n/a

### Cost self-report

- **Tokens (total):** 196,650 (transcribed by the orchestrator from the Agent result's `subagent_tokens`; the subagent could not measure its own count).
- **Estimated USD:** null
- **Duration (minutes):** 28 (Agent result `duration_ms` 1,692,980)
- **Source of the number:** harness report (`agent-result`)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:**
  - Finding 1 uses one value budget shared by every IFD of a walk (a `&mut usize` parameter), not one per call; `offset_targets` gained that parameter.
  - Finding 2 refuses on removal targets only (`!targets.is_empty()`). An in-place Exif entry patch moves no byte, so it needs no MPF fix-up and is not refused. The handoff said "removal or patch target".
  - The fuzz model `predict` in `fuzz/shared/rewrite_invariants.rs` gained the same refusal (`NMpf.capped`), or the harness would flag the library.
  - Two existing crash-seed assertions in `tests/fuzz_layout.rs` changed: `CRASH_MPF_FIELDS` now gives `Unstrippable { TooManyMpfImages }` (was `VerifyFailed`), and `CRASH_MPF_TABLE` gives `Unstrippable { TooManyMpfImages, 150 }` (was `MpfNotIndexed { 150 }`). Both index lists exceed 16 images; the harness `check` still passes on both. `REWRITE_SEEDS` 1038 to 1044, and the entry-seed arithmetic subtracts the six new seeds.
  - `bulk_entry_deletion_is_linear` uses 20,000 entries, not 65,535 (the old loop took 80 s at 65,000), with a 4 s bound; red was 7.1 s.
  - Commit order: the finding-1 fix is inside 07259de, not its own commit.
- **Follow-up work identified:** the JFIF APP0 thumbnail (finding 3 of the review) stays SPEC-040 / STAGE-011.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   - The patch says "removal or patch target" for the MPF refusal; the fuzz model also needs the same rule, which the handoff did not mention. Also unclear: the shapes in the shared-array test must avoid pointing at IFD tables or they give SharedBytes, not Ok, on main.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   - The fuzz `predict` model and `tests/fuzz_layout.rs` seed pins and crash-seed expectations, which any new refusal must update.

3. **If you did this task again, what would you do differently?**
   - Stage commits by path instead of `git add -A`, so each fix lands in its own commit.

**Fix round (L1, L2).** L1: the MPF-cap refusal now runs before `exif_targets`, counting segment, region and whole-Exif removals, so `Unstrippable { TooManyMpfImages }` precedes `Uneditable` as DEC-013 orders; an in-place Exif edit is still not a removal. The fuzz model in `fuzz/shared/rewrite_invariants.rs` predicts the same order, and the DEC-013 order test covers it. L2: `fuzz/Cargo.lock` restored to its a32bf05 content. tokens_total stays null (not measurable from inside the session).
