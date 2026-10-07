# Unattended access, wake, and session recovery

This feature targets Windows and Linux hosts and controllers. Initial installation,
password setup, firmware settings, and network setup must be completed before the
host is left unattended. RustDesk cannot run before the operating system starts,
unlock a pre-boot encrypted disk, or supply power to a machine disconnected from
electricity. It does not bypass the operating system's authentication.

## Host setup

1. Install RustDesk as a system service. For Linux Wayland and login-screen
   capture, use the DRM build and bundled libdrmtap. A normal X11 session uses the
   existing X11 capture path.
2. Enable automatic startup and service recovery:
   - Linux: `sudo bash packaging/unattended/enable-linux.sh`
   - Windows, from an administrator PowerShell:
     `& .\packaging\unattended\enable-windows.ps1`
   These scripts configure an already installed service. They do not choose a
   password or change authentication policy.
3. In Security settings, select **Accept sessions via password**, set and enable a
   permanent password, and disable the requirement to keep the RustDesk window
   open. Enable **Require unattended access**. Use **Check automatic service
   startup** to check persistent boot configuration.
4. Connect once from the controller, verify capture and input, then test from the
   locked screen and after logout before relying on remote boot.

Unattended mode rejects a desktop login when the system service is unavailable or
automatic startup cannot be verified. Linux currently checks a systemd unit;
non-systemd installations cannot satisfy that check. On Wayland it also requires
available DRM capture and initialized uinput devices. If DRM capture fails,
RustDesk reports an error to the controller instead of opening a portal dialog on
the host. A previously granted portal token is not treated as a guarantee of
prompt-free access.

The existing Windows session chooser remains on the controller. Display selection
appears on the controller after the first image, when several displays are
available. **Remember this display** saves its index for subsequent connections;
an unavailable index falls back to asking. This is an index preference, not a
physical-monitor identity guarantee after hotplug or reordering. Privacy-mode
restrictions and the existing all-local-displays preference take precedence.
**Wake settings → Ask for a display on the next connection** clears the saved index.

The existing service follows graphical session changes. The client now retries
recognized offline, reset, timeout, and connection-closed errors for up to three
minutes, with delays capped at five seconds, when the host advertises unattended
mode or the client has saved wake settings. Password and 2FA prompts remain on the
controller. Authentication failures and explicit manual disconnects are not
converted into unattended retries. A reconnect preserves the remote tab and uses
the normal RustDesk authentication flow; it does not guarantee the underlying
transport survives an OS session change.

## LAN wake and connect

Open a peer's context menu and select **Wake settings**. Enter the wired adapter's
MAC address and its subnet's broadcast address, including UDP port (usually 9),
for example `192.168.1.255:9`. A LAN-discovered MAC is offered when available.
The limited broadcast `255.255.255.255:9` follows the operating system's routing;
choose the correct subnet broadcast on a multihomed controller.

**Wake and connect** sends three magic packets, opens the remote session, and
allows the bounded connection retries to wait for boot. Its acknowledgement means
the packet was sent, not that the firmware accepted it or that boot succeeded.
The connection then presents the existing password, 2FA, Windows session, and
display choices on the controller.

If the host remains offline, verify its MAC, broadcast destination, Ethernet link,
firmware wake settings, network adapter wake settings, standby power, VLANs, and
firewall rules. The client cannot inspect a powered-off host's firmware or detect
whether a switch discarded a broadcast. Wi-Fi and USB adapters need separate
hardware verification.

