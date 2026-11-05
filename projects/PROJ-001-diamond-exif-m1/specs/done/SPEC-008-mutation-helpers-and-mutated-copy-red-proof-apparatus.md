---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-008
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: L             # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: approved  # approved | punch-list | rejected — the OUTCOME of the verify
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
  constraints: [oracle-must-be-shown-red, exiftool-never-linked, no-new-top-level-deps-without-decision, test-before-implementation]
  related_specs: [SPEC-005]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-005]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-001's <capability>". Optional; null is acceptable.
value_link: "STAGE-001: closes the stage's oracle-must-be-shown-red bar by proving a fault injected into the library is caught by the ExifTool comparison"

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
      tokens_total: 261095
      estimated_usd: 1.72
      duration_minutes: 16
      recorded_at: 2026-11-05
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
    - cycle: verify
      agent: claude-opus-5-5
      interface: other
      tokens_total: 217899
      estimated_usd: 1.44
      duration_minutes: 12
      recorded_at: 2026-11-05
      notes: "verdict APPROVED at 8acb291; tokens transcribed by the orchestrator from the sub-agent metered result"
  totals:
    tokens_total: 478994
    estimated_usd: 3.16
    session_count: 2
shipped_at: 2026-11-05
---

# SPEC-008: Mutation helpers and mutated-copy red-proof apparatus

## Context

SPEC-005 showed the **comparator** can fail (replay-and-perturb). It did not show that a **fault in the library** gets caught by the oracle: the verifier's library plants ran by hand in scratch copies and are not repeatable. Constraint `oracle-must-be-shown-red` is satisfied properly only when a mutated copy of the library is built, run on the fixtures, and compared with ExifTool's committed answers to produce a red result, after a **control** run proves the unmutated copy is green (irradiance DEC-009; `scripts/lint-red-proof.sh`). Brief §2.3: irradiance copy-pasted this apparatus three times (`stage_probe_crate`, `build_and_run_probe`); lift it into `tests/support/` **once**. Design §8 also lists byte-mutation red-proofs (corrupt an IFD offset, truncate) and none of the helpers exist.

Two carry-ins from the SPEC-005 verify are folded in: (1) the tool runner has no time or memory limit (ExifTool 13.55 balloons to about 9 GB in 5 s on `two-hop-cycle` before the OS kills it); (2) an SSHORT-read-as-unsigned plant survived every oracle test because no live selector reads a signed integer or float.

The red-proofs compare against the **committed ExifTool records** (`tests/oracle-fixtures/exiftool/*.txt`), so they run in CI with no ExifTool installed and no new live-tool risk. Live variants run when the tool is present.

## Goal

Total, panic-free byte-mutation helpers; one shared probe apparatus that stages a mutated copy of `src/` as a throw-away crate, builds it, runs it on the fixtures and returns its readings; at least five library-fault red-proofs and three byte-mutation red-proofs, each with its control run first; a wall-clock limit on every ExifTool spawn; and a live oracle case that catches a signed-integer fault.

## Inputs

- **Read first:** `docs/porting-brief-diamond-exif.md` §2.3, `guidance/constraints.yaml` (`oracle-must-be-shown-red`), `AGENTS.md`, `guidance/toolchain-brief.md`; the SPEC-005 files `tests/support/{tools,oracle,oracle_tiff}.rs`, `tests/metadata_oracle.rs`, `tests/oracle-fixtures/exiftool/*.txt`; SPEC-005's HANDOFF-013/014 handbacks (what survived, what was divergent).
- **Pattern source (irradiance, READ ONLY, never modify):** `~/PSeven/experiments/crustimg_redo_plus/irradiance/tests/plane_oracle.rs` lines ~626-805 (`TempDir`, `copy_dir_recursive`, `inject_*_fault`, `stage_probe_crate`, `build_and_run_probe`) and its control test (~1044). Port with edit (DNG/plane specifics dropped) and add a ledger row (DEC-003, class 3). **ExifTool's source is never read.**
- **Host:** ExifTool 13.55 at `/opt/homebrew/bin/exiftool`. **Never run ExifTool on `fuzz/seeds/tiff_ifd/two-hop-cycle`**; it runs away. After this spec the runner's limit is the second line of defence, not the first.
- Library `src/` is **not changed** by this spec. No dependencies or dev-dependencies.

## Outputs

### `tests/support/mutate.rs` (std only, total, never panics)

Pure functions over `&mut Vec<u8>` / `&[u8]`; an out-of-range request leaves the bytes unchanged and reports it (returns `false`/`None`), never panics:

