#!/usr/bin/env bash
# Joshify Installer
#
# Usage: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/install.sh | bash
#
# Installs the prebuilt release binary when one exists for this platform, and
# falls back to building from source when it does not. Re-running is safe: if
# the requested version is already installed the script exits without work.
#
# Environment variables:
#   JOSHIFY_VERSION             Tag to install (default: the latest release)
#   JOSHIFY_INSTALL_DIR         Where to put the binary (default: auto-detected)
#   JOSHIFY_BUILD_FROM_SOURCE   Set to 1 to skip the prebuilt binary entirely
#   JOSHIFY_FORCE               Set to 1 to reinstall even if already current
#   JOSHIFY_SKIP_DEPS           Set to 1 to skip installing system dependencies
#   JOSHIFY_ALLOW_UNVERIFIED    Set to 1 to accept a release with no checksums
#   TMPDIR                      Preferred temp dir (respected if it allows exec)

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

REPO_SLUG="bigknoxy/joshify"
REPO="https://github.com/${REPO_SLUG}.git"
API_URL="https://api.github.com/repos/${REPO_SLUG}"
BIN_NAME="joshify"

# Build-time dependencies, per package manager.
DEPS_APT="libasound2-dev pkg-config libssl-dev build-essential libchafa-dev libglib2.0-dev"
DEPS_DNF="alsa-lib-devel pkgconfig openssl-devel gcc chafa-devel glib2-devel"
DEPS_PACMAN="alsa-lib pkg-config openssl base-devel chafa glib2"
DEPS_BREW="pkgconf chafa"

# Runtime-only libraries the prebuilt binary links against. Far lighter than
# the build set: no headers, no compiler, no Rust.
RUNTIME_APT="libasound2t64 libssl3t64"
RUNTIME_APT_FALLBACK="libasound2 libssl3"
RUNTIME_DNF="alsa-lib openssl-libs"
RUNTIME_PACMAN="alsa-lib openssl"

info()  { echo -e "$1"; }
warn()  { echo -e "${YELLOW}$1${NC}"; }
die()   { echo -e "${RED}$1${NC}" >&2; exit 1; }

# --- temp dir handling -------------------------------------------------------
# /tmp is mounted noexec on WSL2 and many hardened Linux hosts. rustup-init and
# the binaries we test need to execute from the temp dir, so probe it first.

# can_exec_in DIR -> 0 if a binary written to DIR can be executed
can_exec_in() {
    local dir="$1" probe
    [ -d "$dir" ] && [ -w "$dir" ] || return 1

    probe="$dir/.joshify-exec-probe.$$"
    printf '#!/bin/sh\nexit 0\n' > "$probe" 2>/dev/null || return 1
    chmod +x "$probe" 2>/dev/null || { rm -f "$probe"; return 1; }

    if "$probe" >/dev/null 2>&1; then
        rm -f "$probe"
        return 0
    fi
    rm -f "$probe"
    return 1
}

# Echo the first temp dir that can execute binaries: $TMPDIR, /tmp, then
# $HOME/.cache/joshify/tmp as a fallback we create ourselves.
find_exec_tmpdir() {
    local candidate fallback
    for candidate in "${TMPDIR:-}" /tmp; do
        [ -n "$candidate" ] || continue
        if can_exec_in "$candidate"; then
            echo "$candidate"
            return 0
        fi
    done

    fallback="${HOME}/.cache/joshify/tmp"
    mkdir -p "$fallback" 2>/dev/null || return 1
    if can_exec_in "$fallback"; then
        echo "$fallback"
        return 0
    fi
    return 1
}

# --- sudo handling -----------------------------------------------------------
# With `curl | bash`, stdin is the script, so sudo has nothing to prompt on.
# /dev/tty still works when a real terminal is attached, so prefer that.

have_tty() {
    [ -e /dev/tty ] && { : < /dev/tty; } 2>/dev/null
}

# SUDO_MODE is one of: none (running as root), passwordless, tty, unavailable
detect_sudo_mode() {
    if [ "$(id -u)" = "0" ]; then
        echo "none"
    elif ! command -v sudo > /dev/null 2>&1; then
        echo "unavailable"
    elif sudo -n true 2>/dev/null; then
        echo "passwordless"
    elif have_tty; then
        echo "tty"
    else
        echo "unavailable"
    fi
}

