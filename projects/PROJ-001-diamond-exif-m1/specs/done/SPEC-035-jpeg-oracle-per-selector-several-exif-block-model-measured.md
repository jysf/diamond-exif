---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-035
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: S                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: punch-list  # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.
  verify_rounds: [punch-list]  # every verify outcome, oldest first, appended by
                                   #   `just advance-cycle` each time the spec leaves verify.
                                   #   verify_verdict keeps only the LAST one, so a spec sent
                                   #   back and then approved read as "approved": irradiance's
                                   #   field showed 1 pushback where its handoffs showed 5.

project:
  id: PROJ-001
  stage: STAGE-008
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-006]                 # [DEC-NNN, DEC-MMM]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-011, SPEC-034]             # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: []                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-008's <capability>". Optional; null is acceptable.
value_link: null

# Self-reported AI cost per cycle. Each cycle (design, build, verify,
# ship) appends one entry to sessions[]. Totals are computed at ship.
# Record a REAL tokens_total for metered cycles (build/verify) — the agent
# that runs the cycle writes it from its own interface (/cost, the API
# usage object, or its tool's report). Only un-metered main-loop cycles
# (design/ship) may be null-with-note. `just cost-audit` enforces this on
# shipped specs. See AGENTS.md §4 and docs/cost-tracking.md. interface:
# claude-code | claude-ai | api | ollama | other.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: 350000
  sessions:
    - cycle: build
      handoff: HANDOFF-067
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 175052
      estimated_usd: 1.16
      duration_minutes: 40
      recorded_at: 2026-11-22
      notes: "Harness subagent_tokens (175,052) at 6.60 USD per Mtok; the builder self-reported 6,303,399 counting cache reads, which is not spend, so the orchestrator replaced it."
    - cycle: verify
      handoff: HANDOFF-068
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 178261
      estimated_usd: 1.18
      duration_minutes: 45
      recorded_at: 2026-11-22
      notes: "Harness subagent_tokens (178,261) at 6.60 USD per Mtok; the verifier self-reported 5,723,009 counting cache reads, replaced by the orchestrator."
  totals:
    tokens_total: 353313
    estimated_usd: 2.34
    session_count: 2
shipped_at: 2026-11-22
---

# SPEC-035: JPEG oracle: per-selector several-Exif-block model, measured

## Context

SPEC-034's verify found that ExifTool 13.55, given several Exif blocks, does not
simply let the later block win: for four selectors (`IFD0:ImageWidth`,
`IFD0:ImageHeight`, `IFD0:Orientation`, `IFD0:XResolution`) the FIRST WebP
`EXIF` block wins and for the other sixteen the last does. The JPEG oracle,
written in SPEC-011, still models "later block wins" for all 20 selectors
(`read_ours_jpeg`, `tests/support/oracle.rs`), from fixtures that only varied
Make and Model. The WebP fix round saw the same split on two JPEGs it tried, so
the JPEG model is probably wrong the same way. It is test harness only: the
library keeps "first block is primary" (DEC-006) and no `src/` change is in
scope. This is an oracle-correctness follow-up in STAGE-008's parity thread;
it blocks nothing, but SPEC-032 (audit wiring) will lean on the per-selector
rule and the JPEG audit already inherits the wrong model.

## Goal

Replace the JPEG oracle's "later block wins" rule with a per-selector rule that
is MEASURED against ExifTool on two full Exif blocks that differ in all 20
selectors, in both orders, and pinned by a test that reads all 20 selectors.
Every existing JPEG, MPO and seed comparison stays clean or its divergence
stays recorded.

## Inputs

- `tests/support/oracle.rs` `read_ours_jpeg` (about line 824), `read_ours_webp`
  and `WEBP_FIRST_BLOCK_WINS` (about line 1262): the WebP model to mirror
- `tests/support/oracle_webp.rs` `tiff_full(v)` (about line 132) and the
  `exif-full-one-two` / `-two-one` fixtures: the shape to reuse
- `tests/support/oracle_jpeg.rs`: fixture table, `exif_app1`, `clean`, builders
- `tests/jpeg_oracle.rs::multi_exif_is_compared_against_the_union_with_the_later_block_winning`
  and `tests/webp_oracle.rs::several_exif_model_is_per_selector_and_pinned`
- `guidance/questions.yaml` `exiftool-several-exif-segments` and
  `several-exif-segments` (their notes say "later wins")
- ExifTool 13.55 (`exiftool -T -n -s3`), run, never read

## Outputs

