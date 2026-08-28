#!/usr/bin/env bash
# Tests for scripts/check-badges.sh: a red workflow badge is checked against
# GitHub before it fails the run.
#
# Run: bash tests/check_badges_test.sh
#
# The script's curl is replaced by a stub (CHECK_BADGES_FETCH) that answers
# from files in a temp dir and logs every URL it was asked for.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/check-badges.sh"
FAILURES=0

ok()   { printf '  ok   %s\n' "$1"; }
fail() { printf '  FAIL %s\n' "$1"; FAILURES=$((FAILURES + 1)); }

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

CI_BADGE='https://img.shields.io/github/actions/workflow/status/bigknoxy/joshify/ci.yml?branch=main&style=for-the-badge&label=CI'
cat > "$WORK/README.md" <<README
<img src="$CI_BADGE" alt="CI">
<img src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" alt="License">
README

# The stub: last argument is the URL. Badge SVGs come from $WORK/ci_badge and
# $WORK/license_badge; the GitHub API answer from $WORK/api.json (a missing
# file means the API was unreachable).
cat > "$WORK/fetch" <<'STUB'
#!/usr/bin/env bash
url="${!#}"
echo "$url" >> "$WORK/fetch.log"
case "$url" in
    *img.shields.io/github/actions/workflow/status/*) cat "$WORK/ci_badge" ;;
    *img.shields.io/badge/license*) cat "$WORK/license_badge" ;;
    *api.github.com/repos/bigknoxy/joshify/actions/workflows/ci.yml/runs*)
        [ -f "$WORK/api.json" ] && cat "$WORK/api.json" || exit 22 ;;
    *) exit 22 ;;
esac
STUB
chmod +x "$WORK/fetch"
printf '<svg><title>LICENSE: MIT</title></svg>' > "$WORK/license_badge"

run_check() { # -> stdout in $OUT, exit code in $RC
    : > "$WORK/fetch.log"
    OUT=$(WORK="$WORK" CHECK_BADGES_README="$WORK/README.md" CHECK_BADGES_FETCH="$WORK/fetch" \
        GH_TOKEN="" GITHUB_TOKEN="" GITHUB_WORKFLOW_REF="" bash "$SCRIPT" 2>&1)
    RC=$?
}

api_json() { # status conclusion [status conclusion ...] -> newest first
    local runs="" sep=""
    while [ $# -ge 2 ]; do
        runs="$runs$sep{\"status\":\"$1\",\"conclusion\":$2}"
        sep=","
        shift 2
    done
    printf '{"workflow_runs":[%s]}' "$runs" > "$WORK/api.json"
}

echo "a passing badge"
# ---------------------------------------------------------------------------
printf '<svg><title>CI: PASSING</title></svg>' > "$WORK/ci_badge"
rm -f "$WORK/api.json"
run_check
[ "$RC" -eq 0 ] && ok "passes" || fail "passes (rc=$RC): $OUT"
if grep -q api.github.com "$WORK/fetch.log"; then
    fail "does not consult GitHub when the badge is green"
else
    ok "does not consult GitHub when the badge is green"
fi

echo "a red badge while the workflow is still running"
# ---------------------------------------------------------------------------
# This is the race: CI's badge job read the Visual Tests badge as FAILING
# while that workflow was mid-run on the same push and seconds from passing.
printf '<svg><title>CI: FAILING</title></svg>' > "$WORK/ci_badge"
api_json in_progress null completed '"success"'
run_check
[ "$RC" -eq 0 ] && ok "is not a failure when the latest completed run passed" \
    || fail "is not a failure when the latest completed run passed (rc=$RC): $OUT"
case "$OUT" in
    *"stale badge"*) ok "says the badge is stale" ;;
    *) fail "says the badge is stale: $OUT" ;;
esac

echo "a red badge for a workflow that really failed"
# ---------------------------------------------------------------------------
api_json in_progress null completed '"failure"'
run_check
[ "$RC" -ne 0 ] && ok "fails when the latest completed run failed" \
    || fail "fails when the latest completed run failed: $OUT"
case "$OUT" in
    *"GitHub: failure"*) ok "names GitHub's conclusion" ;;
    *) fail "names GitHub's conclusion: $OUT" ;;
esac

api_json completed '"cancelled"'
run_check
[ "$RC" -ne 0 ] && ok "a cancelled latest run is not a pass" \
    || fail "a cancelled latest run is not a pass: $OUT"

echo "a red badge when GitHub cannot be asked"
# ---------------------------------------------------------------------------
# Unknown must not become a pass: the badge is the only evidence, and it is red.
rm -f "$WORK/api.json"
run_check
[ "$RC" -ne 0 ] && ok "trusts the red badge when the API is unreachable" \
    || fail "trusts the red badge when the API is unreachable: $OUT"
case "$OUT" in
    *"GitHub: unknown"*) ok "says it could not tell" ;;
    *) fail "says it could not tell: $OUT" ;;
esac

echo "a workflow with no completed run yet"
# ---------------------------------------------------------------------------
api_json in_progress null
run_check
[ "$RC" -eq 0 ] && ok "is not graded on a badge with nothing behind it" \
    || fail "is not graded on a badge with nothing behind it (rc=$RC): $OUT"

echo ""
if [ "$FAILURES" -eq 0 ]; then
    echo "All check-badges.sh tests passed."
else
    echo "$FAILURES check-badges.sh test(s) failed."
    exit 1
fi
