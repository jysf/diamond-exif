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
  id: HANDOFF-024
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: completed                  # pending | accepted | completed | rejected

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
  status: completed                     # completed | blocked | rejected
  tokens_total: 243756             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.61
  duration_minutes: 12
  branch: worktree-agent-a3c82dfab672c9cf4
  pr: null
  completed_at: 2026-11-05               # YYYY-MM-DD
  notes: "verdict APPROVED at 36f0022; tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-024: Verify the audit model

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-012` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-012-audit-model-container-sniff-inventory-pii-categories-and-findings-pure-library-dec-for-the-category-table.md` (API, rules, named tests normative); **DEC-008** incl. its Amendment (the Exif 3.0 ids were corrected after the build's live check; judge the amendment against the ExifTool measurement)
- **Build handoff and Handback incl. Fix round:** `HANDOFF-023-build-audit-model-...md` (claims to verify, not to trust; read deviations D1-D12)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 290441d..HEAD -- src fuzz tests decisions` (build commit `9ced293`, fix `3d39354`); main files `src/audit/{mod,sniff,category}.rs`, `fuzz/fuzz_targets/audit.rs`, `fuzz/shared/audit_invariants.rs`, `tests/audit_oracle.rs`, `tests/audit_model.rs`, `tests/support/audit_tags.rs`

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status`). Run `just fuzz-smoke audit` for 60 s and record executions and coverage (the builder saw ~115 exec/s; is that a harness problem or inherent?).
- **Read `src/audit` against the spec rules and DEC-008 line by line**: sniff (every magic, every prefix, never overreads), the table (compare each row to DEC-008 yourself, not to the test), GPS context-sensitivity (same ids in IFD0 vs GPS), Interop (D3), the IFD1 thumbnail rules (0x0201; strips-only; range check against the blob; `starts_with_ffd8`), regions mapping, pointer tags never findings, multi-block and after-scan handling, inventory cap and what happens to findings beyond it (D1: `Finding.item: Option<usize>`), health/clean semantics (D10: `ExtraExif` makes a JPEG not clean; is that right for an audit?), determinism. Hunt for: panic-capable constructs, arithmetic overflow, unbounded allocation (findings vector bound), work that is superlinear, anything that copies value bytes.
- **Judge the builder's deviations**, especially D1, D3, D5 (spec invariant contradicted DEC-008 for TIFF tags 0x02BC/0x83BB/0x8649/0x8773 mapped to Region categories: is the DEC or the invariant right, and is the model sound?), D7 (an existing test was touched), D10, D12.
- **Test the testers.** Write mutants the builder did not try (table row moved to another category; GPS rule keyed on tag range instead of IFD kind; Interop treated as non-GPS table; `0x0201` accepted outside `Ifd1`; thumbnail range check `<` vs `<=`; `after_scan` dropped; second block ignored; findings dropped past the cap; `clean` ignoring jpeg_issues; sniff accepting a 3-byte TIFF prefix; `RIFF` without `WEBP`; BigTIFF classified as Tiff; severity of a category changed; `Category::ALL` missing a variant; the independent re-derivation in the harness copying a library bug). Report which the suite catches and which survive. Redo at least three fuzz plants including one against the real libFuzzer target; confirm each iff-invariant is both-directional.
- **Re-measure the table-name check yourself** with ExifTool (run it, never read its source): pick the rows that matched, the 9 naming-only rows and the 0xA437-0xA43C rows; is the amendment right? Is any row of DEC-008 still doubtful (ids you can check by building a one-entry TIFF)? Also check there is no remaining id where DEC-008's category is arguably wrong.
- **Re-check the amplification measurement** (`container_amplification_is_measured_and_reported`): reproduce the headline numbers (input size, entries, peak RSS) and say whether the reported cause (overlapping IFDs via SubIFDs, next-words zeroed) is right and whether the audit itself is bounded on that input.
- Check seeds and records: the existing 66 seeds and 120 records byte-identical; the 7 new audit seeds deterministic and each claiming its shape (`xxd`); the new record reproduces; CI change is only `audit` in the `fuzz-smoke` matrix; no dependencies; `scripts/`, irradiance untouched; `decisions/` changes only the DEC-008 amendment.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-013/014/015/016/017 work, the Container memory fix, CI wiring beyond reading it

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

- **Branch / PR:** branch `worktree-agent-a3c82dfab672c9cf4` (fast-forwarded to `claude/rust-exif-metadata-6351d5` at `36f0022`; only this handoff file is committed on top). No push, no PR, no cycle advanced.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** Yes. Every gate re-run green, the table matches DEC-008 (incl. the amendment) row for row, the ExifTool measurement reproduces, the amplification reproduces. The findings below are test gaps and notes, none a defect in `src/audit`.
- **For `verify`:** ✅ **APPROVED at `36f002264c1a7924e26322382f1397b7824b4f70`** (the code under review; build `9ced293`, fix `3d39354`), with the non-blocking follow-ups F1-F6.

