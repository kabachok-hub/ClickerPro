param([switch]$Offline)
$ErrorActionPreference = 'Stop'
$clickerRoot = Split-Path -Parent $PSScriptRoot
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP 'clicker-pro-target'
$clickerManifest = Join-Path $clickerRoot 'rust_clicker/Cargo.toml'
$cargoNetworkArgs = @()
if ($Offline) { $cargoNetworkArgs += '--offline' }
& cargo test --release --locked @cargoNetworkArgs --manifest-path $clickerManifest -- --skip screen_capture::tests::test_capture_screen --skip win_input::tests::test_find_roblox
if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
& cargo build --release --locked @cargoNetworkArgs --manifest-path $clickerManifest
if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
Write-Output (Join-Path $env:CARGO_TARGET_DIR 'release/rust_clicker.exe')
