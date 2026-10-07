$ErrorActionPreference = 'Stop'
$repoDir = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $repoDir
try {
    if (!(Test-Path src/bridge_generated.rs) -or !(Test-Path flutter/lib/generated_bridge.dart)) {
        & flutter_rust_bridge_codegen --rust-input src/flutter_ffi.rs --dart-output flutter/lib/generated_bridge.dart --c-output flutter/macos/Runner/bridge_generated.h
        if ($LASTEXITCODE -ne 0) { throw 'Bridge generation failed' }
    }
    & python build.py --flutter
    if ($LASTEXITCODE -ne 0) { throw 'Windows desktop build failed' }
    & cargo build --locked --release -p base --bin rustdesk-wake
    if ($LASTEXITCODE -ne 0) { throw 'Wake helper build failed' }
    Write-Host 'Use the desktop installer to install the service, then run enable-windows.ps1 as administrator.'
} finally {
    Pop-Location
}
