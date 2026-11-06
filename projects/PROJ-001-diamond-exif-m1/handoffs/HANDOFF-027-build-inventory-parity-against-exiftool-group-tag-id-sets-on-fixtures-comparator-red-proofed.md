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
  id: HANDOFF-027
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-06
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-013

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
  tokens_total: 475405             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 3.14
  duration_minutes: 43
  branch: worktree-agent-a0ecfa0e072d2a8a1
  pr: null
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "tokens transcribed by the orchestrator from the sub-agent metered result (subagent_tokens 475,405; 2,603,306 ms wall-clock including idle time)"
  synced_at: 2026-11-06
---

# HANDOFF-027: Inventory parity against ExifTool: group + tag-id sets on fixtures, comparator red-proofed

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-013` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (model, divergence classes, outputs, named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-013-inventory-parity-against-exiftool-group-tag-id-sets-on-fixtures-comparator-red-proofed.md`. Its measured table is a **prediction to re-measure**; where your measurement differs, yours wins and the Handback says so.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Decisions: DEC-001, DEC-005, DEC-006, DEC-007, DEC-008 (read; do not edit)
- Code: `src/audit/{mod,category,sniff}.rs`, `src/tiff/mod.rs`, `tests/support/{tools,oracle,oracle_tiff,oracle_jpeg,audit_tags,probe,mutate,hostile}.rs`, `tests/{audit_oracle,audit_model,budget_oracle,jpeg_oracle,mutation_oracle,fuzz_layout}.rs`, `examples/oracle_record.rs`, `fuzz/shared/audit_invariants.rs`, `.github/workflows/ci.yml`
- Prior art for the carried items: `projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-024-*.md` (F1 to F6, mutants M20 to M22) and `HANDOFF-026-*.md` (mutants m04, m07, m08); for the apparatus: HANDOFF-021/022 (SPEC-011)
- The ExifTool command for inventory is `exiftool -a -u -G1 -H -s <file>`; ExifTool is run, never read.

Constraints: `panic-free-library`, `forbid-unsafe-code`, `oracle-must-be-shown-red`, `exiftool-never-linked`, `test-before-implementation`, `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs 1 to 7, every named test, every Acceptance Criterion. **`src/` does not change**; a bug the oracle finds in `src/` is reported in the Handback with the measured output and the affected test stopped (process rule 3).
- Order: the carried tests and their by-hand mutant red-proofs first (M20/M21/M22, m04/m07/m08: control, apply the mutant by hand, show the named failure with its assertion text, revert, `git diff` empty), then the inventory model with its parser tests, then probe each new shape alone, then records, sweep and `KNOWN_DIVERGENCES`, live and replay tests, red-proofs, probe inventory mode with mutants i1 to i6, fuzz harness linearity (F5) and the 60 s `audit` smoke, CI path.
- `KNOWN_DIVERGENCES` entries are **pinned multiset differences**, never "some difference". Add the `guidance/questions.yaml` entries the spec lists.
- The existing 125 records, the seed hash lists and all existing tests stay unchanged except the ones the spec names (the `audit-` prefix exemption in `tests/jpeg_oracle.rs`; seed or record counts only if you add seeds, which you should not). Any other change to an existing test, fixture or record is a **finding to report**.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), and the extended CI path for the new records directory.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-027); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Any `src/` behaviour change; the DEC-008 amendment candidates (stay a question); resolving the depth, SubIFD or budget differences from ExifTool
- Value, XMP/ICC/IPTC/JUMBF content comparison; PNG/WebP/HEIC; the CLI; SPEC-014/015/016
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

- **Branch / PR:** branch `worktree-agent-a0ecfa0e072d2a8a1` (fast-forwarded to `claude/rust-exif-metadata-6351d5` at `97f11c6` first; the `--ff-only` merge was accepted). Not pushed, no PR, no cycle advanced. The commit hash is in the report that carried this handoff back.
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes, with the deviations below. `src/` is unchanged (`git diff --stat -- src` is empty); every named test exists under its name; measured, not predicted, numbers are used throughout.
- **For `verify`:** n/a (build).

**What was measured (ExifTool 13.55, `exiftool -a -u -G1 -H -s`, each call alone under the 5 s tool limit).** 247 inputs, 178 distinct by content (the design's 171 plus the 7 new shapes), 177 run (`two-hop-cycle` is recorded `# outcome: not run`). 146 of the 177 agree exactly; 31 are pinned as exact multiset differences (`KNOWN_DIVERGENCES`, 32 rows with Class E). No stderr line on any input. Groups seen: `IFD0 IFD1 IFD2 IFD3 ExifIFD GPS InteropIFD SubIFD SubIFD1 SubIFD2 ICC-header ICC_Profile IPTC JUMBF XMP-xmp JFIF Photoshop`; the unmodelled ones are exactly `JFIF` and `Photoshop`, no other group appeared.

