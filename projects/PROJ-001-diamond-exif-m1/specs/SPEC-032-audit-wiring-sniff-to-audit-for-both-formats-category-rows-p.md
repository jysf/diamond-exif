---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-032
  type: story                      # epic | story | task | bug | chore
  cycle: verify  # frame | design | build | verify | ship
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
  stage: STAGE-008
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-020]                 # [DEC-NNN, DEC-MMM]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-029, SPEC-030, SPEC-031, SPEC-034, SPEC-035, SPEC-033]             # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-031, SPEC-034]

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
  tokens_estimate: 650000
  sessions:
    - cycle: build
      handoff: HANDOFF-069
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 237190
      estimated_usd: 1.57
      duration_minutes: 21
      recorded_at: 2026-11-22
      notes: "Part A only; cost is the harness subagent_tokens figure recorded by the orchestrator."
    - cycle: build
      handoff: HANDOFF-070
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 456904
      estimated_usd: 3.02
      duration_minutes: null
      recorded_at: 2026-11-22
      notes: "tokens left null by instruction: the orchestrator records the harness figure"
  totals:
    tokens_total: 691690
    estimated_usd: 4.57
    session_count: 2
---

# SPEC-032: Audit wiring: PNG and WebP into exif-audit (kinds, text-keyword categories, honest preview, CLI)

## Context

Last spec of STAGE-008. The readers exist and are oracle-checked (SPEC-028 to
031, 034); `sniff` names both formats and `write::strip` already refuses them,
but `audit()`, the CLI read gate (`src/bin/exif-audit/read.rs`) and the
renderers still call PNG and WebP unsupported. Design is in **DEC-020** (read
it first; it carries the decisions below). Measured at design (ExifTool 13.55):
PNG `tEXt` keywords print as `PNG:Title`, `Author`, `Description`, `Copyright`,
`CreationTime` (keyword `Creation Time`), `Software`, `Disclaimer`,
`PNGWarning` (keyword `Warning`), `Source`, `Comment`; unknown keywords print as
their own names; lowercase `author` printed as `Author`, `AUTHOR` as `AUTHOR`;
a `tIME` chunk prints `PNG:ModifyDate`; `pHYs` prints non-personal tags.

## Goal

`exif-audit` audits PNG and WebP: every Exif block inventoried (nothing merged),
located XMP, ICC, text, raw-profile and `tIME` regions as items and findings
under existing categories, text keywords categorised, a preview that never
claims a removal, the writer still refusing both, and renderers, CLI gate,
fuzz seeds and README updated.

## Inputs

- DEC-020, DEC-008, DEC-009, DEC-010, DEC-017, DEC-018, DEC-019
- `src/audit/{mod,category,preview,sniff}.rs` (`audit_jpeg` is the model)
- `src/png/metadata.rs` (`PngRegion`, `PngExifBlock`, `PngMetadata`),
  `src/webp/metadata.rs`, `src/png/mod.rs` (`ChunkKind`, `trailer()`)
- `src/report/{mod,text,json,csv}.rs`, `src/bin/exif-audit/read.rs`,
  `src/write/mod.rs` (`strip`)
- Fixture builders: `tests/support/oracle_png.rs`, `oracle_webp.rs`,
  `png_shapes.rs`, `webp_shapes.rs`; committed records
  `tests/oracle-fixtures/exiftool/{png,webp}-*.txt`
- `guidance/questions.yaml` entries cited by `KNOWN_DIVERGENCES` for the two
  formats

## Outputs

- **Modified `src/audit/`:** `ContainerKind::{Png, Webp}` (enum becomes
  `#[non_exhaustive]`); `RegionKind::{Text, RawProfile, Time}`; `audit_png` and
  `audit_webp` beside `audit_jpeg`; `region_category` rows plus a
  `text_keyword_category(keyword: &[u8]) -> Category` table (data, one row per
  keyword) in `category.rs`; `Preview::strippable()`; docs updated (module
  doc, `Health::jpeg_issues` = container-layer issues for every format,
  `Origin::after_scan` = after the image data).
- **Modified `src/write/mod.rs`:** refusal carries `ContainerKind::Png|Webp`.
- **Modified `src/report/*`, `src/bin/exif-audit/read.rs`:** read gate takes
  `Png|Webp`; status `Unsupported` only for HEIC, GIF, BigTIFF, empty,
  unknown; `container_name`; text, JSON (additive `strippable`) and CSV say
  "not strippable" for these formats and "located" not "valid" for regions.
- **Fuzz:** PNG and WebP seeds added to `audit` and `audit_render` (copied from
  the existing `png_chunks` and `webp_riff` seeds, generated by the one
  existing script or table), `tests/fuzz_layout.rs` pins updated.
- **Tests and records:** per-shape unit tests, keyword table test, a committed
  `png-text-keywords` fixture and record (tag names ExifTool prints for the
  keyword set), the category-presence replay and live tests, red-proofs.
- **Docs:** README (the two "still reports ... as unsupported" statements, the
  verification and exit-code lines), `guidance/questions.yaml` if a new
  divergence appears. **No CHANGELOG entry** (`tests/release_layout.rs` pins
  `[Unreleased]` empty).

## Acceptance Criteria

- [x] `audit()` returns `ContainerKind::Png` / `Webp` for those inputs; every
      other sniffed kind is unchanged; truncated or `Err` parses are
      `unreadable(kind)`, never clean.
- [x] Every parsed Exif block of a PNG and a WebP is inventoried with its block
      index and chunk index; two blocks give two sets of items; no merge.
- [x] A WebP with 17 `EXIF` blocks (GPS only in the 17th) is not clean, and its
      preview is unreliable; a test names the case.
