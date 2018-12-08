See [docs/build.md](../docs/build.md) first.

Note: we are using *qt-backend* just for example.

### custom.rs

Render image using a manually constructed SVG render tree.

```bash
cargo run --features "qt-backend" --example custom_rtree
```

### draw_bboxes.rs

Draw bounding boxes aroung all shapes on input SVG.

```bash
cargo run --features "qt-backend" --example draw_bboxes -- bboxes.svg bboxes.png -z 4
```

### minimal.rs

A simple SVG to PNG converter.

```bash
cargo run --features "qt-backend" --example minimal -- in.svg out.png
```
