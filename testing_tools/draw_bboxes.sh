#!/usr/bin/env bash

cd ../

INPUT_SVG="tools/rendersvg/tests/images/bbox.svg"

cargo run --features "cairo-backend" --example draw_bboxes -- "$INPUT_SVG" bboxes_1_cairo.png
cargo run --features "qt-backend" --example draw_bboxes -- "$INPUT_SVG" bboxes_1_qt.png

cargo run --features "cairo-backend" --example draw_bboxes -- "$INPUT_SVG" bboxes_4_cairo.png -z 4
cargo run --features "qt-backend" --example draw_bboxes -- "$INPUT_SVG" bboxes_4_qt.png -z 4
