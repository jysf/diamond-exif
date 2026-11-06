---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-011
  type: story                      # epic | story | task | bug | chore
  cycle: verify  # frame | design | build | verify | ship
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
  stage: STAGE-002
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-001, DEC-002, DEC-004, DEC-006, DEC-007]
  constraints: [oracle-must-be-shown-red, panic-free-library, forbid-unsafe-code, fuzz-target-per-surface, test-before-implementation, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-005, SPEC-008, SPEC-009, SPEC-010]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-010]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-002's <capability>". Optional; null is acceptable.
value_link: "STAGE-002: JPEG reading is checked against ExifTool, not just against itself, and the differences are written down"

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
  tokens_estimate: 500000
  sessions:
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 415740
      estimated_usd: 2.74
      duration_minutes: 26
      recorded_at: 2026-11-05
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
  totals:
    tokens_total: 415740
    estimated_usd: 2.74
    session_count: 1
---

# SPEC-011: JPEG read parity against ExifTool

## Context

STAGE-001 built the oracle for bare TIFF; SPEC-005 and SPEC-008 gave it a red-proofed comparator and a mutated-copy apparatus. SPEC-009/010 built the JPEG walker and `jpeg::read`, and claimed **container independence** (a TIFF blob gives the same reading bare and inside a JPEG) with a unit test only. This spec points the oracle at JPEG: the claim is checked against ExifTool, the JPEG-specific behaviour (several Exif segments, Exif after the scan, truncation, signatures, XMP/IPTC presence) is measured rather than assumed, and every difference is written down.

**What this design measured first** (ExifTool 13.55, macOS, `-T -n -s3` with the 20 existing selectors, each call under `perl -e 'alarm 8'`; none hung, none exited non-zero, about 35 JPEG shapes tried). These are the facts the spec rests on; the build re-measures every one through the real harness and records it.

| Shape | ExifTool 13.55 | `jpeg::read` today |
|---|---|---|
| one Exif APP1, then scan, EOI | reads it | reads it |
| two Exif APP1s, same tag, different values (either order) | **union of tags, the later segment's value wins** | first block only; second is `ExtraExif` |
| Exif APP1 *after* the first SOS | **reads nothing** | reads it (`after_scan`) |
| JFIF APP0 before or after Exif | reads it | reads it |
| Exif in APP2 | reads nothing | not Exif |
| APP1 `Exif\0` + non-zero 6th byte + valid TIFF | **reads it** | **not Exif (six-byte test)** |
| APP1 `Exif` + non-NUL 5th byte | reads nothing | not Exif |
| APP1 payload exactly `Exif\0` (5 bytes) | reads nothing | not Exif |
| Exif APP1 is the **last** segment and the file ends exactly there | warns `JPEG format error`, **reads nothing** | reads it; `MissingSos`, `MissingEoi` |
| Exif APP1, then a COM segment ending exactly at EOF | reads it | reads it |
| Exif APP1, EOI, no SOS | reads it | reads it; `MissingSos` |
| Exif APP1 *after* EOI | reads nothing; warns `Missing JPEG SOS` | not read; `MissingSos` |
| truncated mid-scan / SOS header only | reads it | reads it; `MissingEoi` |
| bad TIFF inside Exif | warns `Malformed APP1 EXIF segment`, nothing read | block error, JPEG layer clean |
| stray bytes between segments | warns `Skipped unknown N bytes` | `StrayBytes` |
| APP1 length 1 / past EOF | warns `JPEG format error` | `BadSegmentLength` / `Truncated` |
| XMP APP1 (valid packet) | `XMP-xmp:CreatorTool` present | `Region::Xmp` |
| IPTC in APP13 `Photoshop 3.0` + `8BIM` 0x0404 | `IPTC:ObjectName` present | `Region::Iptc` |
| tiny ICC / JUMBF stubs | ExifTool lists nothing | located (not comparable here) |

