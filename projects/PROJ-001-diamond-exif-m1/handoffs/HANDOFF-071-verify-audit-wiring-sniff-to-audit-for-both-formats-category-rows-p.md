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
  id: HANDOFF-071
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-22
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-032

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
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: punch-list              # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-032-verify
  pr: null
  completed_at: 2026-11-22         # YYYY-MM-DD
  notes: "tokens left null by instruction: the orchestrator records the harness subagent_tokens figure"
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-071: <Task Title — same as the spec's title>

## Delegation Summary

One sentence: `<from_agent>` (acting as `<from_role>`) hands `SPEC-032`
to `<to_agent>` (acting as `<to_role>`) for the **`<cycle>`** cycle.

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-<slug>/brief.md`
- **Stage:** `./projects/PROJ-001-<slug>/stages/STAGE-008-<slug>.md`
- **Spec:** `./projects/PROJ-001-<slug>/specs/SPEC-032-<slug>.md`
- **Reviewer brief (OPTIONAL, verify handoffs):** `./guidance/agents/<lens>.md` —
  pick a lens if this review should look at one thing hard. Omit for a
  general review; see `./guidance/agents/README.md`.
- **Toolchain brief:** `./guidance/toolchain-brief.md` — this repo's test
  framework, lint quirks, runtime globals and gotchas. Read it; it exists so a
  cold agent doesn't rediscover them (DEC-004 rule 5).

### Decisions that apply

- `DEC-NNN` — <one-line summary of why this matters here>
- `DEC-MMM` — <one-line summary>

### Constraints that apply

Check `./guidance/constraints.yaml` for full text. These constraints
apply to the paths touched by this task:

- `constraint-id-1` — <one-line summary>
- `constraint-id-2` — <one-line summary>

### Prior related work

- `HANDOFF-YYY` — <one-line summary, if relevant>
- `PR #NNN` — <link, if relevant>

## Expected Deliverables

*(For a `verify` handoff, replace this block with the verify contract below.)*

- Code changes implementing SPEC-032's Acceptance Criteria.
- All failing tests in SPEC-032 now passing.
- Any new tests required to cover edge cases.
- A PR against `main` from branch `feat/spec-XXX-<slug>`.
- PR description referencing: this handoff ID, the spec ID, the stage
  ID, the project ID, all referenced `DEC-*`, and any new `DEC-*`
  created during implementation.

### If `cycle: verify` — the verify contract

You are **not** the agent that implemented this. Review it cold.

- Acceptance criteria met? Tests actually pass? Build reflection answered
  honestly (*"nothing was unclear"* is suspicious)?
- Decision drift — run `just decisions-audit --changed`.
- Constraint violations; non-trivial choices missing a `DEC-*`.
- For any criterion claiming **runtime behavior** (a component registers, a hook
  fires, a binary resolves on PATH, a config takes effect), confirm the
  *behavioral* surface was exercised — not just the shape validated. That is the
  defect class that escapes (AGENTS.md §12).
- Output exactly ONE of: ✅ APPROVED / ⚠ PUNCH LIST / ❌ REJECTED.

## Out of Scope

Explicit list of what this handoff does NOT include. If the receiving agent
thinks any of these need to happen, they should create a new spec in
the stage's backlog, not expand this handoff.

- ...

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

- **Branch / PR:** `claude/spec-032-verify` (from `claude/spec-032-audit-b` at `217a5fa`; no PR, by instruction)
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** no: the last criterion (`just lint` green) fails on the pinned 1.90.0 toolchain (H1). The others are met and exercised by tests; the variant grid found privacy gaps outside the criteria (H2, M1).
- **For `verify`:** ⚠ PUNCH LIST (reviewed at `217a5fa`)

#### Gates (RUSTUP_TOOLCHAIN=1.90.0, ~/.cargo/bin first)

