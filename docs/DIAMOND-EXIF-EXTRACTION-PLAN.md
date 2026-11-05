# diamond-exif — extraction plan (session 1)

**Author:** diamond-exif-extract session (`diamond-exif-extract-8a86a5`), 2026-11-03.
**Status:** PLAN ONLY. No files created in any target repo; irradiance left byte-for-byte unchanged (read-only base).
**Blocked on:** the exact repo path for diamond-exif — orchestrator is getting it from the maintainer.

This plan is grounded in a direct read of the authority docs and the actual source at
`irradiance@` branch `claude/diamond-exif-extract-8a86a5` (worktree). Every line/claim below was
verified against the files, not transcribed from the brief alone.

Sources read: `docs/porting-brief-diamond-exif.md` (the authority), `exif-core-design.md`
(`~/PSeven/.../throwaway-irradiance/.agents/worktrees/exif-design/`), `docs/future-plans.md`,
`AGENTS.md` §1/§2, `docs/provenance-ledger.md`, `src/ifd.rs` (header + consts + ByteOrder +
readers + Entry/Ifd + Container::parse/walk_chain/read_ifd/payload/uints/rationals/scalars),
`src/lib.rs` (lint block + Error enum), `tests/support/tools.rs` (choke point + parse_fields).

---

## 0. What this session did and did not do

- **Did:** read the full authority chain; verified the brief's §1 item table against the real
  `src/ifd.rs`; confirmed the five-lint deny set (`src/lib.rs:95-102`), the 6 irradiance-only
  gate scripts (all present), the `tiff_ifd` fuzz target, and the provenance ledger rule;
  produced this plan + the spec-workflow recommendation + the first DEC list.
- **Did NOT:** create or scaffold anything in `~/PSeven/.../diamond-exif` (orchestrator said the
  maintainer is setting up a dedicated new repo; hold all creation for the relayed path); modify
  any irradiance file (per "use the worktree only as a READ base").

---

## 1. Recommendation: run diamond-exif under the ContextCore spec workflow — with two scoping calls

**Recommendation: YES — run it under ContextCore (new REPO → one PROJECT → STAGES → SPECS),
scaffolded from irradiance's OWN harness, not the template.** Rationale:

1. The maintainer wants irradiance's house style carried *exactly* — and that style **is** the
   ContextCore machinery: DEC decision files, the per-algorithm provenance ledger, the
   oracle-must-be-shown-red gate (`lint-red-proof`), `validate`/`gate-audit`/`cost-audit`, and
   bit-exact oracles. Those are not reproducible as loose conventions; they are the `just` gates.
2. The design doc already decomposes the work into milestones **M1a–M1f + M2** (design §12) that
   map 1:1 onto ContextCore STAGEs, and the per-cell oracle matrix (§8) maps onto SPECs.
3. The brief mandates a **"first DEC, written before porting any tests"** (the tolerant-walk
   policy, §0 risk 1 / design §16.1). A DEC is a ContextCore artifact; doing this right *requires*
   the workflow.
4. The work spans many sessions with delegated build/verify/ship cycles — exactly the workflow's
   delegation model (irradiance ran PROJ-002 this way).

A direct scaffold-and-port (lints + ledger only, skipping DEC/spec ceremony) would be faster but
would drop the mechanical enforcement that makes the permissive + panic-free claims *defensible*
rather than asserted — which the maintainer explicitly values. So: full workflow, with these two
scoping calls flagged for the maintainer:

### Scoping call A (recommend single-crate first) — workspace vs single crate
Design §3 proposes a cargo **workspace** (`crates/exif-core` + `crates/exif-pro` + `site/`). But
every ContextCore gate assumes a **single-crate** repo: `lint-red-proof.sh:80` hard-codes
`LIB_RS="${REPO_ROOT}/src/lib.rs"` and stages a single-crate `Cargo.toml`/`src/` (brief §3.2);
`lint-fuzz-red-proof` assumes `fuzz/` is a second graph *outside* the library. Adapting all gates
for a workspace up front is real work (brief §3.2 lists the exact edits).
`exif-pro` and `site/` are **M2** (design §9/§12). **Recommend: start exif-core as a single-crate
repo** (`src/`, `tests/`, `fuzz/` at root — identical shape to irradiance, gates nearly drop-in);
promote to a workspace at M2 when `exif-pro` actually arrives, as its own DEC. This is lower-risk
and keeps the first foundation port focused.

### Scoping call B — gate source
**Clone the gate harness from irradiance's `scripts/` + `app.just` + `justfile` + `guidance/`
(the fixed versions), NOT `jysf/spec-driven-template`.** Brief §3.1/§16.9: 6 gate scripts and the
`_lib.sh`/`validate.sh` bug fixes exist in irradiance but not in the local template clone
(template 0.6.39 vs repo 0.6.38). A fresh template repo silently inherits the *unfixed* gates and
loses all 6 red-proof/audit scripts. (G4: template drift vs the *current* upstream release was
unverifiable at brief time — run `gh release list -R jysf/spec-driven-template` first.)

