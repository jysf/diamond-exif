---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-014
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: M             # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: approved  # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.

project:
  id: PROJ-001
  stage: STAGE-003
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-008, DEC-009]    # [DEC-NNN, DEC-MMM]
  constraints: [panic-free-library, forbid-unsafe-code, no-io-in-library, fuzz-target-per-surface, test-before-implementation, oracle-must-be-shown-red, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-012, SPEC-015, SPEC-016]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-012]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-003's <capability>". Optional; null is acceptable.
value_link: "STAGE-003: what each strip policy would remove, stated once before any writer exists, and never reported as safe for a block the library cannot read"

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
  tokens_estimate: 400000
  sessions:
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 268470
      estimated_usd: 1.77
      duration_minutes: 18
      recorded_at: 2026-11-06
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result (subagent_tokens 268,470; 1,094,453 ms wall-clock)"                      # one line if unusual (rework, no meter, etc.)
    - cycle: verify
      agent: claude-opus-5-5
      interface: other
      tokens_total: 185245
      estimated_usd: 1.22
      duration_minutes: 12
      recorded_at: 2026-11-06
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result (subagent_tokens 185,245; 699,500 ms wall-clock); the verifier's own 40-minute estimate is not used"
  totals:
    tokens_total: 453715
    estimated_usd: 2.99
    session_count: 2
shipped_at: 2026-11-06
---

# SPEC-014: StripPolicy presets as data and the policy preview; DEC for preset semantics

## Context

SPEC-012 gave every finding a DEC-008 category. This spec adds the other half of what the audit tells a user: **what a strip policy would do to those findings**, before any writer exists. DEC-009 (written in this design, normative; read it first) decides the preset contents and the preview's three dispositions. The shape is deliberately small: a policy is a **set of categories**, a preset is a named set, and the preview is a lookup of each finding's category in that set, plus one honest-limit rule for blocks the library cannot read (`KeptUnread`). Everything the later work needs hangs off it: SPEC-015 renders dispositions, SPEC-016 can turn `KeptUnread` and residual severity into exit codes, and the writer (M1c/M1d) takes the same `StripPolicy` and must do exactly what the preview promised.

Three design points this spec fixes (the reasons are in DEC-009):

1. **Presets are nested category sets**, `Gps` ⊂ `GpsAndDevice` ⊂ `AllPii` ⊂ `All` ⊂ `Everything`. `AllPii` removes `Thumbnail` and `MakerNote` (design section 5 does not list them; DEC-009 explains why a PII preset must). `All` keeps `Icc` and `Jumbf`; `Everything` removes them.
2. **A block that is kept but could hold what the policy removes is `KeptUnread`**, not `Kept`. Under `Gps`, an XMP packet, a thumbnail, a MakerNote and a JUMBF box are all `KeptUnread`. The preview never says a file is safe because a policy "covers" a category the library could not look inside.
3. **The preview is over findings only.** Inventory-only entries (no category) are out of scope; the DEC records why and the question file carries it.

## Goal

`StripPolicy` and `StripSpec` as plain data in the library; `preview(&FileAudit, StripPolicy) -> Preview` giving one disposition per finding (same order), counts, residual findings, residual severity and a `reliable` flag; derived from `Category` and DEC-009 only, so the audit, the preview and the writer cannot drift. Hand-typed expectation matrices, a nesting property, fixture tests over every preset, an extension of the existing `audit` fuzz harness, and a mutant list each shown red.

## Inputs

- **Reads:** DEC-009 (normative), DEC-008, `src/audit/{mod,category}.rs`, `src/lib.rs`, `tests/support/{audit_tags,oracle_tiff,oracle_jpeg}.rs` (fixture builders), `fuzz/shared/audit_invariants.rs`, `fuzz/fuzz_targets/audit.rs`, `tests/fuzz_layout.rs`, design `docs/exif-core-design.md` sections 5 and 7.
- **Does not read:** ExifTool source. ExifTool is **not run** by this spec: presets are this library's own contract, there is nothing to compare (see Non-Goals).
- Constraints `panic-free-library` (five-lint deny set), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (the existing `audit` target is extended; no new input surface), `test-before-implementation`, `oracle-must-be-shown-red` (applied to our own tests: every mutant below), `no-new-top-level-deps-without-decision`. No dependencies.

## Outputs

### Public API

