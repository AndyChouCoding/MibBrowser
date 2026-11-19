# Run from PowerShell on a Windows x64 development machine.
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
$tooling = Join-Path $project '.tooling'
$env:CARGO_HOME = Join-Path $tooling 'cargo'
$env:RUSTUP_HOME = Join-Path $tooling 'rustup'
$env:COREPACK_HOME = Join-Path $tooling 'corepack'
$env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH

function Invoke-Checked($program, [string[]]$arguments) {
    & $program @arguments
    if ($LASTEXITCODE -ne 0) { throw "$program exited with code $LASTEXITCODE" }
}

Push-Location $project
try {
    if (-not (Get-Command corepack.cmd -ErrorAction SilentlyContinue)) {
        throw 'Install Node.js 24 with Corepack and reopen PowerShell.'
    }
    New-Item -ItemType Directory -Force $tooling | Out-Null
    $rustup = Join-Path $env:CARGO_HOME 'bin/rustup.exe'
    if (-not (Test-Path $rustup)) {
        $installer = Join-Path $tooling 'rustup-init.exe'
        Invoke-WebRequest -Uri 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' -OutFile $installer -UseBasicParsing
        Invoke-Checked $installer @('-y', '--no-modify-path', '--profile', 'minimal', '--default-toolchain', 'stable')
    }
    Invoke-Checked $rustup @('component', 'add', 'rustfmt', 'clippy')
    Invoke-Checked 'corepack.cmd' @('pnpm', 'install', '--frozen-lockfile')
    foreach ($task in 'lint', 'typecheck', 'test', 'build') {
        Invoke-Checked 'corepack.cmd' @('pnpm', $task)
    }
    $manifest = 'src-tauri/Cargo.toml'
    Invoke-Checked 'cargo.exe' @('fmt', '--manifest-path', $manifest, '--check')
    Invoke-Checked 'cargo.exe' @('clippy', '--manifest-path', $manifest, '--all-targets', '--all-features', '--', '-D', 'warnings')
    Invoke-Checked 'cargo.exe' @('test', '--manifest-path', $manifest, '--all-features')
} finally {
    Pop-Location
}
