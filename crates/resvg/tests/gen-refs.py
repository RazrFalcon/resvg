#!/usr/bin/env python3

import os
import subprocess
import sys
from pathlib import Path

filter = sys.argv[1] if len(sys.argv) > 1 else "tests"
files = [el for el in Path('tests').rglob("*.svg") if filter in str(el)]

for file in files:
    subprocess.run(["cargo", "run", "--release",
                    "--",
                    "--width", "300",
                    "--skip-system-fonts",
                    "--use-fonts-dir", "tests/fonts",
                    "--font-family", "Noto Sans",
                    "--serif-family", "Noto Serif",
                    "--sans-serif-family", "Noto Sans",
                    "--cursive-family", "Yellowtail",
                    "--fantasy-family", "Sedgwick Ave Display",
                    "--monospace-family", "Noto Mono",
                    file,
                    file.with_suffix(".png")])
    subprocess.run(["oxipng", "-o", "6", "-Z", file.with_suffix(".png")])