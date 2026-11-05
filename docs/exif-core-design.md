# diamond-exif — Design Document (exif-core / exif-pro)

**Repo name:** `diamond-exif` — the library is `diamond-exif` (crates.io name
`diamondexif`; crates.io normalizes hyphens), the CLI is `exif-audit`, the paid
crate is `exif-pro`. "Diamond" carries the product's two claims: hardened
(panic-free, fuzz-cleaned) and certified (ExifTool-verified, report-backed).

**Date:** 2026-11-01
**Status:** design only — no implementation in this change
**Scope:** Milestone 1 ends at one-shot `sanitize()` + `audit_directory()` + static site. SaaS hosting, HEIC writing, and a frontend are explicitly out.
**Build location:** a new directory (not this repo). This doc travels there.

---

## 1. Summary

Two tracks:

- **exif-core** (open source, MIT OR Apache-2.0, published to crates.io): panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC read, write, and strip across JPEG, TIFF, PNG, WebP, and HEIC (read-only), verified differentially against ExifTool.
- **exif-pro** (private scaffold): batch audit API behind a `pro` feature flag with a build-time license gate. Not fully implemented — one working proof-of-concept feature.

Plus a static landing site (`site/`) with no JS, no tracking.

The differentiators are the non-functional ones and they are treated as gates, not aspirations:

| Constraint | Enforcement |
|---|---|
| Panic-free on untrusted input | Five-lint deny + fuzz targets per parser + red-proof tests |
| Pure Rust | `cargo tree -p exif-core` sys-crate count must equal 0 (CI gate) |
| Zero-copy parse | `bytes::Bytes` slicing; no buffer duplication on the parse path |
| Deterministic output | No unsorted map iteration on write paths; no injected timestamps; golden-file tests |
| ExifTool parity | Every read/write/strip op has a `metadata_oracle.rs` case; oracle must be shown red |

## 2. Relationship to `irradiance`

This repo is the foundation, not the host. What carries over:

**Code, ported and adapted (provenance: own code — **class 3** under the ledger
discipline: derived from our own implementation, which is itself derived from the
published specs; class 1 is published-spec-only):**
- `src/ifd.rs` (3,209 lines) — bounds-checked TIFF/IFD walking: `ByteOrder`, `Entry` (`tag`/`field_type`/`count`/`byte_len`), `Ifd` (`entries`, `entry(tag)`, `has`), offset/depth/parent tracing with cycle guards. EXIF **is** a TIFF IFD embedded in each container (JPEG APP1, PNG `eXIf`, WebP `EXIF` chunk, HEIC Exif item), so this engine is the core of every parser. Strip the DNG-specific semantics (CFA, `Sensor`, packed-bits plane math); keep the byte-level accessors. **Known limits from the porting brief:** it is a strict read-only walker — no ASCII type support, Exif/GPS/Interop/MakerNote pointers are never followed, and any bad IFD fails the whole parse (DEC-012 there). Those are net-new work; see §16.
- `tests/support/tools.rs` — the subprocess-oracle pattern: ExifTool run as a **tool, never linked** (GPL/Artistic — imposes nothing), exit-status checked explicitly because `exiftool` exits 0 on truncated input, and loud skips naming the missing tool.

**Discipline, inherited wholesale:**
- `#![forbid(unsafe_code)]` and the deny set — **extended to five lints**: `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, **plus `arithmetic_side_effects`**. Offset arithmetic on attacker-controlled 32-bit values is exactly where `a + b` overflows; this repo's experience justifies the fifth lint from day one. `#[allow]` beneath the crate root forbidden (`-F` re-check in CI, same shape as `lint-no-allow` here).
- **Oracle-must-be-shown-red**: every oracle path ships a deliberate-fault test that fails (e.g. corrupted IFD offset in a fixture must turn the parity test red).
- **Fuzz targets arrive with the first parser spec**, not retrofitted — one target per container parser plus one per rewriter.
- **A skipped test is loud**: no ExifTool on PATH → skip with a message naming the tool, never silent green.

**Not carried over:** pixel decoding, tone mapping, demosaic, opcodes, the `irr` binary. exif-core knows nothing about pixels beyond preserving them byte-for-byte.

## 3. Workspace layout

```
diamond-exif/                 # new directory / new repo
├── Cargo.toml                 # workspace
├── crates/
│   ├── exif-core/             # published crate
│   │   ├── src/
│   │   ├── tests/
│   │   │   ├── metadata_oracle.rs   # ported + extended to write/strip
│   │   │   └── fixtures/            # committed tier-A corpus (licence-clean)
│   │   ├── examples/axum_upload.rs
│   │   ├── examples/actix_upload.rs
│   │   └── fuzz/
│   └── exif-pro/              # private, feature-gated
├── site/                      # static landing site
├── pricing.md
└── .github/workflows/ci.yml
```

