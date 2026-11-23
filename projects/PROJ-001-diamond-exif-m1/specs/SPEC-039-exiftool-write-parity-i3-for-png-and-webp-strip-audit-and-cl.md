---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-039
  type: story                      # epic | story | task | bug | chore
  cycle: design                    # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.
  verify_rounds: []                # every verify outcome, oldest first, appended by
                                   #   `just advance-cycle` each time the spec leaves verify.
                                   #   verify_verdict keeps only the LAST one, so a spec sent
                                   #   back and then approved read as "approved": irradiance's
                                   #   field showed 1 pushback where its handoffs showed 5.

project:
  id: PROJ-001
  stage: STAGE-010
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: []                    # [DEC-NNN, DEC-MMM]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-037, SPEC-038, SPEC-032]             # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-037, SPEC-038]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-010's <capability>". Optional; null is acceptable.
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
  tokens_estimate: null
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-039: ExifTool write parity (I3) for PNG and WebP strip, audit and CLI switch-over

> **OUTLINE, `cycle: frame`.** This spec exists so its ID is stable and
> siblings can declare `depends_on`. Capture **scope** (Context / Goal /
> Non-Goals) and **dependencies** only: the *approach* is designed just in time
> when this moves to `design`. Do not pre-design it here.

## Context

The strip output of PNG and WebP is only trusted once ExifTool agrees (I3, the write-parity layer of the design): the stripped file read by ExifTool prints none of what the policy removed and everything else unchanged. The audit still says PNG and WebP are `not strippable`, and the CLI has no way to strip them; this spec flips that honestly.

## Goal

ExifTool write parity (I3) for PNG and WebP strip on the fixtures and seeds, for every policy, with the comparator red-proofed and mutated-copy red-proofs of the writer; then the switch-over: `ContainerKind::strippable()` true for PNG and WebP (the single helper from SPEC-032), preview dispositions `Removed`, CLI and README, goldens updated deliberately.

## Non-Goals

- A release (its own spec afterwards); `set` operations; HEIC.

## Notes for the Implementer

Carried from SPEC-037 and SPEC-038 (verifies, 2026-11-23): (1) the gated live ExifTool checks `tests/strip_png_live.rs` and `tests/strip_webp_live.rs` are the seed of the formal I3; extend them, do not rewrite them. (2) WebP has a completeness sweep (`tests/strip_webp_sweep_live.rs`: every `exiftool -listx` four-character id and the table's codes as one chunk each, in 8 payload shapes, one ExifTool run; fails on any code that prints a non-structural tag and is not in the table); PNG has no twin, and PNG shipped with only the builders' own chunk list. Build it: move `listx_ids`, `parse_listing`, `new_tags`, `unexplained` and the scratch directory into `tests/support/sweep.rs`; one PNG per candidate chunk type placed between `IHDR` and `IDAT`, CRC-correct, with the same payload shapes plus zlib and `Exif\0\0` shapes and a keyword-prefixed text shape for `tEXt`/`zTXt`/`iTXt`; judge against the PNG table, the rule codes and a pinned structure-tag list; re-run the 16 case spellings for every code that prints. The sweep's known limit (types `-listx` does not list and the table lacks) is a stated residual. (3) Both formats: `ContainerKind::strippable()` flips to true only per file where the writer has shown it can; the preview then reports `Removed` and `KeptUnread` as for JPEG; goldens change deliberately. (4) Residuals to carry into the README and the preview wording: a C2PA `caBX`/`C2PA` manifest keeps Exif and creator assertions under every preset but `Everything`; XMP with GPS keeps `Composite:GPS*` under `Gps`, `GpsAndDevice`, `AllPii`; carriers and every WebP `LIST` are removed under every preset. (5) The two formats refuse differently from JPEG for RIFF `WAVE`, `AVI `, `RIFX` and lower-case `webp` (`NotJpeg`, an old error name): consider a neutral name.


The honest-preview rule (DEC-020 clause 6) is reversed only here, and only for what the writer has shown it can do: a policy the writer refuses for a file stays `not strippable` for that file. Reuse the JPEG I3 apparatus (`tests/strip_oracle`-style) and the SPEC-031/034 fixture tables; measure the hostile edges before design, not at verify (signal `design-must-measure-hostile-edges-not-only-the-happy-keys`).

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`: `type: lesson` (with its N-count) for a recurring
   coding pattern, `type: process-debt` for tooling/process friction. A close
   then forces the decision. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(Runtime/operational defects — the escape-prone class — only exist once the
   artifact meets its real host. `escaped` here is a signal to strengthen the
   §12 behavioral pre-flight for that surface.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — <answer | none>
