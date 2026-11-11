---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-022
  type: story                      # epic | story | task | bug | chore
  cycle: design  # frame | design | build | verify | ship
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

project:
  id: PROJ-001
  stage: STAGE-004
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-008, DEC-009, DEC-010]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-021]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-004's <capability>". Optional; null is acceptable.
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

# SPEC-022: TIFF and DNG in-place strip: never touch strip or tile offsets, SubIFDs, TIFF writer fuzz target

> **Designed (see Design below); originally an outline at `cycle: frame`.** This spec exists so its ID is stable and
> siblings can declare `depends_on: [SPEC-022]`. Capture **scope** (Context /
> Goal / Non-Goals) and **dependencies** only — the *approach* is designed
> just-in-time when this moves to `design`. Do not pre-design it here.

## Context

TIFF and DNG are the same IFD engine with a different container: no segment layer, and raw image data referenced by strip or tile offsets that must never move (a DNG's raw SubIFD can be tens of megabytes, as in SPIKE-001's corpus).

## Goal

`strip` for TIFF and DNG in place (entry deletion and scrubbing, SubIFDs, never touching strip or tile offsets), with a TIFF writer fuzz target and the same invariants and ExifTool oracle.

## Non-Goals

- BigTIFF; a write CLI; `set` operations

---

## Design (SPEC-022, approved by the user; complexity M)

**Goal.** `strip(data, policy)` accepts classic TIFF and DNG (`Sniffed::Tiff`; BigTIFF stays `Unsupported`), rewriting IN PLACE: same length, nothing moves, strip/tile/raw data byte-identical. Same `StripPolicy` presets, same `Stripped` result, same `finish` + self-check path as JPEG.

**Reuse, do not rewrite.** `src/write/exif.rs::plan_block(blob, container, selection)` already plans same-length entry deletion + scrubbing on one TIFF blob and its module doc says SPEC-022 calls it on a bare TIFF. `src/write/jpeg.rs::plan` + `exif.rs::plan_entries` show how a caller builds a `Selection` from the audit and maps blob patches to input coordinates (for a bare TIFF the blob is the whole file, offset 0). `src/write/mod.rs::strip` currently returns `Unsupported{Tiff}` for TIFF: add a TIFF branch (new `src/write/tiff.rs`, mirroring `jpeg.rs`).

