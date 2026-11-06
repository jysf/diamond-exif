---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-013
  type: story                      # epic | story | task | bug | chore
  cycle: build  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
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
  decisions: [DEC-001, DEC-005, DEC-006, DEC-007, DEC-008]
  constraints: [oracle-must-be-shown-red, exiftool-never-linked, panic-free-library, forbid-unsafe-code, test-before-implementation, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-005, SPEC-008, SPEC-011, SPEC-012, SPEC-017]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-012]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-003's <capability>". Optional; null is acceptable.
value_link: null

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
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-013: Inventory parity against ExifTool: group + tag-id sets on fixtures, comparator red-proofed

## Context

SPEC-005/008/011 built the comparator, records and red-proof apparatus for tag *values*. SPEC-012 built the audit model: `audit(&[u8]) -> FileAudit` whose **inventory** lists every IFD entry (`ItemKind::Tag { ifd, ifd_index, tag, .. }`, per Exif block for a JPEG) and every located region (`ItemKind::Region`). Nothing yet checks that inventory against the reference tool. An audit that silently omits an entry ExifTool lists, or invents one ExifTool does not, is wrong in the direction that matters. This spec points the oracle at the *set* of (group, tag id) entries, for bare TIFF and for JPEG, with every difference written down.

**What this design measured first** (ExifTool 13.55, macOS; `exiftool -a -u -G1 -H -s <file>`, each call under `perl -e 'alarm 8; exec @ARGV'`, output captured to a variable, exit status taken from the tool itself, never through a pipe). `-G1` gives the specific group (`IFD0`, `ExifIFD`, `GPS`, ...), `-H` gives the tag id in hex, `-a` keeps duplicates, `-u` keeps unknown tags. Our side was dumped with a throw-away program over `audit()`. Corpora swept: 32 `fuzz/seeds/tiff_ifd` seeds, 37 `jpeg_segments` seeds, 76 `audit` seeds (many are byte copies of the other two), the 12 TIFF and 42 JPEG hand-built oracle fixtures, and the 41 one-entry TIFFs of `tests/support/audit_tags.rs`: 240 files, **171 distinct by content**. **No call timed out except `two-hop-cycle`** (alarm, "Deep recursion on subroutine ProcessDirectory"). Comparison: multiset of (group, id) lines, ExifTool lines with id `-` dropped (those are synthetic: ThumbnailImage, XMP/JUMBF/ICC values) and our pointer-tag items dropped.

