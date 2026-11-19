from __future__ import annotations

import re

EXEMPT_MARKER_RE = re.compile(
    r"(?:\b(?:TODO|FIXME|HACK|OPTIMIZE|cSpell|Copyright)\b|SPDX-License-Identifier|SAFETY:|INVARIANT:|(?:design-lint|ui-check):\s*allow\(|detekt:|@Suppress\b)"
)


def is_exempt_comment(comment_text: str) -> bool:
    return bool(EXEMPT_MARKER_RE.search(comment_text))


def _match_rust_raw_string_start(text: str, idx: int) -> tuple[int, int] | None:
    if idx > 0 and (text[idx - 1].isalnum() or text[idx - 1] == "_"):
        return None
    pos = idx
    if text[pos] == "b":
        pos += 1
    if pos >= len(text) or text[pos] != "r":
        return None
    pos += 1
    hashes = 0
    while pos < len(text) and text[pos] == "#":
        hashes += 1
        pos += 1
    if pos < len(text) and text[pos] == '"':
        return pos + 1 - idx, hashes
    return None


def _consume_rust_raw_string(text: str, pos: int, hashes: int) -> int:
    closing = '"' + ("#" * hashes)
    end = text.find(closing, pos)
    if end == -1:
        return len(text)
    return end + len(closing)


def _consume_triple_quoted_kotlin(text: str, pos: int) -> int:
    end = text.find('"""', pos)
    if end == -1:
        return len(text)
    return end + 3


def _consume_char_literal(text: str, idx: int) -> int | None:
    n = len(text)
    if idx + 1 >= n:
        return None
    if text[idx + 1] == "\\":
        pos = idx + 2
        if pos >= n:
            return None
        if text[pos] == "u" and pos + 1 < n and text[pos + 1] == "{":
            close = text.find("}", pos + 2)
            if close != -1 and close + 1 < n and text[close + 1] == "'":
                return close + 2
            return None
        pos += 1
        while pos < n and pos - idx <= 5:
            if text[pos] == "'":
                return pos + 1
            if text[pos] in ("\n", "\r"):
                return None
            pos += 1
        return None
    if text[idx + 1] not in ("'", "\n", "\r") and idx + 2 < n and text[idx + 2] == "'":
        return idx + 3
    return None


def _consume_quoted_string(text: str, pos: int, rust_mode: bool) -> int:
    n = len(text)
    brace_stack: list[int] = []
    while pos < n:
        ch = text[pos]
        if ch == "\\" and pos + 1 < n:
            pos += 2
            continue
        if not rust_mode and text.startswith("${", pos):
            brace_stack.append(1)
            pos += 2
            continue
        if brace_stack:
            if ch == '"':
                pos += 1
                pos = _consume_quoted_string(text, pos, rust_mode=False)
                continue
            if ch == "{":
                brace_stack[-1] += 1
            elif ch == "}":
                brace_stack[-1] -= 1
                if brace_stack[-1] == 0:
                    brace_stack.pop()
            pos += 1
            continue
        if ch == '"':
            return pos + 1
        if ch == "\n" and not rust_mode and not brace_stack:
            return pos + 1
        pos += 1
    return n


def _find_comment_spans(text: str, rust_mode: bool) -> list[tuple[int, int, str]]:
    spans: list[tuple[int, int, str]] = []
    n = len(text)
    i = 0
    while i < n:
        if rust_mode:
            raw_start = _match_rust_raw_string_start(text, i)
            if raw_start is not None:
                prefix_len, hashes = raw_start
                i = _consume_rust_raw_string(text, i + prefix_len, hashes)
                continue
        elif text.startswith('"""', i):
            i = _consume_triple_quoted_kotlin(text, i + 3)
            continue

        if text[i] == '"':
            i = _consume_quoted_string(text, i + 1, rust_mode=rust_mode)
            continue

        if text[i] == "'":
            if rust_mode and i > 0 and text[i - 1] == "b":
                char_end = _consume_char_literal(text, i)
                if char_end is not None:
                    i = char_end
                    continue
            char_end = _consume_char_literal(text, i)
            if char_end is not None:
                i = char_end
                continue
            i += 1
            continue

        if text.startswith("//", i):
            end = text.find("\n", i)
            if end == -1:
                end = n
            spans.append((i, end, text[i:end]))
            i = end
            continue

        if text.startswith("/*", i):
            start = i
            i += 2
            depth = 1
            while i < n and depth > 0:
                if rust_mode and text.startswith("/*", i):
                    depth += 1
                    i += 2
                elif text.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            spans.append((start, i, text[start:i]))
            continue

        i += 1
    return spans


def strip_comments_from_text(
    text: str,
    rust_mode: bool,
) -> tuple[str, list[tuple[int, str]]]:
    spans = _find_comment_spans(text, rust_mode=rust_mode)
    removed: list[tuple[int, str]] = []
    remove_ranges: list[tuple[int, int]] = []

    line_starts = [0]
    for idx, ch in enumerate(text):
        if ch == "\n":
            line_starts.append(idx + 1)

    def offset_to_line(offset: int) -> int:
        lo, hi = 0, len(line_starts) - 1
        while lo <= hi:
            mid = (lo + hi) // 2
            if line_starts[mid] <= offset:
                lo = mid + 1
            else:
                hi = mid - 1
        return hi + 1

    for start, end, body in spans:
        if is_exempt_comment(body):
            continue
        removed.append((offset_to_line(start), body.splitlines()[0].strip()[:120]))

        line_begin = text.rfind("\n", 0, start) + 1
        next_nl = text.find("\n", end)
        before_on_line = text[line_begin:start]
        after_on_line = text[end : (len(text) if next_nl == -1 else next_nl)]

        if before_on_line.strip() == "" and after_on_line.strip() == "":
            cut_end = (next_nl + 1) if next_nl != -1 else end
            remove_ranges.append((line_begin, cut_end))
        elif after_on_line.strip() == "":
            ws_start = start
            while ws_start > line_begin and text[ws_start - 1] in (" ", "\t"):
                ws_start -= 1
            remove_ranges.append((ws_start, end))
        else:
            cut_end = end
            while cut_end < len(text) and text[cut_end] == " ":
                cut_end += 1
            remove_ranges.append((start, cut_end))

    if not remove_ranges:
        return text, removed

    merged: list[tuple[int, int]] = []
    for r_start, r_end in sorted(remove_ranges):
        if merged and r_start <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], r_end))
        else:
            merged.append((r_start, r_end))

    chunks: list[str] = []
    cursor = 0
    for r_start, r_end in merged:
        chunks.append(text[cursor:r_start])
        cursor = r_end
    chunks.append(text[cursor:])
    return "".join(chunks), removed