- `set_u16_at(bytes, at, value, order) -> bool`, `set_u32_at(bytes, at, value, order) -> bool` (`order` is a local `Order { Little, Big }`).
- `flip_bit(bytes, byte_index, bit) -> bool` (bit 0..=7; flipping twice restores).
- `truncate_at(bytes, len)` (a `len` at or past the end is a no-op).
- `swap_byte_order_mark(bytes) -> bool`: `II` to `MM` and back, **only** bytes 0..2; the rest is deliberately left as written, so the file now lies about its byte order.
- `locate_entry(bytes, ifd_offset, tag) -> Option<EntryLoc { entry_at, value_at }>`: a tiny independent reader (not `exif-core`) that finds a tag's 12-byte entry in an IFD so a test can corrupt "the GPS pointer value" without hard-coding offsets.

### `tests/support/probe.rs` (std only)

- `TempDir` (removed on drop, even when the test panics; unique per process and test), `copy_dir_recursive`.
- `stage_probe_crate(dest, inject: Option<&dyn Fn(&Path)>) -> ()`: writes a minimal manifest for package `exif-core` (no dependencies, same `[lib]`), copies `src/` from the real tree, copies `tests/support/oracle.rs` next to a generated `src/bin/probe.rs`, then calls `inject` with the staged `src/` directory. The probe binary parses the file given as its argument with `exif_core::tiff::Container`, renders `oracle::SELECTORS` through the **same** `read_ours` code, and prints the readings as tab-separated text (`-` for `None`), so the mutated library is compared through exactly the normalisation the live oracle uses.
- `build_and_run_probe(dir, file) -> Result<Vec<Option<String>>, String>`: builds with the cargo that is running the tests (`CARGO` env var, falling back to `cargo`) in its **own** target directory under the `TempDir`, with `--offline`, so it never touches the repo's `target/`; runs the probe; parses stdout; every failure is a `String` that includes the captured stderr.
- `inject_replace(path, needle, replacement)`: reads a staged file, **asserts the needle occurs exactly once** (a moved injection point must fail loudly, never turn a red-proof vacuous), writes it back. Every injected replacement carries a `/* RED-PROOF INJECTION */` marker comment.

### `tests/mutation_oracle.rs` (new integration test; the red-proofs)

Each library-fault test has the same shape: **control first** (the unmutated staged copy, run on the fixture, equals the committed ExifTool record: zero mismatches through `oracle::diff`), then the mutated copy, which must give **at least one** mismatch **naming the expected selector(s)**. Parse committed records with `tools::parse_record`/`parse_fields`; no live tool is needed.

Library faults (the needle is whatever uniquely locates the code in the current `src/`; the builder finds it and states it in a comment; if a needle moves the test fails, per `inject_replace`):

1. `mutant_big_endian_short_read_is_caught`: `ByteOrder::u16` big-endian arm reads little-endian. Expect mismatches on SHORT-valued selectors (Orientation, ISO) of the `*-be` fixture, and **none** on the little-endian fixture (the mutation cannot show there; assert that, it is the proof the test reads the right file).
2. `mutant_ascii_drops_the_last_byte_is_caught`: expect `IFD0:Make` on the unterminated `ascii-edges` fixture.
3. `mutant_rational_swaps_numerator_and_denominator_is_caught`: expect a Rational selector (FNumber or XResolution).
4. `mutant_gps_pointer_is_not_followed_is_caught`: remove the GPS arm of the pointer table; expect the `Composite:GPSLatitude/Longitude` selectors to go `None` against ExifTool's values.
5. `mutant_signed_rational_sign_is_lost_is_caught`: `srationals` forces a positive numerator; expect `ExifIFD:ExposureCompensation` (or the SRATIONAL selector in the table).
6. `mutant_signed_short_is_read_unsigned_is_caught`: **the SPEC-005 survivor.** `ByteOrder::i16` (or the `sints` SSHORT arm) zero-extends; this must go red through the new signed selector below.

Byte-mutation red-proofs (use `mutate.rs`; no probe crate needed; each compares the mutated bytes' reading with the **original** fixture's committed ExifTool record):

7. `corrupting_the_gps_pointer_is_reported_and_isolated`: `exif-gps` with the GPS pointer value set past EOF via `locate_entry` + `set_u32_at`. Expect: the diff names exactly the GPS selectors; IFD0 and Exif selectors still match; `Container` carries one issue with `Link::Tag(0x8825)`. **Live variant** (skip loudly without ExifTool): live ExifTool on the mutated bytes also returns IFD0/Exif selectors and no GPS values.
8. `swapping_the_byte_order_mark_changes_the_reading`: `exif-basic` with `swap_byte_order_mark`: the reading must **differ** from the record (not equal, no panic); state what it actually does (parse error or garbage values) as measured.
9. `truncating_into_the_exif_ifd_loses_exactly_what_it_should`: truncate just before the Exif IFD's payload region; Exif selectors vanish, IFD0 selectors intact, issue reported, no panic.

