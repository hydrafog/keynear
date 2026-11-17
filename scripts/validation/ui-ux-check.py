#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import re
import signal
import sys
import tempfile
import unittest
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


RULE_CATALOG: dict[str, tuple[str, str]] = {
    "TOK-001": ("tokens", "design/tokens.json schema is missing or invalid"),
    "TOK-002": ("tokens", "Android Tokens.kt color palette drifts from design/tokens.json"),
    "TOK-003": ("tokens", "Android Tokens.kt / Theme.kt scales, motion, or font drift from design/tokens.json"),
    "TOK-004": ("tokens", "GTK4 style.css tokens or scales drift from design/tokens.json"),
    "COL-001": ("colors", "Hardcoded color literal in Android Kotlin source"),
    "COL-002": ("colors", "Hardcoded color literal in GTK4 CSS selector rule"),
    "COL-003": ("colors", "Hardcoded color literal in Rust UI source"),
    "COL-004": ("colors", "Hardcoded hex color literal in Android XML resource"),
    "DIM-001": ("dimensions", "Hardcoded numeric .dp literal in Android Kotlin source"),
    "SHP-001": ("dimensions", "Hardcoded RoundedCornerShape literal in Android Kotlin source"),
    "DIM-002": ("dimensions", "Hardcoded CSS dimension literal in GTK4 CSS selector rule"),
    "DIM-003": ("dimensions", "Hardcoded widget spacing, margin, or size literal in Rust UI source"),
    "DIM-004": ("dimensions", "Hardcoded dp/sp literal in Android XML resource"),
    "TYP-001": ("typography", "Hardcoded numeric .sp literal in Android Kotlin source"),
    "TYP-002": ("typography", "Hardcoded font family, numeric weight, or inline TextStyle in Kotlin"),
    "TYP-003": ("typography", "Hardcoded typography property literal in GTK4 CSS selector rule"),
    "MOT-001": ("motion", "Hardcoded animation spec, duration, or easing in Android Kotlin source"),
    "MOT-002": ("motion", "Hardcoded transition/animation duration or easing in GTK4 CSS selector rule"),
    "OPA-001": ("opacity", "Hardcoded alpha/opacity literal in UI source"),
    "STR-001": ("strings", "Hardcoded user-facing string literal in Android Kotlin UI/notification code"),
    "STR-002": ("strings", "Hardcoded user-facing string attribute in Android XML"),
    "STR-003": ("strings", "Hardcoded user-facing string literal in Rust UI/tray/notification code"),
    "STR-004": ("strings", "Inline CSS string passed to GtkCssProvider instead of stylesheet asset"),
    "A11Y-001": ("accessibility", "Interactive touch target smaller than platform minimum in design/tokens.json"),
    "A11Y-002": ("accessibility", "Icon-only GTK button missing tooltip or accessible label"),
    "SUP-001": ("suppression", "Malformed inline ui-check suppression comment"),
}

ALL_CATEGORIES = sorted({cat for cat, _ in RULE_CATALOG.values()})

CSS_COLOR_NAMES = {
    "aliceblue", "antiquewhite", "aqua", "aquamarine", "azure", "beige", "bisque",
    "black", "blanchedalmond", "blue", "blueviolet", "brown", "burlywood", "cadetblue",
    "chartreuse", "chocolate", "coral", "cornflowerblue", "cornsilk", "crimson", "cyan",
    "darkblue", "darkcyan", "darkgoldenrod", "darkgray", "darkgrey", "darkgreen",
    "darkkhaki", "darkmagenta", "darkolivegreen", "darkorange", "darkorchid", "darkred",
    "darksalmon", "darkseagreen", "darkslateblue", "darkslategray", "darkslategrey",
    "darkturquoise", "darkviolet", "deeppink", "deepskyblue", "dimgray", "dimgrey",
    "dodgerblue", "firebrick", "floralwhite", "forestgreen", "fuchsia", "gainsboro",
    "ghostwhite", "gold", "goldenrod", "gray", "grey", "green", "greenyellow",
    "honeydew", "hotpink", "indianred", "indigo", "ivory", "khaki", "lavender",
    "lavenderblush", "lawngreen", "lemonchiffon", "lightblue", "lightcoral", "lightcyan",
    "lightgoldenrodyellow", "lightgray", "lightgrey", "lightgreen", "lightpink",
    "lightsalmon", "lightseagreen", "lightskyblue", "lightslategray", "lightslategrey",
    "lightsteelblue", "lightyellow", "lime", "limegreen", "linen", "magenta", "maroon",
    "mediumaquamarine", "mediumblue", "mediumorchid", "mediumpurple", "mediumseagreen",
    "mediumslateblue", "mediumspringgreen", "mediumturquoise", "mediumvioletred",
    "midnightblue", "mintcream", "mistyrose", "moccasin", "navajowhite", "navy",
    "oldlace", "olive", "olivedrab", "orange", "orangered", "orchid", "palegoldenrod",
    "palegreen", "paleturquoise", "palevioletred", "papayawhip", "peachpuff", "peru",
    "pink", "plum", "powderblue", "purple", "rebeccapurple", "red", "rosybrown",
    "royalblue", "saddlebrown", "salmon", "sandybrown", "seagreen", "seashell",
    "sienna", "silver", "skyblue", "slateblue", "slategray", "slategrey", "snow",
    "springgreen", "steelblue", "tan", "teal", "thistle", "tomato", "turquoise",
    "violet", "wheat", "white", "whitesmoke", "yellow", "yellowgreen",
}

CSS_LAYOUT_PROPS = {
    "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
    "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
    "border-radius", "border-top-left-radius", "border-top-right-radius",
    "border-bottom-left-radius", "border-bottom-right-radius",
    "min-width", "min-height", "max-width", "max-height", "width", "height",
    "gap", "row-spacing", "column-spacing", "outline", "outline-width",
    "outline-offset", "border", "border-width", "box-shadow",
}

CSS_SPACING_PROPS = {
    "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
    "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
    "gap", "row-spacing", "column-spacing",
}

CSS_TYPOGRAPHY_PROPS = {"font-family", "font-size", "font-weight", "letter-spacing", "line-height"}
CSS_MOTION_PROPS = {
    "transition", "transition-duration", "transition-timing-function", "transition-delay",
    "animation", "animation-duration", "animation-timing-function", "animation-delay",
}

KOTLIN_ANIM_DEBUG_CALLEES = {
    "animateColorAsState",
    "animateDpAsState",
    "animateFloatAsState",
    "animateIntAsState",
    "animateOffsetAsState",
    "animateSizeAsState",
    "animateValueAsState",
    "rememberInfiniteTransition",
    "animateFloat",
    "animateColor",
    "animateValue",
    "updateTransition",
    "AnimatedVisibility",
    "AnimatedContent",
}

KOTLIN_UI_STRING_SINKS: dict[str, set[str] | None] = {
    "Text": {"text"},
    "ActionButton": {"label"},
    "SectionHeading": {"title"},
    "SettingRow": {"title", "subtitle"},
    "SelectionTile": {"label"},
    "RecessedInput": {"placeholder"},
    "PendingPairingBanner": None,
    "showSnackbar": None,
    "onShowSnackbar": None,
    "Toast.makeText": None,
    "setContentTitle": None,
    "setContentText": None,
    "setShortLabel": None,
    "setLongLabel": None,
    "addAction": None,
    "NotificationChannel": None,
}

RUST_UI_STRING_SINKS = {
    "Label::new",
    "Button::with_label",
    "set_label",
    "set_text",
    "set_title",
    "tooltip_text",
    "set_tooltip_text",
    "placeholder_text",
    "set_placeholder_text",
    "selection_tile",
    "switch_row",
    "section_heading",
    "meta_row",
    "nav_tab_button",
}


@dataclass(frozen=True)
class Finding:
    rule_id: str
    category: str
    file_path: str
    line: int
    message: str
    snippet: str


@dataclass
class LineView:
    raw: str
    code: str
    masked: str


@dataclass
class SuppressionResult:
    allowed: dict[int, set[str]]
    malformed: list[tuple[int, str, str]]


@dataclass
class CssDecl:
    selector: str
    prop: str
    value: str
    line: int


def parse_suppressions(lines: list[str], file_path: str) -> SuppressionResult:
    allowed: dict[int, set[str]] = {}
    malformed: list[tuple[int, str, str]] = []
    pattern = re.compile(r"ui-check:\s*allow\(([^)]*)\)(.*)")
    for idx, line in enumerate(lines, start=1):
        match = pattern.search(line)
        if not match:
            continue
        raw_rules = [r.strip() for r in match.group(1).split(",") if r.strip()]
        trailing = match.group(2).strip()
        trailing_cleaned = re.sub(r"(?:\*/|-->)\s*$", "", trailing).strip()
        reason_match = re.match(r"^(?:-|–|:)\s*(.+)$", trailing_cleaned)
        reason = reason_match.group(1).strip() if reason_match else ""
        unknown = [r for r in raw_rules if r not in RULE_CATALOG]
        if not raw_rules or len(reason) < 3 or unknown:
            detail = "missing reason after '-'" if len(reason) < 3 else f"unknown rule(s): {', '.join(unknown)}"
            if not raw_rules:
                detail = "empty allow(...) rule list"
            malformed.append((idx, detail, line.strip()))
            continue
        allowed.setdefault(idx, set()).update(raw_rules)
        allowed.setdefault(idx + 1, set()).update(raw_rules)
    return SuppressionResult(allowed=allowed, malformed=malformed)


def is_suppressed(sup: SuppressionResult, line_no: int, rule_id: str) -> bool:
    return rule_id in sup.allowed.get(line_no, set())