**Work items.**
1. `src/write/tiff.rs`: build the `Selection` from the audit + policy spec; call `plan_block`; return the same `Planned` shape `finish` needs. Empty plan still goes through verify.
2. Never-touch set: compute protected byte ranges from every IFD in the walk (including SubIFDs 0x014A and IFD chain pages): strip data (0x0111 offsets / 0x0117 counts), tile data (0x0144 / 0x0145), JPEGInterchangeFormat (0x0201/0x0202) when it is page data not a thumbnail, and any other image-data range. The plan must never zero, rewrite or move them; contact is refused (`UneditableReason::SharedBytes` or the closest existing reason; add a variant only if none fits, with a DEC-013 amendment note). Strip/tile offset and byte-count ENTRIES are never deleted or edited by any policy (they are structural, not privacy).
3. IFD1 in TIFF may be another PAGE, not a thumbnail. Thumbnail removal applies only when IFD1 is a JPEG thumbnail (0x0201/0x0202, no StripOffsets/TileOffsets, as audited); a page IFD is never removed.
4. Structural + DNG core tags are always kept and are not privacy categories (Compression, Photometric, Strip/Tile tags, SamplesPerPixel, BitsPerSample, Orientation, DNGVersion, CFA/color-matrix tags 0xC612..). Check how the audit categorises them; do not invent categories (DEC-008/009 are authority). Where the audit already marks them kept, just assert that in tests.
5. GPS/Device/AllPii inside SubIFDs (DNG raw/preview SubIFDs) and in IFD0/Exif IFD get the same deletion and scrub as JPEG Exif. IFD0 carriers XMP 700, IPTC 33723, ICC 34675, Photoshop 34377: follow what the audit/preset says for their categories; if the preset removes them, delete the entry AND scrub the out-of-line bytes; if the preset keeps them, they stay byte-identical. Check this against the audit as oracle and against ExifTool.
6. `src/write/verify.rs`: its JPEG checks assume a scan region. Add the TIFF variant: protected ranges byte-identical; no finding in a removed category; kept categories unchanged; output walks with no new issue kind; Exif inventory is input minus deleted entries; zeroed ranges read zero; re-strip of output is a no-op. Keep the JPEG path untouched.
7. Fixtures: ONE generator script/function in `tests/support` (e.g. `tiff_shapes.rs`, model on `strip_shapes.rs`, independent of the library): stripped TIFF, tiled TIFF, multi-page TIFF (IFD1 = page), DNG-shaped file with raw SubIFD + preview SubIFD (image data = large sparse/pattern bytes, canaries), GPS in IFD0-pointer, in a SubIFD, Exif IFD device tags, XMP/IPTC/ICC carriers in IFD0, both byte orders. Every fixture records what each byte range is, with canary values.
8. Tests (write them FIRST, see them fail): `tests/strip_tiff.rs` (integration, model on `tests/strip.rs`), `tests/strip_tiff_oracle.rs` (ExifTool `-a -u -G1 -s -t -n` over outputs; records in `tests/oracle-fixtures/exiftool-strip-tiff/`; live tests skip loudly without ExifTool; red-proof the oracle comparator by a mutated expectation and say so in the handback; extend `just oracle-record`/CI diff to the new dir). Include: never-touch property (strip/tile/raw bytes identical, length identical, StripOffsets values identical), hostile cases (offset into an IFD table, strips overlapping a GPS value, SubIFD cycle, truncated file, BigTIFF refused, JPEG path unchanged).
9. Fuzz: `fuzz/fuzz_targets/rewrite_tiff.rs` modelled on `rewrite_jpeg.rs` (input = policy byte then TIFF; invariants: no panic; Ok output has same length and every protected range identical; output re-strip is a no-op; refusal is typed). Seeds via `examples/fuzz_seeds.rs` into `fuzz/seeds/rewrite_tiff/`; `just fuzz-smoke rewrite_tiff` 60 s clean; add to CI smoke list and the `fuzz_layout` test as the existing targets are.
10. Gates: `just` lint/fmt/test/deny/deny-fuzz/lint-fuzz/validate as in toolchain brief; a DEC (DEC-014) ONLY if a real decision arises (e.g. page-vs-thumbnail rule, refusal reason), else a DEC-013 amendment note.

**Non-goals.** BigTIFF, write CLI, `set` ops, MakerNote editing, PNG/WebP, anything pixel-decoding, relocating or growing bytes.
**Split trigger.** If you reach the budget with items 1-8 done but 9 undone, commit, write the handback listing exactly what remains, and stop.

---

## Failing Tests

Written first in intent, built in one pass (the planner and tests landed together; red-proofs were run afterwards, see the handoff). Red-proofs: (1) the TIFF branch disabled turns 6 of 8 `strip_tiff` tests red (`Unsupported`); (2) the planner's protected-range keep set emptied turns `hostile_inputs_*` red (the self-check then catches it as `VerifyFailed`, the planner's typed `SharedBytes` is gone); (3) the oracle comparator goes red on four planted faults (`the_comparator_goes_red_on_planted_faults`, no ExifTool needed).

- `tests/strip_tiff.rs` (every shape x 12 policies: length, image data, offsets, canaries, audit, idempotence; GPS and Artist inside a DNG SubIFD; page vs thumbnail IFD1; hostile: strip over a GPS value, strip offset into an IFD table, SubIFD cycle, truncated, unreadable offsets entry; BigTIFF; JPEG unchanged)
- `tests/strip_tiff_oracle.rs` (ExifTool `-a -u -G1 -s -t -n`, 10 records in `tests/oracle-fixtures/exiftool-strip-tiff/`, live + replay + comparator red-proof)
- `fuzz/fuzz_targets/rewrite_tiff.rs` (98 seeds, 60 s clean, 304k runs; `tests/fuzz_layout.rs::ci_fuzz_smoke_list_includes_rewrite_tiff`)

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