---

## 2. Target module layout (design §4, grounded against the code)

Single-crate (scoping call A). `src/` mirrors irradiance's shape:

```
diamond-exif/                    # the new repo (path TBD from maintainer)
├── Cargo.toml                   # edition 2021, MIT OR Apache-2.0, [dependencies] = thiserror only
├── deny.toml                    # irradiance's allow-list verbatim (§3.3)
├── CHANGELOG.md
├── LICENSE-MIT, LICENSE-APACHE
├── AGENTS.md, .repo-context.yaml, justfile, app.just, guidance/, decisions/, projects/
├── docs/
│   ├── provenance-ledger.md     # §5 — first rows below
│   └── oracle-contract.md       # the METHOD (measured/dated/with-command), rewritten for exiftool
├── scripts/                     # cloned from irradiance (scoping call B), workspace-edits N/A if single-crate
├── src/
│   ├── lib.rs                   # #![forbid(unsafe_code)] + five-lint deny; pub mod wiring; crate docs
│   ├── error.rs                 # Error/Result — the 13 TIFF-level variants copied + the Warning channel (G2)
│   ├── reader.rs                # ByteOrder (decode+encode), read_u8/u16/u32/u64, offset_to_usize
│   ├── tiff/
│   │   ├── mod.rs               # Entry, Ifd, Container, parse/walk/read_ifd/payload/scalars (adapted)
│   │   ├── types.rs             # type_size + TYPE_* (all 12) + uints/rationals + ASCII/signed/float decoders
│   │   └── tags.rs              # baseline-TIFF + EXIF/GPS/Interop pointer tags; diamond-exif's own is_structural
│   └── (later: containers/, xmp.rs, iptc.rs, icc.rs, model.rs, policy.rs, edit/, api.rs)
├── tests/
│   ├── metadata_oracle.rs       # ported PATTERN (rewritten), not the file
│   ├── support/{tiff.rs, tools.rs(exiftool-only subset), mutate.rs(NEW), probe.rs(NEW, de-duped)}
│   └── fixtures/                # committed tier-A, licence-clean
└── fuzz/fuzz_targets/tiff_ifd.rs
```

---

## 3. The first foundation module to port (M1a) — sequenced

Budget per brief §7: **"adapt the walk + write ~60% new"**, NOT "port 3,209 lines". Sequence so the
repo is buildable + green at each step and the policy change is isolated:

**Step 0 — scaffold** (first creation session, once path arrives): Cargo.toml, lib.rs with
`#![forbid(unsafe_code)]` + five lints (copy `src/lib.rs:95-102` verbatim), deny.toml, CHANGELOG,
LICENSEs, the cloned gate harness (scoping call B), empty provenance-ledger.md with its header
rule, `just` wiring. Gate: `cargo fmt/clippy/test` green on an empty lib.

**Step 1 — the first DECs** (before any test port): see §4.

**Step 2 — port the STRICT skeleton** (DNG stripped, byte-faithful walk):
| From `src/ifd.rs` | To | Class |
|---|---|---|
| 13 TIFF-level `Error` variants + `Display` | `error.rs` | EDIT (copy, brief C1) |
| `ByteOrder` + `u16/u32/i32` (make `pub(crate)`; add `u64/f32/f64/i16` + encode twins) | `reader.rs` | CLEAN/EDIT |
| `read_u16/read_u32` (+ `read_u8/read_u64`), `offset_to_usize` | `reader.rs` | CLEAN |
| `MAX_IFD_DEPTH`/`MAX_IFDS`/`MAX_TAG_VALUES`, `TIFF_VERSION`, `ENTRY_SIZE` | `tiff/mod.rs` | CLEAN (re-justify depth=8 for Exif→Interop→MakerNote) |
| baseline-TIFF tag consts + `TAG_ORIENTATION` | `tiff/tags.rs` | EDIT (drop DNG tags; add EXIF/GPS/Interop pointers) |
| `type_size` + `TYPE_*` (add ASCII/SBYTE/SSHORT/SLONG/FLOAT/DOUBLE) | `tiff/types.rs` | CLEAN/EDIT |
| `Entry` + `Entry::parse` (add `pub(crate)` ctor + raw accessor for future writer) | `tiff/mod.rs` | EDIT |
| `Ifd` + accessors (reword DNG prose at `:467-469`) | `tiff/mod.rs` | EDIT |
| `Container<'a>` + `parse` + `byte_order/ifd0_offset/ifds/ifd0` | `tiff/mod.rs` | EDIT (payload returns `&'a[u8]`, brief §1.4) |
| `walk_chain` + `read_ifd` + `payload` | `tiff/mod.rs` | EDIT/CLEAN |
| `uints` (drop DNG per-tag RATIONAL gate + integer-division rule) | `tiff/types.rs` | EDIT |
| `rationals` → split into `URational(u32,u32)`/`SRational(i32,i32)` (never divide) | `tiff/types.rs` | EDIT (type changes, brief row 573+) |
| `scalar/required_scalar/values` | `tiff/mod.rs` | CLEAN |
| `is_structural_tag` body | — | **STAYS; rewrite** in `tags.rs` with diamond-exif's own set (brief C2) |
| 17 "clean" hostile-input tests (brief §1.3) + `Build` byte-builder (`:1735-1860`) | `tests/` | PORT |

