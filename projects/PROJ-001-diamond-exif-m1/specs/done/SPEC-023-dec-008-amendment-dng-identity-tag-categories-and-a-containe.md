---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-023
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: M          # stamped at ship: what it ACTUALLY took, same scale.
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
  stage: STAGE-005
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-008, DEC-012, DEC-013]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-022]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: []

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-005's <capability>". Optional; null is acceptable.
value_link: "The strip presets keep their promise on DNG and on multi-page TIFF, the two file kinds where they currently do less than they say."

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
  tokens_estimate: 600000
  sessions:
    - cycle: build
      handoff: HANDOFF-048
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 173750
      estimated_usd: 1.15
      duration_minutes: 14
      recorded_at: 2026-11-19
      notes: "Orchestrator correction: tokens_total is the harness subagent_tokens (173,750; 42 tool uses; 827,609 ms = 13.8 min) at 6.60 USD/Mtok = 1.15 USD, the same basis as earlier specs. The builder reported an estimate of 230,000 and about 60 tool calls, which is not used."
    - cycle: verify
      handoff: HANDOFF-049
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 179858
      estimated_usd: 1.19
      duration_minutes: 14
      recorded_at: 2026-11-19
      notes: "Orchestrator correction: tokens_total is the harness subagent_tokens (179,858; 49 tool uses; 841,423 ms = 14.0 min) at 6.60 USD/Mtok = 1.19 USD, the same basis as earlier specs. The verifier self-reported 4,802,409 (transcript, cache reads included) and 3.97 USD at assumed rates; neither is used. Verdict APPROVED, three Low findings (L1 no fixture puts Make/DateTime/GPS pointer in a page IFD1; L2 no audit fuzz seed carries the four DNG tags; L3 DEC-008 wording, fixed in the ship commit)."
    - cycle: ship
      agent: claude-opus-5-5
      interface: claude-code
      basis: estimate
      tokens_total: null
      estimated_usd: null
      duration_minutes: null
      recorded_at: 2026-11-19
      notes: "Un-metered main-loop cycles (design, orchestration, ship) not measured: null on purpose."
  totals:
    tokens_total: 353608
    estimated_usd: 2.34
    session_count: 3
shipped_at: 2026-11-19
---

# SPEC-023: DEC-008 amendment: DNG identity tag categories and a container-aware TIFF page rule

## Context

