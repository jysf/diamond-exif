#!/usr/bin/env bash
# scripts/test.sh — end-to-end happy-path test for the template.
#
# Copies the repo into a temp dir, runs `just init` + the full cycle
# (new-stage → new-spec → advance-cycle × 4 → archive-spec), and
# asserts the invariants that previous bugs tripped over:
#
#   - init is one-shot and refuses to re-run
#   - advance-cycle preserves the cycle legend comment
#   - archive-spec refuses to archive an already-archived spec
#   - weekly-review emits only repo-relative paths
#
# No external test framework needed. Prints PASS / FAIL per check.
# Exits 0 if everything passes, 1 on the first failure.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# --- Colors (off if not a TTY) ---
if [ -t 1 ] && command -v tput >/dev/null 2>&1; then
    GREEN=$(tput setaf 2 2>/dev/null || printf '')
    RED=$(tput setaf 1 2>/dev/null || printf '')
    DIM=$(tput dim 2>/dev/null || printf '')
    RESET=$(tput sgr0 2>/dev/null || printf '')
else
    GREEN=''; RED=''; DIM=''; RESET=''
fi

pass_count=0
fail_count=0

pass() {
    pass_count=$((pass_count + 1))
    echo "${GREEN}✓${RESET} $*"
}

fail() {
    fail_count=$((fail_count + 1))
    echo "${RED}✗${RESET} $*" >&2
    # Bail on first failure — later checks usually depend on earlier state.
    echo "" >&2
    echo "${RED}FAILED${RESET}  (${pass_count} passed before this one)" >&2
    echo "Scratch dir left at: ${SCRATCH}" >&2
    exit 1
}

assert_eq() {
    local actual="$1" expected="$2" msg="$3"
    if [ "$actual" = "$expected" ]; then
        pass "$msg"
    else
        fail "$msg (expected: '$expected', got: '$actual')"
    fi
}

assert_file() {
    if [ -f "$1" ]; then pass "file exists: $1"; else fail "missing file: $1"; fi
}

assert_no_file() {
    if [ ! -e "$1" ]; then pass "absent: $1"; else fail "unexpected path: $1"; fi
}

assert_contains() {
    local file="$1" pattern="$2" msg="$3"
    if grep -qE "$pattern" "$file"; then
        pass "$msg"
    else
        fail "$msg (pattern '$pattern' not found in $file)"
    fi
}

assert_cmd_fails() {
    local msg="$1"; shift
    if "$@" >/dev/null 2>&1; then
        fail "$msg (expected non-zero exit, got 0)"
    else
        pass "$msg"
    fi
}

# --- Set up scratch dir ---
SCRATCH=$(mktemp -d 2>/dev/null || mktemp -d -t 'template-hardening-test')
# Copy template into scratch/repo, then delete .git so the scratch acts
# like a fresh `Use this template` clone.
cp -R "$TEMPLATE_ROOT" "$SCRATCH/repo"
rm -rf "$SCRATCH/repo/.git"

cd "$SCRATCH/repo"
echo "${DIM}scratch: $SCRATCH${RESET}"
echo ""

# ============================================================
# 1) init: happy path
# ============================================================
printf "1\n" | just init >/dev/null 2>&1 \
    || fail "just init (claude-only) exited non-zero"
assert_file "AGENTS.md"
assert_file ".variant"
assert_eq "$(cat .variant)" "claude-only" "variant marker is claude-only"
assert_no_file "variants"
pass "init: scaffolded claude-only successfully"

# ============================================================
# 1b) app.just: project-owned recipes, imported by the template justfile
# ============================================================
# app.just is a root file (not in variants/), so it ships and survives init.
# --- init leaves the app's root free of the TEMPLATE's own docs -------------
# Every instance used to inherit the template's 100KB+ CHANGELOG, its
# "built with this template" showcase, and its CONTRIBUTING guide — none of
# which describe the app. Worse, the patch lane writes to `[Unreleased]` in a
# CHANGELOG that was the template's history and had no such section.
for tf in PROJECTS.md CONTRIBUTING.md KNOWN_LIMITATIONS.md MIGRATION_TO_REPORTS_AND_COSTS.md; do
    if [ -f "$tf" ]; then
        fail "init left the template's own ${tf} in the scaffolded repo"
    fi
done
pass "init removes the template's own root docs"
assert_file "VERSION"
pass "init keeps VERSION (template provenance survives, per CONTRIBUTING)"
assert_file "LICENSE"
pass "init keeps LICENSE (never auto-deleted — could leave a repo unlicensed)"
# The app gets a FRESH changelog carrying the section the patch lane writes to.
assert_contains "CHANGELOG.md" "## \\[Unreleased\\]" "init seeds an app CHANGELOG with an [Unreleased] section"
if grep -q "spec-driven-template" CHANGELOG.md; then
    fail "the scaffolded CHANGELOG still contains the template's own history"
else
    pass "the scaffolded CHANGELOG is the app's, not the template's"
fi
cl_bytes=$(wc -c < CHANGELOG.md | tr -d ' ')
if [ "$cl_bytes" -lt 2000 ]; then
    pass "scaffolded CHANGELOG is small (${cl_bytes} bytes, was 100KB+)"
else
    fail "scaffolded CHANGELOG is ${cl_bytes} bytes — template history leaked in?"
fi

assert_file "app.just"
# The template justfile opts into it via an OPTIONAL import (escaped: ? and .
# are ERE metachars). `import?` keeps a fresh clone working before the file exists.
assert_contains "justfile" "import\? 'app\.just'" \
    "root justfile imports app.just (optional import)"
# The import resolves to the app recipes: `just build` must reach the app.just
# stub (its TODO reminder), not error with 'no recipe named build'.
app_build_out=$(just build 2>&1 || true)
case "$app_build_out" in
    *"TODO: define 'build'"*) pass "app.just: 'build' recipe resolves through the import" ;;
    *) fail "app.just: 'build' recipe unreachable — import broken? (got: ${app_build_out})" ;;
esac
# Non-clobbering is proven by the rest of this suite: every template recipe below
# runs with `import? 'app.just'` active in the scaffolded justfile.

# ============================================================
# 1c) fresh-history: reset git history + stamp provenance (v0.6.21)
# ============================================================
# Run the script against an ISOLATED throwaway repo (never the scaffold's own
# git) so it can't disturb later git-dependent checks.
FH_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t 'fresh-history-test')
(
    cd "$FH_DIR" || exit 1
    git init -q
    printf 'a\n' > f && git add -A \
        && git -c user.email=t@t -c user.name=t commit -q -m "template commit 1"
    printf 'b\n' >> f \
        && git -c user.email=t@t -c user.name=t commit -qam "template commit 2"
    printf '9.9.9\n' > VERSION
    git remote add origin https://example.com/some-template.git
    # -y skips the confirm; identity resolves from the repo/global config.
    bash "$TEMPLATE_ROOT/scripts/fresh-history.sh" -y >/dev/null 2>&1
) || true
# History collapsed to exactly one commit.
fh_count=$(git -C "$FH_DIR" rev-list --count HEAD 2>/dev/null || echo "?")
assert_eq "$fh_count" "1" "fresh-history: git history reset to a single commit"
# Provenance (template version + source remote) is stamped in that commit.
fh_msg=$(git -C "$FH_DIR" log -1 --format='%B' 2>/dev/null || echo "")
case "$fh_msg" in
    *"spec-driven-template 9.9.9"*) pass "fresh-history: initial commit stamps template version" ;;
    *) fail "fresh-history: initial commit missing template-version provenance" ;;
esac
case "$fh_msg" in
    *"some-template.git"*) pass "fresh-history: initial commit records the origin template remote" ;;
    *) fail "fresh-history: initial commit missing origin-template provenance" ;;
esac
# Lands on 'main'.
fh_branch=$(git -C "$FH_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "?")
assert_eq "$fh_branch" "main" "fresh-history: fresh repo is on 'main'"
rm -rf "$FH_DIR"

# ============================================================
# 2) init: re-run guard
# ============================================================
assert_cmd_fails "re-running init (AGENTS.md present) fails" just init
rm AGENTS.md
assert_cmd_fails "init with variants/ gone also fails" bash -c 'printf "1\n" | just init'
# Restore AGENTS.md by rerunning init cleanly from a fresh scratch for the next checks.
# Simpler: copy AGENTS.md back from the TEMPLATE_ROOT's variant.
cp "$TEMPLATE_ROOT/variants/claude-only/AGENTS.md" ./AGENTS.md
pass "init: re-run guards work in both states"

# ============================================================
# 3) new-stage + new-spec scaffold correctly
# ============================================================
# Simulate the user replacing the REPLACE'd repo id in .repo-context.yaml
# so we can verify the scaffold picks it up.
sed_inplace_portable() {
    if [ "$(uname)" = "Darwin" ]; then sed -i '' "$@"; else sed -i "$@"; fi
}
sed_inplace_portable 's|id: my-app|id: bragfile-test|' .repo-context.yaml

just new-stage "Test Stage" >/dev/null
STAGE_FILE="projects/PROJ-001-example-mvp/stages/STAGE-002-test-stage.md"
assert_file "$STAGE_FILE"
# created_at should be today (not the __TODAY__ placeholder).
today=$(date +%Y-%m-%d)
assert_contains "$STAGE_FILE" "^created_at: ${today}\$" "stage.md created_at filled with today"
# target_complete comment should still say YYYY-MM-DD (not substituted).
assert_contains "$STAGE_FILE" "# optional: YYYY-MM-DD" "stage.md comment placeholder untouched"
# repo.id should come from .repo-context.yaml, not the hardcoded default.
assert_contains "$STAGE_FILE" "^  id: bragfile-test\$" "stage.md repo.id picks up from .repo-context.yaml"

just new-spec "Test Spec" STAGE-002 >/dev/null
SPEC_FILE="projects/PROJ-001-example-mvp/specs/SPEC-002-test-spec.md"
assert_file "$SPEC_FILE"
assert_contains "$SPEC_FILE" "id: SPEC-002" "spec ID set"
assert_contains "$SPEC_FILE" "stage: STAGE-002" "spec parent stage set"
assert_contains "$SPEC_FILE" "^  created_at: ${today}\$" "spec created_at filled"
assert_contains "$SPEC_FILE" "^  id: bragfile-test\$" "spec.md repo.id picks up from .repo-context.yaml"

# ============================================================
# 4) advance-cycle preserves the cycle legend comment
# ============================================================
just advance-cycle SPEC-002 build >/dev/null
assert_contains "$SPEC_FILE" "^  cycle: build.*# frame \| design \| build \| verify \| ship" \
    "advance-cycle build: cycle updated AND comment preserved"

just advance-cycle SPEC-002 verify >/dev/null
assert_contains "$SPEC_FILE" "^  cycle: verify.*# frame \| design" \
    "advance-cycle verify: cycle updated AND comment still present"

just advance-cycle SPEC-002 ship >/dev/null
assert_contains "$SPEC_FILE" "^  cycle: ship.*# frame \| design" \
    "advance-cycle ship: cycle updated AND comment still present"

# ============================================================
# 5) archive-spec: happy path + double-archive refusal
# ============================================================
archive_out=$(just archive-spec SPEC-002 2>&1)
ARCHIVED="projects/PROJ-001-example-mvp/specs/done/SPEC-002-test-spec.md"
assert_file "$ARCHIVED"
assert_no_file "$SPEC_FILE"
# archive-spec stamps a top-level shipped_at (harvest #3) so time-to-value /
# cycle-time are computable from the spec itself.
assert_contains "$ARCHIVED" "^shipped_at: [0-9]{4}-[0-9]{2}-[0-9]{2}$" \
    "archive-spec stamps shipped_at into the front-matter"
# The stage-shipped message must be an observation, not a completion
# claim — the stage's backlog may still list unwritten specs.
if printf '%s\n' "$archive_out" | grep -qE "All specs for .* are shipped\."; then
    fail "archive-spec prints false-positive 'All specs … are shipped' claim"
else
    pass "archive-spec does not claim stage completion"
fi
if printf '%s\n' "$archive_out" | grep -qE "No active specs remain for STAGE-002"; then
    pass "archive-spec reports observation (no active specs remain)"
else
    fail "archive-spec missing expected 'No active specs remain' message"
fi

# Second archive must fail and must NOT create done/done/...
assert_cmd_fails "double-archive of SPEC-002 fails" just archive-spec SPEC-002
assert_no_file "projects/PROJ-001-example-mvp/specs/done/done"

# advance-cycle on an archived spec must also fail.
assert_cmd_fails "advance-cycle on archived spec fails" just advance-cycle SPEC-002 build

# ============================================================
# 6) review emits only repo-relative paths (DEC-001 Phase 3: `weekly-review`
#    is now `review`)
# ============================================================
review_out=$(just review 2>&1)
# The script's output should contain the scratch dir nowhere in path lines.
# It's OK for the scratch name to appear in shell echoes (it doesn't), but
# any `- /foo/...` bullet is a path bullet that must be relative.
if printf '%s\n' "$review_out" | grep -E "^- ${SCRATCH}" >/dev/null; then
    fail "review still prints absolute paths"
else
    pass "review: all bullet paths are repo-relative"
fi
# Sanity-check that it found the archived spec (relative).
if printf '%s\n' "$review_out" | grep -qE "^- projects/PROJ-001-example-mvp/specs/done/SPEC-002-test-spec\.md"; then
    pass "review: includes archived spec as relative path"
else
    fail "review: archived spec missing from output"
fi
# The old `weekly-review` name is GONE (breaking, DEC-001 Phase 3).
assert_cmd_fails "just weekly-review is removed (consolidated to review)" just weekly-review

# ============================================================
# 6b) lifetime-report: dated whole-repo history prompt (the horizon
#     between `status` = now and `review` = recent slice)
# ============================================================
lifetime_out=$(just lifetime-report 2>&1)
# It prints the synthesis prompt.
if printf '%s\n' "$lifetime_out" | grep -q "Lifetime Report"; then
    pass "lifetime-report: prints the synthesis prompt"
else
    fail "lifetime-report: prompt header missing"
fi
# The reason for the port: a dated `Generated: <today>` line, distinct from the
# git-span / project-history dates. Must equal today's date.
today_date=$(date +%Y-%m-%d)
if printf '%s\n' "$lifetime_out" | grep -qE "^Generated: ${today_date}$"; then
    pass "lifetime-report: emits 'Generated: <today>' line"
else
    fail "lifetime-report: missing dated 'Generated: ${today_date}' line"
fi
# Same pipe posture as review: no absolute path bullets.
if printf '%s\n' "$lifetime_out" | grep -E "^- ${SCRATCH}" >/dev/null; then
    fail "lifetime-report still prints absolute paths"
else
    pass "lifetime-report: all bullet paths are repo-relative"
fi

# --- lifetime-data: the same history as a self-contained, LLM-free report ---
lifetime_data=$(just lifetime-data 2>&1)
if printf '%s\n' "$lifetime_data" | grep -q "Lifetime Data Report"; then
    pass "lifetime-data: prints the self-contained data report header"
else
    fail "lifetime-data: data report header missing"
fi
# The at-a-glance count block is data-mode only.
if printf '%s\n' "$lifetime_data" | grep -q "Lifetime at a glance"; then
    pass "lifetime-data: includes the at-a-glance block"
else
    fail "lifetime-data: missing the at-a-glance block"
fi
# Data mode must NOT carry the LLM synthesis wrapper (that's prompt mode).
if printf '%s\n' "$lifetime_data" | grep -q "copy everything below this line into Claude"; then
    fail "lifetime-data: wrongly includes the LLM synthesis prompt"
else
    pass "lifetime-data: is LLM-free (no synthesis prompt)"
fi

# --- lifetime-save: writes a timestamped data report, never overwriting ---
just lifetime-save >/dev/null 2>&1 || fail "lifetime-save exited non-zero"
saved=$(find reports/lifetime -name '*.md' 2>/dev/null | head -1)
if [ -n "$saved" ] && grep -q "Lifetime Data Report" "$saved"; then
    pass "lifetime-save: wrote a timestamped data report under reports/lifetime/"
else
    fail "lifetime-save: no data report written under reports/lifetime/"
fi
# Timestamp is to the second (YYYY-MM-DD-HHMMSS), so repeated runs never collide.
case "$(basename "${saved:-}")" in
    [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9].md)
        pass "lifetime-save: filename is timestamped to the second" ;;
    *) fail "lifetime-save: filename not timestamped to the second" ;;
esac
rm -rf reports/lifetime

# ============================================================
# 7) status runs cleanly post-archive
# ============================================================
just status >/dev/null 2>&1 || fail "just status exited non-zero after archive"
pass "status: clean exit after archive"

# ============================================================
# 8) v5.2 value/cost blocks exist in scaffolded specs
# ============================================================
# Scaffold a fresh spec (SPEC-002 is archived; create a fresh one in
# the stage we still have) to verify the v5.2 shape lands correctly.
just new-spec "Second Test Spec" STAGE-002 >/dev/null
SPEC_V52="projects/PROJ-001-example-mvp/specs/SPEC-003-second-test-spec.md"
assert_file "$SPEC_V52"
assert_contains "$SPEC_V52" "^value_link: null\$" "spec scaffold has value_link: null"
assert_contains "$SPEC_V52" "^cost:\$" "spec scaffold has cost: block header"
assert_contains "$SPEC_V52" "^  sessions: \[\]" "cost.sessions is empty list by default"
assert_contains "$SPEC_V52" "^    tokens_total: 0" "cost.totals.tokens_total is 0"
assert_contains "$SPEC_V52" "^    session_count: 0" "cost.totals.session_count is 0"

# ============================================================
# 9) v5.2 value blocks in brief and stage templates are exposed in
#    the scaffolded copies (example brief still ships pre-v5.2)
# ============================================================
# The template's project-brief and stage markdowns should carry the
# v5.2 blocks — confirm by scaffolding a new stage and inspecting.
just new-stage "V52 Test Stage" >/dev/null
STAGE_V52_PATH=$(ls projects/PROJ-001-example-mvp/stages/STAGE-00*-v52-test-stage.md 2>/dev/null | head -n1 || true)
if [ -n "$STAGE_V52_PATH" ]; then
    assert_file "$STAGE_V52_PATH"
    assert_contains "$STAGE_V52_PATH" "^value_contribution:\$" "new stage has value_contribution: block"
    assert_contains "$STAGE_V52_PATH" "^  advances: null" "value_contribution.advances starts null"
    assert_contains "$STAGE_V52_PATH" "^  delivers: \[\]" "value_contribution.delivers starts []"
else
    fail "new-stage did not produce the expected scaffold file"
