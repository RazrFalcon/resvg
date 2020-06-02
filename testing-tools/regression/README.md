# regression testing

This tool is used for resvg regression testing.

Algorithm:

1. It will build the current sources using the specified backend.
1. It will `git clone` the previous commit from the `master` branch.
1. It will build it using the specified backend.
1. For each image in `resvg/svg-tests/svg`:
   1. It will render the image using the current version.
   1. It will render the image using the previous version.
   1. It will diff raster images.
1. It will print a list of images that were changed. Or none otherwise.

All images are rendered using the [tools/rendersvg](../../tools/rendersvg/README.md).
Also, all images are rendered with a 2x scale, to test scaling correctness.

To account for expected changes in rendering, we are using the `allow-*.txt` files.
Each backend has a corresponding file which should contain a list of file names
from `resvg/svg-tests/svg` that were affected by the current change.

## Run

```sh
cargo run --release -- --backend qt /path/to/tempdir
```

The `--backend` option specifies which backend to test. It can be cairo, qt, skia or raqote.

Since this tool will create/delete a lot of files, you should also specify a custom working directory.
On Linux, a `tmpfs` can be used to reduce HDD/SSD usage.

Depending on the backend, you might need to set additional environment variables
as specified in the [BUILD.adoc](../../BUILD.adoc).
