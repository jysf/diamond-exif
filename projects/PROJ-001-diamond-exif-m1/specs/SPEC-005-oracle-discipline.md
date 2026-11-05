---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-005
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
  decisions: [DEC-003, DEC-002]
  constraints: [oracle-must-be-shown-red, exiftool-never-linked, no-new-top-level-deps-without-decision, test-before-implementation, panic-free-library]
  related_specs: [SPEC-004, SPEC-007, SPEC-008]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-004, SPEC-007]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-001's <capability>". Optional; null is acceptable.
value_link: "STAGE-001: first independent evidence (ExifTool) that what exif-core reads is right, with the comparator itself shown able to fail"

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
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-005: Oracle discipline — ExifTool read parity

## Context

Quality bar: **oracle-must-be-shown-red** (design §8; constraint `oracle-must-be-shown-red`). Until now every claim about what the parser reads rests on tests we wrote ourselves. This spec adds the independent check: run ExifTool (never linked, never read; `exiftool-never-linked`) over the same bytes and compare, field by field, with what `exif-core` reads. A comparator that has never been seen to fail proves nothing, so the comparator is red-proofed before any parity claim is made.

Scope is the **TIFF/EXIF container only** (M1a). There is no JPEG/PNG/WebP yet; ExifTool reads a bare TIFF fine. Fixtures are **tier A**: hand-built, byte by byte, from the TIFF 6.0 / Exif 2.32 layout, so the licence question for M1a fixtures is closed (no real camera files are committed; `rust-ci-and-fixtures` keeps the CI-ownership half). The byte-mutation helpers and the mutated-copy red-proof (inject a fault into a copy of the library, show the oracle catches it) are split out as **SPEC-008**, which builds on this one.

Facts measured on this machine (ExifTool 13.55, `exiftool -T -n -s3 -IFD0:ImageWidth -IFD0:ImageHeight -IFD0:Orientation <file>`, run over the 29 committed seeds; re-measure, do not generalise): exit status is 0 on every one including garbage; values come out tab-separated in the **order the selectors were requested**; a missing tag prints `-`; some seeds yield nothing or odd output (`two-hop-cycle` prints an empty line; `ascii-shapes` prints `97 98 99 0` for a tag we hold as ASCII). Only stdout is trusted, never the exit status.

## Goal

An ExifTool-only test-support layer (`tests/support/`) with a pure, replayable `parse_fields`, a normalisation table mapping `exif-core` readings to ExifTool's `-n` forms, a committed record of ExifTool's output, a live parity test over hand-built fixtures and the committed seeds, and a **replay-and-perturb red-proof** that shows the comparator names exactly the one field that was perturbed.

## Inputs