fi

# ============================================================
# 10) AGENTS.md (post-init, claude-only) has Business Value and
#     Cost Tracking sections
# ============================================================
assert_contains "AGENTS.md" "^## 3\\. Business Value" "AGENTS.md has Business Value section"
assert_contains "AGENTS.md" "^## 4\\. Cost Tracking Discipline" "AGENTS.md has Cost Tracking section"

# ============================================================
# 11) just report-daily writes a file and prints output
# ============================================================
daily_out=$(just report-daily 2>&1)
daily_file="reports/daily/$(date +%Y-%m-%d).md"
assert_file "$daily_file"
# Output should start with the header
if printf '%s\n' "$daily_out" | grep -q "^# Daily report — "; then
    pass "report-daily prints header to stdout"
else
    fail "report-daily did not print expected header"
fi
# Sections present in the written file
assert_contains "$daily_file" "^## Snapshot\$" "daily report has Snapshot section"
assert_contains "$daily_file" "^## Value\$" "daily report has Value section"
assert_contains "$daily_file" "^## Cost activity\$" "daily report has Cost activity section"
assert_contains "$daily_file" "^## Flags\$" "daily report has Flags section"
# Graceful fallback on pre-v5.2 example brief (no value.thesis)
assert_contains "$daily_file" "Project thesis:.*not set" \
    "daily report handles missing project thesis gracefully"

# Re-run overwrites (not append)
lines_before=$(wc -l < "$daily_file" | tr -d ' ')
just report-daily >/dev/null 2>&1
lines_after=$(wc -l < "$daily_file" | tr -d ' ')
assert_eq "$lines_after" "$lines_before" "report-daily re-run overwrites, doesn't grow the file"

# ============================================================
# 12) just report-weekly writes a file and degrades gracefully
# ============================================================
just report-weekly >/dev/null 2>&1
# Determine the expected ISO week filename in the same way the
# script does, so macOS/Linux branches agree with the test.
if [ "$(uname)" = "Darwin" ]; then
    iso_week=$(date -j -f "%Y-%m-%d" "$(date +%Y-%m-%d)" +"%G-W%V")
else
    iso_week=$(date -d "$(date +%Y-%m-%d)" +"%G-W%V")
fi
weekly_file="reports/weekly/${iso_week}.md"
assert_file "$weekly_file"
assert_contains "$weekly_file" "^## Summary\$" "weekly report has Summary section"
assert_contains "$weekly_file" "^## Value advancement\$" "weekly report has Value advancement section"
assert_contains "$weekly_file" "^## Shipped this week\$" "weekly report has Shipped this week section"
assert_contains "$weekly_file" "^## Cost breakdown\$" "weekly report has Cost breakdown section"
# No specs shipped this week in the scratch flow except SPEC-002
# (just-archived today). So the table shouldn't be empty.
if grep -q "SPEC-002" "$weekly_file"; then
    pass "weekly report includes freshly-archived SPEC-002"
else
    fail "weekly report missing freshly-archived SPEC-002"
fi

# ============================================================
# 13) Reports tolerate a spec without cost/value_link blocks
# ============================================================
# The example spec SPEC-001-example-project-logger ships in the
# template pre-v5.2, so it has no cost or value_link. Both reports
# must still run without error — they did above. Add an explicit
# assertion that the daily report surfaces "no cost data yet" when
# the active spec has no cost.sessions entries at all.
if grep -q "no cost data yet" "$daily_file" || grep -q "Specs with no cost data" "$daily_file"; then
    pass "daily report flags pre-v5.2 specs as missing cost data"
else
    fail "daily report did not flag specs without cost data"
fi

# ============================================================
# 14) v5.3 instruction-timeline convention
# ============================================================
# SPEC-003 was scaffolded in §8; new-spec should have created a
# timeline file alongside it, substituting the SPEC ID into the
# template's SPEC-XXX placeholder.
TIMELINE_V53="projects/PROJ-001-example-mvp/specs/SPEC-003-second-test-spec-timeline.md"
assert_file "$TIMELINE_V53"
assert_contains "$TIMELINE_V53" "^# SPEC-003 timeline\$" "timeline header names the spec"
assert_contains "$TIMELINE_V53" '\[ \].*not started' "timeline legend documents [ ] not started"
assert_contains "$TIMELINE_V53" '\[~\].*in progress' "timeline legend documents [~] in progress"
assert_contains "$TIMELINE_V53" '\[x\].*complete' "timeline legend documents [x] complete"
assert_contains "$TIMELINE_V53" '\[\?\].*blocked' "timeline legend documents [?] blocked"

# prompts/ directory should exist as a peer to the spec files so
# the architect's first cycle-prompt write lands in a ready place.
if [ -d "projects/PROJ-001-example-mvp/specs/prompts" ]; then
    pass "prompts/ directory created alongside specs"
else
    fail "prompts/ directory missing after new-spec"
fi

# AGENTS.md (post-init, claude-only) has the Instruction Timeline
# section and documents all four markers. If the legend drifts the
# convention erodes, so each marker gets its own assertion.
assert_contains "AGENTS.md" "^## 9\\. Instruction Timeline\$" \
    "AGENTS.md has Instruction Timeline section"
assert_contains "AGENTS.md" '`\[ \]` not started' \
    "AGENTS.md documents [ ] not-started marker"
assert_contains "AGENTS.md" '`\[~\]` in progress' \
    "AGENTS.md documents [~] in-progress marker"
assert_contains "AGENTS.md" '`\[x\]` complete' \
    "AGENTS.md documents [x] complete marker"
assert_contains "AGENTS.md" '`\[\?\]` blocked' \
    "AGENTS.md documents [?] blocked marker"

# archive-spec should co-move the timeline file into done/, keeping
# the spec and its cycle history paired. SPEC-002 was archived in §5;
# verify its timeline (if one ever existed — SPEC-002 was scaffolded
# by new-spec in this test, so it got one) is also in done/.
ARCHIVED_TIMELINE="projects/PROJ-001-example-mvp/specs/done/SPEC-002-test-spec-timeline.md"
assert_file "$ARCHIVED_TIMELINE"
assert_no_file "projects/PROJ-001-example-mvp/specs/SPEC-002-test-spec-timeline.md"

# ============================================================
# 15) just report status writes reports/daily/<date>-status.md (DEC-001 Phase 3:
#     `daily-status-report` is now `report status`)
# ============================================================
just report status >/dev/null 2>&1
status_snap="reports/daily/$(date +%Y-%m-%d)-status.md"
assert_file "$status_snap"
assert_contains "$status_snap" "^# Daily status - $(date +%Y-%m-%d)\$" \
    "report status header names the date"
# The old `daily-status-report` name is GONE (breaking, DEC-001 Phase 3).
assert_cmd_fails "just daily-status-report is removed (consolidated to report status)" just daily-status-report

# ============================================================
# 16) just backlog (spec-grained "what's next" view)
# ============================================================
backlog_out=$(just backlog 2>&1)
if printf '%s\n' "$backlog_out" | grep -q "^Backlog for "; then
    pass "backlog prints header for active project"
else
    fail "backlog header missing"
fi
# Active stage's un-promoted bullets in the example project should
# include the four "(not yet written)" entries from STAGE-001 (the
# example project ships these). Surface at least one to confirm
# parsing works.
if printf '%s\n' "$backlog_out" | grep -qE "Typed error classes|Env-var loader|Health check"; then
    pass "backlog surfaces un-promoted bullets from active stage"
else
    fail "backlog did not surface un-promoted bullets"
fi
# In-flight section should mention SPEC-001 (the example spec is in
# design cycle in the example project).
if printf '%s\n' "$backlog_out" | grep -q "SPEC-001"; then
    pass "backlog lists in-flight specs"
else
    fail "backlog missing in-flight spec"
fi

# --all flag should not error out and should still produce output.
just backlog --all >/dev/null 2>&1 || fail "backlog --all exited non-zero"
pass "backlog --all exits cleanly"

# ============================================================
# 17) just roadmap (stage-grained view)
# ============================================================
roadmap_out=$(just roadmap 2>&1)
if printf '%s\n' "$roadmap_out" | grep -q "^Roadmap for "; then
    pass "roadmap prints header for active project"
else
    fail "roadmap header missing"
fi
# STAGE-001 should appear with its date range. The example ships
# with status: active, so we expect the active bucket.
if printf '%s\n' "$roadmap_out" | grep -qE "STAGE-001-foundational-infra.*active"; then
    pass "roadmap renders the active stage with bucket"
else
    fail "roadmap did not render active stage correctly"
fi
# Spec counts should appear for the active row (1 in flight, 4 backlog
# from the example project).
if printf '%s\n' "$roadmap_out" | grep -qE "1 in flight, 4 backlog"; then
    pass "roadmap shows correct spec counts for active stage"
else
    fail "roadmap spec counts wrong or missing"
fi
# Planned-but-unframed stages from the brief's ## Stage Plan (ROADMAP
# #8). The example brief plans two "(not yet defined)" stages that
# have no STAGE-*.md file yet; they should surface as "planned".
if printf '%s\n' "$roadmap_out" | grep -qE "planned .*Auth \+ primary flow"; then
    pass "roadmap surfaces planned-but-unframed stages from the brief"
else
    fail "roadmap did not surface planned stages"
fi
# A stage that IS framed (STAGE-001 has a file) must NOT be re-listed
# as planned — no double-counting between the file loop and the plan.
if printf '%s\n' "$roadmap_out" | grep -E "STAGE-001" | grep -q "planned"; then
    fail "roadmap double-listed a framed stage as planned"
else
    pass "roadmap does not double-list framed stages as planned"
fi
# --json exposes the same planned rows in a `planned` array.
roadmap_json=$(just roadmap --json 2>&1)
if printf '%s\n' "$roadmap_json" | grep -q '"bucket":"planned"' \
    && printf '%s\n' "$roadmap_json" | grep -q '"title":"Auth + primary flow"'; then
    pass "roadmap --json includes planned stages"
else
    fail "roadmap --json missing planned stages"
fi

# --- The DECLARED half of the roadmap (DEC-011) -----------------------------
# Absent by default: a brief with no roadmap: block emits an empty bucket, and
# nothing else about the output changes.
if printf '%s\n' "$roadmap_json" | grep -q '"declared":\[\]'; then
    pass "roadmap --json emits an empty declared bucket by default"
else
    fail "roadmap --json missing the declared bucket"
fi

# Declare two items: a pure pillar (no stage behind it) and a reference to the
# framed STAGE-001, which must reconcile rather than double-list.
ACTIVE_BRIEF=$(ls projects/PROJ-*/brief.md | head -1)
cp "$ACTIVE_BRIEF" "$SCRATCH/brief.orig"
# Insert the block right after the opening front-matter fence.
awk 'NR==1 && /^---$/ {
        print
        print "roadmap:"
        print "  - item: \"Portfolio rollup reports\""
        print "    kind: pillar"
        print "    horizon: next"
        print "    resume_when: \"after the daily report stabilizes\""
        print "  - item: STAGE-001"
        print "    horizon: now"
        next
     } { print }' "$SCRATCH/brief.orig" > "$ACTIVE_BRIEF"

lib_rows=$(bash -c 'REPO_ROOT="$(pwd)"; source scripts/_lib.sh; parse_declared_roadmap "$(dirname "'"$ACTIVE_BRIEF"'")"')
if printf '%s\n' "$lib_rows" | grep -q '^Portfolio rollup reports|pillar|next|after the daily report stabilizes|-$'; then
    pass "parse_declared_roadmap reads item/kind/horizon/resume_when"
else
    fail "parse_declared_roadmap row wrong: $lib_rows"
fi

roadmap_json=$(just roadmap --json 2>&1)
if printf '%s\n' "$roadmap_json" | grep -q '"item":"Portfolio rollup reports"'; then
    pass "roadmap --json surfaces a declared pillar"
else
    fail "roadmap --json missing the declared pillar: $roadmap_json"
fi
# The STAGE-001 reference must NOT appear as its own declared item...
if printf '%s\n' "$roadmap_json" | grep -q '"item":"STAGE-001"'; then
    fail "roadmap --json double-listed a declared stage reference"
else
    pass "roadmap --json reconciles a declared stage reference (no double-listing)"
fi
# ...its horizon rides along on the derived stage row instead.
if printf '%s\n' "$roadmap_json" | grep -q '"project.stage":"STAGE-001".*"horizon":"now"'; then
    pass "a declared horizon attaches to the derived stage row"
else
    fail "declared horizon did not reconcile onto STAGE-001: $roadmap_json"
fi
# Human output carries the pillar and the annotation.
roadmap_out=$(just roadmap 2>&1)
if printf '%s\n' "$roadmap_out" | grep -q "Portfolio rollup reports"; then
    pass "roadmap renders the declared pillar"
else
    fail "roadmap human output missing the declared pillar"
fi

# validate: unrecognized kind/horizon is ADVISORY — warns, never fails.
sed -i.bak 's/^    horizon: now$/    horizon: someday/' "$ACTIVE_BRIEF"
rm -f "${ACTIVE_BRIEF}.bak"
val_out=$(just validate 2>&1) || fail "validate failed on an unrecognized roadmap horizon (must be advisory)"
if printf '%s\n' "$val_out" | grep -q "unrecognized roadmap kind/horizon"; then
    pass "validate warns (advisory) on an unrecognized roadmap horizon"
else
    fail "validate did not surface the roadmap advisory: $val_out"
fi

# Restore the brief so later sections see the fixture they expect.
cp "$SCRATCH/brief.orig" "$ACTIVE_BRIEF"

# ============================================================
# security: titles with sed metachars are escaped, not injected
# ============================================================
INJECT_MARKER="/tmp/spec-driven-template-inject-$$"
rm -f "$INJECT_MARKER"
NASTY_TITLE="Pwn|e touch ${INJECT_MARKER} & a\\b"
if just new-stage "$NASTY_TITLE" >/dev/null 2>&1; then
    pass "new-stage accepts a title containing sed metacharacters"
else
    fail "new-stage failed on a title with sed metacharacters (should escape, not break)"
fi
if [ -e "$INJECT_MARKER" ]; then
    fail "SECURITY: sed injection — marker file was created from a hostile title"
    rm -f "$INJECT_MARKER"
else
    pass "no command injection from a hostile title"
fi
NASTY_STAGE=$(ls projects/PROJ-001-example-mvp/stages/STAGE-*pwn* 2>/dev/null | head -n1 || true)
if [ -n "$NASTY_STAGE" ] && grep -qF 'Pwn|e touch' "$NASTY_STAGE"; then
    pass "hostile title is rendered verbatim in the generated file"
else
    fail "hostile title was not rendered verbatim"
fi

# ============================================================
# specs-by-stage: flat ledger across scopes
# ============================================================
sbs_all=$(just specs-by-stage 2>&1)
if printf '%s\n' "$sbs_all" | grep -qE "Specs by stage — all projects"; then
    pass "specs-by-stage defaults to all projects"
else
    fail "specs-by-stage default header wrong: $sbs_all"
fi
if printf '%s\n' "$sbs_all" | grep -qE "^Totals: [0-9]+ shipped"; then
    pass "specs-by-stage prints a totals line"
else
    fail "specs-by-stage totals line missing: $sbs_all"
fi
if printf '%s\n' "$sbs_all" | grep -qE "STAGE-001-foundational-infra"; then
    pass "specs-by-stage groups specs under their stage"
else
    fail "specs-by-stage did not render STAGE-001: $sbs_all"
fi
sbs_active=$(just specs-by-stage --active 2>&1)
if printf '%s\n' "$sbs_active" | grep -qE "active project \(PROJ-001"; then
    pass "specs-by-stage --active scopes to the active project"
else
    fail "specs-by-stage --active header wrong: $sbs_active"
fi
assert_cmd_fails "specs-by-stage rejects an unknown flag" \
    just specs-by-stage --bogus

# ============================================================
# release-notes (v0.6.24) — assembled from each spec's outward outcome (Q5)
# ============================================================
# The Q5 answer slot must exist, or the outcome can't be recorded at all.
assert_contains "projects/_templates/spec.md" "answer \| none" \
    "spec template gives Reflection Q5 an answer slot"

just new-spec "Export to CSV" STAGE-002 >/dev/null 2>&1
RN_A=$(find projects/PROJ-001-example-mvp/specs -maxdepth 1 -name 'SPEC-*-export-to-csv.md' | head -1)
sed_inplace_portable "s/^  cycle: .*/  cycle: ship/" "$RN_A"
sed_inplace_portable "s/^   — <answer | none>/   — Users can export a report to CSV; before they retyped it by hand./" "$RN_A"
just new-spec "Internal refactor" STAGE-002 >/dev/null 2>&1
RN_B=$(find projects/PROJ-001-example-mvp/specs -maxdepth 1 -name 'SPEC-*-internal-refactor.md' | head -1)
sed_inplace_portable "s/^  cycle: .*/  cycle: ship/" "$RN_B"
sed_inplace_portable "s/^   — <answer | none>/   — none/" "$RN_B"

got_outcome=$(bash -c "source scripts/_lib.sh; get_spec_outcome '$RN_A'")
case "$got_outcome" in
    *"export a report to CSV"*) pass "get_spec_outcome reads the Q5 outward outcome" ;;
    *) fail "get_spec_outcome did not read Q5: '$got_outcome'" ;;
esac
# `none` and the unfilled placeholder both yield empty — they stay OUT of notes.
got_none=$(bash -c "source scripts/_lib.sh; get_spec_outcome '$RN_B'")
assert_eq "$got_none" "" "get_spec_outcome treats 'none' as no outcome"

rn_out=$(just release-notes 2>&1)
if printf '%s\n' "$rn_out" | grep -q "export a report to CSV"; then
    pass "release-notes assembles a shipped spec's outcome"
else
    fail "release-notes missing the outcome: $rn_out"
fi
if printf '%s\n' "$rn_out" | grep -q "Internal refactor"; then
    fail "release-notes wrongly included a 'none' spec"
else
    pass "release-notes excludes specs with no user-visible outcome"
fi
if printf '%s\n' "$rn_out" | grep -qE "had no user-visible outcome"; then
    pass "release-notes counts the silent specs (silence is visible, not lost)"
else
    fail "release-notes did not report the no-outcome count: $rn_out"
fi
# --prompt wraps the same notes in a synthesis ask.
rn_prompt=$(just release-notes --prompt 2>&1)
if printf '%s\n' "$rn_prompt" | grep -q "copy everything below this line"; then
    pass "release-notes --prompt emits the synthesis ask"
else
    fail "release-notes --prompt missing the synthesis header"
