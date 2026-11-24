#!/usr/bin/env python3
"""Check that design tokens resolve through the Android and CSS consumers."""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOKENS = json.loads((ROOT / "design/tokens.json").read_text())
CSS = ROOT / "design/prototype/src/design-system/tokens.css"
SCALES_CSS = ROOT / "design/prototype/src/design-system/scales.css"
TOKENS_KT = ROOT / "android/designsystem/src/main/java/org/keynear/designsystem/Tokens.kt"
CONTROL_FILES = sorted(
    (ROOT / "android/designsystem/src/main/java/org/keynear/designsystem").glob("Controls*.kt")
) + [ROOT / "android/designsystem/src/main/java/org/keynear/designsystem/KeynearIcons.kt"]

TRACKING = [("wide", "--tracking-kn-wider"), ("code", "--tracking-kn-code")]
LINE_HEIGHTS = [
    ("caption", "--text-kn-xs--line-height"),
    ("description", "--text-kn-sm--line-height"),
    ("body", "--text-kn-base--line-height"),
    ("heading", "--text-kn-xl--line-height"),
    ("title", "--text-kn-2xl--line-height"),
]
DEPTH_SHADOWS = [
    ("surface", "--kn-depth-surface"),
    ("tonal", "--kn-depth-tonal"),
    ("accent", "--kn-brand-depth-accent"),
    ("sheet", "--kn-depth-sheet"),
    ("thumb", "--kn-depth-thumb"),
]
CSS_COLOR_KEYS = [
    ("backdrop", "--kn-backdrop"),
    ("canvas", "--kn-canvas"),
    ("onCanvas", "--kn-on-canvas"),
    ("mapGround", "--kn-map-ground"),
    ("surface", "--kn-surface"),
    ("surfaceTonal", "--kn-surface-tonal"),
    ("surfaceHover", "--kn-surface-hover"),
    ("textPrimary", "--kn-text-primary"),
    ("textSecondary", "--kn-text-secondary"),
    ("textMuted", "--kn-text-muted"),
    ("warningTonal", "--kn-warning-tonal"),
    ("onWarningTonal", "--kn-on-warning-tonal"),
    ("error", "--kn-error"),
    ("errorTonal", "--kn-error-tonal"),
    ("onDangerTonal", "--kn-on-danger-tonal"),
    ("errorAction", "--kn-error-action"),
    ("onDangerAction", "--kn-on-danger-action"),
    ("errorActionHover", "--kn-error-action-hover"),
    ("errorInk", "--kn-error-ink"),
    ("input", "--kn-input"),
    ("switchTrackOff", "--kn-switch-track-off"),
    ("switchThumb", "--kn-switch-thumb"),
    ("notifSubCard", "--kn-notif-sub-card"),
    ("qsOn", "--kn-qs-on"),
    ("qsHover", "--kn-qs-hover"),
    ("snackbar", "--kn-snackbar"),
    ("onSnackbar", "--kn-on-snackbar"),
    ("trackWell", "--kn-track-well"),
    ("sheetHandle", "--kn-sheet-handle"),
    ("gesturePill", "--kn-gesture-pill"),
    ("onAccent", "--kn-on-accent"),
    ("accent", "--kn-accent-fill"),
]
MOTION_PARITY = [
    ("shortDurationMs", "--kn-duration-150", "SHORT_DURATION_MS"),
    ("pressDurationMs", "--kn-duration-press", "PRESS_DURATION_MS"),
    ("duration200Ms", "--kn-duration-200", "FADE_IN_DURATION_MS"),
    ("morphDurationMs", "--kn-duration-morph", "MORPH_DURATION_MS"),
    ("switchDurationMs", "--kn-duration-switch", "SWITCH_DURATION_MS"),
    ("duration500Ms", "--kn-duration-500", "DURATION_500_MS"),
]
PRESS_SCALE_PARITY = [
    ("press95", "--kn-press-95", "PRESS_95"),
    ("press97", "--kn-press-97", "PRESS_97"),
    ("press98", "--kn-press-98", "PRESS_98"),
    ("press99", "--kn-press-99", "PRESS_99"),
]

failures = []


def fail(message):
    failures.append(message)


def read_text(path):
    return path.read_text() if path.is_file() else ""


def nested(data, *keys):
    for key in keys:
        if not isinstance(data, dict) or key not in data:
            fail(f"missing token path {'.'.join(str(key) for key in keys)}")
            return None
        data = data[key]
    return data


def walk(value, path, leaves):
    if isinstance(value, dict):
        for key, child in value.items():
            walk(child, f"{path}.{key}" if path else key, leaves)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            walk(child, f"{path}[{index}]", leaves)
    else:
        leaves[path] = value


