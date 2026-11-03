#!/usr/bin/env python3
# Copyright © 2026 Alberto Villa Osorno.
# SPDX-License-Identifier: MIT
"""Linux-first notebook CLI: manual handwriting statistics -> printable pages.

Requires Python 3.14+ and Pillow. ImageMagick is optional for SVG overlays.
No browser session, hardware control, API token, or LLM is involved.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import math
import os
import random
import re
import statistics
import subprocess
import tempfile
import unicodedata
import xml.etree.ElementTree as ET
from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance, ImageFont, ImageOps

MM_PER_INCH = 25.4
MAX_PAGES = 40
MAX_PIXELS_PER_PAGE = 32_000_000
MAX_IMAGE_PIXELS = 32_000_000
Image.MAX_IMAGE_PIXELS = MAX_IMAGE_PIXELS
DEFAULT_FONT = (
    "/usr/share/fonts/urw-base35/Z003-MediumItalic.otf"
    if Path("/usr/share/fonts/urw-base35/Z003-MediumItalic.otf").is_file()
    else "DejaVuSans.ttf"
)

DEFAULT = {
    "version": 1,
    "paper": {
        "width_mm": 210,
        "height_mm": 297,
        "style": "ruled",
        "spacing_mm": 7,
        "margin_top_mm": 22,
        "margin_bottom_mm": 16,
        "margin_left_mm": 20,
        "margin_right_mm": 14,
        "line_color": "#c5d4e7",
        "margin_color": "#d88c9c",
        "border_color": "#b4bdca",
        "corner_radius_mm": 2,
        "date_style": "box",
        "date_side": "right",
        "date_y_mm": 7,
        "page_number_style": "bottom-right",
    },
    "handwriting": {
        "font": DEFAULT_FONT,
        "seed": 41,
        "ink": "#173b73",
        "line_height_mm": 7,
        "letter_spacing_mm": 0.15,
        "texture": 0.18,
        "stroke_px": 0,
        "height_mm": {
            "minimum": 2.8,
            "mean": 3.5,
            "median": 3.45,
            "maximum": 4.3,
            "stddev": 0.27,
            "distribution": "gaussian",
        },
        "width_scale": {
            "minimum": 0.85,
            "mean": 1,
            "median": 1,
            "maximum": 1.16,
            "stddev": 0.05,
        },
        "baseline_mm": {
            "minimum": -0.35,
            "mean": 0,
            "median": 0,
            "maximum": 0.35,
            "stddev": 0.09,
        },
        "rotation_deg": {
            "minimum": -2.5,
            "mean": 0,
            "median": 0,
            "maximum": 2.5,
            "stddev": 0.75,
        },
        "glyphs": {},
    },
    "pages": [
        {
            "date": "",
            "printed": False,
            "text": [
                "A notebook that leaves room for living.",
                "Adjust the paper, letters, ink and spacing.",
                "Write a little, save a lot of time.",
            ],
            "images": [],
        }
    ],
}


PAPER_NUMBER_OPTIONS = {
    "width_mm": (40, 500),
    "height_mm": (40, 700),
    "spacing_mm": (1, 50),
    "margin_top_mm": (0, 700),
    "margin_bottom_mm": (0, 700),
    "margin_left_mm": (0, 500),
    "margin_right_mm": (0, 500),
    "corner_radius_mm": (0, 30),
    "date_y_mm": (0, 50),
}
PAPER_CHOICE_OPTIONS = {
    "style": ("blank", "ruled", "grid", "dot"),
    "date_style": ("none", "plain", "box", "underline"),
    "date_side": ("left", "right"),
    "page_number_style": ("none", "bottom-left", "bottom-right"),
}
PAPER_COLOR_OPTIONS = ("line_color", "margin_color", "border_color")
PAPER_BOOLEAN_OPTIONS = ("draw_margin", "draw_border")
HANDWRITING_NUMBERS = {
    "line_height_mm": (2, 40),
    "letter_spacing_mm": (-1, 3),
    "texture": (0, 1),
    "stroke_px": (0, 8),
    "seed": (-2_147_483_648, 2_147_483_647),
}
HANDWRITING_STATS = ("height_mm", "width_scale", "baseline_mm", "rotation_deg")
STAT_FIELDS = ("minimum", "mean", "median", "maximum", "stddev")


def require_number(value, name, low=None, high=None):
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(value)
    ):
        raise ValueError(f"{name} must be a finite number")
    if low is not None and value < low or high is not None and value > high:
        raise ValueError(f"{name} is outside the supported range")
    return float(value)


def color(value):
    if not isinstance(value, str) or not re.fullmatch(
        r"#[0-9a-fA-F]{6}", value
    ):
        raise ValueError("colors must use #RRGGBB")
    return tuple(bytes.fromhex(value[1:]))


def read_json(path):
    if path.stat().st_size > 1_000_000:
        raise ValueError("configuration is too large (limit: 1 MB)")
    result = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(result, dict):
        raise TypeError("configuration must be a JSON object")
    return result


def atomic_write(target, writer, replace=False):
    target = Path(target)
    if not target.parent.is_dir():
        raise ValueError(f"output directory does not exist: {target.parent}")
    if target.exists() and not replace:
        raise FileExistsError(
            f"refusing to overwrite {target}; pass --replace"
        )
    name = None
    try:
        with tempfile.NamedTemporaryFile(
            dir=target.parent, prefix=".atrament-", delete=False
        ) as f:
            name = Path(f.name)
            writer(f)
            f.flush()
            os.fsync(f.fileno())
        if replace:
            os.replace(name, target)
        else:
            os.link(name, target)
            name.unlink()
        name = None
    finally:
        if name is not None:
            name.unlink(missing_ok=True)


def write_json(path, data, replace=False):
    payload = (json.dumps(data, indent=2, ensure_ascii=False) + "\n").encode(
        "utf-8"
    )
    atomic_write(path, lambda f: f.write(payload), replace)


def rebase_notebook_assets(doc, source, target):
    """Keep existing local asset references valid when a JSON file moves."""
    source_dir = Path(source).resolve().parent
    target_dir = Path(target).resolve().parent
    if source_dir == target_dir:
        return

    def adjusted(value):
        if (
            not isinstance(value, str)
            or not value
            or Path(value).is_absolute()
        ):
            return value
        return os.path.relpath(source_dir / value, target_dir)

    profile = doc.get("handwriting", {})
    if isinstance(profile, dict):
        font = profile.get("font")
        if (
            isinstance(font, str)
            and not Path(font).is_absolute()
            and (source_dir / font).is_file()
        ):
            profile["font"] = adjusted(font)
        glyphs = profile.get("glyphs", {})
        if isinstance(glyphs, dict):
            for custom in glyphs.values():
                if isinstance(custom, dict) and "image" in custom:
                    custom["image"] = adjusted(custom["image"])
    pages = doc.get("pages", [])
    if isinstance(pages, list):
        for page in pages:
            if not isinstance(page, dict):
                continue
            for item in page.get("images", []):
                if isinstance(item, dict) and "path" in item:
                    item["path"] = adjusted(item["path"])


def stat_sample(spec, rng, name):
    if isinstance(spec, (float, int)) and not isinstance(spec, bool):
        return require_number(spec, name)
    if not isinstance(spec, dict):
        raise TypeError(f"{name} must be a number or a statistics object")
    if "height_mm" in name or "width_mm" in name:
        minimum, maximum = 0.1, 50
    elif "width_scale" in name:
        minimum, maximum = (0.2 if name.startswith("glyph.") else 0.1), 5
    elif "baseline_mm" in name or "baseline_shift_mm" in name:
        minimum, maximum = -10, 10
    elif "rotation_deg" in name:
        minimum, maximum = -45, 45
    else:
        minimum, maximum = -1000, 1000
    lower = require_number(
        spec["minimum"], f"{name}.minimum", minimum, maximum
    )
    upper = require_number(
        spec["maximum"], f"{name}.maximum", minimum, maximum
    )
    mean = require_number(spec["mean"], f"{name}.mean", minimum, maximum)
    median = require_number(spec["median"], f"{name}.median", minimum, maximum)
    deviation = require_number(spec["stddev"], f"{name}.stddev", 0, 1000)
    if not lower <= mean <= upper or not lower <= median <= upper:
        raise ValueError(f"{name} has inconsistent statistical bounds")
    if deviation > upper - lower:
        raise ValueError(f"{name}.stddev exceeds its variation range")
    if spec.get("distribution", "gaussian") == "triangular":
        val = rng.triangular(lower, upper, median)
    elif spec.get("distribution", "gaussian") == "gaussian":
        val = rng.gauss(mean, deviation)
    else:
        raise ValueError(f"{name} distribution must be gaussian or triangular")
    return min(upper, max(lower, val))


def point_mm(value, dpi):
    mm = require_number(value, "millimeter coordinate", -1000, 1000)
    return round(mm * dpi / MM_PER_INCH)


def image_for_path(path, width, height):
    path = Path(path)
    if not path.is_file() or path.stat().st_size > 20_000_000:
        raise ValueError(f"image missing or larger than 20 MB: {path}")
    if path.suffix.lower() == ".svg":
        if path.stat().st_size > 2_000_000:
            raise ValueError("SVG is too large")
        source = path.read_bytes()
        if (
            b"<!DOCTYPE" in source.upper()
            or b"<!ENTITY" in source.upper()
            or b"<?XML-STYLESHEET" in source.upper()
        ):
            raise ValueError("SVG with external entities is unsupported")
        root = ET.fromstring(source)
        allowed = {
            "svg",
            "g",
            "path",
            "rect",
            "circle",
            "ellipse",
            "line",
            "polyline",
            "polygon",
            "text",
            "tspan",
            "defs",
            "clipPath",
        }
        for node in root.iter():
            tag = node.tag.rsplit("}", 1)[-1]
            if tag not in allowed:
                raise ValueError(f"unsupported SVG element: {tag}")
            for attribute, value in node.attrib.items():
                local = attribute.rsplit("}", 1)[-1].lower()
                if (
                    local in {"href", "src"}
                    or "url(" in value.lower()
                    or "@import" in value.lower()
                ):
                    raise ValueError(
                        "external SVG references and URL paints are unsupported"
                    )
        if root.tag.rsplit("}", 1)[-1] != "svg":
            raise ValueError("input is not an SVG document")
        try:
            proc = subprocess.run(
                [
                    "magick",
                    "-limit",
                    "memory",
                    "128MiB",
                    "-limit",
                    "map",
                    "256MiB",
                    "-limit",
                    "width",
                    "8192",
                    "-limit",
                    "height",
                    "8192",
                    "-background",
                    "none",
                    str(path),
                    "-resize",
                    f"{width}x{height}!",
                    "png:-",
                ],
                capture_output=True,
                check=True,
                timeout=20,
            )
        except (
            FileNotFoundError,
            subprocess.CalledProcessError,
            subprocess.TimeoutExpired,
        ) as exc:
            raise ValueError(
                "SVG conversion requires a working ImageMagick installation"
            ) from exc
        if len(proc.stdout) > 80_000_000:
            raise ValueError("converted SVG too large")
        src = Image.open(io.BytesIO(proc.stdout))
    else:
        if path.suffix.lower() not in {".png", ".jpg", ".jpeg", ".webp"}:
            raise ValueError(
                "images must be PNG, JPG, WebP, or restricted local SVG"
            )
        src = Image.open(path)
    with src:
        src.load()
        if src.width * src.height > MAX_IMAGE_PIXELS:
            raise ValueError("image pixel budget exceeded")
        return (
            ImageOps.exif_transpose(src)
            .convert("RGBA")
            .resize((width, height), Image.Resampling.LANCZOS)
        )


def draw_picture(page, config_base, item, dpi):
    if not isinstance(item, dict) or not isinstance(item.get("path"), str):
        raise TypeError("image placement requires a local path")
    path = Path(item["path"])
    if not path.is_absolute():
        path = config_base / path
    x = point_mm(require_number(item.get("x_mm", 0), "image.x_mm", 0), dpi)
    y = point_mm(require_number(item.get("y_mm", 0), "image.y_mm", 0), dpi)
    w = max(
        1,
        point_mm(
            require_number(item["width_mm"], "image.width_mm", 0.1, 1000), dpi
        ),
    )
    h = max(
        1,
        point_mm(
            require_number(item["height_mm"], "image.height_mm", 0.1, 1000),
            dpi,
        ),
    )
    if w * h > MAX_IMAGE_PIXELS:
        raise ValueError("image destination exceeds pixel budget")
    picture = image_for_path(path, w, h)
    saturation = require_number(
        item.get("saturation", 1), "image.saturation", 0, 5
    )
    if saturation != 1:
        alpha = picture.getchannel("A")
        picture = (
            ImageEnhance.Color(picture.convert("RGB"))
            .enhance(saturation)
            .convert("RGBA")
        )
        picture.putalpha(alpha)
    tint_strength = require_number(
        item.get("tint_strength", 0), "image.tint_strength", 0, 1
    )
    if tint_strength:
        alpha = picture.getchannel("A")
        tint = Image.new(
            "RGB", picture.size, color(item.get("tint", "#000000"))
        )
        picture = Image.blend(
            picture.convert("RGB"), tint, tint_strength
        ).convert("RGBA")
        picture.putalpha(alpha)
    opacity = require_number(item.get("opacity", 1), "image.opacity", 0, 1)
    if opacity != 1:
        picture.putalpha(
            picture.getchannel("A").point(lambda a: round(a * opacity))
        )
    if x + w > page.width or y + h > page.height:
        raise ValueError("image falls outside the physical sheet")
    page.alpha_composite(picture, (x, y))


def draw_paper(page, paper, dpi, transparent, keep_rules, printed):
    draw = ImageDraw.Draw(page)
    width, height = page.size
    line_color = color(paper["line_color"])
    radius = point_mm(
        require_number(
            paper.get("corner_radius_mm", 0), "corner_radius_mm", 0, 30
        ),
        dpi,
    )
    left = point_mm(
        require_number(paper["margin_left_mm"], "margin_left_mm", 0), dpi
    )
    right = width - point_mm(
        require_number(paper["margin_right_mm"], "margin_right_mm", 0), dpi
    )
    top = point_mm(
        require_number(paper["margin_top_mm"], "margin_top_mm", 0), dpi
    )
    bottom = height - point_mm(
        require_number(paper["margin_bottom_mm"], "margin_bottom_mm", 0), dpi
    )
    if not (left + 5 < right and top + 5 < bottom):
        raise ValueError("paper margins leave no writable area")
    if not printed:
        if not transparent:
            draw.rectangle((0, 0, width, height), fill="white")
        if not transparent or keep_rules:
            style = paper["style"]
            if style not in {"blank", "ruled", "grid", "dot"}:
                raise ValueError(
                    "paper style must be blank, ruled, grid, or dot"
                )
            step = point_mm(
                require_number(paper["spacing_mm"], "paper.spacing_mm", 1, 50),
                dpi,
            )
            step = max(step, 2)
            line_width = max(1, point_mm(0.15, dpi))
            if style in {"ruled", "grid"}:
                for yy in range(top, bottom + 1, step):
                    draw.line(
                        (left, yy, right, yy),
                        fill=line_color,
                        width=line_width,
                    )
            if style == "grid":
                for xx in range(left, right + 1, step):
                    draw.line(
                        (xx, top, xx, bottom),
                        fill=line_color,
                        width=line_width,
                    )
            if style == "dot":
                for yy in range(top, bottom + 1, step):
                    for xx in range(left, right + 1, step):
                        r = max(1, point_mm(0.12, dpi))
                        draw.ellipse(
                            (xx - r, yy - r, xx + r, yy + r), fill=line_color
                        )
        if paper.get("draw_margin", True):
            margin_color = color(paper["margin_color"])
            draw.line(
                (left, top, left, bottom),
                fill=margin_color,
                width=max(1, point_mm(0.23, dpi)),
            )
            if paper.get("draw_border", True):
                draw.rounded_rectangle(
                    (left, top, right, bottom),
                    radius=min(radius, (right - left) // 2),
                    outline=color(paper["border_color"]),
                    width=max(1, point_mm(0.12, dpi)),
                )
    return left, right, top, bottom


class Pen:
    """Raster font glyphs individually with bounded, deterministic statistics.

    Font masks are scaffolding: optional custom per-glyph PNGs replace their
    appearance outright. These are not proven samples of anyone's handwriting.
    """

    def __init__(self, cfg, dpi, base):
        self.cfg, self.dpi, self.base = cfg, dpi, base
        font_path = cfg.get("font", DEFAULT_FONT)
        if not isinstance(font_path, str) or not font_path:
            raise ValueError("handwriting.font must be a font file path")
        resolved = Path(font_path)
        if not resolved.is_absolute():
            resolved = base / resolved
            if font_path == "DejaVuSans.ttf" and not resolved.is_file():
                # Pillow does not bundle DejaVu on every Linux distribution.
                # Fontconfig supplies an installed sans-serif when necessary.
                try:
                    match = subprocess.run(
                        ["fc-match", "-f", "%{file}", "sans-serif"],
                        capture_output=True,
                        check=True,
                        text=True,
                        timeout=3,
                    )
                except (
                    FileNotFoundError,
                    subprocess.CalledProcessError,
                    subprocess.TimeoutExpired,
                ):
                    pass
                else:
                    fallback = Path(match.stdout.strip())
                    if fallback.is_file():
                        resolved = fallback
        self.font_path = str(resolved)
        self.font = ImageFont.truetype(self.font_path, 150)
        self.mask_cache = {}
        self.ink = color(cfg.get("ink", "#173b73"))
        self.texture = require_number(cfg.get("texture", 0), "texture", 0, 1)
        self.stroke = int(
            require_number(cfg.get("stroke_px", 0), "stroke_px", 0, 8)
        )
        glyphs = cfg.get("glyphs", {})
        if not isinstance(glyphs, dict):
            raise TypeError("glyphs must be a mapping")
        self.glyphs = {}
        for key, override in glyphs.items():
            normalized = unicodedata.normalize("NFC", key)
            if len(normalized) != 1 or normalized in self.glyphs:
                raise ValueError(
                    "glyph keys must identify distinct characters"
                )
            self.glyphs[normalized] = override

    def mask(self, char, glyph):
        if "image" in glyph:
            path = Path(glyph["image"])
            if not path.is_absolute():
                path = self.base / path
            with Image.open(path) as im:
                if im.width * im.height > MAX_IMAGE_PIXELS:
                    raise ValueError("glyph image exceeds pixel budget")
                rgba = im.convert("RGBA")
                box = rgba.getchannel("A").getbbox()
                return (
                    rgba.getchannel("A").crop(box)
                    if box
                    else Image.new("L", (1, 1))
                )
        substitute = glyph.get("replace", char)
        if not isinstance(substitute, str) or len(substitute) > 4:
            raise ValueError(
                "glyph replacement must be a string up to 4 characters"
            )
        key = (substitute, self.stroke)
        if key not in self.mask_cache:
            bbox = self.font.getbbox(substitute, stroke_width=self.stroke)
            w, h = max(bbox[2] - bbox[0], 1), max(bbox[3] - bbox[1], 1)
            mask = Image.new("L", (w, h))
            ImageDraw.Draw(mask).text(
                (-bbox[0], -bbox[1]),
                substitute,
                font=self.font,
                fill=255,
                stroke_width=self.stroke,
                stroke_fill=255,
            )
            self.mask_cache[key] = mask
        return self.mask_cache[key]

    def glyph_image(self, char, rng):
        custom = self.glyphs.get(char, {})
        if not isinstance(custom, dict):
            raise TypeError("each glyph override must be a mapping")
        stats = custom.get("height_mm", self.cfg["height_mm"])
        height_mm = stat_sample(stats, rng, f"glyph[{char}].height_mm")
        height_mm *= require_number(
            custom.get("height_scale", 1), "height_scale", 0.2, 5
        )
        width_scale = stat_sample(self.cfg["width_scale"], rng, "width_scale")
        width_scale *= require_number(
            stat_sample(
                custom.get("width_scale", 1), rng, "glyph.width_scale"
            ),
            "glyph.width_scale",
            0.2,
            5,
        )
        mask = self.mask(char, custom)
        if "width_mm" in custom:
            w = max(
                1,
                point_mm(
                    stat_sample(custom["width_mm"], rng, "glyph.width_mm"),
                    self.dpi,
                ),
            )
        else:
            ratio = mask.width / max(mask.height, 1)
            w = max(1, point_mm(height_mm * ratio * width_scale, self.dpi))
        h = max(1, point_mm(height_mm, self.dpi))
        if w * h > 1_000_000:
            raise ValueError("glyph is too large")
        mask = mask.resize((w, h), Image.Resampling.LANCZOS)
        if self.texture:
            textured = mask.copy()
            brush = ImageDraw.Draw(textured)
            # Deterministic pinholes produce a restrained dry-pen effect.
            for _ in range(round(w * h * self.texture / 180)):
                xx, yy = rng.randrange(w), rng.randrange(h)
                brush.point(
                    (xx, yy),
                    fill=round(mask.getpixel((xx, yy)) * (1 - self.texture)),
                )
            mask = textured
        ink = color(custom.get("ink", self.cfg.get("ink", "#173b73")))
        image = Image.new("RGBA", (w, h), ink + (0,))
        image.putalpha(mask)
        rotation = stat_sample(
            self.cfg.get("rotation_deg", 0), rng, "rotation_deg"
        )
        rotation += require_number(
            stat_sample(
                custom.get("rotation_deg", 0), rng, "glyph.rotation_deg"
            ),
            "glyph.rotation_deg",
            -45,
            45,
        )
        if rotation:
            image = image.rotate(
                rotation, expand=True, resample=Image.Resampling.BICUBIC
            )
        baseline = stat_sample(
            self.cfg.get("baseline_mm", 0), rng, "baseline_mm"
        )
        baseline += require_number(
            stat_sample(
                custom.get("baseline_shift_mm", 0),
                rng,
                "glyph.baseline_shift_mm",
            ),
            "glyph.baseline_shift_mm",
            -10,
            10,
        )
        return image, point_mm(baseline, self.dpi)

    def draw_line(self, page, text, x, y, right, page_number, line_index):
        if not isinstance(text, str) or len(text) > 10_000:
            raise ValueError("line must be text of at most 10000 characters")
        # Canonically equivalent Spanish accents must use the same glyph and
        # per-character override, regardless of keyboard/input composition.
        text = unicodedata.normalize("NFC", text)
        seed = self.cfg.get("seed", 0)
        if not isinstance(seed, int):
            raise TypeError("handwriting seed must be an integer")
        baseline_y = y
        cursor = x
        space = point_mm(
            require_number(
                self.cfg.get("letter_spacing_mm", 0),
                "letter_spacing_mm",
                -1,
                3,
            ),
            self.dpi,
        )
        line_height = point_mm(
            require_number(
                self.cfg["line_height_mm"], "line_height_mm", 2, 40
            ),
            self.dpi,
        )
        for index, char in enumerate(text):
            if char in "\r\n\x00":
                raise ValueError("split input into separate explicit lines")
            if char in {"\t", " "}:
                cursor += point_mm(8 if char == "\t" else 2.1, self.dpi)
                if cursor > right:
                    raise ValueError(
                        f"page {page_number}: line {line_index + 1} "
                        "overflows right margin"
                    )
                continue
            rngseed = hashlib.sha256(
                f"{seed}:{page_number}:{line_index}:{index}:{char}".encode()
            ).digest()
            rng = random.Random(int.from_bytes(rngseed[:8], "big"))
            glyph, jitter = self.glyph_image(char, rng)
            if cursor + glyph.width > right:
                raise ValueError(
                    f"page {page_number}: line {line_index + 1} overflows "
                    "right margin"
                )
            # Bottom-align the glyph with bounded baseline drift.
            gy = baseline_y + line_height - glyph.height + jitter
            if gy < 0 or gy + glyph.height > page.height:
                raise ValueError(
                    f"page {page_number}: glyph exceeds physical page"
                )
            page.alpha_composite(glyph, (cursor, gy))
            cursor += glyph.width + space
        return cursor


def validate_notebook(doc, dpi):
    if doc.get("version") != 1:
        raise ValueError("notebook version must be 1")
    paper = doc.get("paper")
    handwriting = doc.get("handwriting")
    pages = doc.get("pages")
    if not isinstance(paper, dict) or not isinstance(handwriting, dict):
        raise TypeError("paper and handwriting must be objects")
    if not isinstance(pages, list) or not 1 <= len(pages) <= MAX_PAGES:
        raise ValueError(f"pages must contain 1–{MAX_PAGES} pages")
    width = point_mm(
        require_number(paper["width_mm"], "paper.width_mm", 40, 500), dpi
    )
    height = point_mm(
        require_number(paper["height_mm"], "paper.height_mm", 40, 700), dpi
    )
    if width * height > MAX_PIXELS_PER_PAGE:
        raise ValueError(
            "page pixel budget exceeded; reduce DPI or page dimensions"
        )
    for i, page in enumerate(pages):
        if not isinstance(page, dict) or not isinstance(
            page.get("text", []), list
        ):
            raise TypeError(f"page {i + 1} must be an object with a text list")
        if not isinstance(page.get("images", []), list):
            raise TypeError(f"page {i + 1} images must be a list")
    return width, height


def render_page(doc, index, dpi, base, transparent=False, keep_rules=False):
    width, height = validate_notebook(doc, dpi)
    paper, profile = doc["paper"], doc["handwriting"]
    data = doc["pages"][index]
    printed = data.get("printed", False)
    if not isinstance(printed, bool):
        raise TypeError("printed must be true or false")
    canvas = Image.new("RGBA", (width, height), (255, 255, 255, 0))
    left, right, top, bottom = draw_paper(
        canvas, paper, dpi, transparent, keep_rules, printed
    )
    for item in data.get("images", []):
        if not isinstance(item, dict):
            raise TypeError("each image placement must be an object")
        if item.get("layer", "under") == "under":
            draw_picture(canvas, base, item, dpi)
        elif item["layer"] != "over":
            raise ValueError("image layer must be under or over")
    pen = Pen(profile, dpi, base)
    line_h = point_mm(
        require_number(profile["line_height_mm"], "line_height_mm", 2, 40), dpi
    )
    text = data.get("text", [])
    for i, line in enumerate(text):
        y = top + i * line_h
        if y + line_h > bottom:
            raise ValueError(
                f"page {index + 1}: line {i + 1} overflows bottom margin"
            )
        pen.draw_line(
            canvas,
            line,
            left + point_mm(2, dpi),
            y,
            right - point_mm(2, dpi),
            index + 1,
            i,
        )
    date = data.get("date", "")
    date_style = paper.get("date_style", "none")
    if date and date_style != "none":
        if not isinstance(date, str) or len(date) > 60:
            raise ValueError("date must be short text")
        if date_style not in {"plain", "box", "underline"}:
            raise ValueError(
                "date_style must be none, plain, box, or underline"
            )
        font = ImageFont.truetype(pen.font_path, max(12, point_mm(3.5, dpi)))
        draw = ImageDraw.Draw(canvas)
        box = draw.textbbox((0, 0), date, font=font)
        tw = box[2] - box[0]
        side = paper.get("date_side", "right")
        if side not in {"left", "right"}:
            raise ValueError("date_side must be left or right")
        xx = left if side == "left" else right - tw - point_mm(4, dpi)
        xx -= box[0]
        yy = point_mm(
            require_number(paper.get("date_y_mm", 7), "date_y_mm", 0, 50), dpi
        )
        l, t, r, b = (xx + box[0], yy + box[1], xx + box[2], yy + box[3])
        if date_style == "box":
            outline = (
                l - point_mm(2, dpi),
                t - point_mm(1, dpi),
                r + point_mm(2, dpi),
                b + point_mm(1, dpi),
            )
            l, t, r, b = outline
        elif date_style == "underline":
            b += 2
        if l < 0 or t < 0 or r > width or b > top:
            raise ValueError("date label exceeds header or physical sheet")
        draw.text((xx, yy), date, font=font, fill=pen.ink)
        if date_style == "box":
            draw.rounded_rectangle(
                outline,
                radius=max(1, point_mm(1.5, dpi)),
                outline=pen.ink,
                width=1,
            )
        elif date_style == "underline":
            draw.line(
                (xx + box[0], b, xx + box[2], b),
                fill=pen.ink,
                width=1,
            )
    number_style = paper.get("page_number_style", "none")
    if number_style != "none":
        if number_style not in {"bottom-right", "bottom-left"}:
            raise ValueError(
                "page_number_style must be bottom-left, bottom-right, or none"
            )
        label = str(data.get("number", index + 1))
        if len(label) > 24:
            raise ValueError("page number label too long")
        draw = ImageDraw.Draw(canvas)
        font = ImageFont.truetype(pen.font_path, max(12, point_mm(3, dpi)))
        box = draw.textbbox((0, 0), label, font=font)
        xx = (
            right - box[2] if number_style == "bottom-right" else left - box[0]
        )
        yy = bottom + point_mm(2, dpi)
        l, t, r, b = (xx + box[0], yy + box[1], xx + box[2], yy + box[3])
        if l < 0 or t < 0 or r > width or b > height:
            raise ValueError("page number label exceeds physical sheet")
        draw.text((xx, yy), label, font=font, fill=pen.ink)
    for item in data.get("images", []):
        if item.get("layer") == "over":
            draw_picture(canvas, base, item, dpi)
    return canvas


def write_pdf(pages, page_count, width_mm, height_mm, out):
    """Stream a print-sized raster PDF with one JPEG resource per page."""
    write = out.write
    write(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    count = page_count

    def obj(num, data):
        assert len(offsets) == num
        offsets.append(out.tell())
        write(f"{num} 0 obj\n".encode("ascii"))
        write(data)
        write(b"\nendobj\n")

    obj(1, b"<< /Type /Catalog /Pages 2 0 R >>")
    kids = " ".join(f"{3 + 3 * i} 0 R" for i in range(count))
    obj(2, f"<< /Type /Pages /Count {count} /Kids [{kids}] >>".encode())
    wp, hp = width_mm * 72 / MM_PER_INCH, height_mm * 72 / MM_PER_INCH
    for i, canvas in enumerate(pages):
        if out.tell() > 200_000_000:
            raise ValueError("PDF output budget exceeded (200 MB)")
        page_n = 3 + 3 * i
        jpeg = io.BytesIO()
        background = Image.new("RGB", canvas.size, "white")
        background.paste(canvas, mask=canvas.getchannel("A"))
        background.save(jpeg, "JPEG", quality=94, optimize=True)
        payload = jpeg.getvalue()
        if len(payload) > 25_000_000:
            raise ValueError("one PDF page exceeds its 25 MB image budget")
        obj(
            page_n,
            (
                f"<< /Type /Page /Parent 2 0 R "
                f"/MediaBox [0 0 {wp:.4f} {hp:.4f}] "
                f"/Resources << /XObject << /Im0 {page_n + 2} 0 R >> >> "
                f"/Contents {page_n + 1} 0 R >>"
            ).encode(),
        )
        stream = f"q {wp:.4f} 0 0 {hp:.4f} 0 0 cm /Im0 Do Q\n".encode()
        obj(
            page_n + 1,
            f"<< /Length {len(stream)} >>\nstream\n".encode()
            + stream
            + b"endstream",
        )
        image_header = (
            f"<< /Type /XObject /Subtype /Image /Width {background.width} "
            f"/Height {background.height} /ColorSpace /DeviceRGB "
            f"/BitsPerComponent 8 /Filter /DCTDecode "
            f"/Length {len(payload)} >>\nstream\n"
        ).encode()
        obj(page_n + 2, image_header + payload + b"\nendstream")
    if out.tell() > 200_000_000:
        raise ValueError("PDF output budget exceeded (200 MB)")
    xref = out.tell()
    write(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    for offset in offsets[1:]:
        write(f"{offset:010d} 00000 n \n".encode())
    write(
        (
            f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\n"
            f"startxref\n{xref}\n%%EOF\n"
        ).encode()
    )


def parse_pages(pages, total):
    result = set()
    for section in pages.split(","):
        if not re.fullmatch(r"\d+(?:-\d+)?", section.strip()):
            raise ValueError("page selection must look like 1,3-5")
        start, _, stop = section.strip().partition("-")
        if stop and int(stop) < int(start):
            raise ValueError("page ranges must be ascending")
        for n in range(int(start), int(stop or start) + 1):
            if not 1 <= n <= total:
                raise ValueError(f"page {n} does not exist")
            result.add(n - 1)
    return result


def do_render(args):
    source = Path(args.config).resolve()
    doc = read_json(source)
    dpi = require_number(args.dpi, "dpi", 72, 400)
    validate_notebook(doc, dpi)
    config_base = source.parent
    if args.format == "png":
        out = Path(args.out)
        if out.exists() and not out.is_dir():
            raise ValueError("PNG output must be a directory")
        out.parent.mkdir(parents=True, exist_ok=True)
        names = [f"page-{i + 1:03d}.png" for i in range(len(doc["pages"]))]
        if out.is_dir() and not args.replace:
            for name in names:
                if (out / name).exists():
                    raise FileExistsError(
                        f"refusing to overwrite {out / name}; pass --replace"
                    )
        # Stage every page before publishing any. A later-page layout failure
        # must never overwrite successful earlier pages in the output folder.
        with tempfile.TemporaryDirectory(
            dir=out.parent, prefix=".atrament-pages-"
        ) as temporary:
            staging = Path(temporary)
            for i, name in enumerate(names):
                image = render_page(
                    doc,
                    i,
                    dpi,
                    config_base,
                    args.transparent,
                    args.keep_rules,
                )
                # Keep physical print scale when importing into image editors.
                image.save(
                    staging / name, "PNG", optimize=True, dpi=(dpi, dpi)
                )
            if not out.exists():
                staging.rename(out)
            else:
                for name in names:
                    if args.replace:
                        os.replace(staging / name, out / name)
                    else:
                        os.link(staging / name, out / name)
        for name in names:
            print(out / name)
    else:

        def save(f):
            images = (
                render_page(
                    doc, i, dpi, config_base, args.transparent, args.keep_rules
                )
                for i in range(len(doc["pages"]))
            )
            write_pdf(
                images,
                len(doc["pages"]),
                doc["paper"]["width_mm"],
                doc["paper"]["height_mm"],
                f,
            )

        atomic_write(args.out, save, args.replace)
        print(args.out)


def do_set_paper(args):
    doc = read_json(Path(args.config))
    paper = doc.get("paper")
    if not isinstance(paper, dict):
        raise TypeError("paper must be an object")
    changed = False
    for key, (low, high) in PAPER_NUMBER_OPTIONS.items():
        value = getattr(args, key)
        if value is not None:
            paper[key] = require_number(value, f"paper.{key}", low, high)
            changed = True
    for key in PAPER_CHOICE_OPTIONS:
        value = getattr(args, key)
        if value is not None:
            paper[key] = value
            changed = True
    for key in PAPER_COLOR_OPTIONS:
        value = getattr(args, key)
        if value is not None:
            color(value)
            paper[key] = value
            changed = True
    for key in PAPER_BOOLEAN_OPTIONS:
        value = getattr(args, key)
        if value is not None:
            paper[key] = value
            changed = True
    if not changed:
        raise ValueError("set-paper requires at least one paper option")
    width, height = validate_notebook(doc, 72)
    sample = Image.new("RGBA", (width, height))
    draw_paper(sample, paper, 72, False, False, False)
    rebase_notebook_assets(doc, args.config, args.out)
    write_json(args.out, doc, args.replace)
    print(args.out)


def do_set_handwriting(args):
    doc = read_json(Path(args.config))
    profile = doc.get("handwriting")
    if not isinstance(profile, dict):
        raise TypeError("handwriting must be an object")
    changed = False
    if args.font is not None:
        if not args.font or "\x00" in args.font:
            raise ValueError("font must be a nonempty path without NUL")
        profile["font"] = args.font
        changed = True
    if args.ink is not None:
        color(args.ink)
        profile["ink"] = args.ink
        changed = True
    for key, (low, high) in HANDWRITING_NUMBERS.items():
        value = getattr(args, key)
        if value is not None:
            require_number(value, f"handwriting.{key}", low, high)
            profile[key] = value
            changed = True
    for key in HANDWRITING_STATS:
        for field in (*STAT_FIELDS, "distribution"):
            value = getattr(args, f"{key}_{field}")
            if value is not None:
                stats = profile.get(key)
                if not isinstance(stats, dict):
                    raise TypeError(f"handwriting.{key} must be statistics")
                stats[field] = value
                changed = True
    if not changed:
        raise ValueError("set-handwriting requires at least one option")
    for key in HANDWRITING_STATS:
        stat_sample(profile[key], random.Random(0), key)
    if args.font is not None:
        # Reject a user-selected font that would fail on the first render.
        Pen(profile, 72, Path(args.config).resolve().parent)
    rebase_notebook_assets(doc, args.config, args.out)
    write_json(args.out, doc, args.replace)
    print(args.out)


def do_set_page(args):
    source = Path(args.config).resolve()
    doc = read_json(source)
    validate_notebook(doc, 72)
    pages = doc["pages"]
    if args.create:
        if args.page != len(pages) + 1 or len(pages) >= MAX_PAGES:
            raise ValueError("--create must select the next available page")
        pages.append({"date": "", "printed": False, "text": [], "images": []})
    elif not 1 <= args.page <= len(pages):
        raise ValueError("selected page does not exist")
    changed = args.create
    item = pages[args.page - 1]
    for field, maximum in (("date", 60), ("number", 24)):
        value = getattr(args, field)
        if value is not None:
            if len(value) > maximum or any(ch in value for ch in "\r\n\x00"):
                raise ValueError(
                    f"page {field} must be short, single-line text"
                )
            item[field] = value
            changed = True
    if args.printed is not None:
        item["printed"] = args.printed
        changed = True
    if args.line is not None:
        item["text"] = args.line
        changed = True
    if args.append_line is not None:
        item.setdefault("text", []).extend(args.append_line)
        changed = True
    if args.clear_text:
        item["text"] = []
        changed = True
    if not changed:
        raise ValueError("set-page requires page content or state changes")
    # Reject overflowing page text and header/footer labels before writing.
    render_page(doc, args.page - 1, 72, source.parent)
    rebase_notebook_assets(doc, source, args.out)
    write_json(args.out, doc, args.replace)
    print(args.out)


def do_add_image(args):
    source = Path(args.config).resolve()
    doc = read_json(source)
    width, height = validate_notebook(doc, 72)
    page_number = args.page
    if not 1 <= page_number <= len(doc["pages"]):
        raise ValueError("selected page does not exist")
    target = Path(args.out).resolve()
    source_image = Path(args.image)
    if not source_image.is_absolute():
        source_image = source.parent / source_image
    source_image = source_image.resolve()
    if args.tint_strength is not None and args.tint is None:
        raise ValueError("--tint-strength requires --tint")
    if args.tint is not None:
        color(args.tint)
    item = {
        "path": os.path.relpath(source_image, target.parent),
        "x_mm": require_number(args.x_mm, "image.x_mm", 0, 1000),
        "y_mm": require_number(args.y_mm, "image.y_mm", 0, 1000),
        "width_mm": require_number(args.width_mm, "image.width_mm", 0.1, 1000),
        "height_mm": require_number(
            args.height_mm, "image.height_mm", 0.1, 1000
        ),
        "layer": args.layer,
        "saturation": require_number(
            args.saturation, "image.saturation", 0, 5
        ),
        "opacity": require_number(args.opacity, "image.opacity", 0, 1),
    }
    if args.tint is not None:
        item["tint"] = args.tint
        item["tint_strength"] = require_number(
            args.tint_strength if args.tint_strength is not None else 0.5,
            "image.tint_strength",
            0,
            1,
        )
    # Render the exact asset at bounded preview DPI before persisting a path.
    # This also rejects missing, unsafe, unreadable, or off-sheet images.
    sample = Image.new("RGBA", (width, height))
    draw_picture(sample, target.parent, item, 72)
    rebase_notebook_assets(doc, source, target)
    images = doc["pages"][page_number - 1].setdefault("images", [])
    if len(images) >= 50:
        raise ValueError("page already contains 50 images")
    images.append(item)
    write_json(target, doc, args.replace)
    print(target)


def do_calibrate(args):
    source = Path(args.config)
    doc = read_json(source)
    glyph = unicodedata.normalize("NFC", args.glyph)
    if len(glyph) != 1:
        raise ValueError("--glyph must be exactly one character")
    if "handwriting" not in doc:
        raise ValueError("configuration has no handwriting section")
    custom = doc["handwriting"].setdefault("glyphs", {}).setdefault(glyph, {})
    measurements = (
        ("height_mm", args.heights, 0.1, 50),
        ("width_mm", args.widths, 0.1, 50),
        ("rotation_deg", args.rotations, -45, 45),
        ("baseline_shift_mm", args.baselines, -10, 10),
    )
    for key, raw, low, high in measurements:
        if raw is None:
            continue
        vals = [
            require_number(float(s.strip()), key, low, high)
            for s in raw.split(",")
        ]
        if not vals:
            raise ValueError("measurements cannot be empty")
        custom[key] = {
            "minimum": min(vals),
            "mean": statistics.mean(vals),
            "median": statistics.median(vals),
            "maximum": max(vals),
            "stddev": statistics.pstdev(vals),
            "distribution": "gaussian",
        }
    if all(raw is None for _, raw, _, _ in measurements):
        raise ValueError("provide at least one measurement series")
    rebase_notebook_assets(doc, source, args.out)
    write_json(args.out, doc, args.replace)
    print(args.out)


def do_guide(args):
    path = Path(args.image)
    with Image.open(path) as image:
        rgba = image.convert("RGBA")
    if not 0 <= args.upper < args.lower <= rgba.height:
        raise ValueError(
            "guide lines must satisfy 0 <= upper < lower <= image height"
        )
    draw = ImageDraw.Draw(rgba)
    draw.line(
        (0, args.upper, rgba.width, args.upper),
        fill=(220, 40, 45, 255),
        width=2,
    )
    draw.line(
        (0, args.lower, rgba.width, args.lower),
        fill=(50, 100, 225, 255),
        width=2,
    )
    draw.text(
        (10, args.upper + 4),
        f"Measured visual span: {args.lower - args.upper} px",
        fill=(220, 40, 45, 255),
    )
    atomic_write(args.out, lambda f: rgba.save(f, "PNG"), args.replace)
    print(
        f"{args.lower - args.upper} pixels between manual guides; "
        "no scale inferred"
    )


def do_mark(args):
    doc = read_json(Path(args.config))
    pages = doc.get("pages", [])
    if not isinstance(pages, list):
        raise TypeError("pages must be a list")
    selected = parse_pages(args.pages, len(pages))
    for page_idx in selected:
        pages[page_idx]["printed"] = not args.unmark
    rebase_notebook_assets(doc, args.config, args.out)
    write_json(args.out, doc, args.replace)
    print(
        f"Marked {len(selected)} page(s) "
        f"{'unprinted' if args.unmark else 'printed'}"
    )


def do_print(args):
    pdf = Path(args.pdf).resolve(strict=True)
    if pdf.suffix.lower() != ".pdf":
        raise ValueError("print requires an existing PDF file")
    copies = int(require_number(args.copies, "copies", 1, 20))
    command = ["lp", "-n", str(copies)]
    if args.printer:
        if not re.fullmatch(r"[\w.:-]{1,128}", args.printer):
            raise ValueError("invalid printer name")
        command += ["-d", args.printer]
    command += ["--", str(pdf)]
    try:
        subprocess.run(command, check=True, timeout=30)
    except FileNotFoundError as exc:
        raise ValueError("CUPS 'lp' is not installed") from exc


def main(argv=None):
    cli = argparse.ArgumentParser(
        description="Personal statistical handwriting notebook generator"
    )
    cmds = cli.add_subparsers(dest="action", required=True)
    p = cmds.add_parser(
        "init", help="write an editable notebook JSON template"
    )
    p.add_argument("--out", required=True)
    p.add_argument("--replace", action="store_true")
    p.set_defaults(
        run=lambda args: (
            write_json(args.out, DEFAULT, args.replace),
            print(args.out),
        )
    )
    p = cmds.add_parser(
        "render", help="render a notebook to printable PDF or PNG pages"
    )
    p.add_argument("--config", required=True)
    p.add_argument("--format", choices=("pdf", "png"), default="pdf")
    p.add_argument("--out", required=True)
    p.add_argument("--dpi", type=int, default=200)
    p.add_argument(
        "--transparent",
        action="store_true",
        help="omit paper fill and rules; keep content and margins",
    )
    p.add_argument(
        "--keep-rules",
        action="store_true",
        help="include grid/lines even with --transparent",
    )
    p.add_argument("--replace", action="store_true")
    p.set_defaults(run=do_render)
    p = cmds.add_parser(
        "set-paper", help="change sheet size, rules, margins and date layout"
    )
    p.add_argument("--config", required=True)
    for key in PAPER_NUMBER_OPTIONS:
        p.add_argument(f"--{key.replace('_', '-')}", type=float)
    for key, values in PAPER_CHOICE_OPTIONS.items():
        p.add_argument(f"--{key.replace('_', '-')}", choices=values)
    for key in PAPER_COLOR_OPTIONS:
        p.add_argument(f"--{key.replace('_', '-')}")
    for key in PAPER_BOOLEAN_OPTIONS:
        p.add_argument(
            f"--{key.replace('_', '-')}",
            action=argparse.BooleanOptionalAction,
            default=None,
        )
    p.add_argument("--out", required=True)
    p.add_argument("--replace", action="store_true")
    p.set_defaults(run=do_set_paper)
    p = cmds.add_parser(
        "set-handwriting", help="tune ink and global glyph distributions"
    )
    p.add_argument("--config", required=True)
    p.add_argument("--font")
    p.add_argument("--ink")
    for key in HANDWRITING_NUMBERS:
        option_type = int if key in {"stroke_px", "seed"} else float
        p.add_argument(f"--{key.replace('_', '-')}", type=option_type)
    for key in HANDWRITING_STATS:
        for field in STAT_FIELDS:
            p.add_argument(f"--{key.replace('_', '-')}-{field}", type=float)
        p.add_argument(
            f"--{key.replace('_', '-')}-distribution",
            choices=("gaussian", "triangular"),
        )
    p.add_argument("--out", required=True)
    p.add_argument("--replace", action="store_true")
    p.set_defaults(run=do_set_handwriting)
    p = cmds.add_parser(
        "set-page", help="create a sheet or edit its date, text, and flags"
    )
    p.add_argument("--config", required=True)
    p.add_argument("--page", type=int, required=True)
    p.add_argument("--create", action="store_true")
    p.add_argument("--date")
    p.add_argument("--number")
    p.add_argument(
        "--printed", action=argparse.BooleanOptionalAction, default=None
    )
    page_text = p.add_mutually_exclusive_group()
    page_text.add_argument("--line", action="append")
    page_text.add_argument("--append-line", action="append")
    page_text.add_argument("--clear-text", action="store_true")
    p.add_argument("--out", required=True)
    p.add_argument("--replace", action="store_true")
    p.set_defaults(run=do_set_page)
    p = cmds.add_parser(
        "add-image", help="place or recolor a local picture on one page"
    )
    p.add_argument("--config", required=True)
    p.add_argument("--page", type=int, required=True)
    p.add_argument("--image", required=True)
    for name in ("x-mm", "y-mm", "width-mm", "height-mm"):
        p.add_argument(f"--{name}", type=float, required=True)
    p.add_argument("--layer", choices=("under", "over"), default="under")
    p.add_argument("--saturation", type=float, default=1)
    p.add_argument("--opacity", type=float, default=1)
    p.add_argument("--tint")
    p.add_argument("--tint-strength", type=float)
    p.add_argument("--out", required=True)
    p.add_argument("--replace", action="store_true")
    p.set_defaults(run=do_add_image)
    p = cmds.add_parser(
        "calibrate",
        help="enter manually measured per-letter geometry samples",
    )
    p.add_argument("--config", required=True)
    p.add_argument("--glyph", required=True)
    p.add_argument(
        "--heights", help="comma-separated glyph heights in millimeters"
    )
    p.add_argument(
        "--widths", help="comma-separated glyph widths in millimeters"
    )
    p.add_argument(
        "--rotations", help="comma-separated glyph angle offsets in degrees"
    )
    p.add_argument(
        "--baselines", help="comma-separated baseline offsets in millimeters"
    )
    p.add_argument("--out", required=True)
    p.add_argument("--replace", action="store_true")
    p.set_defaults(run=do_calibrate)
    p = cmds.add_parser(
        "guide",
        help="place two manual visual ruler lines on a photograph",
    )
    p.add_argument("--image", required=True)
    p.add_argument("--upper", type=int, required=True)
    p.add_argument("--lower", type=int, required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--replace", action="store_true")
    p.set_defaults(run=do_guide)
    p = cmds.add_parser(
        "mark-printed", help="mark notebook pages already physically printed"
    )
    p.add_argument("--config", required=True)
    p.add_argument("--pages", required=True, help="1,3-5")
    p.add_argument("--out", required=True)
    p.add_argument("--unmark", action="store_true")
    p.add_argument("--replace", action="store_true")
    p.set_defaults(run=do_mark)
    p = cmds.add_parser(
        "print", help="explicitly send an existing PDF to CUPS"
    )
    p.add_argument("--pdf", required=True)
    p.add_argument("--printer")
    p.add_argument("--copies", type=int, default=1)
    p.set_defaults(run=do_print)
    args = cli.parse_args(argv)
    try:
        args.run(args)
    except (
        ValueError,
        FileNotFoundError,
        FileExistsError,
        OSError,
        KeyError,
        TypeError,
        json.JSONDecodeError,
        ET.ParseError,
        subprocess.CalledProcessError,
    ) as exc:
        cli.exit(2, f"atrament notebook: {exc}\n")


if __name__ == "__main__":
    main()
