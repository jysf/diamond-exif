---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-004
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: M             # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: punch-list  # approved | punch-list | rejected — the OUTCOME of the verify
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
  decisions: [DEC-001, DEC-002, DEC-003, DEC-004, DEC-005]
  constraints: [panic-free-library, forbid-unsafe-code, fuzz-target-per-surface, test-before-implementation, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-003, SPEC-006, SPEC-007]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-003]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-001's <capability>". Optional; null is acceptable.
value_link: "STAGE-001: Exif, GPS and Interop IFDs become reachable, labelled and individually survivable, with bounded work"

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
      tokens_total: 185074
      estimated_usd: 1.22
      duration_minutes: 10
      recorded_at: 2026-11-05
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
    - cycle: verify
      agent: claude-opus-5-5
      interface: other
      tokens_total: 181124
      estimated_usd: 1.20
      duration_minutes: 9
      recorded_at: 2026-11-05
      notes: "verdict PUNCH LIST at a68757d (code 06f1702); tokens transcribed by the orchestrator from the sub-agent metered result"
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 10855
      estimated_usd: 0.07
      duration_minutes: 3
      recorded_at: 2026-11-05
      notes: "punch-list fix round (resumed builder). The harness reported a cumulative 195,929 for the resumed agent; this is the 10,855 above the 185,074 already recorded for the first round."
  totals:
    tokens_total: 377053
    estimated_usd: 2.49
    session_count: 3
shipped_at: 2026-11-05
---

# SPEC-004: Multi-IFD pointer walk

## Context

The walker follows only `SubIFDs` today. EXIF data lives in IFD0, the Exif IFD (0x8769), the GPS IFD (0x8825) and the Interop IFD (0xA005), none of which are reached yet (brief §0 risk 1). This spec adds them under the tolerant policy and labels every IFD with how it was reached. Decoders (ASCII, signed, float) are split out as SPEC-007, because they touch `types.rs` and not the walk; this keeps each spec independently verifiable. Policy for which pointer is followed from where is **DEC-005** (read it first; it is normative).

Carry-ins, all in this spec:
- SPEC-003 design: prove with the **real** GPS pointer that "a corrupt GPS IFD leaves IFD0 (and the Exif IFD) readable".
- SPEC-003 verify (low finding): bound the work per pointer tag. DEC-005 does this by rejecting a `SubIFDs` entry with more than `MAX_IFDS` offsets before decoding.
- SPEC-003 build: update the `tiff_ifd` fuzz target in the same change as any new walk behaviour.

## Goal

`Container::parse` follows ExifIFD, GPS, Interop and SubIFDs per DEC-005; every `Ifd` reports its `kind()`; a bad GPS (or Exif, or Interop) pointer or IFD is an issue and never hides the rest; work and issue counts stay bounded by constants.

## Inputs

- **Read first:** `decisions/DEC-005-pointer-tag-walk-policy.md`, `decisions/DEC-001-tolerant-walk-policy.md` (with clarifications), `src/tiff/mod.rs` (`Walk`, `walk_from`, `walk_sub_ifds`, `follow`, tests and the `Build` byte-builder), `src/tiff/issue.rs`, `src/tiff/tags.rs`, `src/error.rs`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`.
- **Handoff-004 and 008 context:** the SPEC-003 verify report (`HANDOFF-008`, finding 1).
- **Host traps:** `guidance/toolchain-brief.md` (`~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`).
- No irradiance source: this is net-new code (DEC-003: no new ledger row; note the walk change on the existing `src/tiff/mod.rs` row).

## Outputs

### Public API (all `#[non_exhaustive]` enums)

```rust
/// How an IFD was reached (DEC-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IfdKind { Ifd0, Ifd1, Chain, Sub, Exif, Gps, Interop }

impl Ifd { pub fn kind(&self) -> IfdKind; }
impl Container<'_> {
    /// First IFD of this kind in walk order, if any.
    pub fn ifd_of_kind(&self, kind: IfdKind) -> Option<&Ifd>;
}
pub const MAX_ISSUES: usize = 5 * MAX_IFDS + 1;   // in tiff/mod.rs, documented as the guaranteed bound
```