fi
# The release-spec carries the one authored, outward-facing home.
assert_contains "projects/_templates/release-spec.md" "## Release Notes" \
    "release-spec template has a Release Notes section"
rm -f "$RN_A" "$RN_B"
rm -f projects/PROJ-001-example-mvp/specs/SPEC-*-export-to-csv-timeline.md \
      projects/PROJ-001-example-mvp/specs/SPEC-*-internal-refactor-timeline.md 2>/dev/null || true

# ============================================================
# depends_on + ready set + claim (v0.6.23) — the fan-out data foundation
# ============================================================
just new-spec "Alpha base" STAGE-002 >/dev/null 2>&1
just new-spec "Beta needs alpha" STAGE-002 >/dev/null 2>&1
DEP_A=$(find projects/PROJ-001-example-mvp/specs -maxdepth 1 -name 'SPEC-*-alpha-base.md' | head -1)
DEP_B=$(find projects/PROJ-001-example-mvp/specs -maxdepth 1 -name 'SPEC-*-beta-needs-alpha.md' | head -1)
DEP_A_ID=$(basename "$DEP_A" | sed -E 's/^(SPEC-[0-9]+).*/\1/')
sed_inplace_portable "s/^depends_on: \[\]/depends_on: [${DEP_A_ID}]/" "$DEP_B"

# The reader handles the inline list form.
got_dep=$(bash -c "source scripts/_lib.sh; get_spec_depends_on '$DEP_B'")
assert_eq "$got_dep" "$DEP_A_ID" "get_spec_depends_on reads an inline dependency list"

# Ready set: B is BLOCKED while A is unshipped.
ready_out=$(just ready 2>&1)
if printf '%s\n' "$ready_out" | grep -qE "waits on ${DEP_A_ID}"; then
    pass "ready: a spec with an unshipped dependency is blocked"
else
    fail "ready: dependency did not block: $ready_out"
fi

# Claim removes a spec from the ready set; a second claimant is refused.
just claim "$DEP_A_ID" alice >/dev/null 2>&1
got_claim=$(bash -c "source scripts/_lib.sh; get_spec_claimed_by '$DEP_A'")
assert_eq "$got_claim" "alice" "claim writes claimed_by"
ready_claimed=$(just ready 2>&1)
if printf '%s\n' "$ready_claimed" | grep -qE "1 claimed"; then
    pass "ready: a claimed spec is counted out of the ready set"
else
    fail "ready: claimed spec not excluded: $ready_claimed"
fi
assert_cmd_fails "claim refuses a spec already held by someone else" \
    just claim "$DEP_A_ID" bob
just unclaim "$DEP_A_ID" >/dev/null 2>&1
got_unclaim=$(bash -c "source scripts/_lib.sh; get_spec_claimed_by '$DEP_A'")
assert_eq "$got_unclaim" "" "unclaim clears claimed_by"

# Shipping the dependency unblocks the dependent.
sed_inplace_portable "s/^  cycle: .*/  cycle: ship/" "$DEP_A"
ready_after=$(just ready 2>&1)
if printf '%s\n' "$ready_after" | grep -qE "0 blocked"; then
    pass "ready: shipping a dependency unblocks its dependent"
else
    fail "ready: dependent still blocked after dep shipped: $ready_after"
fi

# A dangling depends_on ref is ADVISORY — it must never fail the gate.
sed_inplace_portable "s/^depends_on: \[${DEP_A_ID}\]/depends_on: [SPEC-909]/" "$DEP_B"
if just validate >/dev/null 2>&1; then
    pass "validate never fails on a dangling depends_on ref (advisory)"
else
    fail "validate wrongly failed on a dangling depends_on ref"
fi
dep_adv=$(just validate 2>&1 || true)
case "$dep_adv" in
    *"SPEC-909"*) pass "validate surfaces the dangling depends_on advisory" ;;
    *) fail "validate did not surface the dangling depends_on advisory" ;;
esac
# Clean up so later scopes/counts aren't perturbed.
rm -f "$DEP_A" "$DEP_B"
rm -f projects/PROJ-001-example-mvp/specs/SPEC-*-alpha-base-timeline.md \
      projects/PROJ-001-example-mvp/specs/SPEC-*-beta-needs-alpha-timeline.md 2>/dev/null || true

# ============================================================
# frame-stage / new-spec --outline (v0.6.25) — planned stage → dispatchable batch
# ============================================================
just new-stage "Frame Batch" >/dev/null 2>&1
FS_STAGE=$(find projects/PROJ-001-example-mvp/stages -maxdepth 1 -name 'STAGE-*-frame-batch.md' | head -1)
FS_STAGE_ID=$(basename "$FS_STAGE" | sed -E 's/^(STAGE-[0-9]+).*/\1/')
assert_file "$FS_STAGE"
# Give the stage a backlog: two un-promoted lines, one carrying a size tag, plus
# an already-promoted line that must be left alone.
sed_inplace_portable 's|^- \[ \] (not yet written) — <summary.*|- [ ] (not yet written) — Widget loader\
- [ ] (not yet written) — Widget cache [XL] with eviction\
- [ ] SPEC-900 (design) — Pre-existing item|' "$FS_STAGE"

# --dry-run writes nothing.
fs_dry=$(just frame-stage "$FS_STAGE_ID" --dry-run 2>&1)
case "$fs_dry" in
    *"Widget loader"*) pass "frame-stage --dry-run previews the promotions" ;;
    *) fail "frame-stage --dry-run did not preview: $fs_dry" ;;
esac
if grep -q "(not yet written) — Widget loader" "$FS_STAGE"; then
    pass "frame-stage --dry-run leaves the backlog untouched"
else
    fail "frame-stage --dry-run mutated the stage file"
fi

just frame-stage "$FS_STAGE_ID" >/dev/null 2>&1
FS_A=$(find projects/PROJ-001-example-mvp/specs -maxdepth 1 -name 'SPEC-*-widget-loader.md' | head -1)
FS_B=$(find projects/PROJ-001-example-mvp/specs -maxdepth 1 -name 'SPEC-*-widget-cache-with-eviction.md' | head -1)
if [ -n "$FS_A" ] && [ -n "$FS_B" ]; then
    pass "frame-stage promotes every (not yet written) line into a spec"
else
    fail "frame-stage did not create one spec per backlog line"
fi
FS_A_ID=$(basename "$FS_A" | sed -E 's/^(SPEC-[0-9]+).*/\1/')
assert_contains "$FS_A" "^  cycle: frame" "an outline spec is parked at cycle: frame"
assert_contains "$FS_A" "OUTLINE" "an outline spec carries the scope-not-approach banner"
assert_contains "$FS_A" "stage: ${FS_STAGE_ID}" "an outline spec attaches to its stage"
# The [XL] tag on the backlog line is carried onto the spec, not dropped.
assert_contains "$FS_B" "^  complexity: XL" "frame-stage carries the backlog size tag to task.complexity"
# The backlog line is rewritten in place with the stable ID; Count recomputed.
assert_contains "$FS_STAGE" "^- \[ \] ${FS_A_ID} \(frame\) — Widget loader" \
    "frame-stage rewrites the backlog line with the new stable ID"
assert_contains "$FS_STAGE" "^- \[ \] SPEC-900 \(design\) — Pre-existing item" \
    "frame-stage leaves already-promoted backlog lines alone"
assert_contains "$FS_STAGE" "\*\*Count:\*\* 1 shipped / 3 active / 1 pending" \
    "frame-stage recomputes the backlog Count"
# Prose in the section that merely NAMES "(not yet written)" is not a bullet and
# must never be promoted into a spec (the stage template explains itself there).
if find projects/PROJ-001-example-mvp/specs -maxdepth 1 -name 'SPEC-*-just-frame-stage*.md' | grep -q .; then
    fail "frame-stage promoted a prose line from the Spec Backlog section"
else
    pass "frame-stage promotes only checkbox bullets, not section prose"
fi
# The whole point: a stable ID a sibling can declare a dependency on.
sed_inplace_portable "s/^depends_on: \[\]/depends_on: [${FS_A_ID}]/" "$FS_B"
fs_ready=$(just ready 2>&1)
if printf '%s\n' "$fs_ready" | grep -qE "waits on ${FS_A_ID}"; then
    pass "frame-stage IDs are usable as depends_on targets in the ready set"
else
    fail "outline ID did not work as a depends_on target: $fs_ready"
fi
# Re-running is a no-op, not a duplicate batch.
fs_again=$(just frame-stage "$FS_STAGE_ID" 2>&1)
case "$fs_again" in
    *"nothing to frame"*) pass "frame-stage is a no-op once the backlog is promoted" ;;
    *) fail "frame-stage re-run did not no-op: $fs_again" ;;
esac
# --outline and --release are mutually exclusive (an outline is never a cut).
assert_cmd_fails "new-spec refuses --outline together with --release" \
    ./scripts/new-spec.sh "Bad Combo" "$FS_STAGE_ID" --outline --release
# The stage template teaches the ritual and its self-audit.
assert_contains "projects/_templates/stage.md" "frame-stage" \
    "stage template documents frame-stage in the Spec Backlog section"
assert_contains "projects/_templates/stage.md" "outlines survived unchanged" \
    "stage template asks the outline-survival question at stage close"
rm -f "$FS_A" "$FS_B"
rm -f projects/PROJ-001-example-mvp/specs/SPEC-*-widget-loader-timeline.md \
      projects/PROJ-001-example-mvp/specs/SPEC-*-widget-cache-with-eviction-timeline.md 2>/dev/null || true
rm -f "$FS_STAGE"

# ============================================================
# expected-vs-actual estimation loop (v0.6.26)
# ============================================================
just new-spec "Estimated work" STAGE-002 >/dev/null 2>&1
CAL_A=$(find projects/PROJ-001-example-mvp/specs -maxdepth 1 -name 'SPEC-*-estimated-work.md' | head -1)
assert_contains "$CAL_A" "complexity_actual: null" "spec template carries an actual-size slot"
assert_contains "$CAL_A" "tokens_estimate: null" "spec template carries a token-estimate slot"
assert_contains "projects/_templates/spec.md" "XS \| S \| M \| L \| XL \| XXL" \
    "spec template documents the t-shirt scale"

# The widened enum is a real gate value: XXL validates, gibberish does not.
sed_inplace_portable "s/^  complexity: .*/  complexity: XXL/" "$CAL_A"
if just validate >/dev/null 2>&1; then
    pass "validate accepts a t-shirt size (XXL) as task.complexity"
else
    fail "validate rejected XXL — the t-shirt scale is not wired into the gate"
fi
sed_inplace_portable "s/^  complexity: .*/  complexity: HUGE/" "$CAL_A"
assert_cmd_fails "validate still gates an unrecognized complexity" just validate
# An unrecognized ACTUAL size is ADVISORY — it must never fail the gate.
sed_inplace_portable "s/^  complexity: .*/  complexity: M/" "$CAL_A"
sed_inplace_portable "s/^  complexity_actual: .*/  complexity_actual: ENORMOUS/" "$CAL_A"
if just validate >/dev/null 2>&1; then
    pass "validate never fails on an unrecognized complexity_actual (advisory)"
else
    fail "validate wrongly gated on complexity_actual"
fi
cal_adv=$(just validate 2>&1 || true)
case "$cal_adv" in
    *ENORMOUS*) pass "validate surfaces the complexity_actual advisory" ;;
    *) fail "validate did not surface the complexity_actual advisory" ;;
esac

# Readers: expected, actual, token estimate.
sed_inplace_portable "s/^  complexity_actual: .*/  complexity_actual: L/" "$CAL_A"
sed_inplace_portable "s/^  tokens_estimate: .*/  tokens_estimate: 40000/" "$CAL_A"
got_exp=$(bash -c "source scripts/_lib.sh; get_spec_complexity '$CAL_A'")
got_act=$(bash -c "source scripts/_lib.sh; get_spec_complexity_actual '$CAL_A'")
got_est=$(bash -c "source scripts/_lib.sh; get_tokens_estimate '$CAL_A'")
assert_eq "$got_exp" "M" "get_spec_complexity reads the expected size"
assert_eq "$got_act" "L" "get_spec_complexity_actual reads the actual size"
assert_eq "$got_est" "40000" "get_tokens_estimate reads the token prediction"
assert_eq "$(bash -c "source scripts/_lib.sh; size_rank XXL")" "6" "size_rank orders the t-shirt scale"
assert_eq "$(bash -c "source scripts/_lib.sh; size_rank nope")" "0" "size_rank returns 0 for an unrecognized size"
# A null actual reads as empty, not the literal "null" (drift would be silent).
sed_inplace_portable "s/^  complexity_actual: .*/  complexity_actual: null/" "$CAL_A"
assert_eq "$(bash -c "source scripts/_lib.sh; get_spec_complexity_actual '$CAL_A'")" "" \
    "an unset actual size reads as empty, not 'null'"

# calibration: a shipped spec that took MORE than expected reads as under-estimated.
sed_inplace_portable "s/^  complexity_actual: .*/  complexity_actual: L/" "$CAL_A"
sed_inplace_portable "s/^  cycle: .*/  cycle: ship/" "$CAL_A"
sed_inplace_portable 's/^  sessions: \[\]/  sessions:\
    - cycle: build\
      tokens_total: 90000\
      interface: claude-code/' "$CAL_A"
cal_out=$(just calibration 2>&1)
case "$cal_out" in
    *"1 under-estimated"*) pass "calibration counts an actual bigger than expected as under-estimated" ;;
    *) fail "calibration size drift wrong: $cal_out" ;;
esac
case "$cal_out" in
    *"Observed token band"*) pass "calibration prints the observed token band per expected size" ;;
    *) fail "calibration did not print the observed band: $cal_out" ;;
esac
case "$cal_out" in
    *"1 under · 0 over"*) pass "calibration compares the token estimate with actual tokens" ;;
    *) fail "calibration token drift wrong: $cal_out" ;;
esac
if command -v python3 >/dev/null 2>&1; then
    cal_json=$(just calibration --json 2>/dev/null)
    if printf '%s' "$cal_json" | python3 -c 'import json,sys; json.load(sys.stdin)' 2>/dev/null; then
        pass "calibration --json is valid JSON"
    else
        fail "calibration --json is not valid JSON"
    fi
    cal_verdict=$(printf '%s' "$cal_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["specs"][0]["size_verdict"])' 2>/dev/null)
    assert_eq "$cal_verdict" "under" "calibration --json exposes the per-spec size verdict"
fi
# An unrecorded actual is surfaced, never treated as an error.
sed_inplace_portable "s/^  complexity_actual: .*/  complexity_actual: null/" "$CAL_A"
cal_unrec=$(just calibration 2>&1)
case "$cal_unrec" in
    *"no recorded actual size"*) pass "calibration surfaces specs with no recorded actual size" ;;
    *) fail "calibration did not surface the unrecorded actual: $cal_unrec" ;;
esac
assert_cmd_fails "calibration rejects an unknown flag" just calibration --nope
rm -f "$CAL_A"
rm -f projects/PROJ-001-example-mvp/specs/SPEC-*-estimated-work-timeline.md 2>/dev/null || true

# --- spec titles (v0.6.22) — the ledger was ID-only; stages showed their slug
# --- but specs didn't. Title renders last, untruncated, in both surfaces.
if printf '%s\n' "$sbs_all" | grep -q "· title"; then
    pass "specs-by-stage legend documents the title column"
else
    fail "specs-by-stage legend missing the title column"
fi
if printf '%s\n' "$sbs_all" | grep -q "Test Spec"; then
    pass "specs-by-stage renders each spec's human title"
else
    fail "specs-by-stage did not render spec titles: $sbs_all"
fi
# --json carries it too — consumers (standup) read the title from here.
sbs_json=$(just specs-by-stage --json 2>/dev/null)
if printf '%s\n' "$sbs_json" | grep -q '"title"'; then
    pass "specs-by-stage --json includes a title field"
else
    fail "specs-by-stage --json missing the title field"
fi
if printf '%s\n' "$sbs_json" | grep -q 'Test Spec'; then
    pass "specs-by-stage --json title carries the real value"
else
    fail "specs-by-stage --json title value missing"
fi
# get_spec_title degrades to the filename slug when the H1 is absent/odd.
fb_dir=$(mktemp -d 2>/dev/null || mktemp -d -t 'spectitle')
printf -- '---\ntask:\n  id: SPEC-999\n---\n\nno heading here\n' \
    > "${fb_dir}/SPEC-999-fallback-slug-title.md"
fb_title=$(bash -c "source scripts/_lib.sh; get_spec_title '${fb_dir}/SPEC-999-fallback-slug-title.md'")
assert_eq "$fb_title" "fallback slug title" "get_spec_title falls back to the filename slug"
rm -rf "$fb_dir"

# ============================================================
# decisions-audit: lint + scope auditing
# ============================================================
# Clean state: the example DEC-001 is well-formed and has an
# affected_scope, so a plain audit should pass and report "clean".
audit_out=$(just decisions-audit 2>&1)
if printf '%s\n' "$audit_out" | grep -qE "clean: structure valid"; then
    pass "decisions-audit reports clean on the example decision"
else
    fail "decisions-audit did not report clean: $audit_out"
fi

# A structurally broken decision (missing created_at + insight.type and
# a dangling supersedes) must make the audit exit non-zero.
cat > decisions/DEC-666-broken.md <<'BROKEN'
---
insight:
  id: DEC-666
supersedes: DEC-999
superseded_by: null
---

# DEC-666: Intentionally broken
BROKEN
assert_cmd_fails "decisions-audit exits non-zero on a broken decision" \
    just decisions-audit
rm -f decisions/DEC-666-broken.md

# --changed maps pending edits to the decisions that govern them.
# Needs a git repo (scratch had its .git removed at setup), so init one.
git init -q >/dev/null 2>&1
git add -A >/dev/null 2>&1
git commit -qm "scratch baseline" >/dev/null 2>&1
mkdir -p src/lib
echo "// touched" >> src/lib/log.ts
changed_out=$(just decisions-audit --changed 2>&1)
if printf '%s\n' "$changed_out" | grep -qE "DEC-001"; then
    pass "decisions-audit --changed flags DEC-001 for an edit to src/lib/log.ts"
else
    fail "decisions-audit --changed missed DEC-001: $changed_out"
fi

# --- DEC `status:` (harvested from the template's own log) ------------------
# Declared status wins over the derived active/superseded, so a `proposed`
# decision no longer renders as though it were in force.
cat > decisions/DEC-700-proposed.md <<'PROPOSED'
---
insight:
  id: DEC-700
  type: decision
  confidence: 0.5
repo:
  id: my-app
