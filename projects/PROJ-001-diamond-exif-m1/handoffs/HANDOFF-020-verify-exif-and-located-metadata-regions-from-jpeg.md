---
# Maps to ContextCore handoff.* semantic conventions.
#
# ONE handoff per delegated CYCLE. With build and verify running on different
# agents you get TWO handoffs per spec (HANDOFF-N build, HANDOFF-M verify) —
# `handoff.cycle` is what distinguishes them.
#
# The `handback:` block below is the RETURN path and it is not optional: it is
# how cost gets into the spec without the orchestrator hand-counting anything.
# `just handback-sync SPEC-NNN` reads it and appends the cost session for you.
# Rationale + the full contract: docs/decisions/DEC-013-delegated-cost-handback.md

handoff:
  id: HANDOFF-020
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-010

project:
  id: PROJ-001
  stage: STAGE-002
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number
# from your own interface:
#   Claude Code   → run `/cost`
#   API           → the `usage` object (input + output, summed)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed                     # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: 45
  branch: worktree-agent-aa9d68de47bb5e32e
  pr: null
  completed_at: 2026-11-05               # YYYY-MM-DD
  notes: "verify sub-agent has no /cost; the orchestrator transcribes the metered figure from the sub-agent result. Verdict APPROVED at f4d67db (code 9239b87)."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-020: Verify Exif and located metadata regions from JPEG

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-010` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec (API, extraction rules and named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-010-exif-and-located-metadata-regions-from-jpeg.md`; **DEC-006** incl. its SPEC-010 clarifications; DEC-001, DEC-002, DEC-004, DEC-005
- **Build handoff and Handback:** `HANDOFF-019-build-exif-and-located-metadata-regions-from-jpeg.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`)
- Code under review: `git diff 0052025..HEAD` (build commit `9239b87`); main new file `src/jpeg/metadata.rs`, shared harness `fuzz/shared/jpeg_metadata_invariants.rs`

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `just lint-fuzz`, `just lint-no-allow`, `cargo test` on stable and 1.90.0, `-F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`, `just oracle-record` empty diff). Run `just fuzz-smoke jpeg_segments` for 60 s and record executions and coverage.
- **Read the extraction against the spec rules line by line**: Exif signature and `Container::parse` hand-off (payload[6..]); primary vs `ExtraExif`; the 16-block cap; fatal TIFF error kept on the block, JPEG layer still clean; extended XMP (35-byte signature then 40 header bytes), ICC chunk consistency (one aggregate issue, the builder's `at` rule), IPTC `8BIM` resource walk (id 0x0404, even padding, the builder's resource-offset `at`, last resource may omit its pad), JUMBF `JP`+En+Z; `issues()` layering and order; `is_clean`; the walker halt interaction (nothing recorded after a halt; Exif blocks and regions still reported). Hunt for: arithmetic overflow, out-of-range indexing, unbounded loops or allocation (every resource walk must advance), panic-capable constructs in non-test code, any issue recorded outside the single bounded path, a quadratic blow-up (many segments each scanned repeatedly).
- **Judge two builder policy picks:** (1) ICC `at` when chunks are individually consistent but the count of chunks differs from `count` (first chunk's offset); (2) IPTC `RegionTooShort`/`StrayBytes` use the resource's input offset, `need`/`have` from the resource start. Say whether each is defensible.
- Check the walker change: bad length skips four bytes; the SPEC-009 test updated, nothing else weakened. Diff the SPEC-009 tests for any deletion or loosening.
- **Test the testers.** Redo the seven required mutations (a)-(g); write mutants of your own the builder did not try (e.g. Exif signature compare off by one; `payload[6..]` vs `[5..]`; ExtraExif at the 16 boundary `>` vs `>=`; first-Exif-primary replaced by last; extended-XMP header length 39/41; ICC `seq == 0` accepted; IPTC pad byte ignored; IPTC id compare wrong; JUMBF En/Z byte order swapped; `is_clean` ignoring a block's issues; `issues()` swapping layer order; halt check removed). Report which the suite catches and which survive; the builder says one equivalent mutant survives (unreachable issue-bound guard in `Extract::record`) — judge that. Redo at least three fuzz-invariant plants including one against the real libFuzzer target; confirm iff-invariants are both-directional.
- Container independence: verify the claim yourself with a bare TIFF blob vs the same blob inside a JPEG, including a damaged-GPS-IFD case and a fatal-TIFF case.
- Seeds: the existing 50 are byte-identical; the 13 new ones deterministic (regenerate twice) and each exercises the shape its name claims (inspect with `xxd`). Check the cap constants against the spec formula.
- Check `src/tiff/mod.rs` edit is test-module only; `[dependencies]`/`[dev-dependencies]` empty; `scripts/`, `decisions/`, the oracle code and records, and irradiance untouched; CI workflow unchanged.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-011 work, running ExifTool, CI wiring

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback` section below.** An unfilled handback is an
incomplete cycle, and `just handback-sync` will tell the orchestrator so.

On success:
1. Fill the `handback:` front-matter — **including a real `tokens_total`**.
2. Fill the `## Handback` section (the reflection is part of it, not optional).
3. Set `handoff.status` → `completed` and `handback.status` → `completed`.
4. Open a PR (build) or return your verdict (verify).

If you cannot complete the task:
1. Fill the `## Handback` section with what was done and what blocked you.
2. Set `handoff.status` → `rejected`, `handback.status` → `blocked`.
3. **Still report your token usage** — blocked work costs money too.
4. Set the spec's `task.blocked: true` and add a question to
   `/guidance/questions.yaml`.

---

## Handback

*Filled in by the receiving agent. The orchestrator does not reconstruct any of
this — it transcribes it. The reflection questions are part of completion.*

### Execution notes

- **Branch / PR:** `worktree-agent-aa9d68de47bb5e32e` (fast-forwarded to `f4d67db` from `claude/rust-exif-metadata-6351d5`); no PR (verify cycle).
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes. Every named test exists and passes; rules 1-8 read line by line against the code; the walker change is minimal; every gate is green; the seven required mutations are red; the fuzz smoke is clean.
- **For `verify`:** ✅ **APPROVED at `f4d67db`** (code under review: build commit `9239b87`; `f4d67db` adds only the build-cost record and this handoff). No blocking finding; three Low findings and two informational notes below, none needs a fix before ship.

**Gates (re-run cold on `f4d67db`, `PATH=$HOME/.cargo/bin:$PATH`, `RUSTUP_TOOLCHAIN=1.90.0` unless stated):** `cargo fmt --check` root and `--manifest-path fuzz/Cargo.toml` green; `just lint` green; `just lint-fuzz` (clippy +stable) green; `just lint-no-allow` green; `cargo +stable test` and `cargo +1.90.0 test` both green (143 unit, 4 crate_policy, 10 fuzz_layout, 28 metadata_oracle + 1 ignored, 36 mutation_oracle + 1 ignored); `cargo +1.90.0 rustc --lib -- -F unsafe_code` and `just forbid-unsafe` green; `just deny` and `just deny-fuzz` "licenses ok" (only the existing not-encountered warnings); `just validate` green (11 artifacts); `just decisions-audit` 0 structural errors, 7 existing scope warnings; `just cost-audit` green; `just oracle-record` (ExifTool 13.55, 41 records, `two-hop-cycle` NOT RUN by the recipe) with an empty `git status` afterwards.

**Fuzz smoke (clean tree):** `just fuzz-smoke jpeg_segments`: `#157562 DONE cov: 1466 ft: 6428 corp: 552/3454Kb exec/s: 2582`, "Done 157562 runs in 61 second(s)", exit 0 (INITED at cov 1208 from the 34 seeds and an empty corpus).

**Spec read, line by line (all conform):** Exif: `after(6)` = `payload[6..]`, `Container::parse` of exactly that slice, first is primary, `ExtraExif { at: segment.offset() }` for every Exif after the first including those past the cap, blocks only while `exif_seen < 16`, after-scan Exif read. Fatal TIFF kept on the block; `issues()` yields nothing for it; JPEG layer stays empty. Extended XMP: 35-byte signature, then 32 + 4 + 4 header, `have` = bytes after the signature. ICC: one aggregate issue, `check_icc` runs once after the loop. IPTC: `8BIM`, id BE, Pascal name padded to even, size BE, data plus a pad when odd; the last resource may omit its pad (the cursor overshoots, the loop ends, no issue); each iteration advances by at least 12 bytes, all arithmetic saturating. JUMBF: `JP` + En (u16 BE) + Z (u32 BE), no C2PA claim. `issues()` = JPEG layer then blocks in order; `is_clean` requires every block `Ok` and clean. Halt: `Extract::record` is the only push site and returns when `jpeg.halted()`; blocks and regions are still produced. No indexing/unwrap/panic in non-test code (lint-no-allow green); work is linear in segments with at most 16 TIFF parses; no quadratic re-scan.

**Policy picks, judged:** (1) ICC count-of-chunks mismatch with every chunk individually consistent names the first chunk: **defensible** (no chunk is individually wrong; the first one declared the count the set fails; the rule is in the module docs and pinned by `icc_chunk_mismatches_are_one_issue`, case "count 3 with two chunks present"). (2) IPTC `RegionTooShort`/`StrayBytes` at the resource's input offset with `need`/`have` from the resource start: **defensible** (it points at the faulty resource rather than the segment, it is the only frame in which `need`/`have` are meaningful, and the `Error::RegionTooShort.at` doc says "or of the resource inside it").

**Walker change:** `p + 4` on `BadSegmentLength` only; the SPEC-009 test diff removes exactly the one `StrayBytes { at: 4, len: 2 }` expectation; the new test `a_bad_length_skips_the_marker_and_both_length_bytes` covers the mid-input and at-EOF cases; no other SPEC-009 test, `tests/fuzz_layout.rs` line or seed is deleted or loosened (the only removed line in `tests/fuzz_layout.rs` is the `JPEG_SEEDS` array-length declaration). Rule-2.2 doc corrected.

**Container independence, checked independently** (a temporary `tests/` probe, deleted): a big-endian TIFF (Make, an Exif IFD with a rational, a GPS IFD) read bare and inside APP1 behind a JFIF segment gives an **identical `Debug` of the whole `Container`** (data, byte order, IFDs, issues) for: clean; GPS pointer past EOF (not clean, 1 Exif-layer issue, JPEG layer empty, `exif()` Some); GPS pointer looping to IFD0 (same); fatal `Truncated { at: 65535, len: 2 }` and fatal `NotTiff` (equal errors, `exif()` None, 0 issues, not clean).

**Seeds:** the 50 pre-existing seeds have no diff in `0052025..HEAD` (13 additions only; 29 TIFF + 34 JPEG present). `cargo run --example fuzz_seeds` twice: tree clean both times and an identical hash of the JPEG seed set (deterministic, matches the committed bytes). `xxd` confirms each name: `exif-bad-tiff` (`Exif\0\0junkjunk`), `extended-xmp-short` (payload 74 = 35+39, then 75 = 35+40 at the boundary), `icc-chunks-ok` (1/2, 2/2), `icc-chunks-mismatch` (1/3, 1/2, then a 1-byte header), `iptc-8bim` (0x0408 with odd name "ab" and 3-byte data plus pad, then 0x0404 with 5 bytes plus pad), `iptc-truncated-resource` (size 0x100 with 3 bytes), `jumbf-header` / `jumbf-short` (6 / 5 bytes after `JP`), `exif-in-jpeg` (II TIFF, Make "DiamondCam", Exif and GPS pointers), `exif-after-scan` (APP1 after SOS), `two-exif` (2), `seventeen-exif` (17), `xmp-and-extended-xmp` (one XMP, two extended chunks with one GUID). Cap constant: `MAX_JPEG_METADATA_ISSUES = (1024+1)+(4096+1) = 5122`, as the spec formula, pinned by the test.

**Scope:** the `src/tiff/mod.rs` change is inside `mod tests` only (error guard); `[dependencies]` and `[dev-dependencies]` are empty; no diff in `scripts/`, `decisions/` (after the design commit), `.github/`, `Cargo.toml`, `Cargo.lock`, `fuzz/Cargo.toml`, `justfile`, `app.just`, the oracle code or `tests/oracle-fixtures`. irradiance is outside this worktree and was not inspected.

**Mutants** (each applied alone, `cargo test --lib jpeg::` and `cargo test --test fuzz_layout`, then reverted; tree clean afterwards):

| # | Mutant | Result | Caught by (representative) |
|---|---|---|---|
| a | tiff slice `payload[5..]`; `payload[7..]` | caught / caught | `exif_in_a_jpeg_reads_like_the_bare_tiff`, `a_signature_only_exif_is_a_truncated_tiff`, harness |
| b | `ExtraExif` on the first, not the second | caught | `a_second_exif_segment_is_an_extra_exif_issue_and_still_a_block` |
| c | block cap `<` to `<=`; `+ 1 <` | caught / caught | `exif_blocks_are_capped_at_sixteen_but_extras_are_all_reported` |
| d | ICC check removed | caught | `icc_chunk_mismatches_are_one_issue` |
| e | 8BIM name pad removed; pad inverted | caught / caught | `iptc_0404_resource_is_located_with_even_padding` |
| f | `is_clean` `&&` to `\|\|`; ignores only fatal blocks; JPEG layer only | caught x3 | `is_clean_requires_every_layer`, `a_fatal_tiff_is_kept_on_the_block_not_promoted` |
| g | ext-XMP header 39; 41 | caught / caught | `a_short_extended_xmp_header_is_region_too_short` |
| 1 | Exif signature compared on 5 bytes (SPEC-009 classifier) | **survived** unit + fuzz_layout; caught by the real fuzz target (`kind`) | none deterministic (L3) |
| 2 | primary = last Ok block | caught | `a_second_exif_segment_is_an_extra_exif_issue_and_still_a_block` |
| 3 | ICC `seq == 0` accepted | caught | `icc_chunk_mismatches_are_one_issue` (real target: not found in 60 s, I1) |
| 4 | IPTC data pad ignored | caught | `iptc_0404_resource_is_located_with_even_padding` |
| 5 | IPTC id `0x0444`; `==` to `!=` | caught / caught | `iptc_0404_resource_is_located_with_even_padding` |
| 6 | JUMBF Z little-endian; En/Z fields swapped | caught / caught | `jumbf_header_fields_are_read` |
| 7 | `issues()` layer order swapped | caught | `issues_name_the_layer_jpeg_first_then_blocks_in_order` |
| 8 | halt check removed from `record` | caught | `nothing_is_recorded_after_a_halt` |
| 8b | issue-bound guard removed (the builder's survivor) | survived, **equivalent** (confirmed: unhalted max is 1024 walker + 4096 segments + 1 ICC = 5121 < 5122; a halted walk records nothing) | n/a |
| 9 | ICC count-mismatch `at` = last chunk | caught | `icc_chunk_mismatches_are_one_issue` |
| 10 | IPTC `body_at` off by one | caught | `a_truncated_8bim_resource_is_region_too_short_and_stops` |
| 11 | `ExtraExif.at` + 1 | caught | `a_second_exif_segment_is_an_extra_exif_issue_and_still_a_block` |
| 12 | ext-XMP `have` = payload length | caught | `a_short_extended_xmp_header_is_region_too_short` |
| 13 | ICC data from byte 1 | caught | `icc_chunks_report_sequence_and_count` |
| 14 | XMP packet off by one | caught | `xmp_packet_is_the_payload_after_the_signature` |
| 15 | ICC `differs` skips chunk 1 | caught | `icc_chunk_mismatches_are_one_issue` |
| 16 | walker bad-length skip back to 2 | caught | `a_bad_length_skips_the_marker_and_both_length_bytes` |
| 17 | IPTC single trailing byte ignored | caught (harness only) | `jpeg_harness_holds_on_each_extraction_guard_alone` |
| 18 | `ExtraExif` not reported past the cap | caught | `exif_blocks_are_capped_at_sixteen_but_extras_are_all_reported` |
| 19 | ICC repeated seq ignored | caught | `icc_chunk_mismatches_are_one_issue` |
| 20 | ICC `seq > count` to `>=` | caught | `consistent_icc_chunks_are_clean` |
| 21 | 8BIM `need` for the size field = `size_at` | caught (harness only) | `jpeg_harness_holds_on_each_extraction_guard_alone` |
| 22 | IPTC `StrayBytes.len` = 1 | caught | `non_8bim_bytes_where_a_resource_should_start_are_stray_bytes` |
| 23 | ext-XMP data `.or(Some(&[]))` | survived, equivalent (unreachable once the first three gets are `Some`; an ill-chosen mutant) | n/a |
| 24 | fatal blocks dropped (no block) | caught | `a_fatal_tiff_is_kept_on_the_block_not_promoted` |
| 25 | container parses the payload including the signature | caught | `exif_in_a_jpeg_reads_like_the_bare_tiff` |
| 27 | `issues()` only from block 0 | caught | `issues_name_the_layer_jpeg_first_then_blocks_in_order` |
| 29 | 8BIM `have` = whole body length | caught | `a_truncated_8bim_resource_is_region_too_short_and_stops` |
| 30 | `ExtraExif` only from the third | caught | `a_second_exif_segment_is_an_extra_exif_issue_and_still_a_block` |
| 31 | IPTC `at` ignores the position | caught | `a_truncated_8bim_resource_is_region_too_short_and_stops` |
| 32 | ICC odd-one-out names the first chunk | caught | `icc_chunk_mismatches_are_one_issue` |

**Fuzz-invariant plants:** through the shared harness (`tests/fuzz_layout.rs`): `is_clean` false whenever a region exists (cleanliness iff, the too-strict direction: red); ICC aggregate always emitted (IccChunkMismatch iff, the too-eager direction: red); container parses `tiff` minus its last byte (container independence: red); plus mutants 17 and 21 above, red only through the harness. Both directions of each iff are therefore shown red: cleanliness (f vs the region plant), ICC aggregate (d and 3 vs the always-emit plant), the extended-XMP 75-byte iff (g 39 vs g 41), `ExtraExif` (b vs 30). **Against the real libFuzzer target:** ICC `seq > count` guard removed: crash `IccChunkMismatch iff the chunks disagree` (`jpeg_metadata_invariants.rs:305`) on an input whose first chunk has seq 0x2B of count 2; Exif signature compared on 5 bytes: crash `kind` (`jpeg_segments.rs:149`). Both were found while loading the corpus accumulated by my earlier runs (within seconds). ICC `seq == 0` accepted was **not** found in a 60 s run (129,308 runs); see I1. Artifacts removed, plants reverted, tree clean.

**Findings (none blocking):**
- **L1 (Low, test):** the third case of `the_jpeg_layer_issue_total_never_exceeds_the_documented_constant` is vacuous: 4096 short segments followed by SOS halt the walk at the 4097th segment, so the extraction records nothing and the assertion holds trivially. The largest reachable total (4096 short segments and no SOS, unhalted: 4096 + MissingSos + MissingEoi) is never built. The bound itself is right (see 8b).
- **L2 (Low, docs):** `AnyIssue::Exif.issue` offsets (and the `ExifBlock::container()` offsets generally) are relative to `tiff()`, not to the file. That is what makes container independence hold, but neither `AnyIssue` nor `ExifBlock` says so; an audit consumer mapping them to file offsets needs `segment.offset() + 4 + 6`. Also `Error::RegionTooShort` mixes frames: `at` is the segment's `FF` for the header kinds while `need`/`have` count from after the signature, so "counted from its start" is ambiguous there. Both per spec; one doc sentence each would do.
- **L3 (Low, SPEC-009 area):** no deterministic test has an APP1 payload `Exif\0` followed by a non-zero sixth byte, so a 5-byte signature compare survives `cargo test`; only the real fuzz target catches it. A one-line case in the classifier tests would pin it.
- **I1 (Info):** the real target did not reach an ICC chunk with `seq == 0` (otherwise consistent) in 60 s; it is caught deterministically by the unit test and the harness's per-guard input. An ICC seq-0 seed would make the smoke itself sensitive to it.
- **I2 (Info):** the ICC aggregate is appended last, so `jpeg_issues()` is not sorted by offset when the aggregate names an early chunk; this matches the spec and the harness ("the ICC aggregate is last").

### Cost self-report

- **Tokens (total):** null. The verify sub-agent has no `/cost` or usage readout in its own interface; the orchestrator transcribes the metered figure from the sub-agent result.
- **Estimated USD:** null (follows from the metered tokens)
- **Duration (minutes):** about 45
- **Source of the number:** harness report (to be transcribed by the orchestrator)

### Drift and new artifacts

- **New decisions emitted:**
  - none
- **Deviations from spec:**
  - none found. The builder's extra tests and the placement of the walker tests in `src/jpeg/mod.rs` are as its Handback states; the new names are re-exported from `exif_core::jpeg` (`src/lib.rs` already exposes the module), which meets "re-export the new names".
- **Follow-up work identified:**
  - An optional small patch, or fold into SPEC-011: L1 strengthen the bound test with the unhalted maximum; L2 two doc sentences (TIFF-relative offsets; `RegionTooShort` frames); L3 a classifier test for `Exif\0X`; I1 an ICC seq-0 seed.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Little. "cargo test on stable" needs `+stable` explicitly, because with `~/.cargo/bin` first the default is nightly; and the walker's halt behaviour is stated only in DEC-006 and the walker, so judging "nothing recorded after a halt" meant reading both.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Which offset frame each layer's issues use (file vs TIFF-relative). It follows from container independence but is not written in DEC-006 or the spec, and the STAGE-003 audit CLI will need it.

3. **If you did this task again, what would you do differently?**
   — Run the real-target plants against a fresh corpus directory, so "found within 60 s" measures the fuzzer rather than a corpus accumulated by earlier runs; and write mutants as compile-checked patches rather than string substitutions (two of mine were ill-formed or equivalent).
