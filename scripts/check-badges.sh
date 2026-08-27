#!/usr/bin/env bash
# Verify every badge and media reference in README.md.
#
# Usage:
#   scripts/check-badges.sh                     # check badges + local media
#   scripts/check-badges.sh --expect-release v1.2.3
#                                               # additionally assert the release
#                                               # badge has caught up to this tag
#
# Exits non-zero if a badge endpoint is broken, a badge reports a failing
# workflow, or a referenced local file is missing.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
README="$REPO_ROOT/README.md"
EXPECT_RELEASE=""
FAILURES=0

while [ $# -gt 0 ]; do
    case "$1" in
        --expect-release) EXPECT_RELEASE="${2:-}"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

ok()   { printf '  ok   %s\n' "$1"; }
fail() { printf '  FAIL %s\n' "$1"; FAILURES=$((FAILURES + 1)); }

# shields.io renders the badge text into the SVG <title>, e.g. "CI: passing".
badge_title() {
    curl -sS --max-time 20 --retry 3 --retry-delay 2 "$1" 2>/dev/null \
        | grep -oE '<title>[^<]*</title>' | head -1 | sed -e 's/<[^>]*>//g'
}

echo "Badges"
# ---------------------------------------------------------------------------
BADGES=$(grep -oE 'src="https://img\.shields\.io/[^"]+"' "$README" | sed -e 's/^src="//' -e 's/"$//')

if [ -z "$BADGES" ]; then
    fail "no shields.io badges found in README.md"
fi

# A workflow must not grade its own status badge. While the run is in progress
# the badge reflects the previous run on the branch; once a run is cancelled
# or fails, the badge stays red until a later run passes - which it never can,
# because this check fails first. GITHUB_WORKFLOW_REF looks like
# "owner/repo/.github/workflows/ci.yml@refs/heads/main".
SELF_WORKFLOW_FILE=""
if [ -n "${GITHUB_WORKFLOW_REF:-}" ]; then
    SELF_WORKFLOW_FILE=$(printf '%s' "$GITHUB_WORKFLOW_REF" | sed -e 's/@.*//' -e 's|.*/||')
fi

while IFS= read -r url; do
    [ -n "$url" ] || continue
    if [ -n "$SELF_WORKFLOW_FILE" ]; then
        case "$url" in
            */actions/workflow/status/*"/$SELF_WORKFLOW_FILE?"*|*"/actions/workflow/status/"*"/$SELF_WORKFLOW_FILE")
                ok "skipped $SELF_WORKFLOW_FILE badge - a workflow cannot grade its own run"
                continue
                ;;
        esac
    fi
    # README embeds &amp;-free raw URLs; curl needs them as-is.
    title=$(badge_title "$url")

    if [ -z "$title" ]; then
        fail "badge endpoint returned nothing ($url)"
        continue
    fi

    # The for-the-badge style uppercases the rendered title, so match on a
    # lowercased copy.
    case "$(printf '%s' "$title" | tr '[:upper:]' '[:lower:]')" in
        # shields renders these when the target does not resolve.
        *invalid*|*"not found"*|*"no status"*|*inaccessible*)
            fail "$title — badge does not resolve ($url)"
            ;;
        *failing*|*error*)
            fail "$title — badge reports a broken workflow"
            ;;
        *)
            ok "$title"
            ;;
    esac
done <<< "$BADGES"

if [ -n "$EXPECT_RELEASE" ]; then
    echo "Release badge freshness"
    # ---------------------------------------------------------------------
    rel_url=$(grep -oE 'src="https://img\.shields\.io/github/v/release/[^"]+"' "$README" \
        | head -1 | sed -e 's/^src="//' -e 's/"$//')
    if [ -z "$rel_url" ]; then
        fail "README has no github/v/release badge to verify"
    else
        # The for-the-badge style uppercases the rendered title ("RELEASE:
        # V1.2.3"), so compare case-insensitively or this can never match.
        want=$(printf '%s' "$EXPECT_RELEASE" | tr '[:upper:]' '[:lower:]')

        # shields caches release lookups for several minutes; poll for up to
        # five before calling it stale.
        got=""
        for _ in $(seq 1 20); do
            got=$(badge_title "$rel_url")
            case "$(printf '%s' "$got" | tr '[:upper:]' '[:lower:]')" in
                *"$want"*) break ;;
            esac
            sleep 15
        done
        case "$(printf '%s' "$got" | tr '[:upper:]' '[:lower:]')" in
            *"$want"*) ok "release badge shows $EXPECT_RELEASE" ;;
            *) fail "release badge shows '$got', expected $EXPECT_RELEASE" ;;
        esac
    fi
fi

echo "Local media references"
# ---------------------------------------------------------------------------
MEDIA=$(grep -oE '(src|href)="[^"]+"' "$README" \
    | sed -e 's/^[a-z]*="//' -e 's/"$//' \
    | grep -vE '^(https?:|#|mailto:)' \
    | sort -u)

while IFS= read -r ref; do
    [ -n "$ref" ] || continue
    if [ -e "$REPO_ROOT/$ref" ]; then
        ok "$ref"
    else
        fail "$ref — referenced by README.md but missing from the repo"
    fi
done <<< "$MEDIA"

echo ""
if [ "$FAILURES" -eq 0 ]; then
    echo "All README badges and references are valid."
else
    echo "$FAILURES README badge/reference problem(s) found."
    exit 1
fi
