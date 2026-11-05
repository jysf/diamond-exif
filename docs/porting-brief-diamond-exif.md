# Porting brief: irradiance → diamond-exif

**Status:** design only. No source file was changed to write this.
**Written:** 2026-11-03, against `origin/main` at `a8b9af8`, by reading the files below at that commit.
**Read against:** `exif-core-design.md` at commit `289de90` on branch `worktree-exif-design`
(read from `~/PSeven/experiments/crustimg_redo_plus/throwaway-irradiance/.agents/worktrees/exif-design/`).
Section numbers written `design §N` refer to that file. Module paths below are its §4 layout.

Every `file:line` here was read at the moment the sentence was written. Where a
claim could not be verified, it says so.

> **Scope of the word "port".** Nothing is linked. diamond-exif is a new repo; "port" means
> *copy a file's text, then edit it*. Provenance of that copy is §5.

---

## 0. The three biggest porting risks (read this first)

1. **`ifd.rs` is a strict, read-only, DNG-shaped walker; diamond-exif needs a tolerant,
   multi-IFD, ASCII-aware reader that can later write.** Four things the design assumes
   exist do not:
   - **No ASCII reader and no signed/float readers.** `Container::uints` accepts only
     BYTE/SHORT/LONG/IFD/UNDEFINED/RATIONAL (`src/ifd.rs:1039`); `rationals` accepts
     RATIONAL/SRATIONAL (`:1143`). `type_size` *knows* all twelve sizes (`:207-215`) but
     nothing decodes ASCII, SBYTE, SSHORT, SLONG, FLOAT or DOUBLE. EXIF's `Make`, `Model`,
     `Artist`, `Copyright` and `DateTimeOriginal` are ASCII. `types.rs` is largely **new code**.
   - **Only `SubIFDs` (tag 330) is followed** (`:979-984`). ExifIFD (0x8769), GPS (0x8825),
     Interop (0xA005) and MakerNote are never walked.
   - **The walk is fatal-on-first-error by design** (`DEC-012`, `:976-978`: "a malformed
     `SubIFDs` entry is fatal to the whole container"). `Container::parse` returns `Err` for
     any bad IFD (`:855`). For an upload-sanitiser, one corrupt GPS IFD must not erase the
     readable EXIF IFD. That is a **policy change** that flips the semantics of about ten
     passing tests, not an edit.
   - **There is no write path or encoder.** `Entry`'s fields are private and `Entry::parse`
     is private (`:339-344`, `:370`); `ByteOrder` only *decodes* (`:269-294`). The only
     byte encoders in the repo live in test code (`src/ifd.rs:1735+` `Build`,
     `tests/support/tiff.rs:26+` `Order`). Design §7's `tiff_apply.rs` has nothing to port.
2. **The `Bytes` zero-copy claim in the design disagrees with the code.** `Container<'a>`
   borrows `&'a [u8]` (`:794-799`), `payload` returns a slice tied to `&self` (`:994`), and
   `uints`/`rationals` allocate a `Vec` (`:1025`, `:1139`). Only the 12-byte entry headers are
   copied (`:791`). Adopting `bytes::Bytes` (design §3) changes every signature that returns a
   borrow. Recommendation in §1.4: keep `&[u8]` inside `tiff/`, use `Bytes` only at the API edge.
3. **The gates you want to clone are not in the template.** 6 of the 52 `scripts/*.sh` here —
   exactly `lint-red-proof`, `lint-fuzz-red-proof`, `gate-audit`, `gate-audit-red-proof`,
   `cost-audit-red-proof`, `validate-red-proof` — do not exist in the local template clone,
   and 8 more differ from it, including the `_lib.sh` fixes that make the gates fail through
   their own `die` (§3). A fresh template-based repo inherits the *unfixed* versions.

**Where the design doc is contradicted by this repo's code** (all detailed below):

| Design says | Repo shows |
|---|---|
| `reader.rs`: cursor over `Bytes` with u8/u16/u32/u64 gets (§4) | free fns `read_u16`/`read_u32` over `&[u8]` only (`:298`, `:309`); no u8/u64 |
| `types.rs`: type system BYTE..DOUBLE with bounds-checked reads (§4) | sizes for all 12 types, decoders for 6 of them |
| Port is "provenance class 1" (§2) | ledger class 1 is *a published specification*; our own prior code is **class 3** (§5) |
| "extend to five lints … this repo's experience justifies the fifth" (§2) | `src/lib.rs:96-102` already denies all five; nothing to extend |
| I2 uses `exiftool -j` (§8) | `tests/support/tools.rs:120` uses `-T -n -s3`, deliberately with no JSON parser; `Field.values` is `Option<Vec<u32>>` (`:102-106`) — numeric only |
| red-proof helpers in `tests/support/perturb.rs` (task brief) | `perturb.rs` is pixel-domain only; the byte/source red-proofs live elsewhere (§2.3) |

---

## 1. The `src/ifd.rs` port map

`src/ifd.rs` is 3,209 lines (matches design §2): code `1-1715`, `#[cfg(test)] mod tests`
`1716-3209` (59 `#[test]` fns). Its only import is `use crate::Error;` (`:52`) — **no external
crate**, and `core` only (verified: `grep '^use ' src/ifd.rs` → `:52` and the test-only `:1729`).

### 1.1 Item table

Classes: **CLEAN** = ports unchanged · **EDIT** = ports after the named edit · **STAYS** = DNG/RAW domain.

| Lines | Item | Class | Destination / edit |
|---|---|---|---|
| 63, 69, 75 | `MAX_IFD_DEPTH` 8, `MAX_IFDS` 512, `MAX_TAG_VALUES` 1<<20 | CLEAN | `tiff/mod.rs`. Re-justify numbers: the depth doc cites "TIFF/EP and Exif chains add a second" level (`:60`); Exif→Interop→MakerNote is deeper than DNG's. 8 still holds; say so |
| 78, 81 | `TIFF_VERSION` 42, `ENTRY_SIZE` 12 | CLEAN | `tiff/mod.rs` |
| 93-115 | TIFF-baseline tag consts (`NEW_SUBFILE_TYPE`…`SUB_IFDS`) | EDIT | `tiff/tags.rs`; keep only tags EXIF/TIFF define, add the EXIF/GPS/Interop pointer tags. `TAG_ORIENTATION` (107) is also an EXIF tag and ports |
| 116-175 | DNG/CFA/colour tag consts | STAYS | RAW |
| 179, 184 | `PHOTOMETRIC_LINEAR_RAW`, `PHOTOMETRIC_CFA` | STAYS | |
| 187-201 | `TYPE_*` consts | EDIT | `tiff/types.rs`; **add** `ASCII`(2), `SBYTE`(6), `SSHORT`(8), `SLONG`(9), `FLOAT`(11), `DOUBLE`(12) — currently unnamed (`:186` "only the ones this module reads as integers are named") |
| 207-215 | `type_size` | CLEAN | `tiff/types.rs`. Already total over the 12 TIFF types |
| 239-254 | `is_structural_tag` | **STAYS, then rewritten** | The list is `DEC-012`'s DNG structural set (`:233`). diamond-exif needs its *own* structural set (IFD pointers, `StripOffsets` for thumbnails). Do not copy the body |
| 262-267 | `ByteOrder` | CLEAN | `reader.rs` (design puts the cursor there; see gap G1) |
| 269-294 | `ByteOrder::{u16,u32,i32}` (private) | EDIT | `reader.rs`; make `pub(crate)`; **add** `u64`/`f32`/`f64`/`i16`, and **encode** twins for the writer |
| 298, 309 | `read_u16`, `read_u32` | CLEAN | `reader.rs`; add `read_u8`, `read_u64`. The `.get().try_into()` idiom (`:296-305`) is the template for them |
| 322 | `offset_to_usize` | CLEAN | `reader.rs` |
| 339-423 | `Entry`, `Entry::parse` | EDIT | `tiff/mod.rs`. Add a `pub(crate)` constructor + raw-value accessor for the writer; `parse` (370) is verbose but correct — do not "simplify" it, the `.get().ok_or().try_into()` shape is what `clippy::indexing_slicing` accepts |
| 428-472 | `Ifd` + accessors | EDIT | `tiff/mod.rs`. Doc at `:467-469` ("which is all the opcode lists need") is DNG prose — reword |
| 480-512 | `Compression` | STAYS | Only DNG codes 1/7; EXIF thumbnails use 6. diamond-exif needs a different enum |
| 523-785 | `ActiveArea`, `DefaultCrop*`, `Cfa`, `Sensor`, `impl Sensor` | STAYS | RAW |
| 573+ | `Rational {num: i32, den: i32}` | **EDIT (type changes)** | EXIF `RATIONAL` is *unsigned* 32/32. `rationals` narrows via `i32::try_from` and errors above `i32::MAX` (`:1179-1186`). A metadata reader must hold `URational(u32,u32)` and `SRational(i32,i32)` separately. Port the *shape* (verbatim, never divide — `:1135-1138`), not the struct |
| 794-799 | `Container<'a>` fields | EDIT | `tiff/mod.rs`; see §1.4 |
| 812-821 | `SensorMatch` | STAYS | |
| 830-857 | `Container::parse` | EDIT | `tiff/mod.rs`. **Policy edit:** return a tolerant result carrying per-IFD errors (design says typed `Error`; this needs a `Warning` channel — gap G2). Also: `parse(data)` already works on a TIFF blob that starts at its own byte 0, which is what a JPEG APP1 / PNG `eXIf` slice gives it, because all offsets are blob-relative (`:846`, `:1011`). Container code must hand it the slice *after* `Exif\0\0` |
| 860-878 | `byte_order`, `ifd0_offset`, `ifds`, `ifd0` | CLEAN | `tiff/mod.rs` |
| 887-934 | `walk_chain` | EDIT | `tiff/mod.rs`. Cycle guard (`visited: Vec<u32>`, `:904`) is **linear `contains`** — fine at `MAX_IFDS`=512, state it. Recursion only via `sub_ifd_offsets_of_last` (`:926`); generalise to "child-IFD pointer tags" |
| 937-969 | `read_ifd` | CLEAN | `tiff/mod.rs`. `as_chunks::<ENTRY_SIZE>` (`:963`) makes the 12-byte width a type fact |
| 979-984 | `sub_ifd_offsets_of_last` | EDIT | the hard-wired `TAG_SUB_IFDS`; becomes the pointer-tag dispatcher |
| 994-1015 | `payload` | EDIT | `tiff/mod.rs`. The one place the hostile `count × sizeof` is computed (`:999-1002`). Port as-is; change only the return lifetime per §1.4 |
| 1025-1120 | `uints` | EDIT | `tiff/types.rs`. Remove the DNG per-tag RATIONAL gate (`:1032`) and the integer-division rule (`:1078-1107`); add ASCII/signed/float decoders |
| 1139-1193 | `rationals` | EDIT | `tiff/types.rs`; see `Rational` row |
| 1196-1214 | `scalar`, `required_scalar`, `values` | CLEAN | `tiff/mod.rs` |
| 1224-1243 | `array<N>` | EDIT | generic "fixed-length tag, wrong length costs the field". Reuse the idea (EXIF has fixed-length tags) but it calls `uints` and a `malformed` out-param (`:1228`) |
| 1254-1273 | `cfa_bytes` | STAYS | |
| 1302-1396 | `is_sensor_ifd`, `scan_sensor`, `no_sensor_ifd_error`, `sensor_candidates`, `sensor_ifd` | STAYS | |
| 1404-1416 | `cost_the_field` | EDIT | **Keep.** It is the `DEC-012` "interpretation tolerance" primitive and is generic over `T`; it is the exact tool the tolerant EXIF read needs |
| 1423-1445 | `ifd0_rationals`, `ifd0_scalar` | EDIT | small helpers; IFD0-specific only by name |
| 1451-1490 | `parse_cfa` | STAYS | |
| 1491-1715 | `Container::sensor` | STAYS | |

### 1.2 Coupling points (where the porting effort hides)

Everything marked CLEAN above reaches, directly or transitively, into a STAYS item at these seams:

| # | Coupling | Cheapest severance |
|---|---|---|
| C1 | `use crate::Error` (`:52`). The crate-wide `Error` (`src/lib.rs:144`, `#[non_exhaustive]`, about 40 variants through `:560`) mixes TIFF, plane, develop, warp, demosaic and colour errors. `ifd.rs` names 16 of them (`Truncated`×28, `UnexpectedFieldType`×20, `ValueOverflow`, `CyclicIfd`, `MalformedRationalValue`, `TagTooLarge`, `MissingTag`, `IfdDepthExceeded`, `UnsupportedTiffVersion`, `TooManyIfds`, `RationalValueOutOfRange`, `NotTiff`, `OffsetOutOfRange`, plus the STAYS-only `NoSensorIfd*`, `UnsupportedCompression`) | **Copy** the 13 TIFF-level variants and their `Display` into `error.rs`. A trait extraction buys nothing for a 13-variant enum |
| C2 | `walk_chain` → `sub_ifd_offsets_of_last` → `uints` → `is_structural_tag` → DNG tag consts | **Duplicate a small helper:** a `fn is_structural(tag) -> bool` in `tiff/tags.rs` with diamond-exif's own list. Do not port `is_structural_tag`'s body |
| C3 | `uints` bakes in DNG's "RATIONAL divides to an integer" rule (`:1078-1107`) and the per-tag gate (`:1032`) | Delete both. Write `types.rs` readers that never divide |
| C4 | One `impl<'a> Container<'a>` block spans `823-1715`; generic and DNG methods interleave (`array` 1224 and `cfa_bytes` 1254 sit between generic readers and sensor selection) | Rust allows multiple `impl` blocks. Cut at: generic = `823-1215`, helpers `1224` + `1404-1445` (EDIT), DNG = the rest |
| C5 | `Sensor`/`Cfa`/`ActiveArea` appear in the unit-test module and `tests/support/tools.rs:23-26` | Tests: §1.3. `tools.rs`: §2.2 |
| C6 | `src/ifd.rs` has **no dependency on `plane.rs`/`develop.rs`** (verified: its only `use` is `crate::Error`) | Nothing to sever; this is the good news |

### 1.3 Unit-test situation (59 tests, `src/ifd.rs:1728-3209`)

All sit in `mod tests` (`#[cfg(test)]` at `:1716`), which opts out of the five lints with a
scoped `#[allow]` (`:1717-1727`). diamond-exif's `-F` re-check runs `--lib` precisely so this
module is invisible to it (`app.just:167-170`, quoted in the allow comment). Keep that shape.

| Range | Tests | Disposition |
|---|---|---|
| `1735-1860` | `Build` byte-level builder | **Port.** It is deliberately independent of `tests/support/tiff.rs` (`:1731-1733`: "a shared builder would let one bug hide the other"). It is also the only *encoder* in the library crate's tests |
| `2190-2410` (17 tests) | header both byte orders, empty input, non-TIFF mark, wrong version, IFD0 offset 0, offset/count/payload past EOF, count-overflow product, inline value, unknown type, self-ref / two-hop / chain cycles, depth limit, long chain | **Port as-is**, except `2322` (uses `TAG_BLACK_LEVEL`, a DNG tag — swap for an EXIF tag, keep the SRATIONAL-refused assertion only if your `uints` still refuses it) |
| `2063`, `2097` | SRATIONAL / RATIONAL read verbatim | **Port with edit** to EXIF tags and the new `URational`/`SRational` types |
| `2969` | SubIFD offsets walked in order with parent recorded | **Port with edit** — becomes the Exif/GPS/Interop pointer-order test |
| `2996` | every error variant has a non-empty `Display` | **Port with edit**: variant list must match diamond-exif's `Error` (`:2999-3022` lists 15) |
| `1914-2189` other 10 (CFA, colour matrix, black level) | | **Stay** |
| `2411-2530` | sensor selection by rule, compression refused, layer-0 packing | **Stay** |
| `2544-2700`, `3035`, `3128` | malformed tag / structural-tag fatality (`DEC-012`) | **Rewrite**: the *mechanism* (structure fatal, interpretation costs the field) is the thing to carry; the tag lists and the "structural is fatal" expectations flip under the tolerant-walk policy (G2) |
| `2737-2830` | RATIONAL `DefaultCropSize`, denominator really divided | **Stay** (DNG integer-division rule) |
| `2849`, `2860-2960`, `3144`, `3178` | opcode presence, Orientation-from-IFD0 for a SubIFD plane | **Stay** |

Counts: 17 clean + 12 in `1914-2189` = 29; the other 30 are in `2411-3209`. I did not
individually re-run each of the 30; the table assigns ranges by reading test names and the
header comments, not by executing each. Run `cargo test --lib ifd::` after the port.

### 1.4 The `Bytes` vs `&[u8]` decision (design §3 vs the code)

`Container<'a>` is a borrowing view (`:794-799`); `payload` is `fn payload<'s>(&'s self, …) -> &'s [u8]` (`:994`).
With `bytes::Bytes` the natural signature returns an owned, refcounted slice. **Recommendation:**
keep the walker over `&[u8]` (so `ifd.rs`'s lifetimes and its tests port unchanged) and have
`api.rs` hold the `Bytes` and lend `&[u8]` into `tiff/`. Note the *new* lifetime requirement from
the irradiance side (PROJ-004 Stage A) is the same one: a slice accessor returning `&'a [u8]`
tied to the **input**, not `&self`. Make `payload`'s result `&'a [u8]` in the port; it is a
one-token change because the data is already `&'a [u8]` (`:795`) and `Entry` is `Copy`.

---

## 2. Test-support tooling

### 2.1 `tests/support/tiff.rs` (627 lines) — hand-built fixtures

Own work, byte by byte from TIFF 6.0 §2, tier A (`:1-8`). Compiled twice via `#[path]`: into
`tests/ifd_reader.rs` and `examples/fuzz-seeds.rs` (`:10-16`, `examples/fuzz-seeds.rs:40-42`).

**Adapts directly:** `Order` (21-47), `Entry` (50), `short`/`long`/`at_offset` (63-96),
`Ifd::new` (106), `tiff()` (120, the assembler), `inline_bytes`, `two_shorts` (194, 206), and the
whole hostile set `347-575`: `truncated_header`, `truncated_ifd0_offset`, `bad_magic`,
`bad_version`, `ifd0_past_eof`, `ifd0_inside_header`, `entry_count_past_eof`,
`payload_offset_past_eof`, `count_overflow`, `self_referential_subifd`, `subifd_cycle_two`,
`chain_cycle`, `long_cycle`, `deep_subifd_nest`, `unknown_field_type`, `zero_entries`, `no_ifd0`,
`empty`.

**Stays:** the DNG tag consts (155-180), `sensor_entries`/`cfa_sensor_entries`/`valid_sensor`/
`valid_cfa_sensor`/`cfa_dng_16bit` (220-326), `valid_subifd` (335),
`malformed_black_level_repeat_dim`, `rational_default_crop_size`,
`malformed_photometric_*` (487-560).

**Carry the idiom, not just the code:** `all()` (`:581`) is one list of **29** named fixtures consumed
by both the hostile-input test *and* the fuzz-seed writer, "so a new fixture cannot land in one and be
forgotten by the other" (`:577-580`). That single-source rule is the most reusable thing in the file.

**Fixture variants diamond-exif needs that this repo never built** (recommendations, not built):
- *JPEG:* SOI-only; APP1 with 65,533-byte payload (the cap, design §6) and 65,534; two APP1 (Exif +
  XMP); extended-XMP pair with GUID/digest; APP1 length field pointing past EOF; APP1 length 0/1/2;
  segment with `0xFF` fill bytes; missing SOS; SOS with no EOI; Exif block whose TIFF header is
  `MM` while a second APP1 is `II`.
- *PNG:* `eXIf` before vs after `IDAT`; bad chunk CRC; chunk length > remaining; `iTXt` XMP with
  compression flag set; missing `IEND`.
- *WebP:* VP8X flags say EXIF but no chunk; odd-length chunk with and without the pad byte; RIFF
  size field ≠ file size (both directions).
- *Cross-container:* the same TIFF/EXIF blob wrapped in each container, so one parity row can
  assert container-independence.
- *TIFF-side:* ExifIFD pointer cycles back to IFD0; GPS pointer past EOF with a readable ExifIFD
  (the G2 tolerance case); ASCII count 0, unterminated ASCII, ASCII with embedded NUL;
  UNDEFINED `UserComment` with each 8-byte charset prefix; MakerNote with offsets relative to
  itself vs to the file.

### 2.2 `tests/support/tools.rs` (910 lines) — tool shells

| Lines | Item | Disposition |
|---|---|---|
| 36-62 | `ToolError {NotOnPath, Exec, Parse}` + `Display` | **Ports.** `NotOnPath` is the "loud skip names the tool" mechanism |
| 64-80 | `run_tool` — **single choke point** for every tool spawn; `ErrorKind::NotFound` → `NotOnPath` (`:65-79`) | **Ports.** The doc comment (`:60-63`) is the design: one guard, not two |
| 83-91 | `exiftool_available`, `dnglab_available` | exiftool ports; **drop dnglab** |
| 100-127 | `Field`, `exiftool()`, `run_exiftool` (`-T -n -s3`, `:120`) | **Ports with edit.** `Field.values: Option<Vec<u32>>` (`:102-106`) is numeric-only; diamond-exif needs strings, rationals, floats, lists |
| 129-190 | `parse_fields` — pure, replayable against committed text (`:123-128`) | **Ports.** This is what makes the tier-A red-proof possible with no tool |
| 192-540 | `ToolValue`, `SENSOR_TAGS`, `ToolReading`, `diff`, `Mismatch`, `compare_optional` | **Stays** (DNG `Sensor` diff; imports `irradiance::ifd::{…}` at `:23`) |
| 542-800 | `dnglab_*`, `unique_*` JSON key scrapers | **Stays**; diamond-exif's oracle is exiftool only |

**Documented traps already encoded** (`tools.rs:14-18`, `:107-110`): `exiftool` exits **0** on a truncated
file and on an absent tag — "only stdout is trusted"; `dnglab` exits 2 on truncated input. The wrappers
therefore never check exiftool's status. ⚠ **This is a *read*-side measurement.** diamond-exif's I3 runs
exiftool in *write* mode (`-Orientation=6 -o …`, design §8). Whether exiftool's write-mode exit status
carries signal was not measured here and must not be assumed to match — measure it first (rule 4 below).

**Design gap:** design §8 I2 says `exiftool -j`. `tools.rs` uses tab-separated `-T -n -s3` precisely so the
test code needs **no JSON dependency** (`:1-6`). Design §3 lists only `bytes` + `thiserror` as dependencies and
is silent on dev-dependencies. Using `-j` means a JSON reader in tests (e.g. `serde_json`) — that is a written
decision under the `no-new-top-level-deps-without-decision` constraint, or keep the tab form and build the
normalization table (design risk 1) on top of it. Also note `-n` suppresses print conversion; the
normalization table will need both numeric and printed forms for some tags.

**Skip idiom** (port verbatim): `eprintln!("SKIP <test> — exiftool not on PATH")` and return, e.g.
`tests/metadata_oracle.rs:138`. A skip is a printed line, never a silent pass (`tests/support/corpus.rs:22`,
`:307`).

### 2.3 `tests/support/perturb.rs` (189 lines) — **not** the red-proof helper the brief assumes

It is **pixel-domain only**, written for the develop oracle's SSIMULACRA2 tier-A proof (`:1-9`):
`shift_horizontal` (16), `apply_linear_gamma` (51), `apply_missing_radial_warp` (97), with the sRGB
curve and Q2M warp constants (`:83-86`). **Do not port it.** It generalises to nothing byte-level.

What the red-proof discipline *actually* uses in this repo, and what to carry:

1. **Perturb the compared value, replay the committed tool text.** `tests/metadata_oracle.rs:458-545`:
   `include_str!` of a committed `exiftool` output line (`tests/oracle-fixtures/exiftool-*.txt`),
   parsed by the **same** `parse_fields` code a live run uses; `oracle_is_clean_on_an_unmodified_reading`
   (`:522`) is the control, `oracle_names_the_one_field_that_was_perturbed` (`:531`) sets one field wrong and
   asserts **exactly one** mismatch naming it. Runs in CI with no tool and no corpus. → *the model for I2's red-proof.*
2. **Mutated-copy apparatus** (inject a fault into a *copy* of the library source, build, run, assert the output
   changed): `tests/plane_oracle.rs` — `TempDir` (626), `inject_chunk_extraction_fault` (676),
   `stage_probe_crate` (735), `build_and_run_probe` (768), driven by `:805` with a **control** at `:1044`
   (`the_bayer_red_proof_control_is_green`). ⚠ `stage_probe_crate`/`build_and_run_probe` are **copy-pasted
   three times** (`plane_oracle.rs:735,768`; `develop.rs:967,996`; `develop_oracle.rs:695,724`). Lift into
   `tests/support/` once, in diamond-exif, before the second use.
3. **Control run first.** `DEC-009` (`scripts/lint-red-proof.sh:22-29`): assert the *unmutated* copy is green, or a red result
   proves nothing.

For design §8's three red-proofs — corrupt an IFD offset (I2), truncate between APP1 and SOS (typed error),
flip a pixel byte (I1) — **none of the three byte-mutation helpers exists here.** Write a small
`tests/support/mutate.rs`: `set_u32_at`, `flip_bit`, `truncate_at`, `swap_byte_order_mark`. They are ~20 lines.

### 2.4 Fuzz target

`fuzz/fuzz_targets/ifd.rs` (the `tiff_ifd` target) ports nearly verbatim: it calls `Container::parse` and
then walks every IFD/entry and resolves each payload "because payload resolution is where the hostile
`count × sizeof` product actually gets used" (`:38-41`) — keep that, it is the half-reader trap. Its doc
carries the `+toolchain` PATH trap (`:10-18`, see §4). Seeds come from `tiff.rs::all()` via
`examples/fuzz-seeds.rs`; no corpus file is a seed (`DEC-003`).

---

## 3. Gates and scripts

### 3.1 Template version and drift

- This repo: `VERSION` = **0.6.38** — *template provenance, not the app version* (`.repo-context.yaml:76-79`).
- Local clone of `jysf/spec-driven-template` at `~/PSeven/spec-driven-template/spec-driven-template`:
  `VERSION` = **0.6.39**, HEAD `22aec2e`, dated 2026-09-04.
- ⚠ **I could not compare to the current GitHub release.** `gh api repos/jysf/spec-driven-template/…` returned
  "error connecting to api.github.com" (twice). The local clone may be stale. *Before cloning, run
  `gh release list -R jysf/spec-driven-template` and compare to 0.6.39.*

Comparing `scripts/*.sh` here (52 files) against the local clone's 46 (`cmp`, whole file):

| Class | Count | Files |
|---|---|---|
| Byte-identical to template | 38 | everything not listed below |
| **Only here** (not in template) | **6** | `lint-red-proof`, `lint-fuzz-red-proof`, `gate-audit`, `gate-audit-red-proof`, `cost-audit-red-proof`, `validate-red-proof` |
| Differs from template | 8 | `_lib.sh` (225 diff lines), `validate.sh` (114), `test.sh` (66), `cost-audit.sh` (41), `handback-sync.sh` (22), `archive-spec.sh` (21), `specs-by-stage.sh` (18), `decisions-audit.sh` (8) |

The `_lib.sh` differences I read are real bug fixes found here, **not yet upstream** in 0.6.39: a `|| true`
guard on the project-resolution `grep` (§16 rule 3), `next_id` excluding `tests/` (it had minted `SPEC-901`),
a YAML-comment strip that no longer truncates quoted `notes:` containing `#`, and the front-matter
**parse** gate (SPEC-016). **A fresh repo from the template gets the unfixed `_lib.sh` and `validate.sh`.**
Recommendation: copy this repo's `_lib.sh`, `validate.sh`, `cost-audit.sh`, `handback-sync.sh` over the
template's, and upstream the fixes to the template; otherwise diamond-exif re-earns them.
(I did not diff the other four in detail — `decisions-audit`, `archive-spec`, `specs-by-stage`, `test.sh` —
and cannot say whether they are fixes or irradiance-specific.)

### 3.2 What clones, and what is coupled

| Script | Verdict | Coupling found |
|---|---|---|
| `lint-red-proof.sh` (351) | **Clone, 3 edits** | `LIB_RS="${REPO_ROOT}/src/lib.rs"` (`:80`) → `crates/exif-core/src/lib.rs`; copies `Cargo.toml`/`Cargo.lock`/`src/` of a *single-crate* repo (`:10`) → needs workspace-aware staging; injects five violations after the last inner attribute of `lib.rs` (`:14`). The **clippy-shadowing guard** (`:95-105`: Homebrew's `cargo-clippy` shadows rustup's) is host-specific and stays |
| `lint-fuzz-red-proof.sh` (263) | **Clone, edit** | assumes `fuzz/` is a second cargo graph outside the library (`DEC-011`/`DEC-027`, `:9-13`). design §3 puts `fuzz/` inside `crates/exif-core/`; the `--manifest-path fuzz/Cargo.toml` path changes |
| `gate-audit.sh` (381) + `gate-audit-red-proof.sh` (130) | **Clone verbatim** | scans `scripts/*.sh` for the three mute-death shapes (`:14-37`); script-only. The red-proof checks `for f in .repo-context.yaml VERSION justfile app.just AGENTS.md guidance` exist (`:46`) — i.e. it assumes the template's file layout |
| `cost-audit.sh` (168), `cost-audit-red-proof.sh` (172) | **Clone, reset constants** | `STAGE_ORCH_COST_GRANDFATHERED="STAGE-001"` default (`:105`) and `COST_AUDIT_GRANDFATHERED` in `_lib.sh` — empty them for a fresh repo; the DEC-005 "no token meter" branch (`:28`, `:37`) is platform-specific |
| `validate.sh` (418), `validate-red-proof.sh` (94) | **Clone** | the parse gate; the red-proof's fixture is `tests/oracle-fixtures/SPEC-900-unparseable-front-matter.md` — carry that file too |
| `_lib.sh` (1894) | **Clone this repo's, not the template's** | see §3.1 |

All of the above are driven by `just` targets in `app.just` (`lint-red-proof` 108, `gate-audit` 141,
`gate-audit-red-proof` 150, `lint-no-allow` 167, `lint-ci` 188, `lint-fuzz` 215) — so cloning scripts without
`app.just` leaves them unwired. CI runs them (`.github/workflows/ci.yml`, 16.9K); I did not trace which jobs call which.

### 3.3 `deny.toml`

Carries over **as-is**: the allow-list (`MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`,
`BSD-3-Clause`, `Zlib`, `0BSD`, `Unicode-3.0`), `all-features = true`, `confidence-threshold = 0.9`. One RAW-adjacent line:
the `libfuzzer-sys` `NCSA` exception — **not RAW-specific**, it is needed by any `cargo fuzz` graph. The
`[dev-dependencies]` in `Cargo.toml` (`toml`, `ssimulacra2`) are RAW-oracle-specific and do **not** carry.
Add `bytes`/`thiserror` to the allow reasoning; design §11 also wants `cargo tree -p exif-core | grep -- -sys` to
match zero — that check does not exist here (this repo's `[dependencies]` is empty), so it is new CI.

---

## 4. Discipline text to inherit

Re-copying is cheaper than re-earning: the four rules below are recorded in `AGENTS.md` as having cost, between
them, "a false green that shipped a panic past seven gates, … a blocking constraint's CI gate that was dark
for **17 consecutive runs**, and one unverified reading of one file propagating into three separate documents"
(`AGENTS.md:1409-1417`).

| Carry | Source | Note |
|---|---|---|
| The four disciplines (panic-free, oracle-red, provenance, permissive deps) | `AGENTS.md:36-58` | Replace "RAW is attacker-influenced binary" with EXIF-in-upload wording; keep the rest |
| **Rule 1** — `measurement-over-generalised`: a probe result becomes a claim only with *the exact command and its scope*; delete or second-point any generalising word | `AGENTS.md:1418-1430` | |
| **Rule 2** — assert the match count: pattern-matching tooling anchors deliberately, excludes comments, **asserts how many times it matched**, never takes the first hit | `AGENTS.md:1431-1440` | |
| **Rule 3** — a gate fails through its *own* `die`: every result-bearing `grep` guarded `|| true` and counted; "guarding one `grep` in a pipeline is not enough" | `AGENTS.md:1441-1458` | The two measured traps in it are the valuable part |
| **Rule 4** — a claim about a file is verified by *running the reader* at the moment of writing, **corrections included** | `AGENTS.md:1459-1486` | Applies directly to every exiftool-parity statement |
| "Measured once, on one file is not 1.0" | `AGENTS.md:1487-1492` | Confidence discipline |
| The measured `+toolchain` trap + commands | `AGENTS.md:234-247`; fuller in `guidance/toolchain-brief.md:17-40` | Verbatim block below |

**The toolchain block to carry** (`AGENTS.md:234-247`; *host facts of the maintainer's Mac, measured 2026-08-15/16 — re-verify per host*):
`cargo` on `PATH` is Homebrew's, which does not understand `+nightly`; `cargo fuzz` shells out to a bare
`cargo build`, so both the outer **and** inner command must resolve to the rustup shim:

```bash
PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run <target> <corpus> <seeds> -- -max_total_time=60
```

Design §15 already has this exact line. Also carry the `lint-ci` pin: bare `cargo clippy` is Homebrew's older
clippy while CI floats on `stable`, and that gap kept CI red for four specs (`app.just:172-175`).
Do **not** carry the specific version numbers (1.97.1 / 0.7.2 / 13.55) as facts; they are this machine's.

---

## 5. Provenance and licensing

**Class.** Design §2 says the port is "provenance: own code, class 1". Under this repo's ledger
(`docs/provenance-ledger.md:17-30`) **class 1 is "a published specification"** and **class 3 is "our own prior code"**.
The port is class 3 *derived from* class 1: `src/ifd.rs` itself is recorded as written from TIFF 6.0 and the Adobe DNG
spec, "not from any implementation" (`src/ifd.rs:11-17`, ledger row `:39`). Record both.

Suggested first rows for diamond-exif's `docs/provenance-ledger.md`:

| Item | Source | Class | Licence |
|---|---|---|---|
| `tiff/*`, `reader.rs` — IFD walk, bounds-checked reads, cycle guards | **Class 3:** irradiance `src/ifd.rs` @ `a8b9af8`, same author. **Upstream class 1:** TIFF 6.0 (1992) §2 | 3 (→1) | MIT OR Apache-2.0, own code |
| `tests/support/tools.rs` subset — subprocess-oracle pattern | Class 3: irradiance, same author. exiftool is **run, never linked** | 3 | own code; exiftool imposes nothing |
| `containers/jpeg.rs`, `png.rs`, `webp.rs`, `heic.rs`, `xmp.rs`, `iptc.rs`, `icc.rs` | **New.** Class 1 only: CIPA DC-008 (Exif), ITU T.81 / JFIF, PNG spec, WebP container spec, ISO/IEC 14496-12, Adobe XMP spec, IPTC IIM, ICC spec. Row **must name the exact document and edition** | 1 | — |

⚠ The ledger rule is "reading a copyleft implementation is not permitted" (`docs/provenance-ledger.md:28-30`). exiftool is Perl under
Artistic/GPL: the oracle may be *run*; its source must not be read to resolve a format question. Say so in the
new ledger's header.

**Copyleft in the ported code — checked, not assumed.** `src/ifd.rs` imports only `crate::Error` and (tests)
`super::*` (`:52`, `:1729`); `[dependencies]` in `Cargo.toml` is empty. `tests/support/tiff.rs` and the
`tools.rs` subset I mapped use `std` only (`tools.rs:20-21`). `dnglab` (LGPL-2.1) appears only as a spawned tool and is
excluded from the port. `tools.rs:23` `use irradiance::ifd::{…}` is an *internal* import, not a dependency. No copyleft rides along.

---

## 6. What NOT to port

| Path | Why |
|---|---|
| `src/plane.rs` | Sensor-plane strip location and packed-bit unpack (`DEC-008`) — pixel data; design §2 "knows nothing about pixels" |
| `src/develop.rs` | Levels/geometry/orientation of *pixels* |
| `src/demosaic.rs`, `src/color.rs` | Colour science — design §14 anti-goal |
| `src/tone.rs`, `src/tone_table.rs` | sRGB OETF encode (791 KB generated table) |
| `src/warp.rs` | Lens-distortion resampling |
| `src/opcode.rs` | DNG opcode-list parser; EXIF has no opcodes |
| `src/bin/irr.rs` | The dev/oracle binary; diamond-exif's CLI is `exif-audit` (design §3) |
| `tests/support/perturb.rs` | Pixel-domain fault generators (§2.3) |
| `tests/support/ssimulacra2.rs`, `tests/perceptual_oracle.rs`, `tests/tone_oracle.rs` | Perceptual image-similarity oracle — a pixel claim |
| `tests/support/{corpus,pnm,md5,opcode,oracle}.rs`, `tests/corpus_manifest.rs`, `tests/corpus/manifest.toml` | Tier-B licensed-camera corpus (`DEC-003`, `IRRADIANCE_CORPUS_DIR`), PNM writers, DNG md5/opcode/levels oracles. diamond-exif commits no real-camera files (design §8) |
| `tests/{plane_oracle,develop,develop_oracle,demosaic_oracle,colour_metadata_oracle,warp,opcode,plane_unpack}.rs` | Each tests a stays-module. `tests/metadata_oracle.rs` is the **pattern** to copy (rewritten), not the file |
| `fuzz/fuzz_targets/{plane,develop,demosaic,opcode,warp_opcode}.rs` | Targets for stays-modules. `fuzz/fuzz_targets/ifd.rs` **does** port (§2.4) |
| `dnglab` wrappers (`tools.rs:542-800`) | diamond-exif's oracle is exiftool only |
| `docs/measured-q2m-dng.md`, `docs/oracle-contract.md`, `docs/conformance-matrix.md` | Leica/DNG measurements; the *method* (measured, dated, with the command) is what to copy |

---

## 7. Summary

**The three biggest risks**, in priority order:

1. **`ifd.rs` is read-only, strict, DNG-shaped and ASCII-blind.** `types.rs`, the multi-IFD pointer walk, the tolerant-walk policy and the entire
   writer-side encode/construct API are new code, not a port. Budget M1a as *"adapt the walk + write ~60% new"*, not *"port 3,209 lines"*.
2. **The tolerant-walk policy change (G2) invalidates a quarter of the tests it appears to port.** `DEC-012`'s "structure is fatal" is the repo's
   most heavily tested invariant; EXIF needs the opposite for non-IFD0 IFDs. Decide it in a written decision *before* porting tests.
3. **The red-proof and gate machinery is the repo's own, not the template's**, and the template copy is behind on bug fixes. Cloning from the template
   alone silently loses the six gates and the mute-death fixes.

**Design-doc points contradicted by the code:** the table in §0 (`Bytes`; `reader.rs` surface; `types.rs` completeness; "class 1"; the fifth lint
already present; `-j` vs `-T`; `perturb.rs`).

### Open gaps (not resolved here)

- **G1.** Where does `ByteOrder` live — `reader.rs` or `tiff/types.rs`? Design §4 is silent; `reader.rs` functions take it, so `reader.rs`.
- **G2.** Tolerant-walk result shape: `Result<Container, Error>` cannot carry "IFD0 OK, GPS IFD corrupt". Needs a `Warning`/`Diagnostics` channel in `error.rs` — a new decision.
- **G3.** Dev-dependency for exiftool `-j` JSON (§2.2).
- **G4.** Template drift vs the current release could not be checked (network) — §3.1.
- **G5.** The 30 tests in `2411-3209` were assigned by range, not run individually — §1.3.
- **G6.** Which `.github/workflows/ci.yml` jobs invoke which scripts — not traced.
