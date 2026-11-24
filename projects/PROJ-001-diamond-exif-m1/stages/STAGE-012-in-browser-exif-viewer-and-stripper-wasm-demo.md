---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-012                     # stable, zero-padded, continuous across the repo
  status: proposed                  # proposed | active | shipped | cancelled | on_hold
  priority: low                     # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-24
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "Shows the panic-free, ExifTool-verified core working on a visitor's own file, with nothing uploaded, and gives non-Rust users a no-install way to audit and strip."
  delivers:
    - "A static web page: drop a JPEG, TIFF, PNG or WebP, see its metadata inventory grouped by audit category, preview a strip policy, download the stripped file"
    - "A wasm build of exif-core behind a thin bindings crate, with a Node round-trip test in CI"
    - "A GitHub Pages deploy gated on a headless-browser smoke test whose output is checked by ExifTool"
  explicitly_does_not:
    - "Add any dependency to exif-core or change its API for the browser"
    - "Upload, store or log anything; no backend, no analytics, no cookies"
    - "Offer a strip the audit marks not strippable (PNG and WebP wait for SPEC-039)"
    - "Ship an npm package (a later decision, if ever)"

# Orchestration cost — the spend that has no spec to attach to (roadmap:
# orchestration + framing cost attribution). Framing a stage, deciding the spec
# breakdown, and cross-spec steering all happen BEFORE/BETWEEN specs, so today
# they are invisible and recorded cost is systematically under-counted.
#
# THE ORCHESTRATOR FILLS THIS — not the human. At stage close, read your own
# session total (`/cost` in Claude Code; the `usage` object via API) and append
# one entry. Stage grain ONLY: do not try to split orchestration across specs —
# that is a division you cannot observe, so any per-spec number is invented.
# Warn-only, never a gate. A null here is honest; a guess is not. (DEC-013 §5)
orchestration_cost:
  sessions: []                      # - tokens_total: N
                                    #   estimated_usd: N
                                    #   recorded_at: YYYY-MM-DD
                                    #   notes: "framing + spec breakdown"
---

# STAGE-012: In-browser EXIF viewer and stripper (wasm demo)

## What This Stage Is

A static web page that runs exif-core, compiled to WebAssembly, in the visitor's browser. Drop a photo and the page shows what metadata it carries (the same inventory and categories as `exif-audit`), previews what a strip policy would remove, and hands back a stripped copy. The file never leaves the machine and there is no backend. It is both a demo of the core's claims and a usable no-install tool, and every result points at the matching `exif-audit` command for auditing a whole folder. `exif-audit` itself writes nothing, so this page is the first place a user can get a stripped file without writing Rust; that is part of the DEC's question.

## Why Now