STAGE-005, first and only spec. The SPEC-022 verify found two places where the audit's DEC-008 table makes
the strip presets promise more than they deliver on TIFF-family files. Both are recorded as questions in
`guidance/questions.yaml` and were decided on 2026-11-11 (PR #35), each as "yes, as recommended":

1. `dng-identity-tags-have-no-category` (verify finding M3). DNG tags 0xC614 UniqueCameraModel, 0xC615
   LocalizedCameraModel, 0xC68B OriginalRawFileName and 0xC634 DNGPrivateData (Adobe MakN, a copy of the
   MakerNote with serials) fall through `by_id` to `None`, so no category and no preset names them. Under
   `Everything`, ExifTool still prints UniqueCameraModel, LocalizedCameraModel and OriginalRawFileName.
2. `tiff-ifd1-strips-is-a-page` (M4). The audit calls an IFD1 with strips a `Thumbnail` in every container.
   In a JPEG's Exif block that is right (an uncompressed TIFF thumbnail). In a bare TIFF it is a page, so the
   three presets that include `Thumbnail` (AllPii, All, Everything) refuse every multi-page TIFF with
   `ThumbnailNotLocated`. Fail-closed, but unusable on scans and faxes. A tiled IFD1 page is refused too,
   though the audit does not call it a `Thumbnail` (DEC-013 amendment D.3).

Governing code: `src/audit/category.rs` (`categorize`, `by_id`), `src/audit/mod.rs` (`add_container`, the
IFD1 rule at the `TAG_STRIP_OFFSETS` match, `audit_tiff` passes `Origin { block: None, .. }`),
`src/write/exif.rs` (`select`, `plan_block`, `THUMB_IFD_TAGS`, the IFD1 whole-removal block),
`src/write/tiff.rs`. The audit is the oracle for the writer (DEC-013), so the audit change comes first and the
writer must follow it; the writer never invents categories.

## Goal

Every DNG identity tag is named by a DEC-008 category, so `Everything` (and the presets that include the
category) removes it and ExifTool agrees; and a multi-page or tiled-page bare TIFF can be stripped under
`AllPii`, `All` and `Everything` with its pages kept byte-identical.

## Decisions made here (the design; the DEC amendment records them)

**Part 1, categories.** Add to `by_id` (they apply in any non-GPS, non-Interop IFD, as the other DNG tag
0xC62F already does):

| Tag | Name | Category | Why |
|---|---|---|---|
| 0xC614 | UniqueCameraModel | `CameraModel` | same class as Make/Model; Low; in AllPii, All, Everything |
| 0xC615 | LocalizedCameraModel | `CameraModel` | same |
| 0xC68B | OriginalRawFileName | `FreeText` | a file name a person chose and can contain a name; not a device serial, so not `DeviceId` (High, and in `GpsAndDevice`) |
| 0xC634 | DNGPrivateData | `MakerNote` | Adobe MakN is a copy of the MakerNote and carries its serials; handled as the MakerNote is: removed whole, never edited |

Residual stated, not hidden: `GpsAndDevice` still leaves 0xC634 (as it leaves 0x927C), because `MakerNote` is
not in that preset (DEC-009). A DEC-008 note says so.

**Part 2, the page rule.** The thumbnail rule becomes container-aware. In a bare TIFF (the `audit_tiff` path,
`Origin.block == None`) an IFD1 is a `Thumbnail` only when it holds the JPEG pair 0x0201/0x0202 and no strip
or tile tags (`0x0111`, `0x0117`, `0x0144`, `0x0145`). An IFD1 that holds strips or tiles is a page: its
0x0111 is not categorised `Thumbnail`, and neither is anything else about it. In a JPEG's Exif block the rule
is unchanged (an IFD1 with strips and no 0x0201 is still a `Thumbnail`). Writer consequences:

- `select` sets `thumbnail` only when the audit has a `Thumbnail` finding; a kept page IFD1 is never a
  whole-IFD removal. Its entries still go by their own categories (a Make or DateTime inside a page IFD1 is
  deleted by its tag like in IFD0), and its strip and tile data are protected, as `protected_ranges` already
  does for every IFD.
- The DEC-013 D.3 refusal (`ThumbnailNotLocated`, strips form) is then reachable only from a JPEG Exif IFD1;
  amend D.3 to say so. A JPEG-in-TIFF thumbnail IFD1 (0x0201/0x0202, no strips) is unchanged and still
  removed whole.
- Edge cases decided: an IFD1 holding both the JPEG pair and strips is NOT a thumbnail-removable IFD1 and not
  a page either; the audit reports it as it does today (`Thumbnail`) and the writer keeps refusing it
  (`ThumbnailNotLocated`). Only the clean cases change behaviour.

## Inputs

- `guidance/questions.yaml` entries `dng-identity-tags-have-no-category`, `tiff-ifd1-strips-is-a-page`.
- `decisions/DEC-008-audit-categories-and-severity.md`, `DEC-012`, `DEC-013` (amendments C, D.3).
- The code listed in Context. Fixtures: `tests/support/tiff_shapes.rs` (stripped, tiled, multipage, thumbnail,
  dng, jumbf). ExifTool oracle records under `tests/oracle-fixtures/exiftool-strip-tiff/` and the replay and
  red-proof pattern of `tests/strip_tiff_oracle.rs`.

## Outputs

- **Modified:** `src/audit/category.rs` (`by_id` rows; table-order and doc comments), `src/audit/mod.rs`
  (the IFD1 rule keyed on the container), `src/write/exif.rs` (`select` and the IFD1 block follow the audit),
  `src/write/tiff.rs` doc comment, `src/audit/preview.rs` if it lists the thumbnail rule.
- **Decisions:** a DEC-008 amendment (the four rows, the residual, the container-aware rule) and a DEC-013
  D.3 amendment, each short, in the same files.
- **Fixtures** (one generator script, per the budget rule): `dng_identity(be)` with all four tags in IFD0 and
  a SubIFD; `multipage` already exists; add a tiled IFD1 page and a page-plus-JPEG-pair IFD1 case.
- **Records:** ExifTool outputs for the new fixtures under `tests/oracle-fixtures/exiftool-strip-tiff/`.
- **questions.yaml:** both entries to `answered` with the implementing spec named; `README.md` known-limits
  bullet removed.

## Acceptance Criteria

- [x] `categorize` and the audit report the four DNG tags under the categories in the table, in IFD0 and in a
      SubIFD, both byte orders.
- [x] `strip(dng_identity, Everything)` and `AllPii` leave none of the four tags (entries deleted, out-of-line
      bytes zeroed); `GpsAndDevice` and `Gps` leave them byte-identical; the strip, tile and SubIFD data stay
      byte-identical; the self-check passes.
- [x] ExifTool on the `Everything` output of the DNG fixture prints none of IFD0:UniqueCameraModel,
      LocalizedCameraModel, OriginalRawFileName (the case recorded in the question), and prints
      DNGPrivateData nowhere.
- [x] In a bare TIFF, an IFD1 with strips or tiles yields no `Thumbnail` finding; in a JPEG Exif block an IFD1
      with strips and no 0x0201 still does.
- [x] `strip(multipage, AllPii | All | Everything)` returns bytes: page 2's strips are byte-identical, its
      IFD1-held Make/DateTime entries (if any) are deleted per policy, and the JPEG-pair thumbnail case still
      removes IFD1 whole.
- [x] A tiled IFD1 page strips under the same presets (it was refused before).
- [x] An IFD1 with both the JPEG pair and strips is still refused (`ThumbnailNotLocated`).
- [x] Idempotence holds: stripping the output again is a no-op.
- [x] Every existing SPEC-020 to SPEC-022 test passes; any that asserted the old refusal is changed in this
      spec and the change is named in the handoff.

## Failing Tests

Written first, red before the code changes. Mutated-copy red-proofs for the oracle rows (revert the `by_id`
row, or restore the old IFD1 rule, and the oracle test fails).

- **`src/audit/category.rs` unit tests**
  - `"dng identity tags categorize"` — asserts: 0xC614/0xC615 CameraModel, 0xC68B FreeText, 0xC634 MakerNote,
    in Ifd0 and in a SubIFD kind.
- **`tests/audit_model.rs`**
  - `"bare tiff ifd1 with strips is a page"` — asserts: no `Thumbnail` finding for `multipage` and for the tiled
    IFD1 shape; one finding for the JPEG-pair shape.
  - `"jpeg exif ifd1 with strips stays a thumbnail"` — asserts: the JPEG rule is unchanged (guards the
    container-awareness).
- **`tests/strip_tiff.rs`**
  - `"dng identity tags go under everything and all_pii, stay under gps_and_device"` — asserts the AC above, both
    byte orders, with protected ranges identical.
  - `"multipage tiff strips under all_pii all everything"` and `"tiled page ifd1 strips"` — assert Ok, pages
    identical, idempotent.
  - `"ifd1 with jpeg pair and strips is still refused"` — asserts `ThumbnailNotLocated`.
  - Replace the existing multi-page refusal test, naming it in the handoff.
- **`tests/strip_tiff_oracle.rs`**
  - `"exiftool agrees on the dng identity fixture after everything"` and
    `"exiftool agrees on the multipage fixture after all_pii"` with replay records and a red-proof each.
- **`fuzz/` seeds:** add the three new shapes to `examples/fuzz_seeds.rs`; the `rewrite_tiff` smoke must stay
  clean (60 s).

## Non-Goals

- Parsing DNGPrivateData or the MakerNote, or any MakerNote editing. The blob is removed whole or left.
- PNG, WebP, HEIC. `set` operations. Other DNG tags (0xC62F serials already categorised; DefaultCrop and the
  like are structural, not privacy).
- Changing what `GpsAndDevice` contains (DEC-009).
- The audit-side blob-carrier parsing question (`audit-blob-carrier-parsing`) and the other open ExifTool
  divergences.

## Notes for the Implementer

- Audit first, writer second. Do not special-case DNG in the writer: the tags must be removed because the
  audit names them, through the existing entry-deletion path. If `plan_block` refuses 0xC634 or any of the four
  (an out-of-line value that overlaps something), that is a finding to report in the handback, not to bypass.
- The container discriminator already exists: `audit_tiff` builds the `Origin` with `block: None`; the JPEG path
  uses `Some(n)`. Use that, not a new flag, and keep the per-IFD context computed once per IFD (a hostile IFD
  can hold 65,535 entries; no per-entry scans).
- `THUMB_IFD_TAGS` is the allow-list for a removed IFD1. Leave it alone; the page IFD1 is kept, not removed.
- Zero panics on untrusted input; five-lint deny set; `forbid(unsafe_code)`. No new dependency.
- Budget: one builder, sonnet, about 100 tool calls or 250k context. Generate the fixtures with one script.
  Cold verify on opus, one round; a second only for High or Medium findings.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — Put the two out-of-spec edits the build needed (the `verify_tiff` entries self-check and the independent fuzz model's IFD1 rule) into the design: both follow from "the audit is the oracle", and the second weakens the independence of the model unless it is checked, which the verifier had to do. Also give the fuzz audit seed set a DNG-identity seed in the design, not as a finding.

2. **Does any template, constraint, or decision need updating?**
   — DEC-008 amended (four DNG rows, the GpsAndDevice residual, the container-aware IFD1 rule) and DEC-013 D.3 narrowed to JPEG. Process: both agents again self-reported token counts that include cache reads (verify 4,802,409 vs harness 179,858); record only the harness figure. `just lint` needs `RUSTUP_TOOLCHAIN=1.90.0` when `~/.cargo/bin` is first in PATH.

3. **Is there a follow-up spec I should write now before I forget?**
   — Two Low test gaps, small enough for a patch: (L1) no fixture puts Make, DateTime or a GPS pointer inside a page IFD1, so "a page's identity entries go by category" is verified only by the verifier's throwaway probe; (L2) no `fuzz/seeds/audit/` file carries 0xC614, 0xC615, 0xC68B or 0xC634, so the fuzz model's four new rows are never run by a seed (and the replay oracle cannot see a library regression, only the live oracle tests can). L3 (DEC-008 wording) was fixed at ship. **L1 and L2 were fixed afterwards on the same PR** (no new spec): a `page_identity` shape (an IFD1 strips page holding Model, DateTime, Artist and a GPS pointer) is in `all()`, with ExifTool records and rewrite_tiff seeds; three audit fuzz seeds (`audit-dng-identity-le/be`, `audit-page-identity-le`) carry the four DNG tags and a page IFD1. Reverting the four `by_id` rows now turns `audit_harness_holds_on_every_seed_and_degenerate_inputs` red. The seed-count pins in six test files moved (audit seeds 78 to 81, distinct inventory inputs 180 to 183). Cost of the follow-up is not recorded as a session: orchestrator work, un-metered.

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — build
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
   — Before: `Everything` left DNG UniqueCameraModel, LocalizedCameraModel and OriginalRawFileName in a file and every multi-page or tiled-page TIFF was refused under `AllPii`, `All` and `Everything`; after: both strip, with page image data byte-identical and ExifTool printing none of the four DNG identity tags (1,075 differential runs, 0 violations).
