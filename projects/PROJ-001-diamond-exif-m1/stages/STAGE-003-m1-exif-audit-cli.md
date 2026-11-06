---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-003                     # stable, zero-padded, continuous across the repo
  status: active                    # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-05
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The first thing a user can run: exif-audit turns the verified TIFF/JPEG reader into a read-only privacy scan (what metadata is in these files, which of it is personal, what a strip policy would remove), the open-core funnel of the product shape."
  delivers:
    - "A pure-library audit model: container sniff, an inventory of every tag and located region, and PII findings by category (GPS, device identity, authorship, timestamps, thumbnail, makernote presence), with the category table recorded in a DEC"
    - "StripPolicy presets as data and a policy preview (what each preset would remove) with no writer involved"
    - "Text, JSON and CSV reports that stay well-formed on hostile input, with their own fuzz target"
    - "The exif-audit binary: scans files and directories, isolates per-file errors, deterministic output, CI exit codes"
  explicitly_does_not:
    - "Write, strip or rewrite any file (writer is M1c); the preview only reports"
    - "Read PNG, WebP or HEIC (a later stage); other containers are reported as unsupported, not as clean"
    - "Decode XMP, ICC, IPTC or C2PA contents, or parse MakerNotes (their presence is reported, serials inside them are not)"
    - "Ship exif-pro, a license gate, telemetry, or any network access"

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

# STAGE-003: M1 exif-audit CLI

## What This Stage Is

Make the verified reader useful from a shell. `exif-audit PATH...` walks files and directories, sniffs each file's container (TIFF or JPEG; anything else is reported as unsupported, never as clean), builds an inventory of every tag and located metadata region, classifies what is personal (GPS, device identity and serial numbers, authorship, timestamps, an embedded thumbnail that may carry its own metadata, the mere presence of a MakerNote), previews what each strip policy would remove, and prints the result as text, JSON or CSV with exit codes a CI job can act on. The classification and the preview live in the pure library (no I/O); the binary adds only argument parsing, the directory walk and the exit-code contract. Nothing is written back.

## Why Now

STAGE-001 and STAGE-002 delivered a reader whose parity with ExifTool is shown, tolerant on hostile input, for the two containers uploads arrive in most. The audit CLI is the first deliverable named in the product shape and exercises that reader end to end on whole directories; it also forces the decisions the writer needs anyway (what counts as PII, what a strip policy means) before any byte is rewritten, so the writer stages inherit a settled policy instead of inventing one. PNG/WebP breadth does not unblock any of that.

## Success Criteria

- `exif-audit` on a file or directory emits inventory, PII findings and a policy preview as text, JSON and CSV, deterministically (sorted paths, stable field order), with documented exit codes (clean, findings above the threshold, usage or I/O error).
- No input makes the library or the binary panic; a bad file never aborts a directory run (per-file typed outcome); an unsupported container is reported as unsupported, not as clean.
- JSON and CSV output are well-formed for any input bytes, proven by a fuzz target over the renderers (constraint `fuzz-target-per-surface`), including tag values with control characters, quotes, invalid UTF-8 and very long strings.
- The inventory (group and tag id) equals ExifTool's on the hand-built fixtures, with the comparator red-proofed as before; any difference is a recorded divergence.
- The PII category table and the strip-policy presets are each recorded in a DEC before they are coded, and every preset is checked against fixtures (what it would remove is exactly what the table says).
- The library still has no I/O and no dependencies; the binary adds none either unless a DEC says otherwise; all CI jobs green on every PR, including a job that builds the binary.

## Scope

### In scope
- Container sniff for TIFF and JPEG; unsupported-container outcome
- Inventory and PII findings from `Container` and `JpegMetadata`; the category table (DEC)
- `StripPolicy` presets as data; policy preview from a `FileAudit`
- Text, JSON and CSV renderers (pure, in the library or a module shared with the fuzz target)
- The `exif-audit` binary: argument parsing, directory walk (symlink and size-cap policy), per-file isolation, exit codes, `--json` / `--csv`, README usage, `just dev`
- Inventory parity against ExifTool on fixtures (group + tag id), with red-proofs

### Explicitly out of scope
- Any write, strip, rewrite or in-place edit
- PNG, WebP, HEIC readers; XMP, ICC, IPTC, C2PA decoding; MakerNote parsing
- `exif-pro`, license gates, telemetry, networking, a TUI
- Recursing into an embedded thumbnail JPEG (its presence is a finding; scanning inside it is an open question, `audit-thumbnail-recursion`)