**Dependency policy for exif-core (M1):** `thiserror`. `bytes` **only at the
API edge** — the core borrows `&[u8]` (matching `Container<'a>` in irradiance)
so the parse path stays copy-free without a dependency. Nothing else.
`cargo-deny` enforces the allow-list; all pure Rust, no `*-sys`. Any addition is a written decision.

## 4. Module decomposition (exif-core)

```
src/
├── lib.rs            # public API surface, lint denies, crate docs
├── error.rs          # Error / Result (thiserror); every fallible path typed
├── reader.rs         # Bounds-checked cursor over Bytes: u8/u16/u32/u64 get,
│                     # signed offset math via checked_*, unknown-type refusal
├── tiff/
│   ├── mod.rs        # ported ifd.rs: walk, entry accessors, SubIFD/depth guards
│   ├── types.rs      # TIFF type system (BYTE..DOUBLE) with bounds-checked reads
│   └── tags.rs       # tag registry + category classification (see §6)
├── containers/
│   ├── jpeg.rs       # segment walk; APP1 Exif, APP1 XMP (+ extended XMP),
│   │                 # APP2 ICC_PROFILE, APP11 JUMBF (C2PA), APP13 IPTC
│   ├── png.rs        # chunk walk; eXIf, iTXt "XML:com.adobe.xmp", iCCP
│   ├── webp.rs       # RIFF/VP8X; EXIF, XMP, ICCP chunks; odd-length padding byte
│   └── heic.rs       # ISOBMFF box walk; iinf/iloc meta items — READ-ONLY in M1
├── xmp.rs            # packet locate/extract; targeted property splice on the
│                     # well-formed subset, result must re-parse before accept
├── iptc.rs           # IPTC IIM record reader (APP13 / tag 0x83BB)
├── icc.rs            # profile passthrough (extract/preserve; size sanity only)
├── model.rs          # Metadata { exif, xmp, iptc, icc, c2pa_present }
├── policy.rs         # StripPolicy presets + builder
├── edit/
│   ├── plan.rs       # EditPlan: container-specific op list, built from policy
│   ├── jpeg.rs       # segment splice (rebuild APPn chain, memcpy scan data)
│   ├── png.rs        # chunk splice (insert/replace before IDAT)
│   ├── webp.rs       # RIFF header + chunk rewrite (VP8/VP8L untouched)
│   └── tiff_apply.rs # IFD surgery: same-length in-place patch; rebuild on resize
└── api.rs            # Metadata::parse / strip / write_back / sanitize()
```

A container module does three things: locate the metadata regions, hand the TIFF
blob / XMP packet to the shared engines, and know how to splice a modified
region back. Format knowledge is contained; the IFD/XMP/ICC engines are shared.

## 5. Public API

```rust
let meta = Metadata::parse(&jpeg_bytes)?;          // zero-copy over Bytes
let stripped = meta.strip(StripPolicy::GpsAndDevice)?;
let output = stripped.write_back(&jpeg_bytes)?;    // pixels byte-identical

// one-shot
let sanitized = exif_core::sanitize(&jpeg_bytes, StripPolicy::AllPii)?;
```

```rust
pub enum StripPolicy {
    /// GPS block only (GPS IFD pointer + all 0x0000–0x001F tags + makernote GPS).
    Gps,
    /// GPS + device identification (serials, ImageUniqueID, internal IDs).
    GpsAndDevice,
    /// Everything a privacy auditor would flag: GPS, device IDs, Artist,
    /// Copyright, UserComment, DateTimeOriginal, camera Make/Model.
    AllPii,
    /// All EXIF/XMP/IPTC — matches `exiftool -all=` for those groups.
    /// ICC profiles and C2PA manifests are PRESERVED (per requirements);
    /// use `Everything` to remove them too.
    All,
    /// Also removes ICC + C2PA. Explicit opt-in only.
    Everything,
    Custom(StripSpec),   // builder: per-category include/exclude
}
```

Write operations (M1): `set(Orientation)`, `clear(Orientation)`, `strip(Gps)`,
`set(Artist/Copyright/DateTimeOriginal/UserComment)`, `set/replace/remove XMP
packet`. Values are typed — `Orientation` is an enum, `UserComment` handles the
8-byte charset prefix, `DateTimeOriginal` is a validated struct.