Gate: `cargo test --lib tiff::` green; the strict walk passes all ported hostile-input tests.

**Step 3 — apply the tolerant-walk policy (G2)** as its own change, DEC-001-gated. This flips
~25% of the ported tests (brief §0 risk 1, §1.3 rows `2544-2700`/`3035`/`3128`) — do it *after*
the strict skeleton is green so the diff is legible. `Container::parse` returns a result that
carries per-IFD/region outcomes (`Metadata { issues: Vec<ParseIssue> }`-shaped) so "IFD0 OK, GPS
IFD corrupt" survives and is reportable.

**Step 4 — multi-IFD pointer walk + wider types** (the ~60% new): generalise
`sub_ifd_offsets_of_last` into a pointer-tag dispatcher that follows ExifIFD (0x8769), GPS
(0x8825), Interop (0xA005) in addition to SubIFDs (330) — brief §0 risk 1. Add the ASCII +
signed + float decoders in `types.rs` (Make/Model/Artist/Copyright/DateTimeOriginal are ASCII).

**Step 5 — oracle discipline:** the exiftool-only `tools.rs` subset + `parse_fields` (extend
`Field.values` beyond numeric-only `Option<Vec<u32>>` to strings/rationals/floats/lists), the
`metadata_oracle.rs` replay-and-perturb red-proof **pattern** (rewritten; brief §2.3), a new
`tests/support/mutate.rs` (`set_u32_at`/`flip_bit`/`truncate_at`/`swap_byte_order_mark`, ~20 lines),
the mutated-copy probe helper lifted into ONE shared `tests/support/probe.rs` (it is copy-pasted
3× in irradiance — brief §2.3), and the `tiff_ifd` fuzz target (ports nearly verbatim, brief §2.4).

A good **stopping point for the first creation session**: Steps 0–2 (scaffold + DECs + strict
skeleton, green). Steps 3–5 are follow-on specs.

---

## 4. The DECs to write first (before porting tests)

- **DEC-001 — tolerant-walk policy + diagnostics channel (G2).** THE first decision. Non-IFD0
  IFD/region failures are collected, not fatal; `Result<Container,Error>` → a container carrying
  `Vec<ParseIssue>`. IFD0/header failures stay fatal. (brief §0 risk 1; design §16.1)
- **DEC-002 — `&[u8]` core, `bytes::Bytes` only at the API edge.** Keep the walker over `&[u8]`
  so lifetimes + tests port unchanged; `payload` returns `&'a [u8]` tied to the input (one-token
  change, brief §1.4). This also matches irradiance PROJ-004 Stage A's locked rule.
- **DEC-003 — provenance class 3 (derived from class 1).** The port is our own prior code (class
  3) derived from published specs (class 1). Record BOTH (brief §5; ledger `:17-30`).
- **DEC-004 — (scoping call A) single-crate now, workspace at M2.** Only if the maintainer agrees.
- Later/at-build: a DEC for the exiftool-`-j`-JSON dev-dependency question (G3) if `-j` is chosen
  over the tab form; a DEC if XMP needs `roxmltree` (design §13.2).

---

## 5. Provenance approach (how every ported file is recorded)

Per `AGENTS.md` discipline 3 + ledger rule (`docs/provenance-ledger.md:17-30`): **every ported
file gets a row before/at the moment it lands**, honestly — "port" = copy-then-edit, which is
class 3 derived from class 1. The new ledger's header must restate: *exiftool (Perl, Artistic/GPL)
may be RUN as an oracle, never READ to resolve a format question* (brief §5), and *running a tool
imposes nothing; reading copyleft source is a violation*.

First rows (brief §5):

