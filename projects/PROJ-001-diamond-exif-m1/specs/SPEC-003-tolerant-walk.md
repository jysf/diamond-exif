---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-003
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
  stage: STAGE-001
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-001, DEC-002, DEC-003, DEC-004]
  constraints: [panic-free-library, forbid-unsafe-code, fuzz-target-per-surface, test-before-implementation, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-002, SPEC-004, SPEC-006]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-002, SPEC-006]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-001's <capability>". Optional; null is acceptable.
value_link: "STAGE-001: one corrupt sub-IFD no longer discards a readable IFD0, and the corruption becomes a reportable finding"

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
  tokens_estimate: 280000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-003: Apply the tolerant-walk policy

## Context

DEC-001 as its own change. Today any structurally bad IFD makes `Container::parse` return `Err`, so one corrupt sub-IFD discards a readable IFD0. The policy flips that for every IFD except IFD0: a bad non-IFD0 IFD is omitted and reported as a `ParseIssue` on the returned container (brief §0 risk 1, design §16.1). About a quarter of the ported hostile-input tests flip from `Err` to `Ok` plus an issue.

Carry-in from SPEC-002 verify (HANDOFF-004): two guards are only weakly tested (the depth limit `+1` and the IFD-count `>=` vs `>` boundary survive mutation). This spec adds an exact-boundary test for each.

**Scope note — GPS.** SPEC-003 applies the policy to what the walker follows today: the IFD0 chain (IFD1 and later) and `SubIFDs`. Exif, GPS and Interop pointers are SPEC-004. The stage-level acceptance "a corrupt GPS IFD leaves IFD0 readable" is therefore proved here with a corrupt `SubIFD` (same code path), and SPEC-004 must repeat it for the real GPS pointer (carry-in recorded on SPEC-004).

## Goal

`Container::parse` returns a container carrying `issues()`; only a bad header or an unreadable IFD0 is `Err`; every other structural failure is an issue and the walk continues. A strict wrapper `Container::parse_strict` (any issue becomes `Err`) preserves the old behaviour for callers and for the tests that flip.

## Inputs

- **Read first:** `decisions/DEC-001-tolerant-walk-policy.md` (policy, including the clarifications section added in this design), `src/tiff/mod.rs` (the walk, `read_ifd`, `sub_ifd_offsets_of_last`, the existing 23 tests), `src/error.rs`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`.
- **Not needed:** irradiance. This is net-new policy; nothing is ported (DEC-003: no new ledger row; see Outputs).
- **Host toolchain traps:** `guidance/toolchain-brief.md` (use `~/.cargo/bin/cargo +1.90.0` for the library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`).

## Outputs

### New file

`src/tiff/issue.rs`, re-exported from `src/tiff/mod.rs` as `exif_core::tiff::{ParseIssue, Link}`:

```rust
/// How the walk reached (or tried to reach) the IFD that went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Link {
    /// The "next IFD" word at the end of an IFD (the IFD0 chain: IFD1, ...).
    Next,
    /// A pointer tag; today only `SubIFDs` (330). SPEC-004 adds Exif/GPS/Interop.
    Tag(u16),
}

/// One recoverable problem found while walking (DEC-001).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseIssue { /* private fields */ }

impl ParseIssue {
    pub(crate) fn new(error: Error, link: Link, from_ifd: usize, offset: Option<u32>) -> ParseIssue;
    /// What went wrong: the existing typed error.
    pub fn error(&self) -> &Error;
    /// Which kind of pointer was being followed.
    pub fn link(&self) -> Link;
    /// Index into `Container::ifds()` of the IFD that held the pointer.
    /// Always a recorded IFD, so `ifds().get(from_ifd)` is `Some`.
    pub fn from_ifd(&self) -> usize;
    /// Offset the pointer led to, or `None` when the pointer value itself
    /// could not be read (a malformed pointer tag).
    pub fn offset(&self) -> Option<u32>;
}
```

`Display` for both: `Link` prints `next-IFD pointer` / `tag 0x014a` (lower-case hex, 4 digits); `ParseIssue` prints `{error} (following {link} from IFD #{from_ifd}` then `, to offset {offset}` only when `Some`, then `)`.

### Files modified

