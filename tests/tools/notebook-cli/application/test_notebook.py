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

    def test_set_paper_configures_physical_grid_and_margins(self):
        target = self.root / "grid.json"
        run = self.run_cli(
            "set-paper",
            "--config",
            str(self.config),
            "--style",
            "grid",
            "--width-mm",
            "105",
            "--height-mm",
            "148",
            "--spacing-mm",
            "5",
            "--margin-top-mm",
            "19",
            "--margin-bottom-mm",
            "13",
            "--margin-left-mm",
            "11",
            "--margin-right-mm",
            "12",
            "--corner-radius-mm",
            "3",
            "--date-style",
            "underline",
            "--date-side",
            "left",
            "--date-y-mm",
            "5",
            "--line-color",
            "#123abc",
            "--no-draw-border",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        paper = notebook.read_json(target)["paper"]
        self.assertEqual(paper["style"], "grid")
        self.assertEqual(paper["width_mm"], 105)
        self.assertEqual(paper["margin_bottom_mm"], 13)
        self.assertEqual(paper["spacing_mm"], 5)
        self.assertEqual(paper["date_style"], "underline")
        self.assertEqual(paper["line_color"], "#123abc")
        self.assertFalse(paper["draw_border"])
        self.doc["paper"] = paper
        image = notebook.render_page(self.doc, 0, 100, self.root)
        self.assertEqual(image.size, (413, 583))
        x = notebook.point_mm(11, 100) + 4 * notebook.point_mm(5, 100)
        y = notebook.point_mm(19, 100) + 10 * notebook.point_mm(5, 100)
        self.assertEqual(image.getpixel((x, y))[:3], (0x12, 0x3A, 0xBC))

    def test_set_paper_rejects_invalid_or_missing_changes_without_output(self):
        for extra in ((), ("--spacing-mm", "0"), ("--margin-left-mm", "95")):
            with self.subTest(options=extra):
                target = self.root / "invalid.json"
                run = self.run_cli(
                    "set-paper",
                    "--config",
                    str(self.config),
                    *extra,
                    "--out",
                    str(target),
                )
                self.assertEqual(run.returncode, 2, run.stderr)
                self.assertFalse(target.exists())

    def test_set_handwriting_updates_ink_and_global_distributions(self):
        target = self.root / "custom-handwriting.json"
        run = self.run_cli(
            "set-handwriting",
            "--config",
            str(self.config),
            "--ink",
            "#3467ab",
            "--seed",
            "137",
            "--texture",
            "0.4",
            "--line-height-mm",
            "6.5",
            "--letter-spacing-mm",
            "0.2",
            "--height-mm-mean",
            "3.7",
            "--height-mm-median",
            "3.6",
            "--height-mm-stddev",
            "0.2",
            "--width-scale-mean",
            "1.02",
            "--baseline-mm-distribution",
            "triangular",
            "--rotation-deg-minimum",
            "-4",
            "--rotation-deg-maximum",
            "4",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        result = notebook.read_json(target)
        hw = result["handwriting"]
        self.assertEqual(hw["ink"], "#3467ab")
        self.assertEqual(hw["seed"], 137)
        self.assertEqual(hw["height_mm"]["mean"], 3.7)
        self.assertEqual(hw["height_mm"]["median"], 3.6)
        self.assertEqual(hw["width_scale"]["mean"], 1.02)
        self.assertEqual(hw["rotation_deg"]["minimum"], -4)
        self.assertEqual(hw["rotation_deg"]["maximum"], 4)
        self.assertEqual(hw["baseline_mm"]["distribution"], "triangular")
        self.assertEqual(self.doc["handwriting"]["height_mm"]["mean"], 3.5)
        self.assertIsNotNone(
            notebook.render_page(result, 0, 100, self.root).getbbox()
        )

    def test_set_handwriting_rejects_invalid_profiles_before_write(self):
        invalid = (
            (),
            ("--height-mm-minimum", "5"),
            ("--height-mm-stddev", "100"),
            ("--ink", "purple"),
            ("--stroke-px", "12"),
            ("--width-scale-mean", "7"),
        )
        for options in invalid:
            with self.subTest(options=options):
                target = self.root / "bad-handwriting.json"
                run = self.run_cli(
                    "set-handwriting",
                    "--config",
                    str(self.config),
                    *options,
                    "--out",
                    str(target),
                )
                self.assertEqual(run.returncode, 2, run.stderr)
                self.assertFalse(target.exists())

    def test_add_image_with_position_saturation_tint_and_opacity(self):
        image_path = self.root / "photo.png"
        Image.new("RGBA", (8, 8), (200, 30, 50, 128)).save(image_path)
        target = self.root / "with-image.json"
        run = self.run_cli(
            "add-image",
            "--config",
            str(self.config),
            "--page",
            "2",
            "--image",
            "photo.png",
            "--x-mm",
            "20",
            "--y-mm",
            "30",
            "--width-mm",
            "25",
            "--height-mm",
            "14",
            "--layer",
            "over",
            "--saturation",
            "0",
            "--tint",
            "#abcdef",
            "--tint-strength",
            "1",
            "--opacity",
            "0.5",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        result = notebook.read_json(target)
        item = result["pages"][1]["images"][0]
        self.assertEqual(item["layer"], "over")
        self.assertEqual(item["tint_strength"], 1)
        self.assertEqual((target.parent / item["path"]).resolve(), image_path)
        self.assertEqual(result["pages"][0]["images"], [])
        page = notebook.render_page(result, 1, 100, target.parent, True)
        x = notebook.point_mm(22, 100)
        y = notebook.point_mm(32, 100)
        self.assertEqual(page.getpixel((x, y)), (171, 205, 239, 64))

    def test_add_image_paths_remain_valid_in_another_output_directory(self):
        Image.new("RGB", (10, 10), "blue").save(self.root / "texture.jpg")
        nested = self.root / "nested"
        nested.mkdir()
        output = nested / "with-photo.json"
        run = self.run_cli(
            "add-image",
            "--config",
            str(self.config),
            "--page",
            "1",
            "--image",
            "texture.jpg",
            "--x-mm",
            "25",
            "--y-mm",
            "45",
            "--width-mm",
            "12",
            "--height-mm",
            "12",
            "--out",
            str(output),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        doc = notebook.read_json(output)
        self.assertEqual(
            doc["pages"][0]["images"][0]["path"], "../texture.jpg"
        )
        render = notebook.render_page(doc, 0, 100, nested)
        x = notebook.point_mm(30, 100)
        y = notebook.point_mm(50, 100)
        self.assertEqual(render.getpixel((x, y))[:3], (0, 0, 254))

    def test_add_image_rejects_invalid_placements_and_missing_assets(self):
        Image.new("RGB", (8, 8), "blue").save(self.root / "tiny.png")
        common = (
            "add-image",
            "--config",
            str(self.config),
            "--image",
            "tiny.png",
            "--page",
            "1",
            "--x-mm",
            "10",
            "--y-mm",
            "10",
            "--width-mm",
            "10",
            "--height-mm",
            "10",
        )
        invalid = (
            ("--page", "3"),
            ("--x-mm", "95"),
            ("--height-mm", "0"),
            ("--opacity", "2"),
            ("--tint", "red"),
            ("--image", "missing.png"),
            ("--tint-strength", "0.8"),
        )
        for options in invalid:
            with self.subTest(options=options):
                target = self.root / "bad-image.json"
                run = self.run_cli(*common, *options, "--out", str(target))
                self.assertEqual(run.returncode, 2, run.stderr)
                self.assertFalse(target.exists())

    def test_set_page_adds_page_with_text_date_and_printed_state(self):
        target = self.root / "third-page.json"
        run = self.run_cli(
            "set-page",
            "--config",
            str(self.config),
            "--page",
            "3",
            "--create",
            "--date",
            "02/11/2026",
            "--number",
            "III",
            "--line",
            "New page!",
            "--line",
            "Another line.",
            "--printed",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        doc = notebook.read_json(target)
        self.assertEqual(len(doc["pages"]), 3)
        self.assertEqual(doc["pages"][2]["date"], "02/11/2026")
        self.assertEqual(doc["pages"][2]["number"], "III")
        self.assertEqual(
            doc["pages"][2]["text"], ["New page!", "Another line."]
        )
        self.assertTrue(doc["pages"][2]["printed"])
        self.assertIsNotNone(
            notebook.render_page(doc, 2, 100, self.root).getbbox()
        )

    def test_set_page_append_and_clear_text(self):
        first = self.root / "appended.json"
        second = self.root / "cleared.json"
        appended = self.run_cli(
            "set-page",
            "--config",
            str(self.config),
            "--page",
            "1",
            "--append-line",
            "Extra note",
            "--out",
            str(first),
        )
        self.assertEqual(appended.returncode, 0, appended.stderr)
        self.assertEqual(
            notebook.read_json(first)["pages"][0]["text"],
            ["Hola, ñandú. ¿Sí?", "Extra note"],
        )
        cleared = self.run_cli(
            "set-page",
            "--config",
            str(first),
            "--page",
            "1",
            "--clear-text",
            "--no-printed",
            "--out",
            str(second),
        )
        self.assertEqual(cleared.returncode, 0, cleared.stderr)
        self.assertEqual(notebook.read_json(second)["pages"][0]["text"], [])
        self.assertFalse(notebook.read_json(second)["pages"][0]["printed"])

    def test_set_page_refuses_missing_pages_and_invalid_content(self):
        invalid = (
            ("--page", "3", "--date", "today"),
            ("--page", "4", "--create"),
            ("--page", "1", "--create"),
            ("--page", "1", "--line", "a" * 300),
            ("--page", "1", "--date", "date" * 20),
            ("--page", "1"),
        )
        for args in invalid:
            with self.subTest(args=args):
                out = self.root / "invalid-page.json"
                result = self.run_cli(
                    "set-page",
                    "--config",
                    str(self.config),
                    *args,
                    "--out",
                    str(out),
                )
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertFalse(out.exists())

    def test_page_edits_rebase_existing_fonts_glyphs_and_images(self):
        (self.root / "font.otf").symlink_to(notebook.DEFAULT_FONT)
        Image.new("RGBA", (12, 12), (0, 0, 0, 255)).save(
            self.root / "glyph.png"
        )
        Image.new("RGB", (12, 12), "blue").save(self.root / "picture.png")
        self.doc["handwriting"]["font"] = "font.otf"
        self.doc["handwriting"]["glyphs"]["a"] = {"image": "glyph.png"}
        self.doc["pages"][0]["images"] = [
            {
                "path": "picture.png",
                "x_mm": 20,
                "y_mm": 40,
                "width_mm": 10,
                "height_mm": 10,
                "layer": "under",
            }
        ]
        notebook.write_json(self.config, self.doc, replace=True)
        nested = self.root / "nested"
        nested.mkdir()
        target = nested / "updated.json"
        run = self.run_cli(
            "set-page",
            "--config",
            str(self.config),
            "--page",
            "1",
            "--append-line",
            "Another a",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        changed = notebook.read_json(target)
        self.assertEqual(changed["handwriting"]["font"], "../font.otf")
        self.assertEqual(
            changed["handwriting"]["glyphs"]["a"]["image"], "../glyph.png"
        )
        self.assertEqual(
            changed["pages"][0]["images"][0]["path"], "../picture.png"
        )
        notebook.render_page(changed, 0, 100, target.parent)

    def test_image_addition_rebases_previous_overlays_when_moving_json(self):
        Image.new("RGB", (8, 8), "blue").save(self.root / "first.png")
        Image.new("RGB", (8, 8), "red").save(self.root / "second.png")
        self.doc["pages"][0]["images"] = [
            {
                "path": "first.png",
                "x_mm": 12,
                "y_mm": 35,
                "width_mm": 10,
                "height_mm": 10,
                "layer": "under",
            }
        ]
        notebook.write_json(self.config, self.doc, replace=True)
        nested = self.root / "nested"
        nested.mkdir()
        target = nested / "two-overlays.json"
        run = self.run_cli(
            "add-image",
            "--config",
            str(self.config),
            "--page",
            "1",
            "--image",
            "second.png",
            "--x-mm",
            "25",
            "--y-mm",
            "45",
            "--width-mm",
            "10",
            "--height-mm",
            "10",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        doc = notebook.read_json(target)
        self.assertEqual(
            [item["path"] for item in doc["pages"][0]["images"]],
            ["../first.png", "../second.png"],
        )
        notebook.render_page(doc, 0, 100, target.parent)

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

    def test_glyph_can_sample_individual_angle_baseline_and_width(self):
        self.doc["handwriting"]["glyphs"]["ñ"] = {
            "rotation_deg": {
                "minimum": -3,
                "mean": -1,
                "median": -1,
                "maximum": 2,
                "stddev": 0.7,
            },
            "baseline_shift_mm": {
                "minimum": -0.4,
                "mean": 0,
                "median": 0,
                "maximum": 0.4,
                "stddev": 0.1,
            },
            "width_scale": {
                "minimum": 0.8,
                "mean": 1,
                "median": 1,
                "maximum": 1.2,
                "stddev": 0.06,
            },
        }
        result = notebook.render_page(self.doc, 0, 120, self.root)
        again = notebook.render_page(self.doc, 0, 120, self.root)
        self.assertEqual(result.tobytes(), again.tobytes())

    def test_decomposed_glyph_override_matches_normalized_character(self):
        self.doc["pages"][0]["text"] = ["ñ"]
        self.doc["handwriting"]["glyphs"] = {
            "n\u0303": {"ink": "#cc0088", "height_scale": 1.5}
        }
        decomposed = notebook.render_page(self.doc, 0, 120, self.root)
        self.doc["handwriting"]["glyphs"] = {
            "ñ": {"ink": "#cc0088", "height_scale": 1.5}
        }
        normalized = notebook.render_page(self.doc, 0, 120, self.root)
        self.assertEqual(decomposed.tobytes(), normalized.tobytes())
        self.doc["handwriting"]["glyphs"]["n\u0303"] = {}
        with self.assertRaisesRegex(ValueError, "distinct characters"):
            notebook.render_page(self.doc, 0, 120, self.root)

    def test_calibrate_adds_rotation_and_baseline_measurements(self):
        out = self.root / "measured.json"
        run = self.run_cli(
            "calibrate",
            "--config",
            str(self.config),
            "--glyph",
            "n\u0303",
            "--rotations=-3,-1,2",
            "--baselines=-0.2,0.0,0.1",
            "--out",
            str(out),
        )
        self.assertEqual(run.returncode, 0, run.stderr)
        spec = notebook.read_json(out)["handwriting"]["glyphs"]["ñ"]
        self.assertEqual(spec["rotation_deg"]["minimum"], -3)
        self.assertEqual(spec["rotation_deg"]["maximum"], 2)
        self.assertAlmostEqual(spec["rotation_deg"]["median"], -1)
        self.assertAlmostEqual(spec["baseline_shift_mm"]["mean"], -1 / 30)
        self.doc["handwriting"]["glyphs"]["ñ"] = spec
        notebook.render_page(self.doc, 0, 120, self.root)

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

    def test_trailing_spaces_and_tabs_cannot_escape_right_margin(self):
        for whitespace in (" " * 150, "\t" * 15):
            with self.subTest(character=repr(whitespace[0])):
                self.doc["pages"][0]["text"] = ["Hola" + whitespace]
                with self.assertRaisesRegex(
                    ValueError, "overflows right margin"
                ):
                    notebook.render_page(self.doc, 0, 120, self.root)

    def test_decomposed_accents_render_like_precomposed_spanish(self):
        self.doc["pages"][0]["text"] = ["pin\u0303ata, cafe\u0301"]
        decomposed = notebook.render_page(self.doc, 0, 120, self.root)
        self.doc["pages"][0]["text"] = ["piñata, café"]
        precomposed = notebook.render_page(self.doc, 0, 120, self.root)
        self.assertEqual(decomposed.tobytes(), precomposed.tobytes())

    @unittest.skipUnless(shutil.which("fc-match"), "Fontconfig absent")
    def test_fontconfig_fallback_without_config_local_copy(self):
        self.doc["handwriting"]["font"] = "DejaVuSans.ttf"
        self.assertFalse((self.root / "DejaVuSans.ttf").exists())
        result = notebook.render_page(self.doc, 0, 120, self.root)
        self.assertIsNotNone(result.getchannel("A").getbbox())

    def test_png_export_keeps_existing_pages_if_later_page_fails(self):
        target = self.root / "pages"
        target.mkdir()
        existing = target / "page-001.png"
        existing.write_bytes(b"keep-original-page")
        self.doc["pages"][1]["text"] = ["a" * 1000]
        notebook.write_json(self.config, self.doc, replace=True)
        run = self.run_cli(
            "render",
            "--config",
            str(self.config),
            "--format",
            "png",
            "--dpi",
            "100",
            "--replace",
            "--out",
            str(target),
        )
        self.assertEqual(run.returncode, 2, run.stderr)
        self.assertIn("overflows right margin", run.stderr)
        self.assertEqual(existing.read_bytes(), b"keep-original-page")
        self.assertFalse((target / "page-002.png").exists())

    def test_relative_font_used_for_header_and_page_number(self):
        (self.root / "font.otf").symlink_to(notebook.DEFAULT_FONT)
        self.doc["handwriting"]["font"] = "font.otf"
        self.doc["paper"]["date_style"] = "plain"
        self.doc["paper"]["page_number_style"] = "bottom-right"
        self.doc["pages"][0]["date"] = "02/11"
        result = notebook.render_page(self.doc, 0, 120, self.root)
        self.assertIsNotNone(result.getchannel("A").getbbox())

    def test_page_number_cannot_render_beyond_bottom_sheet_edge(self):
        self.doc["paper"]["margin_bottom_mm"] = 0
        self.doc["paper"]["page_number_style"] = "bottom-right"
        with self.assertRaisesRegex(ValueError, "page number.*physical sheet"):
            notebook.render_page(self.doc, 0, 120, self.root)

    def test_wide_page_number_cannot_render_off_left_edge(self):
        self.doc["paper"]["width_mm"] = 40
        self.doc["paper"]["page_number_style"] = "bottom-right"
        self.doc["pages"][0]["number"] = "W" * 24
        self.doc["pages"][0]["text"] = []
        with self.assertRaisesRegex(ValueError, "page number.*physical sheet"):
            notebook.render_page(self.doc, 0, 120, self.root)

    def test_boxed_date_cannot_be_silently_cropped_at_sheet_edge(self):
        self.doc["paper"].update(
            {
                "margin_left_mm": 0,
                "date_style": "box",
                "date_side": "left",
                "date_y_mm": 0,
            }
        )
        self.doc["pages"][0]["date"] = "02/11/2026"
        with self.assertRaisesRegex(ValueError, "date label.*physical sheet"):
            notebook.render_page(self.doc, 0, 120, self.root)

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