`tags.rs` adds `EXIF_IFD = 0x8769`, `GPS_INFO = 0x8825`, `INTEROP_IFD = 0xA005` with spec-citing docs (Exif 2.32). `error.rs` adds one variant (additive; `#[non_exhaustive]` already):

```rust
/// A pointer tag must carry exactly one offset.
BadPointerCount { tag: u16, count: u32 }   // Display: "pointer tag {tag:#06x} carries {count} value(s), expected exactly 1"
```

### Files modified

- `src/tiff/mod.rs`: `Ifd` gains a private `kind`; the walk generalised from "SubIFDs only" to the DEC-005 table (a small private table of `(tag, allowed holder kinds, child kind)` consulted by one routine; no copy-pasted per-tag walkers); `Container::ifd_of_kind`; `MAX_ISSUES`; module docs rewritten (they still describe a SubIFD-only walk); tests flipped and added per **Failing Tests**.
- `src/tiff/tags.rs`, `src/error.rs` as above. `src/tiff/issue.rs`: no change expected (`Link::Tag(u16)` already covers new tags); its doc comment on `Link::Tag` that says "today only `SubIFDs`" must be updated.
- `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs` (same `exercise`), `examples/fuzz_seeds.rs`: new invariants and 5 new seeds (below). Regenerate `fuzz/seeds/tiff_ifd/` (existing 20 seeds must stay byte-identical).
- `docs/provenance-ledger.md` (note on the `src/tiff/mod.rs` row: "pointer-tag walk net-new, SPEC-004/DEC-005"), `CHANGELOG.md` (`Added`: `IfdKind`, `kind()`, `ifd_of_kind`, `MAX_ISSUES`, the three pointer tags, `BadPointerCount`; `Changed`: the walk follows Exif/GPS/Interop, and a `SubIFDs` entry over 512 offsets is now rejected), `README.md` status line.

### Walk rules (normative; DEC-005 has the table)

1. **Kinds.** IFD0 is `Ifd0`. Its next-word target is `Ifd1`; the next after that, and later, `Chain`. IFDs reached via `SubIFDs` are `Sub` (and their own chain stays `Sub`); via 0x8769 `Exif`; via 0x8825 `Gps`; via 0xA005 `Interop`.
2. **Order.** For each recorded IFD, depth-first: its `SubIFDs`, then 0x8769, then 0x8825 (Interop is reached from inside the Exif IFD when that IFD is recorded), then continue along its chain. Order of `ifds()` and of `issues()` is walk order.
3. **Permitted holders** (DEC-005): 0x8769 and 0x8825 from `Ifd0`/`Ifd1`/`Chain`/`Sub` only; 0xA005 from `Exif` only; `SubIFDs` from anywhere. A pointer tag elsewhere is ignored without an issue.
4. **Leaf kinds.** The next-word of `Exif`/`Gps`/`Interop` IFDs is stored in `Ifd::next()` but never followed.
5. **Value rule.** 0x8769/0x8825/0xA005 must be type `LONG` (4) or `IFD` (13) with count 1. Other type: `UnexpectedFieldType { tag, field_type }`; count other than 1 (including 0): `BadPointerCount`; unreadable payload: the underlying error (e.g. `Truncated`). Each is one issue with `Link::Tag(tag)`, `from_ifd` = holder, `offset: None`; the holder is kept and its other pointers and chain still walk. A pointer value of 0 is skipped silently (as for `SubIFDs`).
6. **`SubIFDs` cap.** If its declared `count` exceeds `MAX_IFDS`, record `TagTooLarge { tag: 330, count, limit: MAX_IFDS }` with `offset: None` and skip the branch **without decoding any offset**. A count of exactly `MAX_IFDS` is accepted.
7. **Depth and budget** unchanged in meaning: every non-zero followed offset consumes an attempt (IFD0 is attempt 1), the budget halts the whole walk, the depth limit stops the branch. Exif is depth+1 of its holder, Interop depth+1 of Exif.
8. **Duplicates.** If an IFD has a pointer tag twice, only the first entry is used. Two pointers to one offset (e.g. Exif and GPS both pointing at the same IFD) yield `CyclicIfd` for the second; that is existing, accepted behaviour.
9. **Issue bound.** `issues().len() <= MAX_ISSUES`, by construction (DEC-005).
10. Accessors, limits and everything else unchanged.