- **Read first:** `docs/porting-brief-diamond-exif.md` §2.2 (tool shells, traps), §2.3 (the red-proof disciplines), `docs/exif-core-design.md` §8 (the oracle), `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`), `AGENTS.md`, `guidance/toolchain-brief.md`.
- **Pattern source (irradiance, READ ONLY, never modify; ExifTool's own source is never read):** `~/PSeven/experiments/crustimg_redo_plus/irradiance/tests/support/tools.rs` (`ToolError`, `run_tool`, `exiftool_available`, `run_exiftool`, `parse_fields`) and `tests/metadata_oracle.rs:458-545` (replay-and-perturb red-proof). Port with edit (DEC-003, class 3); add rows to `docs/provenance-ledger.md`. DNG/`Sensor`/dnglab items do not port.
- **This repo:** `src/tiff/` public API (`Container`, `IfdKind`, `ascii`, `sints`, `floats`, `uints`, `urationals`, `srationals`), `fuzz/seeds/tiff_ifd/*` (29 committed seeds), `examples/fuzz_seeds.rs` (shows the byte-builder style).
- **No new dependencies**: `[dependencies]` and `[dev-dependencies]` stay empty (std only; this is why the tab-separated `-T -n -s3` form is used and `-j` is not).

## Outputs

### New test-support (std only; each file compiled into the test crates that need it via `mod` / `#[path]`)

- `tests/support/tools.rs`: `ToolError { NotOnPath, Exec, Parse }` with `Display`; `run_tool` (the **single choke point** for spawning; maps `ErrorKind::NotFound` to `NotOnPath`); `exiftool_available()`; `exiftool_version()`; `run_exiftool(path, selectors) -> Result<Vec<Option<String>>, ToolError>` running `exiftool -T -n -s3 <selector>... <file>` and returning one entry per selector in order (`None` for `-`). Pure `parse_fields(selectors, stdout) -> Result<Vec<Option<String>>, ToolError>`: splits the single output line on tabs, checks the field count equals the selector count (else `Parse`), maps `-` to `None`. **Never inspects the exit status.**
- `tests/support/oracle_tiff.rs`: a small independent byte builder (not shared with `src` tests) and `fixtures() -> Vec<(&'static str, Vec<u8>)>`, the single named list used by the record step, the live test and the replay test. Fixtures (both byte orders where noted): `exif-basic` (LE and BE: IFD0 Make, Model, Software, Artist, Copyright, DateTime, Orientation, ImageWidth, ImageLength, XResolution; ExifIFD DateTimeOriginal, ExposureBiasValue SRATIONAL, FNumber RATIONAL, ISOSpeedRatings SHORT), `exif-gps` (GPS IFD with latitude/longitude and N/S/E/W refs, one fixture per ref combination), `ascii-edges` (unterminated Make, embedded-NUL Model, empty Artist), `gps-corrupt-exif-ok` (GPS pointer past EOF with a readable Exif IFD), `ifd1-corrupt` (IFD0 fine, IFD1 truncated). Deterministic: pure functions of this file.
- `tests/support/oracle.rs`: the **normalisation table** and the comparator.
  - `Selector { exiftool: &str /* e.g. "IFD0:Make", "ExifIFD:FNumber", "GPS:GPSLatitude" */, kind: IfdKind, tag: u16, norm: Norm }`; `enum Norm { Ascii, Uint, Sint, Rational, SRational, Float, GpsCoord { ref_tag: u16 } }`.
  - `read_ours(container, selectors) -> Vec<Option<String>>`: for each selector find the first IFD of that `kind` with the tag and render it in ExifTool's `-n` form. Ascii: `as_bytes` as lossy text. Integers: decimal. Rational: the quotient as decimal text. `GpsCoord`: degrees + minutes/60 + seconds/3600, negated for `S`/`W` per the ref tag. Absent or undecodable: `None`.
  - `diff(ours, theirs, selectors) -> Vec<Mismatch>`: `Mismatch { selector, ours, theirs }`. Equality rules, stated in code comments: text compares exactly; integers exactly; rational, signed rational, float and GPS compare as `f64` with relative tolerance `1e-9` (absolute `1e-12` near zero); `None` equals `None`. A selector whose tag has a zero denominator is skipped with a printed note (ExifTool prints `inf`/`undef`; out of scope here).
  - `KNOWN_DIVERGENCES: &[Divergence]`: empty at the start. An entry is **only** allowed with the exact command, the ExifTool version, the measured output and the fixture; see the criteria below.
- `examples/oracle_record.rs` (a recorder, not a test) runs live ExifTool over `fixtures()` and the 29 seeds and writes `tests/oracle-fixtures/exiftool/<name>.txt`, each file starting with a `#` header carrying the exact command line and ExifTool's version, then the single output line. `just oracle-record` wraps it (add to `app.just`). Output is committed.
- `tests/metadata_oracle.rs` (new integration test), containing:
  - **`live_parity_on_hand_built_fixtures`**: for every fixture, `Container::parse`, `read_ours`, `run_exiftool`, `diff`; no mismatches except `KNOWN_DIVERGENCES`.
  - **`live_parity_on_the_committed_seeds`**: for every file in `fuzz/seeds/tiff_ifd/`, for the selectors `IFD0:ImageWidth`, `IFD0:ImageHeight`, `IFD0:Orientation` (value, where ours is `Ok`); where ours is `Err` (fatal), ExifTool's three fields must all be `None`.
  - **Replay tests, no tool needed**: `replay_is_clean_on_an_unmodified_reading` (the control: committed ExifTool text through `parse_fields` against `read_ours` of the rebuilt fixture gives zero mismatches) and `oracle_names_the_one_field_that_was_perturbed` (perturb exactly one value on our side; assert exactly one `Mismatch` and that it names that selector). One perturbation per `Norm` variant.
  - **Loud skips**: when ExifTool is not on `PATH` a live test prints `SKIP <test> — exiftool not on PATH` and returns; the replay tests still run. If the environment variable `DIAMOND_EXIF_REQUIRE_ORACLE=1` is set, a missing ExifTool is a **test failure** (this is how CI keeps a skip from being a silent pass; setting it in CI is repo setup, not this spec).
  - A live test also prints the live ExifTool version and a loud `NOTE` when it differs from the version in the committed record headers (not a failure).

### Files modified

- `app.just`: `oracle-record` recipe (comment says why it exists; never run in CI).
- `docs/provenance-ledger.md`: rows for `tests/support/tools.rs` and `tests/metadata_oracle.rs` (class 3, irradiance, same author; ExifTool run never read; notes on what was dropped/edited). The new fixture builder and `oracle.rs` are net-new (no row; mention in the notes of the tools row).
- `AGENTS.md` §6 and `guidance/toolchain-brief.md`: how to run the live oracle and `just oracle-record`, and the `DIAMOND_EXIF_REQUIRE_ORACLE` variable.
- `guidance/questions.yaml`: update `rust-ci-and-fixtures` (fixtures: tier A synthetic only for M1a, closed; CI ownership still open); add any divergence question raised below.
- `CHANGELOG.md` (`Added`), `README.md` status line.

## Acceptance Criteria

- [ ] `parse_fields` is pure and never reads an exit status; a field-count mismatch is `ToolError::Parse`.
- [ ] Live parity on every hand-built fixture: our readings equal ExifTool's for the whole selector table (text, integers, rationals incl. signed, GPS coordinates with all four refs), both byte orders.
- [ ] Seed sweep: for all 29 seeds the stated agreement rule holds, **or** the seed is in `KNOWN_DIVERGENCES`.
- [ ] **Divergences are reported, never silenced.** Each entry in `KNOWN_DIVERGENCES` carries: the exact command, ExifTool version, the measured stdout, which side is right and why in one sentence (from the format spec, not from ExifTool's source), and a matching entry in `guidance/questions.yaml`. The handback lists every divergence found. Zero found is acceptable only if the sweep really ran live (record the live run's output counts).
- [ ] **Shown red**: the replay control is clean; for each `Norm` variant, perturbing one value yields exactly one mismatch naming that selector; plus one live-mode demonstration: mutate one byte of a fixture's ASCII payload **after** `read_ours` is computed from the original bytes and show the live comparison go red naming the field, then remove the plant.
- [ ] The comparator also catches a *missing* field (ours `None`, theirs `Some`) and an *extra* field (ours `Some`, theirs `None`): one test each.
- [ ] Loud skips: with ExifTool hidden from `PATH` the live tests print `SKIP` lines and pass, the replay tests still run; with `DIAMOND_EXIF_REQUIRE_ORACLE=1` and ExifTool hidden the live tests fail (demonstrate and record both).
- [ ] The committed `tests/oracle-fixtures/exiftool/*.txt` are exactly what `just oracle-record` produces (re-run it and show an empty `git diff`).
- [ ] No ExifTool source was read; irradiance untouched; `[dependencies]` and `[dev-dependencies]` empty.
- [ ] Library gates green and re-run by the orchestrator: `cargo fmt --check`, `just lint`, `just lint-fuzz`, `cargo test` on stable and 1.90.0, `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`. `src/` is unchanged unless a divergence proves a library bug (then stop and report; do not fix inside this spec).

## Failing Tests

All in `tests/metadata_oracle.rs` (and unit tests inside `tests/support/` modules where `parse_fields`/`diff` are pure). Test names are normative.

**`parse_fields` and tooling**
- `parse_fields_maps_dash_to_none_and_keeps_order`: `"a\t-\tc"` with 3 selectors gives `[Some(a), None, Some(c)]`.
- `parse_fields_rejects_a_field_count_mismatch`: 2 fields for 3 selectors is `ToolError::Parse`; also empty output.
- `run_tool_reports_a_missing_binary_as_not_on_path`: a nonexistent program name gives `ToolError::NotOnPath`.

**Comparator**
- `diff_is_clean_for_equal_readings` (every `Norm`), `diff_applies_the_float_tolerance` (1e-10 relative passes, 1e-6 fails), `diff_reports_a_missing_field`, `diff_reports_an_extra_field`, `gps_coordinate_sign_follows_the_ref_tag` (N/S/E/W).
- `oracle_names_the_one_field_that_was_perturbed_<norm>` for each `Norm` variant (or one parameterised test that asserts per variant).

**Replay** (no tool): `replay_is_clean_on_an_unmodified_reading` for every committed record; `replay_headers_record_command_and_version`.

**Live** (skip loudly if no tool): `live_parity_on_hand_built_fixtures`, `live_parity_on_the_committed_seeds`, `live_demo_a_mutated_payload_is_caught` (the plant is applied inside the test to a copy and asserted red; nothing is committed), `live_skip_is_loud_and_required_mode_fails` (spawns the test binary's logic with a scrubbed `PATH` and with `DIAMOND_EXIF_REQUIRE_ORACLE=1`).

**Fixtures**
- `fixtures_are_deterministic_and_uniquely_named`: two calls give identical bytes; names are unique.
- `every_fixture_parses_or_is_a_designed_failure`: each fixture is either `Ok` or listed as an intentional `Err`.

## Non-Goals

- SPEC-008 material: `mutate.rs` byte-mutation helpers and the mutated-copy library red-proof apparatus (`probe.rs`)
- JPEG/PNG/WebP, XMP, IPTC, ICC, MakerNote; ExifTool *write* mode and its exit codes (question `exiftool-write-exit-codes`, needed before M1c)
- CI jobs, pinning ExifTool in CI, setting `DIAMOND_EXIF_REQUIRE_ORACLE` there (repo setup)
- Printed (non `-n`) value forms and ExifTool's tag name tables; zero-denominator rationals; any tag outside the selector table
- Fixing a library bug a divergence reveals (report it, open a question, stop)

## Notes for the Implementer

- Rule `measurement-over-generalised`: a measured ExifTool behaviour is a claim only with its exact command and scope; do not write "ExifTool always/never ..." in comments.
- ExifTool is run, never read: do not open its source or docs beyond `exiftool -h`/`-ver` output and the normal man page for option syntax. If a format question arises, answer it from the TIFF 6.0 / Exif 2.32 layout knowledge in the repo docs; if you cannot, record it as a question.
- `std` only. Tests may use `unwrap`/`expect` per the test-module allowance; non-test crate code is untouched by this spec.
- Use `std::process::Command` through the one `run_tool` choke point; never call `exiftool` anywhere else.
- Keep selectors in the table minimal and exact (`Group:Tag` forms, family-1 groups such as `IFD0`, `ExifIFD`, `GPS`); verify each selector returns what you expect on `exif-basic` before building on it.
- The 29 seeds are inputs only; do not regenerate or edit them. The single-source idiom applies to the new fixture list: the record example, live test and replay test must all call `fixtures()`.
- If the fuzz target/seeds are untouched, no fuzz change is required here.
- Do not push or open a PR; do not advance cycles; do not modify irradiance or `scripts/`.

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