def scan_c_like_lines(text: str, rust_mode: bool = False) -> list[LineView]:
    raw_lines = text.splitlines()
    code_chars: list[list[str]] = [[" " if c != "\t" else " " for c in ln] for ln in raw_lines]
    mask_chars: list[list[str]] = [[" " if c != "\t" else " " for c in ln] for ln in raw_lines]

    n = len(raw_lines)
    block_depth = 0
    in_triple_quote = False
    raw_rust_hashes: int | None = None

    for row in range(n):
        line = raw_lines[row]
        length = len(line)
        col = 0
        while col < length:
            if raw_rust_hashes is not None:
                close_seq = '"' + ("#" * raw_rust_hashes)
                if line.startswith(close_seq, col):
                    for k in range(len(close_seq)):
                        code_chars[row][col + k] = close_seq[k]
                        mask_chars[row][col + k] = '"' if k == 0 else " "
                    col += len(close_seq)
                    raw_rust_hashes = None
                    continue
                code_chars[row][col] = line[col]
                col += 1
                continue

            if in_triple_quote:
                if line.startswith('"""', col):
                    for k in range(3):
                        code_chars[row][col + k] = '"'
                        mask_chars[row][col + k] = '"' if k == 0 else " "
                    col += 3
                    in_triple_quote = False
                    continue
                code_chars[row][col] = line[col]
                col += 1
                continue

            if block_depth > 0:
                if rust_mode and line.startswith("/*", col):
                    block_depth += 1
                    col += 2
                    continue
                if line.startswith("*/", col):
                    block_depth -= 1
                    col += 2
                    continue
                col += 1
                continue

            if line.startswith("//", col):
                break

            if line.startswith("/*", col):
                block_depth = 1
                col += 2
                continue

            if not rust_mode and line.startswith('"""', col):
                for k in range(3):
                    code_chars[row][col + k] = '"'
                    mask_chars[row][col + k] = '"' if k == 0 else " "
                col += 3
                in_triple_quote = True
                continue

            if rust_mode and (col == 0 or not (line[col - 1].isalnum() or line[col - 1] == "_")):
                raw_match = re.match(r'b?r(#*)"', line[col:])
                if raw_match:
                    hashes = len(raw_match.group(1))
                    prefix_len = raw_match.end()
                    for k in range(prefix_len):
                        code_chars[row][col + k] = line[col + k]
                    mask_chars[row][col + prefix_len - 1] = '"'
                    col += prefix_len
                    raw_rust_hashes = hashes
                    continue

            if line[col] == '"':
                code_chars[row][col] = '"'
                mask_chars[row][col] = '"'
                col += 1
                brace_stack: list[int] = []
                while col < length:
                    ch = line[col]
                    code_chars[row][col] = ch
                    if ch == "\\" and col + 1 < length:
                        col += 1
                        code_chars[row][col] = line[col]
                        col += 1
                        continue
                    if not rust_mode and line.startswith("${", col):
                        code_chars[row][col] = "$"
                        code_chars[row][col + 1] = "{"
                        brace_stack.append(1)
                        col += 2
                        continue
                    if brace_stack:
                        if ch == "{":
                            brace_stack[-1] += 1
                        elif ch == "}":
                            brace_stack[-1] -= 1
                            if brace_stack[-1] == 0:
                                brace_stack.pop()
                        col += 1
                        continue
                    if ch == '"':
                        mask_chars[row][col] = '"'
                        col += 1
                        break
                    col += 1
                continue

            if line[col] == "'":
                if col + 2 < length and line[col + 1] != "\\" and line[col + 2] == "'":
                    for k in range(3):
                        code_chars[row][col + k] = line[col + k]
                        mask_chars[row][col + k] = "'" if k in (0, 2) else " "
                    col += 3
                    continue
                if col + 3 < length and line[col + 1] == "\\" and line[col + 3] == "'":
                    for k in range(4):
                        code_chars[row][col + k] = line[col + k]
                        mask_chars[row][col + k] = "'" if k in (0, 3) else " "
                    col += 4
                    continue

            code_chars[row][col] = line[col]
            mask_chars[row][col] = line[col]
            col += 1

    views: list[LineView] = []
    for raw, c, m in zip(raw_lines, code_chars, mask_chars):
        views.append(LineView(raw=raw, code="".join(c), masked="".join(m)))
    return views


