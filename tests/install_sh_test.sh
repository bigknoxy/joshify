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

# install.sh sets -euo pipefail, which leaks into this shell when sourced.
# These tests deliberately call functions that return non-zero, so turn -e
# back off; assertions check return codes explicitly.
set +e

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

echo "release_asset_name"
# ---------------------------------------------------------------------------
# Note: release_asset_name declares `local os arch`, and bash uses dynamic
# scoping — so the stub must NOT use those names or it reads the callee's
# empty locals instead of ours.
asset_for() { # os arch -> the release asset name for that platform
    _TEST_OS="$1"; _TEST_ARCH="$2"
    uname() { case "${1:-}" in -s) echo "$_TEST_OS" ;; -m) echo "$_TEST_ARCH" ;; esac; }
    release_asset_name
    unset -f uname
}

assert_eq "joshify-linux-x86_64.tar.gz" "$(asset_for Linux x86_64)"  "maps Linux/x86_64 to the linux asset"
assert_eq "joshify-linux-x86_64.tar.gz" "$(asset_for Linux amd64)"   "accepts amd64 as an alias of x86_64"
assert_eq "joshify-macos-aarch64.tar.gz" "$(asset_for Darwin arm64)" "maps Darwin/arm64 to the macOS asset"
assert_eq "" "$(asset_for Linux aarch64)"  "has no prebuilt binary for Linux/aarch64 (issue #33)"
assert_eq "" "$(asset_for Darwin x86_64)"  "has no prebuilt binary for Intel macOS (issue #33)"
assert_eq "" "$(asset_for FreeBSD x86_64)" "has no prebuilt binary for unsupported platforms"

echo "normalize_version"
# ---------------------------------------------------------------------------
assert_eq "0.7.2" "$(normalize_version v0.7.2)" "strips a leading v from a tag"
assert_eq "0.7.2" "$(normalize_version 0.7.2)"  "leaves a bare version alone"

echo "installed_version"
# ---------------------------------------------------------------------------
VBIN="$WORK/bin"; mkdir -p "$VBIN"
printf '#!/bin/sh\necho "Joshify 0.7.2"\necho "A beautiful terminal Spotify client built with Rust."\n' > "$VBIN/joshify"
chmod +x "$VBIN/joshify"
assert_eq "0.7.2" "$(PATH="$VBIN:$PATH" installed_version joshify)" "parses the version out of --version output"
assert_eq "" "$(installed_version joshify-definitely-not-installed)" "reports nothing when not installed"

printf '#!/bin/sh\nexit 1\n' > "$VBIN/joshify-broken"; chmod +x "$VBIN/joshify-broken"
assert_eq "" "$(PATH="$VBIN:$PATH" installed_version joshify-broken)" "reports nothing when the binary fails to run"

echo "resolve_install_dir"
# ---------------------------------------------------------------------------
assert_eq "/opt/custom" "$(JOSHIFY_INSTALL_DIR=/opt/custom resolve_install_dir)" "honours an explicit JOSHIFY_INSTALL_DIR"
assert_eq "$VBIN" "$(PATH="$VBIN:$PATH" resolve_install_dir)" "replaces an existing install rather than shadowing it"

FAKE2="$WORK/home2"; mkdir -p "$FAKE2/.cargo/bin"
assert_eq "$FAKE2/.cargo/bin" "$(HOME="$FAKE2" PATH=/usr/bin:/bin resolve_install_dir)" "prefers ~/.cargo/bin when it exists"
FAKE3="$WORK/home3"; mkdir -p "$FAKE3"
assert_eq "$FAKE3/.local/bin" "$(HOME="$FAKE3" PATH=/usr/bin:/bin resolve_install_dir)" "falls back to ~/.local/bin"

echo "verify_checksum"
# ---------------------------------------------------------------------------
PAYLOAD="$WORK/joshify-linux-x86_64.tar.gz"
echo "pretend tarball" > "$PAYLOAD"
GOOD=$(sha256_of "$PAYLOAD")
SUMS="$WORK/SHA256SUMS"

printf '%s  joshify-linux-x86_64.tar.gz\n' "$GOOD" > "$SUMS"
if verify_checksum "$PAYLOAD" "$SUMS" joshify-linux-x86_64.tar.gz; then
    ok "accepts a matching checksum"
else
    fail "accepts a matching checksum"
fi

printf '%s  joshify-linux-x86_64.tar.gz\n' "0000000000000000000000000000000000000000000000000000000000000000" > "$SUMS"
verify_checksum "$PAYLOAD" "$SUMS" joshify-linux-x86_64.tar.gz; RC=$?
assert_eq "1" "$RC" "rejects a mismatched checksum with rc=1 (fatal)"

: > "$SUMS"
verify_checksum "$PAYLOAD" "$SUMS" joshify-linux-x86_64.tar.gz; RC=$?
assert_eq "2" "$RC" "reports rc=2 when no checksums are published"