| Item | Source | Class | Licence |
|---|---|---|---|
| `tiff/*`, `reader.rs` — IFD walk, bounds-checked reads, cycle/depth/count guards | **Class 3:** irradiance `src/ifd.rs` @ its ship commit, same author (jysf). **Upstream class 1:** TIFF 6.0 (1992) §2 | 3 (→1) | MIT OR Apache-2.0, own code |
| `tests/support/tools.rs` subset — subprocess-oracle pattern | Class 3: irradiance, same author. exiftool **run, never linked** | 3 | own code; exiftool imposes nothing |
| `containers/*`, `xmp.rs`, `iptc.rs`, `icc.rs` (later) | **New, class 1 only:** name the exact spec+edition (CIPA DC-008 Exif, ITU T.81/JFIF, PNG spec, WebP container spec, ISO/IEC 14496-12, Adobe XMP, IPTC IIM, ICC) | 1 | — |

**Copyleft check (done, not assumed):** `src/ifd.rs` imports only `crate::Error` + (tests)
`super::*`; `[dependencies]` is empty; `tools.rs` uses `std` only; `dnglab` (LGPL) appears only as
a spawned tool and is **dropped** from the port. No copyleft rides along (brief §5).

---

## 6. Gates, lints, CI

- `#![forbid(unsafe_code)]` + the **five** lints, copied verbatim from `src/lib.rs:95-102`
  (`unwrap_used`, `expect_used`, `indexing_slicing`, `panic`, `arithmetic_side_effects`). NOTE:
  the design §2 calls `arithmetic_side_effects` "the fifth lint to add" — but irradiance already
  denies all five (brief §0 table). Nothing to extend; copy as-is.
- `#[allow]` beneath the crate root forbidden; the `-F` re-check runs `--lib` so the `#[cfg(test)]`
  module's scoped `#[allow]` stays invisible to it (brief §1.3) — keep that shape.
- `deny.toml`: irradiance's allow-list verbatim (MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception,
  BSD-2/3-Clause, Zlib, 0BSD, Unicode-3.0; the `libfuzzer-sys` NCSA exception is needed by any
  `cargo fuzz` graph, not RAW-specific). irradiance's RAW-oracle dev-deps (`toml`, `ssimulacra2`)
  do NOT carry. Add `thiserror` to the reasoning (brief §3.3).
- **New CI gate:** `cargo tree -p <crate> | grep -- -sys` must match zero (design §11) — does not
  exist in irradiance (its `[dependencies]` is empty).
- The Homebrew-cargo `+toolchain`/clippy-shadow traps carry (brief §4; host facts of the
  maintainer's Mac — re-verify per host, do NOT carry the version numbers as facts).

---

## 7. What NOT to port (brief §6)

`src/plane.rs`, `develop.rs`, `demosaic.rs`, `color.rs`, `tone.rs`+`tone_table.rs`, `warp.rs`,
`opcode.rs`, `src/bin/irr.rs` (diamond-exif's CLI is `exif-audit`), `tests/support/perturb.rs`
(pixel-domain — does NOT port), the SSIMULACRA2/perceptual/tone oracles, the tier-B corpus
machinery (`corpus.rs`/`pnm.rs`/`md5.rs`/manifest), the stays-module oracles + fuzz targets, the
dnglab wrappers (`tools.rs:542-800`), and the DNG-measurement docs (copy the *method*, not the
file). The DNG tag consts/`Sensor`/`Cfa`/`ActiveArea` and `is_structural_tag`'s body all STAY.

---

## 8. Open decisions / blockers (for orchestrator + maintainer)

1. **BLOCKER — repo path.** Hold all file creation until relayed. (orchestrator is on it)
2. **Scoping call A** — single-crate now vs workspace now (recommend single-crate; §1).
3. **Scoping call B** — confirm gate harness cloned from irradiance, not the template (§1).
4. **G3** — exiftool `-j` JSON (needs a dev-dep + DEC) vs the tab `-T -n -s3` form + a normalization
   table. Design §8 revised to the tab form; recommend keeping it (no JSON dep).
5. **G4** — `gh release list -R jysf/spec-driven-template` to settle template drift before cloning.
6. **Unmeasured (design §16):** ExifTool **write-mode** exit codes — read-side exits 0 on truncated
   input; measure before building the I3 write-parity check (that's M1c, not M1a).

---

## 9. Concrete next steps for the session that receives the repo path

1. Inspect what's already in the repo (bare `git init`? template clone? empty?) before scaffolding.
2. Confirm scoping calls A+B with the maintainer (via orchestrator) — they change the scaffold shape.
3. Scaffold (Step 0) → write DEC-001..004 (Step 1) → port the strict skeleton + ported hostile
   tests (Step 2), green. That is a clean first-session deliverable.
4. Then the tolerant-walk policy (Step 3), multi-IFD + wider types (Step 4), oracle discipline
   (Step 5) as follow-on specs.
5. Add a minimal one-line note to irradiance `docs/future-plans.md` that the extraction has begun
   (the ONLY permitted irradiance edit) — deferred this session per the READ-base instruction.
