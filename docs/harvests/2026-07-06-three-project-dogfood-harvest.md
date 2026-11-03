# Dogfood harvest — three-project retro (2026-07-06)

Harvest of template-improvement signals from the three actively-worked instances,
plus an aggregator retro. Read-only; sources are the instances' own feedback docs
and spec/brief front-matter. This is the durable record — the ranked backlog at
the bottom is what we work from.

## Sources

| Instance | Variant | Scale | Feedback location |
|---|---|---|---|
| `~/PSeven/experiments/bragfile000` | claude-only | 4 projects, 42 specs, 3 releases (v0.1.0→v0.3.1), 27 DECs | `docs/framework-feedback/` (3 docs) |
| `~/PSeven/experiments/crustimg_redo_plus/crustyimg` | claude-only | ~43 specs / 9 stages / 41 DECs, public v0.1.0, mid-PROJ-002 | `docs/framework-feedback/{process-feedback,signals-harvest}.md` |
| `~/PSeven/experiments/zany-animal-slots` | claude-only | PROJ-001 (37 specs) shipped, PROJ-002 in flight | `feedback/2026-07-0{3,4}-*.md` (YAML signals inside dated files) |
| `~/PSeven/experiments/bragfile-report` | claude-only | aggregator | `docs/framework-feedback/2026-07-04-three-project-retro-feedback.md` |

Note: none of these instances use `guidance/signals.yaml` — they predate the
signals-registry convention (v0.5.18) and keep signals in `docs/framework-feedback/`
or dated `feedback/*.md`. Signals-ledger location being ad hoc is itself a minor
signal (instances that predate a convention don't adopt it retroactively).

## Headline: the recent cycle is field-validated

Several features shipped this cycle were **independently reinvented by the projects
before the template shipped them** — the strongest possible dogfood signal:

- **Patch lane (DEC-003)** — crustyimg is its first real user (PATCH-001–003);
  reflection: "the collapsed lane fit well," fixed the #1 friction (a 3-line edit
  that ran a full 4-cycle).
- **Release-spec + runtime pre-flight (DEC-006)** — *both* bragfile and crustyimg
  built their own local release-cut template + runtime checklist first; the retro's
  top ask ("upstream candidate B") was "ship exactly this."
- **Build provenance (DEC-008)** — bragfile already injects ldflags
  (`-X main.version/.commit`), surfaces `brag --version`, and has a `devguard.go`
  reading the build version to block dev-binary→prod-DB migrations. Confirms the
  need; note bragfile uses hand-picked SemVer (a calver divergence, not a failure).
- **Sub-agent rules (DEC-004), portability (DEC-005), continuous numbering
  (v0.5.20), signals-registry disposition ritual (v0.5.18), `just test`→selftest
  (v0.5.16), severity mapping (v0.5.23), archive-spec backlog edit (v0.5.19/23)** —
  all confirmed; most older-instance complaints about these are simply *stale*
  (those repos predate the fix).

**Protect (validated — do NOT "improve"):** the WATCH→codify pipeline at the
N=3-same / N=2-paired-opposing bar (across 121 specs: near-zero drift, one
self-triggered supersession, zero rule reversals); literal-artifact-as-spec;
confidence discipline (mean ~0.82, no 1.0s); independent-verify + DEC-log as the
two actual error-preventers; one-PR-per-spec; PEEL-IF-L sizing. The unanimous
verdict: **"don't push it to codify sooner."**

## Ranked backlog — new / still-open (deduped across projects)

