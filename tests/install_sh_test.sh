#!/usr/bin/env bash
# Tests for the install.sh helper functions (noexec temp dir + sudo detection).
#
# Run: bash tests/install_sh_test.sh
#
# Loads install.sh with JOSHIFY_INSTALL_LIB_ONLY=1 so only the helpers are
# defined and no installation is attempted.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAILURES=0

# shellcheck source=/dev/null
JOSHIFY_INSTALL_LIB_ONLY=1 . "$SCRIPT_DIR/install.sh"

ok()   { printf '  ok   %s\n' "$1"; }
fail() { printf '  FAIL %s\n' "$1"; FAILURES=$((FAILURES + 1)); }

assert_eq() { # expected actual name
    if [ "$1" = "$2" ]; then ok "$3"; else fail "$3 (expected '$1', got '$2')"; fi
}

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

echo "can_exec_in"
# ---------------------------------------------------------------------------
if can_exec_in "$WORK"; then ok "accepts a writable exec dir"; else fail "accepts a writable exec dir"; fi

if can_exec_in "$WORK/does-not-exist"; then fail "rejects a missing dir"; else ok "rejects a missing dir"; fi

RO="$WORK/readonly"
mkdir -p "$RO" && chmod 500 "$RO"
if can_exec_in "$RO"; then fail "rejects a non-writable dir"; else ok "rejects a non-writable dir"; fi
chmod 700 "$RO"

# Simulate noexec without needing mount privileges: shadow chmod so the probe
# never becomes executable, which is what a noexec mount looks like to us.
noexec_sim() {
    chmod() { command chmod "$@" 2>/dev/null; command chmod -x "${!#}" 2>/dev/null; return 0; }
    can_exec_in "$1"
    local rc=$?
    unset -f chmod
    return $rc
}
if noexec_sim "$WORK"; then fail "rejects a dir where the probe cannot execute"; else ok "rejects a dir where the probe cannot execute"; fi

if [ -n "$(find "$WORK" -maxdepth 1 -name '.joshify-exec-probe.*' -print -quit)" ]; then
    fail "cleans up its probe file"
else
    ok "cleans up its probe file"
fi

echo "find_exec_tmpdir"
# ---------------------------------------------------------------------------
assert_eq "$WORK" "$(TMPDIR="$WORK" find_exec_tmpdir)" "respects a usable pre-set TMPDIR"

# An unusable TMPDIR must not be returned; with /tmp available it falls to /tmp,
# and with neither it must land under HOME.
BAD="$WORK/bad"; mkdir -p "$BAD"; chmod 500 "$BAD"
GOT=$(TMPDIR="$BAD" find_exec_tmpdir)
if [ "$GOT" != "$BAD" ] && [ -n "$GOT" ]; then
    ok "falls back off an unusable TMPDIR"
else
    fail "falls back off an unusable TMPDIR (got '$GOT')"
fi
chmod 700 "$BAD"

