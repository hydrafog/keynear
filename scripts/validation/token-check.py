#!/usr/bin/env python3
"""Check that GTK and Android visual values resolve through design/tokens.json."""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOKENS = json.loads((ROOT / "design/tokens.json").read_text())
CSS_PATH = ROOT / "crates/keynear-settings/assets/style.css"
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


CSS_COLOR_VAR = {
    "--kn-canvas": "canvas",
    "--kn-text-primary": "textPrimary",
    "--kn-map-ground": "mapGround",
    "--kn-surface": "surface",
    "--kn-surface-tonal": "surfaceTonal",
    "--kn-text-secondary": "textSecondary",
    "--kn-text-muted": "textMuted",
    "--kn-loading": "loading",
    "--kn-loading-hover": "loadingHover",
    "--kn-loading-ink": "loadingInk",
    "--kn-loading-tonal": "loadingTonal",
    "--kn-on-loading-tonal": "onLoadingTonal",
    "--kn-warning-tonal": "warningTonal",
    "--kn-on-warning-tonal": "onWarningTonal",
    "--kn-error-tonal": "errorTonal",
    "--kn-on-danger-tonal": "onDangerTonal",
    "--kn-error-action": "errorAction",
    "--kn-on-danger-action": "onDangerAction",
    "--kn-error-action-hover": "errorActionHover",
    "--kn-error": "error",
    "--kn-accent": "accent",
    "--kn-on-accent": "onAccent",
    "--kn-focus-ring": "focusRing",
    "--kn-input": "input",
    "--kn-switch-track-off": "switchTrackOff",
    "--kn-switch-thumb": "switchThumb",
    "--kn-snackbar": "snackbar",
    "--kn-on-snackbar": "onSnackbar",
    "--kn-sheet-handle": "sheetHandle",
    "--kn-gesture-pill": "gesturePill",
    "--kn-qs-hover": "qsHover",
    "--kn-track-well": "trackWell",
}

CSS_SHADOW_VAR = {f"--kn-shadow-{name}": name for name in TOKENS["elevation"]["gtk"]}

CSS_SCALAR_VAR = {
    "--kn-radius-xl": f"{TOKENS['radii']['xl']}px",
    "--kn-radius-2xl": f"{TOKENS['radii']['2xl']}px",
    "--kn-radius-3xl": f"{TOKENS['radii']['3xl']}px",
    "--kn-radius-md10": f"{TOKENS['radii']['md10']}px",
    "--kn-radius-lg14": f"{TOKENS['radii']['lg14']}px",
    "--kn-radius-pill": f"{TOKENS['radii']['pill']}px",
    "--kn-space-xs": f"{TOKENS['spacing']['xs']}px",
    "--kn-space-sm": f"{TOKENS['spacing']['sm']}px",
    "--kn-space-md": f"{TOKENS['spacing']['md']}px",
    "--kn-space-lg": f"{TOKENS['spacing']['lg']}px",
    "--kn-space-xl": f"{TOKENS['spacing']['xl']}px",
    "--kn-space-2": f"{TOKENS['dimension']['dp2']}px",
    "--kn-space-6": f"{TOKENS['dimension']['dp6']}px",
    "--kn-space-10": f"{TOKENS['dimension']['dp10']}px",
    "--kn-space-14": f"{TOKENS['dimension']['dp14']}px",
    "--kn-size-touch": f"{TOKENS['touchTarget']['androidDp']}px",
    "--kn-size-touch-gtk": f"{TOKENS['touchTarget']['gtkPx']}px",
    "--kn-size-26": f"{TOKENS['dimension']['dp26']}px",
    "--kn-size-34": f"{TOKENS['dimension']['dp34']}px",
    "--kn-size-54": f"{TOKENS['dimension']['dp54']}px",
    "--kn-size-58": f"{TOKENS['dimension']['dp58']}px",
    "--kn-size-128": f"{TOKENS['dimension']['dp128']}px",
    "--kn-text-title": f"{TOKENS['typeSizesSp']['title']}px",
    "--kn-text-body": f"{TOKENS['typeSizesSp']['body']}px",
    "--kn-text-description": f"{TOKENS['typeSizesSp']['description']}px",
    "--kn-text-caption": f"{TOKENS['typeSizesSp']['caption']}px",
    "--kn-text-sp13": f"{TOKENS['typeSizesSp']['sp13']}px",
    "--kn-text-sp11": f"{TOKENS['typeSizesSp']['sp11']}px",
    "--kn-tracking-wide": TOKENS["tracking"]["wide"],
    "--kn-tracking-code": TOKENS["tracking"]["code"],
    "--kn-font-sans": '"Plus Jakarta Sans", sans-serif',
    "--kn-opacity-disabled": str(TOKENS["opacity"]["disabledContainer"]),
    "--kn-focus-width": f"{TOKENS['dimension']['dp2']}px",
    "--kn-focus-offset": f"{TOKENS['dimension']['dp2']}px",
}


def expected_css(var, theme):
    if var in CSS_SHADOW_VAR:
        return TOKENS["elevation"]["gtk"][CSS_SHADOW_VAR[var]][theme]
    if var in CSS_SCALAR_VAR:
        return CSS_SCALAR_VAR[var]
    key = CSS_COLOR_VAR.get(var)
    if key is None:
        return None
    if key == "accent":
        return TOKENS["brand"]["blue600"]
    return TOKENS[theme][key]


def block(text, selector):
    match = re.search(re.escape(selector) + r"\s*\{(.*?)\n\}", text, re.DOTALL)
    return match.group(1) if match else ""


css = CSS_PATH.read_text()
light_block = block(css, "window.keynear-window")
dark_block = block(css, "window.keynear-window.dark")
if not light_block or not dark_block:
    fail("GTK token declaration blocks are missing")

for theme, text in (("light", light_block), ("dark", dark_block)):
    for var, value in re.findall(r"(--kn-[\w-]+):\s*([^;]+);", text):
        expected = expected_css(var, theme)
        if expected is None:
            continue
        actual = value.strip()
        if actual.upper() != expected.upper():
            fail(f"GTK {theme} {var}: expected {expected}, found {actual}")

body_start = css.find("\n}\n", css.find("window.keynear-window.dark")) + 3
body = css[body_start:]
raw = re.findall(r"#[0-9A-Fa-f]{6,8}|rgba?\([^)]*\)|[0-9]+(?:\.[0-9]+)?(?:px|em)", body)
if raw:
    fail(f"GTK body contains {len(raw)} raw style literals outside the token block: {sorted(set(raw))[:8]}")
defined_vars = set(re.findall(r"(--kn-[\w-]+)\s*:", light_block + dark_block))
used_vars = set(re.findall(r"var\((--kn-[\w-]+)\)", body))
undefined_vars = used_vars - defined_vars
if undefined_vars:
    fail(f"GTK body references undefined token variables: {sorted(undefined_vars)}")
if css.count("{") != css.count("}"):
    fail("GTK stylesheet has unbalanced braces")

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
