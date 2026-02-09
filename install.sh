#!/bin/sh
# install.sh — one-liner installer for Rani367/silicon fork
# Usage: curl -sSf https://raw.githubusercontent.com/Rani367/silicon/master/install.sh | sh
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
FORK_URL="https://github.com/Rani367/silicon.git"
SRC_DIR="$HOME/.silicon-src"
MSRV="1.87"
CARGO_BIN="$HOME/.cargo/bin"

# Config dir: XDG on all Unix (matches etcetera::choose_base_strategy)
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/silicon"

# ── Root warning ─────────────────────────────────────────────────────────────
if [ "$(id -u)" -eq 0 ]; then
    warn "Running as root. The build artifacts will be owned by root."
    warn "Consider running as a normal user instead."
fi

# ── Prerequisites ────────────────────────────────────────────────────────────

# git
if ! command -v git >/dev/null 2>&1; then
    err "git is not installed."
    case "$(uname -s)" in
        Darwin) err "  Install with: xcode-select --install  OR  brew install git" ;;
        Linux)
            if command -v apt >/dev/null 2>&1; then
                err "  Install with: sudo apt install git"
            elif command -v dnf >/dev/null 2>&1; then
                err "  Install with: sudo dnf install git"
            elif command -v pacman >/dev/null 2>&1; then
                err "  Install with: sudo pacman -S git"
            else
                err "  Install git using your system package manager."
            fi
            ;;
    esac
    exit 1
fi
ok "git found"

# cargo / rustc
if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    warn "Rust toolchain not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # Source the env so cargo is available in this session
    . "$CARGO_BIN/env" 2>/dev/null || export PATH="$CARGO_BIN:$PATH"
    if ! command -v cargo >/dev/null 2>&1; then
        err "Failed to install Rust. Please install manually: https://rustup.rs"
        exit 1
    fi
    ok "Rust installed"
else
    ok "cargo found"
fi

# Rust version >= MSRV
rust_version="$(rustc --version | sed 's/rustc \([0-9]*\.[0-9]*\).*/\1/')"
rust_major="$(echo "$rust_version" | cut -d. -f1)"
rust_minor="$(echo "$rust_version" | cut -d. -f2)"
msrv_major="$(echo "$MSRV" | cut -d. -f1)"
msrv_minor="$(echo "$MSRV" | cut -d. -f2)"

if [ "$rust_major" -lt "$msrv_major" ] 2>/dev/null || \
   { [ "$rust_major" -eq "$msrv_major" ] && [ "$rust_minor" -lt "$msrv_minor" ]; } 2>/dev/null; then
    warn "Rust $rust_version is below minimum $MSRV. Running rustup update..."
    rustup update stable
    ok "Rust updated"
else
    ok "Rust $rust_version >= $MSRV"
fi

# C compiler (non-fatal)
if ! command -v cc >/dev/null 2>&1 && \
   ! command -v gcc >/dev/null 2>&1 && \
   ! command -v clang >/dev/null 2>&1; then
    warn "No C compiler found (cc/gcc/clang). Tree-sitter grammars may fail to build."
    case "$(uname -s)" in
        Darwin) warn "  Install with: xcode-select --install" ;;
        Linux)
            if command -v apt >/dev/null 2>&1; then
                warn "  Install with: sudo apt install build-essential"
            elif command -v dnf >/dev/null 2>&1; then
                warn "  Install with: sudo dnf install gcc"
            elif command -v pacman >/dev/null 2>&1; then
                warn "  Install with: sudo pacman -S base-devel"
            fi
            ;;
    esac
else
    ok "C compiler found"
fi

# PATH check
case ":$PATH:" in
    *":$CARGO_BIN:"*) ;;
    *)
        warn "\$HOME/.cargo/bin is not in your PATH."
        warn "  Add to your shell profile:  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
        ;;
esac

# ── Remove existing Silicon installations ──────────────────────────────────────
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

# ── Clone or update source ───────────────────────────────────────────────────
if [ -d "$SRC_DIR" ]; then
    if [ -d "$SRC_DIR/.git" ]; then
        info "Updating existing source in $SRC_DIR..."
        cd "$SRC_DIR"
        git fetch --depth 1 origin master
        git reset --hard origin/master
        ok "Source updated"
    else
        warn "$SRC_DIR exists but is not a git repo. Removing and re-cloning..."
        rm -rf "$SRC_DIR"
        info "Cloning $FORK_URL into $SRC_DIR..."
        git clone --depth 1 "$FORK_URL" "$SRC_DIR"
        ok "Source cloned"
    fi
else
    info "Cloning $FORK_URL into $SRC_DIR..."
    git clone --depth 1 "$FORK_URL" "$SRC_DIR"
    ok "Source cloned"
fi

# ── Build ────────────────────────────────────────────────────────────────────
info "Building Silicon (this may take a few minutes)..."
cd "$SRC_DIR"
cargo install --path silicon-term --locked
ok "Silicon built and installed to $CARGO_BIN/si"

# ── Language servers ────────────────────────────────────────────────────
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

# ── Symlink runtime ─────────────────────────────────────────────────────────
info "Setting up runtime directory..."
mkdir -p "$CONFIG_DIR"

RUNTIME_TARGET="$CONFIG_DIR/runtime"

if [ -L "$RUNTIME_TARGET" ]; then
    rm "$RUNTIME_TARGET"
elif [ -d "$RUNTIME_TARGET" ]; then
    BACKUP="$RUNTIME_TARGET.bak.$(date +%s)"
    warn "Existing runtime directory found. Backing up to $BACKUP"
    mv "$RUNTIME_TARGET" "$BACKUP"
fi

ln -sf "$SRC_DIR/runtime" "$RUNTIME_TARGET"
ok "Runtime symlinked: $RUNTIME_TARGET -> $SRC_DIR/runtime"

# ── Verify ───────────────────────────────────────────────────────────────────
info "Verifying installation..."
if command -v si >/dev/null 2>&1; then
    si --health
    printf "\n"
    ok "Silicon installed successfully!"
    info "Run ${BOLD}si${RESET} to start editing."
else
    warn "si not found in PATH. You may need to restart your shell or add $CARGO_BIN to PATH."
fi
