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
  id: HANDOFF-023
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-012

project:
  id: PROJ-001
  stage: STAGE-003
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
  duration_minutes: null
  branch: worktree-agent-a46f2413d85fb30a2
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "Implementer subagent cannot see its own token count or wall clock: tokens_total, estimated_usd and duration_minutes are null for the orchestrator to transcribe from the metered figure. Not pushed; no PR (as instructed)."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-023: Audit model (sniff, inventory, PII categories and findings)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-012` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (API shape, rules, named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-012-audit-model-container-sniff-inventory-pii-categories-and-findings-pure-library-dec-for-the-category-table.md`
- **`decisions/DEC-008-audit-categories-and-severity.md`** (the category table and severities; normative; copy the table into the test as data, do not generate it from the code), DEC-005, DEC-006, DEC-007, DEC-002
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof` and `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code to build on: `src/tiff/mod.rs` (`Container`, `Ifd`, `Entry`, `IfdKind`), `src/jpeg/{mod,metadata}.rs`, `src/lib.rs`, `fuzz/fuzz_targets/jpeg_segments.rs`, `fuzz/shared/jpeg_metadata_invariants.rs`, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs`, `tests/support/{tools,oracle_tiff,oracle_jpeg}.rs`, `.github/workflows/ci.yml` (the `fuzz-smoke` matrix), `app.just`
- Prior art: HANDOFF-019/020 (SPEC-010: shared fuzz harness, iff-invariants, plants), HANDOFF-021/022 (SPEC-011: oracle records, CI-only scripts)

Constraints: `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red`, `no-new-top-level-deps-without-decision` (no dependencies of any kind). ExifTool is run, never read.

## Expected Deliverables

- Everything in the spec's Outputs with every named test and Acceptance Criterion, including the live DEC-008 table-name check with its committed record, the Container amplification measurement (report numbers: command, input size, peak memory, time), and the `audit` fuzz target with seeds, invariants (each iff both directions, plus the overlapping-guard variant), plants shown red (at least one against the real libFuzzer target) and a clean 60 s `just fuzz-smoke audit` with executions and coverage.
- **If ExifTool names a table id differently from DEC-008, or does not name it:** do NOT edit `decisions/` or the table. Record the measured command and output in the Handback and `guidance/questions.yaml`; I decide.
- **Run `just lint-fuzz-red-proof` and `just lint-red-proof` locally** (CI-only otherwise; the fuzz one broke PR 14 when a target started `include!`ing a new directory: if you add a shared include dir, confirm the script copies it). Run `just oracle-record` and confirm a clean `git status` afterwards; the existing 120 records and 66 seeds must stay byte-identical.
- CI: add `audit` to the `fuzz-smoke` matrix and nothing else in the workflow. Validate the YAML parses.
- Name every deviation (renamed, folded or missing tests; any derived number that does not match what the rules produce: stop and report rather than adjusting) in the Handback.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` so your worktree has the commit containing HANDOFF-023 (and `290441d` or later).
- Do not modify irradiance, `scripts/`, `decisions/`, or the TIFF/JPEG reader behaviour (a reader bug found is reported, not fixed). No dependencies or dev-dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Strip policies and previews (SPEC-014), renderers (SPEC-015), the binary (SPEC-016), inventory parity beyond the table-name check (SPEC-013)
- Reading inside thumbnails, MakerNote parsing, XMP/ICC/IPTC/C2PA decoding, PNG/WebP/HEIC
- Fixing a Container memory amplification if found (report it)
- Pushing, PRs, cycle advancement

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

- **Branch / PR:** branch `worktree-agent-a46f2413d85fb30a2` (local commit only; commit SHA is in the delivered report, a commit cannot name its own SHA); no PR, not pushed.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** Yes, with the deviations listed below. Two of them change what the spec literally says and need the architect's eye: the `Finding` shape (D1) and the weakened region-category invariant (D5). Three findings need a decision: the DEC-008 table disagrees with ExifTool on 12 rows, 3 of them real id mismatches (Q1).
- **For `verify`:** n/a (build cycle).