Three consequences, decided in this design:

1. **The sixth signature byte is a library bug** (the direction that hides data from an audit): DEC-007 (written in this design, normative) makes the Exif test five bytes. This is the only library behaviour change in this spec.
2. **Several Exif segments**: the library policy stays "first is primary, the rest cost `ExtraExif`" (DEC-006). The *oracle* models ExifTool's rule (union of tags, later block wins) in the test harness, so a multi-Exif file is compared against what ExifTool actually reports, and the one place the cap makes ours differ (a 17th block) is a recorded divergence. Whether the library should expose the merged view is left open for the audit CLI stage (question `several-exif-segments`).
3. **Exif after the scan, and a last-segment-at-EOF file**: ExifTool reads nothing, ours reads the block. For an upload audit and sanitiser, reading *more* than the reference is the safe direction, so these are recorded divergences with ours as the right side, not bugs. `ExifBlock` callers that care can ask the segment's `after_scan()`.

## Goal

JPEG fixtures and committed ExifTool records; live and replay parity with zero **unexplained** mismatches (every difference is a `KNOWN_DIVERGENCES` entry with command, version, measured output and the right side); container independence checked by the oracle across every TIFF fixture; JPEG-shaped mutated-copy and byte-mutation red-proofs (control first); the DEC-007 fix with its tests, seeds and fuzz invariant; the SPEC-010 verify gaps closed; and the two apparatus items carried from STAGE-001 (probe wall-clock limit, marker-leak scan scope).

## Inputs

- **Reads:** `src/jpeg/{mod,metadata}.rs`, `tests/support/{tools,oracle,oracle_tiff,probe,mutate}.rs`, `tests/{metadata_oracle,mutation_oracle,fuzz_layout}.rs`, `fuzz/shared/jpeg_metadata_invariants.rs`, `fuzz/fuzz_targets/jpeg_segments.rs`, `examples/{oracle_record,fuzz_seeds}.rs`, `app.just`, DEC-006, DEC-007.
- **Does not read:** ExifTool source or documentation (it is run, never read; constraint). Every ExifTool claim is a command, a version and measured output.
- Constraint `oracle-must-be-shown-red`, `fuzz-target-per-surface`, `panic-free-library`, `forbid-unsafe-code`, `test-before-implementation`; no dependencies.

## Outputs

### 1. Library: DEC-007 (the only `src/` behaviour change)

`src/jpeg/mod.rs` classification: Exif when marker is APP1, payload length >= 6 and `payload[..5] == *b"Exif\0"`. `src/jpeg/metadata.rs`: `tiff = payload[6..]` unchanged. Update doc comments (`SegmentKind::Exif`, module docs), the SPEC-009 classification test (`app_segments_are_classified_by_marker_and_signature` keeps its canonical cases and gains the sixth-byte cases), and add the L2 doc fixes from SPEC-010 verify: `AnyIssue::Exif`/`ExifBlock` issue offsets are relative to the TIFF blob (file offset = `segment.offset() + 10`), and `Error::RegionTooShort` says that `at` is the segment start while `need`/`have` count from after the signature.

### 2. Fixtures: `tests/support/oracle_jpeg.rs` (new, std only, tier A)

An independent builder (it must not share encoding code with `src/` or with `examples/fuzz_seeds.rs`; the TIFF blobs come from `oracle_tiff::fixtures()`, which is already independent). `fn fixtures() -> Vec<(&'static str, Vec<u8>)>`, deterministic, uniquely named.

