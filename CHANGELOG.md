# Changelog

All notable changes to this app. Newest at top.

Keep an `## [Unreleased]` section on top and add entries under it as you work —
the patch lane files fixes here (`[Unreleased] → Fixed`), and a release spec
(DEC-006) promotes the section to a version heading when it cuts the tag.

## [Unreleased]

### Added

### Changed

**Gate harness reconciled with irradiance's hardened mute-gate scripts** (instance-local; template VERSION unchanged at 0.6.39).

Brought over from irradiance (SPEC-011/016/023, PATCH-002/003) the audit and
red-proof scripts the upstream template has not absorbed. Template-newer content
(everything else, incl. `scripts/test.sh`'s `guidance/agents/` checks) is kept.

- `_lib.sh` — `front_matter_parse_report` (real YAML parse), `|| true` on the
  active-project grep, `next_id` skips `tests/`, quoted-`notes:` `#` no longer
  truncated, CRLF/quoted-tolerant `get_stage_status`, stage orchestration-cost
  helpers. **Not carried:** irradiance's project-specific grandfather lists
  (`SPEC-004 PATCH-002/3/4`, `STAGE-001`) — both defaults are empty here.
- `validate.sh` — front matter is PARSED (ruby `YAML.parse`) across `projects/` +
  decisions, with a completion sentinel; `cost-audit.sh` — shipped stages must
  record `orchestration_cost`; `decisions-audit.sh`, `handback-sync.sh` —
  unguarded-grep fixes, superseded handoffs are not "pending".
- New: `gate-audit.sh` (+ `-red-proof`), `validate-red-proof.sh`,
  `cost-audit-red-proof.sh` (fixture-synthesising: builds its own STAGE-900/901
  from the stage template instead of irradiance's real STAGE-001/002),
  `tests/oracle-fixtures/SPEC-900-unparseable-front-matter.md`.
- `scripts/test.sh` — only the 19 `|| true` guards `gate-audit` demanded.
- `app.just` — `validate-red-proof`, `cost-audit-red-proof`, `gate-audit`,
  `gate-audit-red-proof` recipes.
- **Deliberately not ported:** `lint-red-proof.sh` / `lint-fuzz-red-proof.sh`
  (SPEC-011). They mutate irradiance's Rust crate (`src/lib.rs` panic-free
  `#![deny]` policy, `fuzz/`) and have nothing to run against here.

### Fixed