| Class | Rows | Pinned difference (ours / theirs) | Hides? |
|---|---|---|---|
| A unreadable value | 5 | `ascii-count-past-eof` ours IFD0 010f 0110 0131 0132 013b; `count-overflow`, `-be` ours IFD0 02bd 02be 02bf 02c1; `payload-past-eof`, `-be` ours IFD0 02bd | no |
| B revisited IFD | 2 | `exif-points-at-ifd0` theirs ExifIFD 0100 x1; `self-cycle` theirs IFD1 0100 x1 (file not clean: CyclicIfd) | yes, asserted not clean |
| C depth limit | 1 | `deep-nesting` theirs SubIFD 0100 x3 (12 vs 8 SubIFD lines; IfdDepthExceeded, not clean) | yes, asserted not clean |
| D caps/budget | 4 | `entry-budget-exact` ours SubIFD* 0002 x1, ffff x3; `entry-budget-over` ours 0003 x1, ffff x2; `overlapping-subifds` ours 00ff x255; `audit-many-overlapping-ifds` ours 02ff x767 | no |
| E | 1 | `two-hop-cycle`, not run | n/a |
| F JPEG shapes | 6 | `jpeg-exif-after-scan`, `jpeg-exif-last-segment-at-eof` ours 14 entries (ExifIFD 829d 8827 9003 9204, IFD0 0100 0101 010f 0110 0112 011a 0131 0132 013b 8298); `jpeg-exif-before-and-after-scan` ours IFD0 010f x1; seed `exif-after-scan` ours ExifIFD 9003, GPS 0001, IFD0 010f; `jpeg-seventeen-exif-last-differs` theirs IFD0 010f x1; seed `seventeen-exif` theirs ExifIFD 9003, GPS 0001, IFD0 010f | the two 17-block rows: yes, asserted not clean (16 ExtraExif) |
| R undecodable region (new) | 13 | regions ours-only: `jpeg-icc-stub` Icc, `jpeg-jumbf-stub` Jumbf, seeds `exif-and-xmp` Xmp, `extended-xmp-pair` Xmp, `extended-xmp-short` Xmp, `icc-chunks-mismatch` Icc, `icc-chunks-ok` Icc, `icc-iptc-jumbf` Icc+Jumbf, `icc-seq-zero` Icc, `iptc-8bim` Iptc, `iptc-truncated-resource` Iptc, `jumbf-header` Jumbf, `xmp-and-extended-xmp` Xmp | no |

**Red-proofs.** Controls first everywhere. Perturbation tests (ours side, their side, byte mutation) name the exact `(group, id)`. Replay and live went red when `IFD0 0x0112 Orientation` was deleted from the committed `fixtures.txt` section of `exif-basic-le` (replay: `fixtures/exif-basic-le: UNEXPLAINED difference ... ours only: IFD0 0x0112 x1`; live: `the live listing differs from the committed record`); reverted. Probe mutants (`inventory_mutant_*`, before/after): i1 control clean, mutant `theirs only: InteropIFD 0x0100 x1`; i2 `ours only IFD0 0x000f` vs `theirs only IFD0 0x010f` (and 13 more masked ids); i3 `ours only ExifIFD 0001..0004` vs `theirs only GPS 0001..0004`; i4 pinned divergence disappears on `jpeg-exif-after-scan` and `jpeg-exif-before-and-after-scan` (`StoppedDiverging`); i5 `ours only IFD2 0110, IFD3 0131, IFD4 013b` vs `theirs only IFD1 0110, IFD2 0131, IFD3 013b`; i6 `ours only region Xmp`, `theirs only region Iptc`, XMP fixture unaffected. The hides-invariant test refuses a patched-clean `Health` with `HidesButClean`.

