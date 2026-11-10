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