- `src/error.rs` — add `Clone, PartialEq, Eq` to `Error`'s derives (every field is plain data; needed because `Container` is `Clone` and tests compare errors). No new variants.
- `src/tiff/mod.rs`:
  - `Container` gains a private `issues: Vec<ParseIssue>`.
  - New public API: `pub fn issues(&self) -> &[ParseIssue]`, `pub fn is_clean(&self) -> bool` (no issues), `pub fn parse_strict(data: &'a [u8]) -> Result<Container<'a>, Error>` (calls `parse`; if any issue exists, returns the **first** issue's error by value, else the container).
  - `Container::parse` and the walk rewritten per **Walk rules** below; module docs and the doc on `sub_ifd_offsets_of_last` rewritten (they currently say "strict").
  - Existing tests flipped and new tests added per **Failing Tests**.
- `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs` — fuzz invariants and three new seeds (below).
- `docs/provenance-ledger.md` — edit the `src/tiff/mod.rs` row's Notes: "walk reworked for DEC-001 (SPEC-003); policy is net-new, not ported". No row for `issue.rs` (net-new, nothing ported).
- `CHANGELOG.md` — `[Unreleased] → Changed` (strict walk becomes tolerant) and `Added` (`ParseIssue`, `Link`, `issues`, `is_clean`, `parse_strict`).
- `README.md` — the status line still says "no parsing code yet"; bring it up to date (strict TIFF reader, tolerant walk, fuzz target; still no CLI).

### Walk rules (normative)

IFD0 is the only fatal IFD. Everything else is an issue.

1. **Header**: unchanged. Any failure is `Err`. IFD0 offset `0` is `Ok` with no IFDs and no issues (unchanged).
2. **IFD0**: read with `read_ifd`; any error propagates as `Err` (entry table or the trailing next-word past EOF, offset out of range). IFD-level atomicity applies to IFD0 too: no partial IFD0. On success IFD0 is recorded and counts as attempt 1.
3. **Everything after IFD0** goes through one `follow` routine that is iterative along a chain and recursive only into `SubIFDs` (recursion depth stays bounded by `MAX_IFD_DEPTH`). Walk state, private to `parse`: `visited: Vec<u32>`, `attempts: usize`, `halted: bool`. For each non-zero offset it is about to read, in this order:
   1. `attempts += 1`; if `attempts > MAX_IFDS`: record `TooManyIfds { limit: MAX_IFDS }` (offset `Some(that offset)`), set `halted`, stop. **`halted` ends the whole walk**, not just the branch: the budget is global, so every other branch would fail the same way, and stopping bounds both time and the number of issues (DEC-001 clarification).
   2. `depth > MAX_IFD_DEPTH`: record `IfdDepthExceeded`, stop **this branch** (siblings and the parent's chain continue).
   3. offset already in `visited`: record `CyclicIfd { offset }`, stop this chain; else push it.
   4. `read_ifd` fails: record that error, stop this chain. The IFD is **omitted entirely** (never partially included) and, because its next-word is unreadable, its chain ends there.
   5. Success: record the IFD, then walk its `SubIFDs` (depth + 1, parent = this IFD), then continue with its next-word at the same depth and the same parent (depth-first order, as today).
4. **`SubIFDs` pointer tag**: if reading its offsets fails (wrong field type, count over `MAX_TAG_VALUES`, payload past EOF), record the error with `Link::Tag(330)`, `offset: None`, `from_ifd` = the IFD holding the tag, and skip its sub-branch. **The holding IFD is kept** (IFD0 included) and its chain continues. A sub-offset of `0` is skipped silently (as today); every non-zero offset, including duplicates, consumes an attempt.
5. **Link and origin for each issue**: failures reached through the next-word use `Link::Next`; through `SubIFDs` use `Link::Tag(330)`; `from_ifd` is the index of the IFD that held the pointer; `offset` is the offset that was being followed (`None` only for rule 4's unreadable pointer tag).
6. **Bound on issues**: at most one issue per attempt plus one per recorded IFD plus one for `TooManyIfds`, so `issues().len() <= 2 * MAX_IFDS + 1`. No separate issue cap is needed because every pointer follow consumes the attempt budget. Order is walk order.
7. **Unchanged**: `MAX_IFD_DEPTH` (8), `MAX_IFDS` (512), `MAX_TAG_VALUES`, every accessor (`payload`, `uints`, ... still return `Err` per field, DEC-001 "entry-value interpretation").

### Fuzz changes (same change, per SPEC-006's rule)

Extend `tiff_ifd` so that on `Ok(container)` it also checks (the target may `assert!`; a violated invariant is a finding):

- **differential**: `Container::parse_strict(data)` is `Ok` iff `container.is_clean()`; if `Err`, it equals `*container.issues()[0].error()`-by-value (`PartialEq`). And when `parse` is `Err`, `parse_strict` returns an equal `Err`.
- **containment**: every IFD lies fully inside the input: `offset + 2 + 12 * entries().len() + 4 <= data.len()` (compute in `u64`).
- **bounds**: `ifds().len() <= MAX_IFDS`, `issues().len() <= 2 * MAX_IFDS + 1`, and each `issue.from_ifd() < ifds().len()`.
- Exercise `ParseIssue` getters and `Display` (must not panic).

`tests/fuzz_layout.rs`: its `exercise` function mirrors the target's contract — update it to the same checks, and add the three new seed names to `SEEDS`. `examples/fuzz_seeds.rs` gains three deterministic seeds (rewrite `fuzz/seeds/tiff_ifd/`; seeds stay byte-identical across runs): `corrupt-subifd` (IFD0 valid, SubIFD offset past EOF), `malformed-subifds-tag` (SubIFDs tag with a non-LONG type, IFD0 chain to a valid IFD1), `depth-limit-exact` (SubIFD nesting to depth 8, deepest IFD terminal).

## Acceptance Criteria

- [ ] `Container::parse` is `Err` only for a bad header or an unreadable IFD0 (including its trailing next-word); every other structural failure yields `Ok` plus a `ParseIssue` with the right `error`, `link`, `from_ifd`, `offset`.
- [ ] A corrupt `SubIFD` leaves IFD0 and its other readable IFDs present and readable; the corrupt IFD is absent from `ifds()` (atomic omission).
- [ ] `parse_strict` reproduces the pre-SPEC-003 `Err` variants for every flipped test (asserted in the tests, not by inspection).
- [ ] Exact-boundary tests pass: depth 8 clean / depth 9 issue; 512 IFDs clean / 513th issue. Each is shown red (below).
- [ ] A hostile `SubIFDs` array cannot produce unbounded issues or time (test below); `issues().len() <= 2 * MAX_IFDS + 1`.
- [ ] `tiff_ifd` fuzz target extended; `just fuzz-smoke` (60 s) clean, executions and coverage recorded in the handback; **shown red**: plant a bug that violates the differential invariant (e.g. make `is_clean` return `true` unconditionally) and show the seed test and/or libFuzzer catches it; remove the plant, tree clean.
- [ ] **Mutation checks recorded** in the handback: for each of (a) `depth > MAX_IFD_DEPTH` -> `>=`, (b) `> MAX_IFD_DEPTH` -> `> MAX_IFD_DEPTH + 1`, (c) `attempts > MAX_IFDS` -> `>=`, (d) `>` -> `> MAX_IFDS + 1`, the named test goes red; then revert.
- [ ] Library gates green and re-run independently by the orchestrator: `cargo fmt --check`, clippy `-D warnings` with the five-lint set (`just lint`), `cargo test` on stable and on 1.90.0, `cargo rustc --lib -- -F unsafe_code`, `just lint-fuzz`, `rustfmt --check` on the fuzz crate, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`.
- [ ] `[dependencies]` still empty; no panic-capable construct added to non-test code (the five lints enforce it); no irradiance file touched.

## Failing Tests

All in `src/tiff/mod.rs` `mod tests`, using the existing `Build` byte-builder. "strict" means `Container::parse_strict` returning the stated `Err`. Write each flipped test so it asserts the new tolerant outcome **and** the old strict `Err`.

**Unchanged outcomes (still `Err` from `parse`)**
- `an_ifd_offset_past_eof_is_truncated`, `an_entry_count_past_eof_is_truncated` — IFD0 cases, asserts as today.
- NEW `ifd0_with_its_next_word_cut_off_is_still_fatal` — IFD0 entry table complete but the file ends before the 4-byte next-word: `Err(Truncated)`.
- `an_ifd0_offset_of_zero_yields_no_ifds` — add `is_clean()` and `issues().is_empty()`.

**Flipped (was `Err`, now `Ok` + one issue)**
- `a_self_referential_subifd_terminates` — IFD0 (offset 8) has `SubIFDs -> 8`. `ifds().len()==1`; one issue: `Cyclic{8}`, `Link::Tag(330)`, `from_ifd 0`, `offset Some(8)`; `!is_clean()`; strict `Err(CyclicIfd{8})`.
- `a_two_hop_subifd_cycle_terminates` — 8 -> 200 -> 8. `ifds().len()==2`; issue `Cyclic{8}`, `Tag(330)`, `from_ifd 1`, `Some(8)`; strict as before.
- `a_chain_that_points_at_itself_terminates` — IFD0.next = 8. `ifds().len()==1`; issue `Cyclic{8}`, `Link::Next`, `from_ifd 0`, `Some(8)`.
- `nesting_past_the_depth_limit_terminates` — 10 IFDs at depths 0..=9, each at a distinct offset, **the depth-9 IFD carrying no `SubIFDs`** so only the limit can stop it. `ifds().len()==9`; issue `IfdDepthExceeded{limit: 8}`, `Tag(330)`, `from_ifd 8`, `offset Some(8 + 9*40)`; strict `Err(IfdDepthExceeded)`.
- `a_long_chain_stops_at_the_ifd_limit` — chain of 513 empty IFDs. `ifds().len()==512`; one issue `TooManyIfds{512}`, `Link::Next`, `from_ifd 511`, `offset Some(the 513th IFD's offset)`; strict `Err(TooManyIfds)`.

**Boundary tests (carry-in; kill the surviving mutants)**
- NEW `nesting_at_exactly_the_depth_limit_is_clean` — 9 IFDs, depths 0..=8, deepest terminal: `is_clean()`, `ifds().len()==9`, `ifds().last().depth()==8`.
- NEW `a_chain_of_exactly_max_ifds_is_clean` — 512 chained IFDs: `is_clean()`, `ifds().len()==512`.

**Tolerant behaviour (new)**
- `a_corrupt_subifd_leaves_ifd0_readable` — IFD0 = baseline entries + `SubIFDs -> 0xFFFF_FF00`. `ifds().len()==1`; `required_scalar(ifd0, IMAGE_WIDTH)==4`; issue `Truncated`, `Tag(330)`, `from_ifd 0`, `Some(0xFFFF_FF00)`; strict `Err(Truncated)`.
- `one_bad_subifd_does_not_hide_its_siblings` — `SubIFDs` count 3 -> [good @200, past-EOF, good @400]. `ifds()` offsets `[8, 200, 400]`, both parents `Some(0)`; exactly one issue.
- `a_corrupt_ifd1_keeps_ifd0` — IFD0.next -> IFD whose entry count runs past EOF. `ifds().len()==1`; one issue `Truncated`, `Link::Next`, `from_ifd 0`.
- `a_bad_subifd_is_omitted_atomically` — a sub-IFD whose entries are complete but whose next-word is cut off by EOF: `ifds().len()==1` (no partial IFD), one `Truncated` issue.
- `a_malformed_subifds_tag_costs_the_branch_not_the_ifd` — IFD0 has `SubIFDs` as type 10 (SRATIONAL), count 1, and a valid IFD1 via next. `ifds().len()==2` (IFD0 kept, chain followed); one issue `UnexpectedFieldType{tag:330, field_type:10}`, `Tag(330)`, `from_ifd 0`, `offset None`.
- `a_subifds_payload_past_eof_is_a_pointer_issue` — `SubIFDs` LONG count 4, offset past EOF: IFD0 kept, one `Truncated` issue with `offset None`.
- `issues_arrive_in_walk_order` — a bad `SubIFD` and a bad IFD1 in one file: `issues()[0].link()==Tag(330)`, `issues()[1].link()==Link::Next`.
- `a_hostile_subifds_array_cannot_flood_the_issue_list` — `SubIFDs` with 100,000 offsets all equal to 8: exactly 512 issues (511 `CyclicIfd{8}` then one `TooManyIfds{512}`, the last), `issues().len() <= 2*MAX_IFDS+1`. Derivation: IFD0 is attempt 1; sub-offsets 1..=511 are attempts 2..=512 (cyclic, one issue each); sub-offset 512 is attempt 513, over budget. If your count differs, stop and report it in the handback rather than adjusting the number.
- `a_clean_container_reports_no_issues` — extend the existing SubIFD-order test: `is_clean()`, `issues().is_empty()`.
- `parse_strict_agrees_with_parse_on_fatal_input` — header and IFD0 failures give equal `Err` from both.
- `parse_issue_display_names_error_and_location` — `Display` is non-empty, contains the inner error's text and `IFD #0`; `Link::Tag(330)` prints `tag 0x014a`.

**Fuzz layout** (`tests/fuzz_layout.rs`): `seeds_are_committed_and_cover_the_hostile_shapes` lists the three new seeds; `every_seed_parses_or_errors_without_panicking` runs the extended `exercise`.

## Non-Goals

- Following Exif, GPS or Interop pointers (SPEC-004), and the real GPS-corrupt test (carried to SPEC-004)
- Per-entry salvage inside a damaged IFD (DEC-001 "Validation": revisit only if the CLI or writer needs it)
- Any change to accessors' per-field `Err` behaviour, to limits, or to field decoders
- CI wiring, the 24 h fuzz run, structure-aware fuzzing

## Notes for the Implementer

- Keep the five lints satisfied in non-test code: `get`, `checked_*`, `saturating_add`, no indexing, no `unwrap`. `attempts += 1` must be `saturating_add` or `checked_add`.
- The count of attempts includes IFD0 (attempt 1). Do not restructure so that IFD0 is read through `follow`; its failure must stay fatal.
- The recursion is only into `SubIFDs`; keep the chain loop iterative.
- `parse_strict` should be a few lines. Do not clone `Error` unnecessarily: take the first issue by value (`into_iter().next()`), since `Error` is now `Clone` but the container is being consumed anyway.
- Update the fuzz target in the same change; run the 60 s smoke with the PATH-first-`~/.cargo/bin` and `+nightly` invocation in the target's doc comment.
- Do not push or open a PR.

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