**Carried mutants (control = the new test passing on the unmutated tree; mutant applied to `src/` by hand; reverted; `git diff -- src` empty after each).**

| Mutant | Result with the mutant |
|---|---|
| M20 `exif_issues = container.issues().len()` | `health_exif_issues_is_the_sum_over_all_blocks` fails: `bad then good: summed over blocks, left: 0, right: 1` |
| M21 region `after_scan: false` | `a_region_after_the_scan_has_after_scan_origin_on_item_and_finding` fails: `after: item, left: false, right: true` |
| M22 SHORT/LONG gate removed | `thumbnail_pointer_needs_short_or_long_types` fails: `BYTE, left: Some((0, 8)), right: None` |
| m04 a `Truncated` IFD's declared count charged in `follow` (the verify's wording; 180 lib tests still pass) | `a_truncated_ifd_spends_no_entry_budget` fails: the walk records `[Truncated, TooManyEntries]` instead of one issue. A second variant (budget check moved before the table-bounds check) is caught by that test and by 3 lib tests |
| m07 `read_ifd(offset, usize::MAX)` for `Link::Next` | `entry_budget_counts_the_top_level_chain` fails: `chain of 3 x 1, left: [], right: [(TooManyEntries { limit: 2 }, Next, Some(16))]` (and the existing `max_issues_bound...` lib test) |
| m08 only `IfdKind::Sub` entries counted | `entry_budget_counts_the_top_level_chain` and `entry_budget_counts_exif_gps_and_interop_links` fail (`gps c=4, left: [], right: [(TooManyEntries { limit: 8 }, Tag(34853), Some(34))]`). Note: IFD0 is counted by a separate path, so the Exif-link case alone does not catch m08; the GPS and Interop cases do |

F1 tightened (a stray `audit-zzz.txt` record turns `tiff_records_are_byte_identical_to_the_committed_set` red: 46 vs 45; removed). F5: the guards now count their work (`thread_local` counter); `audit_harness_guards_are_linear_in_the_input` was red on the old code (thumbnails 300,184,952 steps vs 200,084 allowed; regions 4,510,500 vs 126,032) and is green after (80,043 and 12,000 steps; half-size inputs take half). `just fuzz-smoke` 60 s: `audit` 312,554 runs, cov 1738, ft 6643, corpus 736, about 5,123 exec/s (SPEC-017 baseline: 275,097 runs, cov 1723); `tiff_ifd` 1,823,965 runs, cov 1146, ft 5029; `jpeg_segments` 138,445 runs, cov 1494, ft 6396; all clean.

