# Copyright © 2026 Alberto Villa Osorno.
# SPDX-License-Identifier: MIT
"""Focused practical notebook CLI examples; no server or plotter required."""

import copy
import importlib.util
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

REPO = Path(__file__).resolve().parents[4]
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location(
    "atrament_notebook", REPO / "tools/notebook-cli/application/notebook.py"
)
notebook = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(notebook)


class NotebookCliTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory(dir=REPO / ".temp")
        self.addCleanup(self.dir.cleanup)
        self.root = Path(self.dir.name)
        self.doc = copy.deepcopy(notebook.DEFAULT)
        self.doc["paper"].update(
            {
                "width_mm": 100,
                "height_mm": 140,
                "margin_top_mm": 14,
                "margin_bottom_mm": 10,
                "margin_left_mm": 9,
                "margin_right_mm": 9,
            }
        )
        self.doc["paper"]["date_style"] = "none"
        self.doc["paper"]["page_number_style"] = "none"
        self.doc["pages"] = [
            {
                "date": "",
                "printed": False,
                "text": ["Hola, ñandú. ¿Sí?"],
                "images": [],
            },
            {
                "date": "",
                "printed": True,
                "text": ["Otra hoja."],
                "images": [],
            },
        ]
        self.config = self.root / "notebook.json"
        notebook.write_json(self.config, self.doc)

    def run_cli(self, *args):
        return subprocess.run(
            ["python3", str(REPO / "cli/notebook.py"), *args],
            text=True,
            capture_output=True,
            check=False,
        )

    def test_multi_page_pdf_is_a_real_print_size_pdf(self):
        target = self.root / "notes.pdf"
        run = self.run_cli(
            "render",
            "--config",
            str(self.config),
            "--dpi",
            "120",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        data = target.read_bytes()
        self.assertTrue(data.startswith(b"%PDF-1.4"))
        self.assertIn(b"/Count 2", data)
        self.assertIn(b"/MediaBox [0 0 283.4646 396.8504]", data)
        if shutil.which("pdfinfo"):
            info = subprocess.run(
                ["pdfinfo", str(target)],
                text=True,
                capture_output=True,
                check=True,
            )
            self.assertIn("Pages:           2", info.stdout)

    def test_png_transparency_and_printed_sheet(self):
        target = self.root / "pages"
        run = self.run_cli(
            "render",
            "--config",
            str(self.config),
            "--format",
            "png",
            "--dpi",
            "100",
            "--transparent",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        with Image.open(target / "page-001.png") as first:
            self.assertEqual(first.mode, "RGBA")
            self.assertIn("dpi", first.info)
            for axis_dpi in first.info["dpi"]:
                self.assertAlmostEqual(axis_dpi, 100, delta=0.1)
            self.assertEqual(first.getpixel((0, 0))[3], 0)
            left = notebook.point_mm(9, 100)
            top = notebook.point_mm(14, 100)
            self.assertGreater(first.getpixel((left, top + 12))[3], 0)
        with Image.open(target / "page-002.png") as second:
            self.assertEqual(second.getpixel((left, top + 12))[3], 0)
            self.assertIsNotNone(second.getchannel("A").getbbox())

    def test_deterministic_glyph_variation_and_manual_character_override(self):
        self.doc["handwriting"]["glyphs"]["ñ"] = {
            "height_scale": 1.2,
            "ink": "#551177",
            "rotation_deg": 1,
        }
        first = notebook.render_page(self.doc, 0, 120, self.root)
        second = notebook.render_page(self.doc, 0, 120, self.root)
        self.assertEqual(first.tobytes(), second.tobytes())
        self.doc["handwriting"]["seed"] += 1
        changed = notebook.render_page(self.doc, 0, 120, self.root)
        self.assertNotEqual(first.tobytes(), changed.tobytes())

    def test_calibrate_uses_real_measurements_without_ocr(self):
        out = self.root / "calibrated.json"
        run = self.run_cli(
            "calibrate",
            "--config",
            str(self.config),
            "--glyph",
            "a",
            "--heights",
            "2.0,3.0,4.0",
            "--widths",
            "1.0,1.2,1.4",
            "--out",
            str(out),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        spec = notebook.read_json(out)["handwriting"]["glyphs"]["a"]
        self.assertEqual(spec["height_mm"]["median"], 3)
        self.assertEqual(spec["height_mm"]["mean"], 3)
        self.assertEqual(spec["height_mm"]["minimum"], 2)
        self.assertEqual(spec["height_mm"]["maximum"], 4)
        self.assertAlmostEqual(spec["height_mm"]["stddev"], (2 / 3) ** 0.5)

    def test_guide_places_only_manual_lines(self):
        source = self.root / "capture.png"
        Image.new("RGB", (100, 100), "white").save(source)
        target = self.root / "guides.png"
        run = self.run_cli(
            "guide",
            "--image",
            str(source),
            "--upper",
            "20",
            "--lower",
            "41",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn("21 pixels", run.stdout)
        with Image.open(target) as img:
            self.assertEqual(img.getpixel((99, 20)), (220, 40, 45, 255))
            self.assertEqual(img.getpixel((99, 41)), (50, 100, 225, 255))

    def test_mark_printed_targets_explicit_pages(self):
        out = self.root / "marked.json"
        run = self.run_cli(
            "mark-printed",
            "--config",
            str(self.config),
            "--pages",
            "1-2",
            "--out",
            str(out),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertTrue(
            all(page["printed"] for page in notebook.read_json(out)["pages"])
        )
        with self.assertRaisesRegex(ValueError, "ascending"):
            notebook.parse_pages("2-1", 3)

    def test_fails_closed_on_overflow_no_output(self):
        self.doc["pages"][0]["text"] = ["a" * 1000]
        notebook.write_json(self.config, self.doc, replace=True)
        target = self.root / "overflow.pdf"
        run = self.run_cli(
            "render",
            "--config",
            str(self.config),
            "--dpi",
            "100",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 2)
        self.assertIn("overflows right margin", run.stderr)
        self.assertFalse(target.exists())

    def test_refuses_overwrite_without_explicit_replace(self):
        target = self.root / "notes.pdf"
        target.write_bytes(b"keep-me")
        run = self.run_cli(
            "render",
            "--config",
            str(self.config),
            "--dpi",
            "100",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 2)
        self.assertEqual(target.read_bytes(), b"keep-me")

    def test_relative_font_used_for_header_and_page_number(self):
        (self.root / "font.otf").symlink_to(notebook.DEFAULT_FONT)
        self.doc["handwriting"]["font"] = "font.otf"
        self.doc["paper"]["date_style"] = "plain"
        self.doc["paper"]["page_number_style"] = "bottom-right"
        self.doc["pages"][0]["date"] = "02/11"
        result = notebook.render_page(self.doc, 0, 120, self.root)
        self.assertIsNotNone(result.getchannel("A").getbbox())

    def test_bad_image_shape_yields_readable_refusal(self):
        self.doc["pages"][0]["images"] = ["malformed"]
        with self.assertRaisesRegex(
            TypeError, "image placement must be an object"
        ):
            notebook.render_page(self.doc, 0, 120, self.root)

    def test_extreme_manual_scale_refuses_without_uncaught_overflow(self):
        self.doc["handwriting"]["height_mm"] = 1e308
        with self.assertRaisesRegex(ValueError, "millimeter coordinate"):
            notebook.render_page(self.doc, 0, 120, self.root)

    def test_rejects_unsafe_svg_references(self):
        target = self.root / "remote.svg"
        target.write_text(
            '<svg xmlns="http://www.w3.org/2000/svg">'
            '<image href="https://example.com/a.png" /></svg>'
        )
        with self.assertRaisesRegex(ValueError, "unsupported SVG element"):
            notebook.image_for_path(target, 20, 20)

    @unittest.skipUnless(
        shutil.which("magick"), "optional SVG converter absent"
    )
    def test_svg_overlay_renders(self):
        target = self.root / "star.svg"
        target.write_text(
            '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">'
            '<circle cx="10" cy="10" r="9" fill="red"/></svg>'
        )
        self.doc["pages"][0]["images"] = [
            {
                "path": "star.svg",
                "x_mm": 50,
                "y_mm": 60,
                "width_mm": 10,
                "height_mm": 10,
                "saturation": 0.3,
                "tint_strength": 0.2,
                "tint": "#0088ff",
            }
        ]
        result = notebook.render_page(self.doc, 0, 120, self.root)
        x = notebook.point_mm(55, 120)
        y = notebook.point_mm(65, 120)
        self.assertNotEqual(result.getpixel((x, y))[:3], (255, 255, 255))


if __name__ == "__main__":
    unittest.main()
