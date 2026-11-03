# Blog post outlines — lessons from three shipped projects (2026-07-06)

Preserved from the 2026-07-06 three-project dogfood harvest
([docs/harvests/](../harvests/2026-07-06-three-project-dogfood-harvest.md)) so the
strongest, evidence-backed lessons don't evaporate. Each is a *draft outline*, not
a finished post — write when ready. Ranked by strength/novelty.

Evidence base: three real projects built on the template (bragfile, crustyimg,
zany-animal-slots), 120+ specs, 9 stages, multiple public releases; near-zero
design→ship drift, a single self-triggered decision supersession.

---

## 1. (Flagship) "Where software defects actually escape — and why `validate` lies"

**Claim:** Across three projects, **100% of *logic* defects were caught** in
design/build/verify; **100% of the defects that escaped to production were
operational/runtime.** The gap isn't logic — it's the seam between "the shape is
valid" and "the thing actually runs on a real host."

**Evidence (concrete, memorable):**
- A manifest passed `validate --strict` yet registered **zero** MCP servers.
- A dual tag landed on the same release commit; goreleaser then did the wrong thing.
- macOS Gatekeeper quarantined a freshly-downloaded, unsigned artifact.
- A package-manager tap/registry trust gate blocked a *new* user's first install.
- **A dev binary migrated the *production* database.**

**The lesson:** shape-check and behavior-check are *different checks* — neither
substitutes for the other. Exercise the surface that *runs* the behavior, not the
one that validates its shape (this became the template's "behavioral pre-flight"
convention + a generic runtime release checklist).

**Why it generalizes:** every team ships artifacts through a runner and a package
manager; every team will hit this class. **Audience:** broad engineering.

---

## 2. "Trust git and disk — not what your AI sub-agent tells you"

**Claim:** When you delegate build/verify to a fresh AI sub-agent, its
self-report is the least trustworthy artifact in the loop. Reconcile against
**actual git + disk state** before you advance anything.

**Evidence:**
- Truncated self-reports claimed "done" with the commit / tests / gate missing.
- Sub-agents auto-background and share the checkout → a design commit landed on
  the wrong branch (recovered via cherry-pick + reset).
- A build sub-agent silently defaulted to Opus — ~6× the intended cost.
- Session-limited metering returned **662 tokens for a full verify**, passing a
  non-null cost gate while silently deflating the totals.

**The lesson:** a named "reconcile over self-report" rule + a die-mid-cycle
recovery procedure + explicit per-cycle model config + an implausible-cost flag.
The spec-as-source-of-truth is what lets a fresh agent pick up at all; the missing
piece was the *orchestration discipline* around delegation.

**Why it generalizes:** this is the AI-agent moment, and almost nobody is writing
about orchestrating delegated agents *honestly*. **Audience:** AI-assisted dev,
agent builders. **Timely.**

---

## 3. "Process ceremony pays off by stakes, not by size"

**Claim:** The same spec-driven ceremony that *prevented real errors* on
high-stakes work was *pure tax* on low-stakes mechanical edits. Value tracks
**stakes, not size.**

**Evidence:**
- The template earned its keep on hardening, release-engineering, and license
  landmines (an AGPL dependency caught *before* it entered the tree).
- It was tax on one-line config edits that still ran a full design→build→verify→
  ship (a 3-line `deny.toml` change ran four cycles).
- The two elements that actually prevented errors, every time: **the independent
  verify cycle** and **the decision (DEC) log.** Everything else is stakes-tiered
  optional.

**The lesson:** tier the process by stakes (the "patch lane" was the first cut —
a collapsed cycle for bounded fixes that keeps independent verify + DEC and drops
the rest). Don't make small = cheap; make *low-stakes* = cheap.

**Why it generalizes:** every process-heavy team over-applies ceremony uniformly.
**Audience:** eng leaders, process/tooling designers.

---

## 4. "Measure the real thing at *design* time, and the build writes itself"

**Claim:** The highest-frequency single lesson across the projects (**N=17 of 43
specs** in one): when you probe the *real* API / measure the *real* baseline
against the pinned tree **during design**, the build stops being a discovery loop
and becomes a near bit-for-bit transcription.

**Evidence:**
- Probing load-bearing crates against the real pinned tree caught a "can't publish
  to crates.io" gap, a true MSRV floor (1.85 guessed vs 1.89 real), and wrong
  assumed API signatures — all at design, not mid-build.
- Reproducing a slot-machine's target RTP with a real measurement *before* tuning
  made the build a zero-iteration transcription (0 defects across the stage).
- Adversarial mutation ("revert the change, confirm the guard fails") both proved
  test teeth *and* surfaced a dead config field the engine never read.

**Why it generalizes:** a concrete, repeatable AI-dev workflow move. **Audience:**
AI-assisted developers.

---

## 5. (Meta) "Don't codify a lesson until it recurs — the N=3 rule"

**Claim:** The template's best feature is a *discipline of restraint*: a lesson
becomes a rule only after it recurs (N=3 same-outcome, or N=2 paired-opposing).
Codifying sooner produces brittle rules that fire wrongly.

**Evidence:** Across 120+ specs, near-zero design→ship drift and exactly **one**
decision supersession (self-triggered) — under a WATCH→codify pipeline that
deliberately *lags*. The retro's own verdict: *"don't push it to codify sooner."*
Live example: a wrapper shipped one day was removed the next once it was clear the
agent could call the tool directly.

**Why it generalizes:** every framework/tooling author feels the pull to
generalize the first time they see a pattern. **Audience:** framework/DX authors,
meta-process nerds. **Pairs well with #3.**

---

## Runners-up / smaller angles

- **"The literal artifact *is* the spec"** — a spec that's the real file (config,
  schema, shell, markdown) compounds rather than degrades; format-agnostic across
  Go/YAML/JSON-schema/shell.
- **"The multi-wave repo unmasks your worst latent bug"** — the resolver that
  silently targeted the *shipped* project once a second wave opened; the failure
  only exists in exactly the configuration the tool promotes.
- **"Cost tracking that can silently lie"** — metering truncation and the
  implausibility floor that flags it.
