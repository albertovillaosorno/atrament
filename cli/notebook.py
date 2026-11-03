#!/usr/bin/env python3
# Copyright © 2026 Alberto Villa Osorno.
# SPDX-License-Identifier: MIT
"""Stable CLI entry point for the standalone personal notebook generator."""

import runpy
from pathlib import Path

if __name__ == "__main__":
    implementation = (
        Path(__file__).resolve().parents[1]
        / "tools"
        / "notebook-cli"
        / "application"
        / "notebook.py"
    )
    runpy.run_path(str(implementation), run_name="__main__")
