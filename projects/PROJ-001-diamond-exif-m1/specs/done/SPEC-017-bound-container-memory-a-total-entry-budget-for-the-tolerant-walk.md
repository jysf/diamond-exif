---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-017
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: S                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: M          # stamped at ship: what it ACTUALLY took, same scale.
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
  decisions: [DEC-001, DEC-005, DEC-002]                    # [DEC-NNN, DEC-MMM]
  constraints: [panic-free-library, forbid-unsafe-code, fuzz-target-per-surface, test-before-implementation, oracle-must-be-shown-red, no-new-top-level-deps-without-decision]                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-012, SPEC-016]                # [SPEC-NNN]

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
value_link: "STAGE-003: the reader can be pointed at untrusted files in bulk without a 790 KB file costing 950 MB"

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
  tokens_estimate: 300000
  sessions:
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 283432
      estimated_usd: 1.87
      duration_minutes: 25
      recorded_at: 2026-11-05
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result" 
    - cycle: verify
      agent: claude-opus-5-5
      interface: other
      tokens_total: 221119
      estimated_usd: 1.46
      duration_minutes: 16
      recorded_at: 2026-11-06
      notes: "verdict APPROVED at e44659a; tokens transcribed by the orchestrator from the sub-agent metered result (the harness marked that result possibly interim)"
  totals:
    tokens_total: 504551
    estimated_usd: 3.33
    session_count: 2
shipped_at: 2026-11-06
---

# SPEC-017: Bound Container memory: a total entry budget for the tolerant walk

## Context

Found and measured by the SPEC-012 build and re-measured in this design: `Container::parse` bounds IFD *count* (`MAX_IFDS` = 512 visits) and each IFD's table by the input length, but nothing bounds the **total** entries kept. Overlapping IFD tables (distinct offsets, same bytes) let 511 SubIFDs each claim 65,535 entries inside the same ~790 KB: it parses **cleanly** to 33,488,386 entries, about 804 MB at 24 bytes per entry (~1,018x the input), ~950 MB peak, 103 ms, with no issue. A server parsing uploads, and SPEC-016's directory scanner, cannot accept that.

**Design-time measurement** (ExifTool 13.55, macOS, each call under `perl -e 'alarm 25'`, the same file shape built by `hostile.py`: IFD0 with one `SubIFDs` entry of N offsets two bytes apart into `0xFF` bytes, next-words zero): N = 8, 60 and 511 all return in 0.04-0.05 s at about 22 MB resident. For N = 60 and 511 it prints `Warning: Ignoring 50 / 501 SubIFD directories` (it keeps the first 10); for N = 8 it prints `Warning: Bad format (65535) for SubIFD entry 0` and reads nothing. So the reference tool is not exposed to this shape; this library is, and the fix is ours. We do **not** copy ExifTool's 10-SubIFD limit (a different policy; our SubIFD cap stays `MAX_IFDS`, DEC-005).

**The rule that falls out of the format** (DEC-001 clarification written in this design, normative): a real file's IFD tables are disjoint and each entry occupies its own 12 bytes, so the total of retained entries times 12 can never exceed the file length. The budget is exactly that: `entry_budget = data.len() / 12`. No magic constant to tune, memory linear in the input (at most `2 x len` bytes of entries), and real files sit far below it (sampled local photos carry 49-63 IFD entries in total, against tables that are a small fraction of an Exif blob of kilobytes).

## Goal

The tolerant walk stops retaining entries once their total would exceed `data.len() / 12`: the IFD that would cross the budget is omitted atomically, one `TooManyEntries` issue is recorded, and the walk halts (everything recorded so far stays readable). The 789 KB file above becomes a typed outcome with about `len/12` entries at most. Nothing about any well-formed or merely damaged file changes.

## Inputs