- **Modified:** `tests/support/oracle_jpeg.rs` adds `jpeg-exif-full-one-two` and
  `jpeg-exif-full-two-one` (two Exif APP1s from `tiff_full(1)` and
  `tiff_full(2)`, made visible to the JPEG builders without importing PNG or
  WebP builders beyond what `oracle_webp` already shares; move `tiff_full` to a
  shared module only if that is the smaller change).
- **Modified:** `tests/support/oracle.rs`: a `JPEG_FIRST_BLOCK_WINS` list set
  FROM THE MEASUREMENT (expected to be the same four, but the measurement
  decides) and `read_ours_jpeg` using it; the doc comment states the measured
  rule and drops the "later block wins" claim.
- **Modified:** `tests/jpeg_oracle.rs`: a pinned test mirroring the WebP one;
  the old multi-exif test keeps its Make/Model cases.
- **Created:** records `tests/oracle-fixtures/exiftool/jpeg-exif-full-one-two.txt`
  and `-two-one.txt`, via `just oracle-record` (idempotent; no other record
  changes).
- **Modified:** `guidance/questions.yaml`: correct the two notes that say
  "later wins" (state the per-selector rule, cite SPEC-035).
- No `src/` change, no new DEC, no CHANGELOG entry.

## Acceptance Criteria

- [x] The two new fixtures are generated by the one table, uniquely named, and
      their records are committed with a shown-red replay-and-perturb proof.
- [x] For each of the 20 selectors, ExifTool's value on both fixtures is the
      first block's or the last block's, the same winner in both orders; the
      test asserts it, and asserts the list equals `JPEG_FIRST_BLOCK_WINS`.
- [x] `read_ours_jpeg` agrees with ExifTool on all 20 selectors on both
      fixtures (`assert_clean`).
- [x] Every other JPEG, MPO, seed and strip comparison is unchanged: no new
      divergence entry, no record diff outside the two new files.
- [x] A mutant that reverts to "later wins for all 20" fails the new test
      (shown by a mutated copy: control first, one injection, marker must not
      leak) and `just mutation-oracle` is still green.
- [x] `just lint`, `just test`, `just forbid-unsafe`, `just validate`,
      `just cost-audit` green.

## Failing Tests

- **`tests/jpeg_oracle.rs`**
  - `several_exif_model_is_per_selector_and_pinned` (JPEG) asserts the above
    on `jpeg-exif-full-one-two` and `-two-one`, live-gated like the siblings,
    replay always.
  - `jpeg_per_selector_model_red_proof` asserts that the all-later-wins
    reading differs from ExifTool on exactly the first-wins selectors.

## Non-Goals

- Changing the library's primary-block policy or exposing a merged view
  (`several-exif-segments` stays open).
- The 17th-block cap divergence and the other recorded JPEG divergences.
- PNG (reads the first `eXIf` only) and WebP (already per selector).
- The Make-as-SHORT limit (L1) beyond noting whether JPEG shows it.

## Notes for the Implementer

- Measure first: build the two fixtures, run ExifTool, look at all 20 values,
  then write the list. If JPEG differs from WebP (not the same four), the
  measurement wins and the doc comment says so.
- A block whose value for a selector `read_one` cannot read is skipped by the
  model; check whether JPEG hits the same Make-as-SHORT limit and say so in the
  doc comment either way.
- Budget: about 100 tool calls or 250k context. Stop and hand back if either
  is near; do not loop on the red-proof.

---

## Reflection

1. **What would I do differently next time?**
   — Ask for the Make-as-SHORT measurement as an explicit test, not a doc note: the builder skipped it and the verifier found the JPEG model has the same limit on both the last-wins and first-wins sides. The WebP fix round's per-selector grid should have been carried into the JPEG spec at design.

2. **Does any template, constraint, or decision need updating?**
   — No DEC. The two "later wins" notes in questions.yaml are corrected. Process: builders and verifiers self-report cache-read-inclusive token counts (6.3M and 5.7M for work the harness metered at about 175k each); the orchestrator replaced them. The handoff template's "transcript" basis invites this.

3. **Is there a follow-up spec I should write now before I forget?**
   — No. The L1 limit is now documented for JPEG, WebP and the audit notes. SPEC-032 consumes the per-selector rule.

4. **Where was the worst defect caught?** — verify (stale doc comments and the skipped Make-as-SHORT measurement; nothing in code or records).

5. **What can a user do now that they couldn't before?** — none (test harness only); the JPEG oracle now matches ExifTool on all 20 selectors for several Exif blocks, confirmed on a 19-shape grid.
