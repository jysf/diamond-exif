---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-040
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: high                   # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: approved  # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.
  verify_rounds: [approved]  # every verify outcome, oldest first, appended by
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
  to_agent: claude-sonnet-5-5       # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: 2026-11-24

references:
  decisions: [DEC-006, DEC-008, DEC-009, DEC-012, DEC-013]
  constraints: [test-before-implementation, panic-free-library, forbid-unsafe-code, oracle-must-be-shown-red, fuzz-target-per-surface, exiftool-never-linked, no-io-in-library]
  related_specs: [SPEC-019, SPEC-020, SPEC-033]

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
  sessions:
    - cycle: design
      agent: claude-opus-5-5
      interface: claude-code
      basis: null
      tokens_total: null
      estimated_usd: null
      duration_minutes: null
      recorded_at: 2026-11-24
      notes: "main-loop design (ExifTool measurements, DEC-012/DEC-013 amendments, spec, handoff); un-metered, null by design"
    - cycle: build
      handoff: HANDOFF-088
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 338116
      estimated_usd: null
      duration_minutes: 47
      recorded_at: 2026-11-24
      notes: "Orchestrator-transcribed from the Agent result: subagent_tokens 338116, tool_uses 214, duration_ms 2799998. The implementer was told not to edit this file, so the body below is the orchestrator's summary of its report. estimated_usd null: no input/output split."
    - cycle: verify
      handoff: HANDOFF-089
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 245439
      estimated_usd: null
      duration_minutes: 25
      recorded_at: 2026-11-24
      notes: "The verifier cannot read its own token count or wall time from inside the run; the orchestrator transcribes tokens_total and duration from the Agent result (subagent_tokens, duration_ms). Transcribed: subagent_tokens 245439, tool_uses 107 (the agent estimated about 85), duration_ms 1480597. estimated_usd null: no input/output split."
    - cycle: ship
      agent: claude-opus-5-5
      interface: claude-code
      basis: null
      tokens_total: null
      estimated_usd: null
      duration_minutes: null
      recorded_at: 2026-11-24
      notes: "main-loop ship (AC6 wording, toolchain brief, cost sync, CHANGELOG, archive, STAGE-011 close); un-metered, null by design. The orchestrator's session total is recorded once, at stage grain, in STAGE-011 orchestration_cost."
  totals:
    tokens_total: 583555
    estimated_usd: 0.00
    session_count: 4
shipped_at: 2026-11-24
---

# SPEC-040: DEC-012 amendment: JFIF APP0 thumbnail is a Thumbnail region, audited and stripped

## Context

STAGE-011's only spec. PROJ-001.

An external code review (against `fd94d4f`, 2026-11-23, finding 3) found two related problems. A JPEG whose APP0 `JFIF\0`
segment carries the optional uncompressed thumbnail audits with `clean=true` and zero findings, and
`strip(Everything)` returns the file byte for byte unchanged. ExifTool extracts the thumbnail as `JFIF:ThumbnailTIFF`.
The cause is DEC-012's benign list. The code (`src/jpeg/metadata.rs`, the `App(n)` arm near line 821) does what that
list says. The same review's other three findings shipped as PATCH-001 (#59). They are unrelated.

**Measured in design (ExifTool 13.55, built files, 2026-11-24).** The full table is in the DEC-012 amendment of
2026-11-24 and is normative. In short: ExifTool's thumbnail is `min(3·X·Y, bytes present)` bytes after the 14-byte
header. It prints none when X·Y is 0 or no data byte is present. It reads JFIF in any APP0, at any position, of any
version, and each segment on its own. It ignores `JFIF\0` in APP1. It cannot delete the thumbnail alone:
`-ThumbnailTIFF=` and `-ThumbnailImage=` are no-ops. `-all=` and `-JFIF:all=` delete the **whole** APP0 segment, and
the file still decodes. The scratch builder and files were in the design session's scratchpad and are not kept; the
implementer rebuilds them (see Notes).

## Goal

The audit reports a JFIF APP0 thumbnail as a `Thumbnail` finding on a new `JfifThumbnail` region. Every policy that
removes `Thumbnail` (`AllPii`, `All`, `Everything`, and a `Custom` set that names it) deletes that whole APP0 segment.
The self-check and ExifTool both confirm that the thumbnail is gone, and that JFIF segments without a thumbnail are
untouched.

## Decisions made here

The open questions from framing are settled in **DEC-012's amendment of 2026-11-24**, with a pointer in DEC-013's
amendment of the same date:

1. **Remove the whole segment; don't rewrite it.** This is the same unit of removal as DEC-013 rule 3, and the same
   result as `exiftool -JFIF:all=`. Losing the density fields is accepted: the image decodes without them, as
   measured.