- **Wrapped (container independence):** every `oracle_tiff::fixtures()` entry except those the TIFF recorder does not run, wrapped as `SOI, APP1 "Exif\0\0" + tiff, SOS header + 2 scan bytes, EOI`, named `jpeg-<tiff-name>`. A TIFF blob longer than 65,527 bytes is not wrapped (none exist; the builder asserts it).
- **Shape fixtures** (names normative, content as the table above): `jpeg-jfif-then-exif`, `jpeg-exif-then-jfif`, `jpeg-exif-in-app2`, `jpeg-exif-sixth-byte-nonzero`, `jpeg-exif-fifth-byte-nonzero`, `jpeg-exif-five-byte-payload`, `jpeg-two-exif-same-tag` (A then B, Make differs), `jpeg-two-exif-same-tag-reversed`, `jpeg-two-exif-disjoint-tags` (union), `jpeg-seventeen-exif-last-differs` (17 blocks, only the 17th has a different Make), `jpeg-exif-after-scan`, `jpeg-exif-after-eoi`, `jpeg-exif-last-segment-at-eof`, `jpeg-exif-then-com-at-eof`, `jpeg-exif-eoi-no-sos`, `jpeg-truncated-between-app1-and-sos` (design §8 red-proof 1), `jpeg-truncated-mid-scan`, `jpeg-truncated-mid-app1`, `jpeg-stray-bytes`, `jpeg-bad-length`, `jpeg-bad-tiff-in-exif`, `jpeg-xmp`, `jpeg-xmp-in-wrong-marker`, `jpeg-iptc`, `jpeg-iptc-without-0404`, `jpeg-extended-xmp-pair`.
- Each shape fixture is **probed alone** under the ExifTool time limit before it joins any loop (the rule in "Process" below); a fixture on which ExifTool does not return becomes a `"*"` divergence and is never run through the tool, exactly like `two-hop-cycle`.

### 3. Comparator additions (`tests/support/oracle.rs`)

