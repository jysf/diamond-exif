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
  id: HANDOFF-018
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-009

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

# HANDOFF-018: Verify the JPEG segment walker

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-009` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-009-jpeg-segment-walker.md` (walk rules and named tests are normative); **DEC-006** (note: the orchestrator reworded its "suppresses" sentence after the build's fuzz finding; judge that wording against the code)
- **Build handoff and Handback:** `HANDOFF-017-build-jpeg-segment-walker.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`), DEC-001, DEC-002, DEC-004
- Code under review: `git diff dcdd328..HEAD` (build commit `04c5711`)

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt on both graphs, `just lint`, `just lint-fuzz`, `just lint-no-allow`, `cargo test` on stable and 1.90.0, `-F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`, `just oracle-record` empty diff). Run `just fuzz-smoke jpeg_segments` for 60 s and record executions and coverage; also let `tiff_ifd` run briefly to confirm the `fuzz-smoke [TARGET]` recipe still serves both.
- **Read the walk against the numbered rules line by line**: header, stray runs, fill, `FF 00`, standalone markers, bad length, truncation (segment omitted), SOS entropy scan (what ends it: `FF` followed by not `00`, not `D0..D7`, not `FF`), progressive scans, `after_scan`, end-of-walk `MissingSos`/`MissingEoi` ordering and the halt interactions, caps, `scan_region`, `trailer`. Hunt for: a path that records an issue outside the single `record` function; a non-advancing cursor (find an input where `p` does not advance, or prove none exists); arithmetic that could overflow or index out of range; unbounded allocation; any panic-capable construct in non-test code.
- **Judge the spec/builder gaps**: (1) `BadSegmentLength` then `StrayBytes { at: 4, len: 2 }` for the two length bytes (two issues for one fault; is that acceptable or noise?); (2) a fill-byte run reaching EOF reports `Truncated` at the last `FF`; (3) a lone `FF` at the end of scan data is not an issue; (4) `trailer()` with no EOI returns `data[len..]`; (5) the cap-inside-end-of-walk behaviour (`[.., MissingSos, TooManyIssues]`).
- **Test the testers.** Redo the five boundary mutations and confirm each named test goes red; weaken or change several tests and check they fail; write mutants of your own the builder did not try (for example: SOS scan ends at `FF D0` too, RST not skipped; `FF FF` fill not skipped inside a scan; `Sof` classification includes `0xC4`; `after_scan` set on the SOS itself; EOI offset off by one; `L` read little-endian; segment payload range off by one; stray-run merging broken so two runs become one) and report which the suite catches and which survive. Redo at least three fuzz-invariant plants including one against the real fuzz target; confirm the iff-invariants are both-directional.
- Check seeds: the 29 TIFF seeds byte-identical; the 19 JPEG seeds deterministic (regenerate twice, same hashes) and each seed actually exercises the shape its name claims (inspect a few by hand with `xxd`).
- Check the CI workflow change: the `fuzz-smoke` matrix parses and runs both targets; no other job changed.
- Check the `src/tiff/mod.rs` edit is test-module only (exhaustive error guard) and that `[dependencies]`/`[dev-dependencies]` are empty, `scripts/`, `decisions/` (apart from the orchestrator's DEC-006 rewording), the oracle code and records are untouched, and irradiance is untouched.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-010/011 work, running ExifTool (not needed here), CI wiring beyond reading the workflow

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
