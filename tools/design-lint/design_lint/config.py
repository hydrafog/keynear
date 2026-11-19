from __future__ import annotations

import re
from dataclasses import dataclass

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
    "SUP-001": ("suppression", "Malformed inline design-lint suppression comment"),
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
    pattern = re.compile(r"(?:design-lint|ui-check):\s*allow\(([^)]*)\)(.*)")
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
                message=f"Invalid design-lint suppression ({detail}); format must be 'design-lint: allow(RULE-ID) - reason'",
                snippet=snippet[:160],
            )
        )


def scan_c_like_lines(text: str, rust_mode: bool = False) -> list[LineView]:
    raw_lines = text.splitlines()
    code_chars: list[list[str]] = [[" " for _ in ln] for ln in raw_lines]
    mask_chars: list[list[str]] = [[" " for _ in ln] for ln in raw_lines]

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

    return [
        LineView(raw=raw, code="".join(c), masked="".join(m))
        for raw, c, m in zip(raw_lines, code_chars, mask_chars)
    ]

