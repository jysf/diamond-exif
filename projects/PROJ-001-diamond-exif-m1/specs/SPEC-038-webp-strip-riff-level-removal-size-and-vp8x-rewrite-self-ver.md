---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-038
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
  decisions: [DEC-013, DEC-019, DEC-020, DEC-021, DEC-022]                 # [DEC-NNN, DEC-MMM]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-030, SPEC-034, SPEC-036, SPEC-037]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-037]                     # e.g. [SPEC-002]

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
  tokens_estimate: 900000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-038: WebP strip: RIFF-level removal, size and VP8X rewrite, self-verify, rewrite_webp fuzz

## Context

SPEC-037 gave `strip` the PNG container (DEC-021), and its verify showed what the design must do for a second container: strip from the chunk walk, measure which chunk types the oracle decodes (not only those the audit lists), measure case folding per type, and check outputs with the real tool. WebP is a RIFF stream; metadata sits in top-level chunks, and a removal must keep a size field and flag bits consistent. **DEC-022** (written at design) carries the decisions; read it first.

Measured at design (ExifTool 13.55, `-a -u -G1 -H -s`): a `LIST` chunk of type `INFO` prints `Artist`, `Copyright`, `Software`, `Comment`, `DateCreated` in group `RIFF`; a `JUNK` chunk holding text prints `TextJunk`; a `C2PA` chunk is decoded as JUMBF; lower-case `exif`, `xmp `, `iccp` are not decoded (printed as `Unknown_*`); an `EXIF` chunk placed beyond the declared RIFF size is still read. The audit lists none of `LIST/INFO`, `JUNK` text or `C2PA`, so today a WebP with only a `LIST/INFO` chunk audits clean while ExifTool shows the author.

## Goal

`strip(webp, policy)` for every `StripPolicy`: chunk removal by category (`EXIF` all, none or entry edit; `XMP `/`XMP\0`; `ICCP`), opaque carriers removed (the measured grid, DEC-022 clause 4), the RIFF size and `VP8X` flag rules, typed refusals, a self-check by re-audit, a gated live ExifTool check of the outputs, and a `rewrite_webp` fuzz target with a 60 s CI smoke.

## Inputs