created_at: 2026-08-12
supersedes: null
superseded_by: null
status: proposed
deciders: [a-human, claude]
---

# DEC-700: A draft that is not binding yet
PROPOSED
if just dash decisions 2>/dev/null | grep -qE "DEC-700 .*proposed"; then
    pass "dash decisions renders a declared status: proposed (not 'active')"
else
    fail "dash decisions did not honor status: proposed"
fi
if just dash decisions --json 2>/dev/null | grep -q '"deciders":\["a-human","claude"\]'; then
    pass "dash decisions --json emits deciders"
else
    fail "dash decisions --json missing deciders"
fi

# The one hard rule: status and superseded_by must not disagree about the
# same fact. superseded_by set + a non-superseded status is a structural error.
sed -i.bak 's/^superseded_by: null$/superseded_by: DEC-001/' decisions/DEC-700-proposed.md
rm -f decisions/DEC-700-proposed.md.bak
assert_cmd_fails "decisions-audit rejects status/superseded_by contradiction" \
    just decisions-audit
rm -f decisions/DEC-700-proposed.md

# An UNDECLARED status still derives exactly as before — the field is additive,
# so a pre-existing decision log (which declares nothing) reads unchanged.
cat > decisions/DEC-702-undeclared.md <<'UNDECLARED'
---
insight:
  id: DEC-702
  type: decision
  confidence: 0.9
repo:
  id: my-app
created_at: 2026-08-12
supersedes: null
superseded_by: null
---

# DEC-702: A decision from before the status field existed
UNDECLARED
if just dash decisions 2>/dev/null | grep -qE "DEC-702 .*active"; then
    pass "an undeclared status still derives 'active' (additive, no migration)"
else
    fail "undeclared status no longer derives 'active'"
fi
rm -f decisions/DEC-702-undeclared.md

# ============================================================
# decisions-index: the generated INDEX.md
# ============================================================
# Opt-in by existence: before any index exists, --check must pass quietly. This
# is what lets the CI job ship to every instance without breaking the ones that
# never wanted an index.
assert_no_file "decisions/INDEX.md"
just decisions-index --check >/dev/null 2>&1 \
    && pass "decisions-index --check passes when no index has been generated" \
    || fail "decisions-index --check failed on a repo with no index"

just decisions-index >/dev/null 2>&1 || fail "just decisions-index exited non-zero"
assert_file "decisions/INDEX.md"
assert_contains "decisions/INDEX.md" "GENERATED by" "INDEX.md marks itself generated"
assert_contains "decisions/INDEX.md" "DEC-001" "INDEX.md lists the example decision"
assert_contains "decisions/INDEX.md" "^## Active" "INDEX.md splits out an Active table"
# affected_scope is deliberately not a column (unreadable at scale; it's a
# different view). Guard the decision so it isn't quietly re-added.
if grep -q "src/lib/log.ts" decisions/INDEX.md; then
    fail "INDEX.md leaked affected_scope into the table"
else
    pass "INDEX.md omits affected_scope by design"
fi
# --check is the CI guard: clean right after a regenerate...
just decisions-index --check >/dev/null 2>&1 \
    && pass "decisions-index --check passes on a fresh index" \
    || fail "decisions-index --check failed on a fresh index"
# ...and fails once the log changes underneath it.
cp decisions/DEC-001-example-structured-logging.md decisions/DEC-701-new.md
sed -i.bak 's/id: DEC-001/id: DEC-701/' decisions/DEC-701-new.md
rm -f decisions/DEC-701-new.md.bak
assert_cmd_fails "decisions-index --check fails when the index is stale" \
    just decisions-index --check
rm -f decisions/DEC-701-new.md
just decisions-index >/dev/null 2>&1

# ============================================================
# cost-audit: the cost-capture gate has teeth
# ============================================================
# SPEC-002 was archived (shipped) in §5 with an empty cost block, so the
# gate must fail until real build/verify numbers are recorded.
assert_cmd_fails "cost-audit fails when a shipped spec lacks build/verify cost" \
    just cost-audit

# status surfaces the same gap.
status_cost_out=$(just status 2>&1)
if printf '%s\n' "$status_cost_out" | grep -qE "Specs missing cost data"; then
    pass "status shows the 'Specs missing cost data' section"
else
    fail "status missing the cost-data section: $status_cost_out"
fi
if printf '%s\n' "$status_cost_out" | grep -qE "SPEC-002.*missing"; then
    pass "status lists SPEC-002 as missing build/verify cost"
else
    fail "status did not flag SPEC-002: $status_cost_out"
fi

# Grandfathering a pre-process spec lets the gate pass (empty list by
# default; here we opt SPEC-002 out via the env var).
if COST_AUDIT_GRANDFATHERED=SPEC-002 just cost-audit >/dev/null 2>&1; then
    pass "cost-audit passes when SPEC-002 is grandfathered"
else
    fail "cost-audit still failed with SPEC-002 grandfathered"
fi

# Recording real build+verify tokens clears the gate without grandfathering.
awk '
    /^  sessions: \[\]/ {
        print "  sessions:"
        print "    - cycle: build"
        print "      interface: claude-code"
        print "      tokens_total: 120000"
        print "      estimated_usd: 0.50"
        print "      recorded_at: 2026-06-17"
        print "    - cycle: verify"
        print "      interface: claude-code"
        print "      tokens_total: 30000"
        print "      estimated_usd: 0.15"
        print "      recorded_at: 2026-06-17"
        next
    }
    { print }
' "$ARCHIVED" > "$ARCHIVED.tmp" && mv "$ARCHIVED.tmp" "$ARCHIVED"
if just cost-audit >/dev/null 2>&1; then
    pass "cost-audit passes once real build/verify cost is recorded"
else
    fail "cost-audit failed after recording real cost"
fi

# #5 (harvest): implausibly-low metered cost is flagged as ADVISORY — surfaced
# but does NOT fail the gate (sub-agent metering can be silently truncated).
sed_inplace_portable 's/tokens_total: 120000/tokens_total: 500/' "$ARCHIVED"
if ca_out=$(just cost-audit 2>&1); then ca_rc=0; else ca_rc=$?; fi
if [ "$ca_rc" = 0 ] && printf '%s\n' "$ca_out" | grep -q "implausibly-low"; then
    pass "cost-audit flags implausibly-low metered cost (advisory, non-blocking; harvest #5)"
else
    fail "cost-audit did not warn on implausibly-low cost (rc=${ca_rc}): ${ca_out}"
fi
sed_inplace_portable 's/tokens_total: 500/tokens_total: 120000/' "$ARCHIVED"

# specs-by-stage now shows the cost column header and a recorded-cost total.
sbs_cost=$(just specs-by-stage 2>&1)
if printf '%s\n' "$sbs_cost" | grep -qE "cost \(usd · tokens\)"; then
    pass "specs-by-stage header advertises the cost column"
else
    fail "specs-by-stage missing cost column header: $sbs_cost"
fi
if printf '%s\n' "$sbs_cost" | grep -qE "Recorded cost:"; then
    pass "specs-by-stage prints a Recorded cost total"
else
    fail "specs-by-stage missing Recorded cost line: $sbs_cost"
fi

# ============================================================
# dash: unified read command, lenses dispatch to the existing views
# ============================================================
# Each lens must reproduce the view it aliases.
dash_now=$(just dash now 2>&1)
if printf '%s\n' "$dash_now" | grep -qE "Specs missing cost data"; then
    pass "dash now → status view"
else
    fail "dash now did not render the status view: $dash_now"
fi
dash_future=$(just dash future 2>&1)
if printf '%s\n' "$dash_future" | grep -qE "^Roadmap for "; then
    pass "dash future → roadmap view"
else
    fail "dash future did not render the roadmap view: $dash_future"
fi
dash_next=$(just dash next 2>&1)
if printf '%s\n' "$dash_next" | grep -qE "^Backlog for "; then
    pass "dash next → backlog view"
else
    fail "dash next did not render the backlog view: $dash_next"
fi
dash_ledger=$(just dash ledger 2>&1)
if printf '%s\n' "$dash_ledger" | grep -qE "Specs by stage —|Recorded cost:"; then
    pass "dash ledger → specs-by-stage view"
else
    fail "dash ledger did not render the ledger view: $dash_ledger"
fi
# Flags pass through the lens to the underlying view.
just dash ledger --active >/dev/null 2>&1 || fail "dash ledger --active exited non-zero"
pass "dash ledger passes flags through to specs-by-stage"
# Default (no lens) stitches the dashboard.
dash_def=$(just dash 2>&1)
if printf '%s\n' "$dash_def" | grep -qE "Dashboard —" && printf '%s\n' "$dash_def" | grep -qE "Recorded cost"; then
    pass "dash (no arg) stitches now + future + recorded cost"
else
    fail "dash default dashboard missing expected sections: $dash_def"
fi
# Unknown lens is rejected (no silent fall-through to the dashboard).
assert_cmd_fails "dash rejects an unknown lens" just dash bogus

# ============================================================
# validate: the schema gate (structural front-matter)
# ============================================================
just validate >/dev/null 2>&1 && pass "validate passes on a well-formed repo" \
    || fail "validate failed on a clean repo"
# A spec with an invalid enum value must fail the gate.
cat > projects/PROJ-001-example-mvp/specs/SPEC-099-broken.md <<'BROKENSPEC'
---
task:
  id: SPEC-099
  type: task
  cycle: bogus
  complexity: S
project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: my-app
---
BROKENSPEC
assert_cmd_fails "validate fails on an invalid task.cycle" just validate
rm -f projects/PROJ-001-example-mvp/specs/SPEC-099-broken.md
just validate >/dev/null 2>&1 && pass "validate passes again once the bad spec is removed" \
    || fail "validate still failing after the bad spec was removed"
# Prompt files (specs/prompts/SPEC-*.md) share the glob but must NOT be
# validated as specs — the example ships SPEC-001-build/design prompt files.
if [ -f projects/PROJ-001-example-mvp/specs/prompts/SPEC-001-build.md ]; then
    just validate >/dev/null 2>&1 && pass "validate ignores specs/prompts/ cycle-prompt files" \
        || fail "validate wrongly flagged a prompts/ file"
fi

# ============================================================
# project.activity: optional, open-set, warn-only (v0.6.15)
# ============================================================
ACTIVITY_BRIEF="projects/PROJ-001-example-mvp/brief.md"
# BSD sed can't insert a newline, so add/replace/remove the activity
# line with awk. Empty VALUE removes it.
brief_set_activity() {  # brief_set_activity FILE VALUE
    awk -v val="$2" '
        /^  activity:/ { next }
        { print }
        /^  status:/ && val != "" && !ins { print "  activity: " val; ins = 1 }
    ' "$1" > "$1.tmp" && mv "$1.tmp" "$1"
}

# A recognized activity is valid and read back by _lib.
brief_set_activity "$ACTIVITY_BRIEF" requirements
just validate >/dev/null 2>&1 \
    && pass "validate passes with a recognized project.activity" \
    || fail "validate wrongly failed on a recognized activity"
got_activity=$(bash -c 'source scripts/_lib.sh; get_project_activity projects/PROJ-001-example-mvp')
assert_eq "$got_activity" "requirements" "get_project_activity reads a set value"

# An UNRECOGNIZED activity must NEVER fail the gate (open set) but SHOULD
# surface an advisory naming the value.
brief_set_activity "$ACTIVITY_BRIEF" banana
if just validate >/dev/null 2>&1; then
    pass "validate never fails on an unrecognized activity (open set)"
else
    fail "validate wrongly failed on an unrecognized activity"
fi
# Capture then match (not `| grep -q`): under `set -o pipefail`, grep -q
# closing the pipe early would SIGPIPE validate and fail the pipeline.
activity_adv=$(just validate 2>&1 || true)
case "$activity_adv" in
    *"activity='banana'"*) pass "validate warns (advisory) on an unrecognized activity" ;;
    *) fail "validate did not surface the unrecognized-activity advisory" ;;
esac

# Absent/null activity reads as empty and stays clean.
brief_set_activity "$ACTIVITY_BRIEF" ""
got_empty=$(bash -c 'source scripts/_lib.sh; get_project_activity projects/PROJ-001-example-mvp')
assert_eq "$got_empty" "" "get_project_activity is empty when activity is absent"
just validate >/dev/null 2>&1 \
    && pass "validate clean after activity removed" \
    || fail "validate failing after activity removed"

# ============================================================
# --json: the structured-output contract (DEC-001 §2)
# ============================================================
HAVE_PY3=0; command -v python3 >/dev/null 2>&1 && HAVE_PY3=1
# Like json_ok, but for GATES: a gate exits non-zero by design when it finds
# violations, and `out=$(cmd)` under `set -e` would abort the suite. The
# contract being tested is "emits a valid envelope regardless of exit code".
json_ok_gate() {
    local label="$1"; shift
    local out; out=$("$@" 2>/dev/null || true)
    if printf '%s' "$out" | grep -q '"schema_version":1'; then
        pass "${label}: emits the envelope"
    else
        fail "${label}: missing envelope: $out"
    fi
    if [ "$HAVE_PY3" = 1 ]; then
        if printf '%s' "$out" | python3 -c 'import json,sys; json.load(sys.stdin)' 2>/dev/null; then
            pass "${label}: valid JSON"
        else
            fail "${label}: invalid JSON: $out"
        fi
    fi
}

json_ok() {
    local label="$1"; shift
    local out; out=$("$@" 2>/dev/null)
    if printf '%s' "$out" | grep -q '"schema_version":1'; then
        pass "${label}: emits the envelope"
    else
        fail "${label}: missing envelope: $out"
    fi
    if [ "$HAVE_PY3" = 1 ]; then
        if printf '%s' "$out" | python3 -c 'import json,sys; json.load(sys.stdin)' 2>/dev/null; then
            pass "${label}: valid JSON"
        else
            fail "${label}: invalid JSON: $out"
        fi
    fi
}
json_ok "status --json"         just status --json
json_ok "specs-by-stage --json" just specs-by-stage --json
json_ok "roadmap --json"        just roadmap --json
json_ok "backlog --json"        just backlog --json
json_ok "dash --json"           just dash --json
# A lens carries the underlying command name (delegation).
if just dash now --json 2>/dev/null | grep -q '"command":"status"'; then
    pass "dash now --json delegates to status"
else
    fail "dash now --json did not delegate to status"
fi
# Usage errors return exit 2 (distinct from gate failures, which are 1).
if just specs-by-stage --bogus >/dev/null 2>&1; then
    fail "specs-by-stage --bogus should have failed"
else
    rc=$?
    [ "$rc" = 2 ] && pass "usage error exits 2 (DEC-001 §2 contract)" \
        || fail "usage error exit was ${rc}, expected 2"
fi

# ============================================================
# template-version: the versioning system
# ============================================================
assert_file "VERSION"
ver=$(tr -d ' \t\n\r' < VERSION)
# Semver shape.
if printf '%s' "$ver" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    pass "VERSION is semver (${ver})"
else
    fail "VERSION is not semver: '${ver}'"
fi
# Human output names the template + the VERSION value.
tv=$(just template-version 2>&1)
if [ "$tv" = "spec-driven-template ${ver}" ]; then
    pass "template-version prints the VERSION value"
else
    fail "template-version output '${tv}' != 'spec-driven-template ${ver}'"
fi
# --json carries the same version.
if [ "$HAVE_PY3" = 1 ]; then
    if just template-version --json 2>/dev/null \
        | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="template-version" and d["data"]["version"]==sys.argv[1]' "$ver" 2>/dev/null; then
        pass "template-version --json carries the version"
    else
        fail "template-version --json wrong or invalid"
    fi
fi
# Drift guard: the VERSION value must appear in the newest CHANGELOG entry.
if grep -qE "v${ver}([^0-9]|\$)" "$TEMPLATE_ROOT/CHANGELOG.md"; then
    pass "VERSION matches a CHANGELOG entry (no drift)"
else
    fail "VERSION ${ver} not found in CHANGELOG.md (version/changelog drift)"
