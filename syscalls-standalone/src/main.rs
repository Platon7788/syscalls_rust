//! syscalls-standalone -- generate a self-contained C/H/ASM bundle from
//! syscalls-rust/lib.rs. Drop the output folder into an MSVC project and
//! include `syscalls.h`.
//!
//! Usage:
//!     cargo run -p syscalls-standalone -- --out <path>

mod emit_asm_x64;
mod emit_c;
mod emit_h;
mod emit_props;
mod emit_stubs_x86;
mod parse;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = match parse_out(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}\n\nusage: syscalls-standalone --out <dir> [--lib <lib.rs>]");
            return ExitCode::from(2);
        }
    };
    let lib_rs = parse_lib(&args).unwrap_or_else(default_lib_path);

    let content = match std::fs::read_to_string(&lib_rs) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", lib_rs.display());
            return ExitCode::from(2);
        }
    };

    let parsed = parse::parse(&content);
    println!(
        "parsed lib.rs: {} types, {} structs, {} constants, {} syscall fns",
        parsed.types.len(),
        parsed.structs.len(),
        parsed.constants.len(),
        parsed.functions.len(),
    );

    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("error: cannot create {}: {e}", out_dir.display());
        return ExitCode::from(2);
    }

    let outputs: [(&str, String); 6] = [
        ("syscalls.h", emit_h::emit(&parsed)),
        ("syscalls.c", emit_c::emit()),
        (
            "syscallsstubs.x64.asm",
            emit_asm_x64::emit(&parsed.functions),
        ),
        (
            "syscallsstubs.x86.c",
            emit_stubs_x86::emit(&parsed.functions),
        ),
        ("syscalls.props", emit_props::PROPS.to_string()),
        ("README.md", emit_props::README.to_string()),
    ];

    for (name, content) in &outputs {
        let path = out_dir.join(name);
        if let Err(e) = std::fs::write(&path, content) {
            eprintln!("error: cannot write {}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!("wrote {} ({} bytes)", path.display(), content.len());
    }

    println!("done. drop-in ready at {}", out_dir.display());
    ExitCode::SUCCESS
}

fn parse_out(args: &[String]) -> Result<PathBuf, String> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--out" || a == "-o" {
            let v = it
                .next()
                .ok_or_else(|| "--out requires a value".to_string())?;
            return Ok(PathBuf::from(v));
        }
    }
    Err("missing --out <dir>".to_string())
}

fn parse_lib(args: &[String]) -> Option<PathBuf> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--lib" {
            return it.next().map(PathBuf::from);
        }
    }
    None
}

fn default_lib_path() -> PathBuf {
    // Prefer the sibling `lib.rs` inside the workspace layout so `cargo run`
    // from the workspace root Just Works. Fall back to the current directory.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let sibling = Path::new(&manifest).join("..").join("lib.rs");
    if sibling.exists() {
        return sibling;
    }
    PathBuf::from("lib.rs")
}