- **Reads:** `src/tiff/mod.rs` (`Walk`, `follow`, `read_ifd`, `MAX_IFDS`, `MAX_ISSUES`, `ENTRY_SIZE`), `src/error.rs`, `src/tiff/issue.rs`, DEC-001 (incl. its SPEC-017 clarification), DEC-005
- **Tests that move:** `tests/audit_oracle.rs::container_amplification_is_measured_and_reported` (and its bounded in-test version), the audit fuzz seed `audit-many-overlapping-ifds`, anything that built overlapping tables on purpose
- Constraints: `panic-free-library` (five lints, `checked`/`saturating` arithmetic), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red`, no dependencies

## Outputs

### Library (`src/tiff/mod.rs`, `src/error.rs`)

- `Error::TooManyEntries { limit: usize }` (additive on the `#[non_exhaustive]` enum), `Display`: "IFD walk reached the limit of {limit} retained entries". Extend the exhaustive error-display guard tests.
- `pub fn entry_budget(data_len: usize) -> usize` (`data_len / ENTRY_SIZE`, `ENTRY_SIZE` stays private or becomes a documented constant) and `Container::entry_count(&self) -> usize` (total entries retained), both documented.
- `Walk` gains `entries: usize` (entries of recorded IFDs, IFD0 included). In `follow`, **before decoding any entry**, read the IFD's declared count and, if `walk.entries + count > budget`, record `TooManyEntries { limit: budget }` (link, from_ifd, `Some(offset)`), omit the IFD, set `walk.halted`, return `None`. The count check happens before the `Vec<Entry>` is allocated, so the omitted IFD allocates nothing (the same discipline as the SubIFDs count cap, DEC-005). IFD0 goes through the same accounting (it cannot exceed the budget: a table past the end is already `Truncated`, so no special case, but it is counted).
- `MAX_ISSUES` is unchanged (the halting issue is `TooManyEntries` or `TooManyIfds`, never both, and it occupies a budgeted attempt); a test pins the bound.
- Doc updates: `Container::parse`, the module limits section, `MAX_ISSUES`.

### Fixtures and records

- Hostile and boundary fixtures are **generated in tests** (the 790 KB file is not committed): `hostile(n)` as in `tests/audit_oracle.rs`, plus a hand-built overlapping pair whose totals sit exactly on and one over the budget.
- One oracle record `tests/oracle-fixtures/exiftool/hostile-subifds-overlap.txt` for the N = 511 shape (small: ExifTool prints dashes plus the SubIFD warning in `# stderr:`; the file is regenerated at record/live time). The existing 120+1 records and 66+7 seeds stay byte-identical, and every existing parity test passes unchanged. If any existing fixture, seed or record changes behaviour, that is a **finding to report**, not something to paper over.

### Fuzz

- `tiff_ifd` and `audit` targets gain the invariant `container.entry_count() <= entry_budget(data.len())` (always), `TooManyEntries` appears at most once, only as the last issue, and never together with `TooManyIfds`; when present, no IFD is recorded after it. Shown red by planting the check out (against the real libFuzzer target) and by a single-guard variant where two guards overlap.
- New `tiff_ifd` seeds: `entry-budget-exact` (totals equal the budget, clean), `entry-budget-over` (one over, `TooManyEntries`), `overlapping-subifds` (a reduced hostile shape that crosses the budget). The 29 existing `tiff_ifd` seeds stay byte-identical; the audit seed `audit-many-overlapping-ifds` is regenerated only if its bytes must change (it should not: the file is the same, its behaviour changes), and its test expectation is updated.

## Acceptance Criteria

- The 789,516-byte N = 511 file: `Container::parse` returns Ok with `entry_count() <= len/12`, one `TooManyEntries` issue (last), no `TooManyIfds`; IFDs recorded before the cross are intact. Peak memory of the parse is measured and reported (target: a small multiple of the input, not ~1,000x).
- Boundary exact: total entries equal to the budget parse clean; one more crosses and is omitted atomically (the IFD is absent, not truncated).
- No well-formed fixture changes: every existing test, record and seed behaves as before (reported if not). Real-photo plausibility is documented, and the guarantee is by construction, not by headroom: disjoint tables satisfy `12 x entries <= len - 8 - 6 x IFDs`, so a legitimate file never trips the budget even at about 1.0x (a table-dense layout of 65,535 entries sits at budget 65,539); sampled photos measured 1.4x-86.6x depending on the Exif blob size, and two with tiny blobs sit at 1.4x and 2.7x without risk. (Amended at ship from the original "at least 10x headroom" wording, which was not a property of the rule.)
- `TooManyEntries` is reported by `parse_strict` as the first issue's error like any other; the audit reports it as a parse issue (health not clean) with no change to the audit module.
- Gates: fmt (both graphs), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof` (run locally), `cargo test` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status`; 60 s smoke clean for `tiff_ifd` and `audit` with executions and coverage recorded. No dependencies.

## Failing Tests

Named tests (a rename or missing test is a deviation to report):