fi
# Stricter drift guard (v0.6.2): the NEWEST `## … (vX.Y.Z)` header must be the
# current VERSION — catches a VERSION bump whose new entry was never added on top
# (the old top would still "appear" and pass the weaker check above). And every
# versioned header must be unique — no accidental duplicate release sections.
newest_hdr_ver=$(grep -oE '^## .*\(v[0-9]+\.[0-9]+\.[0-9]+\)' "$TEMPLATE_ROOT/CHANGELOG.md" \
    | head -n1 | grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+' | sed 's/^v//')
if [ "$newest_hdr_ver" = "$ver" ]; then
    pass "newest CHANGELOG header is the current VERSION (v${ver} on top)"
else
    fail "newest CHANGELOG header is v${newest_hdr_ver:-∅}, not VERSION ${ver}"
fi
dup_ver=$(grep -oE '^## .*\(v[0-9]+\.[0-9]+\.[0-9]+\)' "$TEMPLATE_ROOT/CHANGELOG.md" \
    | grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+' | sort | uniq -d | head -n1)
if [ -z "$dup_ver" ]; then
    pass "no duplicate versioned CHANGELOG headers"
else
    fail "duplicate CHANGELOG header for version ${dup_ver}"
fi

# ============================================================
# DEC-007 (v0.6.1): app versioning scheme (calver default) + next-version
# ============================================================
# The config ships with the calver default, and the guide exists.
assert_contains ".repo-context.yaml" "^    scheme: calver" \
    "repo-context ships spec.version.scheme: calver (DEC-007 default)"
assert_file "docs/versioning.md"
assert_contains "docs/versioning.md" "template provenance" \
    "versioning.md draws the app-version vs VERSION-file distinction"
# next-version follows the scheme. The scratch repo has no git tags (no .git),
# so calver degrades to this month's .0 — deterministic.
nv_out=$(just next-version 2>&1)
this_ym=$(date +%Y.%m)
if printf '%s\n' "$nv_out" | grep -q "Scheme: calver" \
   && printf '%s\n' "$nv_out" | grep -qF "Next:   v${this_ym}.0"; then
    pass "next-version computes the calver default (v${this_ym}.0)"
else
    fail "next-version calver output wrong: ${nv_out}"
fi
if [ "$HAVE_PY3" = 1 ]; then
    just next-version --json 2>/dev/null \
        | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["data"]["scheme"]=="calver" and d["data"]["next"].startswith("v")' \
        && pass "next-version --json carries scheme + next" || fail "next-version --json wrong"
fi
# The scheme is overridable: flip to monotonic → v1 (no tags), then restore.
sed_inplace_portable 's/^    scheme: calver/    scheme: monotonic/' .repo-context.yaml
nv_mono=$(just next-version 2>&1)
if printf '%s\n' "$nv_mono" | grep -q "Scheme: monotonic" \
   && printf '%s\n' "$nv_mono" | grep -qF "Next:   v1"; then
    pass "next-version honors an overridden scheme (monotonic → v1)"
else
    fail "next-version monotonic override wrong: ${nv_mono}"
fi
sed_inplace_portable 's/^    scheme: monotonic/    scheme: calver/' .repo-context.yaml
# The release-spec points at next-version / the scheme, not a bare vX.Y.Z.
assert_contains "projects/_templates/release-spec.md" "just next-version" \
    "release-spec references just next-version (DEC-007)"

# ============================================================
# DEC-008 (v0.6.4): build provenance stamp (just build-info)
# ============================================================
# build-info emits a provenance stamp: a non-empty ref line + the details. The
# ref is a real git-describe when a repo is reachable and "unknown" otherwise —
# assert structure, not the value, so the check is environment-agnostic.
bi=$(just build-info 2>&1)
if [ -n "$(printf '%s\n' "$bi" | head -1)" ] \
   && printf '%s\n' "$bi" | grep -q "built_at:" \
   && printf '%s\n' "$bi" | grep -q "commit:"; then
    pass "build-info emits a stamp (ref line + commit + built_at)"
else
    fail "build-info output unexpected: $bi"
fi
json_ok "build-info --json"   just build-info --json
if [ "$HAVE_PY3" = 1 ]; then
    just build-info --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="build-info"; ks=set(d["data"].keys()); assert {"ref","commit","commit_short","dirty","built_at"} <= ks' \
        && pass "build-info --json (ref/commit/dirty/built_at)" || fail "build-info --json wrong/invalid"
fi
# The release-spec pre-flight now requires build provenance, and the guide docs it.
assert_contains "projects/_templates/release-spec.md" "build provenance" \
    "release-spec pre-flight requires build provenance (DEC-008)"
assert_contains "docs/versioning.md" "Build provenance" \
    "versioning.md documents build provenance + injection (DEC-008)"

# ============================================================
# Accomplishment-logging guidance survives init
# ============================================================
assert_contains "guidance/recommended-tools.md" "Accomplishment logging" \
    "recommended-tools documents accomplishment logging"
# DEC-010 (coaching, not a wrapper): default-on brag, agent calls it directly.
assert_contains "guidance/recommended-tools.md" "on by default" \
    "accomplishment logging is documented as on-by-default (DEC-010)"
assert_contains "AGENTS.md" "brag add" \
    "AGENTS ship step coaches calling brag directly (DEC-010)"
assert_contains ".repo-context.yaml" "^  accomplishments:" \
    "repo-context declares the accomplishments config (DEC-010)"
assert_contains ".repo-context.yaml" "tool: brag" \
    "default accomplishment tool is brag (DEC-010)"
# No wrapper: the log-win recipe / script were removed in v0.6.11.
assert_cmd_fails "just log-win no longer exists (agent calls brag directly)" just log-win SPEC-002
assert_no_file "scripts/log-accomplishment.sh"

# ============================================================
# dash governance lenses: decisions + questions
# ============================================================
# The example project ships DEC-001 (confidence 0.95) and two open questions.
dd=$(just dash decisions 2>&1)
if printf '%s\n' "$dd" | grep -qE "^Decisions \([0-9]+\)" && printf '%s\n' "$dd" | grep -q "DEC-001"; then
    pass "dash decisions lists DEC-* with a header"
else
    fail "dash decisions output unexpected: $dd"
fi
dq=$(just dash questions 2>&1)
if printf '%s\n' "$dq" | grep -qE "Open questions \([0-9]+ open" && printf '%s\n' "$dq" | grep -q "example-caching-strategy"; then
    pass "dash questions lists open questions"
else
    fail "dash questions output unexpected: $dq"
fi
# Both lenses emit valid JSON with the right command + attribute names.
if [ "$HAVE_PY3" = 1 ]; then
    just dash decisions --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="decisions" and d["data"]["count"]>=1 and "insight.id" in d["data"]["decisions"][0]' \
        && pass "dash decisions --json (insight.* names)" || fail "dash decisions --json wrong/invalid"
    just dash questions --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="questions" and d["data"]["open"]>=1 and "guidance.id" in d["data"]["questions"][0]' \
        && pass "dash questions --json (guidance.* names)" || fail "dash questions --json wrong/invalid"
fi
# The default dashboard surfaces governance flags (human + json).
if just dash 2>&1 | grep -qE "Flags.*open question"; then
    pass "default dash shows governance flags"
else
    fail "default dash missing governance flags"
fi
if [ "$HAVE_PY3" = 1 ]; then
    just dash --json 2>/dev/null | python3 -c 'import json,sys; f=json.load(sys.stdin)["data"]["flags"]; assert "open_questions" in f and "low_confidence_decisions" in f and "open_signals" in f' \
        && pass "default dash --json carries flags" || fail "default dash --json missing flags"
fi

# ============================================================
# Signals registry + dash signals lens
# ============================================================
# The registry artifact + authoring guide land at the repo root via init.
assert_file "guidance/signals.yaml"
assert_file "docs/signals.md"
# The lens lists the seeds, with the awaiting-disposition header.
ds=$(just dash signals 2>&1)
if printf '%s\n' "$ds" | grep -qE "Signals \([0-9]+ awaiting" && printf '%s\n' "$ds" | grep -q "lightweight-verify-lane"; then
    pass "dash signals lists the ledger with an awaiting-disposition header"
else
    fail "dash signals output unexpected: $ds"
fi
json_ok "dash signals --json"   just dash signals --json
# --json carries the signal.* payload, including a lesson's bar (codification bar survives).
if [ "$HAVE_PY3" = 1 ]; then
    just dash signals --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="signals" and d["data"]["awaiting"]>=1 and "signal.id" in d["data"]["signals"][0]; assert any(s["signal.type"]=="lesson" and s["signal.bar"] for s in d["data"]["signals"]), "a lesson must carry its bar"' \
        && pass "dash signals --json (signal.* names + lesson bar preserved)" || fail "dash signals --json wrong/invalid"
fi
# The default dashboard surfaces the awaiting-disposition signal count.
if just dash 2>&1 | grep -qE "signal\(s\) awaiting disposition"; then
    pass "default dash flags signals awaiting disposition"
else
    fail "default dash missing signals flag"
fi
# --- golden-path: the only signal type that records something that WORKED ----
# Every other type is a problem, so "this worked, copy it" had nowhere to live.
assert_contains "guidance/signals.yaml" "golden-path" \
    "signals.yaml documents the golden-path type"
assert_contains "docs/signals.md" "wrong paved road is worse" \
    "signals.md states the harder bar for golden paths"
cat >> guidance/signals.yaml <<'GPEOF'
  - id: example-golden-path
    type: golden-path
    summary: "Table-driven tests for the parser made every later change cheap."
    evidence: "PROJ-001 SPEC-003/007/011 — N=3 same-outcome"
    bar: 'N=3 same-outcome'
    status: watch
    disposition_at: project-close
    first_flagged: 2026-08-12
    last_touched: 2026-08-12
    raised_by: human
    notes: null
GPEOF
if [ "$HAVE_PY3" = 1 ]; then
    just dash signals --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert any(s["signal.type"]=="golden-path" for s in d["signals"]), d' \
        && pass "dash signals --json carries a golden-path signal" || fail "golden-path missing from signals --json"
fi
gp=$(just dash signals 2>&1)
if printf '%s\n' "$gp" | grep -q "example-golden-path"; then
    pass "dash signals lists a golden-path signal"
else
    fail "dash signals did not list the golden-path: $gp"
fi
# It must be a PROJECT-close signal — a golden path is only knowable once the
# whole wave is done, and it inherits that ritual rather than inventing one.
if printf '%s\n' "$gp" | grep -q "project-close"; then
    pass "golden-path is dispositioned at project close"
else
    fail "golden-path did not carry a project-close disposition"
fi
just validate >/dev/null 2>&1 \
    && pass "validate unaffected by the golden-path signal type" \
    || fail "validate broke after adding a golden-path signal"

# The close-disposition ritual is wired into both close prompts.
assert_contains "FIRST_SESSION_PROMPTS.md" "disposition_at: stage-close" \
    "stage-close prompt wires the signal disposition ritual"
assert_contains "FIRST_SESSION_PROMPTS.md" "disposition_at: project-close" \
    "project-close prompt wires the signal disposition ritual"

# ============================================================
# P1 fixes (v0.5.19) — from crustyimg + zany-animal-slots dogfood feedback
# ============================================================

# Cost-schema drift: the prompt cost snippets must record tokens_total (the
# field cost-audit reads), not tokens_input/tokens_output (following the old
# snippets verbatim guaranteed a cost-audit failure — zany #8).
if grep -qE '^[[:space:]]+tokens_input:' FIRST_SESSION_PROMPTS.md; then
    fail "FIRST_SESSION_PROMPTS still has tokens_input in a cost snippet (cost-audit drift)"
else
    pass "cost snippets converged on tokens_total (no tokens_input/output drift)"
fi
assert_contains "FIRST_SESSION_PROMPTS.md" "tokens_total: <REAL combined count" \
    "build cost snippet records tokens_total (the field cost-audit reads)"

# find_spec must exclude specs/prompts/: advance-cycle should hit the real spec,
# never a same-prefixed cycle-prompt file with no front-matter (zany #7).
just new-stage "P1 Stage" >/dev/null 2>&1
P1_STAGE=$(ls projects/PROJ-001-example-mvp/stages/STAGE-*-p1-stage.md 2>/dev/null | head -n1)
P1_STAGE_ID=$(basename "$P1_STAGE" 2>/dev/null | grep -oE 'STAGE-[0-9]+')
just new-spec "P1 Spec" "$P1_STAGE_ID" >/dev/null 2>&1
P1_SPEC=$(ls projects/PROJ-001-example-mvp/specs/SPEC-*-p1-spec.md 2>/dev/null | head -n1)
P1_SPEC_ID=$(basename "$P1_SPEC" 2>/dev/null | grep -oE 'SPEC-[0-9]+')
# Plant a look-alike prompt file with NO front-matter (the exact trap).
mkdir -p "$(dirname "$P1_SPEC")/prompts"
printf '# %s build prompt\nno front-matter here\n' "$P1_SPEC_ID" \
    > "$(dirname "$P1_SPEC")/prompts/${P1_SPEC_ID}-build.md"
just advance-cycle "$P1_SPEC_ID" build >/dev/null 2>&1
if grep -qE "^  cycle: build" "$P1_SPEC"; then
    pass "advance-cycle edits the real spec, not the prompts/ look-alike (find_spec fix)"
else
    fail "advance-cycle did not advance the real spec (find_spec prompts/ regression)"
fi

# archive-spec must perform the backlog edit it advertises (zany #9 / crustyimg):
# flip the entry to [x] shipped and recompute **Count:**.
awk -v id="$P1_SPEC_ID" '
    { print }
    /^## Spec Backlog/ && !seen { print ""; print "- [ ] " id " (build) — p1 backlog test"; seen=1 }
' "$P1_STAGE" > "$P1_STAGE.tmp" && mv "$P1_STAGE.tmp" "$P1_STAGE"
just advance-cycle "$P1_SPEC_ID" ship >/dev/null 2>&1
# Ship is the only moment the ACTUAL size is knowable, so archive-spec nudges
# for it — advisory: it must still archive successfully without one.
p1_archive_out=$(just archive-spec "$P1_SPEC_ID" 2>&1)
case "$p1_archive_out" in
    *complexity_actual*) pass "archive-spec nudges for an unrecorded actual size" ;;
    *) fail "archive-spec did not nudge for complexity_actual: $p1_archive_out" ;;
esac
if grep -qE "^- \[x\] ${P1_SPEC_ID} \(shipped on ${today}\)" "$P1_STAGE"; then
    pass "archive-spec flips the backlog entry to [x] shipped (with date)"
else
    fail "archive-spec did not update the backlog entry: $(grep "$P1_SPEC_ID" "$P1_STAGE")"
fi
if grep -qE '^\*\*Count:\*\* [1-9][0-9]* shipped / [0-9]+ active / [0-9]+ pending' "$P1_STAGE"; then
    pass "archive-spec recomputes the **Count:** line"
else
    fail "archive-spec did not recompute Count: $(grep '\*\*Count' "$P1_STAGE")"
fi

# Deterministic project resolution: an ambiguous PROJ-NNN glob is a HARD ERROR,
# not a silent head -n1 (zany #1). Create a decoy sharing PROJ-001's number.
mkdir -p "projects/PROJ-001-decoy/stages"
assert_cmd_fails "new-stage on an ambiguous PROJ-001 hard-errors" just new-stage "x" PROJ-001
rm -rf "projects/PROJ-001-decoy"

# ============================================================
# Repo-wide continuous numbering (v0.5.20)
# ============================================================
# A stage created in a fresh SECOND project must CONTINUE the repo-wide count,
# not restart at 001. Also exercises mkdir -p (the project has no stages/ dir).
before=$(find projects -name 'STAGE-*.md' -not -path '*/done/*' 2>/dev/null \
         | sed -E 's|.*/STAGE-0*([0-9]+).*|\1|' | sort -n | tail -n1)
mkdir -p projects/PROJ-002-num-test
just new-stage "Cross Proj" PROJ-002-num-test >/dev/null 2>&1
newstage=$(ls projects/PROJ-002-num-test/stages/STAGE-*-cross-proj.md 2>/dev/null | head -n1)
newnum=$(basename "$newstage" 2>/dev/null | sed -E 's|STAGE-0*([0-9]+).*|\1|')
if [ -n "$newnum" ] && [ "$newnum" = "$((before + 1))" ]; then
    pass "new-stage continues numbering repo-wide across projects (not restart at 001)"
else
    fail "expected STAGE-$((before + 1)), got STAGE-${newnum:-<none>} (repo max was ${before})"
fi
rm -rf projects/PROJ-002-num-test

# ============================================================
# Patch lane (v0.5.21, DEC-003)
# ============================================================
assert_file "projects/_templates/patch.md"

# new-patch scaffolds a first-class patch: task.type: patch, own PATCH-* seq, no stage.
just new-patch "Out Dir Auto Create" >/dev/null 2>&1
PATCH_FILE=$(ls projects/PROJ-001-example-mvp/patches/PATCH-*-out-dir-auto-create.md 2>/dev/null | head -n1)
PATCH_ID=$(basename "$PATCH_FILE" 2>/dev/null | grep -oE 'PATCH-[0-9]+')
if [ -n "$PATCH_FILE" ] && [ "$PATCH_ID" = "PATCH-001" ]; then
    pass "new-patch scaffolds PATCH-001 (its own repo-wide sequence)"
else
    fail "new-patch did not scaffold PATCH-001 (got '${PATCH_ID:-none}')"
fi
assert_contains "$PATCH_FILE" "type: patch" "patch uses task.type: patch"
assert_contains "$PATCH_FILE" "cycle: patch" "patch starts at cycle: patch"
if grep -qE '^[[:space:]]+stage:' "$PATCH_FILE"; then
    fail "patch should have NO project.stage line"
else
    pass "patch has no project.stage (attaches to the project, not a stage)"
fi

# advance-cycle accepts the patch lane's cycle and edits the real patch.
just advance-cycle "$PATCH_ID" verify >/dev/null 2>&1
assert_contains "$PATCH_FILE" "cycle: verify" "advance-cycle moves a patch patch->verify"

# validate treats a well-formed patch as first-class (passes).
if just validate >/dev/null 2>&1; then
    pass "validate passes with a well-formed patch present"
else
    fail "validate failed with a well-formed patch present"
fi
# ...and rejects a patch carrying a spec-only cycle (design ∉ patch|verify|ship).
BADPATCH="projects/PROJ-001-example-mvp/patches/PATCH-999-bad.md"
cp "$PATCH_FILE" "$BADPATCH"
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/^  cycle: verify.*/  cycle: design/' "$BADPATCH"; else sed -i 's/^  cycle: verify.*/  cycle: design/' "$BADPATCH"; fi
assert_cmd_fails "validate rejects a patch with an invalid (spec-only) cycle" just validate
rm -f "$BADPATCH"

# cost-audit gates a shipped patch missing its metered (patch+verify) cost.
just advance-cycle "$PATCH_ID" ship >/dev/null 2>&1
assert_cmd_fails "cost-audit fails on a shipped patch with no metered cost" just cost-audit

# status surfaces patches (human + --json). Capture first, then grep the string:
# a live `... | grep -q` closes the pipe on match and SIGPIPEs status under pipefail.
status_out=$(just status 2>&1)
if printf '%s\n' "$status_out" | grep -q "Patches in"; then
    pass "status lists patches by cycle"
else
    fail "status missing the Patches section"
fi
if [ "$HAVE_PY3" = 1 ]; then
    just status --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); ps=d["data"]["patches"]; assert ps and ps[0]["task.id"].startswith("PATCH-")' \
        && pass "status --json carries a patches[] array" || fail "status --json missing patches[]"
fi

# archive-patch files it under patches/done/; a second archive fails.
just archive-patch "$PATCH_ID" >/dev/null 2>&1
if [ -f "projects/PROJ-001-example-mvp/patches/done/$(basename "$PATCH_FILE")" ]; then
    pass "archive-patch moves the patch to patches/done/"
else
    fail "archive-patch did not move the patch to done/"
fi
assert_cmd_fails "double archive-patch fails" just archive-patch "$PATCH_ID"

# --- dash patches lens + reports include patches (v0.5.22) ---
# The patch is now archived (shipped) under patches/done/; the lens lists it.
dp=$(just dash patches 2>&1)
if printf '%s\n' "$dp" | grep -qE "^Patches — " && printf '%s\n' "$dp" | grep -q "$PATCH_ID"; then
    pass "dash patches lists the patch lane by cycle"
else
    fail "dash patches output unexpected: $dp"
fi
json_ok "dash patches --json"   just dash patches --json
if [ "$HAVE_PY3" = 1 ]; then
    just dash patches --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="patches" and d["data"]["total"]>=1 and d["data"]["patches"][0]["task.id"].startswith("PATCH-")' \
        && pass "dash patches --json (task.* payload)" || fail "dash patches --json wrong/invalid"