| Gate | Result |
|---|---|
| `just lint` | **RED**: clippy 0.1.90 `nonminimal_bool` at `fuzz/shared/audit_invariants.rs:967` (`!(png_or_webp && len == 0) && !found`), reached through `tests/fuzz_layout.rs` and `tests/report_render.rs`. Green on rustup stable (clippy 0.1.98, what CI runs), so CI would pass and the local gate does not. |
| `just test` | green (no failures) |
| `just mutation-oracle` | green |
| `just validate`, `just cost-audit`, `just forbid-unsafe` | green |
| `just oracle-record` twice | both exit 0, `git status` clean after each |
| `just fuzz-smoke audit` / `audit_render` (60 s) | clean: 135497 runs (227 seeds), 37816 runs (29 seeds) |
| `git diff main -- src/png src/webp` | empty |
| `decisions/INDEX.md`, `fuzz/Cargo.lock` | not in the branch's diff (the fuzz build dirtied `fuzz/Cargo.lock`; reverted, not committed) |

#### Punch list

- **H1 `just lint` red on 1.90.0.** One-line fix (`if !(found || png_or_webp && len == 0)` or split the condition). The handback said every gate was green, so it was likely run with a newer clippy.
- **H2 ImageMagick raw-profile Exif with GPS: `ok`, clean, exit 0.** For `zTXt`/`tEXt` `Raw profile type exif` holding GPS (alone, or beside a GPS-free `eXIf`), ExifTool prints `GPS:GPSLatitude`/`GPSLatitudeRef` and `IFD0:Make`. The audit reports status `ok`, `health.clean=true`, one `other_segment [medium]` finding, and exits 0 with the default `--fail-on high`, even with `--strict`. DEC-018 says SPEC-032 "reports a compressed or raw-profile region as metadata the audit could not read". The SPEC-031 verify note carried into the spec, and question `png-raw-profile-not-decoded`, say to flag a RawProfile beside an `eXIf`. Neither was done. The replay's `DIFFERENCES` has only the raw-XMP rows: the committed raw-exif fixtures carry no GPS, so the replay never exercised this case. Fix options (architect's call): count a RawProfile as an issue so the file is not clean or `ok`, or raise its severity, or flag the eXIf+RawProfile pair. At least record the GPS case as a divergence.
- **M1 `tIME` of length other than 7 is dropped.** ExifTool 13.55 prints `PNG:ModifyDate` for `tIME` chunks of 0 bytes (`0000:00:00 00:00:00`), 6 bytes (`2020:01:02 03:04:00`) and 8 bytes (`2020:01:02 03:04:05`). The audit has no item and no finding for them and reports clean/`ok`. The 7-byte rule ("only the specified form is a time") is not in DEC-020, was not measured, and has no question. The mutant that drops the length check is caught only by the fuzz harness, which re-states the same rule, never against ExifTool.
- **M2 Known issue confirmed: `Preview::strippable()` is true for every kind except PNG and WebP.** That covers `Unsupported(Gif|Heic|BigTiff|Empty|Unknown)` and unreadable files. CLI JSON for a GIF, a HEIC, a BigTIFF or an empty file under `--policy all-pii` prints `"strippable":true`. It should be `matches!(kind, Tiff | Jpeg)`. Effects:
  - The committed golden `tests/golden/report-full-house.json` pins `"strippable":true` for the unsupported GIF entry (`photos/screenshot.png`) and must be re-recorded.
  - The fuzz harness hard-codes the same rule (`strippable_kind` in `fuzz/shared/audit_invariants.rs`: `kind != "Png" && kind != "Webp"`; `render_invariants.rs` uses it), so it must change in the same commit.
  - The same predicate is duplicated in `src/report/mod.rs` (`Summary`). Summary reliability is unaffected today, because an unsupported or unreadable audit is never `clean`. Text and CSV are unaffected too: an unsupported file has no findings, and the text falls through to "preview not reliable".
  - Consider one shared `strippable(kind)` helper instead of three copies.
