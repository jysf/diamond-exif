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
  id: HANDOFF-060
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-031

project:
  id: PROJ-001
  stage: STAGE-008
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number,
# and say in `basis` how you got it:
#   Claude Code   → your session transcript, measured per docs/cost-tracking.md
#                   "Measuring a cycle": count each message.id once, take its
#                   largest output_tokens, include subagents/ (basis: transcript).
#                   Summing every line over-counts ~2×. `/cost` in the desktop
#                   Code tab reports context size, not spend.
#   API           → the `usage` objects, summed (basis: api-usage)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed                # completed | blocked | rejected
  agent: claude-sonnet-5-5                    # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: estimate                  # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-031-png-parity
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "tokens_total is null: a sub-agent cannot read its own transcript token counts; the orchestrator replaces it with the harness figure. About 75 tool calls."  # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-060: Build SPEC-031 PNG read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-031` to `claude-sonnet-5-5` (implementer) for the **build** cycle. The spec is the contract: read it whole first (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-031-png-and-webp-read-parity-against-exiftool-comparator-red-pro.md`). This is a test-and-evidence spec that mirrors SPEC-011 (JPEG parity): the pattern is `tests/jpeg_oracle.rs`, `tests/support/oracle_jpeg.rs`, `tests/support/oracle.rs`, `tests/support/tools.rs`, `examples/oracle_record.rs`, `tests/mutation_oracle.rs`, `tests/support/probe.rs`. Read them before writing anything; do not reinvent. DEC-017 and DEC-018 describe the readers under test; their ExifTool claims are leads, not evidence: the committed records are the evidence.

## FIRST STEP (the harness may start your worktree from main, which lacks the design)

Run `git checkout -b claude/spec-031-png-parity claude/spec-031-design` (the local branch exists in this repository and holds the spec and this handoff). Confirm `git log --oneline -1` and that this file exists. Do not use `reset --hard`.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**, with two checkpoints: (1) fixtures, `oracle_png.rs`, recorder, records, replay and live tests, divergences, questions, warning table: gates, COMMIT; (2) the mutation oracle and its red-proofs: gates, COMMIT. If past about 60 percent of budget at checkpoint 1, hand back after it listing checkpoint 2 as the remainder.
- Read large files with offset/limit (the JPEG oracle is 1,540 lines: read its structure and the pieces you mirror, not all of it). Prefer one write per file.
- Record `tokens_total: null` with a note if you cannot measure it; the orchestrator replaces it with the harness figure. Never invent one.
- Commit on your branch. Do NOT push, do NOT open a PR, do NOT touch main. No new dependency.
- ExifTool 13.55 is installed; run it only through the repo's `run_tool_with_limit` helpers in tests; for exploratory probing use a timeout-safe approach (macOS has no `timeout`: use `perl -e 'alarm N; exec @ARGV'` or the Rust helper). Never read or link ExifTool's source.
- Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. BSD `sed -i` rejects multi-expression use: use the edit tool or Python. `indexing_slicing` is denied crate-wide in `src/` (tests may index). `fuzz/Cargo.lock` is rewritten by fuzz builds: restore it, do not commit it. `tests/release_layout.rs` pins `[Unreleased]` in CHANGELOG empty: add no CHANGELOG entry. When scripting edits on spec files, the glob also matches `-timeline.md`: use exact names.

## Work items (in this order)

1. Read the pattern files; list the 20 selectors and the region selectors; decide the PNG region selectors from measurement.
2. **Tests first**: the fixture list and comparator tests in `tests/png_oracle.rs` with `tests/support/oracle_png.rs`; see them red for the right reason (no records yet).
3. Probe each hostile shape under the time limit; extend `oracle::` (`read_ours_png`, PNG region selectors, `compare_png`), `examples/oracle_record.rs`, `tools.rs` if needed; run `just oracle-record`; check idempotence (run twice, no diff).
4. Measure then model: the several-Exif rule, raw-profile and compressed-XMP divergences with `Divergence` entries and `guidance/questions.yaml` entries; the warning table.
5. **Gates, COMMIT (checkpoint 1).**
6. The mutated-copy red-proofs (at least six library faults and two byte mutations; control first; injection marker not leaking). **Gates, COMMIT (checkpoint 2).**
7. Final gates: `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe`; `just oracle-record` twice with no diff.

