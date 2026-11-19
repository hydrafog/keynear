from __future__ import annotations

import argparse
import signal
import sys
import unittest
from pathlib import Path

from .scanner import strip_comments_from_text

SKIP_DIRS = {"target", "build", ".gradle", ".kotlin", "node_modules", "dist"}


def collect_target_files(root: Path, paths: list[Path] | None = None) -> list[Path]:
    scan_roots = paths if paths else [root / "crates", root / "android"]
    files: list[Path] = []
    for base in scan_roots:
        resolved = base if base.is_absolute() else (root / base)
        if resolved.is_file():
            if resolved.suffix in (".rs", ".kt"):
                files.append(resolved)
            continue
        if not resolved.is_dir():
            continue
        for item in sorted(resolved.rglob("*")):
            if any(part in SKIP_DIRS for part in item.parts):
                continue
            if item.is_file() and item.suffix in (".rs", ".kt"):
                files.append(item)
    return files


class StripCommentsSelfTests(unittest.TestCase):
    def test_preserves_urls_raw_strings_lifetimes_and_allowed_markers(self) -> None:
        rust_src = "\n".join([
            "// SPDX-License-Identifier: GPL-3.0-only",
            "fn demo<'a>(s: &'a str) -> &'static str {",
            '    let url = "https://example.com/*not*/a/comment";',
            '    let raw = r#"// not stripped /* neither */"#;',
            "    // TODO: keep this marker",
            "    // FIXME: keep this too",
            "    // HACK: preserve",
            "    // OPTIMIZE: preserve",
            "    // cSpell:ignore keynear",
            "    // SAFETY: pointer is valid",
            "    // remove regular note",
            "    let x = 1; // remove inline note",
            "    /* outer /* nested */ comment */",
            "    url",
            "}",
            "",
        ])
        out, removed = strip_comments_from_text(rust_src, rust_mode=True)
        self.assertIn("// SPDX-License-Identifier: GPL-3.0-only", out)
        self.assertIn('"https://example.com/*not*/a/comment"', out)
        self.assertIn('r#"// not stripped /* neither */"#', out)
        self.assertIn("fn demo<'a>(s: &'a str) -> &'static str {", out)
        self.assertIn("// TODO: keep this marker", out)
        self.assertIn("// FIXME: keep this too", out)
        self.assertIn("// HACK: preserve", out)
        self.assertIn("// OPTIMIZE: preserve", out)
        self.assertIn("// cSpell:ignore keynear", out)
        self.assertIn("// SAFETY: pointer is valid", out)
        self.assertNotIn("remove regular note", out)
        self.assertNotIn("remove inline note", out)
        self.assertNotIn("nested", out)
        self.assertIn("    let x = 1;\n", out)
        self.assertEqual(len(removed), 3)

    def test_kotlin_triple_quote_and_interpolation(self) -> None:
        kt_src = "\n".join([
            'val q = """// stay ${"/* stay */"}"""',
            'val s = "mime */* and https://ok ${foo}"',
            "/* strip block */",
            "// design-lint: allow(DIM-001) - keep suppression",
            "// ordinary kotlin comment",
            "val y = 2",
            "",
        ])
        out, removed = strip_comments_from_text(kt_src, rust_mode=False)
        self.assertIn('"""// stay ${"/* stay */"}"""', out)
        self.assertIn('"mime */* and https://ok ${foo}"', out)
        self.assertIn("// design-lint: allow(DIM-001) - keep suppression", out)
        self.assertNotIn("strip block", out)
        self.assertNotIn("ordinary kotlin comment", out)
        self.assertEqual(len(removed), 2)


def main(argv: list[str] | None = None) -> int:
    if hasattr(signal, "SIGPIPE"):
        signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    parser = argparse.ArgumentParser(
        description="Strip non-exempt comments from Rust and Kotlin sources while preserving markers and literals."
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[3],
        help="Repository root directory",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Fail non-zero if any non-exempt comments exist without modifying files",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run built-in tokenizer tests and exit",
    )
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        help="Optional file or directory subsets to scan",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(StripCommentsSelfTests)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        return 0 if result.wasSuccessful() else 1

    root = args.root.resolve()
    files = collect_target_files(root, args.paths or None)
    offending: list[tuple[str, int, str]] = []
    modified_count = 0

    for path in files:
        rel_path = path.relative_to(root).as_posix() if path.is_relative_to(root) else str(path)
        original = path.read_text(encoding="utf-8")
        cleaned, removed = strip_comments_from_text(original, rust_mode=(path.suffix == ".rs"))
        if not removed:
            continue
        for line_no, snippet in removed:
            offending.append((rel_path, line_no, snippet))
        if not args.check and cleaned != original:
            path.write_text(cleaned, encoding="utf-8")
            modified_count += 1

    if args.check:
        if offending:
            print(f"strip-comments: FAILED ({len(offending)} non-exempt comment(s))")
            for rel_path, line_no, snippet in offending:
                print(f"  {rel_path}:{line_no}: {snippet}")
            return 1
        print(f"strip-comments: OK ({len(files)} files checked)")
        return 0

    print(f"strip-comments: stripped {len(offending)} comment(s) across {modified_count} file(s)")
    return 0