| # | Signal | Type · N | Status | Notes |
|---|---|---|---|---|
| 1 | **`get_active_project` is status-blind** | bug · N=2, high-impact | **fixing v0.6.5** | Picks lowest-numbered dir, ignores `status` → in a multi-wave repo `status`/`cost-audit`/`backlog` silently target the *shipped* project; cost-audit runs green on the wrong wave. Bites the exact multi-wave use the template promotes. |
| 2 | **Codify the design-time probe / measure-before-tune** | lesson · **N=17 (crust) + zany cluster** | open | Convergent across 2 projects: run the *real* measurement/probe against the pinned tree during **design** → build becomes bit-for-bit transcription (zero iteration). Not a named step. Adjacent to toolchain-brief but distinct (a design-cycle behavior). |
| 3 | **`shipped_at` never populated on specs** | process-debt · N=3 (all) | **fixing v0.6.5** | Ship date lives in git tags/timeline/cost, not a field → per-spec cycle-time and time-to-value aren't computable. Fix: stamp `shipped_at` at `archive-spec`. Unblocks DEC-009. |
| 4 | **Contract-tests-as-guards kit** | product · emergent, high-value | open — co-design w/ next project | zany *invented* CI guards for subjective quality (motion/contrast/perf/touch) and thereby refuted its own "juice resists TDD" risk. Ship a guard starter-kit + optional named constraints (contrast-aa, state-not-color-only, compositor-only-keyframes). |
| 5 | **Sub-agent metering can silently lie** | risk · N=1 | open | A session-limited subagent returned 662 tokens for a full verify; `cost-audit` only checks non-null, so the undercount passes. Undercuts cost provenance — needs a floor-heuristic/implausibility flag. |
| 6 | **Release pre-flight: evidence-now vs deferred-to-cut** | lesson · N=1 fresh | open — small | Release sessions hard-stop *before* the irreversible tag, so some pre-flight items can't be verified in-session. The release-spec (DEC-006) should give each item two states (evidence-now vs verified-by-orchestrator-at-cut). Also: encode the mechanical-prep / irreversible-cut two-phase split as first-class. |
| 7 | **Reserve→adoption check** | lesson · N=2 (bragfile+retro) | open | When a DEC reserves a capability "for later," prompt for a paired "how will we know it's actually used?" line — else it's invisible debt (DEC-024 provenance namespace shipped with zero readers). |
| 8 | **roadmap/backlog miss planned-but-unframed stages** | product · zany | open | `dash future`/`roadmap` only render stages with a *file*; parse the brief's `## Stage Plan` checkboxes so the arc is visible before framing. `backlog` also truncates titles + lacks [S/M/L] sizing. |
| 9 | **Per-language "known gotchas" appendix** | process-debt · N=7+ (crust) | open | A recurring "the spec should have warned about X" class is language facts (Rust: `non_exhaustive` wildcard, `#[from] io::Error` collision, MSRV floor, `--allow-dirty`; macOS tempdir canonicalize). A per-language appendix the build prompt links, kept out of core. Complements toolchain-brief. |
| 10 | **Scheduled advisory CI convention** | process-debt · N=2 (crust) | open | Advisory gates (cargo-deny, npm audit) only run on push/PR, so vuln-DB drift goes red with zero code change and stays invisible. A cron/scheduled-gate convention. crustyimg shipped its own (PATCH-003). |
| 11 | **User-vs-contributor docs split / client-facing docs** | product · crust | open — co-design w/ next project | No convention separating "how to develop this repo" (AGENTS.md) from "how to use the built tool" (README + usage docs). Ties to the **client-handover** gap (candidate DEC-010). |
| 12 | **`frame` cycle is essentially dead** | lesson · 0/122 specs | open — cheap | Zero specs sat in `frame` across all projects. Consider making Frame optional / absorbing it into design. |
| 13 | **`agents.architect/implementer` misread as contamination under claude-only** | doc nit · bragfile | open — cheap | A verify session misread the tier_map fields as contamination evidence. Clarify the fields' meaning in single-agent variant docs (DEC-005 P2 partially addressed via architect≠implementer). |
| 14 | **Scale-tier roadmap (informational)** | lesson · bragfile scale-recs | watch | AGENTS.md at 41.6 KB (cold-read cost), auto `decisions/INDEX.md` past ~25 DECs, constraint-linting vs honor-system, cross-project `depends_on:` in front-matter. A "what breaks past this scale" list, not current defects. |

