## Dependencies

```
cargo install afl
```

## Run

```
ln -s /path_to_resvg-test-suite/svg in
env RUSTFLAGS="-Clink-arg=-fuse-ld=gold" cargo afl build
cargo afl fuzz -i in -o out target/debug/afl-fuzz
```
