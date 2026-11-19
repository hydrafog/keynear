from __future__ import annotations

import argparse
import json
import signal
import sys
import tempfile
import unittest
from pathlib import Path

from .baseline import apply_baseline, build_baseline_counts, format_text_report
from .config import ALL_CATEGORIES, RULE_CATALOG, Finding
from .scanners import check_android_xml, check_css_files, check_kotlin_files, check_rust_ui_files
from .tokens import check_tokens_parity


def run_checks(
    root: Path,
    categories: set[str] | None = None,
    exclude_categories: set[str] | None = None,
) -> list[Finding]:
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


class DesignLintSelfTests(unittest.TestCase):
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

        theme_dir = tmp / "android/designsystem/src/main/java/org/keynear/designsystem"
        theme_dir.mkdir(parents=True)
        (theme_dir / "Tokens.kt").write_text(
            """
package org.keynear.designsystem
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
            self.assertEqual(run_checks(root), [])

    def test_kotlin_dynamic_dp_and_anim_debug_labels_do_not_false_positive(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = self._make_minimal_repo(Path(d))
            app_dir = root / "android/app/src/main/java/org/keynear/app/ui"
            app_dir.mkdir(parents=True)
            (app_dir / "Sample.kt").write_text(
                """
package org.keynear.app.ui
import androidx.compose.ui.unit.dp
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
            self.assertEqual(run_checks(root), [])

    def test_direct_palette_tokens_kt_passes_parity(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = self._make_minimal_repo(Path(d))
            tokens_kt = root / "android/designsystem/src/main/java/org/keynear/designsystem/Tokens.kt"
            tokens_kt.write_text(
                """
package org.keynear.designsystem
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
            self.assertEqual(run_checks(root), [])

    def test_catches_hardcoded_kotlin_rust_css_xml_and_suppressions(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = self._make_minimal_repo(Path(d))
            app_dir = root / "android/app/src/main/java/org/keynear/app/ui"
            app_dir.mkdir(parents=True)
            (app_dir / "Bad.kt").write_text(
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
    // design-lint: allow(DIM-001) - intentional fixture test
    val allowedPad = 8.dp
    // design-lint: allow(DIM-001)
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

    def test_baseline_allows_existing_and_fails_on_regressions(self) -> None:
        base_findings = [
            Finding("DIM-001", "dimensions", "a.kt", 10, "msg", "10.dp"),
            Finding("DIM-001", "dimensions", "a.kt", 20, "msg", "20.dp"),
        ]
        counts = build_baseline_counts(base_findings)
        regs, baselined, resolved = apply_baseline(base_findings, counts)
        self.assertEqual(regs, [])
        self.assertEqual(baselined, 2)
        self.assertEqual(resolved, 0)

        reg_findings = base_findings + [Finding("DIM-001", "dimensions", "a.kt", 30, "msg", "30.dp")]
        regs, baselined, resolved = apply_baseline(reg_findings, counts)
        self.assertEqual(len(regs), 1)
        self.assertEqual(regs[0].line, 30)
        self.assertEqual(baselined, 2)

        fewer_findings = [base_findings[0]]
        regs, baselined, resolved = apply_baseline(fewer_findings, counts)
        self.assertEqual(regs, [])
        self.assertEqual(baselined, 1)
        self.assertEqual(resolved, 1)


def main(argv: list[str] | None = None) -> int:
    if hasattr(signal, "SIGPIPE"):
        signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    parser = argparse.ArgumentParser(description="Check Keynear UI/UX sources for hardcoded values and design-token drift.")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3], help="Repository root directory")
    parser.add_argument("--category", action="append", choices=ALL_CATEGORIES, help="Run/report only selected category (repeatable)")
    parser.add_argument("--exclude-category", action="append", choices=ALL_CATEGORIES, help="Exclude category (repeatable)")
    parser.add_argument("--baseline", type=Path, help="Path to baseline JSON file; only fail on findings exceeding baseline counts")
    parser.add_argument("--update-baseline", type=Path, help="Write current finding counts to baseline JSON file and exit 0")
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
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(DesignLintSelfTests)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        return 0 if result.wasSuccessful() else 1

    categories = set(args.category) if args.category else None
    exclude_categories = set(args.exclude_category) if args.exclude_category else None
    findings = run_checks(args.root.resolve(), categories=categories, exclude_categories=exclude_categories)

    if args.update_baseline:
        baseline_payload = {
            "version": 1,
            "counts": build_baseline_counts(findings),
        }
        args.update_baseline.write_text(json.dumps(baseline_payload, indent=2) + "\n", encoding="utf-8")
        print(f"design-lint: updated baseline {args.update_baseline} ({len(findings)} findings)")
        return 0

    baselined = 0
    resolved = 0
    if args.baseline:
        if not args.baseline.is_file():
            print(f"design-lint: baseline file not found: {args.baseline}", file=sys.stderr)
            return 1
        raw_baseline = json.loads(args.baseline.read_text(encoding="utf-8"))
        baseline_counts = raw_baseline.get("counts", {})
        findings, baselined, resolved = apply_baseline(findings, baseline_counts)

    if args.format == "json":
        payload = {
            "ok": len(findings) == 0,
            "total": len(findings),
            "baselined": baselined,
            "resolved": resolved,
            "findings": [f.__dict__ for f in findings],
        }
        print(json.dumps(payload, indent=2))
    else:
        print(format_text_report(findings, baselined=baselined, resolved=resolved))

    if findings and not args.warn_only:
        return 1
    return 0

