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
  id: HANDOFF-089
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-24
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-040

project:
  id: PROJ-001
  stage: STAGE-011
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
  verdict: approved                # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 245439             # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 25
  branch: claude/spec-040-verify
  pr: null
  completed_at: 2026-11-24         # YYYY-MM-DD
  notes: "The verifier cannot read its own token count or wall time from inside the run; the orchestrator transcribes tokens_total and duration from the Agent result (subagent_tokens, duration_ms). Transcribed: subagent_tokens 245439, tool_uses 107 (the agent estimated about 85), duration_ms 1480597. estimated_usd null: no input/output split."
  synced_at: 2026-11-24
---

# HANDOFF-089: SPEC-040 verify (DEC-012 amendment: a JFIF APP0 thumbnail is a Thumbnail region)

**FIRST STEP:** `git checkout -b claude/spec-040-verify feat/spec-040-jfif-thumbnail`
(its tip is the build `058d243` plus the commit that adds this handoff). Work read-only on the library. Commit only
to this handoff file, on `claude/spec-040-verify`.

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-040 to an independent `claude-opus-5-5` verifier for
the **verify** cycle. A `claude-sonnet-5-5` implementer built it from HANDOFF-088. You did not
write this code; review it cold.

Read: `AGENTS.md` (§12, §15 "During verify"), `guidance/toolchain-brief.md` (its seed counts
predate this change), `guidance/constraints.yaml`, the SPEC-040 spec (Decisions made here,
AC1–AC14, Failing Tests), the DEC-012 and DEC-013 amendments dated 2026-11-24, DEC-009,
DEC-010, DEC-020, and `HANDOFF-088-*.md` (the build brief and the orchestrator-transcribed
handback, which lists the implementer's deviations). The change is
`git diff d74f576 feat/spec-040-jfif-thumbnail-build` (112 files; most are seeds, ExifTool
records and seed-count pins).

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` and confirm clippy
0.1.90. Fuzz needs nightly. macOS has no `timeout` (use `perl -e 'alarm 8; exec @ARGV' --`).
ExifTool 13.55 is on PATH. Restore `fuzz/Cargo.lock` after fuzz or lint-fuzz runs. Never
commit `decisions/INDEX.md`. Give each historical tree its own `CARGO_TARGET_DIR` (a shared
target dir gave a stale result in the last verify). Leave no background processes. Write
scratch scripts in a scratch directory, not the repo.

## Decisions that apply

- `DEC-012` + amendment 2026-11-24: the JFIF rule (part 1), the narrowed benign list (2), category and finding shape
  (3), presets unchanged (4), and whole-segment removal (5).
- `DEC-013` rule 3 + amendment 2026-11-24: the segment is the unit. A JFIF segment with a thumbnail goes under
  `Thumbnail`.
- `DEC-008` / `DEC-009`: `Thumbnail` is Medium, a block category, removed by `AllPii`, `All` and `Everything`.
- `DEC-010` / `DEC-020`: the region name `jfif_thumbnail` is additive, and `schema_version` stays 2.
- `DEC-001`: a short thumbnail is an issue plus the region of what exists.

## Constraints that apply

- `test-before-implementation`: each test is committed red before its code. AC10 and AC11 need recorded red
  evidence.
- `oracle-must-be-shown-red`: AC11 and AC12 each show a red.
- `panic-free-library` and `forbid-unsafe-code`: the header reads use `get(..)`, and `3·X·Y` uses
  `checked_mul`/`saturating_mul` (at most 195,075, but stay safe anyway).
- `fuzz-target-per-surface`: no new surface. Update the existing models and add seeds.
- `exiftool-never-linked` and `no-io-in-library`.

## What to establish (evidence for each, not opinion)

1. **The gap is closed, shown on real behaviour.** Build your own JFIF inputs (your own
   builder, not the implementer's `strip_shapes` or seed builders): a 2×2 thumbnail; a
   short one (declared 3·X·Y > bytes present); one with bytes past 3·X·Y; 0×0, 2×0, 0×3;
   a header cut short; a bare `JFIF\0`; a JFIF after the Exif APP1; version 2.00; two
   JFIF segments; `JFIF\0` in an APP1. For each, compare `main` (`c5c2231`, red) against the
   tip (green) on: the region list, the audit findings (category, severity, `thumbnail`),
   the JSON/CSV/text report, and `strip` under `Gps`, `GpsAndDevice`, `AllPii`, `All`,
   `Everything`, `none` and `Custom({Thumbnail})`. Check each against the DEC-012 amendment's
   rules and against ExifTool 13.55 (`-a -G1 -s` reads; `-all=` / `-JFIF:all=` writes).
2. **The writer cannot leak or over-cut.** Look for inputs where a JFIF thumbnail survives a
   policy that removes `Thumbnail` with `Ok`, or where a JFIF segment without a thumbnail is
   removed. Ideas: the JFIF segment is the first APP0 and the image's only APP0; JFIF in an
   MPF secondary image or after SOS; JFIF combined with a capped MPF index (DEC-013 refusal
   order); X·Y overflow (255×255); a JFIF segment the walk flags but the audit doesn't
   locate. Confirm `src/write/exif.rs` ignores region findings with `block: None` and that
   check (a) in `src/write/verify.rs` would fail closed on any leak.
3. **No regressions on existing files.** Run every input you can enumerate (the builders in
   `tests/support/*_shapes.rs`, `real_shapes::fixtures()`, and every seed under
   `fuzz/seeds/{jpeg_segments,audit,rewrite_jpeg,rewrite_tiff,tiff_ifd}`) through audit and
   `strip` for the presets plus `none` and each single category, on `c5c2231` and the tip.
   List every changed result. A change on an input that has no JFIF thumbnail is a finding.
4. **The test-suite changes are justified.** Check every changed pin (`tests/fuzz_layout.rs`,
   `inventory_oracle.rs`, `jpeg_oracle.rs`, `audit_model.rs`, `policy_preview.rs`,
   `budget_oracle.rs`, `mutation_oracle.rs`): each must equal the count of added JFIF seeds or
   records, with no existing record or seed changed. Confirm `git diff d74f576 --stat --
   tests/oracle-fixtures fuzz/seeds` shows additions only. Confirm the three fuzz models re-derive
   the rule independently and don't just call the library.
5. **The red-first claims.** Reproduce AC10 (audit change only → `strip(AllPii)` is
   `VerifyFailed { Preview }`; use `acb7ef4`), and AC11/AC12 (red on `c5c2231` or with the
   extraction disabled in a scratch tree). Judge whether `fde1f50` really holds the tests red.
6. **The implementer's deviations** (HANDOFF-088 handback): the AC6 `is_clean` reading (the
   architect accepts it: `clean` is parse health; say whether you agree); `RegionTooShort`'s
   need/have; no Thumbnail-only/`Gps`/`Custom` policy in the strip oracle (is the "kept"
   half of AC11 adequately covered?).
7. **Panic-freedom and constraints.** Check new arithmetic (3·X·Y) and slicing; run
   `just decisions-audit --changed`.
8. **Gates on 1.90.0:** `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just test`,
   `just mutation-oracle`, `just forbid-unsafe`, `just deny`, `just validate`, the oracle tests
   with `DIAMOND_EXIF_REQUIRE_ORACLE=1` (strip_oracle, inventory_oracle, jpeg_oracle,
   audit_oracle), and 60 s smokes `jpeg_segments`, `audit`, `audit_render`,
   `rewrite_jpeg`.

Output exactly ONE verdict: ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST (numbered, each with
severity H/M/L, evidence and a suggested fix) / ❌ REJECTED. Write it into the `## Handback`
section and `handback.verdict` (approved | punch-list | rejected).

## Budget

About 120 tool calls or 300k context. If you get near either, report what you established
and what you did not, and stop.

## Out of Scope

Fixing anything (report it instead); the deferred question `jfif-bytes-past-the-thumbnail`;
correcting AC6's wording or the toolchain brief (the architect does that at ship); CHANGELOG;
push or PR.

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

- **Branch / PR:** `claude/spec-040-verify` (local, from `feat/spec-040-jfif-thumbnail` at `756d817`). No push, no PR.
- **Completed at:** 2026-11-24
- **All acceptance criteria met?** yes. AC6's "`is_clean` is false" is read as the architect ruled (see 6 below).
- **For `verify`:** ✅ APPROVED at `756d817` (library and tests identical to the build tip `058d243`; `756d817` adds only handoff docs).

Scratch work (outside the repo): my own Python builder of 28 JFIF inputs, a probe example compiled against each tree
(regions, issues, findings, previews, JSON/CSV/text reports, `strip` under 8 policies, or 24 with every single
category), trees extracted with `git archive` and built with their own `CARGO_TARGET_DIR` each (`c5c2231`, `fde1f50`,
`acb7ef4`, `058d243`, plus a scratch copy of `058d243` with the extraction turned off).

**1. The gap is closed on real behaviour.** Inputs: 2×2; 4×4 with 12 bytes (short); 2×2 with 17 bytes; 0×0, 2×0,
0×3, 0×3 with data, 0×0 with text; a 13-byte header; bare `JFIF\0`; 2×2 with no data; JFIF after an Exif APP1 with
GPS; version 2.00; two segments (both have thumbnails, and with/without in each order); `JFIF\0` in APP1 (alone and
after Exif); 255×255 (cut at 65,519 bytes); 147×147 full (64,827 bytes); JFIF with JFXX; JFIF with COM; JFIF in the
trailer; JFIF in an MPF secondary; JFIF in a primary with an MPF index; a capped (17-entry) MPF index with JFIF in the
primary and in the secondary.
- On `c5c2231`, every thumbnail input audits with no finding and `strip` under every policy leaves ExifTool printing
  `JFIF:ThumbnailTIFF` (red). On `058d243`: one `JfifThumbnail` region per thumbnail-bearing segment, length
  `min(3XY, have)`; `Thumbnail`/Medium with `thumbnail=Some{range:None, starts_with_ffd8:false}`; JSON
  `"region":"jfif_thumbnail"`, CSV `jfif_thumbnail,12`, text `region jfif_thumbnail ... thumbnail range=none`;
  `schema_version` 2. Previews: gps/gpsdev/custom-gps `kept_unread`, allpii/all/everything/custom-thumb `removed`.
- Short shapes give one `RegionTooShort{what:"JFIF thumbnail", need:3XY, have}` (4×4: need 48 have 12; 255×255:
  need 195075 have 65519; 2×2 with no data: need 12 have 0 and no region). The benign shapes and APP1 `JFIF\0` give
  no region, no issue and no output change from `main`.
- ExifTool 13.55 `-a -G1 -s` on every output: `ThumbnailTIFF` count is 0 under AllPii/All/Everything/Custom{Thumbnail}
  and equal to the input's under Gps/GpsAndDevice/none/Custom{Gps}, no warnings. The region count matches
  ExifTool's `ThumbnailTIFF` count on every input. Our Custom{Thumbnail} and Everything outputs are **byte-identical**
  to `exiftool -JFIF:all=` and `-all=` on t2x2, short4x4, extra, v200, two-both, big255, full147 and jfif-com. They
  differ only on two-mixed, where we keep the thumbnail-less segment, as amendment part 5 says.

**2. No leak, no over-cut.** No input or policy gave `Ok` with a thumbnail left in the primary. A JFIF segment without
a thumbnail was never cut, including the 2×2-with-no-data shape. Splice check: 46 of 50 outputs with only
Jfif/Comment/OtherApp removals equal the input with exactly those ranges cut. The other 4 also carry Exif entry
edits or an MPF index patch, which is correct. The MPF patch was checked with ExifTool: MPImageStart moved 235→214.
With a capped MPF index and a primary JFIF thumbnail, AllPii/All/Custom{Thumbnail} now return
`Unstrippable{TooManyMpfImages}` (fail closed through the PATCH-001 path, because there is now a removal target).
Gps and none still return `Ok`. `select()` in `src/write/exif.rs` ignores the finding: it filters on
`origin().block == b` and on `ItemKind::Tag{ifd: Ifd1}`, and the entry loop `continue`s on non-tag items. A JFIF
thumbnail inside an MPF secondary survives AllPii with `Ok`. That is DEC-012 parts 3 and 5 (embedded images go only
under `Everything`; the preview says `kept_unread` and `holds:["thumbnail"]`), not this spec.

**3. No regressions.** There were 1,760 inputs: 122 builder outputs (strip_shapes, tiff_shapes, tiff hostile,
real_shapes::fixtures, oracle_jpeg/oracle_tiff fixtures; the 118 that predate the change are byte-identical on both
trees) and every seed in jpeg_segments/audit/rewrite_jpeg/rewrite_tiff/tiff_ifd. Each went through the regions,
issues, audit, previews, three reports and `strip` under 24 policies (5 presets, none, Custom{Thumbnail}, Custom{Gps},
each of the 16 single categories). After the `Region` discriminant renumbering is normalized out, 15 inputs changed.
13 have a `JfifThumbnail` region. 2 are the `jfif-thumbnail-declared-no-data` seed (jpeg_segments + audit copy),
which gains the issue only. No input without JFIF changed. No `VerifyFailed` and no panic on either tree.

**4. Test-suite changes.** `git diff d74f576 --name-status -- tests/oracle-fixtures fuzz/seeds` gives 84 A and 2 M. The two
M files (the inventory records for seed-audit and seed-jpeg_segments) have only `+` lines. The seed additions are
8 jpeg_segments, 8 audit and 56 rewrite_jpeg (4 strip shapes × 4 heads + 8 jpeg shapes × 5 heads). Every pin moves
by exactly these: +8/+8 in fuzz_layout/policy_preview/jpeg_oracle/audit_model, 1045→1101 in mutation_oracle,
255→271, 186→194, 81→89, 185→193 in inventory_oracle and budget_oracle, and 8 `jpeg-seed-jfif-*` records. The three
fuzz models re-derive the rule from payload bytes 12 and 13 (`3*x*y`, `p.len()-14`, `take>0`, `declared>0 && len>14`)
and do not call the extraction.

**5. Red-first.** fde1f50 (stubs): 11 positive tests fail by assertion (metadata AC1/3/4/5, audit AC6 ×2, preview AC7,
report AC13, strip AC8 ×2, lone-JFIF). The guard tests (benign, kept-under-gps, never-cut) pass, as they should.
AC10 at acb7ef4: `strip(AllPii|Everything|Custom{Thumbnail})` on my 2×2 file, the after-Exif file and both
two-segment files gives `VerifyFailed { check: Preview }`. Gps and the benign files stay `Ok`. AC11/AC12 with the
extraction off in a scratch copy of 058d243 (`DIAMOND_EXIF_REQUIRE_ORACLE=1`): strip_oracle fails `jfif-thumbnail
under all: groups differ: extra ["JFIF"]` and the replay record mismatch. inventory_oracle fails
`jfif_thumbnail_tiff_is_the_jfif_thumbnail_region`, replay and live parity ("UNEXPLAINED difference" on the 5 JFIF
seeds) and the class-S pins. The in-test perturbation red-proofs (line dropped, our region dropped) are present.
The oracle and fuzz-pin tests were committed after the code (0d48533), as the spec's Notes order allows. Their red
is reproduced here.

**6. Deviations.** AC6 `is_clean`: I agree with the architect. `Health::clean` is parse health (DEC-010 status
`ok`/`issues`), and a COM finding already leaves it true. A 2×2 file is `clean:true`, status ok, with a Medium
finding. `RegionTooShort` need/have = 3XY and the data bytes after the 14-byte header is the natural reading and
matches the fuzz model. "Kept" half of AC11: each strip-oracle JFIF fixture records six Custom policies that keep
`Thumbnail` (xmp, icc, embedded, other, iptc+free_text, rights), and ExifTool shows `ThumbnailTIFF` in each. Gps,
GpsAndDevice, none and Custom{Gps} are unit-tested in `tests/strip.rs`, and I checked them live with ExifTool
(item 1). I judge this adequate.

**7. Panic-freedom and constraints.** The new library code reads with `payload.get(12)`, `get(13)`, `get(14..)` and
`have.get(..n.min(len))`, and computes 3·X·Y with `saturating_mul`. There is no indexing and no unchecked
arithmetic. The fuzz models index, but fuzz/ is at clippy defaults (DEC-004). `just decisions-audit --changed` is
advisory and lists DEC-001/002/006/008/009/010/012/013/020, among others. The change fits each one: issue plus partial
region (DEC-001), borrowed `&[u8]` (DEC-002), Thumbnail row unchanged (DEC-008/009), additive region name with
`schema_version` 2 (DEC-010/020), and the amendments (DEC-012/013).

**8. Gates on 1.90.0 (clippy 0.1.90).** All green: `just lint`, `lint-fuzz`, `lint-no-allow`, `forbid-unsafe`,
`deny`, `validate`, and `just test` with `DIAMOND_EXIF_REQUIRE_ORACLE=1` (39 binaries, 1,480 passed, 0 failed, 0
SKIP; strip_oracle, inventory_oracle, jpeg_oracle and audit_oracle included). `just mutation-oracle`: 67 + 40 + 41
passed. Fuzz smokes (60 s): jpeg_segments 145,611 runs, audit 79,264, audit_render 33,791, rewrite_jpeg 94,366, no
crash. `fuzz/Cargo.lock` restored.

### Cost self-report

- **Tokens (total):** 245,439 (transcribed by the orchestrator from the Agent result `subagent_tokens`)
- **Estimated USD:** null (follows from the token count)
- **Duration (minutes):** 25 (Agent result `duration_ms` 1,480,597)
- **Source of the number:** harness report (Agent result), transcribed by the orchestrator

### Drift and new artifacts

- **New decisions emitted:**
  - none
- **Deviations from spec:**
  - None beyond the build's (AC6 reading, accepted). Observations, not findings: (a) a 2×2-declared JFIF with no
    data now makes the file `issues` with an unreliable preview, though ExifTool prints no warning. This is
    amendment part 1 as written. (b) With a capped MPF index, a primary JFIF thumbnail now makes
    AllPii/All/Custom{Thumbnail} refuse (`TooManyMpfImages`) where they used to return the input unchanged. This
    fails closed and is consistent with PATCH-001.
- **Follow-up work identified:**
  - None required. Optional: a Gps or AllPii preset row in the strip oracle's JFIF fixtures would make the preset
    half of AC11 live-recorded and not only unit-tested.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Little. The diff baseline was `feat/spec-040-jfif-thumbnail-build`, but the branch to check out was a
   different name. Both point at the same code. The worktree sandbox refused `git worktree add` for historical
   trees, so I used `git archive` copies.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — PATCH-001's capped-MPF refusal (`refuse_capped_mpf` runs whenever there is a removal target). A new removal
   target type widens when that refusal fires, which is the one behaviour change on a non-JFIF-only input class.

3. **If you did this task again, what would you do differently?**
   — Build the probe and the input set first and run both trees from the start. A one-pass regression sweep of
   every builder and seed under every single-category policy was the strongest evidence and cost little.