### Tool limits and the signed/float live case (SPEC-005 carry-ins)

- `tests/support/tools.rs`: every ExifTool spawn gets a **wall-clock limit**. `run_tool` spawns the child, polls `try_wait` and `kill`s on expiry, returning a new `ToolError::Timeout { tool, secs }` (with `Display`). Default **5 s**, overridable by `DIAMOND_EXIF_TOOL_TIMEOUT_SECS`. `run_tool` stays the **only** spawn point. State plainly in the doc comment that this bounds time and therefore memory growth only approximately (measured: ExifTool 13.55 on `two-hop-cycle` reached about 9 GB at 5 s); it is defence in depth, and the harness still never runs that seed. A live test treats `Timeout` as a failure with a clear message, not a skip.
- **Signed/float selector.** Add to the selector table and fixtures at least one tag read through `sints` (SSHORT or SLONG) and one through `floats`, **if ExifTool honours the declared field type for some tag** (measure with `exiftool -T -n -s3 ...` on a hand-built file; try at least four candidate tags, record every command and output in the Handback). If after four honest attempts no tag exposes the type, record that measurement as a `guidance/questions.yaml` entry, keep the signed-short mutant (6) red through a **unit-level probe reading** (probe prints a signed value for a hand-built fixture and the expected text is asserted from the format spec), and say so in the Handback. Prefer the real selector.

### Other files

- `app.just`: `mutation-oracle` recipe (runs `cargo test --test mutation_oracle`, comment says it builds throw-away crates and takes longer).
- `docs/provenance-ledger.md`: row for `tests/support/probe.rs` / the TempDir and staging code (class 3, irradiance, same author); `mutate.rs` and the new red-proofs are net-new.
- `AGENTS.md` §6 and `guidance/toolchain-brief.md`: how to run the mutation red-proofs, the `CARGO` trap (bare Homebrew cargo vs rustup), the timeout variable.
- `CHANGELOG.md` (`Added`), `README.md` status line, `guidance/questions.yaml` updates.

## Acceptance Criteria

- [ ] `mutate.rs` helpers are total: out-of-range inputs leave bytes unchanged and report it, never panic; each has a unit test including the boundary case (index == len, bit 8, truncate beyond len, `swap_byte_order_mark` on a non-TIFF).
- [ ] The probe apparatus exists **once** in `tests/support/probe.rs`; grep shows no copy of stage/build/run logic elsewhere. `TempDir` cleans up on panic (tested), and no test leaves files in the repo tree or touches the repo's `target/`.
- [ ] **Each of the six library-fault tests has its control run first and the control is green**, and each mutant is red naming the stated selector(s). For mutant 1 the little-endian fixture stays green.
- [ ] An injection whose needle is absent or duplicated fails loudly (test for `inject_replace` itself, both cases).
- [ ] The three byte-mutation red-proofs pass and assert the stated isolation properties; the live variant of (7) runs when ExifTool is present and skips loudly otherwise.
- [ ] ExifTool spawns are time-limited: a test uses a slow tool (a `sleep` through `run_tool`, unix only) with a 1 s limit and gets `ToolError::Timeout` within a bounded time and no process left behind (check). `run_tool` is still the only spawn point (grep).
- [ ] The SPEC-005 SSHORT survivor is dead: with mutant 6 injected, an oracle test is red (or, in the documented fallback, the unit-level probe test is red) and a normal run is green.
- [ ] The red-proofs run with ExifTool absent (`PATH` scrubbed) and `DIAMOND_EXIF_REQUIRE_ORACLE` unset: only the live variants skip, loudly.
- [ ] Wall-clock for `just mutation-oracle` is recorded in the Handback (each probe build is a full compile of a small crate; if the whole file exceeds roughly 3 minutes, say so and propose how to share a target dir between mutants).
- [ ] `src/`, `scripts/`, `decisions/`, irradiance and the 29 seeds untouched; `[dependencies]` and `[dev-dependencies]` empty; ExifTool never run on `two-hop-cycle`.
- [ ] Library gates green and re-run by the orchestrator: `cargo fmt --check`, `just lint`, `just lint-fuzz`, `cargo test` on stable and 1.90.0, `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`, plus `just oracle-record` leaving an empty `git diff` (never touching `two-hop-cycle`).

## Failing Tests

Names are normative.

**`tests/support/mutate.rs` unit tests** (run in the test crate that includes it): `set_u32_at_writes_both_byte_orders_and_rejects_out_of_range`, `set_u16_at_boundary`, `flip_bit_twice_restores_and_rejects_bit_8`, `truncate_at_beyond_the_end_is_a_no_op`, `swap_byte_order_mark_flips_only_the_mark_and_rejects_non_tiff`, `locate_entry_finds_a_tag_and_none_for_absent_or_out_of_range`.

