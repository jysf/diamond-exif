---
insight:
  id: DEC-001
  type: decision
  confidence: 0.80
  audience:
    - developer
    - agent

agent:
  id: claude-sonnet-5-5
  session_id: null

project:
  id: PROJ-001
repo:
  id: diamond-exif

created_at: 2026-11-04
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope: ["src/tiff/**", "src/error.rs", "tests/**"]

tags:
  - parser
  - error-handling
  - policy
---

# DEC-001: Tolerant walk — per-IFD issues, not whole-parse failure

## Decision

A TIFF/EXIF parse fails (`Err`) only when the header or IFD0 cannot be read; any other IFD that is structurally bad is omitted from the result and reported as a `ParseIssue` on the returned container, so "IFD0 fine, GPS IFD corrupt" is expressible.

## Context

irradiance's `ifd.rs` makes any bad IFD fatal to the whole container (its DEC-012; porting brief §0 risk 1, design §16.1). That is right for a RAW developer, which needs every IFD intact. An upload sanitiser and an audit scanner need the opposite: one corrupt GPS or Interop IFD must not hide a readable Make/Model/DateTime, and the corruption itself is a finding worth reporting. `Result<Container, Error>` cannot express a partial success.

This decision flips about 25% of the portable tests (brief §1.3 rows `2544-2700`, `3035`, `3128`). It is made before any test is ported so the port does not churn twice; the port itself is still sequenced strict first (SPEC-002), policy second (SPEC-003), so the diff stays legible.

## Alternatives Considered

- **Option A: keep strict (fatal on any bad IFD).**
  - What it is: port `ifd.rs` unchanged.
  - Why rejected: cannot meet the sanitiser/audit use case; a single bad sub-IFD discards all metadata.

- **Option B: return a partial container and swallow errors silently.**
  - What it is: skip bad IFDs with no record.
  - Why rejected: hides corruption from the audit CLI and makes tests unable to assert what was dropped.

- **Option C (chosen): partial container plus `Vec<ParseIssue>`.**
  - What it is: `Container::parse(&[u8]) -> Result<Container, Error>`; the `Ok` container exposes `issues() -> &[ParseIssue]`. A `ParseIssue` carries the location (which IFD, via which pointer tag, at which offset) and the existing typed `Error` describing the failure.
  - Why selected: parse signature stays close to the port; the error taxonomy is reused; the issue list is data the audit CLI can report and the fuzz target can check.

## Policy detail

- **Fatal (`Err`):** not a TIFF (byte-order mark or version wrong), IFD0 offset zero or out of range, IFD0 structurally unreadable (entry table or header past end of input).
- **Collected (`ParseIssue`, walk continues):** a non-IFD0 IFD (IFD1, ExifIFD, GPS, Interop, SubIFDs) that is out of range, truncated, cyclic or too deep; a pointer tag whose value is malformed. The bad IFD is omitted entirely (IFD-level atomicity), never partially included.
- **Resource guards (`MAX_IFDS`, `MAX_IFD_DEPTH`, `MAX_TAG_VALUES`):** hitting a guard in a non-IFD0 branch records an issue and stops walking that branch; already-parsed IFDs are kept.
- **Entry-value interpretation** (wrong type or count when a caller reads a value) stays an `Err` from the accessor, scoped to that field, exactly as in irradiance's mechanism ("structure is fatal to its IFD, interpretation costs the field").
- **Fuzz invariants:** `parse` never panics; every IFD present in an `Ok` container lies fully within the input; a clean container has an empty issue list.

## Clarifications from the SPEC-003 design

Refinements that follow from the decision, recorded so the code and this file agree:

- **One global attempt budget.** `MAX_IFDS` bounds IFD *visits attempted* (every non-zero pointer followed, including cyclic and unreadable ones), not just IFDs recorded. Exceeding it records one `TooManyIfds` issue and **halts the whole walk**, not only the branch: the budget is global, so every other branch would fail identically, and halting is what bounds both time and the number of issues (at most `2 * MAX_IFDS + 1`). The depth guard stays per-branch.
- **A malformed pointer tag costs its branch, not its IFD.** If the offsets of a `SubIFDs` entry cannot be read, an issue is recorded and the holding IFD is kept (IFD0 included); only its sub-branch is skipped.
- **IFD0 is atomic too.** An IFD0 whose entry table or trailing next-word is past the end of input is fatal; there is no partial IFD0.
- **Issue shape.** `ParseIssue` = typed `Error` + `Link` (`Next` or `Tag(u16)`) + index of the IFD holding the pointer + target offset (`None` when the pointer value was unreadable). `Error` gains `Clone, PartialEq, Eq`.
- **Strict wrapper.** `Container::parse_strict` ships in SPEC-003 (it is a few lines) and doubles as the differential oracle for the fuzz target and for the tests that flip.

## Consequences

- **Positive:** partial results survive corruption; the audit CLI gets corruption as findings; the policy is testable (a corrupt GPS IFD leaves IFD0 readable).
- **Negative:** callers who ignore `issues()` may treat a damaged file as healthy. Mitigation: the container exposes `is_clean()`, and a strict wrapper (any issue becomes `Err`) is added in SPEC-003 if it stays a few lines.
- **Neutral:** `ParseIssue` joins the public API surface and must stay `#[non_exhaustive]`.

## Validation

Revisit if the audit CLI or the writer needs finer granularity (per-entry salvage inside a damaged IFD), or if fuzzing shows a branch where "omit and continue" yields an inconsistent container.

## References

- Related specs: SPEC-002, SPEC-003, SPEC-004
- External: docs/porting-brief-diamond-exif.md §0, §1.3; docs/exif-core-design.md §16.1; irradiance `src/ifd.rs:976-978` (DEC-012 there)
