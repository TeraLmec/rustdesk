#!/usr/bin/env python3
"""Record the actual system-library dependencies of the pkg-config build."""
import pathlib
import subprocess
import sys
import tempfile


def run(package):
    package = pathlib.Path(package).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="rustdesk-dependencies-") as temporary:
        root = pathlib.Path(temporary)
        payload = root / "payload"
        subprocess.run(["dpkg-deb", "--raw-extract", str(package), str(payload)], check=True)
        (root / "debian").mkdir()
        (root / "debian/control").write_text(
            "Source: rustdesk-unattended-wayland\nSection: net\nPriority: optional\n"
            "Maintainer: RustDesk <info@rustdesk.com>\n\n"
            "Package: rustdesk-unattended-wayland\nArchitecture: any\nDescription: RustDesk\n"
        )
        binaries = []
        for file in payload.rglob("*"):
            if file.is_file() and not file.is_symlink():
                with file.open("rb") as stream:
                    if stream.read(4) == b"\x7fELF":
                        binaries.append("-e" + str(file))
        if not binaries:
            raise RuntimeError("No ELF files in the desktop package")
        result = subprocess.run(
            ["dpkg-shlibdeps", "--ignore-missing-info", "-O",
             "-l" + str(payload / "usr/share/rustdesk/lib"),
             "-l" + str(payload / "usr/lib/rustdesk")] + binaries,
            cwd=root, text=True, stdout=subprocess.PIPE, check=True,
        )
        dependencies = next(
            line.split("=", 1)[1] for line in result.stdout.splitlines()
            if line.startswith("shlibs:Depends=")
        )
        control = payload / "DEBIAN/control"
        lines = control.read_text().splitlines()
        if not any(line.startswith("Depends:") for line in lines):
            raise RuntimeError("Missing package dependency field")
        control.write_text("\n".join(
            line + ", " + dependencies if line.startswith("Depends:") else line
            for line in lines
        ) + "\n")
        subprocess.run(["dpkg-deb", "--root-owner-group", "--build", str(payload), str(package)], check=True)
        print("Verified system dependencies for", len(binaries), "packaged ELF files")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("Usage: package-dependencies.py PACKAGE.deb")
    run(sys.argv[1])