2. **A thumbnail shorter than declared:** the region is the bytes present (`min(n, have)`), plus one `RegionTooShort`
   issue. With no byte present there is the issue and no region. This follows the existing `ex.short(...)` pattern.
3. **Category `Thumbnail`.** The finding carries `thumbnail: Some { range: None, starts_with_ffd8: false }`, and the
   report region name is `jfif_thumbnail`. This adds an enumeration value, as DEC-020 did, so `schema_version` stays
   2.
4. **Presets: none change.** DEC-009's `Thumbnail` row and may-hold row apply as they stand.

Deferred: bytes past the declared thumbnail, and text hidden after a 0×0 header. ExifTool doesn't print them either,
so the oracle can't prove them. This is question `jfif-bytes-past-the-thumbnail`.

## Inputs

- **Normative:** DEC-012 (with its amendment of 2026-11-24), DEC-013 (rule 3, and its amendment of 2026-11-24),
  DEC-008 (`Thumbnail`), DEC-009 (`Thumbnail` rows), DEC-010 (report contract), DEC-020 (precedent for additive region
  kinds), DEC-001 (tolerant issues).
- **Code to read:**
  - `src/jpeg/mod.rs`: `SIG_JFIF` and `SegmentKind::Jfif`.
  - `src/jpeg/metadata.rs`: `Region`, the extraction loop's `App(n)` arm, and `Extract::short`.
  - `src/audit/mod.rs`: `audit_jpeg`, `RegionKind` and `Builder::push_with`.
  - `src/audit/category.rs`: `region_category`.
  - `src/report/{mod,text,json,csv}.rs`.
  - `src/write/jpeg.rs`: `segment_targets` and `RemovalKind`.
  - `src/write/mod.rs`.
  - `src/write/verify.rs`: check (a) already fails on any finding left in `S`.
- **Harnesses that model the extraction or the plan independently** (each must learn the rule):
  - `fuzz/shared/jpeg_metadata_invariants.rs`: re-derives the regions.
  - `fuzz/shared/audit_invariants.rs`: maps regions to categories, and checks "thumbnail range iff `Thumbnail`".
  - `fuzz/shared/rewrite_invariants.rs`: the model of removals, `T::Jfif`, and the `RemovalKind` names.
- **Oracle support:**
  - `tests/support/inventory.rs`: `UNMODELLED_GROUPS` drops every `JFIF` line today.
  - `tests/inventory_oracle.rs`.
  - `tests/strip_oracle.rs` and `tests/support/strip_shapes.rs`.

## Outputs

- **`src/jpeg/metadata.rs`:**
  - A new `Region::JfifThumbnail { segment, data }`. `Region` is `#[non_exhaustive]`.
  - Its arm in `Region::segment()` and `data()`.
  - The JFIF arm of the extraction loop implements DEC-012 amendment part 1.
  - The benign-list comment is updated.
- **`src/audit/mod.rs`:**
  - `RegionKind::JfifThumbnail`.
  - In `audit_jpeg`, the arm passes the thumbnail marker `Some(ThumbnailRange { range: None, starts_with_ffd8: false })`
    for this kind only. The other regions still pass `None`.
- **`src/audit/category.rs`:** `region_category(JfifThumbnail) == Thumbnail`.
- **`src/report/mod.rs`:** `region_name(JfifThumbnail) == "jfif_thumbnail"`. Text, JSON and CSV need no other change.
  The text line prints `thumbnail range=none`, as for a strip-form IFD1.
- **`src/write/mod.rs`:** `RemovalKind::Jfif`, with a doc comment. `RemovalKind` is `#[non_exhaustive]`.
- **`src/write/jpeg.rs`:** in `segment_targets`, a `SegmentKind::Jfif` segment that has a `JfifThumbnail` region gives
  `(Category::Thumbnail, RemovalKind::Jfif)`. A JFIF segment without one gives nothing, as before.
- **Harnesses:** the three fuzz files above, and every exhaustive match on `Region`, `RegionKind` or `RemovalKind` in
  `tests/` and `fuzz/`.
- **Seeds:** JFIF cases for `jpeg_segments`, `audit` and `rewrite_jpeg`, generated by `examples/fuzz_seeds.rs`. The
  pins in `tests/fuzz_layout.rs` are updated.
- **Oracle records:**
  - Inventory: `JFIF:ThumbnailTIFF` (id `0x0009`) becomes the counterpart of a `JfifThumbnail` region. Every other
    `JFIF` line stays dropped.
  - New fixtures are added to the strip oracle. `just oracle-record` regenerates the records, which are committed.
- **`CHANGELOG.md`** under `[Unreleased] → Fixed` (at ship).
- **No new flags or options.** No new public function. One new variant on each of three `#[non_exhaustive]` enums.