fi
# report-daily / report-weekly grow a Patches section when patches exist.
just report-daily >/dev/null 2>&1
daily_file=$(ls -t reports/daily/*.md 2>/dev/null | head -n1)
assert_contains "$daily_file" "## Patches" "report-daily includes a Patches section"
just report-weekly >/dev/null 2>&1
weekly_file=$(ls -t reports/weekly/*.md 2>/dev/null | head -n1)
assert_contains "$weekly_file" "## Patches" "report-weekly includes a Patches section"

# Remove the test patch so the shipped-without-cost file can't affect later runs.
rm -rf projects/PROJ-001-example-mvp/patches

# ============================================================
# Gate --json contracts (DEC-001 §2)
# ============================================================
json_ok_gate "validate --json"    just validate --json
if [ "$HAVE_PY3" = 1 ]; then
    just validate --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="validate" and d["data"]["ok"] is True and d["data"]["checked"]>=1 and d["data"]["violations"]==[]' \
        && pass "validate --json (clean gate: ok=true, empty violations)" || fail "validate --json clean payload wrong"
fi
# A broken artifact must appear as a STRUCTURED finding, not just an exit code.
BADJ="projects/PROJ-001-example-mvp/specs/SPEC-997-badjson.md"
cp projects/PROJ-001-example-mvp/specs/SPEC-002-test-spec.md "$BADJ" 2>/dev/null || \
  cp "$(ls projects/PROJ-001-example-mvp/specs/SPEC-*.md | head -n1)" "$BADJ"
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/^  cycle: .*/  cycle: bogus/' "$BADJ"; else sed -i 's/^  cycle: .*/  cycle: bogus/' "$BADJ"; fi
assert_cmd_fails "validate --json still exits 1 on violations" just validate --json
# Capture BEFORE piping: the gate exits 1 on violations, and under `pipefail` a
# live `just … | python3` would fail the pipeline on the gate's exit code rather
# than on the payload (same trap the patches test documents).
if [ "$HAVE_PY3" = 1 ]; then
    vjson=$(just validate --json 2>/dev/null || true)
    printf '%s' "$vjson" | python3 -c 'import json,sys; d=json.load(sys.stdin); v=d["data"]["violations"]; assert d["data"]["ok"] is False and any("task.cycle" in p for x in v for p in x["problems"]) and any(x["kind"]=="spec" for x in v)' \
        && pass "validate --json names the artifact and the failed check" || fail "validate --json violation payload wrong"
fi
rm -f "$BADJ"

json_ok_gate "cost-audit --json"  just cost-audit --json
if [ "$HAVE_PY3" = 1 ]; then
    cjson=$(just cost-audit --json 2>/dev/null || true)
    printf '%s' "$cjson" | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="cost-audit" and d["data"]["gate"] in ("enforced","disabled") and "violations" in d["data"]' \
        && pass "cost-audit --json (gate + violations payload)" || fail "cost-audit --json payload wrong"
fi

# ============================================================
# Spike lane (v0.6.27, DEC-012)
# ============================================================
assert_file "projects/_templates/spike.md"

# new-spike scaffolds at the REPO root (not under a project — a spike may
# precede any project), with its own SPIKE-* sequence.
just new-spike "Can Bubbletea Do Split Panes" "2h" >/dev/null 2>&1
SPIKE_FILE=$(ls spikes/SPIKE-*-can-bubbletea-do-split-panes.md 2>/dev/null | head -n1)
SPIKE_ID=$(basename "$SPIKE_FILE" 2>/dev/null | grep -oE 'SPIKE-[0-9]+')
if [ -n "$SPIKE_FILE" ] && [ "$SPIKE_ID" = "SPIKE-001" ]; then
    pass "new-spike scaffolds SPIKE-001 at the repo root (own sequence)"
else
    fail "new-spike did not scaffold spikes/SPIKE-001 (got '${SPIKE_ID:-none}')"
fi
assert_contains "$SPIKE_FILE" "type: spike"   "spike uses task.type: spike"
assert_contains "$SPIKE_FILE" "cycle: spike"  "spike starts at cycle: spike"

# Every __TOKEN__ placeholder must be substituted. This is the regression guard
# for the v5.8 substitution-escaping bug class: a prose placeholder containing
# the sed delimiter silently fails to substitute and ships a broken artifact.
if grep -q '__[A-Z_]*__' "$SPIKE_FILE"; then
    fail "new-spike left an unsubstituted placeholder: $(grep -o '__[A-Z_]*__' "$SPIKE_FILE" | sort -u | tr '\n' ' ')"
else
    pass "new-spike substitutes every __TOKEN__ placeholder"
fi
assert_contains "$SPIKE_FILE" "timebox: 2h"      "new-spike TIMEBOX lands in front-matter"
assert_contains "$SPIKE_FILE" "mode: question"   "new-spike defaults to mode: question"
assert_contains "$SPIKE_FILE" "id: null"         "spike project.id is null (may precede any project)"
if grep -qE '^[[:space:]]+stage:' "$SPIKE_FILE"; then
    fail "spike should have NO project.stage line"
else
    pass "spike has no project.stage (attaches to the repo, not a stage)"
fi

# A timebox containing the sed delimiter must survive substitution intact —
# the exact input that broke the original prose-placeholder implementation.
just new-spike "Pipe Timebox" "2h | then stop" >/dev/null 2>&1
PIPE_SPIKE=$(ls spikes/SPIKE-*-pipe-timebox.md 2>/dev/null | head -n1)
assert_contains "$PIPE_SPIKE" "2h | then stop" "new-spike survives a timebox containing '|'"
rm -f "$PIPE_SPIKE"

# --mode build is the vibe-coding session; an invalid mode is rejected.
just new-spike "Vibe Session" "1 session" "build" >/dev/null 2>&1
BUILD_SPIKE=$(ls spikes/SPIKE-*-vibe-session.md 2>/dev/null | head -n1)
assert_contains "$BUILD_SPIKE" "mode: build" "new-spike MODE=build sets the vibe-coding mode"
rm -f "$BUILD_SPIKE"
assert_cmd_fails "new-spike rejects an invalid mode" just new-spike "Bad Mode" "1d" "sideways"
assert_cmd_fails "new-spike requires a question" just new-spike

sed_test() { if [ "$(uname)" = "Darwin" ]; then sed -i '' "$@"; else sed -i "$@"; fi; }

# validate treats a mid-exploration spike as first-class and PASSES it: a spike
# still at `cycle: spike` has legitimately not answered anything yet.
if just validate >/dev/null 2>&1; then
    pass "validate passes a mid-exploration spike (cycle: spike, outcome null)"
else
    fail "validate failed on a well-formed mid-exploration spike"
fi

# ...but rejects a spike carrying a spec-only cycle (design ∉ spike|land).
BADSPIKE="spikes/SPIKE-999-bad.md"
cp "$SPIKE_FILE" "$BADSPIKE"
sed_test 's/^  cycle: spike.*/  cycle: design/' "$BADSPIKE"
assert_cmd_fails "validate rejects a spike with an invalid (spec-only) cycle" just validate
rm -f "$BADSPIKE"

# ...and rejects a spike with no question (a spike with no question is coding).
NOQ="spikes/SPIKE-998-noq.md"
cp "$SPIKE_FILE" "$NOQ"
sed_test 's/^  question: .*/  question: null/' "$NOQ"
assert_cmd_fails "validate rejects a spike with no spike.question" just validate
rm -f "$NOQ"

# advance-cycle accepts the spike lane's cycles and finds a repo-root spike.
just advance-cycle "$SPIKE_ID" land >/dev/null 2>&1
assert_contains "$SPIKE_FILE" "cycle: land" "advance-cycle moves a spike spike->land"

# THE TOOTH (DEC-012): a LANDED spike with a null outcome fails the gate. An
# un-landed spike is the exact failure this lane exists to prevent.
assert_cmd_fails "validate fails a landed spike with a null spike.outcome" just validate

# archive-spike refuses an un-landed spike (outcome still null).
assert_cmd_fails "archive-spike refuses a spike with no outcome" just archive-spike "$SPIKE_ID"

# With a real outcome it lands, files under spikes/done/, and stamps landed_at.
sed_test 's/^  outcome: null.*/  outcome: graduated/' "$SPIKE_FILE"
if just validate >/dev/null 2>&1; then
    pass "validate passes once spike.outcome is set"
else
    fail "validate still failing after spike.outcome was set"
fi
just archive-spike "$SPIKE_ID" >/dev/null 2>&1
ARCHIVED="spikes/done/$(basename "$SPIKE_FILE")"
if [ -f "$ARCHIVED" ]; then
    pass "archive-spike moves a landed spike to spikes/done/"
else
    fail "archive-spike did not move the spike to done/"
fi
assert_contains "$ARCHIVED" "landed_at: 2" "archive-spike stamps landed_at"
assert_cmd_fails "double archive-spike fails" just archive-spike "$SPIKE_ID"

# An unrecognized outcome is rejected rather than silently archived.
just new-spike "Bogus Outcome Spike" >/dev/null 2>&1
BOGUS=$(ls spikes/SPIKE-*-bogus-outcome-spike.md 2>/dev/null | head -n1)
BOGUS_ID=$(basename "$BOGUS" | grep -oE 'SPIKE-[0-9]+')
sed_test 's/^  outcome: null.*/  outcome: mostly-fine/' "$BOGUS"
assert_cmd_fails "archive-spike rejects an unrecognized outcome" just archive-spike "$BOGUS_ID"
assert_cmd_fails "validate rejects an unrecognized spike.outcome" just validate
rm -f "$BOGUS"

# status surfaces open spikes; `spike` is an accepted project.activity value.
just new-spike "Open Spike" "1d" >/dev/null 2>&1
OPEN_SPIKE=$(ls spikes/SPIKE-*-open-spike.md 2>/dev/null | head -n1)
status_out=$(just status 2>&1)
if printf '%s\n' "$status_out" | grep -q "Spikes"; then
    pass "status lists open spikes"
else
    fail "status missing the Spikes section"
fi
if [ "$HAVE_PY3" = 1 ]; then
    just status --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); s=d["data"]["spikes"]; assert s and s[0]["task.id"].startswith("SPIKE-")' \
        && pass "status --json carries a spikes[] array" || fail "status --json missing spikes[]"
fi
# --- dash spikes lens (the house rule: add a lens, not a new command) ---
ds=$(just dash spikes 2>&1)
if printf '%s\n' "$ds" | grep -qE "^Spikes — bounded exploration" && printf '%s\n' "$ds" | grep -q "SPIKE-"; then
    pass "dash spikes lists the spike lane by cycle"
else
    fail "dash spikes output unexpected: $ds"
fi
json_ok "dash spikes --json"   just dash spikes --json
if [ "$HAVE_PY3" = 1 ]; then
    just dash spikes --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="spikes" and d["data"]["total"]>=1 and d["data"]["spikes"][0]["task.id"].startswith("SPIKE-")' \
        && pass "dash spikes --json (task.*/spike.* payload)" || fail "dash spikes --json wrong/invalid"
fi
# The lens flags an un-landed spike (at `land` with a null outcome) — the one
# state the lane exists to make visible.
just advance-cycle "$(basename "$OPEN_SPIKE" | grep -oE 'SPIKE-[0-9]+')" land >/dev/null 2>&1
if just dash spikes 2>&1 | grep -q "un-landed"; then
    pass "dash spikes flags a landed spike with no outcome"
else
    fail "dash spikes did not flag the un-landed spike"
fi
assert_cmd_fails "dash spikes: validate agrees the un-landed spike is invalid" just validate

rm -f "$OPEN_SPIKE"
rm -rf spikes

# `spike` is a recognized project.activity (AGENTS.md §8 reserved it; DEC-012
# makes it official) — so it must NOT draw the unrecognized-activity advisory.
act_probe=$(just validate 2>&1 || true)
if printf '%s\n' "$act_probe" | grep -q "unrecognized project.activity"; then
    fail "validate warned on activity before the spike-activity probe ran"
else
    pass "validate is quiet on activity with no spikes present"
fi

# ============================================================
# dash constraints + dash handoffs lenses (v0.6.2)
# ============================================================
# constraints lens: severity-grouped view of guidance/constraints.yaml.
dc=$(just dash constraints 2>&1)
if printf '%s\n' "$dc" | grep -qE "^Constraints \([0-9]+ total" \
   && printf '%s\n' "$dc" | grep -q "no-secrets-in-code"; then
    pass "dash constraints lists rules by severity"
else
    fail "dash constraints output unexpected: $dc"
fi
json_ok "dash constraints --json"   just dash constraints --json
if [ "$HAVE_PY3" = 1 ]; then
    just dash constraints --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="constraints" and d["data"]["total"]>=1 and "constraint.severity" in d["data"]["constraints"][0]' \
        && pass "dash constraints --json (constraint.* payload)" || fail "dash constraints --json wrong/invalid"
fi
# handoffs lens: claude-only has none → the empty view + a valid empty --json.
dh=$(just dash handoffs 2>&1)
if printf '%s\n' "$dh" | grep -qE "^Handoffs \(0 open / 0 total\)"; then
    pass "dash handoffs shows the empty view in claude-only"
else
    fail "dash handoffs output unexpected: $dh"
fi
json_ok "dash handoffs --json"   just dash handoffs --json
if [ "$HAVE_PY3" = 1 ]; then
    just dash handoffs --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="handoffs" and d["data"]["total"]==0 and d["data"]["handoffs"]==[]' \
        && pass "dash handoffs --json (empty array in claude-only)" || fail "dash handoffs --json wrong/invalid"
fi

# ============================================================
# Harvest backlog fixes (v0.5.23)
# ============================================================

# cost.totals auto-compute: archive-spec recomputes totals from cost.sessions.
cat > projects/PROJ-001-example-mvp/specs/SPEC-777-cost-rollup.md <<'COSTSPEC'
---
task:
  id: SPEC-777
  type: chore
  cycle: ship
  complexity: S
project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: bragfile-test
cost:
  sessions:
    - cycle: build
      tokens_total: 5000
      estimated_usd: 0.03
    - cycle: verify
      tokens_total: 3000
      estimated_usd: 0.02
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---
# SPEC-777: cost rollup test
COSTSPEC
just archive-spec SPEC-777 >/dev/null 2>&1
ROLLUP="projects/PROJ-001-example-mvp/specs/done/SPEC-777-cost-rollup.md"
if grep -qE "^    tokens_total: 8000\$" "$ROLLUP" && grep -qE "^    session_count: 2\$" "$ROLLUP"; then
    pass "archive-spec recomputes cost.totals from sessions (8000 tok / 2 sessions)"
else
    fail "cost.totals not recomputed: $(grep -A3 'totals:' "$ROLLUP" 2>/dev/null)"
fi
rm -f "$ROLLUP"

# decisions-audit: parent/child nesting is info, not a scope warning.
cat > decisions/DEC-101-parent.md <<'PAR'
---
insight:
  id: DEC-101
  type: decision