# WSL has no sound card. WSLg provides a PulseAudio server, but the binary
# speaks ALSA, and no WSL image ships the ALSA-to-Pulse plugin, so every ALSA
# open fails with "Unknown PCM default". joshify then plays through `pacat`
# instead - which is in pulseaudio-utils and not installed by default either.
is_wsl() { grep -qi microsoft /proc/version 2>/dev/null; }

ensure_wsl_audio() {
    is_wsl || return 0
    if command -v pacat > /dev/null 2>&1; then
        return 0
    fi
    if [ -n "${JOSHIFY_SKIP_DEPS:-}" ] || ! command -v apt-get > /dev/null 2>&1; then
        warn "   WSL: install pulseaudio-utils (for pacat) to get local audio."
        return 0
    fi
    if [ "${SUDO_MODE:-unavailable}" = "unavailable" ]; then
        warn "   WSL: run 'sudo apt-get install -y pulseaudio-utils' to get local audio (pacat)."
        return 0
    fi
    info "   WSL detected: installing pulseaudio-utils so joshify can play through WSLg's PulseAudio..."
    # A fresh WSL image has no package lists at all; install fails without this.
    run_privileged apt-get update -qq > /dev/null 2>&1 || true
    if run_privileged env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq pulseaudio-utils; then
        info "   pacat installed."
    else
        warn "   Could not install pulseaudio-utils; run: sudo apt-get update && sudo apt-get install -y pulseaudio-utils"
    fi
}

run_privileged() {
    case "${SUDO_MODE:-unavailable}" in
        none)         "$@" ;;
        passwordless) sudo -n "$@" ;;
        tty)          sudo "$@" < /dev/tty ;;
        *)            return 1 ;;
    esac
}

# --- platform / release resolution -------------------------------------------

# Echo the release asset name for this platform, or nothing if no prebuilt
# binary is published for it. Keep in sync with the release workflow matrix.
release_asset_name() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "${os}:${arch}" in
        Linux:x86_64|Linux:amd64)      echo "joshify-linux-x86_64.tar.gz" ;;
        Darwin:arm64|Darwin:aarch64)   echo "joshify-macos-aarch64.tar.gz" ;;
        *)                             echo "" ;;
    esac
}

# Strip a leading "v" so tags and --version output compare cleanly.
normalize_version() {
    printf '%s' "${1#v}"
}

# Echo the version of an already-installed joshify, or nothing.
installed_version() {
    local bin="$1" out
    command -v "$bin" > /dev/null 2>&1 || return 0
    out="$("$bin" --version 2>/dev/null | head -1)" || return 0
    # "Joshify 0.7.2" -> "0.7.2"
    printf '%s' "$out" | awk '{print $NF}'
}

# Echo the latest published release tag.
latest_release_tag() {
    curl -fsSL --max-time 30 "${API_URL}/releases/latest" 2>/dev/null \
        | grep -m1 '"tag_name"' \
        | sed -e 's/.*"tag_name"[[:space:]]*:[[:space:]]*"//' -e 's/".*//'
}

# Echo the directory the binary should be installed into.
#  1. an explicit JOSHIFY_INSTALL_DIR
#  2. the directory of an existing joshify, so we replace rather than shadow it
#  3. ~/.cargo/bin when it exists (already on PATH for Rust users)
#  4. ~/.local/bin
resolve_install_dir() {
    local existing
    if [ -n "${JOSHIFY_INSTALL_DIR:-}" ]; then
        echo "$JOSHIFY_INSTALL_DIR"
        return 0
    fi
    if existing="$(command -v "$BIN_NAME" 2>/dev/null)" && [ -n "$existing" ]; then
        dirname "$existing"
        return 0
    fi
    if [ -d "${HOME}/.cargo/bin" ]; then
        echo "${HOME}/.cargo/bin"
        return 0
    fi
    echo "${HOME}/.local/bin"
}

sha256_of() {
    if command -v sha256sum > /dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum > /dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        return 1
    fi
}

# verify_checksum FILE SUMS_FILE ASSET_NAME
# 0 = verified, 1 = mismatch (fatal), 2 = cannot verify (no sums / no tooling)
verify_checksum() {
    local file="$1" sums="$2" asset="$3" want got
    [ -s "$sums" ] || return 2

    want="$(grep -F " ${asset}" "$sums" 2>/dev/null | head -1 | awk '{print $1}')"
    [ -n "$want" ] || return 2

    got="$(sha256_of "$file")" || return 2
    [ "$want" = "$got" ]
}

# --- try the prebuilt binary first -------------------------------------------