## Expected Deliverables

Commits on the branch; every acceptance criterion met or listed as not met with the reason; spec ticks; the handback filled, including every divergence found (with its question id) and every choice made where the spec is silent. If parity exposes a reader bug, fix it with a regression test and say so loudly.

## Out of Scope

WebP parity (SPEC-034), audit wiring (SPEC-032), decoding or inflating in the library, any DEC change (raise it instead), JPEG or TIFF oracle behaviour changes, PR creation.

## Return Criteria

Fill the `handback:` front matter AND the `## Handback` section; set `handoff.status` and `handback.status` to `completed`. If you cannot complete: say exactly what is done and what blocked you, set both to `rejected`/`blocked`, and still report tokens.

---

## Handback` section below.** An unfilled handback is an
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

- **Branch / PR:** `claude/spec-031-png-parity` (two commits on top of a04413d); no PR, not pushed.
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** yes, all eleven (ticked in the spec).
- **What was built:** `tests/support/oracle_png.rs` (independent builders, CRC-32, Adler-32, stored-deflate zlib; 12 wrapped TIFFs + 37 shapes = 49 fixtures), `tests/png_oracle.rs` (22 tests), `tests/png_mutation_oracle.rs` (7 library mutants p1-p7, 2 byte mutations, controls, leak test, skip test), PNG section of `tests/support/oracle.rs` (`read_ours_png`, `PNG_REGION_SELECTORS`, `compare_png`, `EXIF_UNREAD`, `PNG_PINS`, `png_warning_lines`, 9 `KNOWN_DIVERGENCES`), PNG branch of the probe, `record_png` in the recorder, 117 records + `png-warnings.txt`.
- **No change to `src/`.** No reader bug was exposed.

### Cost self-report

- **Tokens (total):** null (not measurable from inside the session; orchestrator to replace with the harness figure)
- **Estimated USD:** null
- **Duration (minutes):** not measured
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - The XMP selector means READABLE XMP (an uncompressed `XML:com.adobe.xmp` text we can read), not located XMP. Defined by location, compressed XMP would agree with ExifTool and the spec's "compressed XMP is a divergence" would have no entry. Location is asserted separately in `xmp_keyword_rules_agree`. The ICC selector is by location (`PNG:ProfileName`, which ExifTool prints for any `iCCP`, valid or not; `ICC-header:*` is `-` for a bad header).
  - Extra record `png-warnings.txt` (second ExifTool run `-a -G1 -s -n -Warning -Error` on 20 fixtures): a `-T -s3` run prints warnings to stderr only when no requested tag is found, so the per-fixture records cannot carry ExifTool's warnings for the shapes the warning table needs.
  - Recorded stderr is restricted to `Warning:`/`Error:` lines (`oracle::png_warning_lines`): ExifTool prints Perl diagnostics with the host's Homebrew path for `meta-iccp-no-method` and `meta-ztxt-no-method`.
  - The mutation red-proofs are a separate file (allowed). `app.just` `mutation-oracle` now runs both files. Three existing record-set checks (`jpeg_oracle`, `budget_oracle`, `inventory_oracle`) were widened to ignore `png-*` names (one `.filter` each); no existing record changed.
- **Follow-up work identified:** none new beyond SPEC-034 (WebP) which reuses this apparatus; `png-warnings.txt` is a record shape SPEC-034 may want too.

### Divergences found (every one is in `guidance/questions.yaml`)

1. `png-raw-profile-not-decoded`: ExifTool 13.55 decodes `Raw profile type exif` and `xmp` (tEXt, zTXt, iTXt): all 14 Exif values and `XMP-xmp:CreatorTool`; we locate only (DEC-018). Entries: `png-raw-exif-text|ztxt|itxt` (wildcard `exif:unread`, ExifTool side pinned to the bare-TIFF record), `png-raw-xmp-text|ztxt|itxt`, and `png-exif-and-raw-exif` (IFD0:Make: ExifTool merges and the raw profile wins, RawCam; we read the eXIf, AlphaCam).
2. `png-compressed-xmp-not-inflated`: `png-xmp-itxt-compressed`, `png-xmp-ztxt`: ExifTool inflates, we locate (`compressed = true`).
3. No non-terminating shape: all 49 fixtures and all 68 `png_chunks` seeds returned in under 2 s under an 8 s alarm (probe run before recording). So no `"*"` entry, no not-run record.