# Force both TMPDIR and /tmp to look unusable -> HOME fallback.
FAKE_HOME="$WORK/home"; mkdir -p "$FAKE_HOME"
can_exec_in() { case "$1" in "$FAKE_HOME"/*) return 0 ;; *) return 1 ;; esac; }
assert_eq "$FAKE_HOME/.cache/joshify/tmp" "$(HOME="$FAKE_HOME" TMPDIR="$BAD" find_exec_tmpdir)" \
    "falls back to a dir under HOME when nothing else can exec"
unset -f can_exec_in

echo "detect_sudo_mode"
# ---------------------------------------------------------------------------
id() { echo 0; }
assert_eq "none" "$(detect_sudo_mode)" "reports 'none' when running as root"
unset -f id

id() { echo 1000; }
command() { if [ "${2:-}" = "sudo" ]; then return 1; fi; builtin command "$@"; }
assert_eq "unavailable" "$(detect_sudo_mode)" "reports 'unavailable' when sudo is missing"
unset -f command

sudo() { [ "${1:-}" = "-n" ] && return 0; return 1; }
assert_eq "passwordless" "$(detect_sudo_mode)" "reports 'passwordless' when 'sudo -n' succeeds"

# Password required: with a TTY we can prompt, without one we cannot.
sudo() { return 1; }
have_tty() { return 0; }
assert_eq "tty" "$(detect_sudo_mode)" "reports 'tty' when a terminal is reachable"
have_tty() { return 1; }
assert_eq "unavailable" "$(detect_sudo_mode)" "reports 'unavailable' with no TTY and no passwordless sudo"
unset -f id sudo have_tty

echo "run_privileged"
# ---------------------------------------------------------------------------
SUDO_MODE="none"
assert_eq "hi" "$(run_privileged echo hi)" "runs the command directly as root"

SUDO_MODE="passwordless"
sudo() { echo "sudo:$*"; }
assert_eq "sudo:-n echo hi" "$(run_privileged echo hi)" "uses 'sudo -n' when passwordless"
unset -f sudo

# shellcheck disable=SC2034  # read by run_privileged
SUDO_MODE="unavailable"
if run_privileged echo hi > /dev/null 2>&1; then
    fail "fails instead of running when elevation is unavailable"
else
    ok "fails instead of running when elevation is unavailable"
fi

echo "package manager selection"
# ---------------------------------------------------------------------------
# Runs install.sh for real against a stub PATH and reports which manager it
# chose. Regression guard: on Linux the native manager must win over linuxbrew,
# which does not provide the -dev packages the build needs.
chosen_pkg_mgr() { # uname_output, available commands...
    local uname_out="$1"; shift
    local stub; stub=$(mktemp -d)

    printf '#!/bin/sh\necho "%s"\n' "$uname_out" > "$stub/uname"
    printf '#!/bin/sh\n[ "$1" = "--version" ] && echo "cargo 1.0.0"\nexit 0\n' > "$stub/cargo"
    printf '#!/bin/sh\nexit 0\n' > "$stub/git"
    for c in "$@"; do printf '#!/bin/sh\nexit 0\n' > "$stub/$c"; done
    chmod +x "$stub"/*

    # Real coreutils the installer needs, but no sudo and no package managers
    # beyond the ones requested.
    for c in bash mktemp rm chmod id printf find dirname; do
        [ -e "$stub/$c" ] || ln -sf "$(command -v "$c")" "$stub/$c" 2>/dev/null
    done

    env -i PATH="$stub" HOME="$WORK/pkgmgr-home" "$stub/bash" "$SCRIPT_DIR/install.sh" \
        < /dev/null 2>&1 | grep -oE 'Detected (Debian/Ubuntu|Fedora/RHEL|Arch|Homebrew)' | head -1
    rm -rf "$stub"
}
mkdir -p "$WORK/pkgmgr-home"

assert_eq "Detected Debian/Ubuntu" "$(chosen_pkg_mgr Linux sudo apt-get brew)" \
    "prefers apt over linuxbrew on Linux"
assert_eq "Detected Fedora/RHEL" "$(chosen_pkg_mgr Linux sudo dnf brew)" \
    "prefers dnf over linuxbrew on Linux"
assert_eq "Detected Arch" "$(chosen_pkg_mgr Linux sudo pacman brew)" \
    "prefers pacman over linuxbrew on Linux"
assert_eq "Detected Homebrew" "$(chosen_pkg_mgr Darwin brew)" \
    "uses Homebrew on macOS"
assert_eq "Detected Homebrew" "$(chosen_pkg_mgr Linux brew)" \
    "falls back to brew on Linux when no native manager exists"

echo "install.sh guards"
# ---------------------------------------------------------------------------
if grep -q 'DEBIAN_FRONTEND=noninteractive' "$SCRIPT_DIR/install.sh"; then
    ok "apt install is non-interactive"
else
    fail "apt install is non-interactive"
fi

echo ""
if [ "$FAILURES" -eq 0 ]; then
    echo "All install.sh tests passed."
else
    echo "$FAILURES install.sh test(s) failed."
    exit 1
fi
