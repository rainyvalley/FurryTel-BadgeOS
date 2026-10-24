#!/usr/bin/env python3
"""Wrap a raw RP2040 binary as a UF2 file.

    python3 tools/uf2.py firmware.bin badgeos.uf2

The binary is written at the start of flash (0x10000000).  Drag the resulting .uf2 onto the
RPI-RP2 drive while the badge is in BOOTSEL.
"""

import struct
import sys

BASE = 0x10000000
CHUNK = 256
MAGIC_START0 = 0x0A324655
MAGIC_START1 = 0x9E5D5157
MAGIC_END = 0x0AB16F30
FAMILY_RP2040 = 0xE48BFF56
FLAG_FAMILY = 0x00002000


def wrap(data: bytes) -> bytes:
    blocks = []
    for offset in range(0, len(data), CHUNK):
        chunk = data[offset : offset + CHUNK]
        blocks.append((BASE + offset, chunk + b"\xFF" * (CHUNK - len(chunk))))

    out = bytearray()
    for number, (address, chunk) in enumerate(blocks):
        out += struct.pack(
            "<IIIIIIII",
            MAGIC_START0,
            MAGIC_START1,
            FLAG_FAMILY,
            address,
            CHUNK,
            number,
            len(blocks),
            FAMILY_RP2040,
        )
        out += chunk
        out += b"\xFF" * (476 - CHUNK)
        out += struct.pack("<I", MAGIC_END)
    return bytes(out)


# The firmware keeps its saved settings in the last 4 KB.
SETTINGS_SECTOR = 2 * 1024 * 1024 - 4096


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    try:
        data = open(sys.argv[1], "rb").read()
    except OSError as problem:
        sys.exit(f"error: cannot read {sys.argv[1]}: {problem}")
    if not data:
        sys.exit(f"error: {sys.argv[1]} is empty")
    if data[:4] == b"\x7fELF":
        sys.exit(f"error: {sys.argv[1]} is an ELF; convert it to a raw binary first")
    if len(data) > SETTINGS_SECTOR:
        sys.exit(
            f"error: {sys.argv[1]} is {len(data)} bytes, past the {SETTINGS_SECTOR}-byte limit; "
            "the last 4 KB of flash holds saved settings"
        )
    uf2 = wrap(data)
    try:
        open(sys.argv[2], "wb").write(uf2)
    except OSError as problem:
        sys.exit(f"error: cannot write {sys.argv[2]}: {problem}")
    print(f"{len(data)} bytes -> {len(uf2)} byte UF2")


if __name__ == "__main__":
    main()