install_runtime_deps() {
    if [ -n "${JOSHIFY_SKIP_DEPS:-}" ]; then
        return 1
    fi
    if command -v apt-get > /dev/null 2>&1; then
        run_privileged apt-get update -qq || return 1
        # shellcheck disable=SC2086
        run_privileged env DEBIAN_FRONTEND=noninteractive \
            apt-get install -y -qq $RUNTIME_APT 2>/dev/null && return 0
        # Pre-t64 Debian/Ubuntu use the older package names.
        # shellcheck disable=SC2086
        run_privileged env DEBIAN_FRONTEND=noninteractive \
            apt-get install -y -qq $RUNTIME_APT_FALLBACK
    elif command -v dnf > /dev/null 2>&1; then
        # shellcheck disable=SC2086
        run_privileged dnf install -y $RUNTIME_DNF
    elif command -v pacman > /dev/null 2>&1; then
        # shellcheck disable=SC2086
        run_privileged pacman -S --noconfirm --needed $RUNTIME_PACMAN
    else
        return 1
    fi
}

# Install the released binary. Returns non-zero to mean "fall back to source".
install_from_release() {
    local asset url tarball sums extracted staged rc

    asset="$(release_asset_name)"
    if [ -z "$asset" ]; then
        info "   No prebuilt binary for $(uname -s)/$(uname -m)."
        return 1
    fi

    url="https://github.com/${REPO_SLUG}/releases/download/${TARGET_TAG}/${asset}"
    tarball="${WORK_DIR}/${asset}"
    sums="${WORK_DIR}/SHA256SUMS"

    info "   Downloading ${asset} (${TARGET_TAG})..."
    if ! curl -fsSL --max-time 300 --retry 3 -o "$tarball" "$url"; then
        warn "   Download failed."
        return 1
    fi

    # Checksums are published alongside the assets. Absence is only tolerated
    # for older releases that predate them.
    curl -fsSL --max-time 60 -o "$sums" \
        "https://github.com/${REPO_SLUG}/releases/download/${TARGET_TAG}/SHA256SUMS" \
        2>/dev/null || true

    set +e
    verify_checksum "$tarball" "$sums" "$asset"
    rc=$?
    set -e
    case "$rc" in
        0) info "   Checksum verified." ;;
        1) die "   Checksum MISMATCH for ${asset}. Refusing to install." ;;
        *)
            if [ -n "${JOSHIFY_ALLOW_UNVERIFIED:-}" ]; then
                warn "   No checksum published for ${TARGET_TAG}; continuing (JOSHIFY_ALLOW_UNVERIFIED=1)."
            else
                warn "   No checksum published for ${TARGET_TAG}, cannot verify the download."
                info "   Building from source instead. Set JOSHIFY_ALLOW_UNVERIFIED=1 to accept it."
                return 1
            fi
            ;;
    esac

    tar -xzf "$tarball" -C "$WORK_DIR" || { warn "   Could not extract ${asset}."; return 1; }

    # The tarball holds a single binary named after the platform.
    extracted="${WORK_DIR}/${asset%.tar.gz}"
    if [ ! -f "$extracted" ]; then
        extracted="$(find "$WORK_DIR" -maxdepth 2 -type f -name "${BIN_NAME}*" ! -name '*.tar.gz' | head -1)"
    fi
    [ -n "$extracted" ] && [ -f "$extracted" ] || { warn "   No binary inside ${asset}."; return 1; }
    chmod +x "$extracted"

    # Does it actually run here? A prebuilt binary still needs its shared
    # libraries, so try the runtime packages once before giving up.
    local smoke
    if ! smoke="$("$extracted" --version 2>&1)"; then
        info "   Binary did not run; installing runtime libraries..."
        install_runtime_deps > /dev/null 2>&1 || true
        if ! smoke="$("$extracted" --version 2>&1)"; then
            warn "   Prebuilt binary cannot run on this system:"
            # Report the actual reason. Falling back silently is how a broken
            # --version went unnoticed for a whole release.
            printf '%s\n' "$smoke" | head -3 | sed 's/^/     /'
            return 1
        fi
        info "   Runtime libraries installed."
    fi

    case "$smoke" in
        Joshify\ *) ;;
        *)
            warn "   Downloaded binary did not report a usable version:"
            printf '%s\n' "$smoke" | head -3 | sed 's/^/     /'
            return 1
            ;;
    esac

    mkdir -p "$INSTALL_DIR" || { warn "   Cannot create ${INSTALL_DIR}."; return 1; }

    # Install atomically so a re-run never leaves a half-written binary, and a
    # currently-running joshify keeps its open file handle.
    staged="${INSTALL_DIR}/.${BIN_NAME}.new.$$"
    cp "$extracted" "$staged" || { warn "   Cannot write to ${INSTALL_DIR}."; return 1; }
    chmod 755 "$staged"
    mv -f "$staged" "${INSTALL_DIR}/${BIN_NAME}" || { rm -f "$staged"; return 1; }

    return 0
}