created_at: 2026-06-27
affected_scope:
  - src/engine/**
---
# DEC-101: parent
PAR
cat > decisions/DEC-102-child.md <<'CHI'
---
insight:
  id: DEC-102
  type: decision
created_at: 2026-06-27
affected_scope:
  - src/engine/rng.ts
---
# DEC-102: child
CHI
nest_out=$(just decisions-audit 2>&1)
if printf '%s\n' "$nest_out" | grep -q "nested scope" && ! printf '%s\n' "$nest_out" | grep -qE "[1-9][0-9]* scope warning"; then
    pass "decisions-audit treats parent/child nesting as info, not a conflict"
else
    fail "decisions-audit nesting unexpected: $nest_out"
fi
rm -f decisions/DEC-101-parent.md decisions/DEC-102-child.md

# decisions-audit: a bare-name affected_scope (no separator/wildcard) warns.
cat > decisions/DEC-103-bare.md <<'BARE'
---
insight:
  id: DEC-103
  type: decision
created_at: 2026-06-27
affected_scope:
  - _headers
---
# DEC-103: bare scope
BARE
bare_out=$(just decisions-audit 2>&1)
if printf '%s\n' "$bare_out" | grep -q "has no path separator or wildcard"; then
    pass "decisions-audit warns on a bare-name affected_scope (false-confidence guard)"
else
    fail "decisions-audit bare-name unexpected: $bare_out"
fi
rm -f decisions/DEC-103-bare.md

# severity vocab mapping survives init (both the yaml header and schema-reference).
assert_contains "guidance/constraints.yaml" "critical, high -> blocking" \
    "constraints.yaml documents the severity mapping (critical/high -> blocking)"

# ============================================================
# Runtime coverage (v0.5.24): behavioral pre-flight + defect-catch-stage
# ============================================================
assert_contains "AGENTS.md" "Behavioral pre-flight" \
    "AGENTS.md documents the behavioral pre-flight convention (runtime gap)"
assert_contains "projects/_templates/spec.md" "Where was the worst defect caught" \
    "spec ship reflection carries the defect-catch-stage tag"
assert_contains "projects/_templates/patch.md" "Defect-catch-stage" \
    "patch reflection carries the defect-catch-stage tag"
# Design-time probe / measure-before-build convention (harvest #2, N=17).
assert_contains "AGENTS.md" "Design-time probe" \
    "AGENTS.md §12 carries the design-time-probe / measure-before-build convention"
assert_contains "FIRST_SESSION_PROMPTS.md" "measure-before-build" \
    "the SPEC-design prompt reinforces measure-before-build (harvest #2)"
# frame is documented optional (harvest #12: 0/122 specs used it).
assert_contains "AGENTS.md" "\`frame\` is optional" \
    "cycle model documents frame as optional (harvest #12)"
# agents.* tier fields clarified as NOT contamination (harvest #13).
assert_contains "projects/_templates/spec.md" "context contamination" \
    "spec agents block clarifies tier split isn't contamination (harvest #13)"

# ============================================================
# DEC-005 Phase 1: agent/cost config + graceful cost-audit
# ============================================================
assert_contains ".repo-context.yaml" "metering_source" \
    ".repo-context.yaml carries the DEC-005 cost config"
assert_contains ".repo-context.yaml" "default_model" \
    ".repo-context.yaml carries the DEC-005 agent config"
assert_file "docs/porting.md"

# A shipped patch with no cost normally FAILS cost-audit (metering enforced)...
just new-patch "Metering Test" >/dev/null 2>&1
MPID=$(ls projects/PROJ-001-example-mvp/patches/PATCH-*-metering-test.md 2>/dev/null | head -n1 | xargs -I{} basename {} | grep -oE 'PATCH-[0-9]+')
just advance-cycle "$MPID" ship >/dev/null 2>&1
assert_cmd_fails "cost-audit enforces cost by default (metering_source subagent_tokens)" just cost-audit
# ...but with metering_source: none the gate is DISABLED (no token source).
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/metering_source: subagent_tokens/metering_source: none/' .repo-context.yaml; else sed -i 's/metering_source: subagent_tokens/metering_source: none/' .repo-context.yaml; fi
if just cost-audit >/dev/null 2>&1; then
    pass "cost-audit gate disabled when metering_source=none (DEC-005 non-Claude unblock)"
else
    fail "cost-audit still failed with metering_source=none"
fi
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/metering_source: none/metering_source: subagent_tokens/' .repo-context.yaml; else sed -i 's/metering_source: none/metering_source: subagent_tokens/' .repo-context.yaml; fi
rm -rf projects/PROJ-001-example-mvp/patches

# ============================================================
# DEC-005 Phase 2: config-driven agents.* stamping + wording
# ============================================================
# The example tier_map has design=opus, build=sonnet, so a scaffolded spec's
# architect != implementer — proving tier_map is read (not the hardcoded
# fallback), and fixing the 'architect==implementer looks like contamination'
# misread.
just new-stage "P2 Stage" >/dev/null 2>&1
P2S=$(ls projects/PROJ-001-example-mvp/stages/STAGE-*-p2-stage.md 2>/dev/null | head -n1 | xargs -I{} basename {} | grep -oE 'STAGE-[0-9]+')
just new-spec "P2 Spec" "$P2S" >/dev/null 2>&1
P2SPEC=$(ls projects/PROJ-001-example-mvp/specs/SPEC-*-p2-spec.md 2>/dev/null | head -n1)
arch=$(awk '/^  architect:/{print $2; exit}' "$P2SPEC")
impl=$(awk '/^  implementer:/{print $2; exit}' "$P2SPEC")
if [ "$arch" = "claude-opus-4-7" ] && [ "$impl" = "claude-sonnet-4-6" ] && [ "$arch" != "$impl" ]; then
    pass "new-spec stamps agents.* from tier_map (design!=build, not hardcoded)"
else
    fail "agents stamping wrong: architect=$arch implementer=$impl"
fi
if grep -qE '__ARCHITECT_MODEL__|__IMPLEMENTER_MODEL__' "$P2SPEC"; then
    fail "scaffolded spec still has a model placeholder"
else
    pass "no model placeholders remain in the scaffolded spec"
fi
just new-patch "P2 Patch" >/dev/null 2>&1
P2P=$(ls projects/PROJ-001-example-mvp/patches/PATCH-*-p2-patch.md 2>/dev/null | head -n1)
pimpl=$(awk '/^  implementer:/{print $2; exit}' "$P2P")
[ "$pimpl" = "claude-sonnet-4-6" ] \
    && pass "new-patch stamps agents from tier_map.build" \
    || fail "patch implementer stamping wrong: $pimpl"
rm -rf projects/PROJ-001-example-mvp/patches
# Session wording generalized (DEC-005 §3): no 'Claude session' in AGENTS.
if grep -q "Claude session" AGENTS.md; then
    fail "AGENTS.md still says 'Claude session' (wording not generalized)"
else
    pass "session wording generalized (no 'Claude session' in AGENTS.md)"
fi

# ============================================================
# DEC-004 Phase 1: delegated-execution rules in AGENTS.md
# ============================================================
assert_contains "AGENTS.md" "Delegated execution" \
    "AGENTS.md documents the delegated-execution sub-agent rules"
assert_contains "AGENTS.md" "Reconcile over self-report" \
    "AGENTS.md carries the reconcile-over-self-report rule (DEC-004)"
# Rule 1 generalized beyond build output: a stale number in a doc is a
# self-report by a past session, and four of them were acted on before anyone
# checked the corpus.
assert_contains "AGENTS.md" "verify any claim you are about to act on" \
    "AGENTS.md generalizes rule 1 past build output"

# --- guidance/agents/: OPTIONAL reviewer briefs --------------------------------
# Ship-the-reader applied to documentation: a brief nothing points at is a brief
# nobody opens. Assert REACHABILITY, not just presence.
assert_file "guidance/agents/README.md"
assert_file "guidance/agents/reviewer-correctness.md"
assert_file "guidance/agents/reviewer-design.md"
assert_file "guidance/agents/qa-close.md"
assert_contains "AGENTS.md" "guidance/agents/" \
    "AGENTS.md points at the reviewer briefs (they are discoverable)"
# The rule that makes a brief worth anything — a persona inside the session that
# did the work is a costume, not a reviewer.
assert_contains "guidance/agents/README.md" "did not do the work" \
    "the briefs state the session-boundary rule"
# Each brief must narrow the review AND license a rejection. A reviewer that
# cannot say no is a formatter.
for brief in reviewer-correctness reviewer-design; do
    assert_contains "guidance/agents/${brief}.md" "[Dd]eliberately ignore" \
        "${brief} says what it ignores"
    assert_contains "guidance/agents/${brief}.md" "[Rr]eject only for" \
        "${brief} narrows its licence to reject"
done
assert_contains "guidance/agents/qa-close.md" "not-ready" \
    "qa-close carries its own verdict vocabulary"
# validate must not care about any of this — it is guidance, not schema.
just validate >/dev/null 2>&1 \
    && pass "validate unaffected by guidance/agents/" \
    || fail "validate broke after adding guidance/agents/"

# ============================================================
# DEC-004 Phase 2: dev-dep sanction (rule 4) + toolchain-brief slot (rule 5)
# ============================================================
# Rule 5: the toolchain-brief slot ships and survives init.
assert_file "guidance/toolchain-brief.md"
assert_contains "guidance/toolchain-brief.md" "Toolchain Brief" \
    "toolchain-brief.md is the per-repo toolchain-facts stub"
# AGENTS "During build" tells the agent to read it, and Pointers + the dir
# diagram surface it.
assert_contains "AGENTS.md" "guidance/toolchain-brief\.md" \
    "AGENTS.md references the toolchain brief (During build + Pointers)"
assert_contains "AGENTS.md" "toolchain-brief\.md.*DEC-004" \
    "AGENTS.md dir diagram lists toolchain-brief.md (DEC-004)"
# The delegated-execution section now carries all five rules.
assert_contains "AGENTS.md" "five rules keep" \
    "delegated-execution section grows to five rules (DEC-004 Phase 2)"
# Rule 4: the deps constraint carves out the trivial DEV-only + DEC exception.
assert_contains "guidance/constraints.yaml" "DEV-only dependency" \
    "deps constraint sanctions a trivial DEV-only dep in one build pass (rule 4)"
assert_contains "guidance/constraints.yaml" "top-level RUNTIME dependency" \
    "deps constraint scopes its hard gate to runtime deps (rule 4)"

# ============================================================
# DEC-006 (v0.5.29): release-spec template + runtime pre-flight
# ============================================================
# The template ships in _templates and survives init.
assert_file "projects/_templates/release-spec.md"
assert_contains "projects/_templates/release-spec.md" "type: release" \
    "release-spec template uses task.type: release"
assert_contains "projects/_templates/release-spec.md" "Release Pre-Flight" \
    "release-spec template carries the pre-flight checklist"
# Two-phase cut + evidence-now/deferred-to-cut timing (harvest #6).
assert_contains "projects/_templates/release-spec.md" "Release cut is two-phase" \
    "release-spec encodes the two-phase (reversible prep / irreversible cut) split"
assert_contains "projects/_templates/release-spec.md" "prep-complete, cut-deferred" \
    "release-spec marks the session ship state honestly (harvest #6)"
# All six generic categories are present (category-level, DEC-006).
for cat in "tag integrity" "Artifact trust on a clean host" \
           "Distribution-channel trust" "Data isolation" \
           "Runtime smoke on a clean host" "Rollback"; do
    if grep -qiE "$cat" "projects/_templates/release-spec.md"; then
        pass "release pre-flight covers: ${cat}"
    else
        fail "release pre-flight missing category: ${cat}"
    fi
done

# `just new-release-spec` scaffolds a task.type: release spec into a stage.
just new-release-spec "Test Release" STAGE-002 >/dev/null 2>&1
REL_FILE=$(ls projects/PROJ-001-example-mvp/specs/SPEC-*-test-release.md 2>/dev/null | head -n1)
if [ -n "$REL_FILE" ]; then
    pass "new-release-spec scaffolds a release spec"
else
    fail "new-release-spec did not scaffold a SPEC-*-test-release.md"
fi
assert_contains "$REL_FILE" "type: release" "scaffolded release spec is task.type: release"
assert_contains "$REL_FILE" "stage: STAGE-002" "release spec attaches to its stage"
# The --release flag on new-spec.sh is the primitive (wrapper just forwards it).
just new-spec "Flag Release" STAGE-002 --release >/dev/null 2>&1
FLAG_FILE=$(ls projects/PROJ-001-example-mvp/specs/SPEC-*-flag-release.md 2>/dev/null | head -n1)
assert_contains "$FLAG_FILE" "type: release" "--release flag on new-spec scaffolds a release spec"

# validate treats a release spec as first-class (it uses the standard spec path).
if just validate >/dev/null 2>&1; then
    pass "validate passes with release specs present (first-class)"
else
    fail "validate failed with a well-formed release spec present"
fi

# status recognizes releases: [release] tag (human) + task.type in --json.
rel_status_out=$(just status 2>&1)
if printf '%s\n' "$rel_status_out" | grep -q "\[release\]"; then
    pass "status tags release specs with [release]"
else
    fail "status did not tag the release spec"
fi
if [ "$HAVE_PY3" = 1 ]; then
    just status --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); ss=d["data"]["specs"]; assert any(s.get("task.type")=="release" for s in ss)' \
        && pass "status --json exposes task.type=release" || fail "status --json missing task.type=release"
fi
# Clean up the release specs so the final repo state is undisturbed.
rm -f "$REL_FILE" "$FLAG_FILE" \
    projects/PROJ-001-example-mvp/specs/SPEC-*-test-release-timeline.md \
    projects/PROJ-001-example-mvp/specs/SPEC-*-flag-release-timeline.md

# ============================================================
# DEC-001 Phase 3 (v0.6.0): the `report` namespace + `review` (breaking)
# ============================================================
# The new namespace: daily / weekly / status all resolve.
just report daily >/dev/null 2>&1 && pass "report daily works" || fail "report daily failed"
just report weekly >/dev/null 2>&1 && pass "report weekly works" || fail "report weekly failed"
just report status >/dev/null 2>&1 && pass "report status works" || fail "report status failed"
# A bad subcommand is a usage error (exit 2).
assert_cmd_fails "report with a bad subcommand errors" just report bogus
# The bare-name daily-drivers survive as PERMANENT aliases (muscle memory).
just report-daily >/dev/null 2>&1 && pass "report-daily alias still works" || fail "report-daily alias broke"
just report-weekly >/dev/null 2>&1 && pass "report-weekly alias still works" || fail "report-weekly alias broke"
# `review` is the consolidated weekly-review command.
just review >/dev/null 2>&1 && pass "review works" || fail "review failed"

# report --json (v0.6.3): a lean quantitative envelope on stdout, through the
# namespace AND the bare-name aliases; the flag is stripped from the DATE arg.
json_ok "report daily --json"    just report daily --json
json_ok "report weekly --json"   just report weekly --json
json_ok "report-daily --json"    just report-daily --json
json_ok "report-weekly --json"   just report-weekly --json
if [ "$HAVE_PY3" = 1 ]; then
    just report daily --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="report-daily" and "progress" in d["data"] and "cost" in d["data"]' \
        && pass "report daily --json (progress + cost envelope)" || fail "report daily --json wrong/invalid"
    just report weekly --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["command"]=="report-weekly" and "shipped_this_week" in d["data"] and "week" in d["data"]' \
        && pass "report weekly --json (week + shipped envelope)" || fail "report weekly --json wrong/invalid"
fi

# ============================================================
# Multi-wave resolver: get_active_project prefers status:active (harvest #1)
# ============================================================
# Two non-example waves: the LOWER-numbered one is shipped, the higher active.
# The old resolver (lowest-numbered non-example) would wrongly pick the shipped
# wave; the fix must pick the active one. Placed last + cleaned up so the extra
# projects don't perturb earlier checks.
mkdir -p projects/PROJ-050-old-wave projects/PROJ-051-new-wave
cat > projects/PROJ-050-old-wave/brief.md <<'BRIEF'
---
project:
  id: PROJ-050
  status: shipped
repo:
  id: test
---
# PROJ-050 old wave
BRIEF
cat > projects/PROJ-051-new-wave/brief.md <<'BRIEF'
---
project:
  id: PROJ-051
  status: active
repo:
  id: test
---
# PROJ-051 new wave
BRIEF
if [ "$HAVE_PY3" = 1 ]; then
    ap=$(just status --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["active_project"])')
    assert_eq "$ap" "PROJ-051-new-wave" "resolver prefers status:active over lowest-numbered (harvest #1)"
fi
# A trailing comment on status: must not defeat the resolver (field-2 parse).
sed_inplace_portable 's/^  status: active/  status: active   # current wave/' projects/PROJ-051-new-wave/brief.md
if [ "$HAVE_PY3" = 1 ]; then
    ap2=$(just status --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["active_project"])')
    assert_eq "$ap2" "PROJ-051-new-wave" "resolver tolerates a trailing comment on status:"
fi
# An `on_hold` project (blessed in the coarse status enum, v0.6.16) is NOT
# active: it must not be chosen as the active wave over a status:active one.
mkdir -p projects/PROJ-049-paused
cat > projects/PROJ-049-paused/brief.md <<'BRIEF'
---
project:
  id: PROJ-049
  status: on_hold
repo:
  id: test
---
# PROJ-049 paused wave
BRIEF
if [ "$HAVE_PY3" = 1 ]; then
    ap3=$(just status --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["active_project"])')
    assert_eq "$ap3" "PROJ-051-new-wave" "resolver skips an on_hold project in favor of the active one"
fi
rm -rf projects/PROJ-049-paused projects/PROJ-050-old-wave projects/PROJ-051-new-wave

# ============================================================
# dash defects lens — the defect-escape distribution (reads ship Reflection Q4)
# ============================================================
dd=$(just dash defects 2>&1)
if printf '%s\n' "$dd" | grep -q "Defect-escape distribution"; then
    pass "dash defects renders the distribution"
else
    fail "dash defects output unexpected: $dd"
fi
json_ok "dash defects --json"   just dash defects --json
# An UNANSWERED template must not be counted as data: the vocabulary words appear
# in the template's own prompt text (in backticks) and must never be parsed as
# an answer. This is the parser's central risk.
if [ "$HAVE_PY3" = 1 ]; then
    just dash defects --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert d["answered"]==0 and sum(d["distribution"].values())==0, d' \
        && pass "dash defects ignores the unanswered template vocabulary" || fail "dash defects counted template prose as an answer"
fi
# Now answer Q4 on a real spec and confirm it lands in the right bucket.
DSPEC=$(ls projects/PROJ-001-example-mvp/specs/SPEC-*.md | grep -v timeline | head -n1)
cat >> "$DSPEC" <<'DEFEOF'

## Reflection (Ship)

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — escaped
DEFEOF
if [ "$HAVE_PY3" = 1 ]; then
    just dash defects --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert d["answered"]==1 and d["distribution"]["escaped"]==1, d; assert d["escaped_artifacts"][0]["kind"]=="spec"' \
        && pass "dash defects counts an answered escaped defect" || fail "dash defects did not record the escaped answer"
fi
# Capture before grepping: `grep -q` closes the pipe on match, SIGPIPE-ing the
# producer, which `pipefail` then reports as failure (same trap as dash patches).
dd2=$(just dash defects 2>&1)
if printf '%s\n' "$dd2" | grep -q "ESCAPED"; then
    pass "dash defects surfaces escaped defects prominently"
else
    fail "dash defects did not highlight the escaped defect"
fi
# A different vocabulary word must land in its own bucket, not 'escaped'.
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/^   — escaped$/   — verify/' "$DSPEC"; else sed -i 's/^   — escaped$/   — verify/' "$DSPEC"; fi
if [ "$HAVE_PY3" = 1 ]; then
    just dash defects --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert d["distribution"]["verify"]==1 and d["distribution"]["escaped"]==0, d' \
        && pass "dash defects buckets a non-escaped answer correctly" || fail "dash defects mis-bucketed a verify answer"
fi

# --- task.verify_verdict: the second axis -----------------------------------
# The field ships with a READER, not just a slot — the reserved-but-unwired
# pattern (harvest signal #7) is the thing to avoid here.
VSPEC_ID=$(basename "$DSPEC" .md | sed -E 's/^(SPEC-[0-9]+).*/\1/')
just advance-cycle "$VSPEC_ID" verify >/dev/null 2>&1

# verify → build infers punch-list: a spec that goes BACK is one verify pushed
# on. Stamping only on the way to ship would drop every rejection silently.
adv=$(just advance-cycle "$VSPEC_ID" build 2>&1)
if printf '%s\n' "$adv" | grep -q "punch-list (inferred from → build)"; then
    pass "advance-cycle infers punch-list when a spec leaves verify backwards"
else
    fail "advance-cycle did not infer a punch-list verdict: $adv"
fi
assert_contains "$DSPEC" "verify_verdict: punch-list" "punch-list landed in front-matter"

# verify → ship infers approved.
just advance-cycle "$VSPEC_ID" verify >/dev/null 2>&1
just advance-cycle "$VSPEC_ID" ship >/dev/null 2>&1
assert_contains "$DSPEC" "verify_verdict: approved" "verify → ship infers approved"

# An explicit --verdict wins over the inference: rejected vs punch-list is the
# one call the destination cannot make for you.
just advance-cycle "$VSPEC_ID" verify >/dev/null 2>&1
just advance-cycle "$VSPEC_ID" build --verdict rejected >/dev/null 2>&1
assert_contains "$DSPEC" "verify_verdict: rejected" "--verdict overrides the inference"
assert_cmd_fails "advance-cycle rejects an unknown verdict" \
    just advance-cycle "$VSPEC_ID" build --verdict maybe

# The reader surfaces it, and the zero-pushback case says so explicitly —
# that is the PLAYBOOK failure signature this field exists to make computable.
if [ "$HAVE_PY3" = 1 ]; then
    just dash defects --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert d["verify_verdicts"]["rejected"]==1, d' \
        && pass "dash defects --json reports the verdict distribution" || fail "dash defects --json missing verify_verdicts"
fi
dd3=$(just dash defects 2>&1)
if printf '%s\n' "$dd3" | grep -q "Verify verdicts"; then
    pass "dash defects renders the verify-verdict axis"
else
    fail "dash defects did not render verify verdicts: $dd3"
