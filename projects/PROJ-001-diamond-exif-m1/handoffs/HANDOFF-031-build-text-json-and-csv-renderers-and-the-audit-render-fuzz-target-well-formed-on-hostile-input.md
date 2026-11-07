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
  id: HANDOFF-031
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-06
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-015

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
  tokens_total: 515801             # orchestrator-recorded from the metered sub-agent results (subagent_tokens)
  estimated_usd: 3.40
  duration_minutes: 58
  branch: worktree-agent-a98883200c333058f
  pr: null
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "tokens transcribed by the orchestrator from the sub-agent metered results: 493,090 for the build run and 515,801 reported after the follow-up run, read as cumulative (the follow-up is then 22,711); wall-clock 2,895,129 ms + 600,502 ms = 58 minutes; the agent's own 200-minute estimate is not used"
  synced_at: 2026-11-06
---

# HANDOFF-031: Text, JSON and CSV renderers and the audit_render fuzz target

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-015` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (API, rules 1 to 9, JSON schema, CSV columns, checkers, fuzz guards, named tests and the 30 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-015-text-json-and-csv-renderers-and-the-audit-render-fuzz-target-well-formed-on-hostile-input.md`
- **DEC-010 (normative for escaping, the value-free default, the JSON and CSV shapes and file status; read it, do not edit it):** `decisions/DEC-010-report-rendering-contract.md`
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Decisions: DEC-008, DEC-009 (dispositions, `reliable`, the 2026-11-06 amendment's wording rules), DEC-006
- Code: `src/audit/{mod,preview,category}.rs`, `src/policy.rs`, `src/tiff/mod.rs` (`Container::payload`, `IfdKind`), `fuzz/shared/audit_invariants.rs` and `fuzz/fuzz_targets/audit.rs` (the pattern to copy: guards run alone through `run_guard`, step counter, plants in `tests/fuzz_layout.rs`), `tests/fuzz_layout.rs`, `tests/audit_model.rs` (seed counts 32/37/76 become 32/37/78), `tests/policy_preview.rs`, `examples/fuzz_seeds.rs`, `fuzz/Cargo.toml`, `.github/workflows/ci.yml`, `app.just`
- Prior art: HANDOFF-029/030 (SPEC-014 build and verify: the verify findings F1 to F3 are carried into this spec) and HANDOFF-027/028 (SPEC-013: the linearity weakness).

Constraints: `panic-free-library` (five-lint deny set: `arithmetic_side_effects` forbids unchecked `+`, `*`, `<<`; use `saturating_add`, `checked_*`, iterators; never index a string by byte position), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red` (applied to the mutant list, including the checker mutants), `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files: `src/report/{mod,safe,text,json,csv}.rs`, `fuzz/shared/render_check.rs`, `fuzz/shared/render_invariants.rs`, `fuzz/fuzz_targets/audit_render.rs`, `tests/report_render.rs`, `tests/render_check.rs`, `tests/golden/report-full-house.{txt,json,csv}`, `fuzz/seeds/audit_render/*`; edits: `src/lib.rs`, `src/audit/mod.rs` (`Item::value`, additive only), `fuzz/Cargo.toml`, `.github/workflows/ci.yml` (matrix entry), `app.just` if `fuzz-smoke` needs the target, `examples/fuzz_seeds.rs` (new seeds), `tests/audit_model.rs` and the seed hash lists (the two full-house `audit` seeds), `tests/fuzz_layout.rs`. `audit()` results and every existing test expectation stay unchanged except the seed counts and hash lists the spec names.
- Order: `safe_text` and its tests; the two checkers and their valid/invalid tables (test the testers before using them); `Item::value`; `status_of` and `summarize`; JSON; CSV; text and the golden files; the render guards and plants; the `audit_render` target, seeds and CI entry; the carried items (SPEC-014 F1 to F3, the SPEC-013 linearity wall-clock bound); then the 30 mutants plus the five carried ones (for each: control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- Type DEC-010's escape list, the JSON key lists and the CSV header **from the DEC and the spec by hand** in the harness; none may be imported from `src/report`. The checkers must not call anything in `src/report`.
- A doubt about a DEC-010 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), and `just fuzz-smoke audit` and `just fuzz-smoke audit_render` (60 s each; record executions and coverage). Report the wall-clock bound you chose, the measured time of the quadratic rescan mutant, and the largest output size at the caps.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-031); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- File I/O, directory walking, argument parsing, exit codes (SPEC-016); value decoding; tag names; any ExifTool comparison
- Any change to `audit()` results, the presets or the preview
- Pushing, PRs, cycle advancement, SPEC-016

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback

