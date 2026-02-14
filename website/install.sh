#!/bin/sh
# install.sh — one-liner installer for Silicon (downloads pre-built binary)
# Usage: curl -sSf https://raw.githubusercontent.com/Rani367/Silicon/master/install.sh | sh
set -e

# ── Color helpers (disabled when piped) ──────────────────────────────────────
if [ -t 1 ]; then
    RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
    CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; CYAN=''; BOLD=''; RESET=''
fi

info()  { printf "${CYAN}[info]${RESET}  %s\n" "$*"; }
ok()    { printf "${GREEN}[ok]${RESET}    %s\n" "$*"; }
warn()  { printf "${YELLOW}[warn]${RESET}  %s\n" "$*"; }
err()   { printf "${RED}[error]${RESET} %s\n" "$*" >&2; }

# ── Constants ────────────────────────────────────────────────────────────────
GITHUB_REPO="Rani367/Silicon"
BIN_DIR="$HOME/.local/bin"

# ── Platform detection ───────────────────────────────────────────────────────
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  OS_NAME="linux" ;;
        Darwin) OS_NAME="macos" ;;
        *)
            err "Unsupported OS: $OS"
            err "Pre-built binaries are available for Linux and macOS."
            err "For Windows, use install.ps1. For other platforms, build from source."
            exit 1
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH_NAME="x86_64" ;;
        aarch64|arm64)   ARCH_NAME="aarch64" ;;
        *)
            err "Unsupported architecture: $ARCH"
            err "Pre-built binaries are available for x86_64 and aarch64."
            exit 1
            ;;
    esac

    PLATFORM="${ARCH_NAME}-${OS_NAME}"
    ok "Detected platform: $PLATFORM"
}

# ── Data directory (matches etcetera crate behavior) ─────────────────────────
detect_data_dir() {
    case "$(uname -s)" in
        Darwin)
            DATA_DIR="$HOME/Library/Application Support/silicon"
            ;;
        *)
            DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/silicon"
            ;;
    esac
}

# ── Prerequisites ────────────────────────────────────────────────────────────
check_prerequisites() {
    if ! command -v curl >/dev/null 2>&1; then
        err "curl is required but not found."
        case "$(uname -s)" in
            Darwin) err "  Install with: brew install curl" ;;
            Linux)
                if command -v apt >/dev/null 2>&1; then
                    err "  Install with: sudo apt install curl"
                elif command -v dnf >/dev/null 2>&1; then
                    err "  Install with: sudo dnf install curl"
                elif command -v pacman >/dev/null 2>&1; then
                    err "  Install with: sudo pacman -S curl"
                fi
                ;;
        esac
        exit 1
    fi

    if ! command -v tar >/dev/null 2>&1; then
        err "tar is required but not found."
        exit 1
    fi

    # xz support check (macOS has it built-in, Linux may need xz-utils)
    if ! command -v xz >/dev/null 2>&1; then
        err "xz is required for extracting archives but not found."
        case "$(uname -s)" in
            Linux)
                if command -v apt >/dev/null 2>&1; then
                    err "  Install with: sudo apt install xz-utils"
                elif command -v dnf >/dev/null 2>&1; then
                    err "  Install with: sudo dnf install xz"
                elif command -v pacman >/dev/null 2>&1; then
                    err "  Install with: sudo pacman -S xz"
                fi
                ;;
        esac
        exit 1
    fi

    ok "Prerequisites satisfied (curl, tar, xz)"
}

# ── Root warning ─────────────────────────────────────────────────────────────
if [ "$(id -u)" -eq 0 ]; then
    warn "Running as root. Consider running as a normal user instead."
fi

# ── Main ─────────────────────────────────────────────────────────────────────
detect_platform
detect_data_dir
check_prerequisites

# ── Remove existing Silicon installations ──────────────────────────────────
info "Checking for existing Silicon installations..."

# Homebrew
if command -v brew >/dev/null 2>&1 && brew list silicon >/dev/null 2>&1; then
    info "Removing Homebrew silicon..."
    brew uninstall silicon
    ok "Homebrew silicon removed"
fi

# apt
if command -v apt >/dev/null 2>&1 && dpkg -l silicon 2>/dev/null | grep -q '^ii'; then
    info "Removing apt silicon..."
    sudo apt remove -y silicon
    ok "apt silicon removed"