- `read_ours_jpeg(meta: &JpegMetadata, selectors) -> Vec<Option<String>>`: for each selector, the value from the **last** Exif block (in file order) whose container is `Ok` and that has the tag; `None` if none. This is the ExifTool model measured above, applied in the harness, not in the library. For a file with at most one Exif block it equals `read_ours` of that block.
- `RegionSel` presence selectors: `XMP-xmp:CreatorTool` <-> any `Region::Xmp`, `IPTC:ObjectName` <-> any `Region::Iptc`. Compared as presence only (`present` / `-`); no value comparison (non-goal).
- `KNOWN_DIVERGENCES` entries (all fields filled from the build's own measurement; the right side is stated per entry): `jpeg-exif-after-scan`, `jpeg-exif-last-segment-at-eof`, `jpeg-seventeen-exif-last-differs`, plus any the seed sweep exposes. An entry is never a loosening: the live and replay tests require the divergence to be *present* (a divergence that stops being one fails the test, so the list cannot rot).

### 4. Records

`tests/oracle-fixtures/exiftool/jpeg-<name>.txt` for every JPEG fixture, produced by `just oracle-record` (the recorder grows the JPEG fixtures; `two-hop-cycle`-style not-run entries carry an `# outcome:` line). **JPEG records also carry `# stderr:` lines** (ExifTool's warnings, one per line, path replaced by `<file>`), because for JPEG the warning is the signal; the TIFF records are **byte-identical** to today's (the stderr capture applies to `jpeg-*` records only). The `oracle-records` CI job re-records and requires an empty diff.

### 5. Tests, live and replay (names in "Failing Tests")

Live tests run ExifTool (skip loudly without it; `DIAMOND_EXIF_REQUIRE_ORACLE=1` makes a missing tool a failure, as today); replay tests run the same comparator against the committed records with no tool. A **warning characterisation table** pins, per fixture, our JPEG-layer issue kinds beside the substring ExifTool printed (or none); it is a characterisation, asserted both ways (a changed ExifTool warning or a changed library issue fails the test), not a claim that the two are equivalent.

### 6. Red-proofs (control first, exact-once needle guard, `/* RED-PROOF INJECTION */` marker; the SPEC-008 apparatus)

- **Byte mutations of a JPEG fixture** (design §8 red-proof 2): corrupt an IFD offset *inside the APP1 payload* (offsets computed from `mutate::locate_entry` on the TIFF part plus the segment offset + 10), then require that (a) the oracle comparison names the corrupted field, (b) `jpeg::read` returns Ok, the JPEG layer stays clean and the damage is an issue on that Exif block (layering), (c) truncating between APP1 and SOS gives the typed outcome (`MissingSos` + `MissingEoi`, block still read), no panic.
- **Mutated-copy library faults** (the probe crate gains a JPEG mode: the probe binary, given a file starting `FF D8`, reads through `jpeg::read` and `oracle::read_ours_jpeg`; TIFF mode unchanged): faults injected into the copy of `src/jpeg/metadata.rs` / `src/jpeg/mod.rs`: (m1) `tiff` slice starts at 5; (m2) the sixth-byte check restored (DEC-007 regression); (m3) `exif()` / block choice takes the last block as primary (this must be caught by `jpeg-two-exif-*`, i.e. the harness model, not by the library tests); (m4) Exif after the scan skipped; (m5) cap `MAX_EXIF_BLOCKS` off by one low (caught by the 17-block fixture's divergence disappearing); (m6) APP1 marker guard relaxed to any APP (caught by `jpeg-exif-in-app2`); (m7) region presence: IPTC id compare wrong. Each: the unmutated copy is compared clean first; the mutated reading must produce a **named** mismatch, not merely any.

### 7. Fuzz (DEC-007 and the SPEC-010 gaps)

- `fuzz/shared/jpeg_metadata_invariants.rs` and `fuzz/fuzz_targets/jpeg_segments.rs`: the `kind` re-derivation uses the five-byte rule; the iff-invariant is written both ways (Exif iff APP1 and >= 6 bytes and first five bytes match). Show it red with a plant against the real libFuzzer target (restore the six-byte test in the plant).
- New seeds (committed, deterministic, existing 63 TIFF+JPEG seeds (29 + 34) byte-identical): `exif-sixth-byte-nonzero`, `exif-five-byte-payload`, `icc-seq-zero` (SPEC-010 verify I1). Seed count `jpeg_segments`: 34 -> 37.
- The fuzz seeds that are JPEGs also feed the live sweep (below), after being probed one by one.

### 8. SPEC-010 verify gaps (low), closed

L1: `the_jpeg_layer_issue_total_never_exceeds_the_documented_constant` gets its real worst case (4096 minimal segments, no SOS, no halt: `MissingSos`, `MissingEoi` appended; total checked against `MAX_JPEG_METADATA_ISSUES`) and the vacuous halted case is relabelled. L3: deterministic tests for the sixth-byte cases (now the DEC-007 tests). I1: the `icc-seq-zero` seed. L2: docs (above).

### 9. Apparatus items carried from STAGE-001

- `probe.rs` `build_probe`/`run_probe` get a wall-clock limit (build 600 s, run 30 s; `DIAMOND_EXIF_PROBE_BUILD_SECS` / `DIAMOND_EXIF_PROBE_RUN_SECS` override; same kill-and-reap shape as `run_tool_with_limit`; a timeout is a typed error and a test failure, never a hang). Test: a probe given a fixture that makes the fault loop is killed within the limit (use a tiny limit via the env var).
- The marker-leak test (`mutation_oracle.rs` around line 411) scans `src/` **and** `tests/support/*.rs` (it missed `tests/support/oracle.rs`).
- Not done here (left open, one line in `signals.yaml`): an SBYTE oracle selector (no standard Exif tag of that type is in the selector table).

### Process rules the build follows

1. Every new JPEG shape is run through ExifTool **alone, under the limit,** before it joins a record, sweep or loop. A non-return becomes a `"*"` divergence.
2. Every difference found is a `KNOWN_DIVERGENCES` entry (command, version, measured stdout, right side) **and** an entry in `guidance/questions.yaml`. The questions to add: `several-exif-segments` (ExifTool unions and the later wins; ours is first-primary; open for the audit stage), `exiftool-ignores-exif-after-scan`, `exiftool-drops-exif-when-last-segment-ends-at-eof`, `exiftool-reads-17th-exif-block` (or the measured outcome), `c2pa-in-jumbf` (carried from SPEC-010; unmeasured, stays open).
3. If the sweep shows a difference that points at a **library bug other than DEC-007**, do not fix it silently: record the divergence as unexplained-then-explained only if it is the other tool's quirk; if it is ours, stop the affected test, report it in the Handback with the measured output, and the orchestrator decides.
4. Do not modify `decisions/`, `scripts/` or irradiance. ExifTool is run, never read.

## Acceptance Criteria

- DEC-007 implemented; `Exif\0` + non-zero 6th byte is an Exif block, `Exif`+non-NUL, a 5-byte payload and APP2 are not; the oracle agrees with ExifTool on all four.
- Every wrapped TIFF fixture reads **identically** bare and inside the JPEG per the oracle (live and replay), with the same set of tags.
- Live and replay: zero unexplained mismatches over all JPEG fixtures and the 34(+3) JPEG fuzz seeds; the divergence list is exactly the measured set, each present in its test.
- The TIFF records are byte-identical; `just oracle-record` produces an empty diff on a clean tree; JPEG records carry command, version, source and `# stderr:` lines.
- Both design §8 JPEG red-proofs are shown red (corrupt IFD offset in the APP1 payload; truncate between APP1 and SOS), each with its control run first.
- Mutated-copy faults m1-m7 each give a named mismatch (or, for m5, the divergence-disappears failure); the control run is clean first for each.
- Fuzz: the five-byte iff-invariant shown red with a planted bug against the real target; `just fuzz-smoke jpeg_segments` clean for 60 s (executions and coverage recorded); `tiff_ifd` still serves; fmt on both graphs.
- The probe has a wall-clock limit that is tested; the marker-leak test covers `tests/support`.
- SPEC-010 verify findings L1, L2, L3, I1 closed.
- All gates: fmt (both graphs), `just lint`, `lint-fuzz`, `lint-no-allow`, `cargo test` on stable and 1.90.0, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` (empty diff), and CI's `oracle-records` job and `lint-fuzz` red-proof still green (run `just lint-fuzz-red-proof` locally this time: it is CI-only otherwise).
- No dependencies; `scripts/`, `decisions/`, irradiance untouched.

## Failing Tests

Named tests (a rename or fold is a deviation to report):

**tests/metadata_oracle.rs (or a new `tests/jpeg_oracle.rs` including the same support files; builder's choice, stated in the Handback):**
- `jpeg_fixtures_are_deterministic_and_uniquely_named`
- `every_wrapped_tiff_fixture_is_present_and_matches_its_tiff_bytes` (the JPEG payload after the signature equals the TIFF blob)
- `replay_jpeg_records_are_clean_on_an_unmodified_reading`
- `replay_jpeg_headers_record_command_version_and_stderr`
- `tiff_records_are_byte_identical_to_the_committed_set` (hash list, so a stderr change cannot leak into TIFF records)
- `oracle_names_the_one_field_perturbed_in_a_jpeg_record`
- `container_independence_every_wrapped_fixture_reads_the_same_bare_and_in_jpeg` (replay; compares each `jpeg-<n>` record's tags with `<n>`'s)
- `live_container_independence_against_exiftool`
- `live_jpeg_parity_on_hand_built_fixtures`
- `live_jpeg_parity_on_the_jpeg_fuzz_seeds` (explained divergences must be present)
- `multi_exif_is_compared_against_the_union_with_the_later_block_winning` (same-tag, reversed, disjoint)
- `exif_signature_is_five_bytes_per_exiftool` (sixth-byte-nonzero read, fifth-byte-nonzero not, five-byte payload not, APP2 not)
- `exif_after_scan_is_a_recorded_divergence_and_still_present`
- `last_segment_at_eof_is_a_recorded_divergence_and_still_present`
- `seventeenth_exif_block_divergence_is_recorded_and_still_present`
- `region_presence_xmp_and_iptc_agree_with_exiftool`
- `iptc_without_0404_and_xmp_in_wrong_marker_are_absent_on_both_sides`
- `warning_table_pins_our_issue_kinds_beside_exiftools_warnings`
- `a_divergence_that_stops_being_one_fails` (flip the expectation on a fixture; the list cannot rot)

**tests/mutation_oracle.rs:**
- `jpeg_byte_mutation_corrupt_ifd_offset_in_app1_payload_is_named`
- `jpeg_byte_mutation_keeps_the_jpeg_layer_clean_and_the_damage_on_the_block`
- `jpeg_truncated_between_app1_and_sos_is_a_typed_outcome`
- `jpeg_mutant_tiff_slice_starts_at_five_is_named` (m1), `..._six_byte_signature_restored_is_named` (m2), `..._last_block_primary_is_named` (m3), `..._exif_after_scan_skipped_is_named` (m4), `..._exif_block_cap_off_by_one_is_caught` (m5), `..._app1_marker_guard_relaxed_is_named` (m6), `..._iptc_id_compare_wrong_is_named` (m7)
- `jpeg_probe_control_run_is_clean_before_every_mutant`
- `probe_build_and_run_have_a_wall_clock_limit`
- `injection_marker_does_not_leak_into_src_or_tests_support`

**src (unit) and tests/fuzz_layout.rs:**
- `exif_signature_is_five_bytes_sixth_byte_ignored` (and the 5-byte-payload, 4-byte, fifth-byte cases)
- `app_segments_are_classified_by_marker_and_signature` (updated)
- `the_jpeg_layer_issue_total_never_exceeds_the_documented_constant` (real worst case)
- `icc_seq_zero_seed_yields_one_mismatch_issue`
- `jpeg_harness_five_byte_iff_both_directions`
- `new_seeds_have_the_shapes_they_claim` (SPEC-011 seeds)
- the existing 63 seeds byte-identical (hash list)

## Non-Goals

- PNG, WebP, HEIC; XMP/ICC/IPTC/JUMBF *value* comparison (presence of XMP and IPTC only); C2PA detection
- Exposing a merged multi-Exif view in the library (question `several-exif-segments`, audit stage)
- Changing the library to ignore Exif after the scan or at EOF (recorded divergences, ours is the safer side)
- The SBYTE selector; writing; the `exif-audit` CLI; real camera files (tier A only)

## Notes for the Implementer

- Order of work: DEC-007 test-first (library and fuzz invariant) -> the JPEG builder and its determinism test -> probe each shape alone under the limit and write the divergence list from what you measure (this design's table is a prediction; where your measurement differs, the measurement wins and the Handback says so) -> records -> comparator additions -> live/replay tests -> red-proofs -> probe limit and marker scan -> fuzz seeds and smoke.
- The recorder is `examples/oracle_record.rs`; keep its TIFF output byte-identical. The seed sweep must skip, loudly, any seed ExifTool does not return on.
- The harness model of ExifTool's multi-Exif rule lives in `tests/support/oracle.rs` and is itself red-proofed by `multi_exif_...` and m3; do not move it into `src/`.
- Report in the Handback: every divergence with its measured output, every shape whose measurement differed from this spec's table, the fuzz-smoke executions and coverage, and each red-proof's before/after.
- Shell: use `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint` and `just lint-fuzz-red-proof` under the default PATH (the latter needs stable clippy); fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`. No `timeout(1)` on this host: use `perl -e 'alarm N; exec @ARGV'`. Never measure ExifTool's exit status through a pipe.
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
