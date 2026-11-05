#!/usr/bin/env bash
# scripts/gate-audit.sh — a gate OVER the gates (SPEC-023).
#
# The mute-death shape: a gate script that reports green (or dies) WITHOUT its
# check actually running. Every gate here is `set -euo pipefail` shell, and in
# that context the shape recurs three ways (AGENTS.md §16 rule 3,
# `a-gate-that-fails-mutely-is-a-gate-that-never-ran`):
#
#   SHAPE 1 — a process / command substitution feeding a gate's verdict throws
#             away the exit status of a VERDICT ORACLE (a helper whose non-zero
#             exit IS the verdict, e.g. `front_matter_parse_report`). `set -e`
#             cannot see the exit of a `$(…)` / `< <(…)` producer, so a crash
#             inside it leaves the gate green. The clean idiom captures the rc
#             from a temp file (validate.sh after SPEC-023; `validate-red-proof`
#             lines 69-71).
#   SHAPE 2 — a `grep` whose result a gate depends on, inside a command
#             substitution, is UNGUARDED (`x=$(… grep …)` with no `|| true`). A
#             zero-match returns exit 1, which under set -e/pipefail aborts the
#             gate rather than meaning "zero matches". The clean idiom guards it
#             (`n=$(… | grep -c PAT || true); n=${n:-0}`, ready.sh:77-79).
#   SHAPE 3 — a gate emits a `success` / green verdict without ever asserting a
#             count — §16 rule 2: tooling that matches must "assert how many
#             times it matched". A gate that never compares a count can declare
#             all-clear over an empty scan (a glob that matched nothing).
#
# ⚠ The audit is itself source-matching tooling, so (AGENTS §16 rules 2-3) it
#   MUST (a) exclude comments — it must not fire on validate.sh's own prose that
#   DESCRIBES the shape — and (b) assert its own scan count: a glob that matched
#   nothing, or a file it silently skipped, is a loud failure, not a green pass.
#   A `gate-audit` that silently matches nothing is the very defect it hunts. The
#   Ruby engine below blanks comments, heredoc bodies and single-quoted spans
#   (embedded awk/ruby programs) before it matches, and reports EXAMINED so the
#   harness can assert it saw every gate it was handed.
#
# shellcheck is NOT the engine — SC2143/2181/2015 do not see this SEMANTIC class
# (a swallowed oracle exit, a verdict with no count). A complementary shellcheck
# job is a reasonable second net but optional (SPEC-023 Non-Goals).
#
# Usage: gate-audit.sh [--json]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

SCRIPTS_DIR="${REPO_ROOT}/scripts"

# ── The subject set ──────────────────────────────────────────────────────────
# The audit's subject is the VERDICT-BEARING gate scripts — those that emit a
# pass/fail a gate relies on. A script is a gate by DEFAULT (fail-closed: a new
# script is audited until someone classifies it), and the EXCLUDED set below
# names what emits state rather than a verdict, plus the fixtures that embed the
# shape on purpose. Keep this list honest: do not add a gate here to quiet a
# finding — fix the finding.
#
#   *-view.sh / dash / status / *-report / report_* / roadmap / backlog /
#   calibration / release-notes / specs-by-stage / weekly-review / instances-diff
#       — emit state (lists, dashboards, reports), not a verdict (SPEC-023 subject
#         set; the spec names *-view/dash/status/new-* explicitly).
#   new-* / frame-stage / claim / close-project / advance-cycle / archive-* /
#   scaffold-clean / fresh-history / info / build-info / next-version /
#   template-version
#       — scaffolders and workflow mutators; they act, they do not gate.
#   *-red-proof.sh and gate-audit.sh itself
#       — a red-proof CONSTRUCTS the mute-death shape in a disposable copy as its
#         fixture, and gate-audit carries the shape patterns as detection data;
#         scanning either flags the bait, not a bug. gate-audit's own honesty is
#         proven by gate-audit-red-proof.sh, not by self-scan.
is_excluded() {
    case "$1" in
        *-view.sh|dash.sh|status.sh|*-report.sh|report_*.sh|roadmap.sh|backlog.sh|\
        calibration.sh|release-notes.sh|specs-by-stage.sh|weekly-review.sh|\
        instances-diff.sh) return 0 ;;
        new-*.sh|frame-stage.sh|claim.sh|close-project.sh|advance-cycle.sh|\
        archive-*.sh|scaffold-clean.sh|fresh-history.sh|info.sh|build-info.sh|\
        next-version.sh|template-version.sh) return 0 ;;
        *-red-proof.sh|gate-audit.sh) return 0 ;;
        *) return 1 ;;
    esac
}

