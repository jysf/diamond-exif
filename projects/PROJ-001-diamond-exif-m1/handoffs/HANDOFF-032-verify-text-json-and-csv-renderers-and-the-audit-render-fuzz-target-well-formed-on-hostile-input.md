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
  id: HANDOFF-032
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: 75
  branch: worktree-agent-ae1f46310ab6829e9
  pr: null
  completed_at: 2026-11-06               # YYYY-MM-DD
  notes: "verify APPROVED at 6723aad; delta verify of follow-up 2 PUNCH LIST at e093e57 (F6 timing bound flaky under load); tokens_total null: this sub-agent has no token meter, the orchestrator transcribes the harness's metered figure" # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-032: Verify the text, JSON and CSV renderers and the audit_render fuzz target

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-015` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-015-text-json-and-csv-renderers-and-the-audit-render-fuzz-target-well-formed-on-hostile-input.md` (API, rules 1 to 9, JSON schema, CSV columns, checkers, fuzz guards, named tests and mutants are normative)
- **DEC-010 and its 2026-11-06 amendment:** `decisions/DEC-010-report-rendering-contract.md` (normative); **DEC-009** with its amendment (dispositions, `reliable`, wording)
- **Build handoff and Handback:** `HANDOFF-031-build-...md` (claims to verify, not to trust; read the seven deviations, the follow-up section, and the builder's doubts)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 105a723..HEAD` (build commits through `61817f2`; `dc0362b` and the brief commit touch only docs). Main: `src/report/{mod,safe,text,json,csv,render_tests}.rs`, `src/audit/mod.rs` (`Item::value`), `fuzz/shared/{render_check,render_invariants,audit_invariants}.rs`, `fuzz/fuzz_targets/audit_render.rs`, `tests/{report_render,report_formats,render_check,fuzz_layout}.rs`, `tests/golden/*`, `tests/support/report_inputs.rs`, `examples/fuzz_seeds.rs`, `fuzz/seeds/audit_render/*`, the changes to the inventory oracle suite.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and the three harness files, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`). Run `just fuzz-smoke audit` and `audit_render` 60 s each; record executions and coverage. Confirm `audit()` results are unchanged (`Item::value` additive; `git diff 105a723..HEAD -- src/audit`), `decisions/` and `scripts/` untouched apart from the architect's DEC-010 amendment commit, no dependency added.
- **Judge DEC-010 and the escaping as a security reviewer.** Build or find inputs that the renderers let through unsafely, for each format: a terminal escape sequence, a bidi override, a zero-width or invisible character, a Unicode "tag" character, a very long label, a label that ends a CSV quote early, a JSON-breaking sequence, a formula (`=`, `+`, `-`, `@`, TAB, CR, also after leading spaces or quotes), a NUL, a lone surrogate-looking byte sequence, overlong UTF-8. Say which `safe_text` lets through and whether that matters. Is `safe_text` really injective and total?
- **Check the code against DEC-010 and the spec by hand**: the JSON key order and key set at every level, the CSV header and 19 fields, the five statuses (rebuild the status table from the DEC, then compare), the value-free default (plant your own payload and look in all three outputs, with `values` off and on), the excerpt rule (ASCII up to NUL, 64-byte cap, `truncated`), the preview residuals null unless `reliable`, the summary and the new `residual_findings` rule, the CSV row rules (including `inventory` mode and a file with no findings), the text's required content and forbidden phrases.
- **Test the checkers and the testers.** The independent JSON and CSV checkers are the proof of well-formedness: try to make each accept something invalid (RFC 8259 and RFC 4180 corner cases beyond the builder's table) and reject something valid. Re-run the builder's mutants (the report claims 31 red) at least for 1, 5, 8, 9, 15, 16, 17, 22, 25, 26 to 30 and 31. Write mutants of your own that the builder did not try, for example: JSON writer emitting a raw `\u2028`; a number rendered with a sign or exponent; `container` null for a supported file; CSV guard applied to the `kind` column; the `inventory` flag ignored in text only; excerpt cap applied after escaping in one format only; a status computed from findings instead of health; `summarize` skipping Failed files; a golden file edited to match a bug; a guard that checks one direction of an iff only; the wall-clock linearity bound made vacuous. Report which the suite catches (name the test or guard) and which survive.
- **Judge the deviations.** (2) Adding two `audit` seeds broke the inventory oracle suite and the builder updated counts and lists and **added a pinned Class R divergence** for `seed-audit/audit-full-house-jpeg` plus 28 lines in `tests/oracle-fixtures/exiftool-inventory/seed-audit.txt`: re-run ExifTool yourself on that seed and decide whether the pin is exact and right-sided, not a loosening, and that no existing record or pin changed. (3) the `FailReason` bits; (4) the unreachable `other` IFD name; (5) the wall-clock linearity bound of 55 ms (22 times margin): is it stable, can it flake, is it meaningful; `H_PRESETS` etc. made `pub`. (7) excerpts past the 65,536 item cap.
- Redo one render-guard plant against the real `audit_render` libFuzzer target, and confirm the SPEC-014 F1 plant (mutant 9) is found by the real `audit` target within the smoke run. Check the golden files are what the renderers produce today and that the seeds have the shapes they claim. Check the largest output at the caps (the builder reports 72 MB JSON) and whether any path can produce more.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; SPEC-016 work (the binary, I/O, exit codes); value decoding; tag names; any ExifTool comparison of report content

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

*Filled in by the cold verifier (claude-opus-5-5) of SPEC-015.*

### Execution notes

- **Branch / PR:** branch `worktree-agent-ae1f46310ab6829e9`, fast-forwarded with `git merge --ff-only claude/rust-exif-metadata-6351d5` to `6723aad` (it succeeded; no merge commit). No PR, not pushed, cycle not advanced. The only change on the branch is this handback.
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes. Every gate is green when I run it myself, every named test exists, the builder's mutants that I re-ran are all caught, and the deviations hold up. The findings below are gaps in DEC-010 and in the tests. None of them breaks the spec.
- **For `verify`:** ✅ **APPROVED at `6723aad`** (code as of `61817f2`; `dc0362b` and `6723aad` change only docs). The findings are carried forward; none of them blocks.

### Gates (run by me at `6723aad`, each exit status recorded directly, no pipes)

All of these exited 0: `cargo fmt --all -- --check`, the fuzz graph, `rustfmt --edition 2021 --check` on the three harness files, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on Homebrew stable (520 passed, 0 failed, 7 ignored) and on `~/.cargo/bin/cargo +1.90.0` (520 / 0 / 7), `RUSTFLAGS="-F unsafe_code" cargo +1.90.0 check --all-targets`, `just deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `just oracle-record` (`git status --porcelain` was empty afterwards).

Other checks:
- `cargo run --example fuzz_seeds` reproduces every committed seed byte for byte (clean `git status`).
- Fuzz smoke, 60 s each, from the committed seeds:
  - `audit`: 178,517 runs, `INITED cov: 1781 ft: 4215 corp: 73`, `DONE cov: 2021 ft: 7926 corp: 908/6188Kb`, clean.
  - `audit_render`: 78,793 runs, `INITED cov: 3630 ft: 7579 corp: 17/3835b`, `DONE cov: 4449 ft: 14373 corp: 1031/257Kb exec/s: 1291`, clean.
- Plants against the real libFuzzer targets, each in a scratch copy with a fresh corpus:
  - JSON residuals printed when the preview is not reliable (`audit_render`): found while loading the seeds, by `json_facts` ("residuals printed for an unreliable preview").
  - SPEC-014 mutant 9, `may_hold(MakerNote)` without `Gps` (`audit`): found while loading the seeds, by `preview_dispositions` ("gps: finding 8: expected kept_unread, got kept"). This confirms carried F1 is closed.
- `git diff 105a723..HEAD`:
  - `decisions/` changed only in `dc0362b` (the architect's DEC-010 amendment).
  - `scripts/` is untouched.
  - No dependency added (`Cargo.toml`, `Cargo.lock` and the fuzz lock are unchanged apart from the new `[[bin]]`).
  - `src/audit` changes only by adding `Item::value` (but see F5).

### Hand review against DEC-010 and the spec

- **JSON.** Every level matches rule 5 in key order and key set: top level, `options`, `summary`, `FILE` (audited and failed), `FINDING`, `ITEM`, both `WHERE` forms, `PREVIEW`. Numbers come only from `usize::to_string`. A failed file has `container` null, `inventory_items` 0 and every other field null or empty.
- **Status.** I rebuilt the five-status table from DEC-010 part 5 and compared it with `status_of`: they match. `unsupported` is decided first and is never clean (`src/audit/mod.rs:281`). `ok` is exactly `health().clean`.
- **CSV.** The header and all 19 fields are correct. Rows follow rule 6, including `inventory` mode (an item row carries its finding's category and disposition; a finding past the item cap gets its own row) and the single row for a file with no row otherwise. The formula guard applies only to `label` and `value`, and checks the escaped text.
- **Value-free default.** I planted my own payload (`PLANT-7731-XYZ`, in an ASCII Make, a 114-byte Artist, a type-7 tag and a 64-byte Software) and checked 8 combinations: policy none or `all_pii`, `inventory` and `values` each on and off. With `values` off the payload is absent from all three formats; with it on it is present in all three.
- **Excerpts.** An ASCII tag is cut at the first NUL. A type-7 tag keeps its NUL as `\x00`. A 64-byte payload is not marked `truncated`; a 114-byte one is cut to 64 bytes and marked `truncated`.
- **Preview and summary.** Preview residuals are null unless the preview is reliable. `summarize` follows the amendment: `None` when no file is reliable, otherwise the sum over reliable files only.
- **Text.** The required content is present, `0 findings` is printed for a file with none, and the DEC-009 wording is used. None of the forbidden phrases appears in the text outside labels, details and excerpts.

### Security review of `safe_text` and the escaping (inputs fed through all three renderers)

- **Escaped correctly:**
  - Terminal escapes: OSC title `\x1b]0;..\x07`, CSI, 8-bit CSI U+009B, and a raw 0x9B byte (written `\x9b`).
  - All the bidi controls DEC-010 lists (U+202E, U+2067, ...), plus U+2028 and U+2029.
  - NUL.
  - Invalid and overlong UTF-8: `C0 80`, `C0 AF`, `E0 80 AF`, the surrogate `ED A0 80`, `F4 90 80 80` (above U+10FFFF), and `F8 ...`.
  - Labels that try to break out: `"` and `,` quoting, `</script>`, a literal `\u0000`.
  - A 100,000-byte label: output is linear in it.
- **`safe_text` is total and injective.** The output can be decoded back to the input bytes:
  - `\xNN` below 0x80 can only come from a C0 control or DEL.
  - `\xNN` at 0x80 or above can only come from an invalid byte (every byte below 0x80 is valid UTF-8 and never part of an invalid sequence).
  - C1 characters and the listed controls have their own `\u{..}` form.
  - `\` and `"` are always escaped.
  - Each byte is scanned at most twice, and the output is at most four times the input.
- **What it lets through** (allowed by DEC-010, but the DEC's own revisit trigger covers it): see F1 and F2.

### Checkers

- I ran 41 extra JSON cases and 21 extra CSV cases beyond the builder's table (`scratchpad/chk`). The table:

  | Checker | Accepts (valid) | Rejects (invalid) |
  |---|---|---|
  | JSON | `-0`; exponents; a valid surrogate pair; `\/`; raw DEL; raw U+2028; depth exactly 32 | depth 33; `+1`; `.5`; `1.`; `1e`; `NaN`; `[1,,2]`; `{"a"}`; missing comma; reversed or repeated surrogates; `\x41`; short `\u`; a BOM; a leading form feed; two documents; overlong or surrogate UTF-8 in a string; a duplicate key after unescaping; whitespace only |
  | CSV | `a,b,`; an empty quoted field; doubled quotes; a CR or CRLF inside a quoted field | a space before a quote; junk after a closing quote; a bare CR or LF in an unquoted field; a ragged row; a missing final CRLF; an unterminated quote; non-UTF-8 |

  **0 mismatches.**
- `csv_rows` checks the formula characters on the unquoted value, so it also covers quoted fields, which is stricter than the spec asks.

### Mutants

Each mutant was applied in a scratch copy as an exact text replacement. I ran the lib tests plus `report_render`, `report_formats`, `render_check`, `fuzz_layout` and `policy_preview`, then restored the copy and checked it by hash. The control run was green.

**Builder's mutants re-run, all caught:**

| Mutant | Caught by |
|---|---|
| 1 | `safe_text_escapes_...`, `excerpt_of_a_non_ascii_type...`, `values_are_absent_unless_asked`, `csv_quotes_..._end_to_end`, harness, cross product |
| 5 | the three `safe_text_*` unit tests, plants, harness, cross product, `render_guards_are_linear` |
| 8 | `json_enumerations_are_pinned`, `a_truncated_audit...`, cross product |
| 9 | `json_preview_residuals_are_null_unless_reliable`, golden, plants, harness, cross product, `a_truncated_audit...` |
| 15 | both `csv_formula_guard_*` tests, harness, cross product |
| 16 | only the unit test `csv_formula_guard_prefixes_each_trigger_character_on_hostile_fields_only` |
| 17 | `csv_has_a_row_for_every_file_without_findings`, golden, plants, harness, cross product |
| 22 | `audit_render_seeds_are_committed...`, plants, harness, cross product |
| 25 | `status_of_follows_dec_010`, `summarize_counts...`, golden, plants, harness, cross product, and 4 more |
| 26 | `json_checker_...`, plants |
| 27 | `json_checker_...` only |
| 28 | `json_checker_...`, plants |
| 29 | `csv_checker_...`, plants |
| 30 | `csv_checker_...`, plants |
| 31 | `summarize_counts...`, `summary_residual_is_none_when_no_file_is_reliable`, plants, harness, cross product, `a_truncated_audit...` |
| SPEC-013 rescan without tick | `audit_harness_guards_are_linear_in_the_input` (wall-clock bound) |

**My own mutants:**

| # | Mutant | Result |
|---|---|---|
| V1 | `safe_text` leaves U+2028 raw | caught: `safe_text_escapes_...`, `text_has_no_forbidden_character_or_phrase`, harness, cross product |
| V2 | JSON label skips `safe_text` (lossy UTF-8) | caught: `json_escapes_..._end_to_end`, harness, cross product |
| V3 | JSON number `0` written `-0` | caught: golden, `json_summary_matches_summarize`, `json_preview_residuals...`, plants, harness, cross product, linear |
| V4 | JSON number written with an exponent (`2e1`) | caught: golden, plants, harness, cross product, linear |
| V5 | `container` null for a JPEG | caught: `json_enumerations_are_pinned`, golden, plants, harness, cross product |
| V6 | CSV guard also applied to the `kind` column | caught only by the unit test `csv_formula_guard_prefixes_...` (no real kind starts with a trigger character) |
| V7 | `inventory` ignored in text only | caught: plants control, harness, cross product, `render_guards_are_linear` |
| V8 | text excerpt cut to 64 characters **after** escaping (text only) | **SURVIVED** (F4) |
| V9 | status computed from findings instead of health | caught: `status_of_follows_dec_010`, `summarize_counts...`, golden, and 7 more |
| V10 | `summarize` skips Failed files | caught: `summarize_counts...`, `json_summary_matches_summarize`, golden, plants, harness, cross product |
| V11 | text "kept" reworded and the golden file edited to match | caught: `text_words_kept_and_kept_unread_as_dec_009_requires` |
| V12 | JSON key `after_scan` renamed and the golden file edited to match | caught: `json_has_the_pinned_key_order_at_every_level`, plants, harness, cross product |
| V13 | harness `values_gate` loses its values-off branch | **SURVIVED** (F8) |
| V14 | harness `json_facts` checks residual-null in one direction only | caught: `each_render_guard_goes_red_alone_when_planted` |
| V15 | wall-clock bound made vacuous (3,600 s) plus the rescan mutant | **SURVIVED**, as expected: nothing pins the bound constant itself (F6) |
| V16 | rescan without tick alone | caught: `audit_harness_guards_are_linear_in_the_input` |
| V17 | JSON `residual_severity` printed when unreliable (severity only) | caught: `a_truncated_audit...`, cross product |
| V18 | text residual line printed when unreliable | caught: `text_preview_not_reliable_line`, `summary_residual_is_none...`, golden, `a_truncated_audit...` |
| V19 | `safe_text` leaves U+061C | caught: the unit test only |
| V20 | JSON inventory item `category` always null | caught: plants, harness, cross product, linear |
| V21 | ASCII NUL cut not applied | caught: both excerpt unit tests, `values_are_absent_unless_asked`, golden, plants, harness, cross product |
| V22 | text failure detail printed raw (lossy) | caught: `audit_render_seeds...`, plants, harness, cross product, linear |
| V23 | CSV failed row gets a container value | caught: `csv_has_a_row_for_every_file_without_findings`, golden, plants, harness, cross product |
| V24 | summary residual also sums **unreliable** files | **SURVIVED** (F3) |
| V25 | CSV inventory mode drops the disposition on item rows | caught: `csv_inventory_mode_rows`, plants, harness, cross product |
| V26 | text per-file max severity always `none` | caught: golden only |

### Deviations judged

1. **Format tests in `tests/report_formats.rs`.** Accepted. `lint-red-proof` copies only `src/`, and every named test exists; I checked the 40 names one by one.
2. **Class R pin for `seed-audit/audit-full-house-jpeg`.** Exact and on the right side. I ran ExifTool 13.55 myself (`-a -u -G1 -H -s`). It lists the same 14 tags as the new record, warns "Bad length ICC_Profile" and "Unknown APP11 'JP' segment", and lists no ICC or JUMBF group.
   - `region_only(.., [Icc, Jumbf])` is ours-only, the same as the other stub Class R rows.
   - The record diff only adds lines; no existing pin or record changed.
   - Live parity passes with `REQUIRE_ORACLE=1`.
   - Nit: the comment above the R-count assertion still says "eleven seed payloads".
3. **`FailReason` from bits 5 to 6.** Accepted.
4. **`other` IFD name unreachable.** Accepted.
5. **Wall-clock bound and `pub` grid.** The bound catches the quadratic mutant (V16), but see F6 on flakiness. Making `H_PRESETS` etc. `pub` changes no behaviour.
6. **`reliable_files + unreliable_files` counts audited files.** Accepted.
7. **Excerpts past the 65,536 item cap.** Accepted; for those findings `json_facts` checks only the shape and the values-off rule.
8. **Byte 1 serves as both label length and custom mask.** Accepted. It ties a custom set to a label length; coverage is still adequate.

### Findings (severity)

- **F1 (Medium, DEC-010 gap, not a code defect): formula guard bypass through a `;` separator.**
  - What gets through: `a;=1+cmd|' /C calc'!A0` as a label, or `x;=HYPERLINK(A1)` as an excerpt, is written unquoted because it holds no comma.
  - Why it matters: a spreadsheet in a locale that uses `;` as the separator (Excel in many European locales) splits it into a cell that starts with `=`. This is a known CSV-injection bypass.
  - Possible fixes: quote fields that contain `;`, or apply the guard after any `;`, `,` or TAB inside hostile fields. Either changes DEC-010's "quoted iff" rule, so it is the architect's call.
  - Related, lower: a leading space before a trigger (` =1+1`) and a full-width `＝` are not guarded; whether they evaluate depends on the spreadsheet's import options.
- **F2 (Low, DEC-010 gap): invisible and format characters pass `safe_text`.**
  - The characters: U+200B, U+200C, U+200D, U+2060, U+FEFF, U+00AD, U+206A to U+206F, U+FFF9 to U+FFFB, the tag characters U+E0000 to U+E007F, and variation selectors.
  - Why it matters: `photo\u{200B}.jpg` looks the same as `photo.jpg` in the terminal and spreadsheet. None of these is a control, so text stays well-formed; the risk is a spoofed path in a report.
  - DEC-010's Validation section names exactly this trigger for revisiting the decision.
- **F3 (Low, test gap): the amendment's "reliable files only" sum is not pinned (V24 survives).**
  - Every unreliable file in `summarize_counts...` and `summary_residual_is_none...` has zero findings.
  - The harness has a single audited file, so a reliable and an unreliable file never appear together.
  - Fix: add a test with an unreliable file that has findings beside a reliable one.
- **F4 (Low, test gap): text excerpt content is never compared with the re-derivation (V8 survives).**
  - `values_gate` counts text excerpts but does not compare their content.
  - The golden payloads are 1 to 4 bytes long.
  - Fix: a text excerpt check in `values_gate`, or a golden or unit case whose excerpt escapes to more than 64 characters.
- **F5 (Low): `Item::value` leaks through the derived `Debug`, `PartialEq` and `Eq`.**
  - `{:?}` of a `FileAudit`, `Item` or `Finding` now prints the whole raw payload, whatever `RenderOptions` says (a MakerNote can be 64 KB).
  - `Item` equality now compares payload bytes as well.
  - The doc comment on `Item` (`src/audit/mod.rs` around line 133) still says "Holds no value bytes".
  - Fix: either a manual `Debug` that elides the value and a documented equality rule, or correct the doc comment.
- **F6 (Low): the wall-clock bound may flake on CI.**
  - Measured three times on this host while my mutant builds were loading the CPU: the slowest guard took about 13 ms against the 55 ms bound, so only 4.2 times headroom.
  - The test runs in two CI jobs (ubuntu, debug build, parallel test threads), and its CI behaviour is unmeasured.
  - Suggestion: a relative bound (the quadratic suspect against the fastest linear guard measured in the same run), or record the CI timing once.
  - V15 shows that nothing pins the constant itself; that is inherent.
- **F7 (Low): the largest-output measurement uses a shape that is not the CSV worst case.**
  - The builder's shape gives every item its own finding.
  - With `inventory` on, 65,536 items without findings plus 65,536 findings past the item cap give 131,072 CSV rows. That is about 2 times the reported 22 MB, roughly 44 MB by estimate (not measured).
  - JSON and text are of the same order as reported, about 72 MB.
  - Carry to SPEC-016 together with streaming.
- **F8 (Info): the `values_gate` values-off plants do not target that direction (V13 survives).**
  - The three values-off plants each change one format. The values-on branch's cross-format count checks reject them anyway, so the values-off branch is never what catches them.
  - On the library side, `json_facts` still enforces JSON values-off.
  - Fix: add a plant that adds a matching excerpt to all three formats.
- **F9 (Info).**
  - A text region line prints `segment=` twice (the region's segment and the origin's).
  - The spec says `csv_rows` checks unquoted fields; the code checks all fields, which is stricter.

### Delta verify (follow-up 2: `f2c08ed` code, `e093e57` handback)

`git merge --ff-only worktree-agent-a98883200c333058f` fast-forwarded this branch from `9f32ab6` to `e093e57`. I reviewed `git diff 9f32ab6..e093e57` only and did not re-run the full gate matrix.

**Verdict: ⚠ PUNCH LIST at `e093e57`.** There is one item, F6. F1 to F5 and F9 are closed.

#### Fixes checked, adversarially (my probe calls `render_csv`/`safe_text` directly and parses with `parse_csv_quoted`)

- **F1: closed.**
  - Labels and values I fed in: `a;=1+cmd|' /C calc'!A0`, `;`, ` ;=1`, `a"b;c`, and an excerpt `x;=HYPERLINK(A1)`. Each is quoted, and the strict parser reads back exactly one field equal to the source.
  - A field that starts with a trigger and also holds a `;` (`=a;b`, `+1;2`, `-;-`, `@x,y;z`): the guard is applied before quoting, giving `"'=a;b"`, and the parser reads back the single field `'=a;b`.
  - A raw TAB never reaches the CSV layer (`\t=1;2` arrives as the text `\t=1;2`, quoted because of the `;`).
  - `csv_rows` now checks the quoting rule independently on every field, in both directions; two new plants cover it.
  - **Residual (Low, unchanged and outside this fix):** a leading space before a trigger (` =1+1`) is neither guarded nor quoted.
- **F2: closed.**
  - The exact set is escaped as `\u{…}` in lowercase hex, with five digits for tag characters (`\u{e0041}`).
  - Range edges are escaped: U+00AD, U+200B, U+200D, U+2060, U+206A, U+206F, U+FEFF, U+FFF9, U+FFFB, U+E0000, U+E007F.
  - Neighbours are unchanged: U+00AC, U+00AE, U+200A, U+205F, U+2061, U+2064, U+2070, U+FEFE, U+FF00, U+FFF8, U+FFFC, U+DFFFF, U+E0080, U+E0100.
  - The naive `safe_text` in `render_check.rs` agrees on every one of these. It is typed independently: its own range list, and `{:04x}` padding, which prints five digits on its own.
  - Injectivity still holds: the braces delimit the hex, and no escaped code point collides with literal text (`\u{e0041}` from a tag character versus `\\u{e0041}` from the literal text).
  - The 4x bound still holds: a 4-byte tag character becomes 9 bytes, a 2-byte U+00AD becomes 8.
  - **Info, outside the requested set:** these still pass through: U+2061 to U+2064 (invisible operators), U+180E, U+034F, the variation selectors U+FE00 to U+FE0F and U+E0100 to U+E01EF, and the Hangul fillers U+115F and U+3164.
- **F3: closed.** `summary_residual_counts_the_reliable_files_only_when_an_unreliable_file_has_findings` catches V24.
- **F4: closed.** `excerpt_content_is_the_expected_escaped_text_in_every_format` catches V8.
- **F5: closed.**
  - `{:?}` of an audit holding `HYPERLINK` shows only `value_len`.
  - `PartialEq` is still derived and still compares the payload; two audits of the same bytes compare equal.
  - The doc comment is corrected.
  - `debug_of_an_item_and_an_audit_never_prints_the_payload` catches mutant 37.
- **F9: closed.** A region line now reads `region xmp len=4 block=- segment=1`. The golden `.txt` changed in those four lines only. The "eleven" comment is fixed.
- **F6: meaningful, but not flake-free under load (PUNCH LIST).**
  - The bound is time(N) < 8 × max(time(N/4), 1 ms). A quadratic guard gives a ratio of about 16, so the check does catch it.
  - **Ten runs in a row under heavy load** (8 CPU-burning processes plus my parallel cargo builds, on 14 cores): **4 of 10 failed**:
    - `thumbnail`: 52.3 ms at N against 3.6 ms at N/4 (ratio 14.5)
    - `indices`: 12.4 ms against 0.76 ms
    - `inventory`: 9.1 ms against 0.49 ms
    - `indices`: 25.5 ms against 0.83 ms
  - The passing loaded runs had linear guards at 3.6 to 4.7, with outliers of 7.3 to 10.7.
  - **Without artificial load:** 10/10 passed with the test alone and 10/10 with the whole `fuzz_layout` binary in parallel. Maximum linear ratio was 4.6; `caps` reached 5.4 under the floor.
  - **Cause:** the N and N/4 runs are measured at different moments (best of 3, then best of 5), so a burst of load during the N runs alone fails a linear guard. The 1 ms floor makes the effective bound 8 ms for guards that take less than 1 ms at N/4.
  - **Suggested fix** (the builder's call):
    - interleave the N and N/4 runs so both see the same load, with more runs (best of about 7);
    - raise the floor to about 5 ms, or compare per-element times against a reference guard measured in the same rounds;
    - or retry a failure once before failing.
  - The previous absolute bound held 3/3 under my lighter mutant-build load. Neither version has been measured on CI.
- **New, Low:**
  - `decisions/DEC-010-report-rendering-contract.md` is untouched, so its part 4 ("quoted iff it contains a comma, quote, CR or LF") and its part 2 escape list no longer describe the code. The same is true of SPEC-015 rules 3 and 6.
  - The architect should amend DEC-010; I did not edit `decisions/`.

#### Mutants against the new code

Applied in a scratch copy, which I restored and checked by hash. The control run was green.

| # | Mutant | Caught by |
|---|---|---|
| 32 | `;` dropped from the quoting rule | `csv_quotes_comma_quote_cr_lf_and_doubles_quotes` (unit), `csv_semicolon_fields_are_quoted_and_parse_back_as_one_field`, plants, harness, cross product |
| 33 | U+200B left unescaped | `safe_text_escapes_controls_c1_bidi_separators_quote_and_backslash`, `text_has_no_forbidden_character_or_phrase` |
| 34 | tag range stops at U+E007E | `safe_text_escapes_...` only |
| 35 | the residual also sums unreliable files (V24) | `summary_residual_counts_the_reliable_files_only_when_an_unreliable_file_has_findings` only |
| 36 | text excerpt cut after escaping (V8) | `excerpt_content_is_the_expected_escaped_text_in_every_format` only |
| 37 | `Debug` prints the bytes | `debug_of_an_item_and_an_audit_never_prints_the_payload` |
| N1 (mine) | tag characters written with 4 hex digits | `safe_text_escapes_...` |
| N2 (mine) | the harness's quoting rule forgets `;` | `each_render_guard_goes_red_alone_when_planted`, harness, cross product |
| N3 (mine) | formula `'` written outside the quotes (`'"=a,b"`) | `csv_formula_guard_prefixes_each_trigger_character_on_hostile_fields_only` (unit), harness |
| N4 (mine) | the naive `safe_text` forgets the tag range | `the_naive_safe_text_matches_the_dec_010_examples` only. No committed seed holds a tag character, so the harness itself does not notice; it fails only when the fuzzer produces one. |

#### Gates (at `e093e57`, all exit 0)

- fmt on the workspace, the fuzz graph and the three harness files.
- `just lint`, `lint-fuzz`, `lint-fuzz-red-proof`.
- `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on stable (524 passed, 0 failed, 7 ignored) and on `+1.90.0` (524 / 0 / 7). The golden files match the current output (`golden_text_json_csv_match_byte_for_byte` passes), and so does the 18-seed `audit_render_seeds_are_committed_and_have_the_shapes_they_claim`.
- `cargo run --example fuzz_seeds` leaves `git status --porcelain` empty, so all 18 `audit_render` seeds and every other seed are reproduced byte for byte.
- `just fuzz-smoke audit_render`, 60 s: 65,811 runs, `DONE cov: 4576 ft: 15465 corp: 969/231Kb`, clean.

### Cost self-report

- **Tokens (total):** null. This verifier sub-agent has no token meter in its interface; the orchestrator transcribes the harness's metered figure.
- **Estimated USD:** null
- **Duration (minutes):** about 75 (estimate)
- **Source of the number:** none available to the agent (harness report via the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none (`decisions/` untouched).
- **Deviations from spec:** none by the verifier.
- **Follow-up work identified:** F1 and F2 as DEC-010 questions (revisit trigger). F3, F4 and F8 as test additions, which could ride with SPEC-016. F5 as a small `Item` `Debug`/doc fix. F6 as a CI timing check. F7 carried to SPEC-016 (streaming or an output cap).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Very little. The brief's list of own-mutant ideas was concrete. The only friction was that a worktree-isolated agent cannot run git in a scratch copy, so mutants were restored by hash instead.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-010's injection model assumes a comma separator and a "controls only" escape list. Locale separators and invisible characters belong in its threat list.

3. **If you did this task again, what would you do differently?**
   — Start the mutant batch in a scratch copy first (it takes about 40 minutes) and do the hand review while it runs, which is what worked here. I would also write the semicolon and zero-width probes before reading the code, not after.