**What was built.** `src/audit/{mod,sniff,category}.rs` (the whole public API of the spec, plus `Category::ALL`, `Severity::name`, `region_category`); `fuzz/fuzz_targets/audit.rs` + `fuzz/shared/audit_invariants.rs` (independent re-derivation, 11 named guards); `fuzz/seeds/audit/` (73 seeds); `tests/audit_oracle.rs`, `tests/audit_model.rs`, `tests/support/audit_tags.rs`, the committed record `tests/oracle-fixtures/exiftool/audit-tag-names.txt`; additions to `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`, `examples/oracle_record.rs`; `audit` added to the CI `fuzz-smoke` matrix (that one line) and to the default list of `just fuzz-smoke`; `guidance/questions.yaml` +4 entries. No dependencies; `scripts/`, `decisions/`, irradiance and the TIFF/JPEG reader code are untouched.

**Gate results (all run in this worktree, 2026-11-05).**
- `just lint` (fmt both graphs checked via `cargo fmt --all` + `cargo fmt --manifest-path fuzz/Cargo.toml`; clippy `-D warnings`, Homebrew clippy 0.1.99): clean. `just lint-fuzz` (pinned stable clippy 0.1.98): clean. `just lint-no-allow`: clean. `just forbid-unsafe` (`-F unsafe_code`): clean.
- `just lint-red-proof`: PASS (control clean, all five lints fire at the injected code and still fire without `-D warnings`; proved by clippy 0.1.99). `just lint-fuzz-red-proof`: PASS (control clean; `unnecessary_cast` fires at `fuzz_targets/tiff_ifd.rs:186`; exit 0 without `-D warnings`). The fuzz one copies `fuzz/shared/`, which is where the new include lives, so no script change was needed.
- `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test --no-fail-fast`: Homebrew 1.99.1, `~/.cargo/bin/cargo +stable` (1.98.0) and `+1.90.0`: all green and identical (169 unit, 2 audit_model, 16 audit_oracle (+2 ignored), 4 crate_policy, 19 fuzz_layout, 42 jpeg_oracle, 30 metadata_oracle, 51 mutation_oracle).
- `just deny`, `just deny-fuzz`: licenses ok. `just validate`, `just decisions-audit` (0 structural errors; the 10 scope warnings are pre-existing overlaps), `just cost-audit`: ok.
- `just oracle-record`: rewrote 41 TIFF + 79 JPEG records plus the new audit record; `git status` shows no change to any existing record (the only new file under `tests/oracle-fixtures` is `audit-tag-names.txt`). The 66 old seeds are byte-identical (hash test `the_existing_66_seeds_are_byte_identical`; `git status fuzz/seeds` shows only the new `audit/` directory).
- CI YAML: `.github/workflows/ci.yml` parses (PyYAML `safe_load`); the diff is the one matrix line.
- `just fuzz-smoke audit` (60 s, from the 73 committed seeds, fresh corpus): clean, exit 0. `INFO: seed corpus: files: 72 min: 2b max: 65551b`, initial `cov: 1345 ft: 3311`, final `#6997 DONE cov: 1463 ft: 4734 corp: 324/1164Kb exec/s: 114 rss: 527Mb`, `Done 6997 runs in 61 second(s)`. A first run from the 72 seeds before `audit-ifd1-strips` was added: `#7645 DONE cov: 1453 ft: 4834 corp: 312/1083Kb exec/s: 125 rss: 501Mb`, 7,645 runs. The rate is low (about 115-125 exec/s) because `max_len` is 65,551 (the copied `app1-len-ffff` seed) and every execution runs `audit` twice, the full independent re-derivation and `jpeg::read` again for each thumbnail finding.