fi

# pacman
if command -v pacman >/dev/null 2>&1 && pacman -Qi silicon >/dev/null 2>&1; then
    info "Removing pacman silicon..."
    sudo pacman -Rns --noconfirm silicon
    ok "pacman silicon removed"
fi

# dnf
if command -v dnf >/dev/null 2>&1 && dnf list installed silicon >/dev/null 2>&1; then
    info "Removing dnf silicon..."
    sudo dnf remove -y silicon
    ok "dnf silicon removed"
fi

# snap
if command -v snap >/dev/null 2>&1 && snap list silicon >/dev/null 2>&1; then
    info "Removing snap silicon..."
    sudo snap remove silicon
    ok "snap silicon removed"
fi

# flatpak
if command -v flatpak >/dev/null 2>&1 && flatpak list --app | grep -q com.silicon_editor.Silicon; then
    info "Removing flatpak silicon..."
    flatpak uninstall -y com.silicon_editor.Silicon
    ok "flatpak silicon removed"
fi

# cargo (old silicon-term install)
if command -v cargo >/dev/null 2>&1 && cargo install --list 2>/dev/null | grep -q '^silicon-term'; then
    info "Removing cargo silicon-term..."
    cargo uninstall silicon-term
    ok "cargo silicon-term removed"
fi

# Previous binary installs
if [ -f "$BIN_DIR/si" ]; then
    info "Removing previous $BIN_DIR/si..."
    rm -f "$BIN_DIR/si"
    ok "Previous binary removed"
fi

# ── Fetch latest release ────────────────────────────────────────────────────
info "Fetching latest release..."
RELEASE_JSON="$(curl -sSf "https://api.github.com/repos/$GITHUB_REPO/releases/latest")" || {
    err "Failed to fetch release info from GitHub."
    err "Check your internet connection or visit https://github.com/$GITHUB_REPO/releases"
    exit 1
}

# Extract tag name (portable: no jq dependency)
TAG="$(echo "$RELEASE_JSON" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"
if [ -z "$TAG" ]; then
    err "Could not determine latest release version."
    err "Visit https://github.com/$GITHUB_REPO/releases to download manually."
    exit 1
fi
ok "Latest release: $TAG"

# ── Download ─────────────────────────────────────────────────────────────────
ARCHIVE_NAME="silicon-${TAG}-${PLATFORM}.tar.xz"
DOWNLOAD_URL="https://github.com/$GITHUB_REPO/releases/download/$TAG/$ARCHIVE_NAME"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading $ARCHIVE_NAME..."
HTTP_CODE="$(curl -sSL -w '%{http_code}' -o "$TMPDIR/$ARCHIVE_NAME" "$DOWNLOAD_URL")"
if [ "$HTTP_CODE" != "200" ]; then
    err "Download failed (HTTP $HTTP_CODE)"
    err "URL: $DOWNLOAD_URL"
    err "No pre-built binary available for $PLATFORM."
    exit 1
fi
ok "Downloaded $(du -h "$TMPDIR/$ARCHIVE_NAME" | cut -f1 | tr -d '[:space:]')"

# ── Extract ──────────────────────────────────────────────────────────────────
info "Extracting..."
tar xJf "$TMPDIR/$ARCHIVE_NAME" -C "$TMPDIR"

# The archive contains a directory like silicon-25.7.1-aarch64-macos/
EXTRACT_DIR="$TMPDIR/silicon-${TAG}-${PLATFORM}"
if [ ! -d "$EXTRACT_DIR" ]; then
    # Try to find the extracted directory
    EXTRACT_DIR="$(find "$TMPDIR" -maxdepth 1 -type d -name 'silicon-*' | head -1)"
fi

if [ ! -f "$EXTRACT_DIR/si" ]; then
    err "Binary not found in archive. Contents:"
    ls -la "$TMPDIR"
    exit 1
fi

# ── Install binary ───────────────────────────────────────────────────────────
info "Installing binary to $BIN_DIR/si..."
mkdir -p "$BIN_DIR"
cp "$EXTRACT_DIR/si" "$BIN_DIR/si"
chmod +x "$BIN_DIR/si"
ok "Binary installed: $BIN_DIR/si"

