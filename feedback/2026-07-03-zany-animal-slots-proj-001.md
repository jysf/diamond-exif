---
source: "zany-animal-slots PROJ-001 shipped (2026-07-03), then paused — claude-only variant, animation-heavy non-CRUD frontend"
captured_at: 2026-07-03
captured_by: claude
status: open                # open | addressed | deferred
---

# zany-animal-slots PROJ-001 — template dogfood capture (index + synthesis)

Full source docs live in the instance repo:
- `github.com/jysf/zany-animal-slots/feedback/2026-06-18-template-dogfood-proj-001.md`
  (15 incremental findings, design→ship)
- `github.com/jysf/zany-animal-slots/feedback/2026-07-03-proj-001-retrospective.md`
  (consolidated, prioritized upstream fix list)

PROJ-001 (Animal Slots MVP) shipped to a live deploy, then the project was paused
after one project. The template carried a real-time, animation-heavy, non-CRUD
app to production with **only papercuts — no structural rework**; the engine
froze at SPEC-011 and never changed through 26 more specs.

## The 15 findings (index)

1. `new-stage` glob collides while example + real PROJ dirs share a number
   (`find -name … | head -n1`). Deterministic resolution / hard-error wanted.
2. `just test` collision (self-test vs app suite). **← already fixed upstream
   v0.5.16 (`template-selftest`).**
3. Constraint severity vocab mismatch: plans rate critical/high/medium,
   `constraints.yaml` enum is blocking/warning/advisory. No canonical mapping.
4. `decisions-audit` flags intentional parent/child scope nesting as conflicts
   (broad `src/engine/**` DEC-001 containing narrower DEC-002/003/006 → 7 standing
   "overlapping scope" warnings).
5. Repo-id placeholder lag: scaffolds stamp `repo.id` from `.repo-context.yaml`,
   which still holds `my-app` during Prompt 1c (repo-context update lives in 2a).
6. Wins aren't captured like costs are: cost is gated per-cycle, accomplishment
   capture (brag) was ad-hoc. (Template added ship *guidance* in v0.5.15, but not
   required/gated.) Open decision: convention vs `brag-audit` gate.
7. **`advance-cycle` matched the wrong file and silently no-op'd** — `find_spec`
   didn't exclude `specs/prompts/`, so it edited `SPEC-001-build.md` (no
   front-matter), reported success with a blank old-cycle, left the spec at
   `design`. **Verified still live upstream.**
8. **Cost schema drift** — spec template + inline prompt snippets (2b/3/ship)
   record `tokens_input`/`tokens_output`, but the `cost-audit` gate +
   `cycle_tokens_total` read a single `tokens_total`. Following the prompts
   verbatim guarantees a cost-audit failure. **Verified still live upstream.**
9. **`archive-spec` advertises "updates the stage backlog" but only prints
   hints** → stale backlog markers/counts. **Verified still live upstream.**
10. A non-interactive build sub-agent can't pause to author a DEC, so
    `no-new-top-level-deps-without-decision` forced a workaround (types stub) that
    review had to undo (SPEC-002, wasted round-trip).
11. Sub-agents return truncated/mid-task self-reports; stale timeline markers lie
    (SPEC-004). Durable rule: trust git/disk over any agent self-report.
12/13/15. Tool-prior mismatches: sub-agents reflexively add `exhaustive-deps`
    disables (no react-hooks plugin), reach for `@testing-library/user-event`
    (not installed), write `scripts/*.mjs` failing lint on Node globals. Always
    self-corrected; fix = fold repo toolchain into build-prompt boilerplate.
14. Shared working tree + auto-backgrounded sub-agents corrupted a branch once.
    Rule: launch exactly one build/verify sub-agent, no git/tree ops until it
    reports and merges.

## Two strategic takeaways (retrospective bottom line)

1. **The template needs a documented sub-agent / delegated-execution mode.** It
   was written assuming one interactive agent; delegating build/verify to fresh
   sub-agents surfaced a whole new failure class (Theme C: #10, #11, #14, #12/13/15).
2. **"Contract-tests-as-guards" should be promoted from an emergent trick to a
   recommended pattern.** Turning subjective "juice" (motion, contrast, perf,
   touch targets) into enforceable CI guards directly refuted the project's stated
   risk that juice work resists TDD.

## Prioritized upstream fix checklist (their assessment)

- **P1 — silent-failure bugs:** `find_spec` excludes `prompts/` + warn-on-no-
  front-matter (#7); `archive-spec` edits the backlog it advertises (#9); converge
  cost schema on `tokens_total` (#8); deterministic project resolution + repo-id at
  1b (#1, #5).
- **P2 — vocab / capture:** canonical severity mapping (#3); required
  accomplishment capture, decide convention-vs-gate (#6).
- **P3 — sub-agent execution model:** trivial dev-dep + its DEC in one build pass
  (#10); "trust git/disk over any self-report" in AGENTS.md (#11); shared-tree rule
  (#14); repo-toolchain notes in build-prompt boilerplate (#12/13/15).
- **P4 — audit noise:** intentional parent-scope declaration for decisions (#4).

## What worked — keep

`value.thesis`/`value_contribution` blocks · the 1b→1c→2a→…→1d→1e flow · cost
tracking (once schema fixed) · `decisions-audit` structural lint + `affected_scope`
· **contract-tests-as-guards** (biggest emergent win) · the architecture thesis
holding (engine froze at SPEC-011).