**API invariant:** `strip`/`write_back` never re-encode pixel data. This is
testable for free and tested everywhere (see §8, invariant I1).

## 6. Container facts the design depends on

These are format-spec facts; each gets verified against real fixtures during
the build stage that touches it (design-time probe discipline):

- **JPEG** — metadata lives in APPn segments before SOS; APP1 payload capped at
  65,533 bytes (`Exif\0\0` / `http://ns.adobe.com/xap/1.0/\0` signatures).
  Extended XMP (second APP1 with GUID + 32-byte digest) carries oversized XMP.
  Oversized **EXIF** has no standard chaining → `Error::ExifTooLarge`, documented.
  Scan data = everything from SOS to EOI, memcpy'd untouched.
- **TIFF** — native container; EXIF/XMP/GPS are IFDs or tags (`0x8769`, `0x8825`,
  `0x02BC` XMP, `0x83BB` IPTC-NAA, `0x8773` ICC).
- **PNG** — `eXIf` chunk (2017 spec addition) holds the TIFF blob; XMP in `iTXt`
  with keyword `XML:com.adobe.xmp`; ICC in `iCCP`. Writes insert/replace chunks
  before `IDAT`, preserving required chunk ordering.
- **WebP** — RIFF wrapper; `EXIF`/`XMP`/`ICCP` chunks (declared via `VP8X`
  flags); odd-length chunks carry a pad byte; all RIFF/ chunk sizes rewritten on
  change. `VP8`/`VP8L` bitstreams untouched.
- **HEIC** — ISOBMFF boxes; metadata are items located via `iinf`/`iloc` (Exif
  item, XMP item). **M1: parse only.** Strip = removing items and rewriting
  `iloc`/`iinf` — box-level surgery, deferred to M2 with the same fuzz/oracle
  treatment. Documented limitation: no HEIC write of any kind in M1.
- **MakerNotes** — read enough to classify serial-number tags for stripping;
  **never rewritten** in M1 (vendor offset-fixup quirks). Stripping device info
  that lives inside a makernote = removing the whole makernote, not editing it.
- **C2PA** — detected (JPEG APP11 / XMP `c2pa` namespace), preserved by default;
  not parsed in M1. Manifest extraction is a Pro-tier candidate.

## 7. Writer design

Two-phase: **EditPlan → apply**. Parsing produces a `Metadata`; policy + write
ops produce an `EditPlan` (a list of region operations with before/after bytes);
apply splices them into the original buffer.

- **Same-length replacement** (e.g. `Orientation` SHORT) → in-place byte patch.
- **Any size change** → rebuild only the affected segment/chunk and splice;
  pixel regions (`SOS..EOI`, `IDAT`, `VP8/VP8L`) are never reconstructed, only
  copied.
- **XMP editing**: M1 treats the packet as a structured-but-opaque string —
  locate/remove/insert known properties (`tiff:Orientation`, EXIF namespace
  GPS fields, `xmpRights`, `photoshop:Credit`) with a minimal scanner, then
  **re-parse the result before accepting it**. A full XML tree dependency
  (`roxmltree`, pure Rust) is the documented fallback if the scanner proves
  brittle — decision point at that build stage.
- **Determinism**: byte-identical output for identical input+op. `BTreeMap`
  ordering for tag emission, no injected timestamps, no padding choices that
  depend on environment. Golden-file tests lock this.

## 8. Testing: the oracle is the moat

**Matrix**: every container × {read, write-orientation, write-xmp, strip-GPS,
strip-device, strip-AllPii, strip-All} × {with ICC present, with C2PA present,
with thumbnail}. `tests/metadata_oracle.rs` enumerates it; a cell without a
test is a failing assertion, not a TODO comment.

Three comparison layers per cell:

- **I1 — pixel integrity (free, always runs)**: after any write/strip, the
  pixel regions' bytes hash identically to the original. Strongest invariant,
  zero tooling, layer-0 style.
- **I2 — read parity**: `diamond-exif parse` vs `exiftool -T -n -s3`
  (tab-separated, numeric-only — the wrapper irradiance already uses; avoids a
  JSON dev-dependency) on the same file;
  tag-level comparison with explicit value normalization (dates, rationals,
  signed/unsigned) recorded per tag — pinned ExifTool version in CI.
- **I3 — write parity**: apply the same op with exif-core and with ExifTool
  (e.g. `-Orientation=6`, `-gps:all=`), then run I2 over **both** outputs and
  assert agreement. Whole-file byte equality is deliberately NOT asserted —
  rebuild ordering legitimately differs; I1 + I2 is the honest claim.

