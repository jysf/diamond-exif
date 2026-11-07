---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-018
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
  decisions: [DEC-010, DEC-011]
  constraints: [panic-free-library, forbid-unsafe-code, no-io-in-library, fuzz-target-per-surface, test-before-implementation, oracle-must-be-shown-red, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-015, SPEC-016]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-015]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-003's <capability>". Optional; null is acceptable.
value_link: "STAGE-003: lets the binary scan a large directory holding one input file in memory at a time, with output identical to the one-shot renderers"

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
  tokens_estimate: 350000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-018: Incremental report assembly: ReportBuilder with output equal to the one-shot renderers

## Context

SPEC-015's renderers take a slice of `FileEntry`, and a `FileEntry` borrows the file's bytes (an audit's items borrow the payloads, `Item::value`). A binary that scans a directory would have to keep **every file's bytes alive until the report is rendered**: a 10,000-photo folder at 5 MB each is 50 GB. DEC-011 requires one input file in memory at a time, so the report must be assembled **incrementally**: render each file's fragment as soon as it is audited, then let the bytes go. The JSON document puts `summary` before `files` (DEC-010, pinned), so the builder also accumulates the summary as it goes and assembles the document at the end.

This is library-only, small, and has one strong correctness property that makes it easy to prove: **the builder's output equals the one-shot renderer's output, byte for byte**. It is split out of SPEC-016 so the binary's spec does not carry a refactor of the renderers, and because the equivalence is a fuzzable invariant of its own.

## Goal

`ReportBuilder` with `add(&FileEntry)` and `finish() -> String` for each of the three formats, holding after `add` only the rendered fragment and a summary accumulator (never a reference to the entry), such that for every list of entries and every options value `builder(format, opts).add(e1)..add(eN).finish() == render_<format>(&[e1..eN], opts)` and `builder.summary() == summarize(&[e1..eN], opts)`.

## Inputs

- **Reads:** `src/report/{mod,json,csv,text}.rs` (the per-file fragment writers `file_json`, `file_block` and the CSV row writer already exist; the document assembly in `render_*` and `summarize` are what change), `tests/report_render.rs`, `tests/report_formats.rs`, `tests/golden/*`, `fuzz/shared/render_invariants.rs`, `fuzz/fuzz_targets/audit_render.rs`, `tests/fuzz_layout.rs`, DEC-010.
- **Does not read:** ExifTool. Not run.
- Constraints `panic-free-library` (five-lint deny set), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (the existing `audit_render` target is extended), `test-before-implementation`, `oracle-must-be-shown-red` (our own tests: the mutants below), `no-new-top-level-deps-without-decision`. No dependencies.

## Outputs

```rust
// src/report/mod.rs (re-exported from exif_core::report)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format { Text, Json, Csv }              // name(): "text" "json" "csv"

pub struct ReportBuilder { /* private: format, options, fragments, running Summary */ }
impl ReportBuilder {
    pub fn new(format: Format, opts: RenderOptions) -> ReportBuilder;
    pub fn add(&mut self, entry: &FileEntry<'_>);   // renders the fragment and updates the summary now; keeps no borrow of `entry`
    pub fn summary(&self) -> Summary;               // of the entries added so far; equals `summarize` over them
    pub fn len(&self) -> usize;                     // entries added
    pub fn is_empty(&self) -> bool;
    pub fn finish(self) -> String;                  // assembles the document
}
pub fn render(format: Format, files: &[FileEntry<'_>], opts: &RenderOptions) -> String;   // dispatches to render_text / render_json / render_csv
```

`render_text`, `render_json`, `render_csv` and `summarize` are rewritten **on top of the builder's parts** (one code path, so they cannot drift): a one-shot render is "new, add each, finish". Their public behaviour and every golden file are unchanged. `Summary` gains an internal `add_file` step (the builder's accumulator and `summarize` use the same one); its public fields are unchanged. (Names and exact shapes are the builder's to refine inside these constraints; the rules and named tests are normative.)

### Rules (normative)

1. **Equivalence.** For every `format`, every `RenderOptions`, and every sequence of entries (including none, one, and many; Audited and Failed; with and without a policy), `finish()` equals the one-shot render byte for byte, and `summary()` equals `summarize`.
2. **No retained borrow.** After `add` returns, nothing in the builder refers to the entry or the bytes it borrowed: a test adds an entry built over a buffer that is then dropped (or overwritten) before `finish` is called. (The signature `add(&mut self, &FileEntry<'_>)` with the builder not generic over that lifetime already enforces it at compile time; a compile-fail style test is not required, the drop-before-finish test is.)
3. **Order and independence.** Fragments appear in `add` order. The fragment of a file does not depend on the files before or after it (rendering `[a, b]` is `[frag(a), frag(b)]` joined per format), and adding the same entry twice yields two fragments.
4. **Document assembly per format.** JSON: `{"schema_version":1,"policy":...,"options":...,"summary":<running summary>,"files":[` fragments joined with `,` `]}`; CSV: the header row, then the fragments (each fragment is that file's rows); text: the fragments then the summary block. An empty builder equals `render_x(&[], opts)`.
5. **Memory bound.** The builder's retained size is at most the sum of the fragment lengths plus a constant; it grows by the fragment of each added file and by no input bytes. A test adds a large synthetic entry, drops its buffer, and checks the builder's `finish` output length is the expected fragment-plus-frame length (no hidden copy of the input is observable from the API; the doc comment states the bound).
6. **No behaviour change elsewhere.** Every existing test and golden file passes unchanged; `audit()`, the preview and the DEC-010 escaping are untouched.

### Fuzz

No new target. The existing `audit_render` harness (`fuzz/shared/render_invariants.rs`) gains a guard `builder_equals_oneshot`, run alone through `run_guard`, `iff` in both directions: for the two entries the target already builds (an audited file and a failed one), the builder's output equals the one-shot output for all three formats, in `add` order, and also for the entries in the reverse order and for each entry alone (a difference in any is a violation); and `builder.summary()` equals `summarize`. Plants (both directions, at least two): a builder that drops the last fragment, one that renders a fragment with the options of a different builder, one that emits the JSON summary of the entries before the last. The step counter covers the guard (linear). `lint-fuzz-red-proof` is run locally.

## Acceptance Criteria

- `Format`, `ReportBuilder`, `render` exist as above; rules 1 to 6 hold.
- The equivalence holds on the cross product of every committed seed of all four fuzz targets and the `audit_render` seeds, the three formats, all six policies (and none), and the four `inventory`/`values` combinations, for 0, 1, 2 and 5 entries (a test over the cross product).
- Every existing golden file and test passes unchanged.
- `builder_equals_oneshot` exists with the plants; 60 s `audit_render` smoke clean (executions and coverage recorded); a plant is found by the real target.
- Every mutant below is applied by hand, shown to fail at least one named test or guard (the report names which), and reverted; tree identical to the build commit afterwards.
- Gates: fmt (both graphs and the harness files), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `fuzz-smoke audit_render` (60 s), `oracle-record` with a clean `git status`. No dependencies; `decisions/`, `scripts/` untouched.

## Failing Tests

Named tests (a rename, fold or missing test is a deviation to report):

**src/report (unit):**
- `builder_with_no_entries_equals_the_empty_one_shot_in_every_format`
- `builder_equals_the_one_shot_for_one_and_for_many_entries_in_every_format`
- `builder_fragments_appear_in_add_order_and_are_independent_of_neighbours`
- `builder_adding_the_same_entry_twice_gives_two_fragments`
- `builder_summary_equals_summarize_at_every_step`
- `builder_keeps_no_borrow_of_the_added_entry` (entry built over a buffer dropped before `finish`)
- `builder_json_assembly_puts_the_running_summary_before_the_files`
- `render_dispatches_to_the_matching_one_shot`
- `format_names_are_pinned`

**tests/report_render.rs (integration):**
- `builder_equals_the_one_shot_on_the_seed_cross_product`
- `golden_files_are_unchanged_and_the_builder_reproduces_them`

**tests/fuzz_layout.rs and the harness:**
- `builder_guard_holds_on_every_seed_and_degenerate_inputs`
- `builder_guard_goes_red_alone_when_planted` (both directions)
- `builder_guard_is_linear`

## Mutants (each shown red by hand, then reverted)

1. The builder drops the last fragment in `finish`. 2. The builder joins JSON fragments with `, ` (space). 3. The builder's JSON summary is computed before the last `add`. 4. The CSV builder emits the header per fragment. 5. The text builder puts the summary block first. 6. `add` uses default options instead of the builder's. 7. `add` appends fragments in reverse order. 8. `summary()` skips Failed entries. 9. `render_json` stops using the builder path (a divergent copy with one key swapped). 10. `Format::Csv` dispatches to `render_text`. 11. The empty JSON builder emits `"files":[,]`. 12. `len()` off by one.

## Non-Goals

- Streaming to a writer or to stdout (the builder returns a `String`); bounding the report size (question `cli-large-trees`)
- Any change to the report content, schema, goldens, escaping or statuses
- The binary (SPEC-016)

## Notes for the Implementer

- Order of work, test-first: the equivalence tests (red against a stub `ReportBuilder`), then refactor the three renderers into fragment + assembly parts, then make the one-shot functions call the builder parts, then the harness guard and plants, then the mutants.
- `render_*` and the builder must share the fragment writers and the assembly; do not write the document frame twice. Keep `summarize` as `fold(add_file)`.
- `arithmetic_side_effects`: use `saturating_add` for counters.
- Type the guard's expectations in the harness independently (it compares two library outputs, so no re-derivation is needed beyond the equality itself, but the plants must show both directions of the iff).
- Report in the Handback: every deviation, each mutant with the test or guard that caught it, the fuzz smoke numbers, and any doubt about a DEC-010 rule (a doubt is a question for the architect; do not touch `decisions/`).
- Do not push or open a PR; do not advance cycles.

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
