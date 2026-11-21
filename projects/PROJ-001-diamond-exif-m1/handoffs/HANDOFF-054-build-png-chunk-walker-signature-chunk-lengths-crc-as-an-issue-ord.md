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
  id: HANDOFF-054
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-028

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
  agent: claude-sonnet-5-5         # the model that ACTUALLY ran (per the system prompt)
  verdict: null                    # verify handoffs only
  basis: agent-result                # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 254968              # harness subagent_tokens: 186,546 (build) + 68,422 (first attempt, stopped on a wrong worktree base)
  estimated_usd: 1.68
  duration_minutes: 13
  branch: claude/spec-028-png-walker
  pr: null
  completed_at: 2026-11-21
  notes: "No token counter is available inside this subagent run, so tokens_total is null rather than invented; the orchestrator replaces it with the harness subagent_tokens figure. About 60 tool calls."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-054: Build SPEC-028 PNG chunk walker (DEC-017) with the png_chunks fuzz target

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-028` to `claude-sonnet-5-5` (implementer) for the **build** cycle. The spec is the contract: read it whole first (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-028-png-chunk-walker-signature-chunk-lengths-crc-as-an-issue-ord.md`), then `decisions/DEC-017-png-chunk-walk-and-issue-channel.md`, which fixes every behaviour question. If the two seem to disagree, stop and say so in the handback; do not choose.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**. At the limit: commit what you have, fill the handback (exactly what is done and what remains), stop.
- Read large files with offset/limit. Prefer one write per file over many small edits.
- Record a real `tokens_total` in the handback and say in `basis` where it came from; never invent one (the orchestrator replaces it with the harness `subagent_tokens` figure; do not count cache reads).
- Work in a worktree on branch `claude/spec-028-png-walker` cut from `claude/spec-028-design` (the design branch holds the spec and DEC); commit on it. Do NOT push, do NOT open a PR, do NOT touch main. No new dependency.
- Toolchain: `~/.cargo/bin/cargo +1.90.0`, `RUSTUP_TOOLCHAIN=1.90.0` for `just lint`; read `guidance/toolchain-brief.md`. BSD `sed -i` rejects multi-expression use: edit with the edit tool or Python. Use absolute paths in shell commands; do not `cd` out of the worktree.

## Context to read first

- The spec (Inputs, Outputs, Acceptance Criteria, Failing Tests, Non-Goals, Notes) and DEC-017.
- `src/jpeg/mod.rs`, `src/jpeg/issue.rs`, the `Error` additions DEC-006 made in `src/error.rs`: the pattern to mirror, including docs and the progress argument.
- `fuzz/fuzz_targets/jpeg_segments.rs`, `fuzz/shared/jpeg_metadata_invariants.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`: how a target, its shared model, the seed generator and the seed pins fit together. Many tests pin seed counts and name sets; adding seeds without updating every pin breaks them (it did in SPEC-023), so grep before and after.
- `AGENTS.md` sections 8, 11 to 13, 15; `guidance/constraints.yaml` (`panic-free-library`, `fuzz-target-per-surface`).

### Decisions that apply

- `DEC-017` the contract. `DEC-006` the pattern it copies. `DEC-001` tolerant walk. `DEC-002` borrowed slices. `DEC-003` provenance: net-new code from the PNG specification, class 1, no port; add the ledger row the repo's process requires.

## Work items (in this order; tests first)

1. **Failing tests first.** `tests/support/png_shapes.rs` (builders) and `tests/png_walk.rs`, plus the `crc` unit tests. Run them and see them red.
2. **Errors.** Add the variants listed in DEC-017 to `Error` (additive, `#[non_exhaustive]` already), with Display.
3. **`src/png/`**: `crc.rs` (const table), `issue.rs`, `mod.rs` (walker, `Png`, `Chunk`, `ChunkKind`, caps). Iterative, checked arithmetic, no `unsafe`, no `#[allow]`, no dependency.
4. **Fuzz target** `png_chunks` and `fuzz/shared/png_chunk_invariants.rs` (independent model with a bitwise CRC-32 and its own kind table; every iff asserted both ways). Include the shared file from `tests/fuzz_layout.rs` as for JPEG.
5. **Seeds** (at least 24, named in the tests) from `examples/fuzz_seeds.rs`, using the same builders as the tests; update every pin; existing seeds stay byte-identical.
6. **Wiring.** `fuzz/Cargo.toml`, `ci.yml` smoke matrix, justfile/app.just lists, README (8 fuzz targets, the PNG walker line, known limits unchanged), CHANGELOG Unreleased, `lib.rs` docs. Keep the README claim honest: PNG is walked, metadata not yet read, `exif-audit` still reports PNG unsupported until SPEC-032.
7. **Gates.** `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just fuzz-smoke png_chunks` (60 s), `just forbid-unsafe`, `just cost-audit`.

## Expected Deliverables

Commits on the branch; every acceptance criterion met or listed as not met with the reason; spec acceptance ticks; the handback filled.

## Out of Scope