# --- source build ------------------------------------------------------------

install_build_deps() {
    local pkg_mgr=""
    if [ "$(uname -s)" = "Darwin" ]; then
        if command -v brew > /dev/null 2>&1; then pkg_mgr="brew"; fi
    elif command -v apt-get > /dev/null 2>&1; then
        pkg_mgr="apt"
    elif command -v dnf > /dev/null 2>&1; then
        pkg_mgr="dnf"
    elif command -v pacman > /dev/null 2>&1; then
        pkg_mgr="pacman"
    elif command -v brew > /dev/null 2>&1; then
        pkg_mgr="brew"
    fi

    if [ -n "${JOSHIFY_SKIP_DEPS:-}" ]; then
        warn "JOSHIFY_SKIP_DEPS is set."
        print_manual_deps
        return 0
    fi
    if [ -z "$pkg_mgr" ]; then
        warn "Unknown OS - no supported package manager found."
        print_manual_deps
        return 0
    fi
    if [ "$pkg_mgr" != "brew" ] && [ "$SUDO_MODE" = "unavailable" ]; then
        warn "No TTY and no passwordless sudo available."
        print_manual_deps
        return 0
    fi

    case "$pkg_mgr" in
        brew)
            warn "Detected Homebrew - installing terminal graphics dependencies..."
            # shellcheck disable=SC2086
            brew install $DEPS_BREW
            ;;
        apt)
            warn "Detected Debian/Ubuntu - installing audio dependencies..."
            run_privileged apt-get update -qq
            # shellcheck disable=SC2086
            run_privileged env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq $DEPS_APT
            ;;
        dnf)
            warn "Detected Fedora/RHEL - installing audio dependencies..."
            # shellcheck disable=SC2086
            run_privileged dnf install -y $DEPS_DNF
            ;;
        pacman)
            warn "Detected Arch - installing audio dependencies..."
            # shellcheck disable=SC2086
            run_privileged pacman -S --noconfirm --needed $DEPS_PACMAN
            ;;
    esac
}

print_manual_deps() {
    info "   Install these yourself before building:"
    info "     Debian/Ubuntu: sudo apt-get install -y $DEPS_APT"
    info "     Fedora/RHEL:   sudo dnf install -y $DEPS_DNF"
    info "     Arch:          sudo pacman -S --noconfirm $DEPS_PACMAN"
    info "     macOS:         brew install $DEPS_BREW"
}

install_from_source() {
    local src="${WORK_DIR}/src"

    if ! command -v cargo > /dev/null 2>&1; then
        warn "Rust not found. Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env" 2>/dev/null || true
    fi
    command -v cargo > /dev/null 2>&1 \
        || die "Rust installed but 'cargo' is not on PATH. Run: source \"\$HOME/.cargo/env\" and re-run."

    info "Rust: $(cargo --version)"
    echo ""

    echo "📦 Checking system dependencies..."
    install_build_deps
    echo ""

    echo "🔨 Building Joshify from source (this takes a few minutes)..."
    if [ -n "$TARGET_TAG" ]; then
        git clone --depth 1 --branch "$TARGET_TAG" "$REPO" "$src" 2>/dev/null \
            || git clone --depth 1 "$REPO" "$src"
    else
        git clone --depth 1 "$REPO" "$src"
    fi

    # cargo install --root keeps us consistent with the binary path's location.
    cargo install --path "$src" --root "${INSTALL_DIR%/bin}" --force
}

# Sourced by tests to get every helper without running the installer.
if [ -n "${JOSHIFY_INSTALL_LIB_ONLY:-}" ]; then
    return 0 2>/dev/null || exit 0
fi

echo "⚡ Joshify Installer ⚡"
echo "====================="
echo ""

# --- temp dir ----------------------------------------------------------------
if EXEC_TMPDIR="$(find_exec_tmpdir)"; then
    if [ "$EXEC_TMPDIR" != "${TMPDIR:-/tmp}" ]; then
        warn "Default temp dir cannot execute binaries (noexec?)."
        info "   Using $EXEC_TMPDIR instead."
        echo ""
    fi
    export TMPDIR="$EXEC_TMPDIR"
