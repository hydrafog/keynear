from __future__ import annotations

import re
from pathlib import Path
from typing import Iterable

from .config import (
    CSS_COLOR_NAMES,
    CSS_LAYOUT_PROPS,
    CSS_MOTION_PROPS,
    CSS_TYPOGRAPHY_PROPS,
    KOTLIN_ANIM_DEBUG_CALLEES,
    KOTLIN_UI_STRING_SINKS,
    RUST_UI_STRING_SINKS,
    Finding,
    LineView,
    add_finding,
    check_suppressions_validity,
    parse_suppressions,
    scan_c_like_lines,
)
from .tokens import parse_css


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


def split_named_arg(seg_code: str, seg_masked: str, line_no: int) -> tuple[str | None, str, int]:
    eq_match = re.match(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(?!=)\s*", seg_masked)
    if eq_match:
        return eq_match.group(1), seg_code[eq_match.end() :].strip(), line_no
    return None, seg_code.strip(), line_no


def iter_calls(
    views: list[LineView],
) -> Iterable[tuple[str, int, str, list[tuple[str | None, str, int]]]]:
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


def resolve_kt_dimension(expr: str, dim_values: dict[str, int | float]) -> float | None:
    expr = expr.strip()
    if expr in dim_values:
        return float(dim_values[expr])
    m = re.fullmatch(r"\(?(-?\d+(?:\.\d+)?)\)?\.dp", expr)
    if m:
        return float(m.group(1))
    return None


def check_css_files(root: Path, tokens: dict, findings: list[Finding]) -> None:
    gtk_touch = int(tokens.get("touchTarget", {}).get("gtkPx", 44))
    assets_dir = root / "crates/keynear-settings/assets"
    if not assets_dir.is_dir():
        return
    for css_path in sorted(assets_dir.glob("*.css")):
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


def check_kotlin_files(
    root: Path,
    tokens: dict,
    dim_values: dict[str, int | float],
    findings: list[Finding],
) -> None:
    kt_roots = [
        root / "android/app/src/main/java",
        root / "android/designsystem/src/main/java",
    ]
    kt_files: list[Path] = []
    for base in kt_roots:
        if base.is_dir():
            for p in sorted(base.rglob("*.kt")):
                rel_p = p.relative_to(root).as_posix()
                if rel_p.endswith("/uniffi/keynear_core/keynear_core.kt"):
                    continue
                kt_files.append(p)

    android_touch = float(tokens.get("touchTarget", {}).get("androidDp", 48))

    for kt_path in kt_files:
        rel_path = kt_path.relative_to(root).as_posix()
        text = kt_path.read_text(encoding="utf-8")
        lines = text.splitlines()
        sup = parse_suppressions(lines, rel_path)
        check_suppressions_validity(findings, sup, rel_path)
        views = scan_c_like_lines(text, rust_mode=False)

        is_tokens_kt = rel_path.endswith("/Tokens.kt")
        is_theme_kt = rel_path.endswith("/Theme.kt")

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

        for name, line_no, body_code, args in iter_calls(views):
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
    for mod in ("app", "designsystem", "core-ffi"):
        manifest = root / f"android/{mod}/src/main/AndroidManifest.xml"
        if manifest.is_file():
            xml_paths.append(manifest)
        res_dir = root / f"android/{mod}/src/main/res"
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
        lucide_lines = (
            rust_lucide_exempt_lines(views)
            if rel_path.endswith("views/components/mod.rs") or rel_path.endswith("views/mod.rs")
            else set()
        )

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

        for name, line_no, body_code, args in iter_calls(views):
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

