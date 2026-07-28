//! Parse lib.rs to extract types, structs, constants, and function signatures.
//! Also provides the ROR8 hash used by the SysWhispers3 runtime.

use std::collections::HashSet;

pub const SW3_SEED: u32 = 0xB8A54425;

/// ROR8 hash of a syscall name (must match the Rust runtime).
///
/// The runtime hashes the `Zw`-prefixed export name (that's what shows up in
/// ntdll's export directory in the order syscall numbers assign). Callers of
/// this helper should therefore hash `"ZwFoo"`, not `"NtFoo"`.
pub fn hash_syscall(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut hash = SW3_SEED;
    let mut i = 0;
    while i < bytes.len() {
        let b1 = bytes[i] as u32;
        // Reads one byte past the current position; the terminating NUL byte
        // in the C runtime keeps this in-bounds. Here we mirror that: for the
        // last non-NUL char b2 becomes 0.
        let b2 = if i + 1 < bytes.len() {
            bytes[i + 1] as u32
        } else {
            0
        };
        let partial = b1 | (b2 << 8);
        hash ^= partial.wrapping_add(hash.rotate_right(8));
        i += 1;
    }
    hash
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub typ: String,
}

#[derive(Debug, Clone)]
pub struct Function {
    /// Original snake_case name from lib.rs (e.g. `nt_allocate_virtual_memory`).
    /// Kept for debugging and future tools; not read by the current emitters.
    #[allow(dead_code)]
    pub name: String,
    /// PascalCase NT name (e.g. `NtAllocateVirtualMemory`).
    pub pascal: String,
    pub params: Vec<Param>,
    pub return_type: String,
    /// Hash of the corresponding `Zw*` name — this is the key the C runtime
    /// looks up in its table.
    pub zw_hash: u32,
}

pub struct Parsed {
    pub types: Vec<(String, String)>,
    pub structs: Vec<(String, Vec<(String, String)>)>,
    pub constants: Vec<(String, String, String)>,
    pub functions: Vec<Function>,
}

pub fn parse(content: &str) -> Parsed {
    Parsed {
        types: extract_type_aliases(content),
        structs: extract_structs(content),
        constants: extract_constants(content),
        functions: extract_functions(content),
    }
}

fn extract_type_aliases(content: &str) -> Vec<(String, String)> {
    let re = regex::Regex::new(r"pub type (\w+)\s*=\s*([^;]+);").unwrap();
    re.captures_iter(content)
        .filter_map(|c| {
            let name = c[1].to_string();
            // `pub type c_void = core::ffi::c_void` -> `typedef void X_c_void;` --
            // legal C but noise: nothing references it (Rust `*mut c_void` maps
            // straight to `void*`, not `X_c_void*`). Drop it.
            if name == "c_void" {
                return None;
            }
            Some((name, c[2].trim().to_string()))
        })
        .collect()
}

fn extract_constants(content: &str) -> Vec<(String, String, String)> {
    let re = regex::Regex::new(r"pub const (\w+):\s*(\w+)\s*=\s*([^;]+);").unwrap();
    re.captures_iter(content)
        .map(|c| (c[1].to_string(), c[2].to_string(), c[3].trim().to_string()))
        .collect()
}

fn extract_structs(content: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut structs = Vec::new();
    let struct_re = regex::Regex::new(
        r#"#\[repr\(C\)\]\s*(?:#\[derive[^\]]*\]\s*)*pub struct (\w+)\s*\{([^}]*)\}"#,
    )
    .unwrap();
    let field_re = regex::Regex::new(r"pub (\w+):\s*([^,\n]+)").unwrap();

    for cap in struct_re.captures_iter(content) {
        let name = cap[1].to_string();
        let body = &cap[2];
        let mut fields = Vec::new();
        for f in field_re.captures_iter(body) {
            let fname = f[1].to_string();
            let ftype = f[2].trim().trim_end_matches(',').to_string();
            fields.push((fname, ftype));
        }
        if !fields.is_empty() {
            structs.push((name, fields));
        }
    }
    structs
}

fn extract_functions(content: &str) -> Vec<Function> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let fn_re =
        regex::Regex::new(r#"pub unsafe (?:extern "C" )?fn (\w+)\s*\(([^)]*)\)\s*->\s*(\w+)"#)
            .unwrap();

    for cap in fn_re.captures_iter(content) {
        let name = cap[1].to_string();
        if seen.contains(&name) {
            continue;
        }
        seen.insert(name.clone());

        // Only real NT syscalls — skip runtime helpers (sw3_*, etc.).
        if !name.starts_with("nt_") {
            continue;
        }

        let params = parse_params(&cap[2]);
        let return_type = cap[3].to_string();
        let pascal = to_pascal_case(&name);
        let zw_name = format!("Zw{}", &pascal[2..]);
        let zw_hash = hash_syscall(&zw_name);

        out.push(Function {
            name,
            pascal,
            params,
            return_type,
            zw_hash,
        });
    }
    out
}