def rust_test_line_mask(views: list[LineView]) -> set[int]:
    ignored: set[int] = set()
    in_cfg_test = False
    awaiting_item = False
    brace_depth = 0
    seen_open_brace = False

    for idx, view in enumerate(views, start=1):
        line = view.masked
        if not in_cfg_test and re.search(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", line):
            in_cfg_test = True
            awaiting_item = True
            brace_depth = 0
            seen_open_brace = False
            ignored.add(idx)
            continue
        if in_cfg_test:
            ignored.add(idx)
            if awaiting_item and "{" not in line and ";" in line:
                in_cfg_test = False
                awaiting_item = False
                continue
            for ch in line:
                if ch == "{":
                    brace_depth += 1
                    seen_open_brace = True
                    awaiting_item = False
                elif ch == "}":
                    brace_depth -= 1
            if seen_open_brace and brace_depth <= 0:
                in_cfg_test = False
                seen_open_brace = False
    return ignored


def strip_kotlin_interpolations(content: str) -> str:
    out: list[str] = []
    i = 0
    n = len(content)
    while i < n:
        if content[i] == "\\" and i + 1 < n:
            i += 2
            continue
        if content.startswith("${", i):
            i += 2
            depth = 1
            while i < n and depth > 0:
                if content[i] == '"':
                    i += 1
                    while i < n:
                        if content[i] == "\\" and i + 1 < n:
                            i += 2
                            continue
                        if content[i] == '"':
                            i += 1
                            break
                        i += 1
                    continue
                if content[i] == "{":
                    depth += 1
                elif content[i] == "}":
                    depth -= 1
                i += 1
            continue
        if content[i] == "$" and i + 1 < n and (content[i + 1].isalpha() or content[i + 1] == "_"):
            i += 2
            while i < n and (content[i].isalnum() or content[i] == "_"):
                i += 1
            continue
        out.append(content[i])
        i += 1
    return "".join(out)


def extract_kotlin_string_literals(expr: str) -> list[tuple[str, str]]:
    results: list[tuple[str, str]] = []
    i = 0
    n = len(expr)
    while i < n:
        if expr[i] != '"':
            i += 1
            continue
        start = i
        i += 1
        brace_stack: list[int] = []
        while i < n:
            if expr[i] == "\\" and i + 1 < n:
                i += 2
                continue
            if expr.startswith("${", i):
                brace_stack.append(1)
                i += 2
                continue
            if brace_stack:
                if expr[i] == "{":
                    brace_stack[-1] += 1
                elif expr[i] == "}":
                    brace_stack[-1] -= 1
                    if brace_stack[-1] == 0:
                        brace_stack.pop()
                i += 1
                continue
            if expr[i] == '"':
                raw_inner = expr[start + 1 : i]
                stripped = strip_kotlin_interpolations(raw_inner)
                results.append((raw_inner, stripped))
                i += 1
                break
            i += 1
    return results


def has_human_text(text: str) -> bool:
    letters = re.findall(r"[A-Za-z]+", text)
    if not letters:
        return False
    return any(len(tok) >= 2 for tok in letters)


def iter_calls(views: list[LineView], rust_mode: bool = False) -> Iterable[tuple[str, int, str, list[tuple[str | None, str, int]]]]:
    offsets: list[int] = []
    joined_parts: list[str] = []
    cursor = 0
    for view in views:
        offsets.append(cursor)
        joined_parts.append(view.code)
        cursor += len(view.code) + 1
    full_code = "\n".join(joined_parts)
    full_masked = "\n".join(v.masked for v in views)

    def pos_to_line(pos: int) -> int:
        lo, hi = 0, len(offsets) - 1
        while lo <= hi:
            mid = (lo + hi) // 2
            if offsets[mid] <= pos:
                lo = mid + 1
            else:
                hi = mid - 1
        return hi + 1

    call_pattern = re.compile(r"([A-Za-z_][A-Za-z0-9_:.]*)\s*\(")
    for m in call_pattern.finditer(full_masked):
        name = m.group(1)
        open_paren = m.end() - 1
        line_no = pos_to_line(m.start())
        depth = 0
        end_paren = -1
        i = open_paren
        n = len(full_masked)
        while i < n:
            ch = full_masked[i]
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
                if depth == 0:
                    end_paren = i
                    break
            i += 1
        if end_paren == -1:
            continue
        body_code = full_code[open_paren + 1 : end_paren]
        body_masked = full_masked[open_paren + 1 : end_paren]
        args: list[tuple[str | None, str, int]] = []
        arg_start = 0
        arg_depth = 0
        for idx, ch in enumerate(body_masked):
            if ch in "([{":
                arg_depth += 1
            elif ch in ")]}":
                arg_depth -= 1
            elif ch == "," and arg_depth == 0:
                seg_code = body_code[arg_start:idx]
                seg_masked = body_masked[arg_start:idx]
                arg_line = pos_to_line(open_paren + 1 + arg_start)
                args.append(split_named_arg(seg_code, seg_masked, arg_line))
                arg_start = idx + 1
        tail_code = body_code[arg_start:]
        tail_masked = body_masked[arg_start:]
        if tail_code.strip():
            arg_line = pos_to_line(open_paren + 1 + arg_start)
            args.append(split_named_arg(tail_code, tail_masked, arg_line))
        yield name, line_no, body_code, args


def split_named_arg(seg_code: str, seg_masked: str, line_no: int) -> tuple[str | None, str, int]:
    eq_match = re.match(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(?!=)\s*", seg_masked)
    if eq_match:
        return eq_match.group(1), seg_code[eq_match.end() :].strip(), line_no
    return None, seg_code.strip(), line_no


def normalize_css_hex_to_argb(hex_str: str) -> str:
    h = hex_str.lstrip("#").upper()
    if len(h) == 6:
        return "0xFF" + h
    if len(h) == 8:
        return "0x" + h[6:8] + h[0:6]
    if len(h) == 3:
        return "0xFF" + "".join(c * 2 for c in h)
    return "0x" + h


def camel_to_upper_snake(name: str) -> str:
    s1 = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name)
    return s1.upper()


def parse_tokens_kt_maps(tokens_kt_path: Path) -> tuple[dict[str, str], dict[str, int | float], dict[str, str], dict[str, str]]:
    text = tokens_kt_path.read_text(encoding="utf-8")
    hex_consts: dict[str, str] = {}
    for m in re.finditer(r"const\s+val\s+([A-Z0-9_]+)\s*:\s*Long\s*=\s*(0x[0-9A-Fa-f]+)", text):
        hex_consts[m.group(1)] = "0x" + m.group(2)[2:].upper()

    dim_values: dict[str, int | float] = {}
    int_consts: dict[str, int] = {}
    for m in re.finditer(r"const\s+val\s+([A-Z0-9_]+)\s*=\s*(\d+)", text):
        int_consts[m.group(1)] = int(m.group(2))

    for obj_m in re.finditer(r"\bobject\s+([A-Za-z0-9_]+)\s*\{([^}]*)\}", text, re.DOTALL):
        current_obj = obj_m.group(1)
        body = obj_m.group(2)
        for stmt in re.split(r"[\n;]", body):
            stmt = stmt.strip()
            if not stmt:
                continue
            val_m = re.match(r"^val\s+([A-Za-z0-9_]+)\s*=\s*(.+)$", stmt)
            if val_m:
                prop, rhs = val_m.group(1), val_m.group(2).strip()
                num_m = re.match(r"^\(?(-?\d+(?:\.\d+)?)\)?\.(dp|sp)$", rhs)
                if num_m:
                    num_val: int | float = float(num_m.group(1)) if "." in num_m.group(1) else int(num_m.group(1))
                    dim_values[f"{current_obj}.{prop}"] = num_val
                elif rhs.endswith(".dp"):
                    const_name = rhs[:-3].strip()
                    if const_name in int_consts:
                        dim_values[f"{current_obj}.{prop}"] = int_consts[const_name]
            const_m = re.match(r"^const\s+val\s+([A-Za-z0-9_]+)\s*=\s*(\d[\d_]*)", stmt)
            if const_m:
                dim_values[f"{current_obj}.{const_m.group(1)}"] = int(const_m.group(2).replace("_", ""))

    def extract_palette_assignments(palette_name: str) -> dict[str, str]:
        pat = re.compile(rf"val\s+{palette_name}\s*=\s*KeynearPalette\((.*?)\n\)", re.DOTALL)
        pm = pat.search(text)
        result: dict[str, str] = {}
        if not pm:
            return result
        for am in re.finditer(r"([a-zA-Z0-9_]+)\s*=\s*Color\(([^)]+)\)", pm.group(1)):
            field, arg = am.group(1), am.group(2).strip()
            if arg.startswith("KeynearTokenHex."):
                cname = arg.split(".", 1)[1]
                if cname in hex_consts:
                    result[field] = hex_consts[cname]
            elif arg.startswith("0x"):
                result[field] = "0x" + arg[2:].upper()
        return result

    return hex_consts, dim_values, extract_palette_assignments("LightKeynearPalette"), extract_palette_assignments("DarkKeynearPalette")


def parse_css(text: str) -> tuple[dict[str, str], dict[str, str], list[CssDecl]]:
    raw_lines = text.splitlines()
    cleaned_lines: list[str] = []
    in_comment = False
    for line in raw_lines:
        out: list[str] = []
        i = 0
        n = len(line)
        while i < n:
            if in_comment:
                if line.startswith("*/", i):
                    in_comment = False
                    out.append("  ")
                    i += 2
                else:
                    out.append(" ")
                    i += 1
            elif line.startswith("/*", i):
                in_comment = True
                out.append("  ")
                i += 2
            else:
                out.append(line[i])
                i += 1
        cleaned_lines.append("".join(out))

    define_colors: dict[str, str] = {}
    root_vars: dict[str, str] = {}
    decls: list[CssDecl] = []

    selector_buf: list[str] = []
    current_selector: str | None = None
    decl_buf: list[tuple[int, str]] = []
    brace_depth = 0

    for line_no, line in enumerate(cleaned_lines, start=1):
        stripped = line.strip()
        if brace_depth == 0:
            dc_m = re.match(r"^@define-color\s+([A-Za-z0-9_-]+)\s+([^;]+);", stripped)
            if dc_m:
                define_colors[dc_m.group(1)] = dc_m.group(2).strip()
                continue
        i = 0
        n = len(line)
        while i < n:
            ch = line[i]
            if ch == "{":
                if brace_depth == 0:
                    current_selector = " ".join("".join(selector_buf).split())
                    selector_buf.clear()
                    decl_buf.clear()
                else:
                    decl_buf.append((line_no, ch))
                brace_depth += 1
            elif ch == "}":
                brace_depth = max(0, brace_depth - 1)
                if brace_depth == 0 and current_selector is not None:
                    raw_block = "".join(part for _, part in decl_buf)
                    flush_css_decls(current_selector, decl_buf, root_vars, decls)
                    current_selector = None
                    decl_buf.clear()
                    _ = raw_block
            elif brace_depth == 0:
                selector_buf.append(ch)
            else:
                decl_buf.append((line_no, ch))
            i += 1
        if brace_depth == 0 and selector_buf:
            selector_buf.append(" ")
        elif brace_depth > 0:
            decl_buf.append((line_no, " "))

    return define_colors, root_vars, decls


def flush_css_decls(
    selector: str,
    decl_buf: list[tuple[int, str]],
    root_vars: dict[str, str],
    decls: list[CssDecl],
) -> None:
    cur_chars: list[str] = []
    cur_line = decl_buf[0][0] if decl_buf else 1
    started = False
    for line_no, ch in decl_buf:
        if not started and not ch.isspace():
            cur_line = line_no
            started = True
        if ch == ";":
            stmt = "".join(cur_chars).strip()
            if ":" in stmt:
                prop, val = stmt.split(":", 1)
                prop = prop.strip()
                val = val.strip()
                if prop.startswith("--") and selector == ":root":
                    root_vars[prop] = val
                elif not prop.startswith("--"):
                    decls.append(CssDecl(selector=selector, prop=prop.lower(), value=val, line=cur_line))
            cur_chars.clear()
            started = False
        else:
            cur_chars.append(ch)
    tail = "".join(cur_chars).strip()
    if ":" in tail:
        prop, val = tail.split(":", 1)
        prop = prop.strip()
        val = val.strip()
        if prop.startswith("--") and selector == ":root":
            root_vars[prop] = val
        elif not prop.startswith("--"):
            decls.append(CssDecl(selector=selector, prop=prop.lower(), value=val, line=cur_line))


def add_finding(
    findings: list[Finding],
    sup: SuppressionResult | None,
    rule_id: str,
    file_path: str,
    line: int,
    message: str,
    snippet: str,
) -> None:
    if sup is not None and is_suppressed(sup, line, rule_id):
        return
    category = RULE_CATALOG[rule_id][0]
    findings.append(
        Finding(
            rule_id=rule_id,
            category=category,
            file_path=file_path,
            line=line,
            message=message,
            snippet=snippet.strip()[:160],
        )
    )


def check_suppressions_validity(
    findings: list[Finding],
    sup: SuppressionResult,
    file_path: str,
) -> None:
    for line_no, detail, snippet in sup.malformed:
        findings.append(
            Finding(
                rule_id="SUP-001",
                category="suppression",
                file_path=file_path,
                line=line_no,
                message=f"Invalid ui-check suppression ({detail}); format must be 'ui-check: allow(RULE-ID) - reason'",
                snippet=snippet[:160],
            )
        )


def check_tokens_parity(root: Path, findings: list[Finding]) -> tuple[dict, dict[str, int | float]]:
    tokens_path = root / "design/tokens.json"
    rel_tokens = "design/tokens.json"
    if not tokens_path.is_file():
        add_finding(findings, None, "TOK-001", rel_tokens, 1, "Missing design/tokens.json", rel_tokens)
        return {}, {}

    try:
        tokens = json.loads(tokens_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        add_finding(findings, None, "TOK-001", rel_tokens, exc.lineno, f"Invalid JSON in design/tokens.json: {exc.msg}", str(exc))
        return {}, {}

    required_groups = ["font", "typeScale", "radii", "spacing", "touchTarget", "motion", "brand", "light", "dark"]
    missing_groups = [g for g in required_groups if g not in tokens or not isinstance(tokens[g], dict)]
    if missing_groups:
        add_finding(findings, None, "TOK-001", rel_tokens, 1, f"Missing required token group(s): {', '.join(missing_groups)}", rel_tokens)
        return tokens, {}

    tokens_kt = root / "android/app/src/main/java/org/keynear/app/ui/theme/Tokens.kt"
    theme_kt = root / "android/app/src/main/java/org/keynear/app/ui/theme/Theme.kt"
    dim_values: dict[str, int | float] = {}

    if tokens_kt.is_file():
        rel_kt = "android/app/src/main/java/org/keynear/app/ui/theme/Tokens.kt"
        kt_raw = tokens_kt.read_text(encoding="utf-8")
        kt_lines = kt_raw.splitlines()
        kt_sup = parse_suppressions(kt_lines, rel_kt)
        check_suppressions_validity(findings, kt_sup, rel_kt)
        hex_consts, dim_values, light_pal, dark_pal = parse_tokens_kt_maps(tokens_kt)
        known_hexes = set(hex_consts.values())

        if known_hexes:
            for group in ("light", "dark", "brand"):
                for k, v in tokens[group].items():
                    if not isinstance(v, str) or not v.startswith("#"):
                        continue
                    argb = normalize_css_hex_to_argb(v)
                    if argb not in known_hexes:
                        add_finding(
                            findings,
                            kt_sup,
                            "TOK-002",
                            rel_kt,
                            1,
                            f"Token '{group}.{k}' ({v} -> {argb}) missing from KeynearTokenHex",
                            f"{group}.{k}: {v}",
                        )
        else:
            palette_hexes = {
                "light": set(light_pal.values()),
                "dark": set(dark_pal.values()),
            }
            for mode_name in ("light", "dark"):
                for k, v in tokens[mode_name].items():
                    if not isinstance(v, str) or not v.startswith("#"):
                        continue
                    expected = normalize_css_hex_to_argb(v)
                    if k not in palette_hexes and k not in (light_pal if mode_name == "light" else dark_pal):
                        if expected not in palette_hexes[mode_name]:
                            add_finding(
                                findings,
                                kt_sup,
                                "TOK-002",
                                rel_kt,
                                1,
                                f"Token '{mode_name}.{k}' ({v} -> {expected}) missing from {mode_name.title()}KeynearPalette",
                                f"{mode_name}.{k}: {v}",
                            )

        for mode_name, pal_map in (("light", light_pal), ("dark", dark_pal)):
            for k, v in tokens[mode_name].items():
                if not isinstance(v, str) or not v.startswith("#"):
                    continue
                if k in pal_map:
                    expected = normalize_css_hex_to_argb(v)
                    actual = pal_map[k]
                    if actual != expected:
                        add_finding(
                            findings,
                            kt_sup,
                            "TOK-002",
                            rel_kt,
                            1,
                            f"{mode_name.title()}KeynearPalette.{k} is {actual}, expected {expected} from design/tokens.json",
                            f"{k} = {actual}",
                        )
            for wk, w_field in (("spoke", "waveSpoke"), ("ring", "waveRing"), ("echo", "waveEcho")):
                wv = tokens.get("wave", {}).get(wk)
                if isinstance(wv, str) and wv.startswith("#") and w_field in pal_map:
                    expected = normalize_css_hex_to_argb(wv)
                    if pal_map[w_field] != expected:
                        add_finding(
                            findings,
                            kt_sup,
                            "TOK-002",
                            rel_kt,
                            1,
                            f"{mode_name.title()}KeynearPalette.{w_field} is {pal_map[w_field]}, expected {expected} from wave.{wk}",
                            f"{w_field} = {pal_map[w_field]}",
                        )

        radii_map = {"lg": "KeynearRadii.Lg", "xl": "KeynearRadii.Xl", "2xl": "KeynearRadii.TwoXl", "3xl": "KeynearRadii.ThreeXl"}
        for tk, kt_name in radii_map.items():
            expected_val = tokens["radii"].get(tk)
            if expected_val is not None and dim_values.get(kt_name) != expected_val:
                add_finding(findings, kt_sup, "TOK-003", rel_kt, 1, f"{kt_name} ({dim_values.get(kt_name)}) != tokens.json radii.{tk} ({expected_val})", kt_name)

        spacing_map = {"xs": "KeynearSpacing.Xs", "sm": "KeynearSpacing.Sm", "md": "KeynearSpacing.Md", "lg": "KeynearSpacing.Lg", "xl": "KeynearSpacing.Xl", "2xl": "KeynearSpacing.TwoXl"}
        for tk, kt_name in spacing_map.items():
            expected_val = tokens["spacing"].get(tk)
            if expected_val is not None and dim_values.get(kt_name) != expected_val:
                add_finding(findings, kt_sup, "TOK-003", rel_kt, 1, f"{kt_name} ({dim_values.get(kt_name)}) != tokens.json spacing.{tk} ({expected_val})", kt_name)

        android_touch = tokens["touchTarget"].get("androidDp")
        if android_touch is not None and dim_values.get("KeynearSpacing.MIN_TOUCH_TARGET_DP") != android_touch:
            add_finding(findings, kt_sup, "TOK-003", rel_kt, 1, f"MIN_TOUCH_TARGET_DP != tokens.json touchTarget.androidDp ({android_touch})", "MIN_TOUCH_TARGET_DP")

        type_map = {"title": "KeynearTypeSizes.Title", "heading": "KeynearTypeSizes.Heading", "body": "KeynearTypeSizes.Body", "description": "KeynearTypeSizes.Description", "caption": "KeynearTypeSizes.Caption"}
        for tk, kt_name in type_map.items():
            expected_val = tokens["typeScale"].get(tk, {}).get("sizePx")
            if expected_val is not None and dim_values.get(kt_name) != expected_val:
                add_finding(findings, kt_sup, "TOK-003", rel_kt, 1, f"{kt_name} ({dim_values.get(kt_name)}) != tokens.json typeScale.{tk}.sizePx ({expected_val})", kt_name)
            lh_key = f"KeynearLineHeights.{tk.title()}"
            if lh_key in dim_values:
                expected_lh = round(tokens["typeScale"][tk]["sizePx"] * tokens["typeScale"][tk]["lineHeight"])
                if dim_values[lh_key] != expected_lh:
                    add_finding(findings, kt_sup, "TOK-003", rel_kt, 1, f"{lh_key} ({dim_values[lh_key]}) != rounded typeScale.{tk} lineHeight ({expected_lh})", lh_key)

        motion_map = {
            "settleDurationMs": "KeynearMotion.SETTLE_DURATION_MS",
            "fadeDurationMs": "KeynearMotion.FADE_DURATION_MS",
            "shortDurationMs": "KeynearMotion.SHORT_DURATION_MS",
            "duration200Ms": "KeynearMotion.DURATION_200_MS",
            "duration500Ms": "KeynearMotion.DURATION_500_MS",
        }
        for tk, kt_name in motion_map.items():
            expected_val = tokens["motion"].get(tk)
            if expected_val is not None and (tk in ("settleDurationMs", "fadeDurationMs", "shortDurationMs") or kt_name in dim_values):
                if dim_values.get(kt_name) != expected_val:
                    add_finding(findings, kt_sup, "TOK-003", rel_kt, 1, f"{kt_name} ({dim_values.get(kt_name)}) != tokens.json motion.{tk} ({expected_val})", kt_name)

    if theme_kt.is_file():
        rel_theme = "android/app/src/main/java/org/keynear/app/ui/theme/Theme.kt"
        theme_lines = theme_kt.read_text(encoding="utf-8").splitlines()
        theme_sup = parse_suppressions(theme_lines, rel_theme)
        check_suppressions_validity(findings, theme_sup, rel_theme)
        font_sans = tokens["font"].get("sans", "")
        if font_sans:
            has_font_ref = any(font_sans.lower().replace(" ", "") in ln.lower().replace(" ", "") and "Fallback" not in ln for ln in theme_lines)
            for idx, ln in enumerate(theme_lines, start=1):
                if re.search(r"FontFamily\.(SansSerif|Default|Serif|Monospace)", ln):
                    add_finding(
                        findings,
                        theme_sup,
                        "TOK-003",
                        rel_theme,
                        idx,
                        f"Theme.kt falls back to {ln.strip()} instead of bundling/loading '{font_sans}' from design/tokens.json",
                        ln,
                    )
            _ = has_font_ref

    style_css = root / "crates/keynear-settings/assets/style.css"
    if style_css.is_file():
        rel_css = "crates/keynear-settings/assets/style.css"
        css_text = style_css.read_text(encoding="utf-8")
        css_lines = css_text.splitlines()
        css_sup = parse_suppressions(css_lines, rel_css)
        define_colors, root_vars, decls = parse_css(css_text)
        css_upper = css_text.upper()

        font_sans = tokens["font"].get("sans", "")
        if font_sans and font_sans not in css_text:
            add_finding(findings, css_sup, "TOK-004", rel_css, 1, f"style.css missing font.sans '{font_sans}' from design/tokens.json", font_sans)

        gtk_touch = tokens["touchTarget"].get("gtkPx")
        if gtk_touch is not None and f"{gtk_touch}px" not in css_text:
            add_finding(findings, css_sup, "TOK-004", rel_css, 1, f"style.css missing touchTarget.gtkPx ({gtk_touch}px) from design/tokens.json", f"{gtk_touch}px")

        for mode in ("light", "dark"):
            for k, v in tokens[mode].items():
                if isinstance(v, str) and re.fullmatch(r"#[0-9A-Fa-f]{6}", v):
                    if v.upper() not in css_upper:
                        add_finding(findings, css_sup, "TOK-004", rel_css, 1, f"Token '{mode}.{k}' ({v}) not represented in style.css", f"{mode}.{k}: {v}")

        allowed_radii = {int(v) for v in tokens["radii"].values()} | {0, 9999}
        allowed_sizes = {int(v["sizePx"]) for v in tokens["typeScale"].values()}
        allowed_weights = {int(w) for w in tokens["font"].get("weights", [])}
        allowed_spacing = {int(v) for v in tokens["spacing"].values()} | {0}

        for decl in decls:
            if decl.prop == "border-radius" or (decl.prop.startswith("border-") and decl.prop.endswith("-radius")):
                for px_m in re.finditer(r"(?<![A-Za-z0-9_.-])(\d+(?:\.\d+)?)px\b", decl.value):
                    px_val = float(px_m.group(1))
                    if px_val not in allowed_radii:
                        add_finding(findings, css_sup, "TOK-004", rel_css, decl.line, f"Off-scale border-radius '{px_m.group(0)}' (allowed tokens.json radii: {sorted(allowed_radii)})", f"{decl.prop}: {decl.value}")
            elif decl.prop == "font-size":
                for px_m in re.finditer(r"(?<![A-Za-z0-9_.-])(\d+(?:\.\d+)?)px\b", decl.value):
                    px_val = float(px_m.group(1))
                    if px_val not in allowed_sizes:
                        add_finding(findings, css_sup, "TOK-004", rel_css, decl.line, f"Off-scale font-size '{px_m.group(0)}' (allowed tokens.json typeScale: {sorted(allowed_sizes)})", f"{decl.prop}: {decl.value}")
            elif decl.prop == "font-weight":
                for w_m in re.finditer(r"\b(\d{3})\b", decl.value):
                    w_val = int(w_m.group(1))
                    if allowed_weights and w_val not in allowed_weights:
                        add_finding(findings, css_sup, "TOK-004", rel_css, decl.line, f"Off-scale font-weight '{w_val}' (allowed tokens.json font.weights: {sorted(allowed_weights)})", f"{decl.prop}: {decl.value}")
            elif decl.prop in CSS_SPACING_PROPS:
                for px_m in re.finditer(r"(?<![A-Za-z0-9_.-])-?(\d+(?:\.\d+)?)px\b", decl.value):
                    px_val = float(px_m.group(1))
                    if px_val not in allowed_spacing:
                        add_finding(findings, css_sup, "TOK-004", rel_css, decl.line, f"Off-scale {decl.prop} '{px_m.group(0)}' (allowed tokens.json spacing: {sorted(allowed_spacing)})", f"{decl.prop}: {decl.value}")

        _ = define_colors, root_vars

    return tokens, dim_values


def find_key_line(path: Path, key: str) -> int:
    try:
        for idx, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if f'"{key}"' in line:
                return idx
    except OSError:
        pass
    return 1


def check_css_files(root: Path, tokens: dict, findings: list[Finding]) -> None:
    gtk_touch = int(tokens.get("touchTarget", {}).get("gtkPx", 44))
    for css_path in sorted((root / "crates/keynear-settings/assets").glob("*.css")):
        rel_path = css_path.relative_to(root).as_posix()
        text = css_path.read_text(encoding="utf-8")
        lines = text.splitlines()
        sup = parse_suppressions(lines, rel_path)
        check_suppressions_validity(findings, sup, rel_path)
        _, _, decls = parse_css(text)

        for decl in decls:
            val_no_vars = re.sub(r"var\([^)]*\)", "", decl.value)
            val_no_vars = re.sub(r"@[A-Za-z0-9_-]+", "", val_no_vars)

            has_color = bool(re.search(r"#[0-9A-Fa-f]{3,8}\b|\b(?:rgb|rgba|hsl|hsla)\s*\(", val_no_vars))
            if not has_color:
                words = {w.lower() for w in re.findall(r"\b[A-Za-z]+\b", val_no_vars)}
                if words & CSS_COLOR_NAMES:
                    has_color = True
            if has_color:
                add_finding(findings, sup, "COL-002", rel_path, decl.line, f"Hardcoded color in CSS rule '{decl.selector}' ({decl.prop}); use @define-color or var(--...)", f"{decl.prop}: {decl.value}")

            if decl.prop in CSS_TYPOGRAPHY_PROPS:
                if re.search(r'"[^"]+"|\'[^\']+\'|\b\d+(?:\.\d+)?(?:px|pt|rem|em)\b|\b[1-9]\d{2}\b', val_no_vars):
                    add_finding(findings, sup, "TYP-003", rel_path, decl.line, f"Hardcoded typography '{decl.prop}: {decl.value}' in CSS rule '{decl.selector}'; use var(--...)", f"{decl.prop}: {decl.value}")

            if decl.prop in CSS_MOTION_PROPS:
                if re.search(r"\b\d+(?:\.\d+)?(?:ms|s)\b|cubic-bezier\s*\(|\b(?:ease|ease-in|ease-out|ease-in-out|linear)\b", val_no_vars):
                    add_finding(findings, sup, "MOT-002", rel_path, decl.line, f"Hardcoded motion '{decl.prop}: {decl.value}' in CSS rule '{decl.selector}'; use var(--...)", f"{decl.prop}: {decl.value}")

            if decl.prop == "opacity":
                op_m = re.fullmatch(r"\s*(\d+(?:\.\d+)?)\s*", val_no_vars)
                if op_m and float(op_m.group(1)) not in (0.0, 1.0):
                    add_finding(findings, sup, "OPA-001", rel_path, decl.line, f"Hardcoded opacity '{decl.value}' in CSS rule '{decl.selector}'; use var(--...)", f"{decl.prop}: {decl.value}")

            if decl.prop in CSS_LAYOUT_PROPS:
                for dim_m in re.finditer(r"(?<![A-Za-z0-9_.-])(-?\d+(?:\.\d+)?)(px|pt|rem|em)\b", val_no_vars):
                    if float(dim_m.group(1)) != 0.0:
                        add_finding(findings, sup, "DIM-002", rel_path, decl.line, f"Hardcoded CSS dimension '{dim_m.group(0)}' in '{decl.selector}' ({decl.prop}); use var(--...)", f"{decl.prop}: {decl.value}")
                        break

            if "button" in decl.selector.lower() and decl.prop in ("min-height", "min-width"):
                px_m = re.search(r"(\d+(?:\.\d+)?)px\b", decl.value)
                if px_m and float(px_m.group(1)) < gtk_touch:
                    add_finding(findings, sup, "A11Y-001", rel_path, decl.line, f"Button {decl.prop} ({px_m.group(0)}) below touchTarget.gtkPx ({gtk_touch}px)", f"{decl.selector} -> {decl.prop}: {decl.value}")


def resolve_kt_dimension(expr: str, dim_values: dict[str, int | float]) -> float | None:
    expr = expr.strip()
    if expr in dim_values:
        return float(dim_values[expr])
    m = re.fullmatch(r"\(?(-?\d+(?:\.\d+)?)\)?\.dp", expr)
    if m:
        return float(m.group(1))
    return None


def check_kotlin_files(root: Path, tokens: dict, dim_values: dict[str, int | float], findings: list[Finding]) -> None:
    kotlin_root = root / "android/app/src/main/java/org/keynear/app"
    if not kotlin_root.is_dir():
        return

    android_touch = float(tokens.get("touchTarget", {}).get("androidDp", 48))

    for kt_path in sorted(kotlin_root.rglob("*.kt")):
        rel_path = kt_path.relative_to(root).as_posix()
        text = kt_path.read_text(encoding="utf-8")
        lines = text.splitlines()
        sup = parse_suppressions(lines, rel_path)
        check_suppressions_validity(findings, sup, rel_path)
        views = scan_c_like_lines(text, rust_mode=False)

        is_tokens_kt = rel_path.endswith("ui/theme/Tokens.kt")
        is_theme_kt = rel_path.endswith("ui/theme/Theme.kt")

        if is_tokens_kt:
            for idx, view in enumerate(views, start=1):
                if re.search(r"\bColor\.(?:White|Black|Red|Blue|Green|Yellow|Gray|LightGray|DarkGray|Cyan|Magenta)\b", view.masked):
                    add_finding(findings, sup, "COL-001", rel_path, idx, "Named Compose Color constant in Tokens.kt; use explicit hex color token", view.raw)
            continue

        for idx, view in enumerate(views, start=1):
            masked = view.masked
            stripped = masked.lstrip()
            if stripped.startswith("import ") or stripped.startswith("package "):
                continue

            if re.search(r"\bColor\s*\(\s*(?:0x[0-9A-Fa-f]+|\d)", masked) or re.search(r"\bColor\.(?:White|Black|Red|Blue|Green|Yellow|Gray|LightGray|DarkGray|Cyan|Magenta)\b", masked) or re.search(r"\b(?:AndroidColor|android\.graphics\.Color)\.(?:parseColor|rgb|argb|WHITE|BLACK|RED|BLUE|GREEN|GRAY)\b", masked):
                add_finding(findings, sup, "COL-001", rel_path, idx, "Hardcoded color in Kotlin; use LocalKeynearPalette.current", view.raw)

            for dp_m in re.finditer(r"(?<![A-Za-z0-9_])(?:\(-\s*)?(\d+(?:\.\d+)?)(?:\s*\))?\.dp\b", masked):
                if float(dp_m.group(1)) != 0.0:
                    add_finding(findings, sup, "DIM-001", rel_path, idx, "Hardcoded .dp literal; use KeynearSpacing / KeynearLayout / KeynearRadii / KeynearDimension", view.raw)
                    break

            for sp_m in re.finditer(r"(?<![A-Za-z0-9_])(?:\(-\s*)?(\d+(?:\.\d+)?)(?:\s*\))?\.sp\b", masked):
                if float(sp_m.group(1)) != 0.0:
                    add_finding(findings, sup, "TYP-001", rel_path, idx, "Hardcoded .sp literal; use KeynearTypeSizes", view.raw)
                    break

            if not is_theme_kt:
                if re.search(r"\bFontFamily\.(?:SansSerif|Default|Serif|Monospace|Cursive)\b|\bFontWeight\s*\(\s*\d+\s*\)", masked):
                    add_finding(findings, sup, "TYP-002", rel_path, idx, "Hardcoded FontFamily or numeric FontWeight; use KeynearTypography", view.raw)

            if re.search(r"\bcopy\s*\([^)]*\balpha\s*=\s*\d+(?:\.\d+)?f?\b|\.alpha\s*\(\s*\d+(?:\.\d+)?f?\s*\)", masked) or re.search(r"\bval\s+[A-Za-z0-9_]*(?:Alpha|Opacity)\s*=\s*\d+(?:\.\d+)?f\b", masked):
                if "KeynearOpacity" not in masked:
                    add_finding(findings, sup, "OPA-001", rel_path, idx, "Hardcoded alpha/opacity float literal; use KeynearOpacity", view.raw)

            if not is_theme_kt:
                if re.search(r"\b(?:tween|spring|keyframes|snap)\s*\(", masked):
                    if "KeynearMotion" not in masked:
                        add_finding(findings, sup, "MOT-001", rel_path, idx, "Hardcoded animation spec/duration/easing; centralize in KeynearMotion", view.raw)

        for name, line_no, body_code, args in iter_calls(views, rust_mode=False):
            short_name = name.split(".")[-1]

            if not is_theme_kt and short_name == "TextStyle":
                if not re.search(r"KeynearTypeSizes|KeynearLineHeights|MaterialTheme\.typography", body_code):
                    add_finding(findings, sup, "TYP-002", rel_path, line_no, "Inline TextStyle without design tokens; use MaterialTheme.typography or KeynearTypeSizes", body_code)

            if short_name == "RoundedCornerShape":
                if not all(
                    ("KeynearRadii" in a_val or "KeynearDimension" in a_val or "KeynearSpacing" in a_val or "KeynearShapes" in a_val)
                    for _, a_val, _ in args
                ):
                    add_finding(findings, sup, "SHP-001", rel_path, line_no, "Hardcoded RoundedCornerShape arguments; use KeynearShapes or KeynearRadii", body_code)

            if short_name == "ActionButton":
                for arg_name, arg_val, arg_line in args:
                    if arg_name == "minHeight":
                        resolved = resolve_kt_dimension(arg_val, dim_values)
                        if resolved is not None and resolved < android_touch:
                            add_finding(findings, sup, "A11Y-001", rel_path, arg_line, f"ActionButton minHeight ({resolved:g}.dp) below touchTarget.androidDp ({android_touch:g}.dp)", arg_val)

            if short_name == "TactileSurface":
                has_click = any(a_name == "onClick" for a_name, _, _ in args)
                if has_click:
                    for a_name, a_val, a_line in args:
                        if a_name == "modifier":
                            for sz_m in re.finditer(r"\.(?:size|heightIn|defaultMinSize)\s*\(([^)]+)\)", a_val):
                                for part in sz_m.group(1).split(","):
                                    rhs = part.split("=")[-1].strip()
                                    resolved = resolve_kt_dimension(rhs, dim_values)
                                    if resolved is not None and resolved < android_touch:
                                        add_finding(findings, sup, "A11Y-001", rel_path, a_line, f"Clickable TactileSurface dimension ({resolved:g}.dp) below touchTarget.androidDp ({android_touch:g}.dp)", a_val)

            if name in KOTLIN_UI_STRING_SINKS or short_name in KOTLIN_UI_STRING_SINKS:
                allowed_args = KOTLIN_UI_STRING_SINKS.get(name) or KOTLIN_UI_STRING_SINKS.get(short_name)
                for idx_arg, (arg_name, arg_val, arg_line) in enumerate(args):
                    if allowed_args is not None:
                        if arg_name is not None and arg_name not in allowed_args:
                            continue
                        if arg_name is None and idx_arg > 0:
                            continue
                    if "stringResource" in arg_val or "getString(" in arg_val:
                        continue
                    for raw_str, stripped_str in extract_kotlin_string_literals(arg_val):
                        if has_human_text(stripped_str):
                            add_finding(findings, sup, "STR-001", rel_path, arg_line, f"Hardcoded UI string in {short_name}(...); move to strings.xml", raw_str)

            if short_name not in KOTLIN_ANIM_DEBUG_CALLEES:
                for arg_name, arg_val, arg_line in args:
                    if arg_name == "contentDescription" and "stringResource" not in arg_val and "getString(" not in arg_val:
                        for raw_str, stripped_str in extract_kotlin_string_literals(arg_val):
                            if has_human_text(stripped_str):
                                add_finding(findings, sup, "STR-001", rel_path, arg_line, "Hardcoded contentDescription string; move to strings.xml", raw_str)

        for idx, view in enumerate(views, start=1):
            if re.search(r"\benum\s+class\s+KeynearTab\b", view.masked):
                j = idx - 1
                while j < len(views) and "}" not in views[j].masked:
                    for raw_str, stripped_str in extract_kotlin_string_literals(views[j].code):
                        if has_human_text(stripped_str):
                            add_finding(findings, sup, "STR-001", rel_path, j + 1, "Hardcoded KeynearTab display label; use @StringRes resource ID", raw_str)
                    j += 1


def check_android_xml(root: Path, findings: list[Finding]) -> None:
    xml_paths: list[Path] = []
    manifest = root / "android/app/src/main/AndroidManifest.xml"
    if manifest.is_file():
        xml_paths.append(manifest)
    res_dir = root / "android/app/src/main/res"
    if res_dir.is_dir():
        for p in sorted(res_dir.rglob("*.xml")):
            if p.parent.name.startswith("values") and p.name in {"strings.xml", "colors.xml", "dimens.xml", "attrs.xml"}:
                continue
            if p.parent.name == "xml":
                continue
            xml_paths.append(p)

    string_attrs = {
        "android:text", "android:title", "android:hint", "android:contentDescription",
        "android:label", "android:shortLabel", "android:longLabel", "android:summary",
    }

    for xml_path in xml_paths:
        rel_path = xml_path.relative_to(root).as_posix()
        text = xml_path.read_text(encoding="utf-8")
        lines = text.splitlines()
        sup = parse_suppressions(lines, rel_path)
        check_suppressions_validity(findings, sup, rel_path)

        in_comment = False
        for idx, raw_line in enumerate(lines, start=1):
            line = raw_line
            if in_comment:
                if "-->" in line:
                    line = line.split("-->", 1)[1]
                    in_comment = False
                else:
                    continue
            while "<!--" in line:
                before, after = line.split("<!--", 1)
                if "-->" in after:
                    line = before + " " + after.split("-->", 1)[1]
                else:
                    line = before
                    in_comment = True
                    break

            for attr_m in re.finditer(r'([A-Za-z0-9_:-]+)\s*=\s*"([^"]*)"', line):
                attr_name, attr_val = attr_m.group(1), attr_m.group(2)
                if re.fullmatch(r"#[0-9A-Fa-f]{3,8}", attr_val.strip()):
                    add_finding(findings, sup, "COL-004", rel_path, idx, f"Hardcoded color '{attr_val}' in XML attribute {attr_name}; use @color/...", raw_line)
                dim_m = re.fullmatch(r"\s*(-?\d+(?:\.\d+)?)(dp|sp)\s*", attr_val)
                if dim_m and float(dim_m.group(1)) != 0.0:
                    add_finding(findings, sup, "DIM-004", rel_path, idx, f"Hardcoded dimension '{attr_val}' in XML attribute {attr_name}; use @dimen/...", raw_line)
                if attr_name in string_attrs and attr_val.strip():
                    if not attr_val.startswith("@string/") and not attr_val.startswith("@android:string/") and attr_val != "@null":
                        if has_human_text(attr_val):
                            add_finding(findings, sup, "STR-002", rel_path, idx, f"Hardcoded XML string in {attr_name}; use @string/...", raw_line)


def rust_lucide_exempt_lines(views: list[LineView]) -> set[int]:
    exempt: set[int] = set()
    in_fn = False
    depth = 0
    seen_brace = False
    for idx, view in enumerate(views, start=1):
        if re.search(r"\bfn\s+(?:draw_lucide_path|draw_rounded_rect|lucide_icon)\b", view.masked):
            in_fn = True
            depth = 0
            seen_brace = False
        if in_fn:
            exempt.add(idx)
            for ch in view.masked:
                if ch == "{":
                    depth += 1
                    seen_brace = True
                elif ch == "}":
                    depth -= 1
            if seen_brace and depth <= 0:
                in_fn = False
    return exempt


def check_rust_ui_files(root: Path, tokens: dict, findings: list[Finding]) -> None:
    gtk_touch = int(tokens.get("touchTarget", {}).get("gtkPx", 44))
    settings_src = root / "crates/keynear-settings/src"
    rust_files: list[Path] = []
    if settings_src.is_dir():
        for p in sorted(settings_src.rglob("*.rs")):
            if p.name.endswith("_tests.rs"):
                continue
            rust_files.append(p)
    for extra in (
        root / "crates/keynear-linux/src/tray.rs",
        root / "crates/keynear-linux/src/notifications.rs",
    ):
        if extra.is_file():
            rust_files.append(extra)

    layout_call_re = re.compile(
        r"\.(spacing|set_spacing|row_spacing|column_spacing|margin_start|margin_end|margin_top|margin_bottom|set_margin_start|set_margin_end|set_margin_top|set_margin_bottom|width_request|height_request|content_width|content_height)\s*\(\s*(\d+)\s*\)"
    )

    for rs_path in rust_files:
        rel_path = rs_path.relative_to(root).as_posix()
        is_settings = rel_path.startswith("crates/keynear-settings/")
        text = rs_path.read_text(encoding="utf-8")
        lines = text.splitlines()
        sup = parse_suppressions(lines, rel_path)
        check_suppressions_validity(findings, sup, rel_path)
        views = scan_c_like_lines(text, rust_mode=True)
        test_lines = rust_test_line_mask(views)
        lucide_lines = rust_lucide_exempt_lines(views) if rel_path.endswith("views/mod.rs") else set()

        icon_vars: set[str] = set()
        for idx, view in enumerate(views, start=1):
            if idx in test_lines:
                continue
            masked = view.masked
            code = view.code
            icon_assign = re.match(r"^\s*let\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=.*\b(?:lucide_icon|gtk4::Image|Image::)\b", masked)
            if icon_assign:
                icon_vars.add(icon_assign.group(1))

            if is_settings:
                if idx not in lucide_lines and re.search(r"\b(?:set_source_rgba?|add_color_stop_rgba?|gdk::RGBA::(?:new|parse))\s*\(", masked):
                    add_finding(findings, sup, "COL-003", rel_path, idx, "Hardcoded Cairo/GDK color in Rust UI; derive from GTK style context or CSS tokens", view.raw)

                if re.search(r'"#[0-9A-Fa-f]{3,8}"', code):
                    add_finding(findings, sup, "COL-003", rel_path, idx, "Hardcoded hex color literal in Rust source; read from design/tokens.json or CSS", view.raw)

                if re.search(r"\.load_from_data\s*\(\s*\"", code):
                    add_finding(findings, sup, "STR-004", rel_path, idx, "Inline CSS string passed to CssProvider::load_from_data; load from assets/style.css", view.raw)

                if re.search(r"\.set_opacity\s*\(\s*(\d+(?:\.\d+)?)\s*\)", masked):
                    op_m = re.search(r"\.set_opacity\s*\(\s*(\d+(?:\.\d+)?)\s*\)", masked)
                    if op_m and float(op_m.group(1)) not in (0.0, 1.0):
                        add_finding(findings, sup, "OPA-001", rel_path, idx, "Hardcoded widget opacity literal; use CSS token class", view.raw)

                if idx not in lucide_lines:
                    for lm in layout_call_re.finditer(masked):
                        method, val_str = lm.group(1), lm.group(2)
                        val = int(val_str)
                        if val > 0:
                            add_finding(findings, sup, "DIM-003", rel_path, idx, f"Hardcoded GTK layout dimension .{method}({val}); centralize layout constants or CSS", view.raw)

                    for lm in re.finditer(r"\blucide_icon\s*\([^,]+,\s*(\d+)\s*\)", masked):
                        add_finding(findings, sup, "DIM-003", rel_path, idx, f"Hardcoded icon size ({lm.group(1)}) in lucide_icon(...); use shared icon size constant", view.raw)

                    if (re.search(r"\.(?:arc|set_line_width)\s*\(\s*[^)]*\b\d+(?:\.\d+)?\b", masked) and "TAU" in code) or re.search(r"\b(?:RadialGradient::new|set_line_width)\s*\(", masked):
                        add_finding(findings, sup, "DIM-003", rel_path, idx, "Hardcoded Cairo geometry radius or stroke width; extract named layout constant", view.raw)

                if "Button::builder()" in masked:
                    block_window = " ".join(v.code for v in views[idx - 1 : min(len(views), idx + 10)])
                    stmt_window = block_window.split(";", 1)[0]
                    for bm in re.finditer(r"\.(?:width_request|height_request)\s*\(\s*(\d+)\s*\)", stmt_window):
                        if int(bm.group(1)) < gtk_touch:
                            add_finding(findings, sup, "A11Y-001", rel_path, idx, f"GTK Button dimension ({bm.group(1)}px) below touchTarget.gtkPx ({gtk_touch}px)", view.raw)
                    child_m = re.search(r"\.child\s*\(\s*&\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)", stmt_window)
                    if child_m and child_m.group(1) in icon_vars:
                        if ".tooltip_text(" not in stmt_window and ".label(" not in stmt_window:
                            add_finding(findings, sup, "A11Y-002", rel_path, idx, f"Icon-only gtk4::Button (child &{child_m.group(1)}) missing .tooltip_text(...) for accessibility", view.raw)

        for name, line_no, body_code, args in iter_calls(views, rust_mode=True):
            if line_no in test_lines:
                continue
            if name in RUST_UI_STRING_SINKS or any(name.endswith("." + s) or name.endswith("::" + s) for s in RUST_UI_STRING_SINKS):
                for _, arg_val, arg_line in args:
                    if arg_line in test_lines:
                        continue
                    for lit in re.findall(r'"([^"\\]*(?:\\.[^"\\]*)*)"', arg_val):
                        cleaned = re.sub(r"\{[^}]*\}", "", lit)
                        if has_human_text(cleaned):
                            add_finding(findings, sup, "STR-003", rel_path, arg_line, f"Hardcoded UI string in {name}(...); externalize UI copy", lit)

        if rel_path.endswith("tray.rs"):
            for idx, view in enumerate(views, start=1):
                if idx in test_lines:
                    continue
                if re.search(r"\blabel\s*:\s*\"", view.code):
                    for lit in re.findall(r'label\s*:\s*(?:format!\s*\(\s*)?"([^"\\]*(?:\\.[^"\\]*)*)"', view.code):
                        cleaned = re.sub(r"\{[^}]*\}", "", lit)
                        if has_human_text(cleaned):
                            add_finding(findings, sup, "STR-003", rel_path, idx, "Hardcoded tray menu label string; externalize UI copy", lit)


def run_checks(root: Path, categories: set[str] | None = None, exclude_categories: set[str] | None = None) -> list[Finding]:
    findings: list[Finding] = []
    tokens, dim_values = check_tokens_parity(root, findings)
    check_css_files(root, tokens, findings)
    check_kotlin_files(root, tokens, dim_values, findings)
    check_android_xml(root, findings)
    check_rust_ui_files(root, tokens, findings)

    filtered: list[Finding] = []
    seen: set[tuple[str, str, int, str]] = set()
    for f in findings:
        if categories and f.category not in categories:
            continue
        if exclude_categories and f.category in exclude_categories:
            continue
        key = (f.rule_id, f.file_path, f.line, f.message)
        if key not in seen:
            seen.add(key)
            filtered.append(f)
    filtered.sort(key=lambda f: (f.file_path, f.line, f.rule_id))
    return filtered


class UiUxCheckSelfTests(unittest.TestCase):
    def _make_minimal_repo(self, tmp: Path) -> Path:
        (tmp / "design").mkdir(parents=True)
        tokens = {
            "font": {"sans": "Plus Jakarta Sans", "weights": [400, 500, 600, 700]},
            "typeScale": {
                "title": {"sizePx": 24, "lineHeight": 1.25, "weight": 600},
                "heading": {"sizePx": 20, "lineHeight": 1.3, "weight": 600},
                "body": {"sizePx": 16, "lineHeight": 1.5, "weight": 400},
                "description": {"sizePx": 14, "lineHeight": 1.43, "weight": 400},
                "caption": {"sizePx": 12, "lineHeight": 1.33, "weight": 500},
            },
            "radii": {"lg": 8, "xl": 12, "2xl": 16, "3xl": 24},
            "spacing": {"xs": 4, "sm": 8, "md": 12, "lg": 16, "xl": 24, "2xl": 32},
            "touchTarget": {"androidDp": 48, "gtkPx": 44},
            "motion": {"settleDurationMs": 240, "fadeDurationMs": 180, "shortDurationMs": 150},
            "brand": {"blue600": "#004CED"},
            "light": {"canvas": "#F0F4FA", "scrim": "#00021E73"},
            "dark": {"canvas": "#0B1016", "scrim": "#00000099"},
        }
        (tmp / "design/tokens.json").write_text(json.dumps(tokens), encoding="utf-8")

        theme_dir = tmp / "android/app/src/main/java/org/keynear/app/ui/theme"
        theme_dir.mkdir(parents=True)
        (theme_dir / "Tokens.kt").write_text(
            """
package org.keynear.app.ui.theme
object KeynearTokenHex {
    const val LIGHT_CANVAS: Long = 0xFFF0F4FA
    const val DARK_CANVAS: Long = 0xFF0B1016
    const val ACCENT_BLUE_600: Long = 0xFF004CED
    const val LIGHT_SCRIM: Long = 0x7300021E
    const val DARK_SCRIM: Long = 0x99000000
}
object KeynearRadii { val Lg = 8.dp; val Xl = 12.dp; val TwoXl = 16.dp; val ThreeXl = 24.dp }
object KeynearSpacing { val Xs = 4.dp; val Sm = 8.dp; val Md = 12.dp; val Lg = 16.dp; val Xl = 24.dp; val TwoXl = 32.dp; const val MIN_TOUCH_TARGET_DP = 48 }
object KeynearTypeSizes { val Title = 24.sp; val Heading = 20.sp; val Body = 16.sp; val Description = 14.sp; val Caption = 12.sp }
object KeynearMotion { const val SETTLE_DURATION_MS = 240; const val FADE_DURATION_MS = 180; const val SHORT_DURATION_MS = 150 }
object KeynearDimension { val Dp40 = 40.dp; val Dp48 = 48.dp }
""".strip()
            + "\n",
            encoding="utf-8",
        )
        (theme_dir / "Theme.kt").write_text('val PlusJakarta = "Plus Jakarta Sans"\n', encoding="utf-8")

        css_dir = tmp / "crates/keynear-settings/assets"
        css_dir.mkdir(parents=True)
        (css_dir / "style.css").write_text(
            """:root {
    --font-sans: "Plus Jakarta Sans", sans-serif;
    --touch-min: 44px;
    --light-canvas: #F0F4FA;
    --dark-canvas: #0B1016;
}
window.keynear-window {
    background-color: var(--light-canvas);
    font-family: var(--font-sans);
    min-height: var(--touch-min);
}
""".strip()
            + "\n",
            encoding="utf-8",
        )
        return tmp

    def test_clean_repo_passes(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = self._make_minimal_repo(Path(d))
            findings = run_checks(root)
            self.assertEqual(findings, [])

    def test_kotlin_dynamic_dp_and_anim_debug_labels_do_not_false_positive(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = self._make_minimal_repo(Path(d))
            screen = root / "android/app/src/main/java/org/keynear/app/ui/Sample.kt"
            screen.write_text(
                """
package org.keynear.app.ui
import androidx.compose.ui.unit.dp
// 16.dp in a comment and Color(0xFFFF0000) in a comment
@Composable
fun Sample(amp: Float, radiusDp: Float, angle: Float, resLabel: String) {
    val ampPx = amp.dp.toPx()
    val xOffset = (cos(angle) * radiusDp).dp
    val zeroDp = 0.dp
    val tint by animateColorAsState(targetValue = LocalKeynearPalette.current.accent, label = "navTint")
    val c = Color.Transparent
    val ts = TextStyle(
        color = LocalKeynearPalette.current.textPrimary,
        fontSize = KeynearTypeSizes.Body,
    )
    Text(text = stringResource(R.string.app_name))
    Text(text = "$amp:$radiusDp")
    Log.i("TAG", "Color(0xFF000000) 24.dp hardcoded string")
}
""".strip()
                + "\n",
                encoding="utf-8",
            )
            rs_dir = root / "crates/keynear-settings/src/views"
            rs_dir.mkdir(parents=True)
            (rs_dir / "ok.rs").write_text(
                """
fn build_ok() {
    let icon = lucide_icon(LucideIcon::ArrowLeft, ICON_SIZE);
    let back_btn = gtk4::Button::builder().child(&icon).tooltip_text(back_label).build();
    let row = gtk4::Box::builder().spacing(0).build();
    let row_btn = gtk4::Button::builder().child(&row).build();
}
""".strip()
                + "\n",
                encoding="utf-8",
            )
            findings = run_checks(root)
            self.assertEqual(findings, [])

    def test_direct_palette_tokens_kt_passes_parity(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = self._make_minimal_repo(Path(d))
            tokens_kt = root / "android/app/src/main/java/org/keynear/app/ui/theme/Tokens.kt"
            tokens_kt.write_text(
                """
package org.keynear.app.ui.theme
val LightKeynearPalette = KeynearPalette(
    canvas = Color(0xFFF0F4FA),
    scrim = Color(0x7300021E),
)
val DarkKeynearPalette = KeynearPalette(
    canvas = Color(0xFF0B1016),
    scrim = Color(0x99000000),
)
object KeynearRadii { val Lg = 8.dp; val Xl = 12.dp; val TwoXl = 16.dp; val ThreeXl = 24.dp }
object KeynearSpacing { val Xs = 4.dp; val Sm = 8.dp; val Md = 12.dp; val Lg = 16.dp; val Xl = 24.dp; val TwoXl = 32.dp; const val MIN_TOUCH_TARGET_DP = 48 }
object KeynearTypeSizes { val Title = 24.sp; val Heading = 20.sp; val Body = 16.sp; val Description = 14.sp; val Caption = 12.sp }
object KeynearMotion { const val SETTLE_DURATION_MS = 240; const val FADE_DURATION_MS = 180; const val SHORT_DURATION_MS = 150 }
""".strip()
                + "\n",
                encoding="utf-8",
            )
            findings = run_checks(root)
            self.assertEqual(findings, [])

    def test_catches_hardcoded_kotlin_rust_css_xml_and_suppressions(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = self._make_minimal_repo(Path(d))
            screen = root / "android/app/src/main/java/org/keynear/app/ui/Bad.kt"
            screen.write_text(
                """
package org.keynear.app.ui
@Composable
fun Bad() {
    val bg = Color(0xFF123456)
    val pad = 16.dp
    val sz = 14.sp
    val shape = RoundedCornerShape(12.dp)
    val op = bg.copy(alpha = 0.5f)
    val ff = FontFamily.SansSerif
    val anim = tween(durationMillis = 300)
    // ui-check: allow(DIM-001) - intentional fixture test
    val allowedPad = 8.dp
    // ui-check: allow(DIM-001)
    val badSup = 4.dp
    TactileSurface(modifier = Modifier.size(KeynearDimension.Dp40), onClick = {}) {
        Text(text = "Hardcoded ${1} item${if (true) "" else "s"}")
    }
}
""".strip()
                + "\n",
                encoding="utf-8",
            )
            (root / "crates/keynear-settings/assets/bad.css").write_text(
                """
button.bad {
    background-color: #112233;
    min-height: 32px;
    font-size: 13px;
    transition: opacity 200ms ease;
    opacity: 0.5;
}
""".strip()
                + "\n",
                encoding="utf-8",
            )
            xml_dir = root / "android/app/src/main/res/layout"
            xml_dir.mkdir(parents=True)
            (xml_dir / "item.xml").write_text(
                '<TextView android:text="Hello" android:textColor="#FF0000" android:padding="12dp" />\n',
                encoding="utf-8",
            )
            rs_dir = root / "crates/keynear-settings/src/views"
            rs_dir.mkdir(parents=True)
            (rs_dir / "bad.rs").write_text(
                """
fn build(cr: &cairo::Context) {
    cr.set_source_rgba(1.0, 0.0, 0.0, 0.5);
    let b = gtk4::Box::builder().spacing(12).build();
    let lbl = gtk4::Label::new(Some("Hardcoded Rust"));
    let icon = lucide_icon(LucideIcon::X, 24);
    let ib = gtk4::Button::builder().child(&icon).build();
    let prov = gtk4::CssProvider::new();
    prov.load_from_data("label { color: red; }");
}
#[cfg(test)]
mod tests {
    #[test]
    fn ok() {
        let lbl = gtk4::Label::new(Some("Test only"));
    }
}
""".strip()
                + "\n",
                encoding="utf-8",
            )
            findings = run_checks(root)
            rule_ids = {f.rule_id for f in findings}
            for expected in (
                "COL-001", "COL-002", "COL-003", "COL-004",
                "DIM-001", "SHP-001", "DIM-002", "DIM-003", "DIM-004",
                "TYP-001", "TYP-002", "TYP-003",
                "MOT-001", "MOT-002", "OPA-001",
                "STR-001", "STR-002", "STR-003", "STR-004",
                "A11Y-001", "A11Y-002", "SUP-001",
            ):
                self.assertIn(expected, rule_ids, f"Missing expected rule {expected} in {rule_ids}")
            suppressed_lines = [f for f in findings if f.file_path.endswith("Bad.kt") and f.line == 12]
            self.assertEqual(suppressed_lines, [])
            test_mod_findings = [f for f in findings if f.file_path.endswith("bad.rs") and f.line >= 12]
            self.assertEqual(test_mod_findings, [])


def format_text_report(findings: list[Finding]) -> str:
    if not findings:
        return "ui-ux-check: OK (0 findings)"
    by_cat: dict[str, int] = {}
    for f in findings:
        by_cat[f.category] = by_cat.get(f.category, 0) + 1
    summary = ", ".join(f"{cat}={count}" for cat, count in sorted(by_cat.items()))
    lines = [f"ui-ux-check: FAILED ({len(findings)} findings: {summary})"]
    for f in findings:
        lines.append(f"  [{f.rule_id}] {f.file_path}:{f.line}: {f.message}")
        if f.snippet:
            lines.append(f"    -> {f.snippet}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    if hasattr(signal, "SIGPIPE"):
        signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    parser = argparse.ArgumentParser(description="Check Keynear UI/UX sources for hardcoded values and design-token drift.")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2], help="Repository root directory")
    parser.add_argument("--category", action="append", choices=ALL_CATEGORIES, help="Run/report only selected category (repeatable)")
    parser.add_argument("--exclude-category", action="append", choices=ALL_CATEGORIES, help="Exclude category (repeatable)")
    parser.add_argument("--format", choices=("text", "json"), default="text", help="Output format")
    parser.add_argument("--warn-only", action="store_true", help="Report findings without non-zero exit status")
    parser.add_argument("--list-rules", action="store_true", help="List all rule IDs and exit")
    parser.add_argument("--self-test", action="store_true", help="Run built-in self-tests and exit")
    args = parser.parse_args(argv)

    if args.list_rules:
        for rid, (cat, desc) in sorted(RULE_CATALOG.items()):
            print(f"{rid}\t{cat}\t{desc}")
        return 0

    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(UiUxCheckSelfTests)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        return 0 if result.wasSuccessful() else 1

    categories = set(args.category) if args.category else None
    exclude_categories = set(args.exclude_category) if args.exclude_category else None
    findings = run_checks(args.root.resolve(), categories=categories, exclude_categories=exclude_categories)

    if args.format == "json":
        payload = {
            "ok": len(findings) == 0,
            "total": len(findings),
            "findings": [f.__dict__ for f in findings],
        }
        print(json.dumps(payload, indent=2))
    else:
        print(format_text_report(findings))

    if findings and not args.warn_only:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