- **M3 Empty-region rule is WebP-measured but applied to PNG, where ExifTool does print something.** The PNG cases with length-0 regions: an empty-data `iCCP` named `Profile` (ExifTool prints `PNG:ProfileName` and `PNG:ICC_Profile` of 0 bytes; the replay's own ICC rule counts `PNG:ProfileName` as ICC present), an empty `iTXt` XMP (ExifTool prints `PNG:XMP` of 0 bytes), and `tEXt Author` with an empty value (ExifTool prints `PNG:Author` empty). In all three the audit has an item, no finding, and reports clean. Little personal data is at stake, but DEC-020 item 3 ("ExifTool treats it as absent") is false for PNG. Record it as a divergence or narrow the rule.
- **L1 Keyword table vs ExifTool's PNG keyword names.** `create-date` prints `PNG:CreateDate` and `modify-date` prints `PNG:ModDate` (named tags, not auto-names: compare `date:create`, which prints `PNG:Datecreate`); `Artist` prints `PNG:Artist`. The audit calls them `free_text` (medium) where they are a timestamp or authorship. They are still findings, so nothing is missed. Candidates for the one-row table.
- **L2 README overclaims.** "a GPS, XMP or ICC finding exactly when ExifTool prints the matching tag" omits "except the recorded differences" (17 of them).
- **L3 The 17-block test is weaker than its name.** Every `EXIF` chunk after the first already raises `ExtraExif`, so a 2-block WebP is equally not clean. `webp_seventeen_exif_blocks_with_gps_in_the_last_is_not_clean` therefore passes whatever the cap handling does. The outcome (not clean, unreliable) is right. Also, `issues` exits 0 without `--strict`, so a 17-block WebP whose only GPS is in the 17th exits 0 while ExifTool prints GPS. That is a recorded divergence (`webp-exif-block-cap`), but worth a README line.
- **L4 Cosmetic.** The golden's unsupported entry is still labelled `photos/screenshot.png` but holds a GIF (`container: gif`).

#### Criteria check (each exercised, not just present)

Kind dispatch and unreadable (`kind_dispatch_and_clean_minimal_files`, `hostile_prefixes_never_panic_and_are_never_clean`, `garbage_after_the_signature_does_not_panic`). Per-block inventory (`png_two_exif_blocks_are_both_inventoried_with_indexes`, `webp_two_exif_blocks_are_both_inventoried`). 17 blocks (see L3). Region matrix, empty regions and after-IDAT/IEND origins (`png_regions_match_the_category_matrix`, `png_empty_region_has_an_item_and_no_finding`, `png_trailer_and_after_idat_origins`, `webp_regions_and_empty_regions`). Keyword rule measured, pinned row by row and replayed against the committed `png-text-keywords` record, with a perturbation red-proof. Preview for every preset (`png_and_webp_previews_keep_everything_for_every_policy`). Strip refusal for every preset with the new kinds; `Custom` is covered because `strip` refuses on sniff before it reads the policy. CLI exit codes (four new `tests/cli.rs` tests). Category-presence replay, live test and perturbation red-proof. Fuzz seeds come from `examples/fuzz_seeds.rs` and are pinned byte for byte by `tests/fuzz_layout.rs`. The harness re-derivation is independent of `audit()`: it uses the readers' outputs with its own chunk-index map, its own after-image rule and its own typed-again keyword rule, plus 15 new plants. It shares the strippable rule's mistake (M2).

#### Strip refusal and renderers (item 3)

Ten PNG/WebP files (GPS Exif, XMP, ICC, tIME, text, raw profile, trailer), each under all five presets in text, JSON and CSV: `"removed":0` and `"kept_unread":0` everywhere, JSON disposition `kept` (70 rows), CSV `not_strippable` (70 rows), and text "not strippable ... nothing would be removed" on every preview. No output claims a removal.

#### JSON / CSV compatibility (item 5)

Key-by-key comparison of `report-full-house.json` (main) against both new goldens: the only key added is `preview.strippable`, last in `preview`; no key is removed or reordered. `schema_version` stays 2. The CSV header is byte-identical. New enum values appear (`region` `text`, `raw_profile`, `time`; `container` `png`, `webp`; CSV disposition `not_strippable`), which is additive.

#### Mutation (item 7), scratch copy, 1.90.0, restored after each

The baseline passed. Each mutant was red:

- m1, PNG block index +1: 8 tests (`png_two_exif_blocks...`, `png_with_gps...`, harness, render goldens).
- m2, empty region gets a finding: 8 tests, including the replay and the live presence test.
- m3, Copyright row dropped: 9 tests, including the unit row table, the keyword replay and the live keyword test.
- m4, WebP-layer issues ignored: 7 tests, including both 17-block tests.
- m5, PNG trailer not inventoried: 8 tests.
- m6, WebP strippable: 7 tests.
- m7, tIME of any length: 4 tests, harness only (see M1).

#### Variant grid (item 2): 94 files outside the builders' fixtures, `exif-audit --json --policy all-pii --inventory` vs `exiftool -a -G1 -s -n`

Every case where the audit is ok/clean but ExifTool shows more is listed above (H2, M1, M3). Every case where the audit claims more than ExifTool is a recorded decision or divergence:

- two `eXIf` with GPS only in the second: the audit lists it (DEC-020, all copies) and is not clean;
- `webp-iccp` with an unvalidated stub: `webp-iccp-profile-not-validated`;
- PNG trailer garbage: `other_segment`, per DEC-020.

Everything else agreed. That includes the measured keyword rule for tEXt, iTXt and zTXt across Author/author/AUTHOR/aUTHOR/Copyright/copyright/Creation Time/creation time/Software/warning (`creation time` → FreeText is the recorded `png-creation-time-keyword-case`). Also agreeing: compressed iTXt and zTXt with each standard keyword; XMP in iTXt, compressed iTXt, tEXt and zTXt (iTXt XMP with lang `en` → free_text, which ExifTool prints as `PNG:XMLcomadobexmp`); eXIf before IDAT, after IDAT and after IEND; text after IEND; the WebP simple, VP8X, Exif-prefix, no-flag, before-image, animated, trailing-bytes, trailing-EXIF, `XMP ` / `XMP\0` and empty-region shapes; and 16 blocks with GPS last (GPS listed).

### Cost self-report

- **Tokens (total):** null (left for the orchestrator to record from the harness `subagent_tokens` figure, by instruction)
- **Estimated USD:** null
- **Duration (minutes):** not measured
- **Source of the number:** harness report, recorded by the orchestrator

### Drift and new artifacts

- **New decisions emitted:**
  - none
- **Deviations from spec:**
  - none by the verifier. The builder's deviations (CSV `not_strippable` cell, `Summary` reliability rule, `unreadable` vs `issues` for a failed WebP Exif block) are consistent with DEC-010 and DEC-020.
- **Follow-up work identified:**
  - H1 to M3 above for the punch-list round. L1 (keyword rows) could go to SPEC-033. Raw-profile decoding stays with question `png-raw-profile-not-decoded`.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The spec's "Carried from SPEC-030/031 verify" notes (raw profile "could not read", flag eXIf+RawProfile) are not acceptance criteria and DEC-020 does not take them up, so it was unclear whether their absence is a defect or a decision; I treated it as a defect because DEC-018 assigns it to SPEC-032 by name.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The toolchain brief's "confirm `clippy --version` reports 0.1.90" belongs in the build handoff: CI's stable clippy and the pinned 1.90 clippy disagree on `nonminimal_bool`.

3. **If you did this task again, what would you do differently?**
   — Use case-distinct scratch file names from the start (macOS's case-insensitive filesystem merged `Author`/`author`/`AUTHOR` on the first grid run).