### Execution notes

- **Branch / PR:** branch `worktree-agent-a98883200c333058f` (fast-forwarded to `claude/rust-exif-metadata-6351d5` at 105a723 first: `git merge --ff-only` succeeded, no merge commit). No PR, not pushed, cycle not advanced. The final commit hash is in the report that carried this handback.
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes, with the deviations listed below. Gates run locally and green: `cargo fmt --all -- --check`, the same with `--manifest-path fuzz/Cargo.toml`, `rustfmt --edition 2021 --check` on the three harness files; `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`; `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on Homebrew stable and on `~/.cargo/bin/cargo +1.90.0` (all targets pass); `RUSTFLAGS="-F unsafe_code" cargo +1.90.0 check --all-targets`; `just oracle-record` (`git status --porcelain` clean after it, once the new records were committed, see deviation 3); `just fuzz-smoke audit` and `audit_render`.
- **What was built:** `src/report/{mod,safe,text,json,csv}.rs` (+ `render_tests.rs`), `Item::value` (additive: `PhantomData` replaced by `value: Option<&'a [u8]>`, derives unchanged), `fuzz/shared/{render_check,render_invariants}.rs`, `fuzz/fuzz_targets/audit_render.rs`, 17 `audit_render` seeds, two new `audit` seeds (`audit-full-house-tiff`, `audit-full-house-jpeg`), `tests/{report_render,report_formats,render_check}.rs`, `tests/golden/report-full-house.{txt,json,csv}`, `tests/support/report_inputs.rs`, edits to `fuzz/Cargo.toml`, the CI matrix, `app.just`, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs` and the seed-count assertions. No dependency added; `decisions/`, `scripts/`, irradiance untouched; existing seeds and records byte-identical (`git status` on `fuzz/seeds` shows additions only).

### Cost self-report

