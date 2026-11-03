---
source: "crustyimg PROJ-001 shipped (v0.1.0) — 43 specs / 9 stages, claude-only variant"
captured_at: 2026-07-03
captured_by: claude
status: open                # open | addressed | deferred
---

# Spec-driven process — feedback after PROJ-001 shipped (crustyimg v0.1.0)

Written at the v0.1.0 release: PROJ-001 (the crustyimg MVP) shipped end-to-end —
~43 specs across 9 stages, 41 DECs, publicly installable via crates.io, a
Homebrew tap, and cross-platform GitHub Release binaries. Honest feedback on how
the spec-driven template (the "Claude plays every role" / claude-only variant)
served this project. Companion to the same-named doc in the bragfile project; the
conclusions here are crustyimg's own. About the process, not the product.

## Outcome assessment: the methodology delivered

Empty repo → hardened, publicly-installable image CLI without a death march or a
mid-project rewrite (notable given the prototype it replaced died of exactly
that). The single-image-library + pipeline architecture held across every stage;
the untrusted-input hardening (STAGE-006) was a coherent gate, not a scramble.
Produced code with a reviewable decision trail.

## What genuinely worked

- **The DEC log is the highest-value element.** 41 standalone, supersedable
  records meant almost nothing got re-litigated across 9 stages. DEC-004
  (pure-Rust codec policy) silently governed every format spec; the license
  constraints caught AGPL/GPL deps (gifski, imagequant, the heic crate) before
  they entered the tree. Supersession (not deletion) kept history legible.
- **Design-time probes became the signature move — and paid off every time.**
  The insistence on a real design cycle before build created space to probe
  load-bearing crates against the actual pinned tree before writing failing
  tests. Caught: clap_complete's exact version pin, that cargo-dist can't publish
  to crates.io (before committing a wrong config), and the true MSRV floor (1.85
  guess vs the real 1.89 from a transitive dep). Emerged from design/build
  separation; worth codifying by name.
- **The independent verify cycle caught real defects.** Verify by a different
  session (not the builder) is the best quality lever in the variant. Flagged the
  MSRV floor too low before merge and independently re-derived a security
  advisory's reachability argument rather than rubber-stamping. Self-review would
  have missed both.
- **Spec-as-source-of-truth made fresh sessions cheap.** Build ran as a brand-new
  Sonnet session driven only by the spec + prescriptive prompt. When a build
  subagent died mid-cycle, the orchestrator picked up from spec state.
- **Constraint gates + cost capture were real, not decorative.** cargo-deny /
  clippy-fmt / lean-build / MSRV gates caught actual regressions (incl. ambient
  advisory drift). Per-cycle cost capture (~$57.72 total metered build+verify)
  gave honest ROI visibility.

## Friction I actually hit

1. **The full cycle is disproportionate for trivial changes.** SPEC-043 was a
   3-line deny.toml edit and still ran design → build → verify → ship with a
   dispatched subagent. The judgment (accepting advisories with a reachability
   assessment) belonged in a DEC; the ceremony around three lines was pure tax.
   No lightweight lane.
2. **Ship bookkeeping is manual and error-prone.** advance-cycle / archive-spec
   don't update the parent stage's backlog list or **Count:**; cost totals, ship
   reflection, and the `git mv` to done/ are all by hand, and helpers occasionally
   mis-glob. Produced a standing "verify the git index before the ship commit"
   rule because editor/linter churn kept re-staging stale content. Same gap
   KNOWN_LIMITATIONS.md calls out; in a 43-spec project it added up.
3. **Subagent sessions are fragile and the template assumes they aren't.** Build
   subagents died on API overloads mid-cycle (twice); background-dispatched
   subagents can't get a Bash permission at all. Recovery (orchestrator verifies
   partial output, finishes in main loop) worked but is an unwritten resilience
   pattern.
4. **Ambient advisory drift broke a gate with zero code change.** cargo-deny
   advisories went red on main because the RustSec DB updated underneath us, and
   stayed red across several doc-only pushes before it was caught. The gate runs
   on push/PR, not on a schedule, so time-varying advisories are invisible until
   the next push trips them.
5. **Docs accreted without a user-vs-contributor split.** docs/USAGE.md is titled
   "How to use this template" but also holds CLI batch/dogfood examples — a user
   looking for "how do I process a folder of images" won't find it. README became
   tool-first only after a dedicated spec (SPEC-040). The template gives no
   guidance on separating "how to develop this repo" from "how to use the tool."

## Improvements I'd prioritize (in order)

1. **A lightweight lane for mechanical changes** — collapse design+build into one
   step for a doc/config one-liner with no design surface, KEEP the independent
   verify. SPEC-043 shouldn't cost four cycles.
2. **Automate ship bookkeeping** — archive-spec updates the stage backlog line +
   **Count:**, and computes cost.totals from cost.sessions. Single biggest source
   of manual error.
3. **A scheduled cargo-deny advisory job** — run the advisory check on a cron,
   decoupled from code pushes, so DB drift surfaces on its own.
4. **Name the subagent-recovery pattern** — "if a build/verify subagent dies
   mid-cycle, the orchestrator verifies its partial output and completes in the
   main loop" as an explicit, cost-annotated procedure.
5. **Codify the design-time probe** — for any load-bearing external crate/tool,
   probe it against the real pinned tree during design and record the verified
   calls in the dep's DEC.
6. **A user-vs-contributor docs split** — README + docs/ CLI-usage home for the
   tool; docs/development.md + AGENTS.md for the workflow.

## What I would NOT change

The DEC discipline · the independent verify cycle · design-before-build +
spec-as-truth · constraint gates + per-cycle cost capture.

## One meta-observation

The template's value tracked the **stakes** of the change, not its size. It
earned its overhead handsomely on the hardening specs and the release-engineering
work; it felt like pure tax on the mechanical changes. The two things that most
prevented actual errors were the **independent verify** and the **DEC log** — not
the ceremony of the four named cycles. If you kept only those two and made
everything else optional-by-stakes, you'd retain most of the quality for a
fraction of the overhead.