New file `src/policy.rs` (`pub mod policy;` in `src/lib.rs`, with `StripPolicy` and `StripSpec` re-exported at the crate root), new file `src/audit/preview.rs` (re-exported from `exif_core::audit`), one method added to `Category` in `src/audit/category.rs`. `audit()` and everything else in `src/` is unchanged.

```rust
// src/policy.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct StripSpec { /* private: a bit set over Category::ALL */ }
impl StripSpec {
    pub const fn none() -> StripSpec;
    pub fn all() -> StripSpec;                       // every category in Category::ALL
    pub fn include(self, c: Category) -> StripSpec;  // builder, by value
    pub fn exclude(self, c: Category) -> StripSpec;
    pub fn contains(self, c: Category) -> bool;
    pub fn len(self) -> usize;
    pub fn is_empty(self) -> bool;
    pub fn is_subset_of(self, other: StripSpec) -> bool;
    pub fn intersects(self, other: StripSpec) -> bool;
    pub fn categories(self) -> impl Iterator<Item = Category>;   // in Category::ALL order
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StripPolicy { Gps, GpsAndDevice, AllPii, All, Everything, Custom(StripSpec) }
impl StripPolicy {
    pub const PRESETS: [StripPolicy; 5];             // Gps, GpsAndDevice, AllPii, All, Everything, in that order
    pub fn spec(self) -> StripSpec;                  // DEC-009 table, one column per preset
    pub fn name(self) -> &'static str;               // "gps" "gps_and_device" "all_pii" "all" "everything" "custom"
}

/// DEC-009 may-hold relation: the categories a (block) category can contain. Empty for tag categories and for `Icc`.
pub fn may_hold(block: Category) -> StripSpec;

// src/audit/category.rs
impl Category { pub fn is_block(self) -> bool; }     // Thumbnail, MakerNote, Xmp, Iptc, Jumbf, Icc

// src/audit/preview.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition { Removed, Kept, KeptUnread }
impl Disposition { pub fn name(self) -> &'static str; }          // "removed" "kept" "kept_unread"

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts { pub removed: usize, pub kept: usize, pub kept_unread: usize }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview { /* private */ }
impl Preview {
    pub fn policy(&self) -> StripPolicy;
    pub fn dispositions(&self) -> &[Disposition];                // parallel to FileAudit::findings(), same order
    pub fn disposition(&self, finding: usize) -> Option<Disposition>;
    pub fn counts(&self) -> Counts;
    pub fn residual_findings(&self) -> usize;                    // kept + kept_unread
    pub fn residual_severity(&self) -> Option<Severity>;
    pub fn reliable(&self) -> bool;
}
pub fn preview(audit: &FileAudit<'_>, policy: StripPolicy) -> Preview;   // total: never fails, never panics
```