### Measured facts worth the orchestrator's attention

- **Several `eXIf`: ExifTool uses the FIRST block that is a TIFF and ignores later ones** (warning "IFD0 pointer references previous IFD0 directory"); tags are NOT merged. This is the opposite of the JPEG rule (merge, later wins). It equals the library's own `PngMetadata::exif()` (first block that parsed), so no harness model beyond that was needed; `several_eXIf_model_is_measured_and_pinned` pins it with five fixtures and shows the JPEG rule would be named wrong. Not a divergence. A fatal block is skipped in either order ("Invalid eXIf chunk").
- ExifTool ignores the `eXIf` CRC (`png-exif-bad-crc` reads normally, silent); we report `BadChunkCrc` but read too. It also reads an `eXIf` after `IEND`, before `IHDR`, and with the `Exif\0\0` prefix (warns "Improper Exif00 header"), all agreeing with us.
- Wrapping any of the 12 TIFF fixtures in an `eXIf` chunk changes none of ExifTool's 20 values and none of ours (container independence, live and recorded).
- Every wrapped fixture and every other shape agreed first time: no unexplained mismatch on the 68 seeds.

### Choices where the spec is silent

- `png-keyword-79` added beside `png-keyword-80` (the boundary); `png-xmp-itxt-flag-2`, `png-xmp-after-idat`, `png-truncated-between-ihdr-and-idat`, `png-exif-good-then-fatal`/`-fatal-then-good` added from measurement.
- Seeds are compared on the 20 Exif selectors only (as for JPEG); regions only on fixtures.
- The raw-profile body is built ImageMagick-style (`\nexif\n<len>\n<hex>`, with an `Exif\0\0` prefix on the data); one layout only.
- Mutants chosen: p1 TIFF offset +1, p2 first block not primary (`insert(0, ..)`), p3 `Exif\0\0` prefix not skipped, p4 trailer scan disabled, p5 XMP keyword routing broken, p6 `iCCP` not located, p7 `iTXt` compression flag ignored (makes a recorded divergence disappear: reported as `stale`, plus the invalid flag-2 shape named); byte mutations b1 GPS pointer inside the `eXIf` payload (CRC recomputed with the oracle's own CRC-32), b2 truncation after the `eXIf` chunk (typed outcome) and inside it (named, live-checked).

### Gate results

`cargo fmt --check`, `cargo clippy --all-targets -D warnings` (via `just lint`), `cargo test` (all binaries, includes `png_oracle` 37 incl. support tests), `just mutation-oracle` (58 + 39 passed), `just validate`, `just cost-audit`, `just forbid-unsafe`: green. `just oracle-record` run three times: no diff after the first (new files only; no existing record changed). Live runs were against the installed ExifTool 13.55 and passed; skip-is-loud and `DIAMOND_EXIF_REQUIRE_ORACLE=1` tested in both files. Not run: fuzz smoke, `lint-red-proof` scripts (nothing in `src/` or fuzz changed).

### Surprising

- ExifTool prints `Raw profile` Exif as a normal `IFD0` group, indistinguishable from eXIf, and `PNG:ProfileName` for an `iCCP` whose ICC header is garbage (`Bad length ICC_Profile` warning but the name is still read).
- `-T -n -s3` on an unknown signature still prints a `-\t-` line on stdout (so no special case in `parse_fields`).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   The "region presence" wording for compressed XMP: by location there is no divergence, so I had to define the selector as readable XMP to honour the pinned-divergence criterion; worth a sentence in the spec.
2. **Was there a constraint or decision that should have been listed but wasn't?**
   Three existing record-directory checks (`jpeg_oracle`, `budget_oracle`, `inventory_oracle`) enumerate the whole directory and break when a new record family arrives; SPEC-034 will hit the same.
3. **If you did this task again, what would you do differently?**
   Write the shared `Divergence` wildcard (`exif:unread`) first; run the whole `cargo test` earlier, since the directory-enumeration failures only showed at the end.
