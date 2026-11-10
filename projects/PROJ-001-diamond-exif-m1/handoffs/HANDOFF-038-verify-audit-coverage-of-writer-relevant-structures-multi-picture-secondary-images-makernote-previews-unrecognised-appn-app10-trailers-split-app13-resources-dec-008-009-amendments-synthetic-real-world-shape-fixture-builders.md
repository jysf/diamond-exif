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
  id: HANDOFF-038
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-09
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-019

project:
  id: PROJ-001
  stage: STAGE-004
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
  status: completed                # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: 95
  branch: worktree-agent-ac9dd68660a4c9cb4
  pr: null
  completed_at: 2026-11-09         # YYYY-MM-DD
  notes: "verify verdict APPROVED at 86ca32e (code at 75d8fdc); tokens null: the verifier cannot see its meter, the orchestrator transcribes it"
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-038: Verify audit coverage of writer-relevant structures

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-019` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-019-*.md` (normative rules 1 to 9, the fixtures, the ~52 named tests and the 28 mutants are the contract)
- **DEC-012 (normative, with its 2026-11-09 amendment that rules on the build's four doubts), DEC-006, DEC-008, DEC-009, DEC-010 with their amendments:** `decisions/`
- **Build handoff and Handback:** `HANDOFF-037-build-...md` (claims to verify, not to trust; read the seven deviations)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; python reading ExifTool output needs `errors="replace"`)
- Code under review: `git diff 8d755b9..HEAD` (build `ccbae57..05fb303`, then `75d8fdc` = cost record, rustfmt of two fuzz helper files, DEC-012 amendment). Main: `src/jpeg/metadata.rs`, `src/audit/{mod,category}.rs`, `src/policy.rs`, `src/report/{json,text,mod}.rs`, `tests/support/real_shapes.rs`, `tests/real_shapes.rs`, `fuzz/shared/{jpeg_metadata,audit,render}_invariants.rs`, `examples/{fuzz_seeds,oracle_record}.rs`, `tests/oracle-fixtures/exiftool-inventory/real-shapes*.txt`, goldens, seeds `shape-*`.
- Real-file evidence: SPIKE-001 (archived under `spikes/done/`). Public samples may still be in the scratchpad of the orchestrator (`.../scratchpad/public/{A,B,C}`); the user's local photos are in `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0`. **Privacy rule for local photos: read in place, copy nothing, record only counts, short hashes, category and region names; never tag values, file contents or paths. Run the binary without `--values`.**

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and `rustfmt --check --edition 2021 fuzz/shared/*.rs`, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`, `-F unsafe_code` on lib and bin, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`), `just fuzz-smoke jpeg_segments`, `audit` and `audit_render` 60 s each, `cargo build --release --locked --bin exif-audit`. Confirm `decisions/` is unchanged by the build commits (only `75d8fdc` amends DEC-012, by the architect), no dependency added, `scripts/` untouched.
- **Read the JPEG layer line by line** (`src/jpeg/metadata.rs`): the MPF locator (header `MPF\0`, TIFF header BE and LE, tag 0xB002, 16-byte entries, offsets relative to the byte after `MPF\0`, entry 0 primary, `FF D8` check, range arithmetic with overflow, the 16-image bound, the second MPF segment, the interaction with the tolerant TIFF engine's entry budget), COM, OtherApp and its benign list (APP0 `JFIF\0`, APP14 `Adobe\0` only; `JFXX`, `FPXR`, `AROT` are OtherApp), the trailer computation (trailer minus embedded ranges: overlapping or nested ranges, a range reaching past the trailer start, a range inside the scan data, adjacent ranges, zero-length remainder), and the Photoshop rule (every `Photoshop 3.0\0` APP13 segment is an Iptc region; the data is the payload after the header). Try to break it with hostile inputs the builder did not: overlapping MPF entries, an entry pointing at the MPF segment itself, an entry whose range covers the primary image, offsets near `u32::MAX`, a 0xB002 count that lies, MPF inside a thumbnail, an MPF whose image is itself a JPEG with an MPF (no recursion allowed), 100 MPF segments, an APP2 `MPF\0` of 5 bytes.
- **Audit and policy against DEC-012 as amended:** the 15 categories in `Category::ALL` order; `is_block`; severities (Medium); the preset grid cell by cell (`EmbeddedImage` and `OtherSegment` removed only by `Everything`, `KeptUnread` under the other four); `may_hold` (every category except itself; `MakerNote` may hold `EmbeddedImage`); `holds` is informational, one level, never a disposition input, never added to the outer inventory; region-to-category mapping (`Comment` to `FreeText`); the `StripSpec` 16-bit set still round-trips with 15 categories.
- **Report contract:** `schema_version` 2; `embedded` key after `thumbnail` with the pinned shape; 15 `by_category` keys; region and category enumerations; CSV unchanged at 19 columns; text prints `holds`; goldens regenerated deliberately (the builder's account of the golden diff: JSON schema 1 to 2, two zero categories, `"embedded":null` per finding, iptc region `len` 6 to 18). Check that diff yourself with `git diff 8d755b9 -- tests/golden`.
- **Oracle discipline.** ExifTool is run, never linked. The builder reports ExifTool counts the MPF primary as `MPImage1` so groups = regions + 1 (constant `MPIMAGE_PRIMARY_OFFSET = 1`), with damaged-file pairs pinned (3 groups/1 region, 17/15, 0/0). Re-measure with ExifTool on at least three public samples and the synthetic MPOs. Confirm the class S exclusion is exact (it removes only the new structures), the two inputs moved by the Photoshop rule are pinned as class R with a `KNOWN_DIVERGENCES` entry, and the oracle-must-be-shown-red proofs still go red (`lint-red-proof`). Judge the builder's deviation 1 (the 178-results claim) and deviations 2 to 7.
- **Local real-file re-run (counts only).** Build the release binary and run it over `_incoming0` and the public samples with `--format json --policy all-pii`; reproduce the builder's numbers (4 JPEGs with MPF and 2 images each, 8 `embedded_image` findings, `holds` of the Nikon-style extras empty and of the iPhone-style extras `[xmp]` and `[xmp, icc]`, two `AROT` files, one Leica trailer, split APP13 sample giving 2 iptc regions) and compare each with ExifTool. Report any file where we name a structure ExifTool does not or the reverse.
- **Test the testers.** Re-run the builder's 28 mutants (the Handback maps each to a test) for at least 1, 4, 7, 9, 12, 16, 19, 22, 25, 27 and 28, including the two the builder had to rewrite (22 and 27). Write mutants the builder did not try, for example: entry offsets taken relative to the file start instead of the MPF header; the `FF D8` check dropped; the benign list including `JFXX`; `Adobe` matched by prefix only; trailer computed without subtracting overlapping ranges twice-safely; `holds` computed with embedded location left on (recursion); `Comment` mapped to `OtherSegment`; the 16-image bound off by one; the Photoshop region data starting at the 8BIM instead of after the header; `KeptUnread` for `EmbeddedImage` under `Everything`; the `by_category` order wrong in text but right in JSON. Report which the suite catches (name the test or guard) and which survive.
- **Fuzz.** Redo one plant against the real libFuzzer `jpeg_segments` target and one against `audit`, from the committed `shape-*` seeds only. Check that every new class of input (BE and LE MPF, truncated, out-of-range, non-JPEG, inner, 17-image, split APP13, AROT, FPXR, COM, trailer, Canon-style absolute note, 64 KB Exif) has a seed in each of the three corpora, and that the old three corpora and their recorder counts (249 inputs, 180 distinct) are untouched.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; locating MakerNote previews; any writer; TIFF-side changes; changing DEC-012 (report doubts as questions)

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

### Execution notes

- **Branch / PR:** `worktree-agent-ac9dd68660a4c9cb4`, no PR (verify cycle). `git merge claude/rust-exif-metadata-6351d5` fast-forwarded to `86ca32e`.
- **Completed at:** 2026-11-09
- **All acceptance criteria met?** Yes. Every gate is green, the real-file numbers reproduce, and every mutant the brief named is caught. Three of my new mutants survive the deterministic suite (one is equivalent); the findings below are all Low or nit.
- **For `verify`:** ✅ **APPROVED at `86ca32e`** (the code under review is `75d8fdc`; `86ca32e` adds only the verify brief). The Low findings are follow-ups, not blockers.

### Gates (re-run cold, exit codes captured per gate, never through a pipe)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check`, fuzz graph fmt, `rustfmt --check --edition 2021 fuzz/shared/*.rs` | 0 / 0 / 0 |
| `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof` | all 0 |
| `cargo +1.90.0 test --no-fail-fast` (`DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`) | 0: 662 passed, 0 failed, 8 ignored (the documented `env_limit_child` and the size-measurement ones) |
| `cargo +stable test --no-fail-fast` (1.98.0), same env | 0: 662 passed, 0 failed, 8 ignored |
| `cargo rustc --lib -- -F unsafe_code`, `--bin exif-audit` | 0 / 0 |
| `just deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit` | all 0 |
| `cargo run --example oracle_record`, then `git status --porcelain` | 0, 0 lines |
| `cargo build --release --locked --bin exif-audit` | 0 |
| `just fuzz-smoke jpeg_segments` (60 s) | 0: 194,882 runs, cov 1853, ft 8097, corpus 722 |
| `just fuzz-smoke audit` (60 s) | 0: 65,631 runs, cov 2288, ft 9128, corpus 775 |
| `just fuzz-smoke audit_render` (60 s) | 0: 37,523 runs, cov 5162, ft 16493, corpus 1045 |

`decisions/` is touched only by `75d8fdc` (DEC-012 amendment, +9 lines). There are no changes to `scripts/`, `Cargo.toml`/`Cargo.lock` or the fuzz manifests. Under `fuzz/seeds` and `tests/oracle-fixtures`, the build only added files (`--name-status` shows no M or D), so no existing seed or record changed bytes.

### Code read line by line (`src/jpeg/metadata.rs`, audit, policy, report)

- **MPF locator.** It checks for APP2 and `MPF\0` (gated on `n == 2`). The base offset is `at + 4 + 4`, which is the byte after `MPF\0`. Parsing uses the tolerant `Container` (its entry budget applies) and reads 0xB002 via `payload()`. It takes whole 16-byte chunks, skips entry 0, and records `TooManyMpfImages` once at index 16 and then breaks. Every sum uses `checked_add`, every slice uses `get`, and nothing indexes directly. A second MPF segment is `OtherApp` plus `MpfMalformed`. Inside an embedded image (`read_with(.., false)`), MPF is a plain `OtherApp` with no issue. All of this matches DEC-012 as amended.
- **Benign list.** Only `JFIF\0` (classified `Jfif`) and APP14 `Adobe\0` are benign. `JFXX`, `FPXR`, `AROT`, `JFIF` without the NUL, `Adobe` without the NUL, `Adobe\0` on APP13, and an empty APPn are all `OtherApp` (measured).
- **Trailer.** It runs from EOI+2 to the end, minus the covered ranges. Each range is clamped, the list is sorted, and a running max is swept. With no EOI there is no trailer. I checked adjacent, overlapping, nested, inside-the-primary, partly covered (two pieces) and zero-length cases: all are correct.
- **Photoshop.** Every `Photoshop 3.0\0` APP13 segment is an `Iptc` region whose data is the payload after the 14-byte header. The `8BIM` walk only produces issues.
- **Audit and policy.** `Category::ALL` has 15 entries with the two new ones last, both Medium and `is_block`. Bits `0x2000`/`0x4000` are used and `all() == 0x7FFF`. In the preset grid, the two new categories are removed only by `Everything`. `may_hold` excludes each category from itself, and `MakerNote` gains `EmbeddedImage`. `holds` is a fold over the inner findings with location off: it is informational, one level deep, and never feeds a disposition or the outer inventory. `Comment` maps to `FreeText`; `OtherApp` and `Trailer` map to `OtherSegment`; trailer items are `after_scan`.
- **Report.** `schema_version` is 2. `embedded` comes after `thumbnail` and has the pinned shape. `by_category` has 15 keys in `ALL` order. CSV is unchanged. Text prints `holds`.
- **Golden diff checked by script.** I removed every `,"embedded":null` (15 of them, one per finding, each directly after `thumbnail`), the two zero `by_category` keys, schema 2→1 and iptc `len` 18→6. The result is byte-identical to the `8d755b9` JSON. Text and CSV change only in the iptc `len` (6→18).

### Hostile inputs (a scratch crate of mine, builders written independently)

These cases are all total, bounded and correct: overlapping entries (same range twice gives two regions and covers the trailer once), nested ranges, an entry at offset 0 (it lands on `II*`, so `MpfImageNotJpeg`), offset or size `u32::MAX` (`MpfImageOutsideInput`), past the end by one, size 0 (not FFD8), size 2 (exactly `FF D8`), and a 0xB002 count claiming 65,535 entries (`MpfMalformed` plus `OtherApp`). The count of 20 bytes is read as 1 whole entry, giving 0 regions and no issue, per amendment item 4. An inner image with its own MPF gives `holds = [other_segment]` with no recursion. 100 MPF segments: the first is read and 99 are `OtherApp` plus 99 `MpfMalformed`. An APP2 `MPF\0` of 4 or 5 bytes is `OtherApp` plus `MpfMalformed`. 20 entries at the same image give 15 regions and one `TooManyMpfImages`. An embedded range inside a COM payload before EOI is located (both regions are reported). For timing, 15 overlapping 208 KB images take about 12× one image, which is linear with a constant of 15.

### Oracle (ExifTool 13.55, run, never linked)

I re-measured `MPImage<n>` group counts on the 15 `shape-*` seeds (byte copies of the fixtures). two-be/le: 3 groups for 2 regions. image-not-jpeg and offset-out-of-range: 3 groups for 1 region. seventeen: 17 groups for 15 regions. inner-mpf: 2 groups for 1 region. truncated-index: 0 groups for 0 regions. ExifTool counts the primary, so `MPIMAGE_PRIMARY_OFFSET = 1` holds on well-formed files and the damaged pairs are pinned as the builder says.

On the Canon-style seeds ExifTool reads all three Canon fields intact; after the note is moved it reads `-`, `-` and the wrong string. The relocation hazard reproduces. The 64 KB seed's APP1 payload is 65,533 bytes.

Class S excludes only `Comment`, `OtherApp` and `Trailer`. `EmbeddedImage` is compared by count, and MPF/MPImage groups are classed `MultiPicture`. The two inputs moved by the Photoshop rule are pinned as class R rows plus one `oracle::KNOWN_DIVERGENCES` entry. `lint-red-proof` is green.

### Local real-file re-run (release binary, `--json --policy all-pii --inventory`, no `--values`; read in place, nothing copied; counts, short hashes and names only)

`_incoming0`: 21 files (13 JPEG, 5 TIFF-family, 3 unsupported). I reproduced every builder number:

- 4 MPF JPEGs with 2 `embedded_image` each (8 in total). On all 4, our `(offset, length)` pairs equal ExifTool's `MPImageStart`/`MPImageLength` for images 2..n.
- `holds` is `[]` and `[]` on the three Nikon-style files. ExifTool on the extracted images, piped in memory (`MPImage2:PreviewImage`, `MPImage3`), shows only ExifIFD `ExifImageWidth`/`ExifImageHeight`, so no category.
- `holds` is `[xmp]` and `[xmp, icc]` on the iPhone-style file, matching ExifTool's XMP and ICC groups on the extracted images.
- 2 files have APP10 `AROT`; ExifTool shows `AROT` on both.
- The Leica-style file has 1 trailer region. ExifTool does not name it (class S, as designed).
- Two Nikon MPOs leave 2 trailer pieces each: 3/130 and 478/219 bytes. These are the real gaps between primary EOI and image 2 and between image 2 and image 3, measured against ExifTool's ranges, so they are not a bug.
- One FPXR file gives 3 `other_app`; ExifTool -v1 shows 3 APP2 segments and a FlashPix group.
- In total, 10 `other_segment` findings across 6 files, all `kept_unread` under `all-pii`.
- COM: 0 found by us and 0 by ExifTool (`[File] Comment`).

Public A/B/C: 17 JPEGs. The split-APP13 sample gives 2 `iptc` regions with status `issues`, and ExifTool shows the IPTC group. Its one COM matches ExifTool's one. The APP14 segments on two samples are Adobe (benign; we report nothing). There is no MPF, AROT or trailer in this set.

**No file where we name a structure ExifTool does not, or the reverse**, apart from the class S kinds (trailer), which ExifTool has no counterpart for by design.

### Mutants (control green first; each applied by `perl`, the suites run, then `git checkout`, tree clean after every one)

These are the builder's mutants I re-ran, with the tests that went red (selection):

- **1** (offset from file start, which is also my "relative to file start"): `mpf_index_yields_one_embedded_image_per_secondary_entry_in_order`, `mpf_big_and_little_endian_headers`, `trailer_minus_embedded_images_is_the_remainder`, `mpimage_group_count_equals_embedded_image_regions_on_the_mpo_fixtures`, `jpeg_metadata_harness_holds_on_every_seed_and_degenerate_inputs` and about 25 more.
- **4**: `mpf_big_and_little_endian_headers`, `the_mpo_fixture_audits_with_holds_matching_its_inner_images`, the jpeg harness.
- **7**: `mpf_seventeenth_image_is_an_issue`, the jpeg harness, `new_seeds_are_committed_and_have_the_shapes_they_claim`.
- **9** (which is also my "Comment mapped to `OtherSegment`"): `new_region_kinds_map_to_the_dec_012_categories`, `comment_findings_follow_free_text`, `a_comment_is_a_free_text_finding`.
- **12**: `jfxx_fpxr_and_arot_are_other_app`, the jpeg harness.
- **16**: `holds_is_the_inner_audit_with_embedded_location_off`, `embedded_image_finding_carries_range_ffd8_flag_and_holds`, `an_inner_mpf_does_not_recurse`, the audit harness.
- **19**: `makernote_may_hold_embedded_image`, `may_hold_equals_the_dec_012_table`.
- **22**: `category_all_has_fifteen_entries_and_the_two_new_names_are_pinned`, `preview_is_deterministic` and others.
- **25**: `by_category_has_fifteen_keys_in_order`, the golden tests, `each_render_guard_goes_red_alone_when_planted`.
- **27**: my first version (adding `Iptc` to `CLASS_S_KINDS`) does not compile against the hand-typed `[_; 3]` pin. Rewritten to compile (Iptc skipped in `ours()`), it is caught by `class_s_region_kinds_are_excluded_exactly_and_the_existing_pins_are_unchanged`, `replay_every_corpus_is_clean_except_pinned_divergences`, `new_fixture_records_replay_and_reproduce` and the live parity tests.
- **28** (which is also my "`holds` with embedded location on"): `an_inner_mpf_does_not_recurse`, the audit harness.

These are my new mutants and what caught them:

| Mutant | Caught by |
|---|---|
| FFD8 check dropped | `mpf_entry_not_starting_ffd8_is_an_issue_and_no_region`, `hostile_mpf_is_bounded_and_total`, harness |
| `JFXX` benign | `jfxx_fpxr_and_arot_are_other_app` |
| `Adobe` without the NUL | `jfif_and_adobe_app14_are_benign` |
| Iptc data skips the first 12 bytes | golden tests, `a_truncated_8bim_resource_is_region_too_short...`, jpeg seed walk |
| `Everything` keeps `EmbeddedImage` (KeptUnread) | `custom_all_equals_everything`, `counts_sum_to_findings...`, preview tests |
| `by_category` reversed in text only | `golden_text_json_csv_match_byte_for_byte`, `golden_files_are_unchanged...` |
| bound minus one (15 images) | `mpf_seventeenth_image_is_an_issue`, `hostile_mpf...`, harness |
| second MPF segment also read | `unparsable_mpf_segment_is_other_app_plus_an_issue` |
| trailer from EOI not EOI+2 | many (`a_clean_jpeg_without_metadata_is_clean...` etc.) |
| `TooMany` without `break` | `mpf_seventeenth_image_is_an_issue` |
| inner read with location on | `an_inner_mpf_does_not_recurse`, audit harness |
| `holds` from the first inner finding only | `holds_is_the_inner_audit...`, audit harness |
| trailer not `after_scan` | `other_app_and_trailer_are_other_segment_findings_in_container_order`, harness |
| `MPIMAGE_PRIMARY_OFFSET = 0` | `comparator_is_red_when_...primary_miscounted`, `mpimage_group_count_...` |
| **trailer sweep `cursor = b` (not max)** | **SURVIVES the suite**; found by the real libFuzzer `jpeg_segments` from the shape seeds within 120 s (harness `regions`) |
| **`MPF\0` accepted on any APPn** | **SURVIVES** (no test or seed has `MPF\0` outside APP2) |
| `starts_with_ffd8: true` constant | survives; **equivalent** (a located image always starts FFD8) |

### Fuzz plants (real libFuzzer, committed `shape-*` seeds copied to a scratch dir, nothing else)

- `jpeg_segments` with mutant 1 crashes at `jpeg_metadata_invariants.rs:505` with "extraction issues" (`MpfImageNotJpeg`).
- `audit` with mutant 28 crashes at `audit_invariants.rs:1460` with guard `findings` on an `EmbeddedImage` finding.
- `jpeg_segments` with the trailer-cursor survivor crashes at `:527` ("regions") after about 147k runs.

All three are reverted and the tree is clean. Every class has a seed in both `jpeg_segments` and `audit`: BE/LE MPF, truncated, out-of-range, non-JPEG, inner, 17-image, split APP13, AROT, FPXR, COM, trailer, Canon note (plus moved) and 64 KB; the 15 files are byte-identical across the two. `audit_render` has 7 seeds (MPO, split APP13, trailer), which is what the spec required. The old corpora stay at 249 inputs / 180 distinct (the `shape-` prefix is skipped; replay is green).

### Findings

- **High:** none. **Medium:** none.
- **Low 1, test gap: nested trailer ranges.** The trailer subtraction is correct today, but nothing in the deterministic suite pins it for nested ranges; only fuzzing finds the regression. To reproduce: in `uncovered()` change `cursor = cursor.max(b)` to `cursor = b`, and the suite stays green. On an MPO whose entry 1 spans images A+B+C and entry 2 is B alone, the mutant reports C's bytes as a `Trailer` (my repro: `Trl(148,25)` on a 173-byte file; unmutated, there is no trailer). Fix: add a unit test with an inner range ending before the outer one, and a seed.
- **Low 2, test gap: MPF marker.** Nothing pins that only APP2 carries MPF. To reproduce: drop `n == 2 &&` from the MPF branch, and the suite stays green. Fix: add a test and a seed with `MPF\0` on APP3, expecting `OtherApp` and no issue. The harness's `is_mpf` already checks 0xE2, so the seed would also arm the fuzz guard.
- **Low 3, unreported deviation.** The named test `golden_files_match_after_deliberate_regeneration` does not exist and is not in the builder's deviation list. It is covered in substance by `golden_text_json_csv_match_byte_for_byte` and `golden_files_are_unchanged_and_the_builder_reproduces_them`, which caught two of my mutants.
- **nit 1.** The jpeg harness imports `MAX_MPF_IMAGES` from the code under test, although the spec says to type the constants from the DEC by hand. A changed constant is still caught by `mpf_seventeenth_image_is_an_issue`.
- **nit 2 (for the writer stage, not a defect).** `holds` is `[]` both for an inner image with no metadata and for one that does not parse; for example, a 2-byte `FF D8` entry is a located image with `holds []`. An embedded range may also overlap other regions of the primary (an FFD8 inside a COM payload is both a `Comment` and an `EmbeddedImage`). DEC-012 allows both; SPEC-020..022 should expect them.

### Judgment of the builder's deviations

1. **The "178 results unchanged" claim is accurate as qualified.** The only moved results are the two the DEC-012 amendment rules on (pinned as class R, 34 rows), and the old corpora's records are byte-identical. Accepted.
2. **16 images including the primary (15 regions).** Ratified by amendment item 1, implemented exactly, and pinned by the bound mutants both ways. Accepted.
3. **No trailer-piece cap.** Ratified by amendment item 2 and correct. Accepted.
4. **Only the first MPF is read.** Ratified by amendment item 4 and caught by the second-MPF mutant. Accepted.
5. **0xB001 ignored; count is whole 0xB002 entries.** Ratified by amendment item 4. Accepted.
6. **`embedded: Option<EmbeddedSummary>` with `holds: StripSpec`.** Consistent with DEC-012 §3 and §7. Accepted.
7. **Full-house seeds unchanged, tests narrowed.** Correct, since no existing seed may change bytes; the new categories are covered by the shape seeds and `real_shape_fixtures_match_dec_009_as_amended_for_every_preset`. Accepted.

The remaining builder notes are also accepted. The two unnamed tests are covered by existing tests. The seed-identity test plus the records test cover what was asked. The Canon `-T -n -s3` value records are a reasonable record form.

### Cost self-report

- **Tokens (total):** null (the verifier cannot see its own meter; the orchestrator transcribes the metered figure)
- **Estimated USD:** null
- **Duration (minutes):** about 95 (wall clock, most of it the mutant campaign)
- **Source of the number:** none available to the agent; harness report via the orchestrator

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none by the verifier. In the builder's work, Low 3 is an unreported test-name deviation.
- **Follow-up work identified:** a small test-only patch for Low 1 and Low 2 (two unit tests and two seeds), optionally with the harness typing `MAX_MPF_IMAGES` by hand (nit 1).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?** The brief said "the three corpora" in two senses (fuzz targets versus inventory seed corpora), and the "seven deviations" against a nine-bullet list. Also, mutant 27's natural form does not compile against the hand-typed pin, so it needs a compiling variant.
2. **Was there a constraint or decision that should have been listed but wasn't?** One: worktree-isolated git commands have to be issued plainly, so compound gate and mutant loops had to live in script files.
3. **If you did this task again, what would you do differently?** I would write the mutant runner first and start it right after the gates, so the slow part runs while the code is being read. I would also include nested-range and wrong-marker cases in the first batch of hostile inputs, because those exposed the two survivors.
