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
