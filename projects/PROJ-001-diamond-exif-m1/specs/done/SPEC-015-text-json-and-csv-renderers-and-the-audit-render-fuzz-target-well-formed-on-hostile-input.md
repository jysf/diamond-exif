---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-015
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
  stage: STAGE-003
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-008, DEC-009, DEC-010]
  constraints: [panic-free-library, forbid-unsafe-code, no-io-in-library, fuzz-target-per-surface, test-before-implementation, oracle-must-be-shown-red, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-012, SPEC-013, SPEC-014, SPEC-016]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-012, SPEC-014]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-003's <capability>". Optional; null is acceptable.
value_link: "STAGE-003: a report a person or CI job can read, safe for any input bytes, that never calls an unreadable or unsupported file clean and prints no personal values unless asked"

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
  tokens_estimate: 650000
  sessions:
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 586061
      estimated_usd: 3.87
      duration_minutes: 80
      recorded_at: 2026-11-06
      notes: "tokens transcribed by the orchestrator from the sub-agent metered results (cumulative over the build run and three follow-ups; final figure 586,061); see HANDOFF-031"
    - cycle: verify
      agent: claude-opus-5-5
      interface: other
      tokens_total: 341485
      estimated_usd: 2.25
      duration_minutes: 27
      recorded_at: 2026-11-06
      notes: "verify APPROVED at 6723aad; delta verify of follow-up 2 PUNCH LIST at e093e57 (F6 timing bound flaky under load); delta verify 2 of follow-up 3 APPROVED at 9c44a21 (final). Tokens transcribed by the orchestrator from the sub-agent metered results; the agent was resumed twice and each completion reported a cumulative figure: 268,724, 322,683, 341,485 (final, used); wall-clock 1,064,494 + 371,651 + 209,131 = 1,645,276 ms = 27 minutes; the verifier's own 75-minute estimate is not used" # one line if unusual (rework, no meter, etc.)
  totals:
    tokens_total: 927546
    estimated_usd: 6.12
    session_count: 2
shipped_at: 2026-11-06
---

# SPEC-015: Text, JSON and CSV renderers and the `audit_render` fuzz target (well-formed on hostile input)

## Context

STAGE-003 has an audit (SPEC-012), an inventory checked against ExifTool (SPEC-013) and a policy preview (SPEC-014). Nothing yet turns them into something a person or a CI job can read. This spec adds three **pure** renderers over a list of per-file results, plus the escaping and summary they share, and a new fuzz target, because the report text is attacker-controlled bytes: paths, value excerpts and failure messages end up inside a JSON document read by a CI job, a CSV opened in a spreadsheet, and text printed to a terminal.

DEC-010 (written in this design, normative; read it first) decides the contract: reports are **value-free by default** (values only with an explicit option, as a bounded raw excerpt); one **display-safe text** function escapes controls, bidi and invalid UTF-8 before any format-specific escaping; JSON is one document with a pinned schema; CSV is RFC 4180 with a spreadsheet-formula guard; and every file has exactly one `status` of which `ok` is emitted only for a clean audit. SPEC-016 (the binary) supplies paths and I/O failures and turns the summary into exit codes; this spec does no I/O.

Two library changes are needed and nothing else in `src/` moves: a new module `src/report/`, and `Item::value()` in `src/audit/mod.rs` (the audit already holds the container when it creates an item; the borrowed payload slice uses the lifetime SPEC-012 reserved for exactly this). The same change carries the findings SPEC-013 and SPEC-014 verifies left for this spec (below).

## Goal

`render_text`, `render_json` and `render_csv` over `&[FileEntry]` and `&RenderOptions`, each returning a `String` that is well-formed for **any** input bytes (proved by independent checkers, in unit tests and in a new `audit_render` fuzz target), deterministic, with pinned field order, the DEC-010 escaping, privacy default and status rules; plus `summarize`, the run summary the binary reads. The carried findings F1 to F3 (SPEC-014) and the linearity weakness (SPEC-013) are closed.

## Inputs

