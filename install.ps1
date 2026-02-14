# install.ps1 — one-liner installer for Silicon (downloads pre-built binary)
# Usage: irm https://raw.githubusercontent.com/silicon-editor/Silicon/master/install.ps1 | iex
#Requires -Version 5.1
$ErrorActionPreference = 'Stop'

# ── Color helpers ────────────────────────────────────────────────────────────
function Write-Info  { param([string]$Msg) Write-Host "[info]  $Msg" -ForegroundColor Cyan }
function Write-Ok    { param([string]$Msg) Write-Host "[ok]    $Msg" -ForegroundColor Green }
function Write-Warn  { param([string]$Msg) Write-Host "[warn]  $Msg" -ForegroundColor Yellow }
function Write-Err   { param([string]$Msg) Write-Host "[error] $Msg" -ForegroundColor Red }

# ── Constants ────────────────────────────────────────────────────────────────
$GitHubRepo = 'silicon-editor/Silicon'
$Platform   = 'x86_64-windows'
$BinDir     = Join-Path $env:LOCALAPPDATA 'Programs\silicon'
$DataDir    = Join-Path $env:APPDATA 'silicon'

# ── Helper: check if command exists ──────────────────────────────────────────
function Test-Command {
    param([string]$Name)
    $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

# ── Remove existing Silicon installations ──────────────────────────────────
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

# Previous binary installs
$SiBin = Join-Path $BinDir 'si.exe'
if (Test-Path $SiBin) {
    Write-Info "Removing previous $SiBin..."
    Remove-Item -Force $SiBin
    Write-Ok 'Previous binary removed'
}

# ── Fetch latest release ────────────────────────────────────────────────────
Write-Info 'Fetching latest release...'
try {
    $releaseInfo = Invoke-RestMethod -Uri "https://api.github.com/repos/$GitHubRepo/releases/latest" -UseBasicParsing
    $tag = $releaseInfo.tag_name
} catch {
    Write-Err 'Failed to fetch release info from GitHub.'
    Write-Err "Check your internet connection or visit https://github.com/$GitHubRepo/releases"
    exit 1
}

if (-not $tag) {
    Write-Err 'Could not determine latest release version.'
    Write-Err "Visit https://github.com/$GitHubRepo/releases to download manually."
    exit 1
}
Write-Ok "Latest release: $tag"

# ── Download ─────────────────────────────────────────────────────────────────
$archiveName = "silicon-$tag-$Platform.zip"
$downloadUrl = "https://github.com/$GitHubRepo/releases/download/$tag/$archiveName"

$tmpDir = Join-Path $env:TEMP "silicon-install-$(Get-Random)"
New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

try {
    Write-Info "Downloading $archiveName..."
    $archivePath = Join-Path $tmpDir $archiveName
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
    $size = [math]::Round((Get-Item $archivePath).Length / 1MB, 1)
    Write-Ok "Downloaded ${size}MB"

    # ── Extract ──────────────────────────────────────────────────────────────
    Write-Info 'Extracting...'
    Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force

    # The archive contains a directory like silicon-25.7.1-x86_64-windows/
    $extractDir = Join-Path $tmpDir "silicon-$tag-$Platform"
    if (-not (Test-Path $extractDir)) {
        $extractDir = Get-ChildItem -Path $tmpDir -Directory -Filter 'silicon-*' | Select-Object -First 1 -ExpandProperty FullName
    }

    $siBinary = Join-Path $extractDir 'si.exe'
    if (-not (Test-Path $siBinary)) {
        Write-Err "Binary not found in archive."
        Get-ChildItem -Path $tmpDir -Recurse | Format-Table Name, Length
        exit 1
    }

    # ── Install binary ───────────────────────────────────────────────────────
    Write-Info "Installing binary to $BinDir\si.exe..."
    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }
    Copy-Item -Force $siBinary (Join-Path $BinDir 'si.exe')
    Write-Ok "Binary installed: $BinDir\si.exe"

    # PATH check and update
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath -notlike "*$BinDir*") {
        Write-Info "Adding $BinDir to user PATH..."
        [Environment]::SetEnvironmentVariable('Path', "$BinDir;$userPath", 'User')
        $env:PATH = "$BinDir;$env:PATH"
        Write-Ok 'PATH updated (restart terminal for full effect)'
    }

    # ── Install runtime ──────────────────────────────────────────────────────
    Write-Info "Installing runtime to $DataDir..."
    if (-not (Test-Path $DataDir)) {
        New-Item -ItemType Directory -Path $DataDir -Force | Out-Null
    }

    $runtimeTarget = Join-Path $DataDir 'runtime'
    $runtimeSource = Join-Path $extractDir 'runtime'

    if (Test-Path $runtimeTarget) {
        Remove-Item -Recurse -Force $runtimeTarget
    }
    Copy-Item -Recurse -Force $runtimeSource $runtimeTarget
    Write-Ok "Runtime installed: $runtimeTarget"

} finally {
    # Cleanup temp directory
    if (Test-Path $tmpDir) {
        Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
    }
}

# ── Verify ───────────────────────────────────────────────────────────────────
Write-Info 'Verifying installation...'
$siPath = Join-Path $BinDir 'si.exe'
if (Test-Path $siPath) {
    & $siPath --health
    Write-Host ''
    Write-Ok 'Silicon installed successfully!'
    Write-Info 'Run `si` to start editing.'
} else {
    Write-Err 'Installation failed. Binary not found.'
    exit 1
}