# PATH check
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        warn "\$HOME/.local/bin is not in your PATH."
        warn "  Add to your shell profile:  export PATH=\"\$HOME/.local/bin:\$PATH\""
        ;;
esac

# ── Install runtime ─────────────────────────────────────────────────────────
info "Installing runtime to $DATA_DIR..."
mkdir -p "$DATA_DIR"

if [ -d "$DATA_DIR/runtime" ]; then
    rm -rf "$DATA_DIR/runtime"
fi
cp -r "$EXTRACT_DIR/runtime" "$DATA_DIR/runtime"
ok "Runtime installed: $DATA_DIR/runtime"

# ── Language servers ─────────────────────────────────────────────────────────
info "Installing language servers..."

# Python: ruff (linting/formatting) + jedi (completions/navigation)
if command -v pip3 >/dev/null 2>&1; then
    pip3 install --quiet --upgrade ruff jedi-language-server 2>/dev/null \
        && ok "Python LSPs installed (ruff + jedi)" \
        || warn "Failed to install Python LSPs via pip3"
elif command -v brew >/dev/null 2>&1; then
    brew install ruff 2>/dev/null && ok "ruff installed via brew" || warn "Failed to install ruff"
    pip3 install --quiet jedi-language-server 2>/dev/null \
        && ok "jedi-language-server installed" \
        || warn "Failed to install jedi-language-server (pip3 not available)"
else
    warn "pip3 not found. Install Python LSPs manually: pip3 install ruff jedi-language-server"
fi

# C/C++: clangd
if command -v clangd >/dev/null 2>&1; then
    ok "clangd already installed"
else
    case "$(uname -s)" in
        Darwin)
            if command -v brew >/dev/null 2>&1; then
                info "Installing clangd via brew (llvm)..."
                brew install llvm 2>/dev/null \
                    && ok "clangd installed via brew" \
                    || warn "Failed to install llvm. Try: xcode-select --install"
            else
                warn "Install clangd with: xcode-select --install  OR  brew install llvm"
            fi
            ;;
        Linux)
            if command -v apt >/dev/null 2>&1; then
                sudo apt install -y clangd 2>/dev/null && ok "clangd installed" || warn "Failed to install clangd"
            elif command -v dnf >/dev/null 2>&1; then
                sudo dnf install -y clang-tools-extra 2>/dev/null && ok "clangd installed" || warn "Failed to install clangd"
            elif command -v pacman >/dev/null 2>&1; then
                sudo pacman -S --noconfirm clang 2>/dev/null && ok "clangd installed" || warn "Failed to install clangd"
            else
                warn "Install clangd using your system package manager."
            fi
            ;;
    esac
fi

# TOML: taplo
if command -v taplo >/dev/null 2>&1; then
    ok "taplo already installed"
elif command -v brew >/dev/null 2>&1; then
    brew install taplo 2>/dev/null && ok "taplo installed" || warn "Failed to install taplo"
elif command -v cargo >/dev/null 2>&1; then
    cargo install taplo-cli 2>/dev/null && ok "taplo installed via cargo" || warn "Failed to install taplo"
else
    warn "Install taplo manually for TOML language support"
fi

# C#: csharp-ls
if command -v dotnet >/dev/null 2>&1; then
    dotnet tool install --global csharp-ls 2>/dev/null \
        || dotnet tool update --global csharp-ls 2>/dev/null
    ok "csharp-ls installed"
else
    warn "dotnet not found. Skipping C# language server."
    warn "  Install .NET SDK first, then run: dotnet tool install --global csharp-ls"
fi

# ── Verify ───────────────────────────────────────────────────────────────────
info "Verifying installation..."
if command -v si >/dev/null 2>&1; then
    si --health
    printf "\n"
    ok "Silicon installed successfully!"
    info "Run ${BOLD}si${RESET} to start editing."
elif [ -x "$BIN_DIR/si" ]; then
    "$BIN_DIR/si" --health
    printf "\n"
    ok "Silicon installed successfully!"
    info "Add $BIN_DIR to your PATH, then run ${BOLD}si${RESET} to start editing."
else
    err "Installation failed. Binary not found."
    exit 1
fi