fn parse_params(s: &str) -> Vec<Param> {
    let mut out = Vec::new();
    if s.trim().is_empty() {
        return out;
    }

    let mut current = String::new();
    let mut depth = 0i32;
    for ch in s.chars() {
        match ch {
            '<' | '(' => {
                depth += 1;
                current.push(ch);
            }
            '>' | ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                if let Some(p) = parse_single(&current) {
                    out.push(p);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if let Some(p) = parse_single(&current) {
        out.push(p);
    }
    out
}

fn parse_single(s: &str) -> Option<Param> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (n, t) = s.split_once(':')?;
    let name = n.trim();
    let typ = t.trim();
    if name.is_empty() || typ.is_empty() {
        return None;
    }
    if name.starts_with('_') || typ.contains("enum ") || typ.contains("struct ") {
        return None;
    }
    Some(Param {
        name: sanitize_param_name(name),
        typ: typ.to_string(),
    })
}

fn sanitize_param_name(name: &str) -> String {
    match name {
        "class" => "class_name",
        "namespace" => "namespace_name",
        "template" => "template_name",
        "typename" => "typename_name",
        "operator" => "operator_name",
        "public" => "public_access",
        "private" => "private_access",
        "protected" => "protected_access",
        "virtual" => "virtual_flag",
        "static" => "static_flag",
        "const" => "const_flag",
        "volatile" => "volatile_flag",
        "mutable" => "mutable_flag",
        "explicit" => "explicit_flag",
        "inline" => "inline_flag",
        "friend" => "friend_access",
        "using" => "using_directive",
        "try" => "try_block",
        "catch" => "catch_block",
        "throw" => "throw_exception",
        "new" => "new_operator",
        "delete" => "delete_operator",
        "this" => "this_pointer",
        "true" => "true_value",
        "false" => "false_value",
        "and" => "and_operator",
        "or" => "or_operator",
        "not" => "not_operator",
        "xor" => "xor_operator",
        "bitand" => "bitand_operator",
        "bitor" => "bitor_operator",
        "compl" => "compl_operator",
        "and_eq" => "and_eq_operator",
        "or_eq" => "or_eq_operator",
        "xor_eq" => "xor_eq_operator",
        "not_eq" => "not_eq_operator",
        _ => return name.to_string(),
    }
    .to_string()
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(c).collect(),
            }
        })
        .collect()
}

/// Rust → C type mapping with an `X` prefix on every NT type.
///
/// This deliberately does NOT emit alias `#defines` back to `NtSomething` —
/// the drop-in bundle is meant to coexist with `<windows.h>` without name
/// collisions, so callers always spell the `X`-prefixed name.
pub fn rust_to_c_type(rust_type: &str) -> String {
    let t = rust_type.trim();
    match t {
        "i8" => return "int8_t".to_string(),
        "i16" => return "int16_t".to_string(),
        "i32" => return "int32_t".to_string(),
        "i64" => return "int64_t".to_string(),
        "u8" => return "uint8_t".to_string(),
        "u16" => return "uint16_t".to_string(),
        "u32" => return "uint32_t".to_string(),
        "u64" => return "uint64_t".to_string(),
        "usize" => return "size_t".to_string(),
        "isize" => return "intptr_t".to_string(),
        "bool" => return "bool".to_string(),
        "c_void" | "core::ffi::c_void" => return "void".to_string(),
        _ => {}
    }

    if let Some(inner) = t.strip_prefix("*mut ") {
        return format!("{}*", rust_to_c_type(inner));
    }
    if let Some(inner) = t.strip_prefix("*const ") {
        return format!("const {}*", rust_to_c_type(inner));
    }
    if t.starts_with("Option<") {
        return "void*".to_string();
    }
    if t.starts_with('[') && t.contains(';') && t.ends_with(']') {
        let inner = &t[1..t.len() - 1];
        let parts: Vec<&str> = inner.split(';').collect();
        if parts.len() == 2 {
            return format!("{}[{}]", rust_to_c_type(parts[0].trim()), parts[1].trim());
        }
        return "void*".to_string();
    }
    if t.contains("enum ") {
        return "uint32_t".to_string();
    }
    if t.contains("struct ") {
        return "void*".to_string();
    }

    // Every remaining NT type gets an `X_` prefix.
    format!("X_{}", t)
}