- [x] Regions: `Xmp`, `Icc` (PNG `iCCP`, WebP `ICCP`), `Text`, `RawProfile`,
      `Time`, PNG `Trailer`; a length-0 region has an item and no finding;
      after-IDAT / after-IEND is recorded in `Origin::after_scan`.
- [x] Text keywords: Author and Copyright to `Authorship`, Creation Time to
      `Timestamps`, Software to `Software`, others `FreeText`; the match rule
      (exact vs first-letter case) is MEASURED against ExifTool on at least
      `Author`, `author`, `AUTHOR`, `aUTHOR` and pinned; the committed
      `png-text-keywords` record agrees with the table (a keyword ExifTool
      prints under a different name is a recorded divergence, not silent).
- [x] `preview()` of a PNG or WebP: every finding `Kept`, `removed == 0`,
      `reliable()` false, `strippable()` false, for every `StripPolicy`; JPEG and
      TIFF previews unchanged byte for byte.
- [x] `strip()` of a PNG and of a WebP returns `StripError::Unsupported` with
      the new kind, for every policy; no output bytes.
- [x] CLI: a PNG with GPS in `eXIf` exits with the findings code, a clean PNG
      is `ok`, a WebP with unreadable metadata is `issues`, HEIC is still
      `unsupported`; text, JSON, CSV golden/shape tests updated for the new
      kinds; no new CLI flag.
- [x] Category-presence replay: for every PNG and WebP oracle fixture with a
      committed record, the audit has a GPS / Xmp / Icc finding iff ExifTool
      printed the GPS / XMP / ICC selector, except recorded divergences
      (`png-*`, `webp-*` ids in `KNOWN_DIVERGENCES`); a perturbed record makes
      the replay name the fixture (shown red); live test gated like its
      siblings.
- [x] No panic on hostile input: `audit` and `audit_render` fuzz 60 s clean with
      the new seeds.
- [x] `just lint`, `just test`, `just mutation-oracle`, `just forbid-unsafe`,
      `just validate`, `just cost-audit`, `just oracle-record` idempotent,
      `just fuzz-smoke audit`, `just fuzz-smoke audit_render` green.

## Failing Tests

- **`src/audit/mod.rs` tests** (or `src/audit/png_webp_tests.rs`): kind
  dispatch; two-block inventory; 17-block WebP not clean; region matrix per
  shape; empty region has no finding; keyword table; unreadable parse.
- **`src/audit/preview.rs`:** PNG and WebP are `Kept`, `strippable()` false,
  JPEG unchanged.
- **`src/write` tests:** `strip` refuses both for all policies.
- **`tests/audit_png_webp_oracle.rs`** (new): category-presence replay + live,
  perturbation red-proof, keyword record test.
- **`src/report` and CLI tests:** the new kinds in the three renderers; the
  exit-code cases above.
- **`tests/fuzz_layout.rs`:** seed pins.

## Non-Goals

- Strip or sanitise PNG or WebP (M1d); new CLI flags; a release or version.
- Tag-level inventory parity against ExifTool for PNG and WebP: **SPEC-036**
  (new backlog item; the inventory oracle has its own corpus and group map).
- Inflating compressed regions, parsing XMP or ICC, decoding raw profiles.
- A new `Category` or a `Disposition` variant (DEC-020); a rights category is
  SPEC-033's.
- Other PNG chunks (`pHYs`, `gAMA`, ...): not personal.

## Notes for the Implementer

- **Build in two commits so a stop is clean.** Part A: `src/audit`, `src/write`
  and their unit tests (the model). Part B: report, CLI, fuzz seeds, oracle
  replay, README. Commit after A. If the budget is near, hand back after A.
- Mirror `audit_jpeg`: one `Builder`, `push`, `finish`. Do not invent a second
  accumulator. `indexing_slicing` and the five-lint deny set apply in `src/`.
- The audit reports no single value, so the per-selector first/last rule of
  SPEC-034/035 does not apply; but the audit must walk `exif_blocks()` and
  never rely on `exif()` (first block only).
- PNG ExifTool reads only the first `eXIf`; the audit shows all (privacy). Do
  not "fix" this to match ExifTool.
- ExifTool stops at an empty WebP null chunk and we read on; ExifTool reads a
  17th WebP block and we do not. Neither is a new divergence to hide: both are
  already recorded; the second must show up as not-clean.
- A located `ICCP` or `XMP ` can be unreadable to ExifTool: wording says
  "located".
- Make the keyword table data (array of `(&[u8], Category)`), tested row by
  row, so SPEC-033 can re-home Copyright in one line.
- Measure with `perl -e 'alarm 8; exec @ARGV' -- exiftool -a -G1 -s -n`
  (macOS has no `timeout`); `just oracle-record` for the new record.
- Budget: about 100 tool calls or 250k context per agent; expect two builder
  agents (A then B) rather than one long run.

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

## Carried from SPEC-030 verify (2026-11-21)

- A zero-length `Xmp` or `Icc` region (WebP) must not count as XMP or ICC present: ExifTool shows nothing for it (DEC-019 clarification).
- PNG `Xmp`/`Icc`/`RawProfile` regions that are compressed, and PNG raw-profile text, are metadata the audit could not read (DEC-018); report them as such.

## Carried from SPEC-031 verify (2026-11-21)

- Any `BadTextChunk` or `BadIccpChunk` (PNG) issue means metadata is present that we could not read (ExifTool exposes such chunks as raw data or a name): report it as unreadable metadata, not as clean.
- A PNG `RawProfile` region beside an `eXIf` may override it (ExifTool merges both and the later raw value wins, including raw-only tags such as GPS): flag the pair.
- A PNG `Icc` region with an empty name is a real profile (ExifTool reads it); `Xmp` is only an `iTXt`/`tEXt`/`zTXt` XMP-keyword chunk with an empty language tag.
