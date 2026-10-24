#!/usr/bin/env python3
"""Generate the badge's content, build the firmware, and wrap it as a UF2.

    python3 tools/build.py                                # badge.yaml -> dist/badgeos.uf2
    python3 tools/build.py --config badges/someone.yaml   # -> dist/someone.uf2

One badge at a time: the generated file (`src/generated.rs`) reflects whichever config you built
last, so rebuild with your own config before flashing your own badge again.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = ROOT / "target" / "thumbv6m-none-eabi" / "release" / "badgeos"


def find_objcopy() -> str:
    for name in ("llvm-objcopy", "objcopy"):
        path = shutil.which(name)
        if path:
            return path
    for path in Path.home().glob(".rustup/toolchains/*/lib/rustlib/*/bin/llvm-objcopy"):
        return str(path)
    sys.exit("error: no llvm-objcopy or objcopy; try `rustup component add llvm-tools`")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", default="badge.yaml")
    args = parser.parse_args()

    config = Path(args.config)
    stem = "badgeos" if config.stem == "badge" else config.stem

    objcopy = find_objcopy()

    try:
        subprocess.run(
            [sys.executable, str(ROOT / "tools/generate.py"), "--config", args.config],
            cwd=ROOT,
            check=True,
        )
    except subprocess.CalledProcessError:
        sys.exit("error: generating src/generated.rs failed; see the message above")

    try:
        subprocess.run(["cargo", "build", "--release"], cwd=ROOT, check=True)
    except subprocess.CalledProcessError:
        sys.exit("error: cargo build failed; see the message above")
    if not TARGET.is_file():
        sys.exit(
            f"error: cargo did not produce {TARGET}; is the thumbv6m-none-eabi target installed?"
        )

    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    binary = dist / f"{stem}.bin"
    subprocess.run([objcopy, "-O", "binary", str(TARGET), str(binary)], check=True)
    uf2 = dist / f"{stem}.uf2"
    subprocess.run([sys.executable, str(ROOT / "tools/uf2.py"), str(binary), str(uf2)], check=True)
    print(f"wrote {uf2.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
