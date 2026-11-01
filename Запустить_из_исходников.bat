@echo off
chcp 65001 >nul
cd /d "%~dp0"
set "CARGO_TARGET_DIR=%TEMP%\clicker-pro-target"
cargo run --release --locked --manifest-path "%~dp0rust_clicker\Cargo.toml"
if errorlevel 1 pause