Microsoft documents that Windows 10 Fast Startup shutdown does not arm the
adapter for WoL, and shutdown wake behavior can depend on firmware. Test the exact
sleep/hibernate/shutdown state you intend to use rather than assuming a successful
sleep wake proves shutdown wake works. See [Microsoft's WoL behavior reference](https://learn.microsoft.com/en-us/troubleshoot/windows-client/setup-upgrade-and-drivers/wake-on-lan-feature).

## Internet wake helper

An always-on Windows or Linux machine on the target LAN runs `rustdesk-wake`.
The controller needs a reachable TCP address for that helper, normally through a
VPN or an explicitly configured TCP port forward. This helper does not implement
rendezvous, reverse tunnels, or CGNAT traversal. Subsequent desktop connections use
the configured RustDesk rendezvous/relay infrastructure, including self-hosted
servers, independently of the wake helper.

Build it with:

```sh
cargo build --locked --release -p base --bin rustdesk-wake
target/release/rustdesk-wake keygen
```

Copy `res/wake-helper.example.json` to a private configuration file. Replace the
sample MAC, broadcast address, and key. Select a listening address reachable by the
controller; the sample binds only loopback. Each device has its own name and one
or two keys. Do not commit real keys. On Unix, set the file mode to `600`.

```sh
chmod 600 /absolute/path/wake.json
target/release/rustdesk-wake check /absolute/path/wake.json
target/release/rustdesk-wake serve /absolute/path/wake.json
```

For automatic helper startup:

- Linux: `sudo bash packaging/unattended/install-helper-linux.sh /absolute/path/wake.json`
- Windows, administrator PowerShell:
  `& .\packaging\unattended\install-helper-windows.ps1 -ConfigFile C:\private\wake.json`

The Linux helper runs under a dedicated account. The Windows helper uses an
at-startup scheduled task; its installed directory is restricted to SYSTEM and
Administrators. Installation refuses to overwrite an existing configuration.
Neither installer changes firewall or router rules. Permit only the helper's
selected TCP port from the networks that need access, and permit outbound LAN UDP
broadcasts to the configured destinations.

On the controller, enable **Use a wake helper** in the peer's wake settings and
enter its `host:port`, device name, and key. Keys are stored using RustDesk's local
credential encryption and are not returned by the settings read API. Leave the key
field empty when editing to retain it; changing the helper or device requires
entering the corresponding key again.

### Protocol and key rotation

The helper sends a fresh random 32-byte challenge. The controller authenticates
the challenge, device name, and versioned wake command with libsodium secretbox.
The helper accepts one command per connection and authenticates its reply with
the same key and challenge. Replaying a prior request on a new connection fails.
The network request cannot specify a MAC address or broadcast destination; only
the helper's allowlist can do that. Frames are limited to 4096 bytes, reads have a
five-second frame deadline, and the helper processes at most one connection per
second with one active connection. This bounds resource usage but does not provide
resilience against a client repeatedly occupying the listener; use network access
controls where availability is important. The device name is visible on the wire;
keys and command payloads are not sent in plaintext. This is a dedicated protocol,
not HTTPS, and requires the matching RustDesk helper/client implementation.

To rotate a device key, add a freshly generated second key to that device's `keys`
list, distribute it to the authorized controllers, then remove the old key. The
helper reloads its configuration for each accepted request, so removing a key
revokes subsequent requests without restarting. A request already being processed
uses the configuration read for that connection. Changing the listening address
requires a restart. A key grants wake authority for its configured device, not
RustDesk desktop access. Desktop authentication remains separate.

## Builds and deployment

Use the repository's documented Flutter/Rust/native build prerequisites and pinned
bridge generator from `.github/workflows/bridge.yml`.

- Linux Debian/Ubuntu: `bash packaging/unattended/build-linux.sh` builds the Flutter
  library with `drm`, `drm-wake`, and system-library support, then invokes the
  existing DRM Debian packaging path, including the pinned libdrmtap build. It
  also builds the standalone helper. This path requires the native development
  packages used by the existing build and the libdrmtap build tools.
  `patchelf` removes build-machine Rust library search paths, and `dpkg-shlibdeps`
  records the resulting system-library dependencies in the package.
- Windows: `& .\packaging\unattended\build-windows.ps1` invokes the existing
  Windows Flutter desktop build and builds the helper. Run on a Windows build
  host with the repository's SDK/vcpkg requirements; install the resulting desktop
  application as a service, then run the supplied service setup script.

From a Linux workstation, push the desired source revision and dispatch
`.github/workflows/unattended-windows.yml` on that revision through GitHub Actions.
The workflow uses a Windows x64 runner, builds the desktop installer and wake
helper, and uploads them together as `rustdesk-unattended-windows-x64` in the run's
Artifacts section. GitHub's CLI can dispatch and download the run from Linux:

```sh
gh workflow run unattended-windows.yml --ref YOUR_PUSHED_BRANCH
gh run list --workflow unattended-windows.yml
gh run download RUN_ID --name rustdesk-unattended-windows-x64
```

The workflow file must exist on the repository's default branch before GitHub
will accept manual dispatch. This is a remote Windows build controlled from Linux;
Flutter does not support building its Windows desktop runner on a Linux host.
The build does not require signing credentials and produces an unsigned installer.

Service recovery is opt-in through the setup scripts. Normal installers and
feature-disabled capture paths retain their existing behavior. Firmware, adapter,
firewall, disk encryption, and router configuration are not modified automatically.

## Compatibility and acceptance checks

| Configuration | Required conditions / expected limitation |
| --- | --- |
| Windows sign-in and desktop | Installed automatic system service; test secure desktop and session switching on the actual Windows version |
| Linux X11 desktop and greeter | Accessible X11 server and input; actual display-manager setup must be verified |
| Linux Wayland desktop and greeter | DRM build, compatible GPU/scanout, bundled libdrmtap, root service, working uinput; no portal fallback |
| Headless host | Requires a capturable physical or virtual display; this feature does not create one |
| LAN shutdown wake | Firmware/NIC support for the exact shutdown state and standby power; Ethernet recommended |
| Internet shutdown wake | Above requirements plus reachable always-on helper on the correct LAN |
| Pre-boot disk PIN or firmware prompt | Outside RustDesk software capture; requires out-of-band hardware access or a separately managed boot configuration |
| Power physically removed | Not wakeable by a magic packet |

Before treating a deployment as validated, record the OS, display manager,
compositor/GPU, build features, and NIC/firmware, then run:

1. Connect without any target-side acceptance. Exercise password and 2FA from the
   controller. Verify an invalid permanent password cannot grant control.
2. Select each monitor, save a preference, reconnect, and test a removed monitor.
3. Lock, unlock, log out, switch users, and log in. Verify capture and keyboard/mouse
   return in the same remote tab or a clear client error is shown.
4. Restart the system service and interrupt the network. Verify bounded recovery,
   cancellation, and no retry after an explicit remote disconnect.
5. Disable boot startup and confirm unattended login reports the configuration
   error. Restore startup and verify the system service is reachable before login.
6. On Wayland, make DRM unavailable and verify no host-side portal prompt appears.
7. Wake from each supported power state over LAN. Repeat from outside the LAN via
   the helper. Distinguish helper acknowledgement from actual successful boot.
8. Use a wrong, rotated, and revoked helper key; replay an earlier request. Verify
   rejected requests produce no LAN wake packet.

Automated tests cover password prerequisites, refusal before portal access, magic
packet bytes, authenticated helper delivery and replay/tamper rejection, and the
client retry policy. They cannot establish GPU capture, NIC power-state behavior,
OS login transitions, or production router/firewall reachability.

## Validation record (2026-10-06)

The Linux amd64 release and Debian package were built successfully, including the
pinned DRM capture library. The package payload, checksums, dynamic dependencies,
and removal of temporary Rust library search paths were checked. This artifact
was built against Ubuntu 24.04 libraries and requires glibc 2.39 or newer plus the
package's listed dependencies; it is not an older-distribution compatibility build.

All 28 base-library tests, the portal-refusal regression test, and both Flutter
retry-policy tests passed (31 total). Rust checks passed with and without DRM.
Targeted Flutter analysis reported no errors and 18 existing informational lints.
The build used Flutter 3.24.5; its dependency resolution changes were excluded
from the source diff. No dependency declarations or submodule revisions changed.

No Windows or Linux target test machines were available. The Windows build and
PowerShell scripts have not been executed on Windows. Service installation,
secure-desktop control, graphical session transitions, GPU capture, and shutdown
wake therefore remain unverified on real targets. Neither system services nor
firmware/network settings were changed on the development machine.

## Regression surface

The final minimization review retained these changes to existing files because
each connects the feature to an existing entry point. New implementation lives in
separate unattended/wake modules, widgets, and packaging scripts.

| Existing files | Changed runtime path and reason |
| --- | --- |
| `src/server/connection.rs` | Opt-in desktop authentication checks, suppression of host-side acceptance requests, and capability advertisement are needed to enforce unattended prerequisites and inform the controller. |
| `src/server/input_service.rs` | Adds a Linux DRM readiness query against existing input devices so a session cannot claim unattended control without input. |
| `src/server/wayland.rs`, `libs/scrap/src/wayland/pipewire.rs` | Refuse interactive capture fallback in unattended mode, including the actual portal entry point, to avoid host-side screen-sharing prompts. |
| `src/server/video_service.rs` | Reports unattended capture refusal to subscribers, including connections awaiting their first frame, so the controller receives an error. |
| `src/flutter_ffi.rs` | Adds generic-command handlers for wake profiles/requests and prerequisite checks without changing bridge signatures. |
| `flutter/lib/common/widgets/peer_card.dart` | Adds wake settings and wake-and-connect actions to Windows/Linux peer menus. |
| `flutter/lib/desktop/pages/desktop_setting_page.dart` | Adds the opt-in security setting and automatic-service-startup check. |
| `flutter/lib/models/model.dart` | Hooks bounded retries into opted-in desktop connections and controller-side display selection into first-image handling. |
| `flutter/lib/consts.dart`, `libs/base/src/config/keys.rs` | Declares feature keys and registers the host option for existing settings persistence. |
| `libs/base/src/config/mod.rs`, `libs/base/src/lib.rs`, `src/lib.rs`, `src/platform/mod.rs`, `src/server.rs` | Registers the new modules; platform-specific application hooks are limited to Windows/Linux. |
| `src/lang/*.rs` (54 maps) | Appends feature labels and error messages; existing keys/translations are preserved and new Italian values are empty. |

With unattended mode disabled and no saved wake settings, the existing
authentication, capture, and reconnect paths remain in use. Wake menu actions are
additive. The deprecated Sciter UI and wire protocol schemas are unchanged.