## Acceptance Criteria

Each criterion maps to a test below. "Built file" means a JPEG built in the test from the shapes in the DEC-012
amendment table.

- [ ] **AC1 (locate):** a JFIF APP0 segment with 2×2 and 12 data bytes gives exactly one `JfifThumbnail` region whose
  data is those 12 bytes, and no issue.
- [ ] **AC2 (benign stays benign):** 0×0, 2×0, a header shorter than 14 bytes, and 0×0 followed by text each give no
  region, no finding and no new issue. The byte-for-byte audit result is the same as on `main`.
- [ ] **AC3 (short):** 4×4 declared with 12 bytes present gives a 12-byte region and one
  `RegionTooShort { what: "JFIF thumbnail", .. }`. 2×2 declared with 0 bytes gives that issue and no region.
- [ ] **AC4 (extra bytes):** 2×2 followed by 12 + 5 bytes gives a 12-byte region. The 5 bytes are not in it.
- [ ] **AC5 (position, version, multiplicity):** a JFIF segment after the Exif APP1, version 2.00, and two JFIF
  segments are each examined on their own (two segments with thumbnails give two regions). `JFIF\0` in an APP1 stays
  `OtherApp`, as today.
- [ ] **AC6 (audit):** the finding is `Thumbnail`, Medium, with `thumbnail == Some { range: None, starts_with_ffd8:
  false }`, on a region item of kind `JfifThumbnail` with the region's length. `health().clean` keeps its meaning (parse health, DEC-010's `ok`): a complete thumbnail leaves it true, as a COM finding does; only the short case's `RegionTooShort` makes it false (corrected at ship: the design text said "`is_clean` is false when it is present", which the build and verify showed is not this codebase's meaning).
  The audit's `holds` for an MPF secondary that carries a JFIF thumbnail includes `Thumbnail`.
- [ ] **AC7 (preview):** `Gps` and `GpsAndDevice` give `KeptUnread`. `AllPii`, `All` and `Everything` give `Removed`.
  `Custom({Thumbnail})` gives `Removed`. `Custom({Gps})` gives `KeptUnread`.
- [ ] **AC8 (strip removes the segment):** under `AllPii`, `All`, `Everything` and `Custom({Thumbnail})`, the output is
  the input with that whole APP0 segment cut, and nothing else changed (an exact byte comparison against the input
  with the segment spliced out). The removal record has kind `Jfif` and category `Thumbnail`. A second JFIF segment
  without a thumbnail stays.
- [ ] **AC9 (strip keeps it otherwise):** under `Gps` and `GpsAndDevice` the APP0 segment is kept byte for byte. A
  JFIF segment without a thumbnail is kept under every preset, including `Everything`.
- [ ] **AC10 (self-check is the gate):** with only the audit change and no writer change, `strip(AllPii)` on the 2×2
  file returns `VerifyFailed { check: Preview }`. Show this red once and record it in the build handback. After the
  writer change it returns `Ok`.
- [ ] **AC11 (ExifTool parity, live and replay):** for each built file and each preset, after the strip ExifTool prints
  `JFIF:ThumbnailTIFF` exactly when the policy keeps `Thumbnail`. When the thumbnail is removed, no `JFIF` group line
  remains for that segment. Shown red: on `main`, the `Everything` row fails because ExifTool still prints
  `ThumbnailTIFF`.
- [ ] **AC12 (inventory parity):** in the inventory oracle, `JFIF:ThumbnailTIFF` matches the `JfifThumbnail` region one
  for one. Shown red with a perturbed record (drop the line) and with the audit side's region dropped. No other
  inventory row moves.
- [ ] **AC13 (report):** the region name `jfif_thumbnail` appears in JSON, CSV and text. The JSON enumeration test is
  pinned and extended. `schema_version` stays 2.
- [ ] **AC14 (no regressions):** every existing test passes, and every changed test or record is listed in the build
  handback with its reason. Real-shape JPEG fixtures (`tests/real_shapes.rs`) whose JFIF segments have no thumbnail
  give the same findings as on `main`.

## Failing Tests

Write these first, red before the code changes. Type expected values and tables by hand; never derive them from the
code under test. Build the JFIF payloads with a small local helper (`jfif(x, y, data)`), not with the library.

- **`src/jpeg/metadata.rs` unit tests:**
  - `"a jfif thumbnail is a region"` (AC1)
  - `"a jfif segment without a thumbnail is benign"` (AC2, every shape)
  - `"a short jfif thumbnail is the bytes present and an issue"` (AC3, both shapes)
  - `"bytes past the jfif thumbnail are not in the region"` (AC4)
  - `"each jfif segment is examined on its own"` (AC5)
