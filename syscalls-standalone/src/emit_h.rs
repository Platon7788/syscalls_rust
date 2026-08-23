//! Emit standalone `syscalls.h` — X-prefixed types and function declarations.
//!
//! Zero dependency on `<windows.h>`. All names use the `X` / `X_` prefix so
//! the bundle drops into a project that already includes the Windows SDK.

use crate::parse::{Function, Parsed, rust_to_c_type};
use std::collections::HashSet;
use std::fmt::Write as _;

pub fn emit(p: &Parsed) -> String {
    let mut h = String::new();

    preamble(&mut h);
    forward_struct_decls(&mut h, p);
    type_aliases(&mut h, p);
    opaque_stubs(&mut h, p);
    struct_bodies(&mut h, p);
    constants(&mut h, p);
    runtime_helpers(&mut h);
    function_decls(&mut h, &p.functions);
    helper_macros(&mut h);
    footer(&mut h);

    h
}

fn preamble(h: &mut String) {
    h.push_str(
        r#"/*
 * X-Syscalls -- direct NT syscall bundle (drop-in for MSVC, CRT-free)
 *
 * Auto-generated -- DO NOT EDIT MANUALLY.
 *
 * Ships with:
 *   syscalls.h              -- this header
 *   syscalls.c              -- runtime (PEB walk, hash table)
 *   syscallsstubs.x64.asm   -- MASM stubs for x64
 *   syscallsstubs.x86.c     -- MSVC naked stubs for x86 (+ WoW64 gate)
 *
 * Every symbol uses an `X` prefix so nothing collides with <windows.h>.
 */

#ifndef X_SYSCALLS_H
#define X_SYSCALLS_H

#include <stdint.h>
#include <stddef.h>

/* Calling convention for NT syscall stubs:
 * on x86 the syscall wrappers exposed by ntdll are __stdcall (callee cleans);
 * on x64 there's only one C convention so the qualifier is empty. */
#if defined(_M_X64) || defined(__x86_64__)
#  define X_STDCALL
#elif defined(_M_IX86) || defined(__i386__)
#  define X_STDCALL __stdcall
#else
#  define X_STDCALL
#endif

#ifdef __cplusplus
extern "C" {
#endif

"#,
    );
}

/// Every parsed struct gets a forward declaration so pointer aliases defined
/// before the body still compile.
fn forward_struct_decls(h: &mut String, p: &Parsed) {
    h.push_str("/* ==================== Forward struct decls ==================== */\n\n");
    for (name, _) in &p.structs {
        writeln!(h, "typedef struct _X_{n} X_{n};", n = name).unwrap();
    }
    h.push('\n');
}

fn type_aliases(h: &mut String, p: &Parsed) {
    h.push_str("/* ==================== Type aliases ==================== */\n\n");
    for (name, rust_type) in &p.types {
        let c = rust_to_c_type(rust_type);
        writeln!(h, "typedef {} X_{};", c, name).unwrap();
    }
    h.push('\n');
}

/// Emit `typedef void* X_FOO;` for every parameter/return type that was
/// referenced but never defined as a `pub type`, `pub struct`, or primitive.
/// Covers opaque routines like `PWNF_DELIVERY_DESCRIPTOR` that appear only in
/// signatures.
fn opaque_stubs(h: &mut String, p: &Parsed) {
    let mut defined: HashSet<String> = HashSet::new();
    for (n, _) in &p.types {
        defined.insert(n.clone());
    }
    for (n, _) in &p.structs {
        defined.insert(n.clone());
    }
    for (n, _, _) in &p.constants {
        defined.insert(n.clone());
    }

    let primitives: HashSet<&str> = [
        "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "usize", "isize", "bool", "c_void",
        // Rust type-syntax keywords that leak out of the identifier scan.
        "mut", "const", "dyn", "impl", "Option", "core", "ffi",
    ]
    .into_iter()
    .collect();

    let mut opaque: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for f in &p.functions {
        for t in std::iter::once(&f.return_type).chain(f.params.iter().map(|p| &p.typ)) {
            for id in extract_identifiers(t) {
                if primitives.contains(id.as_str()) {
                    continue;
                }
                if defined.contains(&id) {
                    continue;
                }
                if seen.insert(id.clone()) {
                    opaque.push(id);
                }
            }
        }
    }

    if !opaque.is_empty() {
        h.push_str("/* ==================== Opaque pointer types ==================== */\n\n");
        opaque.sort();
        for id in opaque {
            writeln!(h, "typedef void* X_{};", id).unwrap();
        }
        h.push('\n');
    }
}

fn extract_identifiers(t: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in t.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            cur.push(ch);
        } else {
            flush_ident(&mut cur, &mut out);
        }
    }
    flush_ident(&mut cur, &mut out);
    out
}

fn flush_ident(cur: &mut String, out: &mut Vec<String>) {
    if cur.is_empty() {
        return;
    }
    let is_ident = cur.chars().next().unwrap().is_alphabetic() || cur.starts_with('_');
    if is_ident {
        out.push(std::mem::take(cur));
    } else {
        cur.clear();
    }
}