**Gates (re-run cold in this worktree, 2026-11-05).** All exit 0: `cargo fmt --all -- --check`, `cargo fmt --manifest-path fuzz/Cargo.toml -- --check`, `just lint`, `just lint-fuzz` (clippy 0.1.98 pinned), `just lint-no-allow`, `just lint-red-proof`, `just lint-fuzz-red-proof`, `just forbid-unsafe`; `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test --no-fail-fast` on Homebrew 1.99.1, `~/.cargo/bin/cargo +stable` and `+1.90.0`, plus plain `cargo test` (each: 170 unit, 2 audit_model, 16 audit_oracle +2 ignored, 4 crate_policy, 19 fuzz_layout, 42 jpeg_oracle, 30 metadata_oracle, 51 mutation_oracle); `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit` (0 structural errors, 10 pre-existing scope warnings), `just cost-audit`; `just oracle-record` (41 TIFF + 79 JPEG + audit record, 41 ids) followed by an empty `git status --porcelain`.

**Fuzz.** `just fuzz-smoke audit` (60 s, fresh corpus): exit 0, `seed corpus: files: 72` (the zero-byte seed is not loaded as a file), `#73 INITED cov: 1346 ft: 3316`, `#11639 DONE cov: 1457 ft: 5114 corp: 406/1498Kb exec/s: 190 rss: 608Mb`, 11,639 runs in 61 s. The builder saw ~115 exec/s; I measured 190. **Cause of the rate:** one execution of the `audit` target on `audit-many-overlapping-ifds` (195,586 entries) costs ~104 ms (3 runs: 108/104/103 ms) against ~35 ms for the `tiff_ifd` target on the same file. So the floor is inherent (the `Container` amplification, SPEC-017) and the harness multiplies it by ~3 (two `audit` calls for determinism plus the independent re-derivation, each a full `Container::parse`, plus `String` flattening of up to 65,536 items each). Not a harness bug; SPEC-017 is the real fix, a cheaper flattening the secondary one.

**Code read against the spec and DEC-008.** Sniff: every magic and prefix as rule 1, `get(..12)` so nothing past 12 bytes is read, BigTIFF and `RIFF` without `WEBP` correct. Table: I compared every `by_id` arm with DEC-008 myself: 40 ids, categories and severities identical, `0x0201` only in `Ifd1`, GPS all-`Gps`, Interop none, pointer tags none. GPS context sensitivity is keyed on `IfdKind`, not id. Thumbnail: first `0x0201`/`0x0202`, count 1, SHORT/LONG, `checked_add`, `blob.get(start..end)` against the block's own TIFF blob, FF D8 from those bytes; strips rule computed once per IFD. JPEG: every `Ok` block, `after_scan` from the segment, failed blocks counted, regions mapped per DEC-008. No panic-capable constructs (only `get`, `checked_add`, `saturating_add`, `try_from`); no superlinear work (per-IFD context computed once, `after_scan` O(1)); memory bounded by the two caps; `Item` holds no value bytes; no dependencies.

**Deviations judged.** D1 (`item: Option<usize>` plus `at`): correct, the only shape that satisfies "never lost to truncation" with valid indices. D3 (Interop none): follows DEC-008's "non-GPS, non-Interop". **D5:** DEC-008 is right and the spec's fuzz invariant was mis-stated: the TIFF tags 700/33723/34377/34675 really are XMP/IPTC/Photoshop/ICC containers, so the same category on a tag item is sound; the replacement guard (region items exactly their category, Jumbf never on tags, Gps iff GPS-IFD tag, range iff Thumbnail) is the true invariant. D7: acceptable, but see F1. **D10:** acceptable for an audit: "clean" means "read without anomalies", and a second Exif segment is one (both blocks are still reported); SPEC-015 should say so in its rendering. D12 (audit tests not shown red before implementation): a process deviation, compensated by the mutation evidence below.