Reading chunk contents, ExifTool tests, audit wiring, writer, any change to JPEG or TIFF behaviour, any change to DEC-017 (raise it in the handback instead), PR creation.

## Return Criteria

Fill the `handback:` front matter (including a real `tokens_total`) AND the `## Handback` section; set `handoff.status` and `handback.status` to `completed`. If you cannot complete: say exactly what is done and what blocked you, set both to `rejected`/`blocked`, and still report tokens.

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

- **Branch / PR:** `claude/spec-028-png-walker` (cut from `claude/spec-028-design`, aaa3c12); no PR, not pushed.
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** yes, all 13 ticked. One handoff item not done: the CHANGELOG `[Unreleased]` entry (see Deviations).
- **For `verify`:** n/a

### Cost self-report

- **Tokens (total):** null: this subagent run exposes no token counter; the orchestrator should use the harness `subagent_tokens` figure.
- **Estimated USD:** null
- **Duration (minutes):** not measured
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none. DEC-017 unchanged.
- **Deviations from spec:**
  - CHANGELOG `[Unreleased]` entry NOT added. `tests/release_layout.rs::changelog_and_readme_carry_the_release` requires `[Unreleased]` to be empty after the alpha cut, so any entry fails `just test`. Needs a decision: relax that pin (it will fail for every future spec that files a changelog entry) or leave the entry to the next release spec. README is updated (8 fuzz targets, the PNG line).
  - Seeds: 33 (spec asks at least 24). No `many-chunks-over-cap` seed (it would be about 786 KB); the caps are covered by `tests/png_walk.rs` and the 1,100-chunk issue-cap seed.
  - Existing exhaustive `Error` matches in three unit-test modules (`src/jpeg/mod.rs`, `src/jpeg/metadata.rs`, `src/tiff/mod.rs`) gained arms for the eight new variants. Test code only; no JPEG or TIFF behaviour changed.
  - `Error::TooManyIssues` is reused for the PNG issue cap (DEC-017 lists no new variant for it), so its Display still says "JPEG walk reached the limit of N issues". Left as is (out of scope to change JPEG text); worth a one-word generalisation later.
  - Provenance: a prose paragraph added to `docs/provenance-ledger.md` (class 1, net-new), as for the JPEG walker; no table row.
  - Pins updated for the new target: `tests/fuzz_layout.rs` (ci.yml matrix string, app.just list string, three places), `app.just`, `ci.yml`.
- **Policy choices where DEC-017 is silent (please review):**
  - Issue order within one chunk: `BadChunkType`, `MissingIhdr` (first chunk only), `BadIhdr`, `BadChunkCrc`.
  - A chunk whose own issue trips the issue cap is not recorded (the terminal `TooManyIssues` replaces that issue). So the invariant "a recorded chunk with `crc_ok() == false` has a `BadChunkCrc` issue at its offset" always holds.
  - `TooManyChunks` fires when a further whole, well-formed chunk would be the 65,537th (a truncated one gets `Truncated` instead). Exactly 65,536 chunks is clean.
  - `Truncated { at, len }`: `len` is 12 when fewer than 12 bytes remain, else the chunk's full size (12 + declared length). `BadChunkLength` carries the declared length; `BadIhdr` carries the length; `NotPng` carries the 8 bytes.
  - Zero chunks at a clean end of input (signature only): `MissingIhdr` then `MissingIend`, both at the end offset (8).
  - `BadIhdr` applies to any `IHDR` chunk, not only the first. A second `IHDR` of length 13 is no issue.
  - The unsafe-lint gotcha: the CRC table is built without indexing (`split_first_mut` in a `const fn`), because `indexing_slicing` is denied crate-wide and `#[allow]` is forbidden.
- **Follow-up work identified:**
  - Decide the CHANGELOG pin (above).
  - SPEC-029 can use `Png::trailer()` and `Chunk::range()`; the trailer is not walked.

### Gates

`just lint` ok; `just lint-no-allow` ok; `just forbid-unsafe` ok; `just lint-fuzz` ok; `just test` all green (0 failures; 21 tests in `tests/png_walk.rs`, 4 new in `tests/fuzz_layout.rs`, 3 CRC unit tests); `just validate` ok; `just cost-audit` ok; `just fuzz-smoke png_chunks` 60 s, 956,642 runs, no crash. `cargo run --example fuzz_seeds` leaves every pre-existing seed byte-identical (git shows no change to them). `fuzz/Cargo.lock` is rewritten by cargo on every fuzz build (exif-core 0.0.0 to 0.1.0-alpha.1); I restored it and did not commit it.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The CHANGELOG instruction against the release-layout pin, and the issue-cap interaction (whether the chunk that trips the cap is recorded).

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — That `indexing_slicing` also bites a `const fn` table build, and the `tests/release_layout.rs` Unreleased-must-be-empty pin.

3. **If you did this task again, what would you do differently?**
   — Run `cargo clippy --lib` with the `-F` lints right after writing the first module, and grep the pins in `tests/release_layout.rs` before touching CHANGELOG.