def check_leaves():
    leaves = {}
    walk(TOKENS, "", leaves)
    for path, value in leaves.items():
        if value is None or (isinstance(value, str) and not value.strip()):
            fail(f"empty token value at {path}")
    print(f"token-check: validated {len(leaves)} token values")


def css_properties(path=CSS):
    props = {}
    for name, value in re.findall(r"(--[\w-]+)\s*:\s*([^;]+);", read_text(path)):
        props.setdefault(name, []).append(value.strip())
    return props


def css_value(props, name, index=0):
    values = props.get(name, [])
    if len(values) <= index:
        fail(f"CSS token {name}[{index}] is missing; token is one-sided")
        return None
    return values[index]


def line_height(value):
    if not value:
        return None
    match = re.fullmatch(r"calc\(\s*([\d.]+)\s*/\s*([\d.]+)\s*\)", value)
    if match:
        return round(float(match.group(1)) / float(match.group(2)), 2)
    try:
        return round(float(value), 2)
    except ValueError:
        fail(f"cannot parse CSS line height {value!r}")
        return None


def shadow_lengths(layer):
    lengths = []
    for token in layer.split():
        if re.fullmatch(r"-?[\d.]+px", token):
            lengths.append(float(token[:-2]))
        elif token == "0":
            lengths.append(0.0)
    return lengths


def outer_blurs(value):
    if not value:
        return []
    blurs = []
    for layer in value.split(","):
        if "inset" in layer:
            continue
        lengths = shadow_lengths(layer)
        if len(lengths) > 2:
            blurs.append(lengths[2])
    return blurs


def check_css_mirrors():
    props = css_properties(CSS)
    scales_props = css_properties(SCALES_CSS)
    for key, prop in TRACKING:
        expected = css_value(props, prop)
        actual = nested(TOKENS, "tracking", key)
        if expected is not None and actual is not None and actual != expected:
            fail(f"tracking.{key}: JSON {actual} diverges from CSS {expected}")

    for step, prop in LINE_HEIGHTS:
        expected = line_height(css_value(props, prop))
        actual = nested(TOKENS, "typeScale", step, "lineHeight")
        if expected is not None and actual is not None and actual != expected:
            fail(f"typeScale.{step}.lineHeight: JSON {actual} diverges from CSS {expected}")

    for family, prop in DEPTH_SHADOWS:
        for theme, index in (("light", 0), ("dark", 1)):
            expected = outer_blurs(css_value(props, prop, index))
            layers = nested(TOKENS, "elevation", "outer", family, theme)
            if expected and isinstance(layers, list):
                actual = [layer.get("blurRadius") for layer in layers]
                if actual != expected:
                    fail(f"elevation.outer.{family}.{theme} blurRadius: JSON {actual} diverges from CSS {expected}")

    for key, prop in CSS_COLOR_KEYS:
        for theme, index in (("light", 0), ("dark", 1)):
            expected = css_value(props, prop, index)
            actual = nested(TOKENS, theme, key)
            if expected is not None and actual is not None and expected.upper() != actual.upper():
                fail(f"{theme}.{key}: JSON {actual} diverges from CSS {prop} ({expected})")

    for key, prop, _ in MOTION_PARITY:
        expected = css_value(props, prop)
        actual = nested(TOKENS, "motion", key)
        if expected is not None and actual is not None and expected != f"{actual}ms":
            fail(f"motion.{key}: JSON {actual}ms diverges from CSS {prop} ({expected})")

    for key, prop, _ in PRESS_SCALE_PARITY:
        expected = css_value(scales_props, prop)
        actual = nested(TOKENS, "pressScale", key)
        if expected is not None and actual is not None and float(expected) != float(actual):
            fail(f"pressScale.{key}: JSON {actual} diverges from scales.css {prop} ({expected})")


def check_one_sided():
    listed = TOKENS.get("oneSidedFamilies", [])
    if not isinstance(listed, list) or not listed:
        fail("oneSidedFamilies must list the one-sided token families")
        return
    mirrored = {"tracking", "typeScale", "elevation", "light", "dark", "motion", "pressScale"}
    for family in listed:
        if family not in TOKENS:
            fail(f"oneSidedFamilies names missing family {family}")
        if family in mirrored:
            fail(f"oneSidedFamilies marks mirrored family {family}")


def argb(value):
    value = value.upper()
    if re.fullmatch(r"#[0-9A-F]{6}", value):
        return "FF" + value[1:]
    if re.fullmatch(r"#[0-9A-F]{8}", value):
        return value[7:9] + value[1:7]
    raise ValueError(f"unsupported token color {value}")