else
    warn "Warning: could not find a temp dir that allows executing binaries."
    info "   Set TMPDIR to a path on an exec-enabled filesystem and re-run."
    echo ""
fi

WORK_DIR="$(mktemp -d)"
cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT

# --- what are we installing, and is it already here? -------------------------
TARGET_TAG="${JOSHIFY_VERSION:-}"
if [ -z "$TARGET_TAG" ]; then
    TARGET_TAG="$(latest_release_tag || true)"
fi

if [ -n "$TARGET_TAG" ]; then
    TARGET_VERSION="$(normalize_version "$TARGET_TAG")"
    info "Target version: ${TARGET_VERSION}"
else
    TARGET_VERSION=""
    warn "Could not determine the latest release; will build from source."
fi

CURRENT_VERSION="$(installed_version "$BIN_NAME")"
if [ -n "$CURRENT_VERSION" ]; then
    info "Already installed: ${CURRENT_VERSION} ($(command -v "$BIN_NAME"))"
fi

if [ -n "$TARGET_VERSION" ] && [ "$CURRENT_VERSION" = "$TARGET_VERSION" ] \
   && [ -z "${JOSHIFY_FORCE:-}" ]; then
    echo ""
    echo -e "${GREEN}✓ Joshify ${CURRENT_VERSION} is already installed and current.${NC}"
    echo "   Re-run with JOSHIFY_FORCE=1 to reinstall."
    exit 0
fi
echo ""

INSTALL_DIR="$(resolve_install_dir)"
SUDO_MODE="$(detect_sudo_mode)"

# --- run ---------------------------------------------------------------------

INSTALL_METHOD=""
if [ -z "${JOSHIFY_BUILD_FROM_SOURCE:-}" ] && [ -n "$TARGET_TAG" ]; then
    echo "📦 Installing prebuilt binary..."
    if install_from_release; then
        INSTALL_METHOD="release binary"
    else
        echo ""
        warn "Falling back to building from source."
        echo ""
    fi
fi

if [ -z "$INSTALL_METHOD" ]; then
    install_from_source
    INSTALL_METHOD="source build"
fi

# --- verify ------------------------------------------------------------------
echo ""
ensure_wsl_audio
INSTALLED_BIN="${INSTALL_DIR}/${BIN_NAME}"
[ -x "$INSTALLED_BIN" ] || die "✗ Installation failed: ${INSTALLED_BIN} is not executable."

FINAL_VERSION="$("$INSTALLED_BIN" --version 2>/dev/null | head -1 | awk '{print $NF}')"

if [ -n "$FINAL_VERSION" ]; then
    echo -e "${GREEN}✓ Joshify ${FINAL_VERSION} installed via ${INSTALL_METHOD}${NC}"
    if [ -n "$TARGET_VERSION" ] && [ "$FINAL_VERSION" != "$TARGET_VERSION" ]; then
        warn "   Note: expected ${TARGET_VERSION}, got ${FINAL_VERSION}."
    fi
else
    # The binary is installed and executable; only the version probe failed.
    # Older builds did not support --version, so this must not fail the install.
    echo -e "${GREEN}✓ Joshify installed via ${INSTALL_METHOD}${NC}"
    warn "   Could not read the version back from the installed binary."
fi
echo "   ${INSTALLED_BIN}"

case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        echo ""
        warn "${INSTALL_DIR} is not on your PATH. Add it:"
        info "   echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc && source ~/.bashrc"
        ;;
esac

echo ""
echo "Run '$BIN_NAME' to start the app."
echo ""
warn "Optional: Non-interactive mode (skip browser auth)"
echo "All three are required together - setting only some of them still"
echo "opens a browser and blocks waiting for the callback:"
echo "  export SPOTIFY_CLIENT_ID=your_client_id"
echo "  export SPOTIFY_CLIENT_SECRET=your_client_secret"
echo "  export SPOTIFY_ACCESS_TOKEN=your_access_token"
echo "Recommended alongside them (tokens expire after an hour otherwise):"
echo "  export SPOTIFY_REFRESH_TOKEN=your_refresh_token"
echo "  export SPOTIFY_TOKEN_EXPIRES_AT=unix_timestamp"
echo ""
echo "🎵 Joshify plays audio locally through your machine's speakers!"
echo "   Press 'd' to switch between local and remote devices."
echo ""
echo "Uninstall with: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/uninstall.sh | bash"
echo ""
