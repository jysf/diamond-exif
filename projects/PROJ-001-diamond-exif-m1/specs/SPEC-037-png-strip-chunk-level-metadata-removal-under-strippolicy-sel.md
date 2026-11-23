---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-037
  type: story                      # epic | story | task | bug | chore
  cycle: build  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
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
  decisions: [DEC-013, DEC-020, DEC-021]                 # [DEC-NNN, DEC-MMM]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-020, SPEC-021, SPEC-032, SPEC-036]             # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-036]                     # e.g. [SPEC-002]

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
  tokens_estimate: 800000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-037: PNG strip: chunk-level metadata removal under StripPolicy, self-verify, rewrite_png fuzz

## Context

DEC-013 gives the writer a foundation (pure function, `Remove` and same-length `Patch`, fail closed, output re-audited) proven on JPEG and TIFF. STAGE-008 and SPEC-036 gave PNG a fuzzed reader, an audit (DEC-020) and a tag-level inventory checked against ExifTool. `write::strip` still returns `Unsupported` for a PNG, and the audit still says PNG is not strippable. **DEC-021** (written at design) carries the decisions below; read it first.

## Goal

`strip(png, policy)` for every `StripPolicy`: whole-chunk removal by the audit's category of each chunk, the `eXIf` rule (all, none, or entry edit with a CRC patch), opaque carriers removed (raw profiles and other chunks ExifTool decodes that the audit does not list), typed refusals, a self-check by re-audit, and a `rewrite_png` fuzz target with a 60 s CI smoke.

## Inputs

- DEC-021, DEC-013 (all parts and amendments), DEC-009, DEC-018, DEC-020
- `src/write/{mod,plan,jpeg,exif,tiff,verify}.rs` (the JPEG planner is the model; `exif.rs` `plan_block` edits a TIFF blob in place), `src/policy.rs`
- `src/png/{mod,metadata,crc}.rs`, `src/audit/mod.rs` (`audit_png`), `src/audit/category.rs` (`text_keyword_category`)
- `tests/support/{oracle_png,png_shapes}.rs` (fixtures), `fuzz/fuzz_targets/rewrite_jpeg.rs`, `fuzz/shared/*` (harness model), `tests/strip*.rs` (JPEG strip tests as the pattern)

## Outputs

- **New `src/write/png.rs`:** `plan(data, spec) -> Result<Planned, StripError>`; `strip` dispatches `Sniffed::Png`; WebP still `Unsupported`.
- **Modified `src/write/{mod,verify}.rs`:** PNG self-check (DEC-021 clause 6), new typed refusal reasons only where the existing set cannot express them (a halted walk, a chunk past EOF), `RemovalKind` variants for PNG chunks (`#[non_exhaustive]` already).
- **Modified `src/png`, `src/audit`** only where the carrier measurement (Part A step 1) shows ExifTool decodes a chunk type the audit does not list: locate it as a region and give it a category (DEC-020 amended, DEC-018 clarification).
- **Fuzz:** `fuzz/fuzz_targets/rewrite_png.rs` with its harness model in `fuzz/shared/`, seeds from the PNG seeds (generated by the existing `examples/fuzz_seeds.rs`), `tests/fuzz_layout.rs` pins, a CI smoke job and `just fuzz-smoke rewrite_png`.
- **Tests:** the failing tests below; the existing SPEC-032 test that `strip` refuses a PNG is updated (PNG now strips; WebP still refuses).
- **Docs:** README (the writer covers PNG; the audit still reports it not strippable until SPEC-039), DEC-021 filled with the measured carrier list. No CHANGELOG entry.

## Acceptance Criteria

