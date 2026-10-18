"""Keep Docker's toolchains aligned with local development and CI."""

import re
import tomllib
from pathlib import Path

dockerfile = Path("Dockerfile").read_text()
versions = {
    "Node.js": (
        Path(".node-version").read_text().strip(),
        r"^FROM node:([\d.]+)-",
    ),
    "Rust": (
        tomllib.loads(Path("rust-toolchain.toml").read_text())["toolchain"]["channel"],
        r"^FROM lukemathwalker/cargo-chef:[\d.]+-rust-([\d.]+)-",
    ),
}

for name, (expected, pattern) in versions.items():
    image = re.search(pattern, dockerfile, re.MULTILINE)
    if image is None or image[1] != expected:
        raise SystemExit(f"Update the Dockerfile to use {name} {expected}")
    print(f"{name} {expected}: Docker and project versions match")