**Red-proofs** (each ships with its oracle): corrupt an IFD offset in a fixture
→ I2 fails; truncate a JPEG between APP1 and SOS → parser returns a typed error,
not a panic; a hand-mutated write output with a pixel byte flipped → I1 fails.

**Fuzz targets** (one per input surface, from the first parser stage):
`tiff_ifd`, `jpeg_segments`, `png_chunks`, `webp_riff`, `heic_boxes`,
`rewrite_jpeg` (mutated input + random op), `rewrite_png`. Seeds from the
committed fixture corpus. CI runs a 60 s smoke per target; the **24-hour clean
run** is a documented local/CI-extended procedure and a milestone gate.

**Fixture corpus** (tier-A: committed, licence-clean): hand-built minimal
files + files produced by `exiftool -o` from our own images + deliberately
truncated/malformed variants. No real-camera files committed.

## 9. exif-pro scaffold

- Workspace member `crates/exif-pro`, depends on `exif-core`. **No feature flags
  inside exif-core** — the free/paid boundary is the crate boundary.
- `pro` feature gates `audit_directory(path) -> AuditReport`: walk directory →
  parse each file → record present categories, PII findings, strip simulation →
  JSON `AuditReport` (per-file + summary). Tested with fixtures.
- **Build-time license gate** (`build.rs`): if `pro` is enabled and
  `EXIF_PRO_LICENSE` is absent/malformed (format check only — e.g. non-empty,
  `EPRO-` prefix), the build fails with a message pointing to the placeholder
  pricing URL. Skips under `DOCS_RS=1` so docs.rs builds. **No runtime
  validation, no phone-home** — noted in the doc that real distribution later
  means a private registry or deploy-keyed git dependency, not a homegrown
  license server.
- `pricing.md`: Community ($0 — everything in exif-core) / Pro ($99–299/mo —
  audit API, compliance reports, custom tag mapping, priority SLA) / Enterprise
  (C2PA extraction, SIEM-format audit export, support). Pricing.md documents
  that Pro ships ≥3–4 features before it is sold, not just the audit PoC.

## 10. Site

`site/` — four static pages, Tailwind via CDN, no build step, no JS framework,
no cookies:

