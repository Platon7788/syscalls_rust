//! syscalls-standalone -- generate a self-contained C/H/ASM bundle from
//! syscalls-rust/lib.rs. Drop the output folder into an MSVC project and
//! include `syscalls.h`.
//!
//! Usage:
//! ```text
//! cargo run -p syscalls-standalone -- --out <path>
//! ```

mod cli;
mod emit_asm_x64;
mod emit_c;
mod emit_h;
mod emit_props;
mod emit_stubs_x86;
mod output;
mod parse;

use std::process::ExitCode;

fn main() -> ExitCode {
    let (out_dir, lib_rs) = match cli::parse(std::env::args_os().skip(1)) {
        Ok(cli::Command::Help) => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        Ok(cli::Command::Generate { out, source }) => (out, source),
        Ok(cli::Command::Recover { out }) => {
            return match output::recover(&out) {
                Ok(message) => {
                    println!("{message}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            };
        }
        Err(e) => {
            eprintln!("error: {e}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    let content = match std::fs::read_to_string(&lib_rs) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", lib_rs.display());
            return ExitCode::from(2);
        }
    };

    let parsed = match parse::parse(&content) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {}:{e}", lib_rs.display());
            return ExitCode::from(2);
        }
    };
    println!(
        "parsed lib.rs: {} types, {} structs, {} constants, {} syscall fns",
        parsed.types.len(),
        parsed.structs.len(),
        parsed.constants.len(),
        parsed.functions.len(),
    );
    for notice in &parsed.notices {
        eprintln!("warning: {}:{notice}", lib_rs.display());
    }
    if parsed.functions.is_empty() {
        eprintln!(
            "error: {} contains no supported syscall declarations",
            lib_rs.display()
        );
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

    if let Err(e) = output::write_bundle(&out_dir, &outputs) {
        eprintln!("error: {e}");
        return ExitCode::from(2);
    }
    for (name, content) in &outputs {
        println!(
            "wrote {} ({} bytes)",
            out_dir.join(name).display(),
            content.len()
        );
    }
    println!("done. drop-in ready at {}", out_dir.display());
    ExitCode::SUCCESS
}
