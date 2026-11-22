#!/usr/bin/env python3
"""Check that Android visual values resolve through design/tokens.json."""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOKENS = json.loads((ROOT / "design/tokens.json").read_text())
TOKENS_KT = ROOT / "android/designsystem/src/main/java/org/keynear/designsystem/Tokens.kt"
CONTROL_FILES = sorted(
    (ROOT / "android/designsystem/src/main/java/org/keynear/designsystem").glob("Controls*.kt")
) + [ROOT / "android/designsystem/src/main/java/org/keynear/designsystem/KeynearIcons.kt"]

failures = []


def fail(message):
    failures.append(message)


def argb(value):
    value = value.upper()
    if re.fullmatch(r"#[0-9A-F]{6}", value):
        return "FF" + value[1:]
    if re.fullmatch(r"#[0-9A-F]{8}", value):
        return value[7:9] + value[1:7]
    raise ValueError(f"unsupported token color {value}")


palette_alias = {
    "waveSpoke": ("wave", "spoke"),
    "waveRing": ("wave", "ring"),
    "waveEcho": ("wave", "echo"),
}
kt = TOKENS_KT.read_text()
for theme in ("light", "dark"):
    section = re.search(
        rf"val {'Light' if theme == 'light' else 'Dark'}KeynearPalette = KeynearPalette\((.*?)\n\)",
        kt,
        re.DOTALL,
    )
    if not section:
        fail(f"Android {theme} palette is missing")
        continue
    for field, literal in re.findall(r"(\w+)\s*=\s*Color\((0x[0-9A-Fa-f]{8})\)", section.group(1)):
        group, key = palette_alias.get(field, (theme, field))
        source = TOKENS.get(group, {}).get(key)
        if source is None:
            continue
        expected = argb(source)
        if literal.upper() != f"0x{expected}".upper():
            fail(f"Android {theme} {field}: expected 0x{expected}, found {literal}")

DIMENSION_TOKENS = {
    "Dp2": "dp2",
    "Dp5": "dp5",
    "Dp6": "dp6",
    "Dp10": "dp10",
    "Dp13": "dp13",
    "Dp17": "dp17",
    "Dp18": "dp18",
    "Dp20": "dp20",
    "Dp26": "dp26",
    "Dp34": "dp34",
    "Dp52": "dp52",
    "Dp58": "dp58",
    "Dp62": "dp62",
    "Dp128": "dp128",
}
for name, key in DIMENSION_TOKENS.items():
    expected = TOKENS["dimension"][key]
    if not re.search(rf"\b{name}\s*=\s*{re.escape(str(expected))}\.dp\b", kt):
        fail(f"Android dimension {name}: expected {expected}.dp")

for name, key in (("Micro", "micro"), ("Sp4", "sp4"), ("Sp11", "sp11")):
    expected = TOKENS["typeSizesSp"][key]
    if not re.search(rf"\b{name}\s*=\s*{re.escape(str(expected))}\.sp\b", kt):
        fail(f"Android type size {name}: expected {expected}.sp")

for name, key in (("DISABLED_CONTAINER", "disabledContainer"), ("DISABLED_CONTENT", "disabledContent"), ("WAVE_MOTE", "waveMote"), ("PRESSED_OVERLAY", "pressedOverlay")):
    expected = TOKENS["opacity"][key]
    if not re.search(rf"\b{name}\s*=\s*{re.escape(str(expected))}f\b", kt):
        fail(f"Android opacity {name}: expected {expected}f")

raw_kotlin = re.compile(r"\b(?:[1-9]\d*|\d+\.\d+)\.(?:dp|sp)\b|\b0\.\d+f\b")
for path in CONTROL_FILES:
    for number, line in enumerate(path.read_text().splitlines(), 1):
        if raw_kotlin.search(line):
            fail(f"{path.relative_to(ROOT)}:{number} contains a raw visual literal: {line.strip()}")

font_asset = ROOT / "android/designsystem/src/main/res/font/plus_jakarta_sans.ttf"
if not font_asset.is_file():
    fail("Android font token asset is missing")
theme_kt = (ROOT / "android/designsystem/src/main/java/org/keynear/designsystem/Theme.kt").read_text()
if "plus_jakarta_sans" not in theme_kt:
    fail("Android theme does not reference the bundled font token")
if "NoIndication" in theme_kt:
    fail("Android theme still suppresses focus indication globally")

if failures:
    print(f"token-check: FAIL ({len(failures)} problems)")
    for failure in failures:
        print(f"  - {failure}")
    sys.exit(1)
print("token-check: OK")