## Spec Backlog

Ordered list of specs composing this stage. Add specs as you identify
them. Update status as specs progress.

Format: `- [status] SPEC-ID (cycle) — one-line summary`

- [x] SPEC-012 (shipped on 2026-11-05) — [M] Audit model: container sniff, inventory, PII categories and findings (pure library); DEC for the category table
- [ ] SPEC-013 (frame) — [M] Inventory parity against ExifTool: group + tag-id sets on fixtures, comparator red-proofed
- [ ] SPEC-014 (frame) — [M] StripPolicy presets as data and the policy preview; DEC for preset semantics
- [ ] SPEC-015 (frame) — [M] Text, JSON and CSV renderers and the `audit_render` fuzz target (well-formed on hostile input)
- [ ] SPEC-017 (frame) — [M] Bound Container memory: a total entry budget for the tolerant walk (found by the SPEC-012 build: a 789 KB TIFF parses cleanly to 33M entries, ~950 MB)
- [ ] SPEC-016 (frame) — [L] The `exif-audit` binary: arguments, directory walk, per-file isolation, exit codes, CLI integration tests, CI build job

**Count:** 1 shipped / 5 active / 0 pending

## Design Notes

- **Where things live.** Classification, inventory and preview are pure functions over `&[u8]` results in `exif_core` (the library has no I/O, constraint `no-io-in-library`). The binary owns `std::fs`, argument parsing and exit codes. Renderers build `String`s and take no writer, so the fuzz target can call them without the binary.
- **No dependencies.** Argument parsing is hand-written (a small, fixed flag set) and JSON/CSV are written by hand with an escaping function that is itself fuzzed; a clap/serde dependency would need a DEC under `no-new-top-level-deps-without-decision`, and none is expected.
- **Honest limits are findings.** A MakerNote is present but unparsed, so serials inside it cannot be listed: the report says "MakerNote present (not parsed)" rather than implying a clean file. Unsupported containers and unreadable files are outcomes, not silent skips.
- **Policy before code.** The PII category table and the strip-policy preset contents are DECs written at the design of their specs, before any code, as DEC-001 was for the tolerant walk. The preview must be derivable from the table so the writer stages cannot drift from what the audit promised.
- **Multi-Exif.** Open question `several-exif-segments` (merged Exif view) is decided in the first spec: the audit must report findings from every Exif block, not only the primary.
- **Oracle.** ExifTool is run for inventory parity (`-G1 -a -H`-style listing of group and tag id); it is never read or linked. Any new hostile shape is probed alone under the time limit first.

## Dependencies

### Depends on
- STAGE-002 (shipped) — the JPEG reader, located regions, and the oracle with JPEG records
- STAGE-001 (shipped) — the TIFF engine, ExifTool oracle and red-proof apparatus
- External: ExifTool 13.55 as the oracle (run, never read or linked)

### Enables
- The writer and strip stages (M1c/M1d) — a settled category table and preset semantics, and a preview to verify writes against
- PNG/WebP breadth — every new container plugs into the same audit model
- `exif-pro` (M2) — `audit_directory` is this stage's library function plus a license gate

## Stage-Level Reflection

*Filled in when status moves to shipped. Run Prompt 1c (Stage Ship) in
FIRST_SESSION_PROMPTS.md to draft this.*

- **Did we deliver the outcome in "What This Stage Is"?** <yes/no + notes>
- **How many specs did it actually take?** <number vs. plan>
- **How many outlines survived unchanged?** <n of m — only if this stage was
  framed with `just frame-stage`. Counts outlines whose scope held from frame to
  ship. A low number means framing ran ahead of what was knowable; a high one
  means it's paying for itself.>
- **What changed between starting and shipping?** <one sentence>
- **Lessons that should update AGENTS.md, templates, or constraints?**
  - <one-line updates>
- **Signals dispositioned at this close?** (Prompt 1d step 7) Every
  `type: lesson` signal in `/guidance/signals.yaml` owned by this stage close
  was walked — codified (at its bar), left `watch`, or dropped. No silent carry.
  - <note what codified / what's still watch + its N>
- **Should any spec-level reflections be promoted to stage-level lessons?**
  - <one-line items — record below-bar ones as `watch` signals; don't codify yet>