- `index.html` — pitch ("panic-free, ExifTool-verified EXIF processing for Rust
  server workloads"), format matrix, 10-line Axum sample, GitHub link.
- `pricing.html` — the three-tier table mirroring pricing.md.
- `docs.html` — links to crate docs and the Axum/Actix examples.
- `compliance.html` — what "ExifTool differential verification" concretely
  means (the I1/I2/I3 layers, red-proofs, fuzzing), and how the enterprise
  compliance report works. This is the compliance-buyer hook.

## 11. CI (`.github/workflows/ci.yml`)

Matrix: Linux / macOS / Windows, stable + MSRV (pin at first build; record it).

Jobs: `cargo fmt --check` · `cargo clippy --all-targets --all-features -- -D
warnings` · the `-F` five-lint re-check · `cargo test --all-features` (oracle
job installs ExifTool and pins its version; absent-tool runs skip loudly) ·
`cargo deny check licenses` · **`cargo tree -p exif-core | grep -- -sys` must
match zero** · fuzz smoke (60 s/target) · `examples/axum_upload.rs` smoke with
ExifTool verification of the output · site link check (optional).

## 12. Milestones

| Stage | Content | Gate |
|---|---|---|
| M1a | Workspace; port `reader.rs` + `tiff/` from `src/ifd.rs`; JPEG parser; oracle I1+I2 (read parity) | red-proof for I2; `tiff_ifd` + `jpeg_segments` fuzz targets exist and have run |
| M1b | PNG + WebP parsers (+ their fuzz targets, oracle rows) | matrix read-cells green |
| M1c | JPEG write/strip: edit plan, segment splice, orientation/GPS/XMP ops | I1+I3 green; `rewrite_jpeg` fuzz; golden determinism tests |
| M1d | PNG + WebP write/strip; StripPolicy presets; makernote read/strip rules | I3 green for all policies |
| M1e | HEIC read-only; IPTC + ICC passthrough; C2PA detection/preserve | read matrix complete; HEIC strip scoped for M2 |
| M1f | 24 h fuzz clean; oracle matrix complete; README (matrix + parity claim + Axum example); examples; CI complete; publish to crates.io | all success criteria |
| M2 | exif-pro scaffold: `audit_directory` + build gate + pricing.md + site | audit PoC tested; site deploys |

## 13. Risks and open questions

1. **ExifTool value normalization** — comparing parsed values across a Perl
   implementation needs an explicit, tested normalization table (dates, rationals,
   charset edges). Budget a full stage task for it; it is where false parity hides.
2. **XMP scanner brittleness** — if targeted splicing breaks on real-world
   packets, adopt `roxmltree` (pure Rust, permissive) as a written decision.
3. **Oversized EXIF in JPEG** — no standard chaining; `ExifTooLarge` error is
   the answer, but document it prominently.
4. **Consumer tolerance** — rewritten files should open in browsers/ImageMagick
   /`dnglab`-style tools; add a spot-check job if time allows.
5. **Pinned ExifTool drift** — CI pins the version; upgrades re-run the whole
   matrix and are expected to surface normalization diffs.

## 14. Explicitly deferred (anti-goals, restated as scope)

HEIC write · SaaS/hosted API · runtime license checks/telemetry in exif-core ·
feature-flag splitting of the core · makernote *editing* · C2PA parsing ·
demosaic/color science · any UI beyond the static site.

## 15. Reproduction commands (target state)

```bash
cargo test --all-features                          # full suite incl. oracle (skips loudly sans exiftool)
cargo test --all-features --test metadata_oracle   # the parity matrix
PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run jpeg_segments fuzz/corpus/jpeg fuzz/seeds/jpeg -- -max_total_time=60
cargo tree -p exif-core | grep -- -sys             # must output nothing
cargo deny check licenses
cargo run --release --example axum_upload -- fixture.jpg out.jpg
exiftool -gps:all out.jpg                          # prints nothing → GPS gone
exiftool -All out.jpg                              # tag-level check vs original
```

---

## 16. Revisions from the irradiance porting brief (2026-11-02)

Findings from irradiance's survey of its own code, folded into this design.
The full brief lives in `irradiance` → `docs/porting-brief-diamond-exif.md`.

1. **Tolerant-walk policy — the first DEC, written before porting any tests.**
   irradiance's `ifd.rs` fails the *whole* parse on any bad IFD (DEC-012 there).
   An upload sanitiser needs the opposite for non-IFD0 IFDs: "IFD0 fine, GPS IFD
   corrupt" must be survivable and reportable, which `Result<Container, Error>`
   cannot express. Required shape: parse collects per-IFD/per-region outcomes
   (e.g. `Metadata { issues: Vec<ParseIssue> }`). This decision flips ~25% of
   the portable tests — decide it first or the port churns twice.
2. **The writer is net-new.** `ifd.rs` is read-only with private `Entry` fields;
   the only byte encoders are in test code. §7 has nothing to port — M1c/M1d
   are the largest stages, not the cheapest. Budget accordingly.
3. **`types.rs` is mostly new.** ASCII types (Make/Model/Artist/DateTimeOriginal)
   are unsupported in the walker, and Exif/GPS/Interop/MakerNote pointer walking
   is net-new. M1a is bigger than "port ifd.rs".
4. **Zero-copy**: keep `&[u8]` borrows in the core; `bytes::Bytes` only at the
   API edge (§3 revised).
5. **Provenance class 3**, not 1 (§2 revised).
6. **Oracle read format**: `exiftool -T -n -s3` (§8 revised).
7. **Red-proof machinery**: `perturb.rs` is pixel-domain and does NOT port. The
   real machinery is `metadata_oracle.rs`'s replay-and-perturb test and the
   mutated-copy helpers in `plane_oracle.rs` — currently copy-pasted three
   times; the port should extract them into one shared helper.
8. **New helpers needed in M1a**: byte-mutation helpers (corrupt an offset,
   truncate a region) — none exist in irradiance.
9. **Gates: clone from irradiance's `scripts/`, not the template.** Six gate
   scripts (lint-red-proof, lint-fuzz-red-proof, gate-audit,
   gate-audit-red-proof, cost-audit-red-proof, validate-red-proof) and `_lib.sh`
   bug fixes exist in this repo but not the local template clone (template
   0.6.39 vs repo 0.6.38). Cloning from the template alone silently loses them.

### Unverified at brief time (carry into diamond-exif's `guidance/questions.yaml`)

- Template drift vs the *current* upstream release (GitHub API unreachable
  during the brief; run `gh release list -R jysf/spec-driven-template` first).
- Test classification: the 30 `ifd.rs` tests were assigned by name/range,
  not by running each one.
- CI job → script wiring untraced.
- **ExifTool write-mode exit codes unmeasured** — the read side exits 0 on
  truncated input; measure the write side before building the I3 write-parity
  check, or the oracle's exit-status gate is a guess.
