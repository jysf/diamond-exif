---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-040
  type: story                      # epic | story | task | bug | chore
  cycle: frame                     # frame | design | build | verify | ship
  blocked: false
  priority: high                   # critical | high | medium | low
  complexity: S                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
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
  stage: STAGE-011
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-006, DEC-008, DEC-009, DEC-012, DEC-013]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: []                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-011's <capability>". Optional; null is acceptable.
value_link: "Makes STAGE-011's 'Everything removes everything' true for JFIF thumbnails."

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

# SPEC-040: DEC-012 amendment: JFIF APP0 thumbnail is a Thumbnail region, audited and stripped

## Context

STAGE-011, first and only backlog item. PROJ-001.

An external code review (ChatGPT, against `fd94d4f`, 2026-11-23, finding 3)
reported that a JPEG whose APP0 `JFIF\0` segment carries the optional
uncompressed thumbnail gets `clean=true` with zero findings, and that
`strip(Everything)` returns identical bytes. ExifTool independently reports
and extracts the thumbnail as `ThumbnailTIFF`.

The architect confirmed the cause in the code: `src/jpeg/metadata.rs:821`
skips APP0 `JFIF\0` as benign, which is exactly what DEC-012's benign list
says. DEC-012 already moved JFXX to `OtherApp` because it "can carry a
thumbnail", but it missed that plain JFIF can carry one too. So the fix starts
with a DEC-012 amendment, not just code.

Layout (JFIF 1.02): `JFIF\0`, version (2), units (1), Xdensity (2),
Ydensity (2), Xthumbnail (1), Ythumbnail (1), then `3 × Xthumbnail ×
Ythumbnail` bytes of RGB. Most encoders write 0×0, so a JFIF segment with no
thumbnail must stay benign: no finding, not removed.

Related: PATCH-001 fixes the same review's findings 1, 2 and 4 (writer
hardening). The two are independent.

## Goal

A JFIF APP0 segment with a non-zero thumbnail is reported by the audit as a
`Thumbnail` region and finding, and every strip policy that targets
thumbnails (`Everything` included) leaves no thumbnail bytes behind,
confirmed by the self-check and by ExifTool.

## Open design questions (resolve when this advances to design)

1. **Remove the whole APP0 segment, or rewrite it?** Removing the segment
   fits DEC-013 rule 3 (the segment is the unit of removal) and is simplest.
   But it also drops the density/aspect fields a decoder might use. Rewriting
   to a 0×0 thumbnail and truncating keeps them, but it is a new kind of
   in-segment edit that DEC-013 doesn't have yet. Measure what ExifTool
   `-all=` / `-ThumbnailImage=` do to JFIF before choosing.
2. **A malformed thumbnail** (the declared `3·X·Y` runs past the segment):
   is that a typed issue plus refusal, or treated as a thumbnail anyway?
   DEC-001's tolerant-walk rule suggests issue + region-of-what-exists.
   Decide in the DEC amendment.
3. **Category and severity:** confirm that DEC-008 `Thumbnail` applies as-is,
   and decide whether the finding text needs to say it is a JFIF thumbnail
   rather than an Exif IFD1 thumbnail.
4. **Which presets target it?** Check DEC-009's preset table. At minimum
   `Everything` and any preset that already removes the IFD1 thumbnail.

## Inputs

What the implementer will read or consume.

- **Files to read:** `path/to/file.ext` — why
- **External APIs:** <name, docs link, auth requirements>
- **Related code paths:** `src/some/module/`

## Outputs

What the implementer will produce.

- **Files created:** `path/to/new.ext` — purpose
- **Files modified:** `path/to/existing.ext` — what changes
- **New endpoints / functions / components:** <names and signatures>
- **New flags / options:** each flag's accepted values **and its default** — an
  unstated default makes the implementer guess.
- **Database changes:** <migrations, if any>

## Acceptance Criteria

Testable outcomes. Each must map to at least one test. Cover happy
path, error cases, edge cases.

- [ ] Criterion 1 (testable)
- [ ] Criterion 2 (testable)
- [ ] Criterion 3 (testable)

## Failing Tests

Written during the **design** cycle, BEFORE handoff. The implementer's
job in **build** is to make these pass.

- **`path/to/test.file`**
  - `"test description 1"` — asserts: ...
  - `"test description 2"` — asserts: ...

## Non-Goals

Explicit scope limits. If the implementer thinks any of these need to
happen, they should create a new spec (in this stage's backlog), not
expand this one.

- ...

## Notes for the Implementer

Gotchas, style preferences, reuse opportunities. Keep short — the full
context graph lives in the handoff file.

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