(Names and exact shapes are the builder's to refine inside these constraints; the rules below and the named tests are normative. `StripSpec` has no public constructor from a raw integer: a caller can only build one from categories.)

### Rules (normative)

1. **Presets are DEC-009's columns.** `Gps.spec()`, `GpsAndDevice.spec()`, `AllPii.spec()`, `All.spec()`, `Everything.spec()` are exactly the sets in the DEC-009 table. `Everything.spec() == StripSpec::all()`. `Custom(s).spec() == s`.
2. **Disposition.** For a finding of category `c` under spec `s`: `Removed` iff `s.contains(c)`; otherwise `KeptUnread` iff `c.is_block()` and `may_hold(c).intersects(s)`; otherwise `Kept`. No other input is consulted (not the severity, not the container, not the origin).
3. **Shape.** `dispositions().len() == audit.findings().len()` and `dispositions()[i]` is the disposition of `findings()[i]`, for every audit including empty and truncated ones. `counts().removed + kept + kept_unread == findings().len()`. `residual_findings() == kept + kept_unread`.
4. **Residual severity.** When the audit's findings are complete (`!health().inventory_truncated`): the maximum `Category::severity()` over the findings whose disposition is not `Removed`, `None` if there are none. When truncated: if `s == StripSpec::all()` then `None`, otherwise the audit's own `max_severity()` (dropped findings' categories are unknown, so none is assumed removed; DEC-009).
5. **Reliable.** `reliable()` is `health().clean && !health().inventory_truncated`. An unsupported, empty, unreadable or truncated audit is never reliable; its preview has zero findings (or the truncated subset) and the doc comment says "zero residual findings supports 'nothing found would remain' only when `reliable()`".
6. **Nesting and monotonicity.** For every audit and each consecutive pair in `PRESETS`, `removed` is non-decreasing and `residual_findings` is non-increasing; `Everything` gives `residual_findings() == 0` and `residual_severity() == None` for every audit. `Custom(StripSpec::none())` removes nothing and targets nothing, so `may_hold(c).intersects(none)` is false and every finding is `Kept`, blocks included (the test pins this one case of the `KeptUnread` rule).
7. **Tag categories are never `KeptUnread`.** Only the six block categories can be.
8. **Multi-block.** The same category in several Exif blocks of a JPEG gives the same disposition for each copy (the policy is per category, not per position).
9. **Determinism, totality, no I/O, no dependencies.** `preview` is a pure function of `(audit, policy)`; work is linear in the number of findings; no allocation beyond one `Vec<Disposition>`.
10. **Names are API.** The six policy names and three disposition names above are pinned by a test (SPEC-015 and SPEC-016 print and parse them).

### The expectation matrices (typed into the tests by hand; not generated from the code)

`REMOVED[preset][category]` is the DEC-009 table (the 5 x 13 grid, "removed" cells exactly as in the DEC). `KEPT_BLOCK[preset][block]` is, for the six block categories that a preset keeps:

| Block | `Gps` | `GpsAndDevice` | `AllPii` | `All` | `Everything` |
|---|---|---|---|---|---|
| `Thumbnail` | KeptUnread | KeptUnread | Removed | Removed | Removed |
| `MakerNote` | KeptUnread | KeptUnread | Removed | Removed | Removed |
| `Xmp` | KeptUnread | KeptUnread | KeptUnread | Removed | Removed |
| `Iptc` | Kept | Kept | KeptUnread | Removed | Removed |
| `Jumbf` | KeptUnread | KeptUnread | KeptUnread | KeptUnread | Removed |
| `Icc` | Kept | Kept | Kept | Kept | Removed |

The tests carry both grids as literals, plus a third literal, the may-hold table of DEC-009, so a change to any of them in the code fails a named cell.

### Fixtures

One **full-house** input per container, built with the existing `tests/support` builders (the build chooses which and reports): a JPEG with one Exif block holding, across IFD0, ExifIFD, GPS and IFD1, at least one tag of each of the seven tag categories (`Gps`, `DeviceId`, `Authorship`, `FreeText`, `Timestamps`, `CameraModel`, `Software`) plus a MakerNote and an IFD1 thumbnail, and one region each of Xmp, Iptc, Icc and Jumbf; and the same Exif as a bare TIFF (no regions). For each preset the test computes the preview and asserts, per finding, `REMOVED`/`KEPT_BLOCK`; and the set of removed categories equals the DEC-009 column. Further inputs: a two-Exif-block JPEG (rule 8), an Exif block after the scan, an unsupported container (a PNG signature), an empty input, a TIFF with unreadable IFD0, and a hand-built truncated `FileAudit` (constructed inside `src/audit/preview.rs` tests, where the private fields are visible) for rule 4.

### Fuzz

No new target and no new input surface: `preview` takes a `FileAudit`, and `audit(bytes)` is already fuzzed. The existing harness (`fuzz/shared/audit_invariants.rs`, run by `fuzz/fuzz_targets/audit.rs` and `tests/fuzz_layout.rs`) gains guards, each a pure function run **alone** through `run_guard` (HANDOFF-009), each `iff` checked in both directions, re-derived in the harness from DEC-009 written again as data (the harness does not call `StripPolicy::spec`, `may_hold` or `is_block`):

- `preview_shape`: lengths, counts sum, residual equals kept plus kept-unread.
- `preview_dispositions`: for each of the five presets and one `Custom` spec chosen deterministically from the input bytes, every disposition equals the independently re-derived one (an expected `Removed` that is missing and an unexpected `Removed` both fail; same for `KeptUnread`).
- `preview_nesting`: removed non-decreasing and residual non-increasing along `PRESETS`; `Everything` leaves nothing.
- `preview_residual_and_reliable`: residual severity per rule 4 (re-derived), `reliable` iff the audit's `clean` and not truncated.

Each guard gets plants in `tests/fuzz_layout.rs` in both directions, and the guards stay linear (the SPEC-013 F5 step counter covers them: they `tick` per element). `lint-fuzz-red-proof` is run locally.

## Acceptance Criteria

- `StripSpec`, `StripPolicy`, `may_hold`, `Category::is_block`, `preview` exist as above; every rule holds; the matrices in the tests equal DEC-009 cell by cell; the code equals the matrices.
- The nesting chain holds on all fixtures and on every committed seed (29 TIFF, 37 JPEG, the audit seeds) for all five presets (a test that runs `audit` then `preview` on each seed).
- A full-house input shows, for each preset, exactly the DEC-009 outcome; the `Gps` preview of the full-house JPEG reports `KeptUnread` for the thumbnail, MakerNote, Xmp and Jumbf and `Kept` for Iptc and Icc; the `All` preview reports `KeptUnread` for Jumbf only.
- Unsupported, empty and unreadable inputs: zero dispositions, `reliable() == false`.
- `Category::ALL` has at most 16 entries (a test; the bit set is a `u16`), the bit of each category is distinct (13 distinct bits), and a category missing from the DEC-009 grid fails `matrix_covers_every_category`.
- Mutants: every mutant in the list below is applied by hand, shown to fail at least one named test (the report names which), and reverted; the tree after reverting is byte-identical to the build commit.
- `src/audit/mod.rs` `audit()` and every existing test's expectations are unchanged; the existing 66 seeds and all committed records are byte-identical; no file under `decisions/`, `scripts/` or irradiance is touched.
- Gates: fmt (both graphs), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof` (run locally; CI-only otherwise), `cargo test` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `fuzz-smoke audit` (60 s clean, executions and coverage recorded), `oracle-record` with a clean `git status`. No dependencies.

## Failing Tests

Named tests (a rename, fold or missing test is a deviation to report):

**src/policy.rs (unit):**
- `strip_spec_has_one_distinct_bit_per_category` (and `category_all_fits_in_sixteen_bits`)
- `strip_spec_none_all_include_exclude_contains_len`
- `strip_spec_categories_iterate_in_category_all_order`
- `strip_spec_subset_and_intersects`
- `each_preset_spec_equals_its_dec_009_column` (hand-typed `REMOVED` grid)
- `presets_are_nested_and_everything_is_every_category`
- `custom_spec_is_returned_unchanged`
- `may_hold_equals_the_dec_009_table` (hand-typed, six block rows, tag categories empty)
- `policy_and_disposition_names_are_pinned`
- `matrix_covers_every_category` (every `Category::ALL` entry appears in the grid)

**src/audit/category.rs (unit):**
- `is_block_is_exactly_the_six_block_categories`

**src/audit/preview.rs (unit):**
- `disposition_rule_matches_the_kept_block_grid` (hand-typed `KEPT_BLOCK`)
- `tag_categories_are_never_kept_unread`
- `custom_none_keeps_everything_and_flags_nothing_unread`
- `custom_all_equals_everything`
- `counts_sum_to_findings_and_residual_is_kept_plus_unread`
- `residual_severity_is_the_max_over_non_removed_findings`
- `truncated_audit_residual_severity_is_conservative` (hand-built `FileAudit`; includes the `all()` exception)
- `unsupported_empty_and_unreadable_audits_have_no_dispositions_and_are_not_reliable`
- `preview_is_deterministic`

**tests/policy_preview.rs (integration):**
- `full_house_jpeg_matches_dec_009_for_every_preset` and `full_house_tiff_matches_dec_009_for_every_preset`
- `the_removed_categories_of_each_preview_equal_the_dec_009_column`
- `gps_preview_of_the_full_house_reports_the_unread_blocks`
- `all_preview_reports_only_jumbf_unread`
- `everything_leaves_no_residual_findings_on_any_reliable_fixture`
- `two_exif_blocks_give_the_same_disposition_per_copy`
- `exif_block_after_the_scan_is_previewed_like_any_other`
- `nesting_holds_on_every_committed_seed_for_every_preset`
- `preview_never_panics_on_every_committed_seed`

**tests/fuzz_layout.rs and the harness:**
- `preview_guards_hold_on_every_seed_and_degenerate_inputs`
- `each_preview_guard_goes_red_alone_when_planted` (table-driven, both directions, one plant per guard minimum, two where a guard has an iff)
- `preview_guards_are_linear` (the step-counter test extended)

## Mutants (each shown red by hand, then reverted)

The report lists, for each, the test or guard that failed:

1. `Gps` preset also contains `DeviceId`.
2. `GpsAndDevice` loses `Gps`.
3. `AllPii` loses `MakerNote`.
4. `AllPii` gains `Xmp`.
5. `AllPii` gains `Software`.
6. `All` loses `Software`.
7. `All` gains `Icc`.
8. `Everything` loses `Jumbf`.
9. `may_hold(MakerNote)` loses `Gps`.
10. `may_hold(Iptc)` gains `Gps`.
11. `may_hold(Icc)` becomes non-empty.
12. `is_block` returns true for `Timestamps` (a tag category goes `KeptUnread`).
13. The disposition rule checks `may_hold(c)` against the wrong operand (`s.contains` where `intersects` is meant).
14. Residual severity ignores `KeptUnread` findings.
15. The truncated rule is dropped (residual is the computed one only).
16. The truncated `all()` exception is dropped.
17. `reliable` is constant true.
18. `kept_unread` is counted as `kept`.
19. Dispositions come out in reverse order.
20. Two categories share one bit in `StripSpec`.
21. `exclude` is a no-op.
22. Two policy names swapped.

## Non-Goals

- Any byte rewriting, `EditPlan`, `sanitize()`, `strip()` or `write_back()` (M1c/M1d); how a block is removed
- A preview of inventory-only (unclassified) entries (question `preview-unclassified-entries`)
- Per-tag, per-IFD or per-origin policies; a policy language; reading a policy from a file or flag
- Rendering and CLI wiring (SPEC-015, SPEC-016)
- Reading inside a thumbnail, MakerNote parsing, XMP/IPTC/C2PA decoding (they would turn `KeptUnread` into something finer, a later spec)
- An ExifTool comparison: ExifTool's `-all=` family has no presets that correspond to ours, so there is nothing sound to compare; the writer stages compare *outputs* to ExifTool strips (design section 8)

## Notes for the Implementer

- Order of work, test-first: `StripSpec` and its unit tests (bits, builder) -> `Category::is_block` -> `StripPolicy` presets with the hand-typed grid -> `may_hold` -> `preview` with the kept-block grid -> integration fixtures -> the fuzz guards and plants -> mutants.
- Type the three literal grids (removed, kept-block, may-hold) **from DEC-009 by hand**, not from the code under test, and type the harness's copy a second time. The point of three copies is that a wrong cell in the code is caught by two independent ones.
- `arithmetic_side_effects` forbids `<<` and `+` on integers in the library. Give each category its bit as a literal constant in a `match` (as `categorize` does for ids), count with an iterator or `saturating_add`, and keep the shifting-free design.
- The full-house builder lives in `tests/support` (test code only) and must not call `audit` or `categorize` to build its expectations.
- `Preview` stores one `Vec<Disposition>`; compute the severity in one pass over the findings.
- Run `lint-fuzz-red-proof` locally; it broke PR 14.
- Report in the Handback: every deviation, each mutant with the test that caught it, the fuzz smoke numbers, and any doubt about a DEC-009 cell (a doubt is a question for me, not an edit: do not touch `decisions/`).
- Do not push or open a PR; do not advance cycles.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — Count the committed seeds before writing a number into a spec (I wrote 29 TIFF seeds; there are 32), and check that a mutant's wording type-checks (mutant 13 did not). I would also have put the two full-house inputs in `fuzz/seeds/audit` in the design, since a harness guard is only as strong as the categories its seeds reach (verify F1).

2. **Does any template, constraint, or decision need updating?**
   — DEC-009 gained an amendment (Icc is kept by decision; read `reliable` first; may-hold gaps recorded as a question). Lesson, N=1: a fuzz guard over a classification needs seeds that reach every class, or it proves nothing about the unreached ones. Not codified at N=1.

3. **Is there a follow-up spec I should write now before I forget?**
   — No new spec. Verify findings F1 to F3 and the renderer wording are carried into SPEC-015; the may-hold gaps and unclassified-entry preview are questions.

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — build
   *(The builder found that mutants 1 and 3 passed the fuzz harness because no seed held a DeviceId or MakerNote finding, and fixed it with full-house inputs; verify found the remaining reach gap in the libFuzzer target.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — Run `StripPolicy` presets as data and `preview` on an audit to see, per finding, whether a policy removes it, keeps it, or keeps a block it cannot read that may hold what the policy targets (before: no statement of what a strip would do; after: five nested presets, 22 mutants shown red, and a preview that never reads an unsupported, unreadable or truncated file as clean).
