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
  id: HANDOFF-020
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-010

project:
  id: PROJ-001
  stage: STAGE-002
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number
# from your own interface:
#   Claude Code   → run `/cost`
#   API           → the `usage` object (input + output, summed)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: null                     # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-020: Verify Exif and located metadata regions from JPEG

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-010` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec (API, extraction rules and named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-010-exif-and-located-metadata-regions-from-jpeg.md`; **DEC-006** incl. its SPEC-010 clarifications; DEC-001, DEC-002, DEC-004, DEC-005
- **Build handoff and Handback:** `HANDOFF-019-build-exif-and-located-metadata-regions-from-jpeg.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`)
- Code under review: `git diff 0052025..HEAD` (build commit `9239b87`); main new file `src/jpeg/metadata.rs`, shared harness `fuzz/shared/jpeg_metadata_invariants.rs`

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `just lint-fuzz`, `just lint-no-allow`, `cargo test` on stable and 1.90.0, `-F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`, `just oracle-record` empty diff). Run `just fuzz-smoke jpeg_segments` for 60 s and record executions and coverage.
- **Read the extraction against the spec rules line by line**: Exif signature and `Container::parse` hand-off (payload[6..]); primary vs `ExtraExif`; the 16-block cap; fatal TIFF error kept on the block, JPEG layer still clean; extended XMP (35-byte signature then 40 header bytes), ICC chunk consistency (one aggregate issue, the builder's `at` rule), IPTC `8BIM` resource walk (id 0x0404, even padding, the builder's resource-offset `at`, last resource may omit its pad), JUMBF `JP`+En+Z; `issues()` layering and order; `is_clean`; the walker halt interaction (nothing recorded after a halt; Exif blocks and regions still reported). Hunt for: arithmetic overflow, out-of-range indexing, unbounded loops or allocation (every resource walk must advance), panic-capable constructs in non-test code, any issue recorded outside the single bounded path, a quadratic blow-up (many segments each scanned repeatedly).
- **Judge two builder policy picks:** (1) ICC `at` when chunks are individually consistent but the count of chunks differs from `count` (first chunk's offset); (2) IPTC `RegionTooShort`/`StrayBytes` use the resource's input offset, `need`/`have` from the resource start. Say whether each is defensible.
- Check the walker change: bad length skips four bytes; the SPEC-009 test updated, nothing else weakened. Diff the SPEC-009 tests for any deletion or loosening.
- **Test the testers.** Redo the seven required mutations (a)-(g); write mutants of your own the builder did not try (e.g. Exif signature compare off by one; `payload[6..]` vs `[5..]`; ExtraExif at the 16 boundary `>` vs `>=`; first-Exif-primary replaced by last; extended-XMP header length 39/41; ICC `seq == 0` accepted; IPTC pad byte ignored; IPTC id compare wrong; JUMBF En/Z byte order swapped; `is_clean` ignoring a block's issues; `issues()` swapping layer order; halt check removed). Report which the suite catches and which survive; the builder says one equivalent mutant survives (unreachable issue-bound guard in `Extract::record`) — judge that. Redo at least three fuzz-invariant plants including one against the real libFuzzer target; confirm iff-invariants are both-directional.
- Container independence: verify the claim yourself with a bare TIFF blob vs the same blob inside a JPEG, including a damaged-GPS-IFD case and a fatal-TIFF case.
- Seeds: the existing 50 are byte-identical; the 13 new ones deterministic (regenerate twice) and each exercises the shape its name claims (inspect with `xxd`). Check the cap constants against the spec formula.
- Check `src/tiff/mod.rs` edit is test-module only; `[dependencies]`/`[dev-dependencies]` empty; `scripts/`, `decisions/`, the oracle code and records, and irradiance untouched; CI workflow unchanged.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-011 work, running ExifTool, CI wiring

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

- **Branch / PR:** [link]
- **Completed at:** YYYY-MM-DD
- **All acceptance criteria met?** yes/no (if no, explain)
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** <real number, or null + why>
- **Estimated USD:** <number, or null>
- **Duration (minutes):** <estimate>
- **Source of the number:** `/cost` | API `usage` | harness report | none available

### Drift and new artifacts

- **New decisions emitted:**
  - `DEC-NNN` — <title> (if any)
- **Deviations from spec:**
  - [list]
- **Follow-up work identified:**
  - [any new specs that should be added to the stage's backlog]

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — <answer>

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — <answer>

3. **If you did this task again, what would you do differently?**
   — <answer>