- DEC-022, DEC-021 (with its clarifications), DEC-013, DEC-019, DEC-020
- `src/write/png.rs` (the model planner), `src/write/{mod,plan,exif,verify}.rs`, `src/policy.rs`
- `src/webp/{mod,metadata,issue}.rs`, `src/audit/mod.rs` (`audit_webp`), `src/audit/category.rs`
- `tests/strip_png.rs`, `tests/strip_png_live.rs`, `fuzz/fuzz_targets/rewrite_png.rs`, `fuzz/shared/rewrite_png_invariants.rs` (the models to follow), `tests/support/{oracle_webp,webp_shapes}.rs`, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs`

## Outputs

- **New `src/write/webp.rs`:** `plan(data, spec) -> Result<Planned, StripError>`; `strip` dispatches `Sniffed::Webp`.
- **Modified `src/write/{mod,verify}.rs`:** the WebP self-check (DEC-022 clause 9) incl. the size and flag checks; new `RemovalKind`/`PatchKind` variants (`RiffSize`, `Vp8xFlags`), `UnstrippableReason` variants only where needed.
- **Modified `src/webp`, `src/audit`:** locate the measured carriers (`LIST/INFO` etc.) as regions with categories per DEC-022 clause 4; DEC-020 clarification; moved tests, fuzz invariant mirrors and parity records reported.
- **Fuzz:** `fuzz/fuzz_targets/rewrite_webp.rs`, `fuzz/shared/rewrite_webp_invariants.rs` (independent of the writer), seeds generated by `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs` pins and plants, CI matrix entry (the eleventh target), `just fuzz-smoke rewrite_webp`.
- **Tests:** `tests/strip_webp.rs`; gated `tests/strip_webp_live.rs` (ExifTool on each output); the SPEC-032 refusal test updated (WebP now strips; HEIC and GIF still refuse).
- **Docs:** README (the writer strips WebP, library only; the audit and CLI still say not strippable until SPEC-039), DEC-022 filled with the measured carrier table. No CHANGELOG entry.

## Acceptance Criteria

- [x] For every WebP fixture and seed and each preset, `strip` returns bytes passing the self-check or a typed refusal; never panics; output length at most the input length; all 98 recorded `oracle_webp` fixtures run through `strip` in a test.
- [x] I1: an independent recomputation (the input with exactly the listed removals deleted and the listed patches applied) equals the output byte for byte; every kept chunk is byte-identical and in order.
- [x] The measured carrier grid is complete: one chunk type per file, ExifTool 13.55, every RIFF chunk type that prints a tag in a WebP (at least `LIST` types `INFO` and others, `JUNK`/`PAD ` text, `C2PA`, `IDIT`, `ID3 `, and every code the build finds), with its category, whether the audit now lists it, and the result of all 16 case spellings of each type; the table is in DEC-022 and a test iterates it.
- [x] Every category rule is tested per chunk kind, in each position (before the image data, after it, after the declared RIFF size), including the 17th and later `EXIF` chunk and metadata after an empty null chunk.
- [x] `EXIF`: all-categories removes every one; none leaves them; a partial policy edits entries in place (a chunk with the `Exif\0\0` prefix too) or refuses with the existing reasons.
- [x] RIFF size: the subtraction rule on files whose declared size is exact, too small, too large and beyond the end; `VP8X` flags cleared exactly for what was removed, other bits untouched; both checked by the self-check.
- [x] Refusals typed with no bytes: a walk halted by a cap, a chunk past the end of the input, a size or flag rule that cannot be applied, an `EXIF` edit that cannot be made exactly; a missing pad byte, an odd chunk without its pad, a wrong declared size and trailing non-chunk bytes do not refuse.
- [x] Live ExifTool (gated, `DIAMOND_EXIF_REQUIRE_ORACLE`): for the carrier grid, the case grid and a dozen fixtures, under all five presets, ExifTool on the OUTPUT prints no tag the preset removes (the tag list is pinned, as in `strip_png_live`); each carrier file prints at least one removable tag before the strip, so a pass is never vacuous. Outputs decode (pixel data unchanged).
- [x] The preview stays honest: `ContainerKind::strippable()` false for WebP, CLI and reports unchanged apart from the preview wording for WebP; a test pins that a working strip and `strippable()` false coexist.
- [x] Idempotent: `strip(strip(x)) == strip(x)` for every fixture and preset.
- [x] `rewrite_webp` fuzz target, seeds, pins, plants, CI job; the harness re-derives the predicted output or refusal independently of `src/write/webp.rs` and shares no carrier list with it by copy: it takes its list from the measured DEC-022 table in a form a test cross-checks against ExifTool-recorded facts, not from the writer.
- [x] `just lint`, `just test`, `just mutation-oracle`, `just forbid-unsafe`, `just validate`, `just cost-audit`, `just oracle-record` idempotent (no machine paths in records), `just fuzz-smoke rewrite_webp`, `webp_riff`, `audit`, `audit_render` green on the pinned 1.90.0.

## Failing Tests

- **`src/write/webp.rs` unit tests and `tests/strip_webp.rs`:** per preset over every fixture; the I1 recomputation; each DEC-022 clause; the size and flag rules; refusals; idempotence; carriers and the case grid; metadata beyond the declared size and after a null chunk; 17+ `EXIF` chunks.
- **`tests/strip_webp_live.rs`** (gated): the live checks above.
- **`tests/strip.rs`, `tests/audit_png_webp.rs`:** WebP no longer refused, HEIC and GIF still refused, `strippable()` false.
- **`tests/fuzz_layout.rs`:** seeds, policy byte, plants.

## Non-Goals

- The audit and CLI switch-over and the formal ExifTool write parity (SPEC-039); a CLI strip command; `set` operations.
- Editing inside `ANMF` frames, re-encoding, or repairing a file's own size or pad faults (a lie stays a lie by the same amount).
- Inside-XMP or inside-ICC edits.

## Notes for the Implementer

- **Two builder parts so a stop is clean.** Part A: the carrier and case measurement (report the table FIRST), the reader and audit extension, `src/write/webp.rs`, the self-check, unit tests (commit the library extension first). Part B: `rewrite_webp` target, seeds, pins and plants, CI, the live test, README, DEC-022 table.
- **The two lessons of SPEC-037 are requirements here:** (1) a model typed from the same measurement as the code cannot catch a wrong measurement, so the live ExifTool test on OUTPUTS is part of the build, not left to verify; (2) measure case folding for every chunk type before writing any rule.
- Strip from the chunk walk (`webp().chunks()`), never from `exif_blocks()` or the region list alone; the walker keeps reading past the declared RIFF size and past a null chunk.
- Reuse `plan_block` for the `EXIF` edit: its patches are in blob coordinates; add the TIFF start (`tiff_at`). No CRC.
- The chunk's removal range includes its pad byte when present; check what `Chunk::range()` covers and what a missing pad does.
- Records must not contain machine paths (use `tools::strip_perl_lib_dir` via `warning_lines`).
- macOS has no `timeout`: `perl -e 'alarm 8; exec @ARGV' -- exiftool ...`. Pinned 1.90.0 for lint; run `just lint-fuzz` too. Restore `fuzz/Cargo.lock` after fuzz runs. Budget about 100 tool calls or 250k context per agent; expect two builder agents, a verify and a fix round.

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