def check_android_palette(kt):
    for theme in ("light", "dark"):
        name = "Light" if theme == "light" else "Dark"
        section = re.search(rf"val {name}KeynearPalette = KeynearPalette\((.*?)\n\)", kt, re.DOTALL)
        if not section:
            fail(f"Android {theme} palette is missing")
            continue
        for field, literal in re.findall(r"(\w+)\s*=\s*Color\((0x[0-9A-Fa-f]{8})\)", section.group(1)):
            source = TOKENS.get(theme, {}).get(field) if isinstance(TOKENS.get(theme), dict) else None
            if source is None:
                continue
            expected = argb(source)
            if literal.upper() != f"0x{expected}".upper():
                fail(f"Android {theme} {field}: expected 0x{expected}, found {literal}")


def check_android_numbers(kt):
    dimensions = {
        "Dp2": "dp2", "Dp4": "dp4", "Dp5": "dp5", "Dp6": "dp6", "Dp10": "dp10", "Dp11": "dp11",
        "Dp12": "dp12", "Dp13": "dp13", "Dp16": "dp16", "Dp17": "dp17", "Dp18": "dp18",
        "Dp20": "dp20", "Dp24": "dp24", "Dp26": "dp26", "Dp32": "dp32", "Dp34": "dp34",
        "Dp46": "dp46", "Dp48": "dp48", "Dp52": "dp52", "Dp58": "dp58", "Dp62": "dp62",
        "Dp128": "dp128", "Dp240": "dp240", "Dp290": "dp290",
    }
    for name, key in dimensions.items():
        expected = nested(TOKENS, "dimension", key)
        if expected is not None and not re.search(rf"\b{name}\s*=\s*{re.escape(str(expected))}\.dp\b", kt):
            fail(f"Android dimension {name}: expected {expected}.dp")

    for name, key in (("Micro", "micro"), ("Sp4", "sp4"), ("Sp11", "sp11")):
        expected = nested(TOKENS, "typeSizesSp", key)
        if expected is not None and not re.search(rf"\b{name}\s*=\s*{re.escape(str(expected))}\.sp\b", kt):
            fail(f"Android type size {name}: expected {expected}.sp")

    for name, key in (("DISABLED_CONTAINER", "disabledContainer"), ("DISABLED_CONTENT", "disabledContent"),
                      ("WAVE_MOTE", "waveMote"), ("PRESSED_OVERLAY", "pressedOverlay")):
        expected = nested(TOKENS, "opacity", key)
        if expected is not None and not re.search(rf"\b{name}\s*=\s*{re.escape(str(expected))}f\b", kt):
            fail(f"Android opacity {name}: expected {expected}f")

    for json_key, _, kt_name in MOTION_PARITY:
        expected = nested(TOKENS, "motion", json_key)
        if expected is not None and not re.search(rf"\b{kt_name}\s*=\s*{re.escape(str(expected))}\b", kt):
            fail(f"Android motion {kt_name}: expected {expected}")

    for json_key, _, kt_name in PRESS_SCALE_PARITY:
        expected = nested(TOKENS, "pressScale", json_key)
        if expected is not None and not re.search(rf"\b{kt_name}\s*=\s*{re.escape(str(expected))}f\b", kt):
            fail(f"Android pressScale {kt_name}: expected {expected}f")


def check_android_consumers():
    kt = read_text(TOKENS_KT)
    check_android_palette(kt)
    check_android_numbers(kt)
    raw_kotlin = re.compile(r"\b(?:[1-9]\d*|\d+\.\d+)\.(?:dp|sp)\b|\b0\.\d+f\b")
    for path in CONTROL_FILES:
        for number, line in enumerate(read_text(path).splitlines(), 1):
            if raw_kotlin.search(line):
                fail(f"{path.relative_to(ROOT)}:{number} contains a raw visual literal: {line.strip()}")

    font_asset = ROOT / "android/designsystem/src/main/res/font/plus_jakarta_sans.ttf"
    if not font_asset.is_file():
        fail("Android font token asset is missing")
    theme_kt = read_text(ROOT / "android/designsystem/src/main/java/org/keynear/designsystem/Theme.kt")
    if "plus_jakarta_sans" not in theme_kt:
        fail("Android theme does not reference the bundled font token")
    if "NoIndication" in theme_kt:
        fail("Android theme still suppresses focus indication globally")


def main():
    check_leaves()
    check_css_mirrors()
    check_one_sided()
    check_android_consumers()
    if failures:
        print(f"token-check: FAIL ({len(failures)} problems)")
        for failure in failures:
            print(f"  - {failure}")
        sys.exit(1)
    print("token-check: OK")


if __name__ == "__main__":
    main()