**Gates run locally.** `cargo fmt --all -- --check` (stable and 1.90.0), fuzz manifest fmt, `rustfmt --check` on the three `fuzz/shared` files; `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof` all exit 0; `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on Homebrew stable 1.99 and on `~/.cargo/bin/cargo +1.90.0` (lib 180, audit_model 5, audit_oracle 17, budget_oracle 22, crate_policy 4, fuzz_layout 24, inventory_oracle 55, jpeg_oracle 42, metadata_oracle 30, mutation_oracle 58), no SKIP; `-F unsafe_code` on both; `just deny`, `deny-fuzz`, `validate`, `decisions-audit` (0 structural errors, the same 10 scope warnings), `cost-audit`; `just oracle-record` then `git status --porcelain` over both record directories empty; `cargo run --example fuzz_seeds` leaves `fuzz/seeds` unchanged.

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 475,405 (transcribed by the orchestrator from the metered sub-agent result)
- **Estimated USD:** 3.14 (475,405 x 6.60 USD per Mtok)
- **Duration (minutes):** 43 (2,603,306 ms, includes idle time)
- **Source of the number:** harness `subagent_tokens`, read by the orchestrator

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - **Class R is new.** The design table did not predict region differences on the seeds: 13 distinct inputs have a located region that ExifTool lists no group for (2 hand-built stubs, 11 minimal seed payloads). They are pinned (`Class::R`, `regions_ours_only`, right side ours) with a new question `exiftool-lists-no-group-for-undecodable-regions`. The design's 31 differing inputs are 18 here (A 5, B 2, C 1, D 4, F 6) plus `two-hop-cycle`; R adds 13.
  - **ICC valid fixture:** ExifTool prints 16 `ICC-header` lines (ids 0x04 to 0x54), not 15. I used a `desc` tag size of 17 (the real data length) where the spec says 13.
  - **Extra files and fields:** `tests/support/inventory_corpus.rs` (corpora, names, dedup, shared by the recorder and the tests; the spec names only `inventory.rs`); `Divergence.measured` and `Class::R`; `inventory::probe_text`/`parse_probe_text` and `probe::run_probe_inventory`; `oracle_tiff::{byte, ptr_array}` and `Val::Ifds`; separate `inventory_fixtures()` functions in `oracle_tiff.rs` and `oracle_jpeg.rs` instead of extending `fixtures()` (so the 12 TIFF and 42 JPEG fixtures, their 125 records and their counts do not move). Two stub fixtures beyond the named list: `jpeg-icc-stub`, `jpeg-jumbf-stub`. Two extra tests: `tiff_region_lines_come_only_from_directory_valued_ids` (Class H: 1 TIFF input carries region-group lines and has one of the four ids) and `every_divergence_names_an_existing_question`.
  - **Fixture keys** in `KNOWN_DIVERGENCES.fixture` are `<corpus>/<name>` of the first input with the bytes (for example `seed-tiff_ifd/self-cycle`, `fixtures/jpeg-exif-after-scan`), not bare names.
  - **Carried tests live in `tests/budget_oracle.rs`** (not `src/tiff`, which would change `src/`), built from hand-laid overlapping images (an exact-fit control beside the crossing case for each link). `tests/audit_model.rs` holds M20 to M22 (`oracle_tiff::byte` added for the BYTE case).
  - `ours_hides` is derived from the sets in the table's constructor (`!theirs_only.is_empty()`), and a test checks the flag against the sets.
  - The mutated-copy probe (`PROBE_MAIN`, `stage_probe_crate`) now also copies `inventory.rs` and has a `--inventory` mode; existing probe behaviour and its 51 tests are unchanged.
  - Mutant i5 is placed in `chain_successor` (`src/tiff/mod.rs`): the chain position lives in the test model, so the library fault that moves it is the IFD kind of the second IFD.
  - Repo docs touched beyond the spec's list: `guidance/toolchain-brief.md` (one paragraph), `app.just` (a comment), `CHANGELOG.md` (one entry).
- **Library bugs found by the oracle:** none. Every difference is a documented limit or a choice, not an invented, mis-grouped or mis-numbered entry; no test was stopped.
- **Follow-up work identified:**
  - Decide Class R (and Class A) for the audit CLI: whether a located region ExifTool cannot decode, or an entry whose value is unreadable, should be flagged differently from a decoded one (SPEC-015 rendering).
  - The two DEC-008 amendment candidates (F6) remain in `audit-table-exiftool-names`.
  - A Class C/D resolution (depth limit, SubIFD cap, entry budget vs ExifTool) stays open (`audit-depth-limit-and-ifd-caps-differ-from-exiftool`).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The design table was a prediction that did not cover the region presence on the seeds, so the first sweep showed 13 unexpected rows; the spec's "stub forms" sentence suggested only two. Also the ICC fixture sizes (13 vs 17) and the "15 lines" figure. Where `Divergence` lives versus where the corpora enumerator lives (the probe crate copies `inventory.rs`, so it cannot import the fixtures) had to be decided.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The hook environment refuses compound shell commands (heredoc plus a following command, loops with globs); file edits had to go through the editor tools and scratch files. Not a repo constraint, but worth a line in the toolchain brief for cold agents.

3. **If you did this task again, what would you do differently?**
   — Run the full sweep before writing the test file (I did, and it paid off), and write the hand-laid budget images with a small layout assertion helper from the start; the exact-fit arithmetic (budget = `len / 12`, overlap by one slot) was the slowest part.