Smaller / already-mostly-addressed (verify coverage, don't re-solve): cost-schema
field drift (converged on `tokens_total`; verify upstream prompts), lean
`--no-default-features` build in both build+verify, "confirm prescribed failing
tests exist" end-of-build check, MSRV-from-metadata (language-specific),
`decisions-audit` affected_scope bare-name normalization (v0.5.23 warns; deeper
path-rooting open), `just archive-spec` false "all shipped" message (largely
addressed v0.5.19/23; confirm), brief `status:` comment-sensitivity (template's
own resolver uses field-2 = comment-robust; confirm), brag-capture-gated-like-cost
(open Q: convention vs gate).

## Business value + time-to-value (the DEC-009 foundation)

**Value capture is already strong and structured** — every project carries a
`thesis` / `beneficiaries` / `success_signals` / `risks_to_thesis` block, credited
with catching scope a feature list would hide. **Protect this.**

**The gap is quantification, not capture.** Success signals are narrative/binary
("all five states reachable," "engine has zero DOM imports"). But *both* projects'
**second waves independently started adding real numbers**:
- zany PROJ-002: target machine RTP ~94% / hit-freq ~40% + a metrics simulator;
  brief's own open question is a "fun proxy metric" (recommend median
  spins-per-session).
- crustyimg PROJ-002: byte-reduction at SSIMULACRA2 ≥ target, ~95% correct
  format routing (benchmarked vs Cloudinary `f_auto`).
- bragfile: seeded `cost`/`session`/`tokens` capture (SPEC-046/047) precisely so an
  economics/adoption metric can be computed — but nothing reads it yet (signal #7).

They are **organically reaching for exactly the measurable target** we want →
DEC-009 should *formalize what they're already doing*, not invent a metrics system.

**Time-to-value (real dates found):**
- crustyimg PROJ-001: first ship **same day** (SPEC-001 created+shipped 2026-06-13);
  public v0.1.0 tagged 2026-07-03 (~3 weeks, release-eng lag). PROJ-002: ~1-day
  first increment.
- zany PROJ-001: first ship **same day** (2026-06-18); full value 2026-07-03 (~15
  days, 37 specs / 6 stages). PROJ-002: first ship 1 day, first stage 2 days.
- bragfile PROJ-001: first ship ~21 days (created 2026-04-19 → v0.1.0 tag
  2026-05-10); full MVP ~28 days **vs a stated "~2 weeks" target — a 2× overrun,
  honestly recorded.**

**Pattern:** time-to-first-ship is consistently same-day/next-day (the scaffold
spec front-loads a shippable increment); time-to-full-value scales with scope
(15–28 days). bragfile's honestly-recorded 2× miss vs a stated target is the
**predicted-vs-realized loop happening ad hoc** — worth formalizing.

**Critical implementation constraint:** `shipped_at` is not reliably populated
(signal #3, confirmed by all three) — so time-to-value must *derive* ship dates,
OR the template must stamp `shipped_at` at archive (chosen — v0.6.5). And
sub-agent metering can silently undercount (signal #5), so any cost-derived value
metric needs the implausibility flag.

## Meta-conclusions worth carrying

1. **"Value tracks stakes, not size."** The template earned its keep on *high-stakes*
   specs (hardening, release-eng, license landmines — AGPL deps caught before the
   tree); it was pure tax on *low-stakes mechanical* ones. The two elements that
   actually prevented errors were **independent verify** and the **DEC log**.
   Argues for stakes-tiering (the patch lane is the first cut).
2. **The multi-wave state is what unmasks the worst latent bugs** (signal #1). The
   template's own "continuous multi-wave repo" design goal is what triggers its
   worst failure — worth a standing "test the second-wave case" habit.
3. **The reflection loop IS the harvesting engine.** bragfile round-2: "of ~90
   findings, the large majority were already harvested into AGENTS.md" via the
   Ship-Q2 prompt. The bottleneck moved from "do we codify?" to "can we *see*
   what's queued to codify?" — a maturity signal, not a defect.
4. **Instances run ahead of upstream** and package fixes as ready-to-apply PRs
   (zany's continuous-numbering PR). The disposition gap risks in-instance fixes
   never landing upstream without a forcing mechanism (the signals registry is that
   mechanism; instances predating it didn't have it).

## Worth timing to a NEW project (not "wait to build", but the real test bed)

- **DEC-009 metric-derivation step** — its whole point is to fire at a project's
  *frame* moment (derive the target metric live). Existing projects set their value
  blocks retroactively. Build the plumbing now; validate the derivation aid on the
  next project's frame.
- **Contract-tests-as-guards kit (#4)** — best validated on a project that needs
  guards from day 1.
- **Client-handover / docs-split (#11)** — shaped by an actual external delivery.
