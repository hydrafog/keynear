from __future__ import annotations

import json
import re
from pathlib import Path

from .config import (
    CSS_SPACING_PROPS,
    CssDecl,
    Finding,
    add_finding,
    check_suppressions_validity,
    parse_suppressions,
)


def normalize_css_hex_to_argb(hex_str: str) -> str:
    h = hex_str.lstrip("#").upper()
    if len(h) == 6:
        return "0xFF" + h
    if len(h) == 8:
        return "0x" + h[6:8] + h[0:6]
    if len(h) == 3:
        return "0xFF" + "".join(c * 2 for c in h)
    return "0x" + h


def parse_tokens_kt_maps(
    tokens_kt_path: Path,
) -> tuple[dict[str, str], dict[str, int | float], dict[str, str], dict[str, str]]:
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

    return (
        hex_consts,
        dim_values,
        extract_palette_assignments("LightKeynearPalette"),
        extract_palette_assignments("DarkKeynearPalette"),
    )


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
                    flush_css_decls(current_selector, decl_buf, root_vars, decls)
                    current_selector = None
                    decl_buf.clear()
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


def _resolve_android_theme_file(root: Path, filename: str) -> Path | None:
    candidates = [
        root / f"android/designsystem/src/main/java/org/keynear/designsystem/{filename}",
        root / f"android/app/src/main/java/org/keynear/app/ui/theme/{filename}",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return None


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

    tokens_kt = _resolve_android_theme_file(root, "Tokens.kt")
    theme_kt = _resolve_android_theme_file(root, "Theme.kt")
    dim_values: dict[str, int | float] = {}

    if tokens_kt is not None:
        rel_kt = tokens_kt.relative_to(root).as_posix()
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

    if theme_kt is not None:
        rel_theme = theme_kt.relative_to(root).as_posix()
        theme_lines = theme_kt.read_text(encoding="utf-8").splitlines()
        theme_sup = parse_suppressions(theme_lines, rel_theme)
        check_suppressions_validity(findings, theme_sup, rel_theme)
        font_sans = tokens["font"].get("sans", "")
        if font_sans:
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