**src/tiff (unit):**
- `entry_budget_is_input_length_over_twelve` (including 0, 11, 12, 13, `usize::MAX`)
- `a_total_exactly_on_the_budget_parses_clean`
- `one_entry_over_the_budget_omits_that_ifd_and_halts`
- `the_over_budget_ifd_allocates_no_entries` (decided from the declared count: assert via a table past the budget that is also truncated, where the budget error wins only if the order in the spec holds; builder pins the actual order and reports it)
- `entries_before_the_crossing_ifd_are_kept_and_readable`
- `too_many_entries_is_the_last_issue_and_never_with_too_many_ifds`
- `the_hostile_overlapping_subifds_file_is_bounded` (N = 511 built in the test; asserts entry_count and issue shape; measures nothing slow)
- `max_issues_bound_still_holds_with_the_entry_budget`
- `parse_strict_reports_too_many_entries`
- `every_error_variant_this_module_returns_displays_something` (extended)
- `ifd0_is_counted_in_the_budget`

**tests/audit_oracle.rs and tests/jpeg_oracle.rs (or a new tests/budget_oracle.rs):**
- `container_amplification_is_bounded_and_reported` (replaces the measurement test: asserts the bound, prints the numbers)
- `live_hostile_subifds_exiftool_survives_and_we_report_the_budget` and `replay_hostile_subifds_record_header_has_the_warning` (skip loudly without the tool)
- `existing_records_and_seeds_are_byte_identical` (hash lists)

**tests/fuzz_layout.rs and the targets:**
- `tiff_harness_entry_budget_invariants_hold_on_every_seed` and the plants as table-driven tests (check removed; `<` vs `<=`; `TooManyEntries` also pushed twice; halt skipped), both directions
- `new_tiff_seeds_have_the_shapes_they_claim`

## Non-Goals

- Copying ExifTool's 10-SubIFD limit, lowering `MAX_IFDS`, or lazy/streaming entry decoding
- Per-IFD or per-tag value budgets (`MAX_TAG_VALUES` already covers values)
- Any change to the audit model, renderers or CLI; any JPEG-layer budget change
- Committing a large hostile file

## Notes for the Implementer

- Test-first: write the budget tests against the current code and watch the hostile-file test go red (it parses to 33M entries today), then implement.
- Keep the arithmetic `checked`/`saturating`; the budget for `usize::MAX` input must not overflow. The check uses the declared count (a `u16`), before allocating.
- Where the order of checks inside `follow` matters (budget vs truncation vs cycle vs depth), pin the actual order in a test and say so in the Handback; the spec's intent is: attempts, depth and cycle checks as today, then, inside `read_ifd`: the count word, table bounds (`Truncated`), and only then the declared count against the remaining budget before any entry is decoded or allocated. (Amended at ship: truncation is checked before the budget, the order the build chose and verify confirmed: budget-first would turn one corrupt count in a small file into a walk-ending halt and lose later good IFDs.)
- Real-photo plausibility: run `exiftool -a -u -G1 -s <jpeg>` on three local JPEGs and count IFD entries (the design sampled 8: 49-63 entries); report the counts and the blob sizes; commit nothing.
- Probe any new ExifTool input alone under the time limit before it joins a loop. Run the two CI-only red-proof scripts locally.
- Report in the Handback: every deviation, any existing fixture/seed/record whose behaviour changed, the before/after memory of the hostile file, fuzz executions and coverage, each red-proof before/after.
- Do not push or open a PR; do not advance cycles; do not edit `decisions/`.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — Argue a guard's headroom from the format, not from samples: I wrote "at least 10x headroom on sampled photos" as an acceptance criterion; two real photos with tiny Exif blobs sat at 1.4x and 2.7x, and the verifier showed the right statement is the disjointness inequality (`12 x entries <= len - 8 - 6 x IFDs`), safe even at about 1.0x. Also state the order of checks that touch damaged-file behaviour: I implied budget-before-truncation, which would have turned one corrupt count into a walk-ending halt; the builder chose truncation first and was right.

2. **Does any template, constraint, or decision need updating?**
   — DEC-001 gained one clarification at design (the entry budget); no change at ship. The criterion wording above was amended in the spec. No template change.

3. **Is there a follow-up spec I should write now before I forget?**
   — No. Verify's two low test gaps (a truncated IFD spends no budget; budget accounting on non-SubIFD links) are carried into SPEC-013's outline. The unrelated observation that ExifTool keeps only the first 10 SubIFD directories is a policy difference recorded in DEC-001, not a task.

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — none
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
   — Point the reader at an untrusted file, or a whole directory of them, without a 790 KB TIFF costing 950 MB: the same file now parses in 0.2 ms to a typed `TooManyEntries` outcome at 4.5 MB (11.1 MB process peak, down from 1.5 GB/826 MB depending on the measurement); real and merely damaged files read exactly as before (121 records and 66+7 seeds byte-identical).