fn struct_bodies(h: &mut String, p: &Parsed) {
    h.push_str("/* ==================== Struct definitions ==================== */\n\n");
    for (name, fields) in &p.structs {
        writeln!(h, "struct _X_{} {{", name).unwrap();
        if fields.is_empty() {
            writeln!(h, "    uint32_t X_reserved;").unwrap();
        } else {
            for (fname, ftype) in fields {
                if let Some((elem, count)) = parse_array(ftype) {
                    let c = rust_to_c_type(&elem);
                    writeln!(h, "    {} {}[{}];", c, fname, count).unwrap();
                } else {
                    let c = rust_to_c_type(ftype);
                    writeln!(h, "    {} {};", c, fname).unwrap();
                }
            }
        }
        writeln!(h, "}};\n").unwrap();
    }
}

fn parse_array(t: &str) -> Option<(String, String)> {
    let t = t.trim();
    if !(t.starts_with('[') && t.ends_with(']') && t.contains(';')) {
        return None;
    }
    let inner = &t[1..t.len() - 1];
    let (elem, count) = inner.split_once(';')?;
    Some((elem.trim().to_string(), count.trim().to_string()))
}

fn constants(h: &mut String, p: &Parsed) {
    h.push_str("/* ==================== Constants (#define) ==================== */\n\n");
    for (name, _typ, value) in &p.constants {
        writeln!(h, "#define X_{} {}", name, normalize_literal(value)).unwrap();
    }
    h.push('\n');
}

/// Convert a Rust literal expression into a C literal expression.
///
/// Handles the two idioms actually used in lib.rs:
///   * `0x1234u32` -> `0x1234`
///   * `0xC0000001u32 as i32` -> `(int32_t)0xC0000001`
fn normalize_literal(v: &str) -> String {
    let v = v.trim();
    // "<hex>u32 as i32"
    let cast_re = regex::Regex::new(r"^0x([0-9A-Fa-f]+)u32\s+as\s+i32$").unwrap();
    if let Some(c) = cast_re.captures(v) {
        return format!("(int32_t)0x{}", &c[1]);
    }
    // Strip integer type suffixes.
    let suffix_re = regex::Regex::new(r"(u|i)(8|16|32|64|size)$").unwrap();
    let mut cleaned = v.to_string();
    for suf in [
        "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize",
    ] {
        cleaned = cleaned.replace(suf, "");
    }
    let _ = suffix_re; // used to shape allowed suffixes; kept for clarity
    cleaned
}

fn runtime_helpers(h: &mut String) {
    h.push_str(r#"/* ==================== Runtime helpers (implemented in syscalls.c) ==================== */
/* NB: helpers use the default calling convention (cdecl on x86, single conv on
 * x64) -- the stubs call them with matching signatures. */

uint32_t X_GetSyscallNumber(uint32_t function_hash);
void*    X_GetSyscallAddress(uint32_t function_hash);
void*    X_GetRandomSyscallAddress(uint32_t function_hash);
uint32_t X_DebugGetCount(void);
uint32_t X_DebugGetHash(size_t index);
void*    X_DebugGetSyscallAddr(size_t index);

"#);
}

fn function_decls(h: &mut String, functions: &[Function]) {
    h.push_str("/* ==================== NT syscall declarations ==================== */\n\n");
    for f in functions {
        let ret = rust_to_c_type(&f.return_type);
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{} {}", rust_to_c_type(&p.typ), p.name))
            .collect();
        let params_str = if params.is_empty() {
            "void".to_string()
        } else {
            params.join(", ")
        };
        writeln!(h, "{} X_STDCALL X{}({});", ret, f.pascal, params_str).unwrap();
    }
    h.push('\n');
}

fn helper_macros(h: &mut String) {
    h.push_str(
        r#"/* ==================== Helper macros ==================== */

#define X_NT_SUCCESS(s)     ((X_NTSTATUS)(s) >= 0)
#define X_NT_INFORMATION(s) ((((uint32_t)(s)) >> 30) == 1u)
#define X_NT_WARNING(s)     ((((uint32_t)(s)) >> 30) == 2u)
#define X_NT_ERROR(s)       ((((uint32_t)(s)) >> 30) == 3u)

#define X_NtCurrentProcess() ((X_HANDLE)(intptr_t)-1)
#define X_NtCurrentThread()  ((X_HANDLE)(intptr_t)-2)

#define X_InitializeObjectAttributes(p, n, a, r, s) do { \
    (p)->Length = sizeof(X_OBJECT_ATTRIBUTES); \
    (p)->RootDirectory = (r); \
    (p)->Attributes = (a); \
    (p)->ObjectName = (n); \
    (p)->SecurityDescriptor = (s); \
    (p)->SecurityQualityOfService = NULL; \
} while (0)

"#,
    );
}

fn footer(h: &mut String) {
    h.push_str(
        r#"#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* X_SYSCALLS_H */
"#,
    );
}