GATES=(); EXCLUDED=()
while IFS= read -r s; do
    [ -n "$s" ] || continue
    base=$(basename "$s")
    if is_excluded "$base"; then EXCLUDED+=("$s"); else GATES+=("$s"); fi
done < <(find "$SCRIPTS_DIR" -maxdepth 1 -type f -name '*.sh' 2>/dev/null | sort)

n_all=$(( ${#GATES[@]} + ${#EXCLUDED[@]} ))

# ── Scan-count assertion, part 1 (§16 rule 2) ────────────────────────────────
# A glob that matched nothing is a loud failure. And the four anchor gates MUST
# be in the subject set — if a mis-glob or a cwd slip drops them, the audit would
# "pass" having checked nothing it exists to check.
[ "${#GATES[@]}" -gt 0 ] \
    || die "gate-audit: the scripts/*.sh glob produced ZERO gate scripts (cwd='${REPO_ROOT}'). A scan over nothing is the mute-death shape this gate hunts — refusing to report green."
for anchor in validate.sh cost-audit.sh decisions-audit.sh handback-sync.sh; do
    found=0
    for g in "${GATES[@]}"; do [ "$(basename "$g")" = "$anchor" ] && found=1 && break; done
    [ "$found" = 1 ] \
        || die "gate-audit: anchor gate '${anchor}' is not in the subject set — the glob or the exclusion list is wrong (cwd='${REPO_ROOT}'). Refusing to report green on a scan that lost a known gate."
done

# ── The engine ───────────────────────────────────────────────────────────────
# Ruby (present on macOS and the GitHub runners; the parse gate already depends
# on it, SPEC-016). It reads each gate path from ARGV, blanks comments / heredoc
# bodies / single-quoted spans, and emits TSV:
#     FINDING<TAB><file><TAB><line><TAB><shape><TAB><detail>
#     EXAMINED<TAB><n>                 — how many gate files it actually read
# Exit 1 if any finding, else 0. Captured by the harness via a temp file so the
# engine's own rc is never swallowed (the shape this gate exists to catch).
report="$(mktemp)"
set +e
ruby -- - "${GATES[@]}" > "$report" <<'RUBY'
# encoding: utf-8
# Mute-death scanner. Input: gate script paths on ARGV. Output: FINDING / EXAMINED.
# The gate scripts carry UTF-8 (✓, §, ⚠, →), so decode as UTF-8 rather than the
# runner's default (US-ASCII when LANG is unset), or every regex op raises.
Encoding.default_external = Encoding::UTF_8
Encoding.default_internal = Encoding::UTF_8

VERDICT_ORACLE = /(parse|verify|validate|check|assert|oracle|audit)/i
# Accessor prefixes: a helper whose job is to EMIT data, not to signal a verdict
# by exit code. Its exit through a substitution is not a swallowed verdict.
ACCESSOR = /\A(get|find|emit|list|has|is|fm|json|resolve|sum|load|read|print|echo|classify)_?/

# Strip comments / heredoc bodies / single-quoted spans from one physical line,
# given the running quote + heredoc state. Returns the "code view" of the line
# (single-quoted text blanked to spaces so positions are preserved; double-quoted
# text KEPT, because a `$(…)` inside double quotes is live code). State is a Hash
# carried across lines: :sq (in single quote), :dq (in double quote), :heredoc
# (delimiter string or nil).
def code_view(line, st)
  # Inside a heredoc body: the line is data until a line that is exactly the
  # delimiter (optionally indented, for <<-).
  if st[:heredoc]
    if line.strip == st[:heredoc]
      st[:heredoc] = nil
    end
    return ""
  end
  out = +""
  i = 0
  n = line.length
  while i < n
    c = line[i]
    if st[:sq]
      if c == "'"
        st[:sq] = false
      else
        out << " "   # blank single-quoted (literal) text, preserve width
      end
      i += 1
      next
    end
    if st[:dq]
      out << c
      if c == "\\" && i + 1 < n
        out << line[i + 1]; i += 2; next
      end
      st[:dq] = false if c == '"'
      i += 1
      next
    end
    # not in any quote
    if c == "\\" && i + 1 < n
      out << c << line[i + 1]; i += 2; next
    end
    if c == "'"
      st[:sq] = true; out << " "; i += 1; next
    end
    if c == '"'
      st[:dq] = true; out << c; i += 1; next
    end
    # an unquoted `#` at the start of a token (start of line or after whitespace)
    # begins a comment — but `${#arr}` and `$#` are NOT comments.
    if c == "#" && (i == 0 || line[i - 1] =~ /\s/)
      break
    end
    out << c
    i += 1
  end
  # A heredoc opener on this (code) line: `<<` / `<<-` then an optional quote and
  # a word. Record the delimiter so following lines are treated as data.
  if out =~ /<<-?\s*["']?([A-Za-z_][A-Za-z0-9_]*)["']?/
    st[:heredoc] = $1
  end
  out
end

# Find the body of the FIRST command/process substitution that starts at or
# after `from` in `s`, honouring nested `$(` and quotes. Returns [body, open_idx]
# or nil. Skips `$((` arithmetic.
def first_subst(s, from)
  i = from
  n = s.length
  while i < n - 1
    two = s[i, 2]
    if two == "$(" && s[i + 2] != "("      # command sub, not $(( arithmetic
      start = i + 2
    elsif two == "<(" || (s[i] == "<" && s[i + 1] == " " && s[i + 2, 1] == "<")
      # `<(` or `< <(` process substitution
      j = s.index("<(", i)
      return nil unless j
      start = j + 2
      i = j
    else
      i += 1
      next
    end
    # scan to the matching close paren from `start`
    depth = 1
    k = start
    sq = false; dq = false
    while k < n
      ch = s[k]
      if sq
        sq = false if ch == "'"
      elsif dq
        dq = false if ch == '"'
      elsif ch == "'"
        sq = true
      elsif ch == '"'
        dq = true
      elsif ch == "("
        depth += 1
      elsif ch == ")"
        depth -= 1
        return [s[start...k], i] if depth.zero?
      end
      k += 1
    end
    return nil   # unbalanced on this logical line
  end
  nil
end

def scan_file(path, raw)
  st = { sq: false, dq: false, heredoc: nil }
  lines = raw.lines
  # Build logical lines (join `\`-continuations) with the ORIGINAL start line no.
  logical = []
  buf = +""; buf_line = nil
  lines.each_with_index do |ln, idx|
    cv = code_view(ln.chomp, st)
    buf_line ||= idx + 1
    if cv =~ /\\\z/
      buf << cv[0..-2] << " "
    else
      buf << cv
      logical << [buf_line, buf]
      buf = +""; buf_line = nil
    end
  end
  logical << [buf_line, buf] unless buf.empty?

  file_has_success = false
  first_success_line = nil
  file_has_count_assert = false

  logical.each do |lineno, code|
    # SHAPE 1 — a substitution whose producer is a verdict oracle.
    code.scan(/(?:\$\(|<\s*<\()\s*([A-Za-z_][A-Za-z0-9_]*)/) do |m|
      name = m[0]
      next if name =~ ACCESSOR
      if name =~ VERDICT_ORACLE
        puts "FINDING\t#{path}\t#{lineno}\tswallowed-oracle-exit\t" \
             "a substitution feeds the gate the output of verdict oracle '#{name}' but discards its exit status (set -e cannot see a $()/<() producer's rc); capture the rc from a temp file instead"
      end
    end

    # SHAPE 2 — an unguarded grep inside a command/process substitution.
    off = 0
    while (res = first_subst(code, off))
      body, open_idx = res
      if body =~ /\bgrep\b/ && body !~ /\|\|\s*(?:true|:)\b/
        puts "FINDING\t#{path}\t#{lineno}\tunguarded-grep\t" \
             "a grep inside a command substitution is unguarded; a zero-match returns 1 and aborts the gate under set -e/pipefail — guard it with `|| true`"
      end
      off = open_idx + 2
    end

    # SHAPE 3 bookkeeping — does the file emit a verdict, and does it ever assert
    # a count? `success` call or a literal green check mark.
    if code =~ /(?:^|[;&|]|\bthen\b|\bdo\b)\s*success\b/ || code =~ /✓/
      file_has_success = true
      first_success_line ||= lineno
    end
    if code =~ /-(?:gt|eq|ne|lt|ge|le)\b/ || code =~ /\$\{#[A-Za-z_]/ || code =~ /\(\([^)]*[<>=][^)]*\)\)/
      file_has_count_assert = true
    end
  end

  # SHAPE 3 — a verdict with no count asserted anywhere in the file.
  if file_has_success && !file_has_count_assert
    puts "FINDING\t#{path}\t#{first_success_line}\tunasserted-count\t" \
         "the gate emits a success/green verdict but never asserts a count (no -gt/-eq/${#...}); §16 rule 2 — a gate must assert how many times its check matched, so a scan over nothing cannot pass as green"
  end
end

paths = ARGV.dup
examined = 0
paths.each do |path|
  examined += 1
  begin
    raw = File.read(path, encoding: "UTF-8")
    scan_file(path, raw)
  rescue => e
    # A gate we cannot read or decode is a scan that did not happen — report it
    # as a finding, never skip it silently (that is the mute-death shape itself).
    puts "FINDING\t#{path}\t0\tunscannable\t#{e.class}: #{e.message}"
  end
end
puts "EXAMINED\t#{examined}"
RUBY
engine_rc=$?
set -e

# ── Scan-count assertion, part 2 ─────────────────────────────────────────────
# The engine must have EXAMINED exactly the gates it was handed. A short count
# means it skipped one (unreadable, or it crashed mid-run) — refuse to report
# green on a partial scan.
examined=$(awk -F'\t' '$1=="EXAMINED"{print $2}' "$report")
examined=${examined:-0}
if [ "$examined" -ne "${#GATES[@]}" ]; then
    rm -f "$report"
    die "gate-audit: the scanner examined ${examined} of ${#GATES[@]} gate script(s) — it skipped or crashed on at least one. Refusing to report green on a partial scan (§16 rule 2). engine rc=${engine_rc}."
fi
if [ "$engine_rc" -gt 1 ]; then
    rm -f "$report"
    die "gate-audit: the scanner exited ${engine_rc} (an engine crash, not a finding). Refusing to report green on a check that did not run."
fi

# ── Collect findings ─────────────────────────────────────────────────────────
f_files=(); f_lines=(); f_shapes=(); f_details=()
while IFS=$'\t' read -r tag file line shape detail; do
    [ "$tag" = "FINDING" ] || continue
    f_files+=("$file"); f_lines+=("$line"); f_shapes+=("$shape"); f_details+=("$detail")
done < "$report"
rm -f "$report"
offenders=${#f_files[@]}

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "$offenders" ]; do
        rel="${f_files[$i]#"${REPO_ROOT}"/}"
        items+=("$(json_obj \
            file "$(json_qs "$rel")" \
            line "${f_lines[$i]}" \
            shape "$(json_qs "${f_shapes[$i]}")" \
            detail "$(json_qs "${f_details[$i]}")")")
        i=$((i + 1))
    done
    [ "$offenders" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit gate-audit "$(json_obj \
        examined "$examined" \
        offenders "$offenders" \
        ok "$([ "$offenders" -eq 0 ] && echo true || echo false)" \
        violations "$arr")"
    [ "$offenders" -gt 0 ] && exit 1
    exit 0
fi

if [ "$offenders" -gt 0 ]; then
    echo "${BOLD}gate-audit: the mute-death shape in ${offenders} place(s) across ${examined} gate script(s):${RESET}"
    echo
    i=0
    while [ "$i" -lt "$offenders" ]; do
        rel="${f_files[$i]#"${REPO_ROOT}"/}"
        printf "  %s:%s  [%s]\n      %s\n" \
            "$rel" "${f_lines[$i]}" "${f_shapes[$i]}" "${f_details[$i]}"
        i=$((i + 1))
    done
    echo
    die "gate-audit: ${offenders} mute-death finding(s). A gate that reports green without its check running is a gate that never ran (AGENTS.md §16 rule 3, signal a-gate-that-fails-mutely-is-a-gate-that-never-ran)."
fi

success "gate-audit: ${examined} verdict-bearing gate script(s) scanned, no mute-death shape (swallowed oracle exit / unguarded grep / unasserted count)."
