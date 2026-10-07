#!/usr/bin/env bash
set -euo pipefail
if [[ ${EUID} -ne 0 || $# -ne 1 ]]; then
    echo 'Usage: sudo bash install-helper-linux.sh /absolute/path/to/config.json' >&2
    exit 1
fi
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(cd -- "$script_dir/../.." && pwd)
binary="$repo_dir/target/release/rustdesk-wake"
"$binary" check "$1"
if [[ -e /etc/rustdesk-wake.json ]]; then
    echo '/etc/rustdesk-wake.json already exists; preserve or update its device keys explicitly.' >&2
    exit 1
fi
if ! id rustdesk-wake >/dev/null 2>&1; then
    useradd --system --user-group --no-create-home --shell /usr/sbin/nologin rustdesk-wake
fi
install -m 755 "$binary" /usr/local/bin/rustdesk-wake
install -o rustdesk-wake -g rustdesk-wake -m 600 "$1" /etc/rustdesk-wake.json
install -m 644 "$repo_dir/res/rustdesk-wake.service" /etc/systemd/system/rustdesk-wake.service
systemctl daemon-reload
systemctl enable --now rustdesk-wake.service
