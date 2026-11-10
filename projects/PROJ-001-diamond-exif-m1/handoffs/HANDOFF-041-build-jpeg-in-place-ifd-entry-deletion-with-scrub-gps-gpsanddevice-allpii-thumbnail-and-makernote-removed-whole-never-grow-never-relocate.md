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
  id: HANDOFF-041
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-10
  status: pending                  # pending | accepted | completed | rejected

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
  status: null                     # completed | blocked | rejected
  agent: null                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-041: JPEG in-place Exif entry deletion with scrub (SPEC-021)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-021` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (rules 1 to 14, library changes, fixtures and leak inputs, oracle, fuzz and seeds, the named tests and the 30 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-021-jpeg-in-place-ifd-entry-deletion-with-scrub-gps-gpsanddevice-allpii-thumbnail-and-makernote-removed-whole-never-grow-never-relocate.md`
- **DEC-013 with its 2026-11-10 amendment (normative: part B is the entry-editing contract this spec implements; part A ratifies SPEC-020's refusal order and Multi-Picture refusals; read it, do not edit it):** `decisions/DEC-013-writer-foundation-fail-closed-removal-and-patch-plan.md`, with DEC-012, DEC-009, DEC-008 (the tag table), DEC-005 (IFD kinds), DEC-001 (tolerant walk).
- **The shipped writer you extend:** `src/write/{mod,plan,jpeg,verify,verify_tests}.rs`, `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/support/{strip_shapes,strip_records}.rs`, `fuzz/shared/rewrite_invariants.rs`, `fuzz/fuzz_targets/rewrite_jpeg.rs`, `examples/{strip_corpus,fuzz_seeds,oracle_record}.rs`, and the SPEC-020 Handback in `projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-039-*.md` (mutant table, ExifTool 13.55 group facts, deviations).
- **Evidence:** `spikes/done/SPIKE-001-*.md`; questions `writer-in-place-delete-and-scrub`, `strip-keeps-orientation` (stays open, out of scope), `writer-stray-bytes`, `writer-mpf-attribute-fields` in `guidance/questions.yaml`.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; macOS `sed -i ''`; never measure exit status through a pipe; in zsh never name a variable `path`; do not leave the shell in a subdirectory).
- Code to read: `src/tiff/mod.rs` (`Container`, `Ifd`, `Entry`, `value_at`, how the walk records issues and dedups IFD offsets), `src/audit/{mod,category}.rs`, `src/jpeg/metadata.rs` (`exif_blocks`).
- **Lessons that cost follow-ups before (SPEC-019 and SPEC-020):**
  - "Unreadable means refuse" is written as "the walk reported any issue", never as one counter. SPEC-020's verifier found `strip(Gps)` returning `Ok` with coordinates left in because only `failed_blocks` was tested. For every refusal rule, write the leak input first (the spec lists 14) and ask which counter your code tests.
  - Every invariant a fuzz guard can find must also have a deterministic unit test; a guard is only as strong as the seeds that reach it (seeds for every policy-byte family, not only one); plants vary in kind and run in both directions.
  - Measure ExifTool's behaviour on the fixture **before** writing the declared tag table; an "unchanged" claim is checked against the rule that could move it.
  - Surviving mutants in SPEC-020 were all "a check that is skipped or compares the wrong thing": for each new check write a test whose plan is wrong in exactly that way.
  - `rustfmt --check --edition 2021 fuzz/shared/*.rs` is part of `just lint`; a mutant's revert must leave `git diff` empty (`git checkout`, never by hand).
  - The tolerant walk may differ from the harness's naive reader on edge cases; fail closed in the library, and in the harness only "library Ok and naive unclean" is a failure.

Constraints: `panic-free-library` (five-lint deny set: no unchecked `+`, `-`, `*`; every offset and length is untrusted: `checked_*`, `get(..)`, never index), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (extend `rewrite_jpeg`), `test-before-implementation`, `oracle-must-be-shown-red`, `exiftool-never-linked`, `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New: `src/write/exif.rs`, tiff accessors, new fixtures in `strip_shapes.rs`, extended oracle tests and records, extended harness and at least 60 new seeds, extended `strip_corpus`. `NeedsEntryEdit` is removed and replaced by `Uneditable`; the SPEC-020 tests that expected it are rewritten (report each). Under whole and none policies the outputs are byte-identical to SPEC-020's (rule 1).
- Order as in the spec's Notes for the Implementer: tiff accessors; `exif.rs` on one hand-built IFD; keep set and `SharedBytes`; GPS child removal; thumbnail child removal; planner integration; `Entries` and `Scrubbed` checks; **measure ExifTool (including rule 9's emptied IFD) before writing the declared tables**; harness, seeds, CI; the 30 mutants; the local corpus.
- The expected bytes in tests come from the fixture builder's **layout record**, not from the library or the walk.
- The local corpus re-run: `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0` holds the user's real photos. **Read it in place only; copy nothing into the repo; write no file derived from it (no temp file, no saved ExifTool output); print and record only counts and 8-hex short hashes, never values, content or paths.** ExifTool on a real output only through a pipe (`--emit` to stdout piped to `exiftool -`), one file at a time.
- A doubt about a DEC-013 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml` with the input that raised it; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), `just fuzz-smoke rewrite_jpeg`, `jpeg_segments`, `audit`, `audit_render`, `cli_args` (60 s each; record runs and cov), `cargo build --release --locked --bin exif-audit`, `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`, and the rest of the spec's gate list.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-041); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Keeping Orientation under `All`/`Everything`; TIFF and DNG; XMP or IPTC editing; MakerNote or thumbnail parsing; a write CLI; PNG, WebP, HEIC; changing the audit, the presets, the report schema, exit codes or the CLI
- Pushing, PRs, cycle advancement, SPEC-022
