We don't use cargo build script, since this data will be changed rarely and
there is no point in regenerating it each time.

Note that `/spec/*` files contain only values that are supported by `usvg`.

To regenerate files run:

```
cargo run
```