**ExifTool re-measurement (13.55, run, never read).** My own one-entry LE TIFFs (independent Python builder, ASCII/UNDEFINED/BYTE per tag), each under `perl -e 'alarm 8; exec @ARGV' exiftool -G1 -a -s -u` and `exiftool -v3`. Matching rows sampled: 0x010E, 0x010F, 0x0131, 0x013B, 0x8298, 0x9003, 0x9010, 0x9290, 0x9286, 0xA420, 0xA433, 0xA435, 0xC62F, 0x9C9B, 0x9C9D, 0xA436: all named as DEC-008. Naming-only rows: 0x0132 ModifyDate, 0x9004 CreateDate, 0x927C MakerNoteUnknownText, 0xA430 OwnerName, 0xA431 SerialNumber, 0x02BC ApplicationNotes (v3 only), 0x8649 PhotoshopSettings (v3 only), 0x8773 ICC_Profile (v3 only), 0x83BB IPTC-NAA (v3), 0x0201 in IFD1 ThumbnailOffset: same as the record. Exif 3.0: 0xA437 Photographer, 0xA438 ImageEditor, 0xA439 CameraFirmware, 0xA43A RAWDevelopingSoftware, 0xA43B ImageEditingSoftware, 0xA43C MetadataEditingSoftware, 0xA43D `Exif_0xa43d` (unknown). **The amendment is right**, and no remaining row's id is doubtful. No row's category is arguably wrong; ImageUniqueID under DeviceId identifies the image rather than the device, but DEC-008 says so explicitly and High is defensible for linkability.

**Amplification re-measured.** `cargo test --release --test audit_oracle --no-run`, then `/usr/bin/time -l target/release/deps/audit_oracle-<hash> container_amplification_is_measured_and_reported --ignored --nocapture`: input 789,516 bytes, `Container::parse` 215 ms, 512 IFDs, 33,488,386 entries, 0 issues, 803,721,264 bytes of entries (1,018x), RSS 792,032 KB after the parse; audit 185 ms incl. its own parse, 65,536 items, 0 findings, truncated; `maximum resident set size` 897,744,896, `peak memory footprint` 897,320,136 (builder: ~950 MB; same order, host noise). The cause is as reported: one `SubIFDs` entry naming 511 distinct offsets two bytes apart into 0xFF bytes, so each is a 65,535-entry IFD over overlapping bytes, next words zeroed so no IFD visit is spent on a failed chain. The audit itself is bounded (items and findings capped, time linear in the 33.5M entries, nothing copied); the unbounded part is `Container`, SPEC-017's business.

