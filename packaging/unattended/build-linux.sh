#!/usr/bin/env bash
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(cd -- "$script_dir/../.." && pwd)
cd "$repo_dir"
if [[ ! -f src/bridge_generated.rs || ! -f flutter/lib/generated_bridge.dart ]]; then
    flutter_rust_bridge_codegen --rust-input src/flutter_ffi.rs --dart-output flutter/lib/generated_bridge.dart --c-output flutter/macos/Runner/bridge_generated.h
fi
cargo build --locked --release --lib --features flutter,drm,drm-wake,linux-pkg-config
cargo build --locked --release -p base --bin rustdesk-wake
patchelf --remove-rpath target/release/liblibrustdesk.so
patchelf --remove-rpath target/release/rustdesk-wake
python3 -c 'import build; build.ffi_bindgen_function_refactor()'
python3 build.py --flutter --drm --skip-cargo
package_version=$(python3 -c 'import build; print(build.get_version())')
python3 packaging/unattended/package-dependencies.py "rustdesk-unattended-wayland-$package_version.deb"
echo 'The desktop package includes the DRM login-screen backend. The wake helper is target/release/rustdesk-wake.'
