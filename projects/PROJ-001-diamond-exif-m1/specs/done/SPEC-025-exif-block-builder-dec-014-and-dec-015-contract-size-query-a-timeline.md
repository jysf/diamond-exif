# SPEC-025 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-025-<cycle>.md`.

## Instructions

- [x] frame — outline in STAGE-007's frame (2026-11-20); design gate on irradiance/DEC-072 cleared and byte-order conversion split out to SPEC-041 at the re-frame (2026-11-27, claude-opus-5-5)
- [x] design — ExifTool 13.55 measurements (offset-carrying sweep over 1,178 sources, writer and reader passes; unknown types; count 0; Exif next-word; layout placed at bases 8 and 1000), DEC-014 amendment and DEC-015 clarification of 2026-11-27, question `strip-offset-tags-incomplete` raised, spec, HANDOFF-092 (2026-11-27, claude-opus-5-5)
- [x] build — HANDOFF-092 (claude-sonnet-5-5), two parts A and B, PR #71; orchestrator fix b0b5130 (the writer sweep without `-m`, DEC-023); DEC-014 accepted as amended by jysf (2026-11-27)
- [x] verify — HANDOFF-093 (claude-opus-5-5): round 1 punch list at 7cda5d3 (1 M, 4 L), fix round 1 (claude-sonnet-5-5, 7bdb98f..fbd360a), round 2 approved at 6055155 (2026-11-27)
- [x] ship — reflection, verify_rounds [punch-list, approved], complexity_actual M, DEC-014 type-0 wording, archived (2026-11-27, claude-opus-5-5)
