#Requires -Version 5.1
<#
.SYNOPSIS
  Install k1fix CLI on Windows.

.EXAMPLE
  .\scripts\install.ps1

.EXAMPLE
  irm https://raw.githubusercontent.com/<user>/k1fix/master/scripts/install.ps1 | iex
#>
[CmdletBinding()]
param(
    [string]$RepoUrl = $(if ($env:K1FIX_REPO_URL) { $env:K1FIX_REPO_URL } else { 'https://github.com/k1fix/k1fix.git' }),
    [string]$InstallDir = $(if ($env:K1FIX_INSTALL_DIR) { $env:K1FIX_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'k1fix\bin' }),
    [string]$Branch = $(if ($env:K1FIX_BRANCH) { $env:K1FIX_BRANCH } else { 'master' }),
    [switch]$NoPath
)

$ErrorActionPreference = 'Stop'
$BinName = 'k1fix.exe'

function Write-Step([string]$Message) {
    Write-Host "==> $Message"
}

function Test-Command([string]$Name) {
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Find-RepoRoot([string]$Start) {
    $dir = (Resolve-Path $Start).Path
    while ($true) {
        $cargo = Join-Path $dir 'Cargo.toml'
        if (Test-Path $cargo) {
            $text = Get-Content -Raw $cargo
            if ($text -match 'name\s*=\s*"k1fix"') {
                return $dir
            }
        }
        $parent = Split-Path $dir -Parent
        if ([string]::IsNullOrEmpty($parent) -or $parent -eq $dir) {
            return $null
        }
        $dir = $parent
    }
}

function Ensure-Rust {
    if ((Test-Command 'cargo') -and (Test-Command 'rustc')) {
        Write-Step "Rust OK: $(rustc --version)"
        return
    }

    Write-Step 'Rust no encontrado. Instalando rustup…'
    $rustup = Join-Path $env:TEMP 'rustup-init.exe'
    Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile $rustup
    & $rustup -y --default-toolchain stable
    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if (Test-Path $cargoBin) {
        $env:Path = "$cargoBin;$env:Path"
    }
    if (-not (Test-Command 'cargo')) {
        throw "rustup instaló pero cargo no está en PATH. Cerrá y abrí una terminal nueva."
    }
}

function Resolve-Source {
    $here = (Get-Location).Path
    $scriptDir = $PSScriptRoot
    if (-not $scriptDir -and $MyInvocation.MyCommand.Path) {
        $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    }

    foreach ($candidate in @($here, $scriptDir)) {
        if (-not $candidate) { continue }
        $root = Find-RepoRoot $candidate
        if ($root) { return $root }
    }

    if (-not (Test-Command 'git')) {
        throw "No hay repo local y falta git. Cloná el repo o instalá Git for Windows."
    }

    $cache = Join-Path $env:LOCALAPPDATA 'k1fix\src'
    Write-Step "Repo local no encontrado. Clonando $RepoUrl ($Branch) → $cache"
    New-Item -ItemType Directory -Force -Path (Split-Path $cache -Parent) | Out-Null
    if (Test-Path (Join-Path $cache '.git')) {
        git -C $cache fetch --depth 1 origin $Branch
        git -C $cache checkout -q FETCH_HEAD 2>$null
        if ($LASTEXITCODE -ne 0) {
            git -C $cache checkout -q $Branch
        }
    }
    else {
        if (Test-Path $cache) { Remove-Item -Recurse -Force $cache }
        git clone --depth 1 --branch $Branch $RepoUrl $cache
    }
    return $cache
}

function Add-ToUserPath([string]$Dir) {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $userPath) { $userPath = '' }
    $parts = $userPath -split ';' | Where-Object { $_ -and $_.Trim() -ne '' }
    if ($parts -contains $Dir) {
        Write-Step "PATH de usuario ya incluye $Dir"
        return
    }
    $newPath = if ($userPath.TrimEnd(';')) { "$userPath;$Dir" } else { $Dir }
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    $env:Path = "$Dir;$env:Path"
    Write-Step "Agregado al PATH de usuario: $Dir"
}

# --- main ---
Write-Step "Instalando $BinName…"
Ensure-Rust

if (-not (Test-Command 'cargo')) {
    throw 'cargo no disponible'
}

$src = Resolve-Source
Write-Step "Fuente: $src"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Write-Step 'Compilando release (puede tardar)…'
Push-Location $src
try {
    cargo build --release --bin k1fix
    if ($LASTEXITCODE -ne 0) { throw "cargo build falló ($LASTEXITCODE)" }
}
finally {
    Pop-Location
}

$built = Join-Path $src 'target\release\k1fix.exe'
if (-not (Test-Path $built)) {
    throw "No se generó $built"
}

$dest = Join-Path $InstallDir $BinName
Copy-Item -Force $built $dest
Write-Step "Binario: $dest"

if (-not $NoPath) {
    Add-ToUserPath $InstallDir
}

& $dest --version
Write-Step "Listo. Probá: k1fix profiles list"
Write-Host "Si 'k1fix' no se reconoce, abrí una terminal NUEVA (PATH)."