- [x] For every PNG fixture and seed and each of the five presets, `strip` returns bytes that pass the self-check or a typed refusal; never panics; output length at most the input length.
- [x] Pixel integrity (I1): an independent recomputation (the input with exactly the listed removals deleted and the listed patches applied) equals the output byte for byte; `IHDR`, `PLTE`, `IDAT` and every kept chunk are unchanged and in order.
- [x] Every category rule of DEC-021 clause 2 is tested per chunk kind, in each position (before `IDAT`, after `IDAT`, after `IEND`), compressed and not.
- [x] `eXIf`: all-categories policy removes every `eXIf` (several, after `IDAT`, after `IEND`); a partial policy edits entries in place and the chunk CRC is valid in the output; none leaves them; an unparseable `eXIf` is removed whole under `Everything`/`All` and is `Uneditable` under a partial policy.
- [x] Opaque carriers: raw profiles (every spelling the audit treats as a raw profile, in `tEXt`, `zTXt`, `iTXt`) and the measured extra chunk types are removed by every preset; `dSIG`-class chunks only by `Everything`; the measured list is in DEC-021 and a test iterates it.
- [x] Refusals: a walk halted by the chunk or issue cap, a chunk length past EOF, and an `eXIf` that cannot be edited exactly return the typed refusal with no bytes; a missing `IEND`, a bad CRC on a removed chunk and a bad CRC on a kept chunk do not refuse (the kept chunk stays as it was).
- [x] The preview stays honest: `ContainerKind::strippable()` is still false for PNG, the CLI and reports are unchanged, and a test pins that `strip` working and `strippable()` false coexist until SPEC-039.
- [x] Idempotent: `strip(strip(x)) == strip(x)` for every fixture and preset.
- [x] `rewrite_png` fuzz target, seeds, layout pins and a 60 s CI smoke job; the harness model re-derives the invariants independently of the library.
- [x] `just lint`, `just test`, `just forbid-unsafe`, `just validate`, `just cost-audit`, `just oracle-record` idempotent, `just fuzz-smoke rewrite_png`, and the smokes of every fuzz target whose code changed (`png_chunks`, `audit`, `audit_render`) green, on the pinned 1.90.0.

## Failing Tests

- **`src/write/png.rs` unit tests and `tests/strip_png.rs`** (new, the JPEG `tests/strip*.rs` pattern): per-preset strip of every PNG fixture; the I1 recomputation; each DEC-021 clause; the refusals; idempotence; the CRC of an edited `eXIf`; carriers; kept bad CRC stays bad.
- **`tests/strip.rs`:** PNG no longer refused; WebP still refused for every policy.
- **`tests/fuzz_layout.rs`** pins; the harness invariants on every seed and prefix.

## Non-Goals

- WebP (SPEC-038); the audit and CLI switch-over and ExifTool write parity (SPEC-039); `set` operations; APNG frame editing; editing inside XMP or ICC payloads; a CLI strip command.

## Notes for the Implementer

- **Two builder parts so a stop is clean.** Part A: the carrier measurement, the reader/audit extension if needed, `src/write/png.rs`, the self-check, unit tests (commit). Part B: `rewrite_png` fuzz target, seeds, pins, CI job, README, DEC-021 filled. Part A hands back the measured carrier grid before anything else is built.
- **Step 1 of Part A is a measurement:** one chunk per file, ExifTool 13.55 `-a -u -G1 -H -s`, for every PNG chunk type that prints Exif, XMP or ICC tags or a personal string (`eXIf`, `zxIf`, `tXMP`, `meTa`, `iCCP`, `tEXt`/`zTXt`/`iTXt`, `tIME`, `dSIG`, `cpIp`, `gIFx`, `gIFg`, `fRAc`, `orNT`, `caNv`, `ceXt`, `iDOT`, plus raw-profile spellings). For each: does the audit list it? If not, the build extends the reader and audit (small) and puts it in DEC-021's list. Also measure what ExifTool prints for a strip OUTPUT of a few shapes (for the verify: this is a design cross-check, the formal I3 is SPEC-039).
- Strip from the chunk walk (`png().chunks()`), never from `exif_blocks()` or the region list alone: SPEC-032 and SPEC-036 verifies found this class five times (17th WebP block, raw-profile GPS, `tIME` lengths, empty regions, lowercase raw-profile keyword).
- Reuse `plan_block` for the `eXIf` edit: its patches are in blob coordinates; add the chunk data start, then one more `Patch` for the CRC (compute it from the edited bytes; `src/png/crc.rs`).
- Empty chunks are metadata for PNG (SPEC-036 verify): a length-0 `iCCP` with a name, an empty iTXt XMP, an empty text value are removed under their category.
- Do not touch `ContainerKind::strippable()`.
- macOS has no `timeout`: `perl -e 'alarm 8; exec @ARGV' -- exiftool ...`. Pinned toolchain 1.90.0 for lint. Budget about 100 tool calls or 250k context per agent; expect two builder agents (A then B), a verify and a fix round.

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