- **Tokens (total):** 515,801 (orchestrator-transcribed; 493,090 reported at the first hand-back plus the follow-up, read as cumulative)
- **Estimated USD:** 3.40 (515,801 tokens at 6.60 USD per Mtok)
- **Duration (minutes):** 58 (harness wall-clock, build run plus follow-up; the agent's own estimate was about 200)
- **Source of the number:** harness report (subagent_tokens)

### Fuzz smoke (60 s each, nightly, from the committed seeds, `just fuzz-smoke`)

- `audit` (78 seeds): clean. 103,085 runs in 61 s, `DONE cov: 2080 ft: 8007 corp: 841/7323Kb` (an earlier identical run: 173,751 runs).
- `audit_render` (17 seeds): clean. 68,357 runs in 61 s, `INITED cov: 3639 ft: 7628 corp: 17/3835b`, `DONE cov: 4469 ft: 14825 corp: 1045/289Kb exec/s: 1120`.
- Plants against the real targets (fresh scratch corpus under the scratchpad, the seeds directory as the only input, working tree reverted afterwards, `git diff` empty):
  - `audit`, the old SPEC-014 mutant 9 (`may_hold(MakerNote)` loses `Gps`): **found at once, on seed load** (`preview_dispositions`: "gps: finding 8: expected kept_unread, got kept"). With the two full-house seeds this closes carried F1.
  - `audit_render`, mutant 15 (`@` missing from the CSV formula guard): found at once on seed load (`csv_rows`: "row 0: column 0 starts a formula: \"@SUM(1)\"").

### Wall-clock bound and the quadratic mutant (SPEC-013 linearity)

- `audit_harness_guards_are_linear_in_the_input` keeps its step counter and now also times each of the eleven audit guards (best of three runs) on `heavy_findings_and_regions()`: twelve Exif blocks of 5,000 thumbnail entries and 4,000 JUMBF segments, 64,000 items and 63,988 findings. **Bound chosen: 55 ms per guard.** Measured on this host (Apple silicon, the test profile): the slowest linear guard (`thumbnail`) takes 12 ms, `category_shape` 3.4 ms, the rest under 3 ms.
- The rescan-without-tick mutant (`!got.findings.iter().any(|f| f.item == Some(i))` replacing the marking pass in `g_category_shape`) took **1.22 s**: `guard category_shape took 1.222467917s ... (bound 55ms): it is quadratic in time`. That is 22 times the bound and 360 times the linear guard. A generous bound and a 20 times margin pull against each other: a JPEG holds at most 4,095 regions, so the ratio cannot grow, and the bound is 4.6 times the slowest linear guard (12 ms), not more. Best-of-three guards against a scheduling hiccup; a runner more than 4 times slower than this host would flake. I judged that acceptable; a bound of 60 ms would give a 20 times margin on the mutant and is the highest that does.

### Largest output at the caps (`largest_output_at_the_caps_is_measured`, `--ignored --nocapture`)

One file, 65,536 items and 65,536 findings (every entry a Make whose shared 70-byte payload escapes to a full 64-byte excerpt of `\x01`), policy `all_pii`:

| options | text | JSON | CSV |
|---|---|---|---|
| inventory + values | 47,710,754 B | 71,959,372 B | 22,151,327 B |
| values only | 24,052,233 B | 37,880,656 B | 22,151,327 B |

About 550 bytes per entry in JSON and 340 in CSV. A 72 MB JSON string for one hostile 786 KB file is large enough that SPEC-016 should know: the renderers return a `String`, so memory is about the size of the report. I added no cap. The bounded test `largest_output_at_the_caps_stays_under_the_ceiling` asserts 1,400 bytes per entry (91.7 MB).

### Mutants (each: control green first, applied by hand as a text replacement, run, reverted with `git checkout`, `git diff` empty; the script is `mutants.py` in the scratchpad)

"harness" = `render_harness_holds_on_every_seed_and_degenerate_inputs`, "cross" = `every_seed_every_policy_every_option_renders_well_formed_output`, "plants" = `each_render_guard_goes_red_alone_when_planted` (its control run fails when the mutant breaks an unplanted case).

| # | Mutant | Caught by (assertion text shortened) |
|---|---|---|
| 1 | `"` unescaped | `safe_text_escapes_controls_c1_bidi_separators_quote_and_backslash` (left `say \"hi\"\\`), `excerpt_of_a_non_ascii_type_is_the_raw_payload_prefix`, `csv_quotes_..._end_to_end`, `values_are_absent_unless_asked`, harness (`json_facts`), cross |
| 2 | DEL left | the same unit test (left `\u{7f}`, right `\x7f`), `audit_render_seeds_are_committed...`, harness, cross |
| 3 | C1 left | the same unit test (`\u{0080}`), `text_has_no_forbidden_character_or_phrase`, harness, cross |
| 4 | U+202E left | the same unit test (`0x202e`), `text_has_no_forbidden_character_or_phrase`, harness, cross |
| 5 | invalid bytes to U+FFFD | `safe_text_writes_invalid_utf8_bytes_as_hex_escapes`, `safe_text_is_injective_on_a_sampled_set` ("two inputs give"), `safe_text_output_bound_holds`, harness, cross, plants, `render_guards_are_linear` |
| 6 | JSON skips `\u00XX` | `json_escapes_quote_backslash_and_every_control` (unit: "a raw control in") only |
| 7 | key order status/label | `json_has_the_pinned_key_order_at_every_level`, golden, harness/cross/plants/`render_guards_are_linear` (`json_shape`), `a_truncated_audit...` |
| 8 | JSON ok for issues | `json_enumerations_are_pinned`, cross (`json_facts`: "status ok, want issues"), `a_truncated_audit...` |
| 9 | residuals when unreliable | `json_preview_residuals_are_null_unless_reliable` ("unreliable: both null"), golden, harness, cross, plants, `a_truncated_audit...` |
| 10 | summary double-counts | `json_summary_matches_summarize`, golden, harness/cross/plants (`summary_facts`) |
| 11 | by_category order | `json_has_the_pinned_key_order_at_every_level`, golden, `json_shape`, `a_truncated_audit...` |
| 12 | quotes not doubled | `csv_quotes_comma_quote_cr_lf_and_doubles_quotes` (unit), `..._end_to_end`, `values_are_absent_unless_asked`, harness, cross (`csv_wellformed`) |
| 13 | CSV LF | `csv_quotes_...` (unit: `out.ends_with("\r\n")`), `csv_uses_crlf`, golden, 8 more format tests, harness, cross (`csv_wellformed`: "bare LF"), `largest_output_at_the_caps_stays_under_the_ceiling` |
| 14 | CR not quoted | `csv_quotes_comma_quote_cr_lf_and_doubles_quotes` (unit) only |
| 15 | guard omits `@` | `csv_formula_guard_prefixes_each_trigger_character_on_hostile_fields_only` (unit) and `..._end_to_end`, harness and cross (`csv_rows`: "starts a formula"), the real `audit_render` plant |
| 16 | guard on category | `csv_formula_guard_prefixes_each_trigger_character_on_hostile_fields_only` (unit: "column 3 for '='") only; no real category starts with a trigger character |
| 17 | no row for a file without findings | `csv_has_a_row_for_every_file_without_findings`, golden, harness/cross/plants (`csv_rows`: "1 rows, want 2") |
| 18 | inventory drops items | `csv_inventory_mode_rows`, `a_truncated_audit...`, harness/cross/`render_guards_are_linear` (`csv_rows`: "16 rows, want 19") |
| 19 | "contains none of it" | `text_words_kept_and_kept_unread_as_dec_009_requires`, golden (txt) |
| 20 | text excerpt with values off | `values_are_absent_unless_asked`, harness/cross/plants (`values_gate`: "a text excerpt with values off"), `a_truncated_audit...` |
| 21 | cap 65 | `excerpt_cuts_ascii_at_nul_and_caps_at_64_bytes_and_flags_truncation`, `excerpt_of_a_non_ascii_type_is_the_raw_payload_prefix`, harness/cross/plants (`json_facts`: excerpt) |
| 22 | truncated on escaped length | `audit_render_seeds_are_committed...`, harness/cross/plants (`json_facts`) |
| 23 | values default true | `excerpt_of_a_non_ascii_type_is_the_raw_payload_prefix` only (its `excerpt(&item, &RenderOptions::default())` is `Some`, want `None`) |
| 24 | failed label unescaped | `json_escapes_quote_backslash_and_every_control_end_to_end`, harness/cross (`json_facts`: label) |
| 25 | `status_of` Ok for Unsupported | `status_of_follows_dec_010` ("an unsupported audit with zero findings is never ok"), `summarize_counts_each_status_category_and_reliability`, 8 more, golden, harness/cross/plants |
| 26 | JSON checker, trailing comma | `json_checker_accepts_valid_documents_and_rejects_each_invalid_one` ("accepted a trailing comma in an array: [1,2,]"), plants (`json_wellformed`) |
| 27 | JSON checker, leading zero | the same test ("accepted a leading zero: [01]") only |
| 28 | JSON checker, raw control | the same test, plants (`json_wellformed`) |
| 29 | CSV checker, ragged row | `csv_checker_accepts_valid_documents_and_rejects_each_invalid_one` ("accepted a ragged row"), plants (`csv_wellformed`) |
| 30 | CSV checker, bare quote | the same test ("accepted a bare quote in an unquoted field"), plants (`csv_wellformed`) |

Carried, applied against the new tests:

| Mutant | Result |
|---|---|
| SPEC-014 15 (truncated rule dropped) | caught by `audit::preview::tests::truncated_audit_residual_severity_is_conservative` and `a_truncated_audit_previews_conservatively_end_to_end` ("gps_and_device ... Some(Low) vs Some(High)") |
| SPEC-014 16 (truncated `all()` exception dropped) | caught by the same two ("everything ... Some(High) vs None") |
| SPEC-014 vG (harness `custom_names` always empty) | caught by `preview_custom_case_is_non_trivial` ("only 0 inputs give a non-trivial custom set") |
| SPEC-014 vM (harness truncated `all()` exception dropped) | **survived at first**: `h_want` is reached only by the preview guards and no truncated input with findings reached them. I added the harness preview guards over the `over` input to `a_truncated_audit_previews_conservatively_end_to_end`; now caught (`preview_residual_and_reliable`: "everything: residual severity None, want Some(\"High\")") |
| SPEC-013 rescan without tick | caught by `audit_harness_guards_are_linear_in_the_input` (1.22 s against 55 ms) |
| SPEC-014 mutant 9 (F1) | found at once by the real `audit` target on seed load, see above |

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  1. **Format tests moved out of `src/`.** `just lint-red-proof` copies only `Cargo.toml` and `src/`, so a unit test under `src/` cannot `include!` the checkers or read `tests/support`. The tests that read a whole report back with the strict parsers are in `tests/report_formats.rs` under their spec names (`values_are_absent_unless_asked`, `json_has_the_pinned_key_order_at_every_level`, `json_enumerations_are_pinned`, `json_preview_residuals_are_null_unless_reliable`, `json_summary_matches_summarize`, `csv_has_the_pinned_header_and_19_fields_per_row`, `csv_uses_crlf`, `csv_has_a_row_for_every_file_without_findings`, `csv_inventory_mode_rows`, the four `text_*`, `render_is_deterministic`). Three names exist twice: the layer-only version in `src/report/render_tests.rs` under the spec name (`json_escapes_quote_backslash_and_every_control`, `csv_quotes_comma_quote_cr_lf_and_doubles_quotes`, `csv_formula_guard_prefixes_each_trigger_character_on_hostile_fields_only`) and the pipeline version in `tests/report_formats.rs` with an `_end_to_end` suffix. Extra tests: `report_names_are_pinned` (src), `largest_output_at_the_caps_stays_under_the_ceiling` (the bounded twin of the ignored named test), `audit_render_target_is_declared_and_runs_in_ci`.
  2. **Option byte.** The failure reason uses bits 5 to 6 of byte 0 (all four `FailReason`s), not bit 5 alone.
  3. **Edits outside the spec's list, forced by the two new `audit` seeds:** counts and lists in `tests/inventory_oracle.rs`, `tests/support/inventory.rs`, `tests/mutation_oracle.rs`, `tests/budget_oracle.rs`, `tests/policy_preview.rs` (78 audit seeds, 249 inputs, 180 distinct, 14 Class R rows, 33 pinned divergences); a new pinned Class R divergence for `seed-audit/audit-full-house-jpeg` (ExifTool lists no ICC or JUMBF group for its stub payloads); `just oracle-record` appended two sections (28 lines, nothing removed) to `tests/oracle-fixtures/exiftool-inventory/seed-audit.txt`. `guidance/toolchain-brief.md` carries the new counts.
  4. `audit_harness_guards_are_linear_in_the_input` times each guard separately (best of three), and `thumbnail_tiff(k)` was split out of `many_thumbnails(k)` (same bytes).
  5. `audit_invariants.rs`: `H_PRESETS`, `H_ALL`, `h_want`, `rank` and `custom_names` became `pub` (no behaviour change) so `render_invariants` reuses the typed DEC-009 grid.
  6. `Summary.reliable_files + unreliable_files` counts audited files (a failed file has no audit), as the `summary_facts` guard in the spec says, not `files`.
  7. The IFD name `other` is in the spec's list but unreachable: `IfdKind` is `#[non_exhaustive]` yet exhaustive inside its own crate, and a wildcard arm would be an unreachable-pattern warning.
  8. `json_facts` cannot compare the excerpt of a finding whose item did not fit in the inventory (past 65,536 entries; unreachable at libFuzzer's default 4 KiB input) because the independent payload walk is capped like the items; its shape and the values-off rule are still checked.
  9. In the `audit_render` input, byte 1 is both the label length and the byte a Custom category set comes from (the spec says "the next input byte").
- **Doubts about DEC-010 / SPEC-015 (for the architect; `decisions/` untouched; also in `guidance/questions.yaml`, id `report-dec-010-build-doubts`):** (1) the summary's `residual_findings` is `Some(0)` when a policy is set and no file is reliable, which reads like "nothing remains"; (2) a `Custom` policy is only named "custom"; (3) the library does not bound the label or the failure detail (rule 8 says bounded) and prints the detail with `values` off; (4) the TAB and CR formula triggers never fire through `render_csv` because `safe_text` escapes them first, and DEL's escape form is unspecified (built `\x7f`).
- **Follow-up work identified:** SPEC-016 should bound a path, pass only OS error text as `detail`, and decide how to stream a report that can reach 72 MB.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Where the named "src/report (unit)" tests could live: the red-proof script copying only `src/` was not in the brief, so the checker-based tests moved after they were written. Also the 20 times margin and a generous wall-clock bound cannot both hold with at most 4,095 regions.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The inventory oracle suite (pinned divergences and corpus counts) breaks when a seed is added to `fuzz/seeds/audit`; the brief named `tests/audit_model.rs` and "the seed hash lists" only.

3. **If you did this task again, what would you do differently?**
   — Run `just lint-red-proof` right after the first `src/` test, add the full-house `audit` seeds and run the oracle suite before writing the renderers, and write the per-guard timing test first.

### Architect follow-up (after 64cb36f): `residual_findings` is `None` when no file is reliable

- **Change:** `summarize` now returns `residual_findings: Some(n)` only with a policy AND `reliable_files > 0` (sum over reliable files only); `None` without a policy or when no audited file is reliable. JSON `summary.residual_findings` is `null` then; the text summary prints `policy <name>: residual not available: no file was read reliably` instead of a zero. This answers doubt (1) above.
- **Tests first (red before the code):** `summary_residual_is_none_when_no_file_is_reliable` (`tests/report_formats.rs`: all files unsupported/unreadable/issues/failed, policies Gps and Everything: `summarize` None, JSON null, the text line, no "residual findings 0"; no policy: no residual line; with one reliable file the sum 5 returns); `summarize_counts_each_status_category_and_reliability` extended (a run of one PNG gives None, the empty run gives None). The `summary_facts` guard re-derives it independently (Some only with a policy and a reliable audited file); two new plants in `each_render_guard_goes_red_alone_when_planted` ("a zero residual over no reliable file", summary and JSON summary), `summary_facts` now has 11 plants in total.
- **Golden files:** unchanged (the golden run has a reliable file); no regeneration.
- **Mutant 31** (`residual_findings` stays `Some(0)` when `reliable_files == 0`): caught by `summarize_counts_each_status_category_and_reliability` (left `Some(0)`, right `None`), `summary_residual_is_none_when_no_file_is_reliable` ("gps: left Some(0) right None"), and the `summary_facts` guard through the harness, cross product, plants control and `a_truncated_audit...`. Reverted, `git diff` empty.
- **A harness bug the fuzzer found in this re-gate (my error, fixed):** the first 60 s `audit_render` run after the change crashed on `text_safe` ("the label line \"file: \\n\" is not there twice"): with an empty label and a failure detail ending in `file: `, the substring count over-counted. The guard now counts whole lines; regression case added to `render_harness_holds_on_every_seed_and_degenerate_inputs`. Mutants 1 to 30 were not re-run for this guard-only change; `each_render_guard_goes_red_alone_when_planted` (the `text_safe` label plants) stays green.
- **Gates re-run after the final commit's content (all exit 0):** fmt (workspace, fuzz graph, three harness files), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on stable and `+1.90.0` (no failure), `-F unsafe_code` check, `just oracle-record` (clean `git status --porcelain`). Fuzz smoke 60 s: `audit` 100,102 runs, `DONE cov: 2088 ft: 8061 corp: 852/6973Kb`; `audit_render` 69,880 runs, `DONE cov: 4489 ft: 15026 corp: 941/247Kb exec/s: 1145` (after the guard fix; the run before it crashed as described).

### Follow-up 2 (after the cold verify, HANDOFF-032, APPROVED at 6723aad)

`git merge --ff-only claude/rust-exif-metadata-6351d5` was accepted (to 9f32ab6). Changes, each test first:

- **F1 CSV `;` bypass.** A field is quoted iff it holds a comma, **semicolon**, quote, CR or LF. Tests: the unit `csv_quotes_comma_quote_cr_lf_and_doubles_quotes` (`a;b`, `;`, `a;=1+cmd` quoted) and `csv_semicolon_fields_are_quoted_and_parse_back_as_one_field` (label `a;=1+cmd|' /C calc'!A0`, an excerpt `x;=1+y` and a failure detail: quoted, the strict parser reads ONE field equal to the source). The checker gained `parse_csv_quoted` (each field also says whether it was written quoted); `csv_rows` re-checks the predicate (typed again, includes `;`) on every field; two plants (a `;` field written unquoted, a plain field quoted); new seed `render-label-semicolon-formula` (18 `audit_render` seeds; generator output otherwise unchanged, existing seeds byte-identical). Golden csv/json unchanged.
- **F2 invisible characters.** `safe_text` escapes U+00AD, U+200B..U+200D, U+2060, U+FEFF, U+206A..U+206F, U+FFF9..U+FFFB and U+E0000..U+E007F as `\u{...}`. Note on the brief: the existing escapes use **lowercase** hex (`\u{202e}`, pinned by the golden and tests), so the new ones do too; tag characters have five digits (`\u{e0041}`). Tests: every listed code point and range edges, neighbours that stay unchanged (U+00AC, U+00AE, U+200A, U+2010, U+2061..U+2064, U+205F, U+2065, U+FEFE, U+FF00, U+FFF8, U+FFFC, U+DFFFF, U+E0080), injectivity alphabet and bound test extended, the harness's naive `safe_text` and its table test (`tests/render_check.rs`), the `text_safe` forbidden-character guard and the forbidden-character helper of `tests/report_formats.rs`, three plants.
- **F3.** `summary_residual_counts_the_reliable_files_only_when_an_unreliable_file_has_findings`: an unreliable JPEG with 2 findings (two Exif segments) next to the reliable full house under Gps: residual `Some(14)` in `summarize`, JSON and text, in both orders.
- **F4.** `excerpt_content_is_the_expected_escaped_text_in_every_format`: a 70-byte ASCII value starting `ab\x01`: exact expected text `ab\x01` + 61 `x` (64 source bytes, 67 characters), truncated true, in JSON, CSV and text; and an exact 64-byte value stays uncut and untruncated.
- **F5.** `Item` has a hand-written `Debug` printing origin, kind and `value_len` only; `PartialEq` unchanged; the doc comment no longer says it holds no value bytes. Test `debug_of_an_item_and_an_audit_never_prints_the_payload` (items, findings and a whole audit holding `SECRET-SERIAL-0042`: absent as text and as a byte list; `value_len: Some(19)`).
- **F6.** The absolute 55 ms bound is gone. `audit_harness_guards_are_linear_in_the_input` keeps the step counter and times each guard (best of 3 at N, best of 5 at N/4; N is 12 Exif blocks of 5,000 thumbnails and 4,000 regions, N/4 is 3 blocks and 1,000 regions) and asserts time(N) < 8 x max(time(N/4), 1 ms). **Measured ratios, linear guards:** 3.8 to 4.7 (`indices` 4.0 to 4.2, `category_shape` 3.9 to 4.4, `inventory` 3.9 to 4.4, `findings` 3.9 to 4.7, `thumbnail` 3.9 to 4.0; the sub-millisecond `caps` guard 3.8 to 7.3 under the 1 ms floor). **Rescan-without-tick mutant:** `category_shape` 75.3 ms at N/4 and 1,204.8 ms at N, ratio **16.0**, red ("it is superlinear in time").
- **F9.** The text region line no longer prints `segment=` twice (`region xmp len=4 block=- segment=1`). Golden `.txt` changed (four region lines, nothing else); `.json` and `.csv` unchanged. The stale "eleven" comment in `tests/inventory_oracle.rs` is fixed. F7 (SPEC-016) and F8: nothing done.

**Mutants 32 to 37 and the F6 check** (each applied by hand, red, reverted; the file equals the index afterwards):

| # | Mutant | Caught by |
|---|---|---|
| 32 | semicolon dropped from the quoting predicate | `csv_quotes_comma_quote_cr_lf_and_doubles_quotes` (unit), `csv_semicolon_fields_are_quoted_and_parse_back_as_one_field`, harness, cross, plants (the `;` unquoted plant's control and mutation) |
| 33 | U+200B left unescaped | `safe_text_escapes_controls_c1_bidi_separators_quote_and_backslash`, `text_has_no_forbidden_character_or_phrase` |
| 34 | tag range off by one at the top (U+E007F left) | `safe_text_escapes_controls_c1_bidi_separators_quote_and_backslash` (the edge U+E007F) |
| 35 | V24 residual also sums unreliable files | `summary_residual_counts_the_reliable_files_only_when_an_unreliable_file_has_findings` |
| 36 | V8 text excerpt cut to 64 characters after escaping | `excerpt_content_is_the_expected_escaped_text_in_every_format` |
| 37 | `Debug` of `Item` prints the bytes | `debug_of_an_item_and_an_audit_never_prints_the_payload` |
| F6 | rescan without tick | `audit_harness_guards_are_linear_in_the_input` (ratio 16.0) |

**Gates (all exit 0, `git status --porcelain` clean after `just oracle-record`):** fmt (workspace, fuzz graph, the three harness files), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`; `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on stable and `+1.90.0` with no failure; `RUSTFLAGS="-F unsafe_code"` check. Fuzz smoke 60 s: `audit` 89,368 runs, `DONE cov: 2094 ft: 8094 corp: 855/6731Kb`; `audit_render` 62,654 runs, `DONE cov: 4600 ft: 15953 corp: 972/243Kb`.

### Follow-up 3 (delta verify at e093e57: PUNCH LIST, one flake and one seed gap)

`git merge --ff-only claude/rust-exif-metadata-6351d5` answered "Already up to date" (the verifier's docs commit 581bc5f was not on that branch yet).

- **F6 flake fixed.** `audit_harness_guards_are_linear_in_the_input` now (a) measures small and large inputs in interleaved rounds (small, large, small, large, ...), 7 rounds, minimum kept for each; (b) uses a 5 ms floor on the small time; (c) allows 3 independent attempts and fails only if every attempt has a guard over the 8x bound. The step counter is kept.
- **Load test** (release of nothing: the built test binary run 20 times in a row with 8 `yes > /dev/null` processes running, killed afterwards, none left): **20 of 20 passed**, every run on its first attempt, worst effective ratio (against the floor) 2.7. Unloaded: **10 of 10 passed**, first attempt each, worst 2.6. Linear guards at their own ratio: 3.7 to 4.2.
- **Rescan-without-tick mutant still red:** `category_shape` 76.4 ms at N/4 and 1,229 ms at N, ratio 16.1 on attempt 1, 16.1 on attempt 2, 16.1 on attempt 3 (76.5 / 1,230 ms and 76.4 / 1,227 ms), then `superlinear in time on all 3 attempts`. Reverted, tree clean.
- **N4 seed.** New `audit_render` seed `render-label-invisible-characters` (19 seeds): label `a` U+200B U+FEFF U+00AD U+E0041 `b`, valid UTF-8, option byte policy `all_pii` + values. Generator, seed-set list and seed-shape test updated. Mutant N4 (the harness's naive `safe_text` forgets U+E0000..U+E007F) is now red in the harness itself: `render_harness_holds_on_every_seed_and_degenerate_inputs` fails on that seed with `json_facts` ("label ... want ...\\u{e0041}" unescaped) and `csv_rows`; reverted.
- `cargo run --example fuzz_seeds` leaves `git status --porcelain` empty after the seed is committed.
- **Gates (all exit 0, `git status --porcelain` clean after `just oracle-record`):** fmt (workspace, fuzz graph, three harness files), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`; `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on stable and `+1.90.0`, no failure; `-F unsafe_code` check. Fuzz smoke 60 s: `audit` 81,290 runs, `DONE cov: 2101 ft: 8148 corp: 872/6777Kb`; `audit_render` 48,956 runs, `DONE cov: 4622 ft: 16098 corp: 975/236Kb`.