### Fuzz changes (same change)

On `Ok(container)`, additionally assert: the first IFD has kind `Ifd0` and the rest do not; every `parent()` index is `<` the IFD's own index; `Exif`/`Gps`/`Interop` IFDs have `Some` parent, an `Interop`'s parent is an `Exif`, an `Exif`/`Gps`'s parent is not `Exif`/`Gps`/`Interop`; child depth is `parent.depth + 1` for `Sub`/`Exif`/`Gps`/`Interop`; `issues().len() <= MAX_ISSUES`; `ifd_of_kind(k)` agrees with a linear scan; the existing SPEC-003 invariants stay. Five new seeds in `examples/fuzz_seeds.rs`: `exif-gps-interop` (a valid file with all three), `gps-past-eof-exif-ok`, `exif-points-at-ifd0`, `pointer-bad-count`, `subifds-count-over-cap`.

## Acceptance Criteria

- [ ] Exif, GPS and Interop IFDs are reached, labelled, and ordered per the rules; `kind()` and `ifd_of_kind` correct.
- [ ] A corrupt GPS pointer or GPS IFD leaves IFD0 **and** the Exif IFD readable, with one issue naming `Link::Tag(0x8825)`; `parse_strict` still reports it as `Err` (first issue's error).
- [ ] Every pointer-value failure mode of rule 5 is an issue, never a fatal error and never a panic; the holder is kept.
- [ ] A `SubIFDs` count above `MAX_IFDS` is rejected before decoding (shown by a test that would otherwise allocate/iterate 100,000 offsets; assert no `CyclicIfd` flood and exactly one issue). Count exactly `MAX_IFDS` accepted.
- [ ] `issues().len() <= MAX_ISSUES` including on the worst-case flood tests; the SPEC-003 boundary tests (depth 8/9, 512/513 IFDs) still pass unchanged in meaning.
- [ ] **Shown red** for the new walk behaviour: record that each of these plants makes a named test go red, then revert: (a) allow `0xA005` from any holder, (b) follow the next-word of `Exif` IFDs, (c) `SubIFDs` cap `>` -> `>=`, (d) drop the GPS arm (never follow 0x8825), (e) mislabel `Ifd1` as `Chain`.
- [ ] `tiff_ifd` fuzz target extended, three new invariants shown red with a planted bug (e.g. wrong `kind` for `Gps`; `parent` index pointing forward), `just fuzz-smoke` 60 s clean with executions and coverage recorded.
- [ ] Library gates green and independently re-run by the orchestrator: `cargo fmt --check`, `just lint`, `just lint-fuzz`, `rustfmt --check` on the fuzz crate, `cargo test` on stable and 1.90.0, `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`.
- [ ] `[dependencies]` still empty; five lints clean in non-test code; irradiance and `scripts/` untouched.

## Failing Tests

In `src/tiff/mod.rs` `mod tests` with the existing `Build` byte-builder (extend it with small helpers if needed). Local test constants for `EXIF_IFD`/`GPS_INFO`/`INTEROP_IFD` are fine if the tags module is not yet exported, but prefer the new `tags::` consts. "Baseline" = the existing `baseline(&b)` entries.

**Happy path**
- `exif_gps_and_interop_are_reached_and_labelled` — IFD0 (baseline + 0x8769 -> E, 0x8825 -> G), E (+ 0xA005 -> I), both byte orders. Depth-first per rule 2, so `ifds()` order is `[IFD0, E, I, G]` (E's Interop is walked inside E, before IFD0's GPS). Kinds `[Ifd0, Exif, Interop, Gps]`; parents `[None, Some(0), Some(1), Some(0)]`; depths `[0, 1, 2, 1]`; `is_clean()`; `ifd_of_kind(Gps)` is the fourth recorded IFD; `ifd_of_kind(Ifd1)` is `None`.
- `an_ifd0_chain_is_labelled_ifd1_then_chain` — three chained IFDs: kinds `[Ifd0, Ifd1, Chain]`; a chain inside a `SubIFDs` branch stays `Sub`.
- `a_chain_pointer_inside_a_leaf_ifd_is_not_followed` — the Exif IFD's next-word points at a valid IFD: it is not in `ifds()`, `is_clean()`, but `exif.next()` still returns the offset.
- `pointer_tags_in_unexpected_places_are_ignored_silently` — 0xA005 in IFD0; 0x8769 inside the Exif IFD; 0x8825 inside the GPS IFD: none followed, `is_clean()`, IFD count unchanged.
- `duplicate_pointer_tags_use_the_first_entry` — two 0x8769 entries pointing at E1 then E2: only E1 recorded; clean.

**Tolerance (the carried-in GPS proof)**
- `a_corrupt_gps_ifd_leaves_ifd0_and_exif_readable` — GPS pointer to an IFD whose entry count runs past EOF. IFD0 and Exif present; `required_scalar(ifd0, IMAGE_WIDTH)==4`; no GPS IFD; one issue `Truncated`, `Link::Tag(0x8825)`, `from_ifd 0`, `offset Some(g)`; `ifd_of_kind(Gps).is_none()`; `parse_strict` is `Err(Truncated)`.
- `a_gps_pointer_past_eof_with_a_readable_exif_ifd` — pointer value `0xFFFF_FF00`: same shape (issue offset `Some(0xFFFF_FF00)`, error `Truncated`), Exif IFD readable.
- `a_pointer_with_the_wrong_type_costs_the_branch_not_the_ifd` — 0x8825 as `SHORT` count 1 and, separately, as `RATIONAL`: `UnexpectedFieldType{tag:0x8825, ..}`, `offset None`, IFD0 and Exif kept, chain to IFD1 still followed.
- `a_pointer_with_the_wrong_count_is_typed` — 0x8769 `LONG` count 2 and count 0: `BadPointerCount{tag:0x8769, count}`, `offset None`, holder kept. (Count 0 inline must not read garbage.)
- `a_zero_pointer_value_is_skipped_silently` — 0x8825 `LONG 1` value 0: clean, no GPS.
- `exif_pointing_back_at_ifd0_is_cyclic` — 0x8769 -> 8: `CyclicIfd{8}`, `Link::Tag(0x8769)`, `from_ifd 0`, `offset Some(8)`; IFD0 kept; strict as before.
- `exif_and_gps_pointing_at_the_same_ifd_report_the_second` — second is `CyclicIfd`, first kept (the accepted "diamond" behaviour).
- `a_bad_interop_pointer_keeps_the_exif_ifd` — Interop past EOF inside E: E kept, one issue `Link::Tag(0xA005)`, `from_ifd` = E's index.
- `issues_arrive_in_walk_order_across_pointer_kinds` — bad SubIFD, bad Exif, bad GPS in one IFD0: `Link::Tag` of issues `[330, 0x8769, 0x8825]`.

**Bounds and work (carry-in)**
- `a_subifds_count_above_the_cap_is_rejected_before_decoding` — `SubIFDs` `LONG` with count 100,000 whose payload is **absent** (offset past EOF) so decoding it would yield `Truncated`; the expected single issue is `TagTooLarge{tag:330, count:100000, limit:512}`, `offset None`. This proves the cap is checked before the payload is touched.
- `a_subifds_count_at_the_cap_is_accepted` — count 512 of offsets all `8`: 511 `CyclicIfd{8}` then one `TooManyIfds{512}` (512 issues total; derivation: IFD0 attempt 1, offsets 1..=511 attempts 2..=512, offset 512 is attempt 513). If your count differs, stop and report it in the handback.
- `the_issue_list_never_exceeds_max_issues` — every IFD of a 512-chain carries all four pointer tags, each malformed: `issues().len() <= MAX_ISSUES`.
- Update `a_hostile_subifds_array_cannot_flood_the_issue_list` (from SPEC-003): its 100,000-offset array now yields the single `TagTooLarge` issue; keep its intent.
- Keep `a_chain_of_exactly_max_ifds_is_clean`, depth 8/9 and all SPEC-003 tests; flip nothing else without reason.

**Errors and Display**
- `every_error_variant_this_module_returns_displays_something` — add `BadPointerCount` (index 11); the exhaustive-match guard forces it.
- `parse_issue_display_for_new_pointer_tags` — `Link::Tag(0x8825)` prints `tag 0x8825`.

**Fuzz layout** (`tests/fuzz_layout.rs`): `SEEDS` gains the five new names; `exercise` mirrors the extended invariants.

## Non-Goals

- ASCII, signed, float and other decoders (SPEC-007); `parse_fields` (SPEC-005)
- MakerNote, thumbnails, `UserComment`, any tag-value interpretation
- Oracle comparison (SPEC-005); writer; JPEG/PNG containers; CI wiring; 24 h fuzz run

## Notes for the Implementer

- Keep the five lints satisfied in non-test code (`get`, `checked_*`/`saturating_*`, no indexing, no `unwrap`). Pointer tag values go through `uints`-style bounds discipline, but check the **type** and **count** before touching the payload where the spec says so (rule 6 especially).
- Do not copy-paste the SubIFDs walker per tag; one table-driven routine.
- The per-tag issue is recorded with the pointer's holder as `from_ifd`; `offset` is `None` for any problem with the pointer value itself and `Some(off)` for a problem with the IFD it led to.
- The recursion depth bound is still `MAX_IFD_DEPTH`; keep chains iterative.
- Update the fuzz target and seeds in the same change; run the 60 s smoke with the PATH-first `~/.cargo/bin` and `+nightly` invocation in the target's doc comment.
- Do not push or open a PR; do not advance cycles; do not modify irradiance or `scripts/`.

---

## Reflection

1. **What would I do differently next time?**
   — State the "halted ends the walk" rule as an explicit test in the spec instead of only as a rule: the builder satisfied the letter of rule 7 for the offset loop but kept recording pointer-value issues after the halt, and only the verifier's adversarial probe found it. Naming a "nothing is recorded after TooManyIfds" test at design would have caught it in build. Also name the exact worst-case shape for any "bound" claim; the verifier found the real ceiling (3 x MAX_IFDS + 1 = 1,537) is well under `MAX_ISSUES` (2,561).

2. **Does any template, constraint, or decision need updating?**
   — DEC-005 gained a note that `Exif -> Sub -> Exif -> Interop` nesting is reachable and accepted. `scripts/decisions-audit.sh` reads only block-style `affected_scope` lists, so `--changed` silently matched nothing; worked around by converting every DEC to block lists and recorded as signal `decisions-audit-reads-block-lists-only` (upstream fix needed in the template and irradiance).

3. **Is there a follow-up spec I should write now before I forget?**
   — SPEC-007 (decoders) is next and already outlined. Possible later tightening, not a spec: `MAX_ISSUES` is loose; `ALL_KINDS` is duplicated in `tests/fuzz_layout.rs` and cannot fail to compile when an `IfdKind` is added (a `#[non_exhaustive]` enum), so a new kind must be added by hand in the fuzz target and layout test.

4. **Where was the worst defect caught?** — verify
   *(issues recorded after the budget halted the walk; fixed in a follow-up build round and mutation-checked again by the orchestrator. Verify verdict is recorded as punch-list; the fix round was reviewed by the orchestrator rather than a second full verify.)*

5. **What can a user do now that they couldn't before?** — Read a file's Exif, GPS and Interop IFDs separately, each labelled by how it was reached, and still get IFD0 and the Exif IFD when the GPS block is corrupt, with the damage reported as a typed finding; and a hostile pointer array can no longer burn work without spending the IFD budget.