It is not urgent: framed now so it is on the list, at `priority: low`, behind STAGE-007, SPEC-039 and the M1e/M1f work. Nothing blocks it technically. `exif-core` has no dependencies and no I/O, and `cargo build --lib --target wasm32-unknown-unknown` already succeeds unchanged (checked 2026-11-24 with rustup's toolchain). It should start after SPEC-039 ships, so the page can strip all four formats on day one without offering anything the audit still marks not strippable.

**Scope tension to settle first.** The PROJ-001 brief puts "workspace promotion, `exif-pro` and the site" out of scope as M2, and design §10 says the site has no JS. This stage needs JS and a second crate. The first spec is the DEC that resolves this: either amend the brief to admit this one page into M1, or move the stage to an M2 project. Until that DEC exists, this stage does not start.

## Success Criteria

- A visitor can drop a JPEG, TIFF/DNG, PNG or WebP file and see its audit inventory and findings, with the same categories and severities `exif-audit --json` reports for that file
- The strip download for each format is byte-identical to native `exif_core::write::strip` for the same file and policy, and ExifTool reading it shows none of what the policy removed
- The browser smoke test, run in CI on every PR that touches the app or `src/`, loads the served page in headless Chrome, strips a fixture and checks the downloaded bytes with ExifTool; a red smoke test blocks the Pages deploy
- `cargo tree -p exif-core` is still empty and the core's `include` allow-list is unchanged
- A hostile file produces a typed error on the page, never a dead wasm instance

## Scope

### In scope
- A DEC: where the bindings crate lives (separate package depending on `exif-core` by path, or an early workspace), the brief/design scope amendment, Pages hosting, the bindings dependency (`wasm-bindgen` pinned exactly, wasm-only), and whether a page that writes stripped files is acceptable while the CLI only reports
- A thin bindings crate: `audit(bytes) -> report JSON`, `strip(bytes, policy) -> bytes + preview`, `version()`; glue only, no logic
- A wasm round-trip test in Node through plain `cargo test --target wasm32-unknown-unknown` with a `.cargo/config.toml` runner, plus a wasm32 build check in CI (closes the open item in `guidance/questions.yaml`)
- The page: `index.html`, one ES module, one stylesheet, a `file://` guard, a tiny `node:http` serve script that sends `.wasm` as `application/wasm`
- A Pages workflow gated on a headless-browser smoke test

### Explicitly out of scope
- Any change to `exif-core`'s API, dependencies or `include` list for the browser's sake
- Upload, storage, analytics, cookies, telemetry
- PNG/WebP strip before SPEC-039 ships; HEIC; `set` operations
- An npm package; a Web Worker (add one only if a measurement shows the main thread stalls)
- The rest of the design §10 site (pricing, compliance pages)

## Spec Backlog

Ordered list of specs composing this stage. Not framed into specs yet on purpose: the stage is low priority, and `just frame-stage STAGE-012` can run when it is picked up.

Format: `- [status] SPEC-ID (cycle) — one-line summary`

- [ ] (not yet written) — [S] DEC: the bindings crate's home, the scope amendment (M1 or M2), Pages hosting, the pinned `wasm-bindgen` dependency, a write surface ahead of the CLI
- [ ] (not yet written) — [M] Bindings crate: `audit`, `strip`, `version` over exif-core; size profile (fat LTO, one codegen unit, strip; `wasm-opt` measured, not assumed); Node round-trip test; wasm32 check in CI
- [ ] (not yet written) — [M] The page: drop, inventory by category, policy preview, download, the `exif-audit` command shown per result, the `file://` guard, the serve script; honest about any format the page cannot strip
- [ ] (not yet written) — [M] Pages deploy gated on a headless-Chrome smoke test whose stripped output is checked by ExifTool

**Count:** 0 shipped / 0 active / 4 pending

## Design Notes

Lessons carried from crustyimg's wasm demo (its DEC-064, DEC-066, `src/wasm.rs`, `demo/`, `.github/workflows/pages.yml`):

- `wasm-pack build --target web`, no bundler and no `npm install`; a script copies `pkg/` into the page's gitignored `vendor/`.
- Build through rustup's stable toolchain explicitly (`rustup which --toolchain stable rustc`, set `PATH` and `RUSTC`): Homebrew's `cargo` is first on `PATH` here and has no wasm32 `std`.
- Run the round-trip with plain cargo and a `[target.wasm32-unknown-unknown] runner = "wasm-bindgen-test-runner"`, not `wasm-pack test`, which builds every integration test for wasm (the oracle tests spawn ExifTool and cannot).
- Bindings return `Result<_, JsError>` and use `#[wasm_bindgen]` structs with getters or a JSON string; a panic in wasm kills the page's instance, so the no-panic contract matters more here.
- The page must be served: ES modules and `instantiateStreaming` both fail on `file://`, so the page detects that and says so instead of hanging.
- The smoke test checks output with a decoder other than the engine (here, ExifTool), so a page that renders but cannot instantiate its wasm, or strips wrongly, goes red.
- crustyimg needed a Worker and a size fight because of its codecs; this build has neither problem, so measure before borrowing either fix.

## Dependencies

### Depends on
- SPEC-039 (STAGE-010): PNG and WebP strip verified against ExifTool and switched on in the audit, so the page strips all four formats
- The DEC above (first spec of this stage)

### Enables
- The design §10 site, if and when it is built: this page is its live demo

## Stage-Level Reflection

*Filled in when status moves to shipped. Run Prompt 1c (Stage Ship) in
FIRST_SESSION_PROMPTS.md to draft this.*
