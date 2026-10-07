#!/usr/bin/env bash
set -euo pipefail

if [[ ${EUID} -ne 0 ]]; then
    echo 'Run this setup script with sudo after installing the RustDesk package.' >&2
    exit 1
fi
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(cd -- "$script_dir/../.." && pwd)
systemctl cat rustdesk.service >/dev/null
install -d -m 755 /etc/systemd/system/rustdesk.service.d
install -m 644 "$repo_dir/res/rustdesk-unattended.conf" /etc/systemd/system/rustdesk.service.d/unattended.conf
systemctl daemon-reload
systemctl enable --now rustdesk.service
echo 'RustDesk starts at boot and restarts after a service failure.'
echo 'Set a permanent password and enable Require unattended access in RustDesk Security settings.'