**Seeds, records, scope.** `git diff 290441d..HEAD` shows no change under `fuzz/seeds/tiff_ifd`, `fuzz/seeds/jpeg_segments`, existing records, `scripts/`, `src/tiff`, `src/jpeg`, `Cargo.toml`/`Cargo.lock`; `.github/workflows/ci.yml` changes only the `fuzz-smoke` matrix line; `decisions/` only the DEC-008 amendment (the orchestrator's commit `ad9e3a8`); the new record reproduces. The 7 new seeds checked with `xxd` (PNG signature + IHDR, `II+\0` BigTIFF, zero bytes, IFD1 strips with no 0x0201, JPEG with Exif/GPS/IFD1 thumbnail whose range lands on `FF D8 FF D9`, two Exif blocks LE/BE with Make each, 255 overlapping SubIFDs in 10,764 bytes) and by the shape test.

**Mutants (my own; applied by script, run against `--lib --test fuzz_layout --test audit_model --test audit_oracle`, each reverted with `git checkout`).**

| # | Mutant | Result | Caught by |
|---|---|---|---|
| M01 | Copyright 0x8298 moved Authorship -> FreeText | caught | table row test, harness on seeds |
| M02 | GPS keyed on tag <= 0x1F instead of IFD kind | caught | 6 unit tests, harness, plants |
| M03 | Interop IFD uses the table | caught | table tests only (harness on seeds: no) |
| M04 | 0x0201 is Thumbnail in any IFD | caught | table tests, unusable-pointer test |
| M05 | thumbnail range requires end < len | caught | checked-range test, harness, shape test |
| M06 | Exif `after_scan` always false | caught | after-scan test, harness |
| M07 | second Exif block ignored | caught | two-blocks, fatal-block tests, harness |
| M08 | findings dropped past the item cap | caught | cap-boundary unit test only |
| M09 | `clean` ignores jpeg_issues | caught | 2 unit tests, harness |
| M10 | sniff accepts 3-byte `II*` | caught | sniff prefix test |
| M11 | `RIFF` without `WEBP` is WebP | caught | 2 sniff tests, harness |
| M12 | BigTIFF sniffed as Tiff | caught | sniff, unsupported tests, harness |
| M13 | DeviceId severity Medium | caught | severity test, cap test |
| M14 | `Category::ALL` missing Icc | caught | severity test |
| M15 | same row bug in library AND harness table | caught | DEC-copied table test (defence in depth holds) |
| M16 | strips rule in any IFD kind | caught | strips test |
| M17 | `starts_with_ffd8` = non-empty | caught | checked-range test |
| M18 | ExtendedXmp -> Iptc | caught | region mapping test, harness |
| M19 | GPS pointer 0x8825 a Gps finding | caught | 4 unit tests |
| M20 | `exif_issues` = last block's count, not the sum | **survived** | none (F2) |
| M21 | region `after_scan` always false | **survived** | none (F3) |
| M22 | thumbnail pointer type gate (SHORT/LONG) removed | **survived** | none (F4) |
| M23 | findings cap removed | caught | findings-cap test |
| M24 | unreadable TIFF `failed_blocks` 0 | caught | unreadable-IFD0 test, harness |
| M25 | max_severity = first finding's | caught | 2 unit tests, harness |

22 of 25 caught by the suite.

**Real libFuzzer target plants (60 s each, fresh corpus, seeds `fuzz/seeds/audit`, mutant applied then reverted):** M06 crashed on seed load (`inventory` guard: `after_scan: false`, want `true`); M20 crashed at ~#106 (`health` guard: `exif_issues: 0`, want `2`); M21 crashed at ~#773 (`inventory` guard on a Jumbf region after the scan); M08 crashed at ~#1069 (`findings: 18 entries, want 46`); M03 survived 60 s (6,651 runs). So M20 and M21 are caught by the CI fuzz smoke, not by any deterministic test. The iff guards read both directions (gps_iff, category_shape's Gps iff and range iff Thumbnail, health's restated `clean` definition, unsupported -> !clean and clean -> supported); the builder's 44 single-guard plants pass.

**Findings (all non-blocking).**
- **F1 (Low).** `tests/jpeg_oracle.rs` (D7) now exempts every `audit-*` file from the "41 non-JPEG records" count; an exact-name exemption for `audit-tag-names.txt` would keep the guard tight.
- **F2 (Low, test gap).** No deterministic test sums `exif_issues` over several blocks (M20 survived). If block 0 has issues and the last block is clean, the mutant makes `exif_issues` 0 and `clean` true, against spec rule 6. Only the fuzz smoke catches it. Suggest a unit test: an issue-carrying block, then a clean block.
- **F3 (Low, test gap).** No deterministic test of a region after the scan (M21 survived); the seed `jpeg-app-after-scan` carries no region. Same kind of one-line test.
- **F4 (Low, test gap).** The SHORT/LONG gate on the thumbnail pointer is untested for a type the reader decodes (only RATIONAL is tried, which fails decoding anyway); a BYTE or SSHORT 0x0201 test would pin it.
- **F5 (Low, harness).** The harness's `g_category_shape` region-has-a-finding loop is O(items x findings) and `g_thumbnail` re-runs `jpeg::read` per thumbnail finding; both are quadratic on hostile JPEGs. They add to the slow rate; worth fixing with the cheaper flattening.
- **F6 (Info, DEC-008).** Ids ExifTool names that the table leaves inventory-only and a future amendment could weigh: 0x013C HostComputer, 0x010D DocumentName, 0xC614 UniqueCameraModel, 0xC65D RawDataUniqueID, 0xA432 LensInfo (measured names above). Also: a pointer tag inside a GPS IFD is a `Gps` finding (DEC-008's "every tag in a GPS IFD" wins over spec rule 2); harmless, worth one sentence in the DEC.

### Cost self-report

- **Tokens (total):** null. The verifier subagent has no `/cost` or `usage` access to its own run; the orchestrator transcribes the metered figure.
- **Estimated USD:** null (same reason).
- **Duration (minutes):** null (no wall clock available; not estimated).
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. The builder's D1-D12 are judged above (all accepted; D5 is a spec mis-statement, DEC-008 is right).
- **Follow-up work identified:**
  - Three short deterministic tests for F2-F4 (could ride on SPEC-013 or SPEC-015).
  - SPEC-017 (Container memory) also lifts the fuzz rate; F5's harness cost alongside.
  - A DEC-008 note on F6 if the policy presets (SPEC-014) need those ids.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Little. The one ambiguity was whether "redo three fuzz plants" meant harness-level plants or library mutants under the real target; I did library mutants under the real target (five), which also answers which survivors the CI smoke catches.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The worktree-isolation shell refuses compound commands, loops and globs; the verify brief could say "use script files" up front. Also, a verify brief could ask for deterministic coverage of every health counter's aggregation, which is exactly where the survivors were.

3. **If you did this task again, what would you do differently?**
   — Start the mutant script before reading the code in depth (it is the long pole, ~1 min per mutant here) and time the fuzz target per seed first, which answers the exec/s question in one command.
