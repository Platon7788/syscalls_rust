# syscalls-standalone

Generate a self-contained C/header/MASM bundle from the sibling `syscalls`
library. Requires stable Rust 1.99+; the workspace pins Rust 1.99.0.
The generator uses regex 1.13.1. Generated files can be integrated into
an MSVC project without a Rust runtime dependency.

Run from the repository root:

```sh
cargo run -p syscalls-standalone -- --out target/bundle
cargo run -p syscalls-standalone --bin audit
cargo clippy -p syscalls-standalone --all-targets -- -D warnings
cargo test -p syscalls-standalone
```

Use `--lib <path>` to select another input file. See the repository
[README](../README.md) for the generated bundle and integration details.