printf '%s  some-other-asset.tar.gz\n' "$GOOD" > "$SUMS"
verify_checksum "$PAYLOAD" "$SUMS" joshify-linux-x86_64.tar.gz; RC=$?
assert_eq "2" "$RC" "reports rc=2 when this asset is absent from SHA256SUMS"

echo "install_from_release"
# ---------------------------------------------------------------------------
# Exercises the real function end to end against a stubbed download: builds a
# release tarball + SHA256SUMS locally and serves them through a curl stub.
REL="$WORK/fake-release"; mkdir -p "$REL"
printf '#!/bin/sh\necho "Joshify 9.9.9"\n' > "$REL/joshify-linux-x86_64"
chmod +x "$REL/joshify-linux-x86_64"
tar -czf "$REL/joshify-linux-x86_64.tar.gz" -C "$REL" joshify-linux-x86_64
( cd "$REL" && sha256_of joshify-linux-x86_64.tar.gz \
    | awk '{print $1"  joshify-linux-x86_64.tar.gz"}' > SHA256SUMS )

# curl stub: resolve the requested URL to a file in $REL by basename.
curl() {
    local out="" url=""
    while [ $# -gt 0 ]; do
        case "$1" in
            -o) out="$2"; shift 2 ;;
            http*) url="$1"; shift ;;
            *) shift ;;
        esac
    done
    [ -n "$url" ] || return 1
    local src
    src="$REL/$(basename "$url")"
    [ -f "$src" ] || return 22          # curl's exit code for a 404 with -f
    cp "$src" "$out"
}
_TEST_OS=Linux; _TEST_ARCH=x86_64
uname() { case "${1:-}" in -s) echo "$_TEST_OS" ;; -m) echo "$_TEST_ARCH" ;; esac; }

# shellcheck disable=SC2034  # both are read by install_from_release
TARGET_TAG="v9.9.9"
# shellcheck disable=SC2034
SUDO_MODE="unavailable"
WORK_DIR="$WORK/dl"; mkdir -p "$WORK_DIR"
INSTALL_DIR="$WORK/target-bin"

if install_from_release > /dev/null 2>&1; then
    ok "installs a verified release binary"
else
    fail "installs a verified release binary"
fi
assert_eq "0.7.2" "$(printf '%s' "$(installed_version "$VBIN/joshify")")" "leaves other installs alone"
if [ -x "$INSTALL_DIR/joshify" ]; then ok "installed binary is executable"; else fail "installed binary is executable"; fi
assert_eq "9.9.9" "$("$INSTALL_DIR/joshify" --version | awk '{print $NF}')" "installed binary is the downloaded one"

# Re-running must overwrite cleanly and leave no staging files behind.
rm -rf "${WORK_DIR:?}"/*
install_from_release > /dev/null 2>&1
if [ -z "$(find "$INSTALL_DIR" -name '.joshify.new.*' -print -quit)" ]; then
    ok "leaves no staging files behind (idempotent re-run)"
else
    fail "leaves no staging files behind (idempotent re-run)"
fi

# A corrupted tarball must be refused outright, never installed.
printf 'corrupted\n' > "$REL/joshify-linux-x86_64.tar.gz"
rm -rf "${WORK_DIR:?}"/*
INSTALL_DIR="$WORK/target-bad"
if ( install_from_release ) > /dev/null 2>&1; then
    fail "refuses a tarball whose checksum does not match"
else
    ok "refuses a tarball whose checksum does not match"
fi
if [ ! -e "$INSTALL_DIR/joshify" ]; then
    ok "installs nothing when the checksum fails"
else
    fail "installs nothing when the checksum fails"
fi

# No prebuilt binary for the platform -> caller falls back to source.
_TEST_ARCH=aarch64
if install_from_release > /dev/null 2>&1; then
    fail "signals fallback when no asset exists for the platform"
else
    ok "signals fallback when no asset exists for the platform"
fi
unset -f curl uname

echo "install.sh guards"
# ---------------------------------------------------------------------------
if [ "$(grep -c 'DEBIAN_FRONTEND=noninteractive' "$SCRIPT_DIR/install.sh")" -ge 2 ]; then
    ok "every apt install path is non-interactive"
else
    fail "every apt install path is non-interactive"
fi

# Regression guard: the native package manager must win over linuxbrew on
# Linux, which does not provide the -dev packages the build needs.
if grep -A6 'uname -s.*Darwin' "$SCRIPT_DIR/install.sh" | grep -q 'apt-get'; then
    ok "Linux prefers the native package manager over linuxbrew"
else
    fail "Linux prefers the native package manager over linuxbrew"
fi

if grep -q 'JOSHIFY_INSTALL_LIB_ONLY' "$SCRIPT_DIR/install.sh"; then
    ok "install.sh can be sourced without running"
else
    fail "install.sh can be sourced without running"
fi

echo ""
if [ "$FAILURES" -eq 0 ]; then
    echo "All install.sh tests passed."
else
    echo "$FAILURES install.sh test(s) failed."
    exit 1
fi