**`tests/support/probe.rs` unit tests**: `tempdir_is_removed_on_drop_and_on_panic`, `inject_replace_asserts_exactly_one_occurrence` (absent and duplicate both fail), `stage_probe_crate_does_not_touch_the_repo_tree`.

**`tests/mutation_oracle.rs`**: the nine tests above, plus `controls_are_green_for_every_fixture_used` (the control for each fixture the mutants use, run once and shared if the builder can do so without coupling).

**`tests/metadata_oracle.rs` / `tools.rs`**: `run_tool_times_out_and_kills_the_child` (unix), `timeout_variable_overrides_the_default`, `live_signed_and_float_selectors_match_exiftool` (or the documented fallback test), and the existing 22 tests unchanged in meaning.

## Non-Goals

- JPEG-level red-proofs (truncate between APP1 and SOS, pixel flip) and I3 write parity: they need the container and writer stages
- CI wiring, `lint-red-proof` script work, a memory limit (std cannot set one portably; the time limit is the control)
- New library behaviour; fixing a bug a red-proof reveals (stop, report, open a question)
- Mutating the fuzz target or seeds

## Notes for the Implementer

- Rule `measurement-over-generalised`: every claim about ExifTool in a comment carries the exact command and version; **never write "ExifTool always/never ..."**. Capture a tool's output to a file or variable before reading its status; do not read `$?` through a pipe.
- A red-proof that cannot fail is worse than none: every mutant test must name the selector it expects, and the needle guard must be exact-once. Do not weaken a control to make a mutant look red.
- Keep each mutation small and local (one expression). Mark it with the `RED-PROOF INJECTION` comment so a leaked injection is greppable; add a test or grep check that no committed file contains the marker outside the tests.
- Probe builds use their own target directory and `--offline`; a stale `CARGO_TARGET_DIR` in the environment must not redirect them into the repo's `target/` (set it explicitly on the child).
- If staging via `oracle.rs` needs small refactors in the shared test-support files, keep behaviour unchanged and say so in the Handback.
- Do not push or open a PR; do not advance cycles; do not modify irradiance or `scripts/`.

---

## Reflection

1. **What would I do differently next time?**
   — Specify the placement of a byte-order mutant, not just its effect. The spec put mutant 1 in `ByteOrder::u16`'s big-endian arm, but that arm also reads the header version word, so the mutated file fails at the header (`UnsupportedTiffVersion`) and can never name a selector; the builder correctly moved it to the SHORT use site in `Container::uints`. Also say explicitly that the probe's own build and run need a wall-clock limit: I limited ExifTool spawns but not the probe, so a looping mutant can hang the suite (verify finding F1).

2. **Does any template, constraint, or decision need updating?**
   — No decision change. The "test the testers" instruction in the verify handoff paid off: the verifier weakened a control, changed expected selector sets, broke a needle, and wrote its own mutants, and all the apparatus checks held. It also found a real library-test gap the ExifTool comparison cannot see: an off-by-one `SubIFDs` cap (`n > MAX_IFDS + 1`) survived the whole suite. I closed that at ship with `a_subifds_count_one_over_the_cap_is_rejected_before_decoding` and mutation-checked it (the mutant now turns it red).

3. **Is there a follow-up spec I should write now before I forget?**
   — Small, none urgent: run the probe's cargo build and binary through the same poll-and-kill helper as ExifTool (F1); the marker-leak test scans only `src`, `examples`, `fuzz` and `scripts`, so it misses `tests/support/oracle.rs` which is copied into every probe crate (F4); an SBYTE oracle selector does not exist, so SBYTE-as-unsigned is caught only by a library unit test (F3). A mutant that breaks parsing reports an apparatus error rather than naming selectors (F5). Fixed at ship: an over-generalised comment about "most tags" in `oracle_tiff.rs` (F2; 3 of 17 candidate tags did not follow the declared type).

4. **Where was the worst defect caught?** — verify
   *(the missing 513-count boundary test for the SubIFDs cap, a SPEC-004 gap that surfaced only because the verifier wrote its own mutants.)*

5. **What can a user do now that they couldn't before?** — Re-run `just mutation-oracle` in about three seconds and watch six deliberately broken copies of the parser (wrong byte order, dropped ASCII byte, swapped rational, unfollowed GPS pointer, lost sign, unsigned SSHORT) each get caught by the ExifTool comparison after their controls pass, so "the oracle would notice a real bug" is something that can be demonstrated rather than believed.
