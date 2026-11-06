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
  id: HANDOFF-028
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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
  tokens_total: null               # the verifier cannot see its own count; the orchestrator transcribes the metered figure
  estimated_usd: null
  duration_minutes: null           # orchestrator transcribes the metered wall-clock
  branch: worktree-agent-a5f27d1307a5a6ee1
  pr: null
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "verify verdict APPROVED at 434edfc; tokens_total null because the sub-agent cannot read its own meter (orchestrator transcribes subagent_tokens)"
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-028: Verify inventory parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-013` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-013-inventory-parity-against-exiftool-group-tag-id-sets-on-fixtures-comparator-red-proofed.md` (model, classes, outputs, named tests normative)
- **Build handoff and Handback:** `HANDOFF-027-build-...md` (claims to verify, not to trust; read the deviations, especially the new Class R and the ICC `desc` size)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 97f11c6..HEAD` (build commits `dd03f0a`, `add150e`, `72f2a23`). Main: `tests/support/inventory.rs`, `tests/support/inventory_corpus.rs`, `tests/inventory_oracle.rs`, `tests/support/probe.rs`, `tests/mutation_oracle.rs`, `tests/audit_model.rs`, `tests/budget_oracle.rs`, `fuzz/shared/audit_invariants.rs`, `examples/oracle_record.rs`, `tests/oracle-fixtures/exiftool-inventory/*`

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain` over both record directories). Run `just fuzz-smoke audit`, `tiff_ifd`, `jpeg_segments` 60 s each; record executions and coverage. Confirm `git diff 97f11c6..HEAD -- src` is empty.
- **Re-measure independently** (do not reuse the builder's sweep): run `exiftool -a -u -G1 -H -s` yourself over the three seed directories and the fixtures (write a scratch dump of `audit()` in the scratchpad) and confirm the identical/divergent split, each pinned divergence's exact diff, and the claim that `two-hop-cycle` is the only non-return. Check that Class R's 13 inputs really are inputs where ExifTool prints no group for a located region, and judge whether Class R is a legitimate recorded divergence or a loosening (a pinned region-kind difference per input, or a rule?).
- **Is any pinned divergence a loosening?** For each `KNOWN_DIVERGENCES` entry check it pins the exact multiset difference, that changing one count or id in the table turns the test red, that a wrong-direction entry is rejected, and that the `ours_hides => !clean` invariant is not satisfiable vacuously. Look for a way an unexplained difference could pass: the `UNMODELLED_GROUPS` list, `DIRECTORY_VALUED` exclusions, dedup-by-bytes, the Class H TIFF region-line rule, `-` id dropping, the `SubIFD*` normalisation (is `SubIFD` numbering really pinned on `tiff-subifds-three`?), the Chain numbering.
- **Test the testers.** Write your own mutants the builder did not try, against `src/audit/mod.rs` and `src/tiff/mod.rs`, for the inventory path: wrong `after_scan` on items, `ifd_index` off by one, `field_type`/`count` swapped, Sub IFDs labelled Chain, IFD1 labelled Chain, Exif entries of block 1 attributed to block 0, region `segment` wrong, region kind Icc/Jumbf swapped, the inventory cap off by one, findings `item` index off by one, a pointer tag missing from the inventory. Report which the new suite catches (name the test) and which survive. Also re-run the builder's mutants i1 to i6 and carried M20/M21/M22/m04/m07/m08 yourself. Redo one fuzz plant of the F5 harness change against the real `audit` target.
- **Judge the carried tests** (F1 to F5, m04/m07/m08): do they pin the behaviour or only the mutant? The F5 step counters: could a guard be linear in the counter yet still quadratic in time?
- Check the 125 existing records and all seeds are byte-identical, the five new record files reproduce, and each record has command, version and `# stderr:` (or none) as specified; CI path covers the new directory; no dependencies; `scripts/`, `decisions/`, irradiance untouched.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-014/015/016 work, the DEC-008 amendment candidates, CI wiring beyond reading it

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

- **Branch / PR:** branch `worktree-agent-a5f27d1307a5a6ee1`. `git merge --ff-only claude/rust-exif-metadata-6351d5` was accepted, so the branch now holds build commits `dd03f0a`, `add150e`, `72f2a23` and `434edfc`. Not pushed, no PR, no cycle advanced. Only this handoff file is committed.
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes. Four Low findings below, none blocking.
- **For `verify`:** ✅ **APPROVED at `434edfc60a26fd1a782ed03d673938aac052d170`**

**Gates (re-run cold by the verifier; each exit status taken directly, never through a pipe).** All exited 0: `cargo fmt --all -- --check` on stable and on `+1.90.0`, the fuzz manifest fmt, `rustfmt --check` on `fuzz/shared/*.rs`, `just lint` (clippy 0.1.99), `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `forbid-unsafe`, `RUSTFLAGS=-F unsafe_code cargo check --all-targets` on both toolchains, `deny`, `deny-fuzz`, `validate`, `decisions-audit` (0 structural errors, 10 scope warnings, the same as before), `cost-audit`, `just oracle-record` (`git status --porcelain` over both record directories: empty; 247 sections, 178 distinct inputs, two-hop-cycle NOT RUN), and `cargo run --example fuzz_seeds` (seeds unchanged). `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` passed on Homebrew stable and on `~/.cargo/bin/cargo +1.90.0` with no SKIP: lib 180, audit_model 5, audit_oracle 17, budget_oracle 22, crate_policy 4, fuzz_layout 24, inventory_oracle 55, jpeg_oracle 42, metadata_oracle 30, mutation_oracle 58. The single ignored test in each binary is the existing `tools::tests::env_limit_child`. `git diff 97f11c6..HEAD -- src` is empty. `scripts/`, `decisions/`, `Cargo.toml`/`Cargo.lock` (both graphs), `fuzz/seeds` and `tests/oracle-fixtures/exiftool/` are untouched. CI's `oracle-records` porcelain path now includes `tests/oracle-fixtures/exiftool-inventory`. All 51 named tests exist under their spec names.

**Fuzz smoke, 60 s each, all clean (exit 0).** `audit`: 272,551 runs, cov 1739, ft 6677, corpus 728 (from a fresh corpus, INITED at 68 seeds). `tiff_ifd`: 1,314,679 runs, cov 1133, ft 4938. `jpeg_segments`: 138,615 runs, cov 1484, ft 6406.

**Independent re-measure.** I did not reuse the builder's sweep. A scratch crate depends on `exif-core` by path and dumps `audit()`. Its chain numbering is my own: ascending `ifd_index` per block, where the builder uses item order. A separate Python script runs `exiftool -a -u -G1 -H -s` (13.55) under `perl -e 'alarm 8; exec @ARGV'` and has its own parser. Results:
- 247 inputs, 178 distinct, 177 run. 146 are identical. 31 diverge. Each of the 31 diffs matches its `KNOWN_DIVERGENCES` row exactly, across classes A 5, B 2, C 1, D 4, F 6 and R 13.
- `two-hop-cycle` (and its copy `seed-audit/tiff-two-hop-cycle`) is the only input not run. ExifTool wrote nothing to stderr on any input.
- The only groups seen beyond the modelled ones are `JFIF` and `Photoshop`; no unknown group appeared.
- Region-group lines on a bare TIFF appear on exactly one input: `audit-tags/tag-83bb`, as IPTC.
- ExifTool never prints any of the eight `DIRECTORY_VALUED` ids as a tag line.
- The only EXIF-family `-` line is `IFD1 ThumbnailImage` (2 lines).
- ExifTool exits 1 on 4 inputs (`audit-empty-marker`, `soi-only`, `ifd0-offset-zero`, `truncated-header`). Its error goes to the file-level `[ExifTool]` group, and both sides are empty there.

**Class R.** It is legitimate: a pinned exact region set per input, not a rule. I inspected the bytes and ExifTool's output for `iptc-8bim`, `icc-iptc-jumbf`, `exif-and-xmp` and `icc-chunks-ok`. In each one the segment is present and located, and ExifTool prints no group for it, because the payload is a minimal or non-decodable stub (for example a 5-byte `01..05` 8BIM 0x0404 resource with no IPTC dataset). The well-formed twins (`jpeg-icc-valid`, `jpeg-jumbf-valid`, `jpeg-xmp`, `jpeg-iptc`) agree. `jpeg-iptc-without-0404` lists no region on either side. All 13 rows are `regions_ours_only` with `ours_hides: false`.

**Loosening checks.** `check_against` requires `pinned == actual` exactly. `ours_hides` is derived in `entries()` and checked by `hides_is_consistent`. 5 rows hide, so the invariant is not vacuous. Twin (byte-identical) listings are asserted equal, so dedup cannot mask an alias. The `SubIFD` numbering is pinned on `tiff-subifds-three`: groups are asserted exactly `IFD0, SubIFD, SubIFD1, SubIFD2`. Six table mutants each turned replay and live red, all reverted:
- a count changed (767 to 766);
- an id changed (`payload-past-eof` 0x02bd to 0x02bc);
- the wrong direction (`self-cycle` moved to `ours_only`);
- a region kind changed (`jumbf-header` Jumbf to Icc);
- a group renamed (`deep-nesting` `SubIFD*` to `IFD1`);
- a row's key broken (`seventeen-exif`).

Each gave `ChangedDiff` or `Unexplained`; the wrong-direction and broken-key cases also fail `ours_hides_implies_not_clean_for_every_divergence`.

**Mutants against `src/` (applied to the real tree, full `cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, restored; `git diff -- src fuzz/shared` empty afterwards).** Catching tests named by binary and test name.

| Mutant | Caught? | By (new SPEC-013 tests in bold) |
|---|---|---|
| v01 Exif items `after_scan: false` | yes | **class_f_jpeg_shapes_match_the_recorded_divergences**, lib `an_exif_block_after_the_scan_is_included_and_flagged`, fuzz_layout harness tests |
| v02 `ifd_index + 1` | yes | fuzz_layout `audit_harness_holds_on_every_seed...`, `..._each_iff_invariant...`, **audit_harness_guards_are_linear_in_the_input** (not the inventory oracle: a constant shift keeps chain positions) |
| v03 `field_type`/`count` swapped | yes | fuzz_layout harness tests only (the inventory model holds no types or counts, by design) |
| v04 Sub IFDs labelled Chain | yes | **replay/live parity, sub_ifd_names_are_pinned..., class_c, class_d, a_truncated_ifd_spends_no_entry_budget**, the inventory probe control, lib |
| v05 IFD1 labelled Chain (= i5) | yes | **inventory_groups_map_ifd_kinds_and_number_the_chain, replay/live parity, oracle_names_a_renumbered_chain_group, entry_budget_counts_the_top_level_chain, thumbnail_pointer_needs_short_or_long_types**, lib |
| v06 block 1 attributed to block 0 | yes | lib `two_exif_blocks_report_findings_from_both...`, fuzz_layout harness, audit_model seeds (not the inventory oracle: `ours()` uses the block only to number the chain) |
| v07 region `segment + 1` | yes | lib `regions_are_items_and_findings_in_segment_order`, fuzz_layout harness |
| v08 Icc/Jumbf swapped | yes | **regions_agree_on_xmp_iptc_icc_and_jumbf_fixtures, stub_icc_and_jumbf..., replay/live parity**, lib |
| v09 item cap `<=` | yes | lib `inventory_cap_boundary_is_exact...`, fuzz_layout |
| v09b findings cap `<=` | yes | lib `the_findings_cap_is_separate...` only |
| v10 finding `item + 1` | yes | **a_region_after_the_scan_has_after_scan_origin_on_item_and_finding**, lib, fuzz_layout, audit_model |
| v11 Exif pointer 0x8769 missing | yes | **directory_valued_ids_are_exactly_the_eight_and_each_has_a_fixture**, lib `audit_lists_pointer_tags...`, fuzz_layout |
| v12 region `after_scan: true` | yes | **a_region_after_the_scan_has_after_scan_origin_on_item_and_finding**, fuzz_layout |
| v13 GPS pointer 0x8825 missing | yes | same as v11 |
| i1, i2, i3, i4, i6 on the real tree | yes | each one's own `inventory_mutant_i*` probe test, plus **replay/live parity** and the probe control (i4 also `class_f`) |
| M20 | yes | **health_exif_issues_is_the_sum_over_all_blocks** (also `inventory_mutant_i4...`) |
| M21 | yes | **a_region_after_the_scan_has_after_scan_origin_on_item_and_finding** |
| M22 | yes | **thumbnail_pointer_needs_short_or_long_types** |
| m04 (Truncated `len/12` charged in `follow`) | yes | **a_truncated_ifd_spends_no_entry_budget** only |
| m07 (`usize::MAX` budget for `Link::Next`) | yes | **entry_budget_counts_the_top_level_chain**, lib `max_issues_bound...` |
| m08 (only `Sub` entries counted) | yes | **entry_budget_counts_exif_gps_and_interop_links**, **entry_budget_counts_the_top_level_chain**, lib |
| f5 (the old per-item `findings.iter().any(...)` rescan put back into `g_category_shape`, with no `tick`) | **survived** | see finding 1 |

Survivors: none among the library mutants; one test-strength survivor (f5).

**Fuzz plant.** v10 was planted on the real `audit` libFuzzer target (fresh scratch corpus, seeds `fuzz/seeds/audit`, 60 s) and crashed on seed load. The violations named `indices` ("finding 0: item index 1 out of range"), the F5-rewritten `category_shape` path ("region item 0 has no finding") and `findings`. The mutant was reverted and `src` is clean; the artifacts are kept in the scratchpad, not in the repo.

**Carried tests judged.** M20, M21, M22, m04, m07 and m08 each pin the behaviour, not just the mutant:
- M20 covers four block orders.
- M21 checks before and after the scan, on both the item and the finding.
- M22 checks six type combinations.
- The m04/m07/m08 tests use exact-fit controls beside a crossing by one, assert the exact issue (error, link, offset) and `entry_count`, and cover all four link kinds.
- F1 is the exact file name.
- F5's code is linear by inspection: one marking pass, plus at most one `jpeg::read` for the whole guard. The test's guarantee is limited; see finding 1.

**Findings (all Low, none blocking).**
1. **Low (test strength, F5).** `audit_harness_guards_are_linear_in_the_input` counts only the `tick()` calls placed in the two guards. A step counter can be linear while time is quadratic. The f5 mutant restores the exact old `O(items x findings)` rescan in `g_category_shape` with no tick, and the test passes. Any new inner loop without a `tick`, or a quadratic loop in another guard, would pass too. The fix itself is correct and linear. The test protects only the instrumented points, so it is a weak regression guard. A possible follow-up is to time it relative to a linear baseline, or to put the tick inside every closure that iterates.
2. **Low (documentation of measured output).** The `measured` string of `seed-jpeg_segments/iptc-8bim` in `KNOWN_DIVERGENCES` says "no 0x0404 resource, so no IPTC group". The seed does carry an 8BIM 0x0404 resource (5 bytes `01 02 03 04 05`, no IPTC dataset); ExifTool lists only `[Photoshop] 0x0408`. The Class G note in `guidance/questions.yaml` (`audit-unmodelled-exiftool-groups-jfif-photoshop`) gives the same wrong example. The pinned diff is right; only the prose is wrong.
3. **Low (Class H rule breadth).** `their_inventory` clears every region on any bare TIFF. `tiff_region_lines_come_only_from_directory_valued_ids` checks that such a TIFF carries one of the four ids, but not that the region kind matches the id (0x83BB/0x8649 to Iptc, 0x02BC to Xmp, 0x8773 to Icc). With one such input today (`tag-83bb`, Iptc) nothing is hidden, but the rule is looser than it needs to be.
4. **Info.** The records do not keep ExifTool's exit status. Four inputs exit 1 with an `[ExifTool] Error` line that is dropped as file-level; both sides are empty there, so this is not a divergence. Also, the 13 Class R rows and the Class F after-scan rows sit on files whose `Health::clean` is true. That is consistent with the invariant, which binds only `ours_hides`.

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** null. The verifier sub-agent cannot read its own meter; the orchestrator transcribes the harness `subagent_tokens` figure.
- **Estimated USD:** null (follows from the transcribed tokens)
- **Duration (minutes):** null (the orchestrator transcribes the metered wall-clock)
- **Source of the number:** harness report, read by the orchestrator

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none found beyond those the build declared in HANDOFF-027: Class R, the ICC `desc` size 17 with 16 header lines, `inventory_corpus.rs`, the carried tests placed in `tests/budget_oracle.rs`, and the `<corpus>/<name>` keys. I re-measured each declared deviation and it holds.
- **Follow-up work identified:**
  - Strengthen the F5 linearity test (finding 1).
  - Correct the `iptc-8bim` `measured` text and the Class G example (finding 2).
  - Optionally tie each TIFF region kind to its directory-valued id (finding 3).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Very little. "Redo one fuzz plant of the F5 harness change" could mean planting into the harness or into the library. I planted into the library, on a path that the rewritten guard must catch.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The worktree hook refuses compound shell commands (heredocs, loops, pipes with regex), so scratch work had to be file-based scripts. A line in the toolchain brief would save a cold agent a few round trips.

3. **If you did this task again, what would you do differently?**
   — I would start the `src` mutant sweep straight after the gates, because it is the long pole (about 2 minutes per mutant across the whole suite), and do the re-measure and reading while it runs.
