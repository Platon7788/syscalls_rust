//! Parse lib.rs to extract types, structs, constants, and function signatures.
//! Also provides the ROR8 hash used by the SysWhispers3 runtime.

#[path = "ast.rs"]
mod ast;
#[path = "validate.rs"]
mod validate;

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

pub struct Field {
    pub name: String,
    pub typ: String,
    /// C preprocessor condition retained from a supported Rust field cfg.
    pub condition: Option<&'static str>,
}

pub struct Parsed {
    pub locations: std::collections::HashMap<String, proc_macro2::Span>,
    pub notices: Vec<String>,
    pub types: Vec<(String, String)>,
    pub structs: Vec<(String, Vec<Field>)>,
    pub constants: Vec<(String, String, String)>,
    pub functions: Vec<Function>,
}

fn located(span: proc_macro2::Span, message: impl std::fmt::Display) -> String {
    let start = span.start();
    format!("{}:{}: {message}", start.line, start.column + 1)
}

pub fn parse(content: &str) -> Result<Parsed, String> {
    let parsed = ast::parse(content)?;
    validate::validate(&parsed)?;
    Ok(parsed)
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