**Red-proofs (before/after).**
1. `sniff` stubbed to `Unknown` before implementation: `sniff_recognises_every_magic_and_never_overreads` ("jpeg: the full magic", left Unknown, right Jpeg) and `sniff_distinguishes_...` RED; implemented: GREEN. In the same run `categorize` carried a planted wrong row (`0x010E => None`): `categorize_matches_the_dec_008_table_row_for_row` ("ImageDescription (0x010E) in Ifd0, left None, right Some(FreeText)") and `categorize_assigns_nothing_outside_the_table` RED; plant removed: GREEN.
2. After the audit tests passed, three mutations of `src/audit/mod.rs`: item cap `<` to `<=` -> `inventory_cap_boundary_is_exact_and_findings_survive_truncation` and `the_findings_cap_is_separate_and_sets_the_same_flag` RED; `clean` ignoring `failed_blocks` -> `a_fatal_exif_block_counts_as_failed_and_the_audit_is_not_clean` RED. Reverted.
3. ExifTool name table (oracle): table name for `0x010F` changed `Make` -> `Maker`: `table_names_replay_matches_the_committed_record` RED ("0x010F (Make): table says "Maker", measured ["Make", "Make"]"), `live_table_names_match_exiftool` RED (same message from live ExifTool), `recorded_differences_are_exactly_these_rows` RED; reverted: GREEN. The comparator itself is shown red on 4 perturbed record rows, a perturbed table row and a missing id by `table_name_check_is_red_when_a_name_is_perturbed`.
4. Harness plants: `audit_harness_each_iff_invariant_goes_red_when_planted` runs 44 plants over the flattened answer; each is rejected by its named guard run ALONE (`run_guard`), the control (no plant) passes every guard, every one of the 11 guards has at least one plant, and each plant is asserted to change the answer.
5. Against the REAL libFuzzer target (two library plants, each reverted): (a) `clean` ignoring `failed_blocks`: `fuzz-smoke` crashed on seed load, `audit invariants violated: [("health", "health HealthF { clean: true, ..., failed_blocks: 1, ... }, want HealthF { clean: false, ... }")]`, exit 1. (b) IFD1-strips rule inverted (`has_jpeg_pointer` instead of `!has_jpeg_pointer`): the 60 s run on the then-72 seeds did NOT find it (exit 0), because no seed had an IFD1 with `0x0111`; added the seed `audit-ifd1-strips`, re-ran: crash on seed load, `audit invariants violated: [("findings", "findings: 1 entries, want 2"), ("max_severity", ...)]`, exit 1. Plant (b) is also covered by two `findings` plants in 4. Logs are in the session scratchpad only (not committed).

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** null. The implementer subagent has no `/cost` or `usage` access to its own run; the orchestrator transcribes the metered figure.
- **Estimated USD:** null (same reason).
- **Duration (minutes):** null (no wall clock available to the subagent; not estimated, to avoid an invented number).
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none (`decisions/` untouched). Proposed amendment for DEC-008, not made: see Q1.
- **Questions raised in `guidance/questions.yaml`:** `audit-table-exif3-tag-ids` (high), `audit-table-exiftool-names` (low), `audit-category-region-vs-tag-shape` (low), `tiff-container-memory-amplification` (medium).