fi
# Advisory only — an unrecognized verdict must never fail the gate.
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/^  verify_verdict: rejected$/  verify_verdict: bogus/' "$DSPEC"; else sed -i 's/^  verify_verdict: rejected$/  verify_verdict: bogus/' "$DSPEC"; fi
vout=$(just validate 2>&1) || fail "validate failed on an unrecognized verify_verdict (must be advisory)"
if printf '%s\n' "$vout" | grep -q "unrecognized task.verify_verdict"; then
    pass "validate warns (advisory) on an unrecognized verify_verdict"
else
    fail "validate did not surface the verify_verdict advisory: $vout"
fi
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/^  verify_verdict: bogus$/  verify_verdict: rejected/' "$DSPEC"; else sed -i 's/^  verify_verdict: bogus$/  verify_verdict: rejected/' "$DSPEC"; fi

# ============================================================
# close-project: the mechanical half of the close ritual
# ============================================================
# The whole point is that it REFUSES. A close that should fail must actually
# fail, or this is decoration — so the refusal paths are tested first.
cp_out=$(just close-project PROJ-001 2>&1 || true)
if printf '%s\n' "$cp_out" | grep -q "REFUSED"; then
    pass "close-project refuses a project that isn't ready"
else
    fail "close-project did not refuse: $cp_out"
fi
assert_cmd_fails "close-project exits non-zero when it refuses" \
    just close-project PROJ-001
# Refusal 1: specs in flight, and it must name them.
if printf '%s\n' "$cp_out" | grep -qE "spec\(s\) still in flight"; then
    pass "close-project refuses on in-flight specs"
else
    fail "close-project missed in-flight specs"
fi
# Refusal 2: the reflection is the artifact close exists to produce.
if printf '%s\n' "$cp_out" | grep -q "Project-Level Reflection is empty"; then
    pass "close-project refuses on an empty Project-Level Reflection"
else
    fail "close-project did not check the reflection"
fi
# Refusal 3: signals owned by this close must be dispositioned.
if printf '%s\n' "$cp_out" | grep -q "awaiting a project-close disposition"; then
    pass "close-project refuses on undisposed project-close signals"
else
    fail "close-project did not force signal disposition"
fi
# --dry-run must never write, even while refusing.
BRIEF_BEFORE=$(shasum < projects/PROJ-001-example-mvp/brief.md)
just close-project PROJ-001 --dry-run >/dev/null 2>&1 || true
if [ "$(shasum < projects/PROJ-001-example-mvp/brief.md)" = "$BRIEF_BEFORE" ]; then
    pass "close-project --dry-run writes nothing"
else
    fail "close-project --dry-run modified the brief"
fi
json_ok "close-project --json"  bash -c 'just close-project PROJ-001 --json 2>/dev/null || true'
if [ "$HAVE_PY3" = 1 ]; then
    # Capture first: close-project exits 1 while refusing, and under `pipefail`
    # that sinks the whole pipeline regardless of what python decides.
    cp_json=$(just close-project PROJ-001 --json 2>/dev/null || true)
    printf '%s' "$cp_json" | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert d["ok"] is False and len(d["refusals"])>=3, d' \
        && pass "close-project --json reports every refusal" || fail "close-project --json missing refusals: $cp_json"
fi

# closed_reason is LOAD-BEARING: 'abandoned' lifts the in-flight refusal,
# because in-flight specs contradict "shipped" but are expected in an
# abandonment. This is the interaction that makes the field more than a label.
CB=projects/PROJ-001-example-mvp/brief.md
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/^closed_reason: null.*$/closed_reason: abandoned/' "$CB"; else sed -i 's/^closed_reason: null.*$/closed_reason: abandoned/' "$CB"; fi
cp_ab=$(just close-project PROJ-001 2>&1 || true)
if printf '%s\n' "$cp_ab" | grep -qE "spec\(s\) still in flight"; then
    fail "closed_reason: abandoned did not lift the in-flight refusal"
else
    pass "closed_reason: abandoned lifts the in-flight refusal"
fi
# An unrecognized closed_reason is advisory — open set, never a gate.
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/^closed_reason: abandoned$/closed_reason: vaporized/' "$CB"; else sed -i 's/^closed_reason: abandoned$/closed_reason: vaporized/' "$CB"; fi
cr_out=$(just validate 2>&1) || fail "validate failed on an unrecognized closed_reason (must be advisory)"
if printf '%s\n' "$cr_out" | grep -q "unrecognized closed_reason"; then
    pass "validate warns (advisory) on an unrecognized closed_reason"
else
    fail "validate did not surface the closed_reason advisory: $cr_out"
fi
if [ "$(uname)" = "Darwin" ]; then sed -i '' 's/^closed_reason: vaporized$/closed_reason: null/' "$CB"; else sed -i 's/^closed_reason: vaporized$/closed_reason: null/' "$CB"; fi

# --- value_realized: predict, then SCORE ------------------------------------
# The template captured predictions and never recorded outcomes, so "was the
# idea any good?" was unanswerable across projects. The refusal is conditional
# on the ending claimed — same shape as the in-flight rule — so it never blocks
# an exploratory or abandoned close.
# A project with NO thesis must not be asked to score one.
cp_nothesis=$(just close-project PROJ-001 2>&1 || true)
if printf '%s\n' "$cp_nothesis" | grep -q "thesis_held is null"; then
    fail "close-project demanded a verdict on a project with no thesis"
else
    pass "close-project does not demand a verdict when there is no thesis"
fi
# Give it a thesis: now claiming 'shipped' without scoring must refuse.
awk '/^closed_reason:/ { print; print "value:"; print "  thesis: \"faster activation cuts month-2 churn\""; print "  success_signals: [\"activation < 5min\"]"; next } { print }' "$CB" > "$CB.tmp" && mv "$CB.tmp" "$CB"
cp_unscored=$(just close-project PROJ-001 2>&1 || true)
if printf '%s\n' "$cp_unscored" | grep -q "thesis_held is null"; then
    pass "close-project refuses a claimed-shipped project with an unscored thesis"
else
    fail "close-project let an unscored thesis through: $cp_unscored"
fi
# 'abandoned' lifts it — you can't be asked to score a thesis you stopped testing.
awk '/^closed_reason: null/ { print "closed_reason: abandoned"; next } { print }' "$CB" > "$CB.tmp" && mv "$CB.tmp" "$CB"
cp_ab2=$(just close-project PROJ-001 2>&1 || true)
if printf '%s\n' "$cp_ab2" | grep -q "thesis_held is null"; then
    fail "closed_reason: abandoned did not lift the thesis-scoring refusal"
else
    pass "closed_reason: abandoned lifts the thesis-scoring refusal"
fi
# Answering it clears the refusal, and 'too-early' is a legal answer.
awk '/^closed_reason: abandoned/ { print "closed_reason: null"; next }
     /^  success_signals:/ { print; print "value_realized:"; print "  thesis_held: too-early"; print "  evidence: \"only 3 weeks of usage\""; next } { print }' "$CB" > "$CB.tmp" && mv "$CB.tmp" "$CB"
cp_scored=$(just close-project PROJ-001 2>&1 || true)
if printf '%s\n' "$cp_scored" | grep -q "thesis_held is null"; then
    fail "answering thesis_held did not clear the refusal"
else
    pass "thesis_held: too-early is a legal answer and clears the refusal"
fi
if [ "$HAVE_PY3" = 1 ]; then
    cp_j=$(just close-project PROJ-001 --json 2>/dev/null || true)
    printf '%s' "$cp_j" | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert d["thesis_held"]=="too-early", d' \
        && pass "close-project --json carries thesis_held" || fail "thesis_held missing from --json"
fi
git checkout -- "$CB" 2>/dev/null || true

# ============================================================
# Stage carries an orchestration_cost slot (the spend with no spec to attach to)
# ============================================================
# A stage scaffolded FROM the template (the shipped example stage is
# hand-authored and is asserted separately below).
just new-stage "Orchestration Cost Probe" >/dev/null 2>&1
SFILE=$(ls projects/PROJ-001-example-mvp/stages/STAGE-*-orchestration-cost-probe.md | head -n1)
assert_contains "$SFILE" "orchestration_cost:" "stage template carries orchestration_cost"
assert_contains "$SFILE" "THE ORCHESTRATOR FILLS THIS" "orchestration_cost says who fills it"

# The slot must have a READER. It shipped without one, which is the
# reserved-but-unwired pattern (harvest signal #7): a field nothing reads is a
# field nobody fills, and the framing spend stays invisible.
if [ "$HAVE_PY3" = 1 ]; then
    just roadmap --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert all("orchestration_tokens" in s for s in d["stages"]), d' \
        && pass "roadmap --json exposes orchestration_tokens per stage" || fail "roadmap --json missing orchestration_tokens"
fi
# An empty slot reads as 0, not as an error — a null here is the honest state.
if [ "$HAVE_PY3" = 1 ]; then
    just roadmap --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; assert all(s["orchestration_tokens"]==0 for s in d["stages"]), d' \
        && pass "an unfilled orchestration_cost sums to 0" || fail "unfilled orchestration_cost did not sum to 0"
fi
# Snapshot the spec cost gate BEFORE populating orchestration spend. The gate is
# not necessarily green here (other fixtures own that), so the meaningful
# assertion is that orchestration does not CHANGE its verdict — it is
# stage-grained and deliberately outside per-spec enforcement.
ca_before=$(just cost-audit 2>&1 || true)

# Record real framing spend and confirm it is summed and surfaced.
if [ "$(uname)" = "Darwin" ]; then
    sed -i '' 's|^  sessions: \[\]                      # - tokens_total: N$|  sessions:\n    - tokens_total: 41000\n      estimated_usd: 0.27|' "$SFILE"
else
    sed -i 's|^  sessions: \[\]                      # - tokens_total: N$|  sessions:\n    - tokens_total: 41000\n      estimated_usd: 0.27|' "$SFILE"
fi
if [ "$HAVE_PY3" = 1 ]; then
    just roadmap --json 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin)["data"]; t=[s["orchestration_tokens"] for s in d["stages"]]; assert 41000 in t, t' \
        && pass "roadmap --json sums recorded orchestration tokens" || fail "orchestration tokens not summed"
fi
rm_out=$(just roadmap 2>&1)
if printf '%s\n' "$rm_out" | grep -q "41000 orch tok"; then
    pass "roadmap surfaces orchestration spend on the stage row"
else
    fail "roadmap did not surface orchestration spend: $rm_out"
fi
# It must NOT leak into the spec cost gate.
ca_after=$(just cost-audit 2>&1 || true)
if [ "$ca_before" = "$ca_after" ]; then
    pass "cost-audit verdict unchanged by stage orchestration_cost"
else
    fail "orchestration_cost leaked into the per-spec cost gate"
fi
if just validate >/dev/null 2>&1; then
    pass "validate unaffected by the stage orchestration_cost slot"
else
    fail "validate broke after adding orchestration_cost to stages"
fi
# The shipped EXAMPLE stage must stay in sync with the template it demonstrates —
# a stale example teaches the wrong shape on day one.
EXSTAGE="projects/PROJ-001-example-mvp/stages/STAGE-001-foundational-infra.md"
assert_contains "$EXSTAGE" "orchestration_cost:" "example stage stays in sync with the template"
rm -f "$SFILE"

# ============================================================
# claude-plus-agents variant — handoff / handback (v0.6.28, DEC-005)
#
# Until now the suite ONLY ever scaffolded claude-only, so the entire
# plus-agents variant — handoffs included — shipped with zero coverage. This
# section scaffolds the OTHER variant in its own scratch repo and exercises the
# delegated-cost path, which is the whole reason a non-Claude build/verify agent
# can satisfy the cost gate at all.
# ============================================================
PA_SCRATCH=$(mktemp -d 2>/dev/null || mktemp -d -t 'template-plus-agents-test')
cp -R "$TEMPLATE_ROOT" "$PA_SCRATCH/repo"
rm -rf "$PA_SCRATCH/repo/.git"
# NOT a subshell: pass/fail counters live in this process, and a `( … )` would
# silently discard every pass recorded inside it (the suite would under-report
# its own coverage). cd back to the primary scratch when done.
cd "$PA_SCRATCH/repo"
printf "2\n" | just init >/dev/null 2>&1 || fail "just init (claude-plus-agents) exited non-zero"
if [ "$(cat .variant 2>/dev/null)" = "claude-plus-agents" ]; then
    pass "init scaffolds the claude-plus-agents variant"
else
    fail "init did not select claude-plus-agents (got '$(cat .variant 2>/dev/null)')"
fi
assert_file "projects/_templates/handoff.md"

# The variant's own gates must pass on a fresh scaffold.
if just validate >/dev/null 2>&1; then
    pass "plus-agents: validate passes on a fresh scaffold"
else
    fail "plus-agents: validate failed on a fresh scaffold"
fi
if just status >/dev/null 2>&1; then
    pass "plus-agents: status runs on a fresh scaffold"
else
    fail "plus-agents: status failed on a fresh scaffold"
fi

just new-stage "Delegated Infra" >/dev/null 2>&1
PA_STAGE=$(ls projects/PROJ-001-example-mvp/stages/STAGE-*-delegated-infra.md | head -n1)
PA_STAGE_ID=$(basename "$PA_STAGE" | grep -oE 'STAGE-[0-9]+')
just new-spec "Delegated Widget" "$PA_STAGE_ID" >/dev/null 2>&1
PA_SPEC=$(ls projects/PROJ-001-example-mvp/specs/SPEC-*-delegated-widget.md | grep -v timeline | head -n1)
PA_SPEC_ID=$(basename "$PA_SPEC" | grep -oE 'SPEC-[0-9]+')

# ONE handoff per delegated CYCLE — a build/verify split gets one each.
just new-handoff "$PA_SPEC_ID" build >/dev/null 2>&1
HB=$(ls projects/PROJ-001-example-mvp/handoffs/HANDOFF-*-build-*.md 2>/dev/null | head -n1)
if [ -n "$HB" ]; then
    pass "new-handoff build scaffolds a build handoff"
else
    fail "new-handoff build produced no file"
fi
assert_contains "$HB" "cycle: build"        "build handoff records cycle: build"
assert_contains "$HB" "to_role: implementer" "build handoff sets to_role: implementer"
assert_contains "$HB" "handback:"            "handoff carries the handback block"
if grep -q '__[A-Z_]*__' "$HB"; then
    fail "new-handoff left an unsubstituted placeholder: $(grep -o '__[A-Z_]*__' "$HB" | sort -u | tr '\n' ' ')"
else
    pass "new-handoff substitutes every __TOKEN__ placeholder"
fi
# to_agent comes from tier_map.<cycle> (DEC-005) — build and verify differ by default.
if grep -q "to_agent: claude-sonnet-4-6" "$HB"; then
    pass "build handoff picks to_agent from tier_map.build"
else
    fail "build handoff did not use tier_map.build"
fi

just new-handoff "$PA_SPEC_ID" verify >/dev/null 2>&1
HV=$(ls projects/PROJ-001-example-mvp/handoffs/HANDOFF-*-verify-*.md 2>/dev/null | head -n1)
assert_contains "$HV" "cycle: verify"     "verify handoff records cycle: verify"
assert_contains "$HV" "to_role: verifier" "verify handoff sets to_role: verifier"
if grep -q "to_agent: claude-opus-4-7" "$HV"; then
    pass "verify handoff picks to_agent from tier_map.verify (different from build)"
else
    fail "verify handoff did not use tier_map.verify"
fi
assert_cmd_fails "new-handoff rejects a non-delegated cycle" just new-handoff "$PA_SPEC_ID" design

# --- handback-sync: the cost path -------------------------------------------
# Nothing handed back yet → must FAIL and name the offender, not silently pass.
assert_cmd_fails "handback-sync fails while handoffs are un-returned" just handback-sync "$PA_SPEC_ID"
hs=$(just handback-sync "$PA_SPEC_ID" 2>&1 || true)
if printf '%s\n' "$hs" | grep -q "has not reported\|not set"; then
    pass "handback-sync explains WHICH handoff is outstanding"
else
    fail "handback-sync gave no actionable reason: $hs"
fi

# Agent reports back WITHOUT a token count while the gate is enforced → still fails.
sed_pa() { if [ "$(uname)" = "Darwin" ]; then sed -i '' "$@"; else sed -i "$@"; fi; }
sed_pa 's/^  status: null.*/  status: completed/' "$HB"
assert_cmd_fails "handback-sync refuses a handback with no tokens_total" just handback-sync "$PA_SPEC_ID"

# With a real self-reported count it transcribes into the spec's cost.sessions.
sed_pa 's/^  tokens_total: null.*/  tokens_total: 48211/' "$HB"
sed_pa 's/^  estimated_usd: null.*/  estimated_usd: 0.32/' "$HB"
sed_pa 's/^  status: null.*/  status: completed/' "$HV"
sed_pa 's/^  tokens_total: null.*/  tokens_total: 12044/' "$HV"
sed_pa 's/^  estimated_usd: null.*/  estimated_usd: 0.08/' "$HV"
if just handback-sync "$PA_SPEC_ID" >/dev/null 2>&1; then
    pass "handback-sync transcribes reported cost once both handbacks are filled"
else
    fail "handback-sync failed with complete handbacks"
fi
assert_contains "$PA_SPEC" "48211" "build cost landed in the spec's cost.sessions"
assert_contains "$PA_SPEC" "12044" "verify cost landed in the spec's cost.sessions"
assert_contains "$PA_SPEC" "^ +- cycle: build$"  "transcribed session carries cycle: build"
assert_contains "$PA_SPEC" "^ +- cycle: verify$" "transcribed session carries cycle: verify"
assert_contains "$HB" "synced_at: 2"         "handback-sync stamps synced_at"

# Idempotence: re-running must not double-count (the cost record's integrity).
just handback-sync "$PA_SPEC_ID" >/dev/null 2>&1 || true
occurrences=$(grep -c "48211" "$PA_SPEC" || true)
if [ "$occurrences" = "1" ]; then
    pass "handback-sync is idempotent (no double-counting on re-run)"
else
    fail "handback-sync double-counted: 48211 appears ${occurrences}× in the spec"
fi

# The transcribed cost must satisfy the real gate — the entire point of the path.
just advance-cycle "$PA_SPEC_ID" ship >/dev/null 2>&1
if just cost-audit >/dev/null 2>&1; then
    pass "cost-audit passes on a shipped spec costed purely from handbacks"
else
    fail "cost-audit still fails after handback-sync — the delegated-cost path is broken"
fi
cd "$SCRATCH/repo"
rm -rf "$PA_SCRATCH"

# ============================================================
# Done
# ============================================================
echo ""
echo "${GREEN}PASS${RESET}  ${pass_count} checks"
echo "${DIM}(scratch dir removed: ${SCRATCH})${RESET}"
rm -rf "$SCRATCH"
exit 0
