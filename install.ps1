# install.ps1 — one-liner installer for Rani367/silicon fork (Windows)
# Usage: irm https://raw.githubusercontent.com/Rani367/silicon/master/install.ps1 | iex
#Requires -Version 5.1
$ErrorActionPreference = 'Stop'

# ── Color helpers ────────────────────────────────────────────────────────────
function Write-Info  { param([string]$Msg) Write-Host "[info]  $Msg" -ForegroundColor Cyan }
function Write-Ok    { param([string]$Msg) Write-Host "[ok]    $Msg" -ForegroundColor Green }
function Write-Warn  { param([string]$Msg) Write-Host "[warn]  $Msg" -ForegroundColor Yellow }
function Write-Err   { param([string]$Msg) Write-Host "[error] $Msg" -ForegroundColor Red }

# ── Constants ────────────────────────────────────────────────────────────────
$ForkUrl   = 'https://github.com/Rani367/silicon.git'
$SrcDir    = Join-Path $env:USERPROFILE '.silicon-src'
$Msrv      = '1.87'
$CargoBin  = Join-Path $env:USERPROFILE '.cargo\bin'
$ConfigDir = Join-Path $env:APPDATA 'silicon'

# ── Helper: check if command exists ──────────────────────────────────────────
function Test-Command {
    param([string]$Name)
    $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

# ── Helper: compare version strings (major.minor) ───────────────────────────
function Compare-Version {
    param([string]$Current, [string]$Required)
    $c = $Current -split '\.'
    $r = $Required -split '\.'
    $cMajor = [int]$c[0]; $cMinor = [int]$c[1]
    $rMajor = [int]$r[0]; $rMinor = [int]$r[1]
    if ($cMajor -lt $rMajor) { return -1 }
    if ($cMajor -gt $rMajor) { return 1 }
    if ($cMinor -lt $rMinor) { return -1 }
    if ($cMinor -gt $rMinor) { return 1 }
    return 0
}

# ── Prerequisites ────────────────────────────────────────────────────────────

# git
if (-not (Test-Command 'git')) {
    Write-Err 'git is not installed.'
    if (Test-Command 'winget') {
        Write-Err '  Install with: winget install Git.Git'
    } else {
        Write-Err '  Download from: https://git-scm.com/download/win'
    }
    exit 1
}
Write-Ok 'git found'

# cargo / rustc
if (-not (Test-Command 'cargo') -or -not (Test-Command 'rustc')) {
    Write-Warn 'Rust toolchain not found. Installing via rustup...'
    $rustupInit = Join-Path $env:TEMP 'rustup-init.exe'
    Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile $rustupInit -UseBasicParsing
    & $rustupInit -y
    Remove-Item $rustupInit -ErrorAction SilentlyContinue

    # Refresh PATH for this session
    $env:PATH = "$CargoBin;$env:PATH"

    if (-not (Test-Command 'cargo')) {
        Write-Err 'Failed to install Rust. Please install manually: https://rustup.rs'
        exit 1
    }
    Write-Ok 'Rust installed'
} else {
    Write-Ok 'cargo found'
}

# Rust version >= MSRV
$rustVersionOutput = & rustc --version
if ($rustVersionOutput -match 'rustc (\d+\.\d+)') {
    $rustVersion = $Matches[1]
    if ((Compare-Version $rustVersion $Msrv) -lt 0) {
        Write-Warn "Rust $rustVersion is below minimum $Msrv. Running rustup update..."
        & rustup update stable
        Write-Ok 'Rust updated'
    } else {
        Write-Ok "Rust $rustVersion >= $Msrv"
    }
}

# MSVC build tools (non-fatal)
$hasVS = $null -ne (Get-Command 'cl.exe' -ErrorAction SilentlyContinue) -or
         (Test-Path 'C:\Program Files\Microsoft Visual Studio') -or
         (Test-Path 'C:\Program Files (x86)\Microsoft Visual Studio') -or
         (Test-Path "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe")
if (-not $hasVS) {
    Write-Warn 'MSVC build tools not detected. Tree-sitter grammars may fail to build.'
    Write-Warn '  Install "Desktop development with C++" from: https://visualstudio.microsoft.com/visual-cpp-build-tools/'
}
else {
    Write-Ok 'MSVC build tools found'
}

# PATH check
if ($env:PATH -notlike "*$CargoBin*") {
    Write-Warn "$CargoBin is not in your PATH."
    Write-Warn '  Restart your terminal or add it manually.'
}

# ── Remove existing Silicon installations ──────────────────────────────────────
Write-Info 'Checking for existing Silicon installations...'

# winget
if (Test-Command 'winget') {
    $wingetList = & winget list --id Silicon.Silicon 2>$null
    if ($LASTEXITCODE -eq 0 -and $wingetList -match 'Silicon\.Silicon') {
        Write-Info 'Removing winget Silicon...'
        & winget uninstall --id Silicon.Silicon --silent
        Write-Ok 'winget Silicon removed'
    }
}

# scoop
if (Test-Command 'scoop') {
    $scoopList = & scoop list 2>$null | Select-String -Pattern '^silicon\b'
    if ($scoopList) {
        Write-Info 'Removing scoop silicon...'
        & scoop uninstall silicon
        Write-Ok 'scoop silicon removed'
    }
}

# chocolatey
if (Test-Command 'choco') {
    $chocoList = & choco list --local-only silicon 2>$null
    if ($LASTEXITCODE -eq 0 -and $chocoList -match 'silicon') {
        Write-Info 'Removing chocolatey silicon...'
        & choco uninstall silicon -y
        Write-Ok 'chocolatey silicon removed'
    }
}

# cargo (old silicon-term install)
if (Test-Command 'cargo') {
    $cargoList = & cargo install --list 2>$null
    if ($cargoList -match '^silicon-term') {
        Write-Info 'Removing cargo silicon-term...'
        & cargo uninstall silicon-term
        Write-Ok 'cargo silicon-term removed'
    }
}

# ── Clone or update source ───────────────────────────────────────────────────
if (Test-Path $SrcDir) {
    if (Test-Path (Join-Path $SrcDir '.git')) {
        Write-Info "Updating existing source in $SrcDir..."
        Push-Location $SrcDir
        & git fetch --depth 1 origin master
        & git reset --hard origin/master
        Pop-Location
        Write-Ok 'Source updated'
    } else {
        Write-Warn "$SrcDir exists but is not a git repo. Removing and re-cloning..."
        Remove-Item -Recurse -Force $SrcDir
        Write-Info "Cloning $ForkUrl into $SrcDir..."
        & git clone --depth 1 $ForkUrl $SrcDir
        Write-Ok 'Source cloned'
    }
} else {
    Write-Info "Cloning $ForkUrl into $SrcDir..."
    & git clone --depth 1 $ForkUrl $SrcDir
    Write-Ok 'Source cloned'
}

# ── Build ────────────────────────────────────────────────────────────────────
Write-Info 'Building Silicon (this may take a few minutes)...'
Push-Location $SrcDir
& cargo install --path silicon-term --locked
Pop-Location
Write-Ok "Silicon built and installed to $CargoBin\si.exe"

# ── Set up runtime (directory junction) ──────────────────────────────────────
Write-Info 'Setting up runtime directory...'
if (-not (Test-Path $ConfigDir)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
}

$RuntimeTarget = Join-Path $ConfigDir 'runtime'
$RuntimeSource = Join-Path $SrcDir 'runtime'

# Remove existing symlink/junction
if (Test-Path $RuntimeTarget) {
    $item = Get-Item $RuntimeTarget -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        # It's a symlink or junction — remove it
        $item.Delete()
    } else {
        # It's a real directory — back it up
        $backup = "$RuntimeTarget.bak.$(Get-Date -Format 'yyyyMMddHHmmss')"
        Write-Warn "Existing runtime directory found. Backing up to $backup"
        Rename-Item $RuntimeTarget $backup
    }
}

# Try directory junction first (no admin required), fall back to copy
try {
    & cmd /c mklink /J "$RuntimeTarget" "$RuntimeSource" | Out-Null
    if (-not (Test-Path $RuntimeTarget)) { throw 'Junction failed' }
    Write-Ok "Runtime junction created: $RuntimeTarget -> $RuntimeSource"
} catch {
    Write-Warn 'Directory junction failed. Copying runtime directory instead...'
    Copy-Item -Recurse -Force $RuntimeSource $RuntimeTarget
    Write-Ok "Runtime copied to $RuntimeTarget"
    Write-Warn 'Note: You will need to re-run this script after updates to refresh the runtime.'
}

# ── Verify ───────────────────────────────────────────────────────────────────
Write-Info 'Verifying installation...'
if (Test-Command 'si') {
    & si --health
    Write-Host ''
    Write-Ok 'Silicon installed successfully!'
    Write-Info 'Run `si` to start editing.'
} else {
    Write-Warn "si not found in PATH. Restart your terminal or add $CargoBin to your PATH."
}
