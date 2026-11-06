#!/usr/bin/env python3
"""Download and subset the fonts embedded by voxalfa-svg.

Requires fonttools:

    python3 -m pip install fonttools
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import urllib.request

from dataclasses import dataclass
from pathlib import Path

NOTO_BASE = "https://raw.githubusercontent.com/notofonts/noto-fonts/main/hinted/ttf"
GOOGLE_FONTS_BASE = "https://raw.githubusercontent.com/google/fonts/main/ofl/firasans"


def chars(start: str, end: str) -> str:
    return "".join(chr(codepoint) for codepoint in range(ord(start), ord(end) + 1))


BASIC_LATIN = chars(" ", "~")
LATIN_1 = chars("\u00a0", "\u00ff")
COMBINING = chars("\u0300", "\u036f")
DASHES = "–—"
QUOTES = "‘’“”"
ELLIPSIS = "…"

TEXT_CHARS = BASIC_LATIN + LATIN_1 + COMBINING + DASHES + QUOTES + ELLIPSIS
SERIF_CHARS = BASIC_LATIN + DASHES + QUOTES + ELLIPSIS
LYRIC_CHARS = BASIC_LATIN + DASHES
MUSIC_CHARS = "♭♮♯"
SOLFA_CHARS = " ,.123456789:adefilmrst"


@dataclass(frozen=True)
class Font:
    url: str
    characters: str


FONTS = [
    Font(f"{GOOGLE_FONTS_BASE}/FiraSans-Bold.ttf", SOLFA_CHARS),
    Font(f"{NOTO_BASE}/NotoSans/NotoSans-Regular.ttf", TEXT_CHARS),
    Font(f"{NOTO_BASE}/NotoSans/NotoSans-Bold.ttf", TEXT_CHARS),
    Font(f"{NOTO_BASE}/NotoSans/NotoSans-Condensed.ttf", LYRIC_CHARS),
    Font(f"{NOTO_BASE}/NotoSerif/NotoSerif-Regular.ttf", SERIF_CHARS),
    Font(f"{NOTO_BASE}/NotoSerif/NotoSerif-Italic.ttf", SERIF_CHARS),
    Font(f"{NOTO_BASE}/NotoSerif/NotoSerif-BoldItalic.ttf", SERIF_CHARS),
    Font(f"{NOTO_BASE}/NotoSansSymbols/NotoSansSymbols-Bold.ttf", MUSIC_CHARS),
]


def font_name(font: Font) -> str:
    return Path(font.url).name


def download(font: Font, cache_dir: Path) -> Path:
    name = font_name(font)
    path = cache_dir / name

    if path.exists():
        return path

    print(f"Downloading {name}")
    cache_dir.mkdir(parents=True, exist_ok=True)

    with urllib.request.urlopen(font.url) as response, path.open("wb") as file:
        shutil.copyfileobj(response, file)

    return path


def subset(source: Path, output: Path, characters: str) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.unlink(missing_ok=True)

    subprocess.run(
        [
            "pyftsubset",
            str(source),
            f"--output-file={output}",
            f"--text={characters}",
            "--layout-features=",
            "--no-hinting",
            "--name-IDs=*",
            "--name-languages=*",
            "--name-legacy",
            "--notdef-outline",
            "--recommended-glyphs",
        ],
        check=True,
    )


def build(output_dir: Path, cache_dir: Path) -> None:
    if shutil.which("pyftsubset") is None:
        raise SystemExit(
            "pyftsubset was not found; install fonttools with "
            "`python3 -m pip install fonttools`"
        )

    for font in FONTS:
        source = download(font, cache_dir)
        name = font_name(font)
        output = output_dir / name

        subset(source, output, font.characters)

        print(
            f"{name}: {source.stat().st_size // 1024} KiB -> "
            f"{output.stat().st_size // 1024} KiB"
        )


def main() -> None:
    font_dir = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(description=__doc__)

    parser.add_argument(
        "--output-dir",
        type=Path,
        default=font_dir,
        help="where subsetted fonts are written",
    )

    args = parser.parse_args()

    build(args.output_dir, font_dir / ".cache")


if __name__ == "__main__":
    main()
