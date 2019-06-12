## Dependencies

```
cargo install afl
```

## Run

```bash
ln -s /path-to-resvg-test-suite/svg in
env RUSTFLAGS="-Clink-arg=-fuse-ld=gold" cargo afl build
cargo afl fuzz -i in -o out target/debug/afl-fuzz
```
