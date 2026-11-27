# SPEC-039 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-039-<cycle>.md`.

## Instructions

- [x] frame — outline in STAGE-010's batch frame; notes carried from the SPEC-037 and SPEC-038 verifies (2026-11-23, claude-opus-5-5)
- [x] design — ExifTool 13.55 measurements (write-mode exit codes, the ours-vs-ExifTool differential over 174 fixtures, PNG type and keyword sweeps, refusal vs preview), DEC-023, DEC-020 amendment, question `exiftool-write-exit-codes` answered, spec, HANDOFF-090 (2026-11-24, claude-opus-5-5)
- [x] build — HANDOFF-090 (claude-sonnet-5-5), three parts A, B, C (2026-11-26)
- [x] verify — HANDOFF-091 (claude-opus-5-5), five independent rounds, each a punch list (rounds 1–4 on the build branch, round 5 on merged main d61cbf2); fix rounds 1–5 (claude-sonnet-5-5); round 5's two L documentation items fixed at ship; leaves verify approved (2026-11-26 to 2026-11-27)
- [x] ship — PR #67 merged (d61cbf2) after a CI fix (5e8bc80, the rewrite_png fuzz model's AC1 keyword rows); round-5 fixes, DEC-023 amendment of 2026-11-27, costs, CHANGELOG, reflection, signals; archived (2026-11-27, claude-opus-5-5)
