#!/usr/bin/env python3
"""Turn badge.yaml into the Rust tables the firmware draws from.

    python3 tools/generate.py            # badge.yaml -> src/generated.rs
    python3 tools/generate.py --config other.yaml --out src/generated.rs

Requires: pyyaml, pillow, qrcode  (pip install pyyaml pillow qrcode)

The QR codes are always version 3 (29x29 modules) so every page has the same layout, and so the
code is as large as the 32-pixel-tall screen allows.  Pictures are fitted inside the screen,
centred, and reduced to one bit: dark pixels are ink.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    import qrcode
    import yaml
    from PIL import Image, ImageOps, UnidentifiedImageError
except ImportError as missing:
    sys.exit(f"error: missing python package ({missing.name}); pip install pyyaml pillow qrcode")

WIDTH = 128
HEIGHT = 32
QR_VERSION = 3
QR_SIZE = 21 + 4 * (QR_VERSION - 1)
NAME_LIMIT = 16
SPECIES_LIMIT = 10
TITLE_LIMIT = 15
LABEL_LIMIT = 23


def fail(message: str) -> None:
    sys.exit(f"error: {message}")


def is_ascii(value: str) -> bool:
    return all(0x20 <= ord(character) <= 0x7E for character in value)


def qr_rows(target: str, where: str) -> list[str]:
    qr = qrcode.QRCode(
        version=QR_VERSION,
        error_correction=qrcode.constants.ERROR_CORRECT_L,
        box_size=1,
        border=0,
    )
    qr.add_data(target)
    try:
        qr.make(fit=False)
    except qrcode.exceptions.DataOverflowError:
        fail(
            f"{where}: {target!r} is too long for a {QR_SIZE}x{QR_SIZE} QR code "
            f"(about 53 characters); shorten it or use a URL shortener"
        )
    return ["".join("#" if module else "." for module in row) for row in qr.get_matrix()]


def picture_rows(path: Path, scale: int, threshold: int, dither: bool, where: str) -> list[str]:
    try:
        image = Image.open(path)
        image = ImageOps.exif_transpose(image)
        # flatten transparency onto white, or a logo's transparent pixels turn into ink
        image = Image.alpha_composite(
            Image.new("RGBA", image.size, "white"), image.convert("RGBA")
        ).convert("L")
    except (UnidentifiedImageError, OSError) as problem:
        fail(f"{where}: cannot read {path}: {problem}")

    image = ImageOps.autocontrast(image, cutoff=1)
    image.thumbnail((WIDTH // scale, HEIGHT // scale), Image.LANCZOS)
    if image.width == 0 or image.height == 0:
        fail(f"{where}: {path} is empty")

    if dither:
        one_bit = image.convert("1", dither=Image.Dither.FLOYDSTEINBERG)
    else:
        one_bit = image.point(lambda value: 0 if value < threshold else 255).convert("1")

    pixels = one_bit.load()
    return [
        "".join("#" if pixels[x, y] == 0 else "." for x in range(one_bit.width))
        for y in range(one_bit.height)
    ]


def rust_string(value: str) -> str:
    # everything reaching here is plain ASCII, so only the quote and backslash need escaping
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def rust_rows(name: str, rows: list[str]) -> str:
    body = "\n".join(f'    "{row}",' for row in rows)
    return f"const {name}: &[&str] = &[\n{body}\n];\n"


def as_text(config: dict, key: str, limit: int, default: str = "") -> str:
    value = config.get(key, default)
    if value is None:
        value = default
    if not isinstance(value, str):
        fail(f"{key}: expected text, got {type(value).__name__}")
    if len(value) > limit:
        fail(f"{key}: {value!r} is too long; the most that fits is {limit} characters")
    if not is_ascii(value):
        fail(f"{key}: {value!r} has characters the badge's fonts cannot draw (use plain ASCII)")
    return value


def as_list(config: dict, key: str) -> list:
    value = config.get(key) or []
    if not isinstance(value, list):
        fail(f"{key}: expected a list")
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", default="badge.yaml")
    parser.add_argument("--out", default="src/generated.rs")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    config_path = Path(args.config).expanduser()
    if not config_path.is_absolute():
        config_path = root / config_path
    if not config_path.is_file():
        fail(f"no config file at {config_path}")
    try:
        config = yaml.safe_load(config_path.read_text(encoding="utf-8")) or {}
    except yaml.YAMLError as problem:
        mark = getattr(problem, "problem_mark", None)
        where = f"{config_path}:{mark.line + 1}" if mark is not None else str(config_path)
        fail(f"{where}: {getattr(problem, 'problem', problem)}")
    if not isinstance(config, dict):
        fail(f"{config_path}: the top level must be a mapping of keys to values")

    name = as_text(config, "name", NAME_LIMIT, "Badge")
    species = as_text(config, "species", SPECIES_LIMIT)
    links = as_list(config, "links")
    pictures = as_list(config, "pictures")

    out = [
        f"// Generated by tools/generate.py from {config_path.name}.  Edit that, not this file.\n",
        f"pub const NAME: &str = {rust_string(name)};\n",
        f"pub const SPECIES: &str = {rust_string(species)};\n",
        "\n/// One scan-me page: a title, the text on screen, the URL in the QR code, and the code.\n",
        "pub struct Link {\n",
        "    pub title: &'static str,\n",
        "    pub label: &'static str,\n",
        "    pub qr: &'static [&'static str],\n",
        "}\n",
    ]

    titles = set()
    link_entries = []
    for index, link in enumerate(links):
        where = f"links[{index}]"
        if not isinstance(link, dict):
            fail(f"{where}: expected a mapping with title and url")
        title = link.get("title")
        target = link.get("url")
        if not isinstance(title, str) or not title:
            fail(f"{where}: needs a title")
        if not isinstance(target, str) or not target.strip():
            fail(f"{where}: needs a url")
        if len(title) > TITLE_LIMIT:
            fail(f"{where}: title {title!r} is too long; the most that fits is {TITLE_LIMIT}")
        if not is_ascii(title):
            fail(f"{where}: title {title!r} is not plain ASCII")
        if title in titles:
            fail(f"{where}: duplicate title {title!r}")
        titles.add(title)

        label = link.get("label") or target.removeprefix("https://").removeprefix("http://")
        if not isinstance(label, str):
            fail(f"{where}: label must be text")
        if len(label) > LABEL_LIMIT:
            fail(f"{where}: label {label!r} is too long; the most that fits is {LABEL_LIMIT}")
        if not is_ascii(label):
            fail(f"{where}: label {label!r} is not plain ASCII")

        const = f"QR_{index}"
        out.append("\n" + rust_rows(const, qr_rows(target, where)))
        link_entries.append(
            f"    Link {{\n"
            f"        title: {rust_string(title)},\n"
            f"        label: {rust_string(label)},\n"
            f"        qr: {const},\n"
            f"    }},"
        )

    out.append("\npub const LINKS: &[Link] = &[\n" + "\n".join(link_entries) + "\n];\n")

    out.append(
        "\n/// A picture: a name for the file list, its rows of '#' and '.', and how many screen\n"
        "/// pixels each character covers.\n"
        "pub struct Picture {\n"
        "    pub name: &'static str,\n"
        "    pub rows: &'static [&'static str],\n"
        "    pub scale: u32,\n"
        "}\n"
    )

    picture_entries = []
    for index, picture in enumerate(pictures):
        where = f"pictures[{index}]"
        if not isinstance(picture, dict):
            fail(f"{where}: expected a mapping with name and image")
        label = picture.get("name")
        image = picture.get("image")
        if not isinstance(label, str) or not label:
            fail(f"{where}: needs a name")
        if len(label) > LABEL_LIMIT or not is_ascii(label):
            fail(f"{where}: name {label!r} must be plain ASCII and at most {LABEL_LIMIT} characters")
        if not isinstance(image, str) or not image:
            fail(f"{where}: needs an image path")

        scale = picture.get("scale", 1)
        if not isinstance(scale, int) or isinstance(scale, bool):
            fail(f"{where}: scale must be a whole number")
        if not 1 <= scale <= 32:
            fail(f"{where}: scale {scale} does not fit; use 1 to 32")
        threshold = picture.get("threshold", 128)
        if not isinstance(threshold, int) or isinstance(threshold, bool) or not 0 <= threshold <= 255:
            fail(f"{where}: threshold must be a whole number from 0 to 255")
        dither = picture.get("dither", False)
        if not isinstance(dither, bool):
            fail(f"{where}: dither must be true or false")

        path = root / image
        if not path.is_file():
            fail(f"{where}: no image at {path}")

        rows = picture_rows(path, scale, threshold, dither, where)
        const = f"PICTURE_{index}"
        out.append("\n" + rust_rows(const, rows))
        picture_entries.append(
            f"    Picture {{\n"
            f"        name: {rust_string(label)},\n"
            f"        rows: {const},\n"
            f"        scale: {scale},\n"
            f"    }},"
        )

    out.append("\npub const PICTURES: &[Picture] = &[\n" + "\n".join(picture_entries) + "\n];\n")

    out_path = Path(args.out).expanduser()
    if not out_path.is_absolute():
        out_path = root / out_path
    out_path.write_text("".join(out), encoding="utf-8")
    print(
        f"wrote {out_path.name}: {len(links)} links, {len(pictures)} pictures, "
        f"name {name!r}, species {species!r}"
    )


if __name__ == "__main__":
    main()