Result: on **140 of the 171 distinct inputs the two sets are identical** (including GPS/Interop/ExifIFD/IFD1 grouping, three-IFD chains, `SubIFD`/`SubIFD1`, two Exif blocks in one JPEG with duplicates kept, every one of the 41 DEC-008 ids). The other 31 fall into these measured classes (they are the design's prediction; the build re-measures every one and the measurement wins):

| Class | Inputs (measured) | ExifTool lists | Ours lists | Who is right |
|---|---|---|---|---|
| **A. value unreadable** (payload or count runs past the end) | `ascii-count-past-eof` (IFD0 271, 272, 305, 306, 315), `count-overflow`, `count-overflow-be` (IFD0 700 to 703, 705), `payload-past-eof`, `payload-past-eof-be` (IFD0 700, 701) | nothing for those ids | the entries | ours (an entry is evidence even when its value is not readable; an audit must not hide it) |
| **B. revisited IFD** | `exif-points-at-ifd0` (ExifTool: extra `ExifIFD` 0x0100), `self-cycle` (extra `IFD1` 0x0100) | re-reads the offset under a second group | visits an offset once (DEC-005) | ours (same bytes; nothing new to strip) |
| **C. depth limit** | `deep-nesting`: ours 8 SubIFD entries, ExifTool 11 | 3 more | stops at the depth limit and reports an issue | ExifTool lists more. Not silent: `Health::exif_issues > 0`, `clean == false` (asserted) |
| **D. caps and budget** | `entry-budget-exact` (ours extra SubIFD 0x0002 x1, 0xFFFF x3), `entry-budget-over` (0x0003 x1, 0xFFFF x2), `overlapping-subifds` (0x00FF x255), `audit-many-overlapping-ifds` (0x02FF x767) | nothing (it keeps only the first 10 SubIFDs) | the entries within `len/12` | neither is "the" answer: two different limits, both documented |
| **E. no return** | `two-hop-cycle` (alarm) | n/a | 1 + 1 entries | not comparable: a `"*"` divergence like SPEC-005, never run live |
| **F. JPEG shapes** (SPEC-011 names reused) | `exif-after-scan` and `last-segment-at-eof` (ExifTool lists nothing; ours lists every entry), `exif-before-and-after-scan` (IFD0 0x010F ours 2, ExifTool 1), `seventeen-exif*` (ours 16, ExifTool 17 per tag) | see left | see left | ours for the first three (reading more is safe); the 17th block is the `MAX_EXIF_BLOCKS` cap, not silent (`ExtraExif` issue, asserted) |
| **G. groups outside the model** | `JFIF` (ids 0, 2, 3, 5: APP0) on every JFIF seed, `Photoshop` (0x0405, 0x0408, ... when no 0x0404 resource) | listed | no item (not in the model) | not a divergence: a recorded **unmodelled group**, an exact list |
| **H. directory-valued TIFF tags** | ids 0x02BC, 0x83BB, 0x8649, 0x8773 (and the pointers 0x8769, 0x8825, 0xA005, 0x014A) | never as a tag line; the *decoded contents* appear under other groups (`IPTC` for 0x83BB, measured: IPTC 0x0078) or nothing when the payload does not decode (the audit-tag fixtures) | the entry as a normal tag | not a divergence: a recorded **directory-valued id list** |

Regions (measured with well-formed minimal payloads; the exact bytes are in "Fixtures"): XMP packet -> `[XMP-xmp] - CreatorTool`; IPTC (`Photoshop 3.0` + `8BIM` 0x0404 + dataset) -> `[IPTC] 0x0005 ObjectName`; ICC profile (128-byte header, one `desc` tag) -> 15 `[ICC-header]` lines (ids 0x04 to 0x54) and `[ICC_Profile] - ProfileDescription`; JUMBF (`JP` + sequence + `jumb` box with a `jumd` box) -> `[JUMBF] - JUMDType/JUMDToggles/JUMDLabel`. **A header-only ICC profile (zero tags) and a bare `jumb` box produced nothing**, so region fixtures must be well-formed, not stubs. Our side: `Region::Xmp` / `Iptc` / `Icc` / `Jumbf` items.

## Goal

Committed ExifTool listings (group + id) for every corpus above, a comparator for the sets, live and replay tests with **zero unexplained differences**, every difference a **pinned** divergence (exact multiset difference, command, version, measured output, right side), a hard invariant that anything we list *fewer* than ExifTool is never silent, and red-proofs (control first) showing a dropped, invented, mis-grouped or mis-numbered entry is named. The six low findings carried from the SPEC-012 and SPEC-017 verifies are closed with deterministic tests.

## Inputs

- **Reads:** `src/audit/{mod,category,sniff}.rs` (the inventory), `src/tiff/mod.rs` (`IfdKind`, budget), `tests/support/{tools,oracle,oracle_tiff,oracle_jpeg,audit_tags,probe,mutate,hostile}.rs`, `tests/{audit_oracle,audit_model,jpeg_oracle,mutation_oracle,fuzz_layout}.rs`, `examples/oracle_record.rs`, `fuzz/shared/audit_invariants.rs`, `fuzz/seeds/**`, `app.just`, `.github/workflows/ci.yml`, DEC-001/005/006/007/008, HANDOFF-024 and HANDOFF-026 (the carried findings and mutant definitions).
- **Does not read:** ExifTool source or documentation (it is run, never read). Every ExifTool claim is a command, a version and measured output.
- Constraints `oracle-must-be-shown-red`, `exiftool-never-linked`, `panic-free-library`, `forbid-unsafe-code`, `test-before-implementation`, `no-new-top-level-deps-without-decision`. No dependencies.

## Outputs

### 1. The inventory model: `tests/support/inventory.rs` (new, std only, tier A)

Test code only; **nothing in `src/` changes** (a bug the oracle finds in `src/` is reported, not fixed here).

- `struct Entry { group: String, id: u16 }`; `type Multiset = BTreeMap<(String, u16), usize>`.
- `fn ours(a: &FileAudit) -> Inventory` where `Inventory { tags: Multiset, regions: BTreeSet<RegionKind> }`. A **Tag** item maps to ExifTool's group by `IfdKind`: `Ifd0` -> `IFD0`, `Ifd1` -> `IFD1`, `Exif` -> `ExifIFD`, `Gps` -> `GPS`, `Interop` -> `InteropIFD`, `Chain` -> `IFD<k+2>` where `k` is the 0-based position among the `Chain` IFDs of the **same block**, `Sub` -> `SubIFD*` (numbering normalised: ExifTool names them `SubIFD`, `SubIFD1`, ...; a separate test pins the first three names on one fixture). Items whose id is in `DIRECTORY_VALUED` are not put in `tags` (Class H). `Region` items go to `regions`.
- `DIRECTORY_VALUED: [u16; 8] = [0x8769, 0x8825, 0xA005, 0x014A, 0x02BC, 0x83BB, 0x8649, 0x8773]`: pointer tags (their child IFD's entries are compared in their own groups) and the four tags whose content ExifTool decodes elsewhere. A test asserts the list is exactly these eight and that each has a fixture.
- `fn theirs(stdout: &str) -> Result<Inventory, ParseError>`: parse `[Group] 0xID Name : value` and `[Group] - Name : value`; drop file-level groups (`System`, `File`, `ExifTool`, `Composite`) as `audit_tags::FILE_LEVEL_GROUPS` does; normalise `SubIFD<n>` -> `SubIFD*`; keep `IFDn`, `IFD0`, `IFD1`, `ExifIFD`, `GPS`, `InteropIFD` as is; map **region groups** to `regions` by prefix: `XMP*` -> `Xmp`, `IPTC` -> `Iptc`, `ICC*` -> `Icc`, `JUMBF` -> `Jumbf`; lines with id `-` in EXIF-family groups are dropped. Any other group must be in `UNMODELLED_GROUPS = ["JFIF", "Photoshop"]` (Class G, dropped); **an unknown group is a `ParseError`** (test failure), so a new group on a new fixture is a decision, not a silent drop. For a bare TIFF, region-group lines are Class H (decoded contents of a directory-valued tag) and are dropped, with a test that the fixture really carries one of the four ids.
- `fn diff(ours, theirs) -> InventoryDiff { ours_only: Multiset, theirs_only: Multiset, regions_ours_only, regions_theirs_only }`, deterministic order, with a `Display` that names `(group, id, count)`.
- `struct Divergence { fixture: &'static str, class: Class, ours_only: &'static [(&'static str, u16, usize)], theirs_only: &'static [(&'static str, u16, usize)], regions_ours_only: &'static [RegionKind], regions_theirs_only: &'static [RegionKind], right_side: Side, question: &'static str, ours_hides: bool }`. A fixture with a divergence **must produce exactly that diff** (not "some diff"): a diff that grows, shrinks or vanishes fails (the list cannot rot, and a pinned diff cannot loosen: it names the values we must still equal). `ours_hides` is true iff `theirs_only` or `regions_theirs_only` is non-empty.
- **Invariant (hard):** for every divergence with `ours_hides`, `audit(...).health().clean == false`. A file for which we list fewer entries than ExifTool must not be reported clean. Applied in the live and replay tests and in the fuzz-corpus sweep. (Measured expectation: Class C has `exif_issues > 0`; the 17th block has `jpeg_issues > 0`.)
- `KNOWN_DIVERGENCES: &[Divergence]` is written by hand from the build's own measurement (the table above is the prediction), one entry per fixture or seed; byte-identical duplicates across seed directories (the `audit` seeds copy the other two) are compared once (dedup by bytes; a test asserts the distinct-input count). Class E (`two-hop-cycle`) is `"*"`: never run live, recorded as `# outcome: not run` like the existing records.

### 2. Fixtures

Existing: the 12 `oracle_tiff` and 42 `oracle_jpeg` fixtures, the 41 `audit_tags` one-entry TIFFs, the three seed directories. New, built in `tests/support/oracle_jpeg.rs` / `oracle_tiff.rs` (same independence rule as before; names normative):

- `tiff-chain-four` (IFD0, IFD1, then two more chained IFDs: pins `IFD2`, `IFD3`), `tiff-subifds-three` (one `0x014A` naming three leaf IFDs: pins `SubIFD`, `SubIFD1`, `SubIFD2`).
- `jpeg-two-exif-with-gps-each` (two Exif blocks, each with a GPS IFD: the duplicate-keeping `-a` case beyond one tag).
- `jpeg-icc-valid`: APP2, payload `ICC_PROFILE\0` + `01 01` + a profile: 128-byte header (bytes 0 to 3 = total profile length, 8 to 11 `02 10 00 00`, 12 `mntr`, 16 `RGB `, 20 `XYZ `, 36 `acsp`, the rest zero) + `00 00 00 01` + tag record `desc`, offset 144, size 13 + the `desc` data (`desc`, 4 zero bytes, `00 00 00 05`, `abcd\0`). Measured above.
- `jpeg-jumbf-valid`: APP11, payload `4A 50 00 01` + `00 00 00 01` + a `jumb` box (`size`, `jumb`) containing a `jumd` box (`size`, `jumd`, 16 zero bytes, toggles `03`, label `test\0`). Measured above.
- Existing `jpeg-xmp` and `jpeg-iptc` cover the other two region kinds; `jpeg-iptc-without-0404` covers the `Photoshop` group (Class G) and `jpeg-xmp-in-wrong-marker` the absent case.
- Each new shape is **probed alone** under the ExifTool limit before it joins any loop (process rule 1).

### 3. Records (`tests/oracle-fixtures/exiftool-inventory/`, new directory)

One file per corpus: `fixtures.txt` (hand-built TIFF + JPEG, including the new ones), `audit-tags.txt` (the 41 one-entry TIFFs), `seed-tiff_ifd.txt`, `seed-jpeg_segments.txt`, `seed-audit.txt`. Each file has a `#` header (command line `exiftool -a -u -G1 -H -s <file>`, ExifTool version) and one section per input: `## <name>`, optional `# outcome: not run (...)`, `# stderr: <line>` lines (path replaced by `<file>`), then one line per **kept** ExifTool line, `group<TAB>id-or-dash<TAB>name`, in ExifTool's own order (file-level groups removed). The existing `tests/oracle-fixtures/exiftool/` records are **byte-identical** afterwards (a hash-list test, as in SPEC-011). `examples/oracle_record.rs` grows the new corpora; `just oracle-record` produces an empty diff on a clean tree; the CI `oracle-records` job's `git status --porcelain` path list is extended to the new directory.

### 4. Tests, live and replay

Live tests run ExifTool (skip loudly without it; `DIAMOND_EXIF_REQUIRE_ORACLE=1` makes a missing tool a failure, as today); replay tests run the same comparator against the committed records with no tool. Both go through `inventory::{ours, theirs, diff}` and the `KNOWN_DIVERGENCES` table. Names in "Failing Tests".

### 5. Red-proofs (control first, exact-once needle guard, `/* RED-PROOF INJECTION */` marker; the SPEC-008 apparatus)

- **Ours-side perturbation** (in-test, on the `Inventory` value): drop one entry, invent one, move a `GPS` entry to `ExifIFD`, renumber `IFD2` as `IFD3`, add a region: each must give a diff that **names** the exact `(group, id)`; control run on the unperturbed inventory is empty first.
- **Their-side perturbation** (on a committed record's parsed text): delete one line, change an id, change a group: the diff names it.
- **Byte mutation of a fixture** (SPEC-008 `mutate::locate_entry`): change one IFD0 tag id in `tiff-exif-basic-le` and in `jpeg-exif-basic-le` (offset = TIFF offset + 10 inside the APP1): the committed (unmutated) record compared with our audit of the mutated bytes names the old and new id; truncating the JPEG between APP1 and SOS leaves the inventory unchanged (the typed outcome from SPEC-011).
- **Mutated-copy library faults** (the probe crate gains an *inventory* mode that prints `inventory::ours` lines for the given file; faults injected into the copy of `src/audit/mod.rs` / `src/tiff/mod.rs`): (i1) `Interop` IFD entries skipped; (i2) tag id stored masked (`tag & 0x00FF`); (i3) `Gps` items labelled `Exif`; (i4) blocks after the scan dropped from the inventory (caught by the *divergence disappearing* on `jpeg-exif-after-scan`); (i5) `Chain` position off by one; (i6) `RegionKind::Iptc` mapped to `Xmp`. Each: the unmutated copy compares clean first; the mutated reading must give a **named** difference, not merely any.
- **The invariant** `ours_hides => !clean` is itself shown red: a test constructs a hiding divergence on a fixture whose `Health` is clean (use a hand-patched copy of the `Health` value) and expects the invariant check to fail by name.

### 6. Carried from the SPEC-012 verify (HANDOFF-024), closed

- **F2 / M20:** a deterministic test that `Health::exif_issues` is the sum over every block (block 0 with issues, the last block clean) and `clean` is false.
- **F3 / M21:** a deterministic test with a metadata region after the scan: `Origin::after_scan` is true on the region item and on its finding.
- **F4 / M22:** thumbnail pointer tests with `BYTE` (type 1) and `SSHORT` (type 8) typed `0x0201`/`0x0202`: `ThumbnailRange::range` is `None`; `SHORT` and `LONG` give `Some`.
- **F1:** tighten the `tests/jpeg_oracle.rs` TIFF-records exemption from the `audit-` prefix to the exact file name `audit-tag-names.txt`.
- **F5:** make `g_category_shape`'s region loop and `g_thumbnail`'s per-finding re-read in `fuzz/shared/audit_invariants.rs` linear in the input (no per-finding re-scan); `just fuzz-smoke audit` clean for 60 s, executions recorded.
- **F6:** *not done here.* The two `DEC-008` amendment candidates (`0xC65D` RawDataUniqueID, `0x013C` HostComputer) stay in the question `audit-table-exiftool-names`; adding a table row is a decision for the audit-CLI stage, and this spec must not change the model.
- Each of M20/M21/M22 is **shown red**: apply the mutant by hand to `src/audit/mod.rs` (definitions in HANDOFF-024), show the named test fails with its assertion text, revert (`git diff` empty), repeat for all three. Reported in the Handback.

### 7. Carried from the SPEC-017 verify (HANDOFF-026), closed

- **m04:** `a_truncated_ifd_spends_no_entry_budget`: a truncated sub-IFD with a large declared count followed by a sibling IFD whose entries fit the remaining budget **exactly**; the sibling must be read in full and no `TooManyEntries` raised. Mutant m04: charge a `Truncated` IFD's declared count anyway.
- **m07/m08:** `entry_budget_counts_the_top_level_chain` (a chain of IFDs crossing the budget: the crossing IFD is omitted whole, the walk halts, `TooManyEntries` issued) and `entry_budget_counts_exif_gps_and_interop_links` (same through `0x8769`, `0x8825`, `0xA005` links). Mutants: m07 budget skipped for `Link::Next`; m08 only `SubIFD` entries counted.
- Each of m04/m07/m08 is shown red by hand the same way (control, mutant, named failure, revert) and reported in the Handback.

### Process rules the build follows

1. Every new shape is run through ExifTool **alone, under the limit,** before it joins a record, sweep or loop. A non-return becomes a `"*"` divergence.
2. Every difference is a `KNOWN_DIVERGENCES` entry (pinned multiset difference, command, version, measured stdout, right side) **and** a `guidance/questions.yaml` entry. Questions to add: `exiftool-omits-entries-with-unreadable-values` (Class A), `exiftool-rereads-a-revisited-ifd` (Class B), `audit-depth-limit-and-ifd-caps-differ-from-exiftool` (Classes C and D, with the `ours_hides => !clean` invariant and the numbers), `audit-directory-valued-tiff-tags` (Class H: the audit lists 0x02BC/0x83BB/0x8649/0x8773 as plain tags; ExifTool decodes them), and one line on the unmodelled groups (Class G). Existing questions for Class F (`exiftool-ignores-exif-after-scan`, `exiftool-drops-exif-when-last-segment-ends-at-eof`, `exiftool-reads-17th-exif-block`) gain a pointer to this spec's entries.
3. A difference that points at a **library bug** (ours claims an entry that is not in the bytes, a wrong group, a wrong id, a hidden entry on a clean file): do not fix it silently. Stop the affected test, record the measured output in the Handback, and the orchestrator decides.
4. Where the measurement differs from this spec's table (counts, group names, which fixture falls in which class), the measurement wins; the Handback lists each difference from the table.
5. Do not modify `decisions/`, `scripts/` or irradiance. ExifTool is run, never read. Do not push or open a PR; do not advance cycles.

## Acceptance Criteria

- `tests/support/inventory.rs` exists with the model above; `src/` is unchanged (the carried items touch tests and `fuzz/shared` only; the mutants are applied by hand and reverted).
- Live and replay: **zero unexplained** differences over every input of every corpus; the divergence table is exactly the measured set, each entry pinned to its exact diff and present in its test; `two-hop-cycle` recorded as not run.
- The invariant `ours_hides => !clean` holds on every divergence, live and replay, and is shown red.
- Every unknown ExifTool group on any input is a failure until named in `UNMODELLED_GROUPS`; the list is exactly `JFIF` and `Photoshop` unless the build measures another (then the Handback says so).
- The six region/ICC/JUMBF/XMP/IPTC presence checks agree with ExifTool on the well-formed fixtures; the stub forms are documented as "ExifTool lists nothing".
- Red-proofs: ours-side, their-side, byte-mutation and mutants i1 to i6 each give a **named** difference with a clean control first.
- Carried tests: M20, M21, M22, m04, m07, m08 each have a deterministic test and each mutant is shown red then reverted; F1 tightened; F5 linear and `just fuzz-smoke audit` clean for 60 s.
- The existing 125 records are byte-identical; the new records reproduce (`just oracle-record`, empty `git status --porcelain`), and CI's `oracle-records` path covers the new directory.
- All gates: fmt (both graphs), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof` (run locally: CI-only otherwise), `cargo test` on stable and 1.90.0, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` (empty diff), `fuzz-smoke` for `tiff_ifd`, `jpeg_segments` and `audit`.
- No dependencies; `scripts/`, `decisions/`, irradiance untouched; `guidance/questions.yaml` has the new entries.

## Failing Tests

Names are normative (a rename or fold is a deviation to report).

**tests/inventory_oracle.rs (new):**
- `directory_valued_ids_are_exactly_the_eight_and_each_has_a_fixture`
- `inventory_groups_map_ifd_kinds_and_number_the_chain`
- `sub_ifd_names_are_pinned_on_the_three_subifd_fixture`
- `theirs_parser_drops_file_level_groups_and_synthetic_ids`
- `theirs_parser_rejects_an_unknown_group`
- `theirs_parser_maps_region_groups_by_prefix`
- `distinct_inputs_are_counted_once_across_seed_directories`
- `replay_every_corpus_is_clean_except_pinned_divergences`
- `replay_inventory_headers_record_command_version_and_stderr`
- `replay_two_hop_cycle_is_recorded_not_run`
- `live_inventory_parity_on_hand_built_fixtures`
- `live_inventory_parity_on_the_audit_tag_fixtures`
- `live_inventory_parity_on_the_seed_corpora`
- `class_a_unreadable_values_are_ours_only_and_still_present`
- `class_b_revisited_ifd_is_theirs_only_and_still_present`
- `class_c_depth_limit_is_theirs_only_and_the_file_is_not_clean`
- `class_d_caps_and_budget_are_ours_only_and_still_present`
- `class_f_jpeg_shapes_match_the_recorded_divergences`
- `seventeenth_block_is_theirs_only_and_the_file_is_not_clean`
- `ours_hides_implies_not_clean_for_every_divergence`
- `the_hides_invariant_goes_red_on_a_clean_file` (red-proof)
- `a_divergence_that_stops_being_one_fails`
- `a_divergence_that_changes_its_diff_fails`
- `regions_agree_on_xmp_iptc_icc_and_jumbf_fixtures`
- `stub_icc_and_jumbf_are_listed_by_us_and_not_by_exiftool`
- `unmodelled_groups_are_exactly_jfif_and_photoshop`
- `existing_oracle_records_are_byte_identical_to_the_committed_set` (hash list)
- `oracle_names_a_dropped_entry`, `oracle_names_an_invented_entry`, `oracle_names_a_regrouped_entry`, `oracle_names_a_renumbered_chain_group`, `oracle_names_an_extra_region`
- `oracle_names_a_deleted_line_in_a_record`, `oracle_names_a_changed_id_in_a_record`, `oracle_names_a_changed_group_in_a_record`
- `byte_mutation_changes_one_tag_id_in_a_tiff_and_is_named`
- `byte_mutation_changes_one_tag_id_in_a_jpeg_app1_and_is_named`
- `truncating_between_app1_and_sos_leaves_the_inventory_unchanged`

**tests/mutation_oracle.rs (extended):**
- `inventory_probe_control_run_is_clean_before_every_mutant`
- `inventory_mutant_i1_interop_skipped_is_named`, `..._i2_masked_tag_id_is_named`, `..._i3_gps_labelled_exif_is_named`, `..._i4_after_scan_blocks_dropped_makes_the_divergence_vanish`, `..._i5_chain_position_off_by_one_is_named`, `..._i6_iptc_region_as_xmp_is_named`

**tests/audit_model.rs (extended; F2/F3/F4):**
- `health_exif_issues_is_the_sum_over_all_blocks`
- `a_region_after_the_scan_has_after_scan_origin_on_item_and_finding`
- `thumbnail_pointer_needs_short_or_long_types`

**src/tiff (unit) or tests/budget_oracle.rs (extended; SPEC-017):**
- `a_truncated_ifd_spends_no_entry_budget`
- `entry_budget_counts_the_top_level_chain`
- `entry_budget_counts_exif_gps_and_interop_links`

**tests/jpeg_oracle.rs / tests/fuzz_layout.rs:**
- the TIFF-records exemption is the exact file name `audit-tag-names.txt` (update the existing test)
- `audit_harness_guards_are_linear_in_the_input` (a large input finishes within a fixed number of steps, counted not timed)
- the existing seed hash lists stay byte-identical

## Non-Goals

- Value comparison (already done for the 20 selectors); the contents of XMP, ICC, IPTC, JUMBF (presence only)
- Changing the audit model, the category table (F6 stays a question) or any `src/` behaviour; a bug the oracle finds is reported, then decided
- Resolving the depth limit, SubIFD cap or entry budget differences from ExifTool (Classes C and D are recorded, with the not-silent invariant)
- PNG, WebP, HEIC; MakerNote contents; the `exif-audit` CLI; real camera files (tier A only)
- New hostile shapes not first probed alone under the ExifTool time limit

## Notes for the Implementer

- Order of work: the carried tests first (they are independent and quick: M20/M21/M22, m04/m07/m08 with the by-hand red proofs) -> `tests/support/inventory.rs` and its parser/model unit tests (test-first) -> probe every new shape alone -> records and recorder -> sweep, write `KNOWN_DIVERGENCES` from what you measure -> live and replay tests -> red-proofs -> probe inventory mode and mutants i1 to i6 -> fuzz harness linearity and smoke -> CI path.
- Reuse, do not duplicate: `tools::run_tool_with_limit`, `tools::warning_lines`, the `# stderr:` record shape from SPEC-011, `mutate::locate_entry`, the probe crate machinery.
- Throw-away sweeps belong in the session scratchpad, not `/tmp`. A scratch dump program over `audit()` is the quickest way to see our side; the committed code must not depend on it.
- Never measure ExifTool's exit status through a pipe. No `timeout(1)` on this host: `perl -e 'alarm N; exec @ARGV'`. Python reading ExifTool output must decode with `errors="replace"` (some seeds yield non-UTF-8 value text).
- Shell: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `just lint-red-proof`, `just lint-fuzz`, `just lint-fuzz-red-proof` under the default PATH (stable clippy); fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; macOS `sed -i ''`.
- Report in the Handback: every divergence with its measured output, every class assignment that differed from the table, the unmodelled groups found, the fuzz-smoke executions and coverage for `audit`, and each red-proof and each carried mutant's before and after.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`: `type: lesson` (with its N-count) for a recurring
   coding pattern, `type: process-debt` for tooling/process friction. A close
   then forces the decision. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(Runtime/operational defects — the escape-prone class — only exist once the
   artifact meets its real host. `escaped` here is a signal to strengthen the
   §12 behavioral pre-flight for that surface.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — <answer | none>