- **Reads:** DEC-010 (normative), DEC-009 (dispositions, `reliable`, the amendment's wording rules), DEC-008, `src/audit/{mod,preview,category}.rs`, `src/policy.rs`, `src/tiff/mod.rs` (`Container::payload`, `IfdKind`), `fuzz/shared/audit_invariants.rs`, `fuzz/fuzz_targets/audit.rs`, `tests/fuzz_layout.rs`, `tests/audit_model.rs`, `examples/fuzz_seeds.rs`, `.github/workflows/ci.yml`, `fuzz/Cargo.toml`, `justfile`/`app.just` (`fuzz-smoke`).
- **Does not read:** ExifTool source. ExifTool is **not run** by this spec (the report format is this tool's own contract).
- Constraints `panic-free-library` (five-lint deny set), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red` (applied to our own tests: every mutant below, including mutants of the independent checkers), `no-new-top-level-deps-without-decision`. No dependencies.

## Outputs

### Public API (new module `exif_core::report`, files `src/report/{mod,safe,text,json,csv}.rs`; `pub mod report;` in `src/lib.rs`)

```rust
pub struct RenderOptions { pub policy: Option<StripPolicy>, pub inventory: bool, pub values: bool }
impl Default for RenderOptions { /* policy None, inventory false, values false */ }

pub enum FailReason { Unreadable, TooLarge, NotRegularFile, Other }      // name(): "unreadable" "too_large" "not_regular_file" "other"
pub enum FileResult<'a> { Audited(FileAudit<'a>), Failed { reason: FailReason, detail: &'a [u8] } }
pub struct FileEntry<'a> { pub label: &'a [u8], pub result: FileResult<'a> }

#[non_exhaustive] pub enum Status { Failed, Unsupported, Unreadable, Issues, Ok }   // name(): "failed" "unsupported" "unreadable" "issues" "ok"
pub fn status_of(result: &FileResult<'_>) -> Status;                        // DEC-010 part 5, nothing else

pub struct Summary { pub files: usize, pub failed: usize, pub unsupported: usize, pub unreadable: usize,
                     pub issues: usize, pub ok: usize, pub findings: usize,
                     pub by_category: [(Category, usize); 13],               // Category::ALL order
                     pub max_severity: Option<Severity>,
                     pub residual_findings: Option<usize>,                  // Some only with a policy; sums reliable files only
                     pub reliable_files: usize, pub unreliable_files: usize }
pub fn summarize(files: &[FileEntry<'_>], opts: &RenderOptions) -> Summary;

pub fn render_text(files: &[FileEntry<'_>], opts: &RenderOptions) -> String;
pub fn render_json(files: &[FileEntry<'_>], opts: &RenderOptions) -> String;
pub fn render_csv(files: &[FileEntry<'_>], opts: &RenderOptions) -> String;
pub fn safe_text(bytes: &[u8]) -> String;                                    // DEC-010 part 2

// src/audit/mod.rs: the one addition
impl<'a> Item<'a> { pub fn value(&self) -> Option<&'a [u8]>; }              // raw payload of a Tag item; None for a Region or an unreadable payload
```

(Names and exact shapes are the builder's to refine inside these constraints; DEC-010, the schemas below and the named tests are normative. `Item::value` is the raw payload exactly as `Container::payload` returns it; it is never copied into the audit and never decoded.)

### Rules (normative)

1. **Value-free default.** With `values == false`, no output of any format contains any byte derived from a tag payload: JSON `value` is `null` for every finding and inventory item, the CSV `value` and `value_truncated` columns are empty, and text prints no excerpt. A test plants a distinctive payload (`SECRET-SERIAL-0042`) in a Make, an Artist and a BodySerialNumber and asserts it appears in none of the three outputs unless `values` is on, and then in all three, escaped.
2. **Excerpt.** With `values == true`, a Tag item with a readable payload gets an excerpt: for field type ASCII (2) the bytes up to the first NUL (or the whole payload if none), otherwise the payload; at most 64 bytes taken from the start, `truncated` true iff bytes were cut. The excerpt text is `safe_text` of those bytes.
3. **`safe_text`** is exactly DEC-010 part 2: the output has no character below U+0020 (all escaped), no DEL, no C1, no bidi control, no U+2028/2029, none of the invisible and format characters of DEC-010 amendment 2 (U+00AD, U+200B to U+200D, U+2060, U+FEFF, U+206A to U+206F, U+FFF9 to U+FFFB, U+E0000 to U+E007F), and no raw `"` or `\`; every invalid UTF-8 byte appears as `\xNN`; valid printable text (including non-ASCII letters) is unchanged; it is injective on the byte strings it covers (two different inputs never give one output) and total (no panic, linear time, output at most 4x the input plus constant).
4. **Status.** `status_of` follows DEC-010 part 5. `Ok` only for `health().clean`. An `Unsupported` kind is never `Ok`, even though it has no findings.
5. **JSON** (UTF-8, one document, keys in this order, no extra keys, no duplicates, numbers non-negative integers):
   `{"schema_version":1,"policy":str|null,"options":{"inventory":bool,"values":bool},"summary":{"files","failed","unsupported","unreadable","issues","ok","findings","by_category":{13 keys in Category::ALL order},"max_severity":str|null,"residual_findings":n|null,"reliable_files","unreliable_files"},"files":[ FILE... ]}`
   with `FILE = {"label":str,"status":str,"container":str|null,"failure":null|{"reason":str,"detail":str},"health":null|{"clean","jpeg_issues","exif_issues","failed_blocks","inventory_truncated"},"max_severity":str|null,"inventory_items":n,"findings":[FINDING...],"inventory":null|[ITEM...],"preview":null|PREVIEW}`;
   `FINDING = {"category","severity","at":WHERE,"block":n|null,"segment":n|null,"after_scan":bool,"disposition":str|null,"thumbnail":null|{"offset":n|null,"length":n|null,"starts_with_ffd8":bool},"value":null|{"text":str,"truncated":bool}}`;
   `ITEM = {"at":WHERE,"block","segment","after_scan","category":str|null,"value":...}` (an inventory item that has a finding names its category);
   `WHERE = {"kind":"tag","ifd":str,"ifd_index":n,"tag":"0xNNNN","field_type":n,"count":n}` or `{"kind":"region","region":str,"segment":n,"len":n}`;
   `PREVIEW = {"policy":str,"counts":{"removed","kept","kept_unread"},"reliable":bool,"residual_findings":n|null,"residual_severity":str|null}` where both residual fields are `null` unless `reliable` is true (DEC-009 amendment);
   `ifd` names: `ifd0 ifd1 chain sub exif gps interop other`; `container`: `tiff jpeg` or the sniffed name (`png webp heic gif bigtiff unknown empty`), `null` for a failed file. `disposition` is `null` unless a policy is set.
6. **CSV** (RFC 4180, CRLF): header `label,status,container,category,severity,kind,ifd,ifd_index,tag,field_type,count,region,region_len,block,segment,after_scan,disposition,value,value_truncated`. Rows: one per finding; with `inventory` on, one per inventory item (category and severity empty for an item with no finding) followed by one per finding that has no item; and **one row for every file that has no row otherwise** (label, status, container only; the rest empty), so no file is ever absent. Every row has exactly 19 fields. A field is quoted iff it contains a comma, semicolon, quote, CR or LF (DEC-010 amendment 2); quoting and the formula guard are DEC-010 part 4; the guard applies to `label`, `value` and the failure detail (carried in `value` for a failed file's row) and to nothing else.
7. **Text** is line-oriented, UTF-8, `\n` line ends, no character below U+0020 other than `\n`, no bidi, C1 or DEL characters. Required content: per file the label (via `safe_text`), the status, the container, a findings count (a zero is printed as `0 findings`), each finding with category, severity, location and (with a policy) disposition worded `removed`, `kept (not targeted by this policy)`, `kept_unread (may hold what this policy removes)`; the preview's residual numbers only when `reliable`, otherwise the line `preview not reliable: the file was not read cleanly`; and a final summary block. The text never contains the phrases "no personal data", "safe" or "clean file". The exact layout is pinned by the golden files below.
8. **Determinism and bounds.** Output is a pure function of the arguments; same input, same bytes. Work and output size are linear in `sum(items + findings)`; per item the output is bounded (the label and excerpt are bounded, 64 bytes of excerpt after escaping at most 4x). The build measures and reports the largest output for one file at the audit caps (`MAX_INVENTORY_ITEMS` items with `inventory` and `values` on).
9. **No I/O, no dependencies, no panics.** Pure functions returning `String`.

### Golden files

`tests/golden/report-full-house.{txt,json,csv}` for the full-house JPEG of SPEC-014 (one Exif block with all seven tag categories, a MakerNote and an IFD1 thumbnail, and one region each of Xmp, Iptc, Icc and Jumbf), label `photos/full-house.jpg`, policy `all_pii`, `inventory` off, `values` on, plus a second entry for an unsupported PNG signature and a third `Failed` entry. The tests compare byte for byte. Regenerating a golden file is a deliberate edit reported in the diff, never automatic.

### The independent checkers

`fuzz/shared/render_check.rs` (text included at module level like `audit_invariants.rs`; not a crate root; formatted by hand): a strict **JSON parser** (RFC 8259: no trailing commas, no leading zeros, no raw control characters in strings, valid escapes only, `\u` surrogate pairs well-formed, input must be valid UTF-8, depth limit 32, duplicate keys rejected, nothing after the value) producing a small value tree, and a strict **CSV parser** (RFC 4180 with CRLF, doubled quotes, quoted fields may hold CR/LF, no bare quote in an unquoted field, a final CRLF). It is written without calling anything in `src/report` and carries its own table-driven tests (`tests/render_check.rs`: a list of valid and invalid documents each way, so a lenient checker is itself shown red). A naive second implementation of `safe_text` (char by char, from the DEC-010 list typed again) lives in the same file for the round-trip guards.

### Fuzz: `audit_render`

New target `fuzz/fuzz_targets/audit_render.rs` (`[[bin]]` in `fuzz/Cargo.toml`, a matrix entry `audit_render` in the CI `fuzz-smoke` job, `just fuzz-smoke audit_render` works). Input layout: byte 0 selects the options (low 3 bits: policy none, `Gps`, `GpsAndDevice`, `AllPii`, `All`, `Everything`, a `Custom` derived from the next input byte, none; bit 3 `inventory`; bit 4 `values`; bit 5 selects the `FailReason`), byte 1 is the label length `L`, the next `L` bytes are the label, and the rest is the file. The target renders two entries: `Audited(audit(file))` with that label, and `Failed` with the same label and a detail made of the first 64 bytes of the file. Invariants live in `fuzz/shared/render_invariants.rs` (shared with `tests/fuzz_layout.rs`, same pattern as `audit_invariants.rs`), as named guards run **alone** through `run_guard`, each `iff` checked in both directions, each re-derived independently:

- `json_wellformed`: the strict parser accepts the output.
- `json_shape`: key order and key set exactly as rule 5 at every level; enumerations only from the allowed sets; numbers non-negative integers.
- `json_facts`: file count, status (re-derived from `kind()` and `health()` here, not via `status_of`), finding count and category sequence, location fields, dispositions (re-derived from the DEC-009 grid typed again), the preview's null-unless-reliable rule, and label text equal to the harness's naive `safe_text`.
- `csv_wellformed` and `csv_rows`: the strict parser accepts; header exact; every row 19 fields; row count equals the rule 6 expectation; the first character of every unquoted `label`/`value` field is not one of the formula characters; the number of rows per file is at least one.
- `text_safe`: no forbidden character; deterministic (rendered twice, equal); contains none of the forbidden phrases; every file label's `safe_text` appears.
- `values_gate`: with `values` off no excerpt text appears in any format; with it on, each excerpt is at most 64 source bytes and `truncated` iff the source was longer (re-derived from the container's payload).
- `summary_facts`: the summary equals the re-derived counts; `reliable_files + unreliable_files == files` over audited files.
- `render_linear`: output length and step count are linear in items plus findings.

Seeds in `fuzz/seeds/audit_render/` (generated by `examples/fuzz_seeds.rs`, committed): at least 14: the two full-house inputs under several option bytes; labels with control characters, quotes, a comma, CR LF, invalid UTF-8, a right-to-left override, `=cmd|' /C calc'!A0`, `@SUM(1)`, a 255-byte label, an empty label; a payload of 300 bytes in an ASCII tag; a PNG signature; an empty file. The two full-house inputs are also added to `fuzz/seeds/audit` (76 to 78 seeds; update `tests/audit_model.rs` and the seed hash lists in the same change; **carried F1**).

### Carried findings closed here

- **SPEC-014 F1** (above): the two full-house seeds in `fuzz/seeds/audit`. A plant of the old mutant 9 (`may_hold(MakerNote)` loses `Gps`) is run against the real `audit` target and must be found within the 60 s smoke, or the build reports how long it took.
- **SPEC-014 F2:** a unit or integration test builds the audit of an input `audit()` really truncates (the `over` input of the cap-boundary test) and checks the preview's truncated branch end to end: residual severity equals the audit's `max_severity` for every non-`Everything` preset and `None` for `Everything`/`Custom(all)`, `reliable` false, counts equal the stored findings. The old mutants 15, 16 and the harness's truncated `all()` exception (vM) are applied against it and caught.
- **SPEC-014 F3:** the preview harness asserts its `Custom` case is non-trivial on at least one input of the layout test (a set neither empty nor `all()`), and the old vG mutant is applied and caught.
- **SPEC-013 linearity:** `audit_harness_guards_are_linear_in_the_input` counts only where a `tick()` sits, so a guard can stay linear in the counter yet be quadratic in time. Strengthen it with a wall-clock bound on a large input (the bound chosen so the quadratic rescan mutant exceeds it by at least 20 times on this host, measured and reported; the bound itself generous enough not to flake on a slow CI runner) in addition to the counter. The rescan-without-tick mutant of the SPEC-013 verify is applied and caught.
- The two remaining SPEC-013 low findings (the inventory Class H region-kind check, ExifTool's exit status in the records) are oracle items with no rendering link: they move to question `inventory-oracle-low-findings` in this design and leave this spec.

## Acceptance Criteria

- `exif_core::report` and `Item::value` exist as specified; rules 1 to 9 hold.
- For every committed seed of all four fuzz targets plus the new `audit_render` seeds, all three renderers produce output that the strict checkers accept, under all six policies and all four `inventory`/`values` combinations (a test over the cross product), with the JSON and CSV facts guards green.
- The golden files match byte for byte; the value-free test (rule 1) passes both ways; the formula guard, the bidi/C1/DEL escapes and the invalid-UTF-8 `\xNN` form are pinned by named tests.
- The checkers reject every invalid case in their own table and accept every valid one.
- `audit_render` exists with the seeds above; 60 s smoke clean (executions and coverage recorded); one guard plant against the real target (found at once); CI matrix and `just fuzz-smoke audit_render` updated; `lint-fuzz-red-proof` run locally.
- Every mutant below is applied by hand, shown to fail at least one named test or guard (the report names which), and reverted; the tree after reverting is byte-identical to the build commit.
- `audit()` behaviour is unchanged (`Item::value` is additive; `Item` equality, ordering and the container-independence tests still pass); `decisions/`, `scripts/` and irradiance untouched; no dependencies; existing seeds and records byte-identical except the two new `audit` seeds and their hash-list entries.
- Gates: fmt (both graphs and the harness files), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `fuzz-smoke audit` and `audit_render` (60 s each), `oracle-record` with a clean `git status`.

## Failing Tests

Named tests (a rename, fold or missing test is a deviation to report):

**src/report (unit):**
- `safe_text_escapes_controls_c1_bidi_separators_quote_and_backslash`, `safe_text_writes_invalid_utf8_bytes_as_hex_escapes`, `safe_text_keeps_valid_printable_unicode`, `safe_text_is_injective_on_a_sampled_set`, `safe_text_output_bound_holds`
- `status_of_follows_dec_010` (each of the five; `ok` only for a clean audit; an unsupported audit with zero findings is `unsupported`)
- `excerpt_cuts_ascii_at_nul_and_caps_at_64_bytes_and_flags_truncation`, `excerpt_of_a_non_ascii_type_is_the_raw_payload_prefix`
- `values_are_absent_unless_asked` (the `SECRET-SERIAL-0042` test, three formats, both ways)
- `json_has_the_pinned_key_order_at_every_level`, `json_enumerations_are_pinned`, `json_escapes_quote_backslash_and_every_control`, `json_preview_residuals_are_null_unless_reliable`, `json_summary_matches_summarize`
- `csv_has_the_pinned_header_and_19_fields_per_row`, `csv_quotes_comma_quote_cr_lf_and_doubles_quotes`, `csv_uses_crlf`, `csv_formula_guard_prefixes_each_trigger_character_on_hostile_fields_only`, `csv_has_a_row_for_every_file_without_findings`, `csv_inventory_mode_rows`
- `text_has_no_forbidden_character_or_phrase`, `text_words_kept_and_kept_unread_as_dec_009_requires`, `text_prints_zero_findings_not_silence`, `text_preview_not_reliable_line`
- `summarize_counts_each_status_category_and_reliability`
- `render_is_deterministic`
- `item_value_is_the_raw_payload_and_none_for_regions_and_unreadable_payloads` (src/audit)

**tests/report_render.rs (integration):**
- `golden_text_json_csv_match_byte_for_byte`
- `every_seed_every_policy_every_option_renders_well_formed_output` (checkers on the cross product, all four seed directories)
- `a_truncated_audit_previews_conservatively_end_to_end` (carried F2)
- `largest_output_at_the_caps_is_measured` (`#[ignore]`d printing the size; a bounded version asserts a ceiling)

**tests/render_check.rs (the checkers tested):**
- `json_checker_accepts_valid_documents_and_rejects_each_invalid_one` (trailing comma, leading zero, raw control character, bad escape, lone surrogate escape, invalid UTF-8, duplicate key, depth 33, trailing data, empty input)
- `csv_checker_accepts_valid_documents_and_rejects_each_invalid_one` (ragged row, bare quote, unterminated quote, missing final CRLF, LF-only line end)

**tests/fuzz_layout.rs and the harness:**
- `render_harness_holds_on_every_seed_and_degenerate_inputs`
- `each_render_guard_goes_red_alone_when_planted` (table-driven, both directions of every iff, at least two plants per guard)
- `render_guards_are_linear` and the strengthened `audit_harness_guards_are_linear_in_the_input` (counter plus wall-clock bound)
- `audit_render_seeds_are_committed_and_have_the_shapes_they_claim`
- `the_two_full_house_inputs_are_audit_seeds` and the updated seed count and hash assertions (78)
- `preview_custom_case_is_non_trivial` (carried F3)

## Mutants (each shown red by hand, then reverted)

Library: 1. `safe_text` leaves `"` unescaped. 2. `safe_text` leaves DEL. 3. `safe_text` leaves C1 controls. 4. `safe_text` leaves U+202E. 5. `safe_text` replaces invalid bytes with U+FFFD. 6. JSON string writer skips `\u00XX` for controls below U+0020 other than `\n`. 7. JSON key order swaps `status` and `label`. 8. JSON emits `status: "ok"` for an audit with issues. 9. JSON `preview` residuals printed when not `reliable`. 10. JSON summary double-counts `unsupported` into `issues`. 11. JSON numbers in `by_category` out of `Category::ALL` order. 12. CSV does not double quotes. 13. CSV uses LF. 14. CSV does not quote a field containing CR. 15. CSV formula guard omits `@`. 16. CSV formula guard also applied to the `category` column. 17. CSV omits the row for a file with no findings. 18. CSV inventory mode drops items without findings. 19. Text prints `kept` as "contains none of it". 20. Text prints an excerpt with `values` off. 21. Excerpt cap 65. 22. `truncated` computed on the escaped length. 23. `values` default true. 24. Failed file's label not passed through `safe_text`. 25. `status_of` returns `Ok` for `Unsupported`.

Checkers: 26. JSON checker accepts a trailing comma. 27. JSON checker accepts a leading zero. 28. JSON checker accepts a raw control character in a string. 29. CSV checker accepts a ragged row. 30. CSV checker accepts a bare quote in an unquoted field.

Carried: the SPEC-014 mutants 15, 16, vG, vM and the SPEC-013 rescan-without-tick, applied against the new tests as stated above.

## Non-Goals

- Reading or writing files, directory walking, argument handling, exit codes (SPEC-016)
- Decoding values (rationals, dates, GPS degrees) and tag names (questions `report-value-decoding`, `report-tag-names`)
- Colour, pretty-printing options, JSON Lines, a streaming interface, SARIF or other formats
- ExifTool comparison of report content
- Any change to `audit()` results, the presets or the preview

## Notes for the Implementer

- Order of work, test-first: `safe_text` and its tests; the checkers and their tables (test the testers before using them); `Item::value`; `status_of` and `summarize`; JSON; CSV; text and the golden files; the render guards and plants; the `audit_render` target, seeds and CI entry; the carried items; then the mutants.
- `arithmetic_side_effects` forbids unchecked `+`, `*`, `<<`: use `saturating_add`, iterators and `checked_*`. Build output with `String::push_str` and `push`; never index a string by byte position.
- Type DEC-010's escape list, the JSON key lists and the CSV header **from the DEC and this spec by hand** in the harness; none may be imported from `src/report`.
- Probe the worst-case output size (rule 8) early; if it is large enough to matter for a binary, report it rather than silently capping.
- Run `lint-fuzz-red-proof` locally; it broke PR 14.
- Report in the Handback: every deviation, each mutant with the test or guard that caught it, fuzz smoke numbers for `audit` and `audit_render`, the wall-clock bound chosen and the measured quadratic-mutant time, the largest output size, and any doubt about a DEC-010 rule (a doubt is a question for me: do not touch `decisions/`).
- Do not push or open a PR; do not advance cycles.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — Put a security review of the escaping into the design, not only into verify: the cold verifier found a real CSV bypass (a semicolon locale splits an unquoted field into a formula cell) and invisible characters that spoof a name, both of which the DEC's own list should have named. I would also have pre-computed the output-size worst case (CSV with inventory on, 131,072 rows) and specified the timing test as a ratio from the start; the absolute 55 ms bound flaked under load.

2. **Does any template, constraint, or decision need updating?**
   — DEC-010 gained two amendments (summary residual needs a reliable file, DEL form, caller-supplied label and detail, Custom naming; then semicolon quoting and the wider invisible-character escape set). Lesson, N=2 with SPEC-014 (a guard is only as strong as what its inputs reach): a harness over a classification needs a seed for every class, here the tag-character range and DeviceId/MakerNote. Added to `guidance/signals.yaml` as a watch lesson.

3. **Is there a follow-up spec I should write now before I forget?**
   — No new spec. SPEC-016 must know: the renderers return a `String` (72 MB JSON, 48 MB text, about 44 MB CSV at the caps), label and detail come from the binary and must be OS text, never file bytes, and `Custom` policies are unreachable from the CLI. Questions carry value decoding, tag names and custom-policy identity.

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — verify
   *(The CSV semicolon bypass and the invisible-character gap were found by the cold verifier; the build's own flaky timing bound was found by the delta verify under load.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — Render an audit as text, JSON or CSV that stays well-formed and safe for any file name or tag value (before: no report at all; after: three renderers proved against independent strict checkers and a new `audit_render` fuzz target, values hidden unless asked, and a file never reported "ok" unless the audit was clean).
