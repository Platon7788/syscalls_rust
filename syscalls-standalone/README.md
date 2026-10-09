# syscalls-standalone

Generate a self-contained C/header/MASM bundle from the sibling `syscalls`
library. Requires stable Rust 1.99+; the workspace pins Rust 1.99.0.
The generator uses syn 2.0.119 to parse Rust syntax into an AST. Generated files can be integrated into
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

Without `--lib`, the input is `lib.rs` in the current working directory.
This is the same for `cargo run` and a directly launched executable.
Use `--help` (or `-h`) for usage; `-o` is an alias for `--out`.
Unknown options, duplicate options, and missing values are errors (exit code 2).
Paths starting with `-` must be prefixed with `./`.

Input with no supported syscall declarations is rejected before output is
created. Unsupported field attributes report the structure and field name.
The AST parser validates Rust syntax, but the C emitter supports a limited
set of declarations: public aliases, named repr(C) structs, integer constants
and the selected function signatures. Unsupported types and expressions
produce errors. Macros and external modules are not expanded; this is not
a general-purpose Rust-to-C compiler. Callback aliases retain their existing
opaque-pointer representation.

Before emission, declaration validation rejects unknown type/constant references,
duplicate names, generated C name collisions (including helper names), invalid
C/C++ field and parameter identifiers, and alias/constant dependency cycles.
Aliases must appear after aliases they reference; structs have forward declarations.
By-value struct fields require an earlier complete definition. Arrays are supported
only as nonempty one-dimensional struct fields, not aliases or pointer targets.
Unknown types are never guessed to be `void*`: opaque types require an explicit
source alias. Repeated function declarations require distinct x86/x64 conditions
and matching parameter/return types. This validation covers the selected
declarations, not Rust type checking, ABI verification, or macro expansion.

Diagnostics include one-based `file:line:column` positions for syntax, declaration,
field, and reference errors. Generation reports top-level items not selected or
expanded (modules, re-exports, macros, unions, enums, traits, statics, impl items,
foreign declarations, and nonselected public functions). Warnings describe the
export boundary; they do not claim those items are translated.

Output is staged in `<out>/.syscalls-write-lock`. A versioned manifest records
filenames and whether each destination existed. `old-N` and `new-N` contain
snapshots; destinations are untouched until staging and the manifest are ready.
A persistent `.syscalls-writer.lock` file carries an OS lock shared by writers
and recovery; its mere existence does not mean a process is still running.
Do not delete the lock file while a writer or recovery is active.

After a process interruption, run:

```sh
cargo run -p syscalls-standalone -- --recover target/bundle
```

Recovery rolls back an unfinished installation or completes cleanup of a committed
transaction. It refuses to overwrite destinations edited after interruption and
preserves recovery snapshots on error. An active writer, unexpected staging
entries, invalid manifests and non-regular files are rejected. Legacy transactions
without the new manifest/preparation marker require manual inspection.
Unrelated files are preserved; no recursive directory deletion is used.
This is **not atomic publication to concurrent readers** and does not promise
power-loss durability or protection against deliberate tampering with snapshots.

`tests/verify-header-layout.ps1` compiles the header as C11 and C++20 on x64/x86.
It compares all 26 exported structures with Rust layout contracts and 12 structures
with matching Windows SDK definitions. Rust-layout agreement alone does not prove
that an undocumented Windows structure is complete or correct.
Run the full verification locally from PowerShell:

```powershell
./tests/verify-quality.ps1 -Offline
```

This runs formatting, Clippy, workspace tests, two generations with SHA-256
comparison of all six artifacts, and the C/C++ layout checks. MSVC with the
Windows SDK and both Rust MSVC targets must be installed. Omit `-Offline` if
dependencies are not cached. No GitHub Actions workflow is installed.
Cargo.lock is retained for the workspace's generator binary.