**Table-name check against ExifTool 13.55 (measured, not from memory).** Method: one-entry little-endian TIFF per id (`tests/support/audit_tags.rs`), `exiftool -G1 -a -s -u <file>` plus `exiftool -v3 -G1 -a -s -u <file>` (the `-v3` listing is the only place a directory tag shows its own name; XMP, Photoshop and ICC print no value line under `-G1 -a -s -u`, and the first recording of the spec's command alone gave `-` for 0x02BC, 0x8649 and 0x8773). Record: `tests/oracle-fixtures/exiftool/audit-tag-names.txt`, 38 ids (37 table ids plus 0x0201 in an IFD1). 26 ids match DEC-008's name exactly. 12 differ; the DEC and the table were NOT edited, per the handoff:
- Real id mismatches (Q1, `audit-table-exif3-tag-ids`): `0xA437` DEC ImageEditor, ExifTool **Photographer**; `0xA438` DEC ImageEditingSoftware, ExifTool **ImageEditor**; `0xA439` DEC MetadataEditingSoftware, ExifTool **CameraFirmware**. One-off probe (not in the record): ExifTool names `0xA43A` RAWDevelopingSoftware, `0xA43B` ImageEditingSoftware, `0xA43C` MetadataEditingSoftware. So the DEC's Exif 3.0 ids look shifted: the two software tags belong at `0xA43B`/`0xA43C`. Proposed amendment (not made): Photographer `0xA437` -> Authorship, ImageEditor `0xA438` -> Authorship, ImageEditingSoftware `0xA43B` and MetadataEditingSoftware `0xA43C` -> Software, and decide `0xA439` CameraFirmware and `0xA43A` RAWDevelopingSoftware (both Software-like).
- Naming only (`audit-table-exiftool-names`): `0x0132` DateTime -> ModifyDate; `0x9004` DateTimeDigitized -> CreateDate; `0x927C` MakerNote -> MakerNoteUnknownText (content-dependent, measured with a text-like payload); `0xA430` CameraOwnerName -> OwnerName; `0xA431` BodySerialNumber -> SerialNumber; `0x02BC` XMP -> ApplicationNotes; `0x8649` Photoshop -> PhotoshopSettings; `0x8773` InterColorProfile -> ICC_Profile; `0x0201` JPEGInterchangeFormat -> ThumbnailOffset. `0x83BB` IPTC-NAA matched (via `-v3`).
- `tests/audit_oracle.rs::DIFFERING` pins exactly these 12 rows, so the list can neither grow nor rot silently.

**Container amplification (measured, existing behaviour; Q4 `tiff-container-memory-amplification`).** Command: `cargo test --release --test audit_oracle --no-run`, then `/usr/bin/time -l target/release/deps/audit_oracle-<hash> container_amplification_is_measured_and_reported --ignored --nocapture` on Apple silicon (the file is built by `hostile(511)` in `tests/audit_oracle.rs`: IFD0 with one `SubIFDs` entry of 511 offsets two bytes apart into 0xFF bytes, so each names 65,535 entries over overlapping bytes; the next-IFD words are zeroed so no IFD chains). **Input 789,516 bytes. `Container::parse`: 512 IFDs, 33,488,386 entries, 0 issues (a clean parse), 803,721,264 bytes of entries (24 bytes each), 1,018x the input, 102.7 ms. RSS 792,048 KB after the parse. Peak: `maximum resident set size` 949,796,864 bytes, `peak memory footprint` 949,404,920 bytes, for the parse plus a second parse inside `audit`; wall 0.61 s.** `audit` of the same file: 174 ms including its own parse; 65,536 items kept, `inventory_truncated = true`, 0 findings (all tag 0xFFFF); the audit's own additions are bounded by the two caps (at most 65,536 items and 65,536 findings; their byte size was not measured separately). A first, chained variant (next words left at 0xFF) gave 257 IFDs and 16.8M entries because each failed chain attempt spends one of the 512 IFD visits; the zeroed version is the true worst case. A bounded version (8 IFDs, 524,281 entries, 786,498 bytes) runs in the normal suite (debug: parse 97 ms, audit 94 ms). The audit does not copy entries and adds nothing proportional to them; it was not changed to work around this.

**Deviations from the spec text.**
- D1. `Finding` is `{ category, item: Option<usize>, at: Item, thumbnail: Option<ThumbnailRange> }`, not `{ category, item: usize, + range }`. Rule 5 requires that "a finding is never lost to truncation", but a finding for an entry past the inventory cap has no index in `items()`; so `item` is `None` exactly then (and `inventory_truncated` is set), `at` always carries what the finding is about, and `items()[i] == at` whenever `item == Some(i)`. `ThumbnailRange { range: Option<(u32, u32)>, starts_with_ffd8: bool }` is the `ThumbnailRange` of the spec, `Some` iff the category is `Thumbnail`.
- D2. `Item<'a>`, `Finding<'a>` and `FileAudit<'a>` keep the spec's lifetime but borrow nothing (a `PhantomData<&'a [u8]>`), so the signature `audit(&[u8]) -> FileAudit<'_>` is exactly the spec's and nothing is copied out of the input. `RegionKind` is a new public enum in `exif_core::audit` (the spec named it, did not define it); `Category::ALL`, `Severity::name` and `region_category` are additions; `Category::name()` returns snake_case (`device_id`, `maker_note`, ...), a choice for SPEC-015 to reuse or change.
- D3. `categorize(Interop, _)` is `None` for every id (DEC-008: the table is for "non-GPS, non-Interop IFDs"), and `categorize(Gps, _)` is `Some(Gps)` for every id. The IFD1-with-strips rule needs the whole IFD, so `categorize(Ifd1, 0x0111)` is `None` and `audit` applies the rule; the per-IFD context is computed once per IFD (a per-entry scan would be quadratic on a 65,535-entry IFD).
- D4. Test placement: `container_independence_audit_of_every_oracle_fixture_bare_and_in_jpeg` and `audit_never_panics_on_every_committed_seed` are in the new `tests/audit_model.rs` (they need `tests/support` and the committed seeds), not in `src/audit` unit tests; the names are unchanged. `every_error_variant_displays` guard: not extended, `Error` gained no variant. Added beyond the named list: `region_kinds_map_to_the_dec_008_categories`, `a_gps_ifd_yields_one_gps_finding_per_tag_and_ifd0_ids_none`, `an_ifd1_thumbnail_with_unusable_pointer_types_has_no_range`, `regions_are_items_and_findings_in_segment_order`, `a_clean_jpeg_without_metadata_is_clean_with_no_findings`, `the_findings_cap_is_separate_and_sets_the_same_flag`, `the_table_has_one_row_per_dec_008_id`, `recorded_differences_are_exactly_these_rows`, `name_parsers_read_exiftool_output_shapes`, `container_amplification_is_bounded_in_test`, `audit_target_is_declared_and_runs_in_ci`. `the_existing_63_seeds_are_byte_identical` (SPEC-011) is kept; `the_existing_66_seeds_are_byte_identical` is added next to it with all 66 hashes taken from the files at HEAD.
- D5. Fuzz invariant "Region categories only on Region items" cannot hold as written: DEC-008 itself puts TIFF tags 0x02BC, 0x83BB, 0x8649 and 0x8773 in Xmp, Iptc and Icc, and the harness found it on the first seed run (`finding 0: category Xmp on a tag item`). The `category_shape` guard asserts what is true in both directions: a Region item has exactly its region's DEC-008 category (Xmp and ExtendedXmp to Xmp), `Jumbf` appears on tag items never, `Gps` iff a GPS-IFD tag, a thumbnail range iff the Thumbnail category. Recorded as Q3 (`audit-category-region-vs-tag-shape`).
- D6. The ExifTool call for the name check also runs `-v3` (see the table-name section). The record carries both listings' lines (`group` is `v3` for the verbose ones), with a second `# command:` line; the live test asserts the live output equals the committed record line for line.
- D7. `tests/jpeg_oracle.rs::tiff_records_are_byte_identical_to_the_committed_set` counts "records that are not `jpeg-`" and expected 41; the new record `audit-tag-names.txt` is neither, so the filter now also excludes the `audit-` prefix (two lines and a comment). The 41 TIFF hashes and the 79 JPEG count are unchanged.
- D8. Seeds: the `audit` directory holds 73 files: the 29 + 37 older seeds copied as `tiff-<name>` / `jpeg-<name>` and 7 new, one more than the six in the spec: `audit-ifd1-strips` (an IFD1 with strip offsets and no JPEG pointer), added because a real-target plant of the strips rule survived 60 s without it (red-proof 5b). `audit-empty-marker` is a zero-byte file; libFuzzer reports `files: 72` of 73 (it skips an empty file and runs the empty input itself), no 1-byte substitute was needed. `audit-many-overlapping-ifds` is the REDUCED version, not the measured worst case: 255 SubIFDs x 767 entries, 10,764 bytes, 195,586 entries (1 + 255 x 767, asserted by `audit_seeds_are_committed_and_have_the_shapes_they_claim`); the full 790 KB / 950 MB file would raise libFuzzer's `max_len` for every input and cost about a second per execution under ASAN. `fuzz_seeds` is deterministic and the old 66 seeds regenerate identically.
- D9. `just fuzz-smoke` with no target now also runs `audit` (the recipe's default list and its comment); the CI change is the single matrix line, as instructed.
- D10. Health details the spec left open: a JPEG with a second Exif segment is not `clean` (the walker's `ExtraExif` issue counts in `jpeg_issues`, both blocks are still audited); a truncation makes `clean` false; `failed_blocks` is 1 for an unreadable bare TIFF. A JPEG whose walk fails outright maps to `failed_blocks = 1, clean = false`, but that branch is unreachable today (`jpeg::read` is `Err` only when the input does not start `FF D8`, which the sniff already ruled out); it is kept so `audit` stays total if the reader changes. `max_severity` is the maximum over every finding found, including any dropped by the findings cap.
- D11. The thumbnail pointer and length are read from the first `0x0201` / `0x0202` entry of the IFD1 (as `Ifd::entry` does), once per IFD; other entries with the same tag in that IFD get the same range.
- D12. Test-first was strict for `sniff` and `categorize` (red before green, red-proof 1); the `audit` unit tests were written in the same pass as the implementation and passed on the first run, so their bite is shown by mutation afterwards (red-proof 2) and by the harness plants and real-target plants (4, 5).

**Not done / out of scope, as instructed:** inventory parity beyond the name check, strip policies, renderers, the binary, reading inside thumbnails (`audit-thumbnail-recursion` stays open: a `Thumbnail` finding carries its range and the FF D8 flag only), MakerNote parsing, any Container fix.

- **Follow-up work identified:**
  - A TIFF spec to bound `Container` memory (Q4), before SPEC-016 parses untrusted files in bulk.
  - A DEC-008 amendment for the three/five Exif 3.0 ids (Q1) and a decision on the nine naming differences (Q2), before SPEC-014 builds presets from the table.
  - `guidance/toolchain-brief.md` still says "29 TIFF and 37 JPEG" seeds and lists two fuzz targets; it now has a third target (`audit`, 73 seeds). Not edited (outside the build brief).
  - The harness is slow (about 115 exec/s) mainly from flattening to `String`s; if the 24 h gate matters, flatten to enums or hash instead.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Three places where two normative statements conflicted: "a finding is never lost to truncation" against "every `Finding.item` is a valid index" (D1); "region categories only on Region items" against DEC-008's own TIFF rows for Xmp/Iptc/Icc (D5); and "`exiftool -G1 -a -s -u` records the name ExifTool gives the id", which prints no name at all for the directory tags 0x02BC, 0x8649 and 0x8773 (D6). Each cost one measured detour before the resolution was clear.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The existing `tiff_records_are_byte_identical_to_the_committed_set` test hard-codes "41 non-JPEG records", which silently forbids any new record file in `tests/oracle-fixtures/exiftool/`; the spec named a new record there without naming that test (D7). Also DEC-008 should have been checked against ExifTool when it was written: its Exif 3.0 ids (0xA437-0xA439) are off, and the spec's own live check is what found it.

3. **If you did this task again, what would you do differently?**
   — Write the `audit` unit tests and watch them go red against a stub before writing `audit` itself (I did that for `sniff`/`categorize` only), and add the IFD1-strips seed on the first pass: I only learned the seeds missed that rule because a real-target plant survived 60 s. I would also keep the harness cheaper (enums instead of `String` facts) to get the fuzzer's rate closer to the other targets.
