from __future__ import annotations

from .config import Finding


def build_baseline_counts(findings: list[Finding]) -> dict[str, dict[str, int]]:
    counts: dict[str, dict[str, int]] = {}
    for f in findings:
        file_map = counts.setdefault(f.file_path, {})
        file_map[f.rule_id] = file_map.get(f.rule_id, 0) + 1
    return {fp: dict(sorted(rules.items())) for fp, rules in sorted(counts.items())}


def apply_baseline(
    findings: list[Finding],
    baseline_counts: dict[str, dict[str, int]],
) -> tuple[list[Finding], int, int]:
    grouped: dict[tuple[str, str], list[Finding]] = {}
    for f in findings:
        grouped.setdefault((f.file_path, f.rule_id), []).append(f)

    regressions: list[Finding] = []
    baselined = 0
    for (fp, rid), items in sorted(grouped.items()):
        allowed = int(baseline_counts.get(fp, {}).get(rid, 0))
        if len(items) > allowed:
            baselined += allowed
            regressions.extend(items[allowed:])
        else:
            baselined += len(items)

    baseline_total = sum(sum(rules.values()) for rules in baseline_counts.values())
    resolved = max(0, baseline_total - baselined)
    regressions.sort(key=lambda f: (f.file_path, f.line, f.rule_id))
    return regressions, baselined, resolved


def format_text_report(findings: list[Finding], baselined: int = 0, resolved: int = 0) -> str:
    if not findings:
        if baselined or resolved:
            return f"design-lint: OK (0 new findings; {baselined} baselined, {resolved} resolved)"
        return "design-lint: OK (0 findings)"
    by_cat: dict[str, int] = {}
    for f in findings:
        by_cat[f.category] = by_cat.get(f.category, 0) + 1
    summary = ", ".join(f"{cat}={count}" for cat, count in sorted(by_cat.items()))
    prefix = "new " if baselined or resolved else ""
    lines = [f"design-lint: FAILED ({len(findings)} {prefix}findings: {summary})"]
    for f in findings:
        lines.append(f"  [{f.rule_id}] {f.file_path}:{f.line}: {f.message}")
        if f.snippet:
            lines.append(f"    -> {f.snippet}")
    return "\n".join(lines)