- **`src/audit/mod.rs` and `src/audit/category.rs` unit tests:**
  - `"a jfif thumbnail is a medium thumbnail finding with no range"` (AC6)
  - `region_category` table row (hand-typed).
- **`tests/policy_preview.rs`:** `"jfif thumbnail dispositions per preset"` (AC7, a hand-typed grid).
- **`tests/strip.rs`:**
  - `"jfif thumbnail segment is cut under thumbnail policies"` (AC8, exact splice comparison, each policy).
  - `"jfif thumbnail is kept under gps presets"` (AC9).
  - `"a jfif segment without a thumbnail is never cut"` (AC9, every preset).
  - `"two jfif segments: only the one with a thumbnail is cut"` (AC8).
- **`tests/strip_oracle.rs`, replay and live:** new fixtures `jfif-thumbnail`, `jfif-thumbnail-short`,
  `jfif-two-segments` and `jfif-no-thumbnail`, in the declared per-policy tag tables (AC11). Show the red on `main`.
- **`tests/inventory_oracle.rs`:** `"jfif thumbnail tiff is the jfif thumbnail region"`, with the two perturbation
  red-proofs (AC12).
- **`src/report` tests:** extend the JSON region-name pin with `jfif_thumbnail` (AC13).
- **Fuzz model tests** (`tests/fuzz_layout.rs`): the new seeds are pinned and the generator reproduces them.

## Non-Goals

- Bytes past the declared thumbnail, or hidden after a 0×0 header (question `jfif-bytes-past-the-thumbnail`).
- Rewriting a JFIF segment in place, or keeping its density fields.
- JFXX (already `OtherApp`), and `JFIF\0` in an APP1 (already `OtherApp`).
- Any change to DEC-009's presets or may-hold rows, or to `schema_version`.
- PNG, WebP and TIFF (JFIF is JPEG-only).

## Notes for the Implementer

- **Order:** commit the tests red first, then the JPEG layer and audit (AC1–7, AC13). Show AC10 red at that point:
  the self-check catches the writer gap. Then the writer (AC8, AC9), then the oracle work (AC11, AC12), then the fuzz
  harnesses and seeds.
- **Building test JPEGs:** the payload is `JFIF\0`, then version (2 bytes), units (1), X density (2), Y density (2),
  `Xthumbnail` (1), `Ythumbnail` (1), then the data. For live ExifTool fixtures you need a decodable JPEG body. Reuse
  a body the strip fixtures already have (`tests/support/strip_shapes.rs`) rather than adding a binary file.
- **The `thumbnail` marker:** `Some { range: None, .. }` already occurs for strip-form IFD1 thumbnails. The writer's
  Exif path selects thumbnails only from IFD1 tag findings in a block, so a region finding with `block: None` can't
  reach it. Check that, don't assume it.
- **Every exhaustive match** on `Region`, `RegionKind` and `RemovalKind` in `tests/` and `fuzz/` needs the new arm. The
  compiler finds them; the independent fuzz models need the rule written into them, not copied from the library.
- `just oracle-record` must be run twice with no diff.

---

## Reflection

*Appended during **ship**.*

1. **What would I do differently next time?**
   — Check every AC against the codebase's existing meaning of a word before writing it. AC6's "`is_clean` is false"
   was written from the review's phrasing, not from DEC-010, and the build had to work around it. Also budget the
   seed-count pins: they took most of the build's overrun (214 tool calls against ~120).

2. **Does any template, constraint, or decision need updating?**
   — Done this session: DEC-012 and DEC-013 amended (design); `guidance/toolchain-brief.md` seed counts now point at
   `tests/fuzz_layout.rs` as the source of truth, and it names `JFIF:ThumbnailTIFF` as the region's counterpart. The
   AC6 text is corrected in place. No signal recorded: the pin-update cost recurs, but its remedy (counts derived
   rather than pinned) is a test-design choice, not yet a lesson.

3. **Is there a follow-up spec I should write now before I forget?**
   — Not a spec. Optional: record a `Gps` or `AllPii` row in the strip oracle's JFIF fixtures, so the "kept" half of
   AC11 is checked live (verify's suggestion). Deferred question `jfif-bytes-past-the-thumbnail` stays open.

4. **Where was the worst defect caught?**
   — design
   *(The AC6 wording error, found in build and confirmed in verify; no code defect was found.)*

5. **What can a user do now that they couldn't before?**
   — Before, a JPEG whose JFIF APP0 carried a thumbnail audited clean with zero findings and kept the thumbnail under
   `Everything`; now the audit reports a Medium `thumbnail` finding (`jfif_thumbnail` region), and `AllPii`, `All` and
   `Everything` remove the segment, with output byte-identical to `exiftool -all=` on the 8 files the verifier compared.
