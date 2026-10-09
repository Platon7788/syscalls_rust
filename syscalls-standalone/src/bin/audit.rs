//! Standalone audit tool: report hash collisions across all 513 syscalls.

use std::collections::HashMap;
use std::path::PathBuf;

// Reuse the same parse code path. The bin only cares about function names +
// hashes; parser also exposes types/structs/constants and helper types that
// this tool doesn't touch, hence the crate-wide allow.
#[allow(dead_code)]
#[path = "../parse.rs"]
mod parse;

fn main() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let lib = PathBuf::from(&manifest).join("..").join("lib.rs");
    let content = std::fs::read_to_string(&lib).expect("read lib.rs");
    let parsed = match parse::parse(&content) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("error: {}:{error}", lib.display());
            std::process::exit(2);
        }
    };

    let mut buckets: HashMap<u32, Vec<String>> = HashMap::new();
    for notice in &parsed.notices {
        eprintln!("warning: {}:{notice}", lib.display());
    }
    for f in &parsed.functions {
        buckets.entry(f.zw_hash).or_default().push(f.pascal.clone());
    }

    let mut collisions = 0;
    for (h, names) in &buckets {
        if names.len() > 1 {
            collisions += 1;
            println!("COLLISION at 0x{:08X}: {}", h, names.join(", "));
        }
    }
    println!(
        "\n{} syscalls, {} unique hashes, {} collision buckets",
        parsed.functions.len(),
        buckets.len(),
        collisions
    );

    if collisions > 0 {
        std::process::exit(1);
    }
}
